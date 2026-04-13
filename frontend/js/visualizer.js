// ── Audio Visualizer Tab ──
// 6 real-time displays: FFT, oscilloscope, spectrogram, stereo, levels, bands.
// Primary data: audio-engine `playback_status.spectrum` (see `applyPlaybackStatusSpectrum` in
// audio-engine.js). Web Audio `AnalyserNode` is only used when the AudioEngine is not supplying spectrum.
// Grid (all) or single mode. Fullscreen. Trello drag to rearrange. Context menus.

let _vizMode = 'all';
let _vizRAF = null;
/** Throttle / idle wake — avoids 60Hz `requestAnimationFrame` when Visualizer tab is open but idle or between throttled frames. */
let _vizBackoffTimer = null;
let _vizSpectrogramData = [];
let _vizSpectrogramIdx = 0; // ring buffer write index
let _vizLastFrame = 0;
const _VIZ_FPS_SINGLE = 30;
const _VIZ_FPS_ALL = 20;
/** Log-frequency axis right edge; must match `NP_SPECTRUM_DISPLAY_MAX_HZ` in audio.js. */
const _VIZ_LOG_FMAX_HZ = 20000;
// Pre-allocated buffers (set once when analyser is available)
let _vizFreqData = null;
let _vizTimeData = null;
let _vizParams = {
    fftSmoothing: 0.8,
    fftLogScale: true,
    oscilloscopeTraceColor: 'cyan',
    /** True: align trace to first rising zero-cross (scope-style). False: free-run (raw buffer order). */
    oscilloscopeTriggeredSweep: true,
    spectrogramSpeed: 1,
    levelsHold: true,
    bandsCount: 10,
};
let _vizPeakHold = -96;
let _vizPeakTimer = null;
/** Last spectrum snapshot for FFT tile when `viz:fft` is frozen. */
let _vizFftFrozenCopy = null;
/** Separate float buffers so oscilloscope freeze does not starve levels (shared `_vizTimeData` was one coupled snapshot). */
let _vizLevelsTimeData = null;
/** Last oscilloscope trace while live — used when `viz:oscilloscope` is frozen. */
let _vizOscFrozenCopy = null;
/** Octave bands magnitude copy while `viz:bands` is live. */
let _vizBandsFrozenCopy = null;
/** Web Audio L/R time buffers for stereo when not on engine scope / spectrum paths. */
let _vizStereoWebFrozenL = null;
let _vizStereoWebFrozenR = null;

const _vizScopeSnapByTileMode = Object.create(null);
const _vizSpecSnapByTileMode = Object.create(null);

function _vizFreezeIdForTileMode(mode) {
    const G = typeof window !== 'undefined' ? window.GRAPH_FREEZE_ID : null;
    if (!G) return null;
    switch (mode) {
        case 'fft':
            return G.VIZ_FFT;
        case 'oscilloscope':
            return G.VIZ_OSC;
        case 'spectrogram':
            return G.VIZ_SPEC;
        case 'stereo':
            return G.VIZ_STEREO;
        case 'levels':
            return G.VIZ_LEVELS;
        case 'bands':
            return G.VIZ_BANDS;
        default:
            return null;
    }
}

function _vizTileFrozen(mode) {
    const id = _vizFreezeIdForTileMode(mode);
    return !!(id && typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(id));
}

function _vizUpdateScopeSnap(tileMode, liveL, liveR, liveN) {
    const n = typeof liveN === 'number' ? liveN : 0;
    if (n < 16 || !liveL || !liveR) return;
    let ent = _vizScopeSnapByTileMode[tileMode];
    if (!ent || ent.n !== n) {
        ent = {sl: new Uint8Array(liveL), sr: new Uint8Array(liveR), n};
        _vizScopeSnapByTileMode[tileMode] = ent;
    } else {
        ent.sl.set(liveL);
        ent.sr.set(liveR);
    }
}

function _vizEngineScopeBuffersForTile(tileMode) {
    const frozen = _vizTileFrozen(tileMode);
    const n = _vizEngineScopeLen();
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const sr = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (!frozen) {
        _vizUpdateScopeSnap(tileMode, sl, sr, n);
        return {sl, sr, n};
    }
    const ent = _vizScopeSnapByTileMode[tileMode];
    if (ent && ent.n >= 16 && ent.sl && ent.sr) {
        return {sl: ent.sl, sr: ent.sr, n: ent.n};
    }
    return {sl, sr, n};
}

function _vizEngineSpectrumBufferForTile(tileMode) {
    const frozen = _vizTileFrozen(tileMode);
    const minB = _vizEngineSpectrumMinBins();
    const u8 =
        typeof window !== 'undefined' && window._engineSpectrumU8 && window._engineSpectrumU8.length >= minB
            ? window._engineSpectrumU8
            : null;
    if (!frozen) {
        if (u8 && tileMode) {
            let s = _vizSpecSnapByTileMode[tileMode];
            if (!s || s.length !== u8.length) s = new Uint8Array(u8.length);
            s.set(u8);
            _vizSpecSnapByTileMode[tileMode] = s;
        }
        return u8;
    }
    const s = _vizSpecSnapByTileMode[tileMode];
    return s && s.length >= minB ? s : u8;
}

function _vizLevelsUsesEngineScopeLive() {
    const n = _vizEngineScopeLen();
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    return n >= 32 && sl && window._engineScopeR && _vizEngineSpectrumOk();
}

function _vizEngineSpectrumMinBins() {
    return typeof window !== 'undefined' && typeof window.ENGINE_PLAYBACK_SPECTRUM_MIN_BINS === 'number'
        ? window.ENGINE_PLAYBACK_SPECTRUM_MIN_BINS
        : 64;
}
/** Last rising-edge trigger index for oscilloscope when the current buffer has no crossing. */
let _vizOscLastTrigger = 0;

function _vizEngineScopeLen() {
    return typeof window !== 'undefined' && typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
}

/** AudioEngine is sending usable spectrum bins (`playback_status.spectrum`). */
function _vizEngineSpectrumOk() {
    if (typeof window !== 'undefined' && typeof window.engineSpectrumLive === 'function') {
        return window.engineSpectrumLive();
    }
    const minB = _vizEngineSpectrumMinBins();
    if (typeof window === 'undefined' || !window._engineSpectrumU8 || window._engineSpectrumU8.length < minB) {
        return false;
    }
    /* Library / floating player playback through JUCE (includes transport paused — spectrum still wired). */
    if (window._enginePlaybackActive === true) {
        return true;
    }
    /* Any engine output with FFT tap (Audio Engine tab: tone, file preview, etc.) */
    if (window._aeOutputStreamRunning === true) {
        return true;
    }
    return false;
}

function _vizOutputSampleRateHz() {
    if (
        typeof window !== 'undefined' &&
        typeof window._engineSpectrumSrHz === 'number' &&
        window._engineSpectrumSrHz > 0 &&
        (window._enginePlaybackActive === true || window._aeOutputStreamRunning === true)
    ) {
        return window._engineSpectrumSrHz;
    }
    if (typeof _playbackCtx !== 'undefined' && _playbackCtx && typeof _playbackCtx.sampleRate === 'number') {
        return _playbackCtx.sampleRate;
    }
    if (typeof window !== 'undefined' && typeof window._engineSpectrumSrHz === 'number' && window._engineSpectrumSrHz > 0) {
        return window._engineSpectrumSrHz;
    }
    return 44100;
}

/**
 * Minimal AnalyserNode stand-in backed by `window._engineSpectrumU8` (engine FFT tap).
 * Time-domain trace is synthetic from spectrum when using the shim; stereo/levels tiles use dedicated engine paths when spectrum is live.
 */
const _vizEngineShim = {
    get frequencyBinCount() {
        const u8 = typeof window !== 'undefined' ? window._engineSpectrumU8 : null;
        const minB = _vizEngineSpectrumMinBins();
        return u8 && u8.length >= minB ? u8.length : 1024;
    },
    get fftSize() {
        const fs = typeof window !== 'undefined' ? window._engineSpectrumFftSize : 0;
        return typeof fs === 'number' && fs > 0 ? fs : 2048;
    },
    getByteFrequencyData(arr) {
        const u8 = window._engineSpectrumU8;
        const minB = _vizEngineSpectrumMinBins();
        if (!u8 || u8.length < minB) {
            arr.fill(0);
            return;
        }
        const n = Math.min(arr.length, u8.length);
        arr.set(u8.subarray(0, n));
        if (n < arr.length) arr.fill(0, n);
    },
    getFloatTimeDomainData(arr) {
        const len = arr.length;
        const n = _vizEngineScopeLen();
        const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
        if (n >= 16 && sl && window._engineScopeR) {
            for (let i = 0; i < len; i++) {
                const t = (i / Math.max(1, len - 1)) * (n - 1);
                const i0 = Math.floor(t);
                const i1 = Math.min(n - 1, i0 + 1);
                const f = t - i0;
                const u = sl[i0] * (1 - f) + sl[i1] * f;
                arr[i] = (u - 128) / 128;
            }
            return;
        }
        const u8 = window._engineSpectrumU8;
        const minB = _vizEngineSpectrumMinBins();
        if (!u8 || u8.length < minB) {
            arr.fill(0);
            return;
        }
        let sum = 0;
        for (let i = 0; i < u8.length; i++) sum += u8[i];
        const avg = sum / (u8.length * 255);
        const pk =
            typeof window._enginePlaybackPeak === 'number' && !Number.isNaN(window._enginePlaybackPeak)
                ? window._enginePlaybackPeak
                : avg;
        const t0 = performance.now() * 0.0025;
        for (let i = 0; i < len; i++) {
            arr[i] = pk * 0.6 * Math.sin((i / len) * Math.PI * 24 + t0) * (0.85 + 0.15 * Math.sin(i * 0.07));
        }
    },
};

function _vizResolveAnalyser() {
    if (_vizEngineSpectrumOk()) return _vizEngineShim;
    // `engineSpectrumLive()` can be false briefly while bins exist (poll / flag ordering). Prefer the
    // engine-backed shim whenever we have a full frame so FFT/spectrogram/bands match stereo/levels.
    if (typeof window !== 'undefined' && window._engineSpectrumU8 && window._engineSpectrumU8.length >= _vizEngineSpectrumMinBins()) {
        return _vizEngineShim;
    }
    const web = typeof _analyser !== 'undefined' ? _analyser : null;
    return web;
}

/** Dark HUD backdrop + faint tech grid (replaces flat clear). */
function _vizHudBackdrop(ctx, w, h) {
    const bg = ctx.createLinearGradient(0, 0, 0, h);
    bg.addColorStop(0, 'rgba(2,10,26,0.98)');
    bg.addColorStop(0.45, 'rgba(6,4,18,0.96)');
    bg.addColorStop(1, 'rgba(0,0,0,0.94)');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, h);
    const step = Math.max(40, Math.floor(Math.min(w, h) / 10));
    ctx.strokeStyle = 'rgba(5,217,232,0.055)';
    ctx.lineWidth = 1;
    for (let x = 0; x <= w; x += step) {
        ctx.beginPath();
        ctx.moveTo(x + 0.5, 0);
        ctx.lineTo(x + 0.5, h);
        ctx.stroke();
    }
    for (let y = 0; y <= h; y += step) {
        ctx.beginPath();
        ctx.moveTo(0, y + 0.5);
        ctx.lineTo(w, y + 0.5);
        ctx.stroke();
    }
    ctx.strokeStyle = 'rgba(211,0,197,0.05)';
    ctx.beginPath();
    ctx.moveTo(0, h * 0.5 + 0.5);
    ctx.lineTo(w, h * 0.5 + 0.5);
    ctx.stroke();
}

/** Vertical neon gradient for spectrum-style bars (cyan → magenta by t∈[0,1]). */
function _vizNeonBarGradient(ctx, x, y0, y1, t) {
    const g = ctx.createLinearGradient(x, y0, x, y1);
    const r = Math.floor(5 + t * 206);
    const gg = Math.floor(217 - t * 167);
    const b = Math.floor(232 - t * 35);
    g.addColorStop(0, `rgba(${r},${gg},${b},1)`);
    g.addColorStop(0.55, `rgba(${r},${gg},${b},0.88)`);
    g.addColorStop(1, `rgba(${Math.min(255, r + 30)},${gg},${b},0.45)`);
    return g;
}

/** Rounded top-only bar (less blocky than fillRect). */
function _vizFillRoundTopBar(ctx, x, yTop, bw, bh, fillStyle) {
    if (bh <= 0.5 || bw <= 0) return;
    const r = Math.min(bh * 0.12, bw * 0.35, 6);
    ctx.fillStyle = fillStyle;
    ctx.beginPath();
    ctx.moveTo(x, yTop + bh);
    ctx.lineTo(x, yTop + r);
    ctx.quadraticCurveTo(x, yTop, x + r, yTop);
    ctx.lineTo(x + bw - r, yTop);
    ctx.quadraticCurveTo(x + bw, yTop, x + bw, yTop + r);
    ctx.lineTo(x + bw, yTop + bh);
    ctx.closePath();
    ctx.fill();
}

/**
 * First rising zero-crossing in a time-domain snapshot (AnalyserNode / engine shim).
 * Returns sample index of the first crossing, or -1 if none (silence / DC / noise floor).
 */
function _vizOscilloscopeTriggerIndex(data, bufLen) {
    for (let i = 1; i < bufLen; i++) {
        if (data[i - 1] < 0 && data[i] >= 0) return i;
    }
    return -1;
}

/** Cached canvas + 2D context per tile mode — avoids `querySelector` every frame. */
const _VIZ_TILE_MODES = ['fft', 'oscilloscope', 'spectrogram', 'stereo', 'levels', 'bands'];
const _vizTileCache = new Map();

function _refreshVizTileCache() {
    _vizTileCache.clear();
    for (const mode of _VIZ_TILE_MODES) {
        const canvas = document.querySelector(`.viz-tile-canvas[data-viz="${mode}"]`);
        if (canvas) _vizTileCache.set(mode, {canvas, ctx: canvas.getContext('2d')});
    }
}

// ── Mode switching ──
document.addEventListener('click', (e) => {
    const btn = e.target.closest('.viz-mode-btn');
    if (!btn || btn.dataset.action) return;
    const mode = btn.dataset.vizMode;
    if (!mode) return;

    document.querySelectorAll('.viz-mode-btn[data-viz-mode]').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    _vizMode = mode;
    _vizSpectrogramData = [];

    const grid = document.getElementById('vizGrid');
    if (!grid) return;
    if (mode === 'all') {
        grid.classList.remove('viz-single');
        grid.querySelectorAll('.viz-tile').forEach(t => t.classList.remove('viz-tile-active'));
    } else {
        grid.classList.add('viz-single');
        grid.querySelectorAll('.viz-tile').forEach(t => {
            t.classList.toggle('viz-tile-active', t.dataset.vizTile === mode);
        });
    }
    _resizeCanvases();
    _refreshVizTileCache();
    startVisualizer(); // restart loop if it died from a draw exception
});

// ── Fullscreen ──
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="vizFullscreen"]')) {
        const container = document.getElementById('vizContainer');
        if (!container) return;
        container.classList.toggle('viz-fullscreen');
        _resizeCanvases();
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        const container = document.getElementById('vizContainer');
        if (container?.classList.contains('viz-fullscreen')) {
            container.classList.remove('viz-fullscreen');
            _resizeCanvases();
        }
    }
});

function _resizeCanvases() {
    requestAnimationFrame(() => {
        const canvases = document.querySelectorAll('.viz-tile-canvas');
        const dpr = window.devicePixelRatio || 1;
        // Batch all reads first, then all writes (avoid layout thrashing)
        const sizes = [];
        for (const canvas of canvases) {
            const rect = canvas.getBoundingClientRect();
            sizes.push({
                canvas,
                w: Math.floor(rect.width * dpr),
                h: Math.floor(rect.height * dpr),
                ok: rect.width > 0 && rect.height > 0
            });
        }
        for (const {canvas, w, h, ok} of sizes) {
            if (ok) {
                canvas.width = w;
                canvas.height = h;
            }
        }
        _refreshVizTileCache();
    });
}

// ── Start/Stop ──
function _vizCancelScheduled() {
    if (_vizBackoffTimer != null) {
        clearTimeout(_vizBackoffTimer);
        _vizBackoffTimer = null;
    }
    if (_vizRAF != null) {
        cancelAnimationFrame(_vizRAF);
        _vizRAF = null;
    }
}

/** @param {number} delayMs 0 = next frame via rAF; >0 = setTimeout (throttle or idle poll). */
function _vizScheduleNext(delayMs) {
    _vizCancelScheduled();
    if (delayMs > 0) {
        _vizBackoffTimer = setTimeout(() => {
            _vizBackoffTimer = null;
            _vizRAF = requestAnimationFrame(_vizLoop);
        }, delayMs);
    } else {
        _vizRAF = requestAnimationFrame(_vizLoop);
    }
}

function startVisualizer() {
    _resizeCanvases();
    if (_vizRAF != null || _vizBackoffTimer != null) return;
    const empty = document.getElementById('vizEmpty');
    if (empty) empty.style.display = 'none';
    _vizScheduleNext(0);
}

function stopVisualizer() {
    _vizCancelScheduled();
    _vizLastFrame = 0;
}

if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
    document.addEventListener('graph-freeze-changed', () => {
        const tab = document.getElementById('tabVisualizer');
        if (tab && tab.classList.contains('active')) _vizScheduleNext(0);
    });
}

// ── Main render loop ──
function _vizLoop(timestamp) {
    _vizRAF = null;
    const tab = document.getElementById('tabVisualizer');
    if (!tab || !tab.classList.contains('active')) {
        /* Inactive tab uses `display:none` on `.tab-content`; stop rAF/throttle timers defensively if
         * this loop wakes without a matching `switchTab` teardown. */
        stopVisualizer();
        return;
    }
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) {
        /* Slow-poll instead of bare return so the loop self-recovers when idle
         * was a transient glitch (stale Tauri window-state poll, brief occlusion).
         * The `ui-idle-heavy-cpu` event listener also restarts, but a missed
         * false→true→false transition would otherwise kill the loop permanently. */
        _vizScheduleNext(500);
        return;
    }

    const ts = timestamp !== undefined ? timestamp : performance.now();

    const vizAnalyser = _vizResolveAnalyser();
    const isPlaying = typeof isAudioPlaying === 'function' ? isAudioPlaying() : typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused;
    const engineOut = typeof window !== 'undefined' && window._aeOutputStreamRunning === true;
    /* Engine tone/preview has no `<audio>` “playing” — still animate from AudioEngine spectrum.
     * Paused library transport still has engine FFT bins — keep tiles alive (independent of play/pause). */
    const vizActive = isPlaying || engineOut || _vizEngineSpectrumOk();
    const empty = document.getElementById('vizEmpty');
    if (empty) empty.style.display = vizAnalyser && vizActive ? 'none' : '';

    if (!vizAnalyser || !vizActive) {
        _vizScheduleNext(400);
        return;
    }

    // Throttle: 20fps in grid, 30fps in single (sub-interval wait uses setTimeout — not 60Hz rAF polls)
    const interval = _vizMode === 'all' ? (1000 / _VIZ_FPS_ALL) : (1000 / _VIZ_FPS_SINGLE);
    if (_vizLastFrame !== 0 && ts - _vizLastFrame < interval) {
        _vizScheduleNext(Math.max(1, Math.ceil(interval - (ts - _vizLastFrame))));
        return;
    }
    _vizLastFrame = ts;

    try {
        const modes =
            _vizMode === 'all'
                ? ['fft', 'oscilloscope', 'spectrogram', 'stereo', 'levels', 'bands']
                : [_vizMode];

        const needByte = modes.some(
            (m) => (m === 'fft' || m === 'spectrogram' || m === 'bands') && !_vizTileFrozen(m)
        );
        const needOscFloat = modes.includes('oscilloscope') && !_vizTileFrozen('oscilloscope');

        const nSc = _vizEngineScopeLen();
        const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
        const engineScopeStereo = nSc >= 32 && sl && window._engineScopeR && _vizEngineSpectrumOk();
        const minB = _vizEngineSpectrumMinBins();
        const specU8Ok =
            typeof window !== 'undefined' && window._engineSpectrumU8 && window._engineSpectrumU8.length >= minB;
        const engineSpecStereo = !!specU8Ok && _vizEngineSpectrumOk();

        const stereoNeedsWebFloat =
            modes.includes('stereo') && !_vizTileFrozen('stereo') && !engineScopeStereo && !engineSpecStereo;

        const levelsNeedsSharedFloat =
            modes.includes('levels') &&
            !_vizTileFrozen('levels') &&
            !_vizLevelsUsesEngineScopeLive() &&
            !_vizEngineSpectrumBufferForTile('levels');

        // Ensure pre-allocated buffers match analyser
        if (!_vizFreqData || _vizFreqData.length !== vizAnalyser.frequencyBinCount) {
            _vizFreqData = new Uint8Array(vizAnalyser.frequencyBinCount);
            _vizTimeData = new Float32Array(vizAnalyser.fftSize);
        }
        // Smoothing only affects frequency data, not time domain — set for FFT/bands (real Web Audio analyser only)
        if (vizAnalyser !== _vizEngineShim && typeof vizAnalyser.smoothingTimeConstant === 'number') {
            vizAnalyser.smoothingTimeConstant = _vizParams.fftSmoothing;
        }

        if (_vizMode === 'all') {
            if (needByte) {
                vizAnalyser.getByteFrequencyData(_vizFreqData);
            }
            const fftSz = vizAnalyser.fftSize;
            if (needOscFloat) {
                if (!_vizTimeData || _vizTimeData.length !== fftSz) _vizTimeData = new Float32Array(fftSz);
                vizAnalyser.getFloatTimeDomainData(_vizTimeData);
                if (!_vizOscFrozenCopy || _vizOscFrozenCopy.length !== fftSz) _vizOscFrozenCopy = new Float32Array(fftSz);
                _vizOscFrozenCopy.set(_vizTimeData);
            } else if (modes.includes('oscilloscope') && _vizTileFrozen('oscilloscope') && _vizOscFrozenCopy) {
                if (_vizOscFrozenCopy.length === fftSz) {
                    if (!_vizTimeData || _vizTimeData.length !== fftSz) _vizTimeData = new Float32Array(fftSz);
                    _vizTimeData.set(_vizOscFrozenCopy);
                }
            }
            if (levelsNeedsSharedFloat) {
                if (!_vizLevelsTimeData || _vizLevelsTimeData.length !== fftSz) _vizLevelsTimeData = new Float32Array(fftSz);
                vizAnalyser.getFloatTimeDomainData(_vizLevelsTimeData);
            }
            _drawTile('fft', vizAnalyser);
            _drawTile('oscilloscope', vizAnalyser);
            _drawTile('spectrogram', vizAnalyser);
            _drawTile('stereo', vizAnalyser);
            _drawTile('levels', vizAnalyser);
            _drawTile('bands', vizAnalyser);
        } else {
            const m = _vizMode;
            if ((m === 'fft' || m === 'spectrogram' || m === 'bands') && !_vizTileFrozen(m)) {
                vizAnalyser.getByteFrequencyData(_vizFreqData);
            }
            if (m === 'oscilloscope' && !_vizTileFrozen(m)) {
                const fftSz = vizAnalyser.fftSize;
                if (!_vizTimeData || _vizTimeData.length !== fftSz) _vizTimeData = new Float32Array(fftSz);
                vizAnalyser.getFloatTimeDomainData(_vizTimeData);
            }
            if (
                m === 'levels' &&
                !_vizTileFrozen(m) &&
                !_vizLevelsUsesEngineScopeLive() &&
                !_vizEngineSpectrumBufferForTile('levels')
            ) {
                const fftSz = vizAnalyser.fftSize;
                if (!_vizLevelsTimeData || _vizLevelsTimeData.length !== fftSz) _vizLevelsTimeData = new Float32Array(fftSz);
                vizAnalyser.getFloatTimeDomainData(_vizLevelsTimeData);
            }
            _drawTile(_vizMode, vizAnalyser);
        }
    } catch (_) {
        /* Draw error — absorb so the rAF loop self-recovers on the next frame. */
    }
    _vizScheduleNext(0);
}

function _drawTile(mode, analyser) {
    let entry = _vizTileCache.get(mode);
    if (!entry) {
        _refreshVizTileCache();
        entry = _vizTileCache.get(mode);
    }
    if (!entry) return;
    const {canvas, ctx} = entry;
    if (!canvas || canvas.width === 0 || !ctx) return;
    const w = canvas.width, h = canvas.height;

    try {
        switch (mode) {
            case 'fft':
                _drawFFT(ctx, w, h, analyser);
                break;
            case 'oscilloscope':
                _drawOscilloscope(ctx, w, h, analyser);
                break;
            case 'spectrogram':
                _drawSpectrogram(ctx, w, h, analyser);
                break;
            case 'stereo':
                _drawStereo(ctx, w, h, analyser);
                break;
            case 'levels':
                _drawLevels(ctx, w, h, analyser);
                break;
            case 'bands':
                _drawBands(ctx, w, h, analyser);
                break;
        }
    } catch (err) {
        console.warn(`[viz] draw error in "${mode}":`, err);
    }
}

// ── FFT Spectrum ──
function _drawFFT(ctx, w, h, analyser) {
    const bufLen = analyser.frequencyBinCount;
    if (!_vizFftFrozenCopy || _vizFftFrozenCopy.length !== bufLen) {
        _vizFftFrozenCopy = new Uint8Array(bufLen);
    }
       const paused = _vizTileFrozen('fft');
    if (!paused) {
        if (_vizMode !== 'all') {
            if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
            analyser.getByteFrequencyData(_vizFreqData);
        }
        _vizFftFrozenCopy.set(_vizFreqData);
    }
    const data = _vizFftFrozenCopy;
    _vizHudBackdrop(ctx, w, h);

    if (_vizParams.fftLogScale) {
        // Log-frequency display — cap columns so retina-wide canvases do not do O(width) pow/log per frame
        const sr = _vizOutputSampleRateHz();
        const minF = 20, maxF = Math.min(sr / 2, _VIZ_LOG_FMAX_HZ);
        const logMin = Math.log10(minF), logMax = Math.log10(maxF);
        const fftSize = analyser.fftSize;
        const maxCols = Math.min(w, 4096);
        const colW = w / maxCols;
        const eng = _vizEngineSpectrumOk();
        const binF =
            typeof window !== 'undefined' && eng && typeof window.engineSpectrumBundledBinF === 'function'
                ? (f) => window.engineSpectrumBundledBinF(f, sr, fftSize, bufLen)
                : (f) => (f * fftSize) / sr - (eng ? 1 : 0);
        for (let c = 0; c < maxCols; c++) {
            const t = (c + 0.5) / maxCols;
            const logF = logMin + t * (logMax - logMin);
            const freq = Math.pow(10, logF);
            const idx = binF(freq);
            const bin = Math.round(Math.max(0, Math.min(bufLen - 1, idx)));
            const barH = (data[bin] / 255) * h;
            const x = c * colW;
            const bw = Math.max(1, colW - 0.25);
            const grad = _vizNeonBarGradient(ctx, x, h - barH, h, t);
            _vizFillRoundTopBar(ctx, x, h - barH, bw, barH, grad);
        }
        // Frequency grid
        ctx.fillStyle = 'rgba(122,139,168,0.42)';
        ctx.font = `${Math.max(8, h / 40)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        [50, 100, 200, 500, 1000, 2000, 5000, 10000].forEach(f => {
            const x = ((Math.log10(f) - logMin) / (logMax - logMin)) * w;
            ctx.fillText(f >= 1000 ? (f / 1000) + 'k' : f + '', x, h - 2);
        });
    } else {
        const sr = _vizOutputSampleRateHz();
        const fftSize = analyser.fftSize;
        const nyq = sr * 0.5;
        const engine = _vizEngineSpectrumOk();
        for (let i = 0; i < bufLen; i++) {
            const barH = (data[i] / 255) * h;
            const k0 = engine ? i + 1 : i;
            const k1 = engine ? i + 2 : i + 1;
            const fL = Math.max(0, (k0 * sr) / fftSize);
            const fR = Math.min(nyq, (k1 * sr) / fftSize);
            const x = (fL / nyq) * w;
            const bw = Math.max(1, ((fR - fL) / nyq) * w - 0.5);
            const t = i / Math.max(1, bufLen - 1);
            const grad = _vizNeonBarGradient(ctx, x, h - barH, h, t);
            _vizFillRoundTopBar(ctx, x, h - barH, bw, barH, grad);
        }
    }
}

// ── Oscilloscope (time-domain trace) ──
function _drawOscilloscope(ctx, w, h, analyser) {
    const bufLen = analyser.fftSize;
    const frozen = _vizTileFrozen('oscilloscope');
    if (_vizMode !== 'all') {
        if (!frozen) {
            if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen);
            analyser.getFloatTimeDomainData(_vizTimeData);
            if (!_vizOscFrozenCopy || _vizOscFrozenCopy.length !== bufLen) _vizOscFrozenCopy = new Float32Array(bufLen);
            _vizOscFrozenCopy.set(_vizTimeData);
        } else if (_vizOscFrozenCopy && _vizOscFrozenCopy.length === bufLen) {
            if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen);
            _vizTimeData.set(_vizOscFrozenCopy);
        }
    } else if (!frozen && (!_vizTimeData || _vizTimeData.length !== bufLen)) {
        return;
    }
    const data = _vizTimeData;
    if (!data || data.length !== bufLen) {
        _vizHudBackdrop(ctx, w, h);
        return;
    }
    _vizHudBackdrop(ctx, w, h);

    const useTrig = _vizParams.oscilloscopeTriggeredSweep === true;
    let start = 0;
    if (useTrig) {
        let t0 = _vizOscilloscopeTriggerIndex(data, bufLen);
        if (t0 < 0) t0 = _vizOscLastTrigger;
        else _vizOscLastTrigger = t0;
        start = t0;
    }

    const color = _vizParams.oscilloscopeTraceColor === 'magenta' ? 'rgba(211,0,197,0.92)' :
        _vizParams.oscilloscopeTraceColor === 'green' ? 'rgba(57,255,20,0.9)' : 'rgba(5,217,232,0.92)';
    const glow = _vizParams.oscilloscopeTraceColor === 'magenta' ? 'rgba(211,0,197,0.35)' :
        _vizParams.oscilloscopeTraceColor === 'green' ? 'rgba(57,255,20,0.35)' : 'rgba(5,217,232,0.4)';
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    const sliceW = w / bufLen;
    for (let i = 0; i < bufLen; i++) {
        const idx = useTrig ? (start + i) % bufLen : i;
        const x = i * sliceW;
        const y = (0.5 - data[idx] * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.shadowColor = glow;
    ctx.shadowBlur = 10;
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Center line + grid
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = 'rgba(122,139,168,0.08)';
    ctx.beginPath();
    ctx.moveTo(0, h / 4);
    ctx.lineTo(w, h / 4);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, h * 3 / 4);
    ctx.lineTo(w, h * 3 / 4);
    ctx.stroke();
}

// ── Scrolling Spectrogram ──
function _drawSpectrogram(ctx, w, h, analyser) {
    const bufLen = analyser.frequencyBinCount;
    if (_vizMode !== 'all') {
        if (!_vizTileFrozen('spectrogram')) {
            if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
            analyser.getByteFrequencyData(_vizFreqData);
        }
    }
    const data = _vizFreqData;

    const maxCols = Math.floor(w / _vizParams.spectrogramSpeed);

    // Ring buffer: pre-allocate Uint8Arrays, overwrite in-place (no Array.from)
    if (_vizSpectrogramData.length !== maxCols) {
        _vizSpectrogramData = new Array(maxCols);
        for (let i = 0; i < maxCols; i++) _vizSpectrogramData[i] = new Uint8Array(bufLen);
        _vizSpectrogramIdx = 0;
    }
    const graphPaused = _vizTileFrozen('spectrogram');
    if (!graphPaused) {
        _vizSpectrogramData[_vizSpectrogramIdx].set(data);
        _vizSpectrogramIdx = (_vizSpectrogramIdx + 1) % maxCols;
    }

    _vizHudBackdrop(ctx, w, h);
    const colW = w / maxCols;
    const binStep = Math.max(1, Math.floor(bufLen / 256));
    const cw = Math.max(0.85, colW * 0.92);
    for (let col = 0; col < maxCols; col++) {
        const ringIdx = (_vizSpectrogramIdx + col) % maxCols;
        const cd = _vizSpectrogramData[ringIdx];
        if (!cd) continue;
        const x = col * colW + (colW - cw) * 0.5;
        for (let bin = 0; bin < bufLen; bin += binStep) {
            const mag = cd[bin] / 255;
            if (mag < 0.012) continue;
            const y = h - (bin / bufLen) * h;
            const binH = Math.max(0.85, Math.ceil((h / bufLen) * binStep));
            const r = Math.floor(mag * 211 + (1 - mag) * 5);
            const g = Math.floor(mag * mag * 55);
            const b = Math.floor(mag * 197 + (1 - mag) * 24);
            /* Solid fill — per-cell `createLinearGradient` was thousands of GPU objects per frame (long-run WebView/GPU pressure). */
            const a = mag * 0.62 + 0.1;
            ctx.fillStyle = `rgba(${r},${g},${b},${a})`;
            ctx.fillRect(x, y - binH, cw, binH);
        }
    }
}

// ── Stereo Field ──
// Pre-allocated stereo buffers
let _vizLeftData = null;
let _vizRightData = null;

function _drawStereo(ctx, w, h, analyser) {
    const {sl, sr, n: nSc} = _vizEngineScopeBuffersForTile('stereo');
    if (nSc >= 32 && sl && sr && _vizEngineSpectrumOk()) {
        _drawStereoEngineScope(ctx, w, h, sl, sr, nSc);
        return;
    }
    const u8 = _vizEngineSpectrumBufferForTile('stereo');
    if (u8) {
        _drawStereoEngine(ctx, w, h, u8);
        return;
    }
    const aL = window._analyserL;
    const aR = window._analyserR;
    if (!aL || !aR) {
        _vizHudBackdrop(ctx, w, h);
        ctx.fillStyle = 'rgba(122,139,168,0.55)';
        ctx.font = `${Math.max(10, Math.min(w, h) / 28)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }

    const bufLen = aL.fftSize;
    if (!_vizLeftData || _vizLeftData.length !== bufLen) {
        _vizLeftData = new Float32Array(bufLen);
        _vizRightData = new Float32Array(bufLen);
    }
    const stFrozen = _vizTileFrozen('stereo');
    if (!stFrozen) {
        aL.getFloatTimeDomainData(_vizLeftData);
        aR.getFloatTimeDomainData(_vizRightData);
        if (!_vizStereoWebFrozenL || _vizStereoWebFrozenL.length !== bufLen) {
            _vizStereoWebFrozenL = new Float32Array(bufLen);
            _vizStereoWebFrozenR = new Float32Array(bufLen);
        }
        _vizStereoWebFrozenL.set(_vizLeftData);
        _vizStereoWebFrozenR.set(_vizRightData);
    } else if (_vizStereoWebFrozenL && _vizStereoWebFrozenR && _vizStereoWebFrozenL.length === bufLen) {
        _vizLeftData.set(_vizStereoWebFrozenL);
        _vizRightData.set(_vizStereoWebFrozenR);
    }

    _vizHudBackdrop(ctx, w, h);
    const cx = w / 2, cy = h / 2;
    const scale = Math.min(cx, cy) * 0.8;

    // Grid
    ctx.strokeStyle = 'rgba(122,139,168,0.1)';
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 5]);
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = 'rgba(5,217,232,0.08)';
    ctx.beginPath();
    ctx.moveTo(cx - scale, cy - scale);
    ctx.lineTo(cx + scale, cy + scale);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(cx + scale, cy - scale);
    ctx.lineTo(cx - scale, cy + scale);
    ctx.stroke();

    // Plot true L vs R — soft phosphor dots (lighter composite)
    const prev = ctx.globalCompositeOperation;
    ctx.globalCompositeOperation = 'lighter';
    for (let i = 0; i < bufLen; i += 2) {
        const l = _vizLeftData[i];
        const r = _vizRightData[i];
        const mid = (l + r) * 0.5;
        const side = (l - r) * 0.5;
        const px = cx + side * scale;
        const py = cy - mid * scale;
        const a = 0.12 + Math.min(0.55, (Math.abs(l) + Math.abs(r)) * 0.35);
        const rad = 0.9 + Math.min(2.2, (Math.abs(mid) + Math.abs(side)) * 1.8);
        const grd = ctx.createRadialGradient(px, py, 0, px, py, rad);
        grd.addColorStop(0, `rgba(5,217,232,${a})`);
        grd.addColorStop(0.55, `rgba(211,0,197,${a * 0.45})`);
        grd.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = grd;
        ctx.beginPath();
        ctx.arc(px, py, rad, 0, Math.PI * 2);
        ctx.fill();
    }
    ctx.globalCompositeOperation = prev;

    ctx.fillStyle = 'rgba(122,139,168,0.5)';
    ctx.font = `${Math.max(9, h / 30)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText('L', 12, cy + 4);
    ctx.fillText('R', w - 12, cy + 4);
    ctx.fillText('MONO', cx, 14);
}

/**
 * AudioEngine spectrum is a mono FFT tap — no true L/R. Split each frequency slice into two
 * half-band energies so we get many (L,R) pairs and stack phosphor dots like the Web Audio path.
 * Radii scale with canvas size (single 1–3px dot was invisible on retina-sized bitmaps).
 */
function _drawStereoEngine(ctx, w, h, u8) {
    _vizHudBackdrop(ctx, w, h);
    const cx = w / 2;
    const cy = h / 2;
    const scale = Math.min(cx, cy) * 0.82;
    const minDim = Math.min(w, h);
    const baseR = Math.max(5, minDim * 0.014);

    ctx.strokeStyle = 'rgba(122,139,168,0.1)';
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 5]);
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = 'rgba(5,217,232,0.08)';
    ctx.beginPath();
    ctx.moveTo(cx - scale, cy - scale);
    ctx.lineTo(cx + scale, cy + scale);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(cx + scale, cy - scale);
    ctx.lineTo(cx - scale, cy + scale);
    ctx.stroke();

    const nSlices = Math.min(96, Math.max(24, Math.floor(u8.length / 8)));
    const sliceW = Math.max(4, Math.floor(u8.length / nSlices));
    const prev = ctx.globalCompositeOperation;
    ctx.globalCompositeOperation = 'lighter';

    for (let s = 0; s < nSlices; s++) {
        const i0 = s * sliceW;
        const i1 = Math.min(u8.length, i0 + sliceW);
        const mid = (i0 + i1) >> 1;
        let L = 0;
        let R = 0;
        for (let i = i0; i < mid; i++) L += u8[i];
        for (let i = mid; i < i1; i++) R += u8[i];
        const tot = L + R + 1e-3;
        const side = (L - R) / tot;
        const mag = tot / ((i1 - i0) * 255);
        const bandT = (s + 0.5) / nSlices - 0.5;
        const magVis = Math.min(1.2, mag * 2.4);
        const px = cx + side * scale * 0.55;
        const py = cy - bandT * scale * 0.55 * (0.35 + magVis * 1.05);
        const a = 0.16 + Math.min(0.55, magVis * 0.95);
        const rad = baseR + Math.min(minDim * 0.08, magVis * minDim * 0.1);
        const grd = ctx.createRadialGradient(px, py, 0, px, py, rad);
        grd.addColorStop(0, `rgba(5,217,232,${a})`);
        grd.addColorStop(0.55, `rgba(211,0,197,${a * 0.42})`);
        grd.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = grd;
        ctx.beginPath();
        ctx.arc(px, py, rad, 0, Math.PI * 2);
        ctx.fill();
    }

    ctx.globalCompositeOperation = prev;

    ctx.fillStyle = 'rgba(122,139,168,0.5)';
    ctx.font = `${Math.max(9, h / 30)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText('L', 12, cy + 4);
    ctx.fillText('R', w - 12, cy + 4);
    ctx.fillText('ENGINE', cx, 14);
}

/**
 * Vectorscope from real engine L/R samples (correlation is meaningful; mono fold → diagonal cloud).
 */
function _drawStereoEngineScope(ctx, w, h, sl, sr, nSc) {
    _vizHudBackdrop(ctx, w, h);
    const cx = w / 2;
    const cy = h / 2;
    const scale = Math.min(cx, cy) * 0.82;
    const minDim = Math.min(w, h);
    const baseR = Math.max(5, minDim * 0.014);

    ctx.strokeStyle = 'rgba(122,139,168,0.1)';
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 5]);
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.strokeStyle = 'rgba(5,217,232,0.08)';
    ctx.beginPath();
    ctx.moveTo(cx - scale, cy - scale);
    ctx.lineTo(cx + scale, cy + scale);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(cx + scale, cy - scale);
    ctx.lineTo(cx - scale, cy + scale);
    ctx.stroke();

    const prev = ctx.globalCompositeOperation;
    ctx.globalCompositeOperation = 'lighter';
    for (let i = 0; i < nSc; i += 2) {
        const l = (sl[i] - 128) / 128;
        const r = (sr[i] - 128) / 128;
        const mid = (l + r) * 0.5;
        const side = (l - r) * 0.5;
        const px = cx + side * scale;
        const py = cy - mid * scale;
        const a = 0.12 + Math.min(0.55, (Math.abs(l) + Math.abs(r)) * 0.35);
        const rad = 0.9 + Math.min(2.2, (Math.abs(mid) + Math.abs(side)) * 1.8);
        const grd = ctx.createRadialGradient(px, py, 0, px, py, rad);
        grd.addColorStop(0, `rgba(5,217,232,${a})`);
        grd.addColorStop(0.55, `rgba(211,0,197,${a * 0.45})`);
        grd.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = grd;
        ctx.beginPath();
        ctx.arc(px, py, rad, 0, Math.PI * 2);
        ctx.fill();
    }
    ctx.globalCompositeOperation = prev;

    const nn = Math.ceil(nSc / 2);
    let corrStr = '—';
    if (nn > 2) {
        const meanL = sumL / nn;
        const meanR = sumR / nn;
        let vL = 0;
        let vR = 0;
        let cLR = 0;
        for (let i = 0; i < nSc; i += 2) {
            const l = (sl[i] - 128) / 128;
            const r = (sr[i] - 128) / 128;
            const dl = l - meanL;
            const dr = r - meanR;
            vL += dl * dl;
            vR += dr * dr;
            cLR += dl * dr;
        }
        const den = Math.sqrt(vL * vR);
        if (den > 1e-8) {
            corrStr = (cLR / den).toFixed(2);
        }
    }

    ctx.fillStyle = 'rgba(122,139,168,0.5)';
    ctx.font = `${Math.max(9, h / 30)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText('L', 12, cy + 4);
    ctx.fillText('R', w - 12, cy + 4);
    ctx.fillText('ENGINE', cx, 14);
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.fillText(`corr ${corrStr}`, cx, h - 8);
}

// ── Level Meters ──
function _drawLevels(ctx, w, h, analyser) {
    const {sl, sr, n: nSc} = _vizEngineScopeBuffersForTile('levels');
    if (nSc >= 32 && sl && sr && _vizEngineSpectrumOk()) {
        _drawLevelsEngineScope(ctx, w, h, sl, sr, nSc);
        return;
    }
    const u8 = _vizEngineSpectrumBufferForTile('levels');
    if (u8) {
        _drawLevelsEngine(ctx, w, h, u8);
        return;
    }
    const bufLen = analyser.fftSize;
    if (_vizMode !== 'all') {
        if (!_vizTileFrozen('levels')) {
            if (!_vizLevelsTimeData || _vizLevelsTimeData.length !== bufLen) _vizLevelsTimeData = new Float32Array(bufLen);
            analyser.getFloatTimeDomainData(_vizLevelsTimeData);
        }
    } else if (!_vizLevelsTimeData || _vizLevelsTimeData.length !== bufLen) {
        _vizHudBackdrop(ctx, w, h);
        return;
    }
    const data = _vizLevelsTimeData;
    if (!data || data.length < 2) {
        _vizHudBackdrop(ctx, w, h);
        return;
    }

    let sumSq = 0, peak = 0;
    for (let i = 0; i < bufLen; i++) {
        sumSq += data[i] * data[i];
        const abs = Math.abs(data[i]);
        if (abs > peak) peak = abs;
    }
    const rms = Math.sqrt(sumSq / bufLen);
    const rmsDb = rms > 0 ? 20 * Math.log10(rms) : -96;
    const peakDb = peak > 0 ? 20 * Math.log10(peak) : -96;

    // Peak hold
    if (_vizParams.levelsHold) {
        if (peakDb > _vizPeakHold) {
            _vizPeakHold = peakDb;
            clearTimeout(_vizPeakTimer);
            _vizPeakTimer = setTimeout(() => {
                _vizPeakHold = -96;
            }, 2000);
        }
    }

    _vizHudBackdrop(ctx, w, h);
    const meterW = Math.min(80, w / 4);
    const meterH = h - 50;
    const startY = 25;
    const rr = 5;

    const drawMeter = (x, db, label) => {
        const pct = Math.max(0, Math.min(1, (db + 60) / 60));
        const barH = pct * meterH;
        ctx.fillStyle = 'rgba(6,8,22,0.75)';
        ctx.beginPath();
        ctx.moveTo(x + rr, startY);
        ctx.lineTo(x + meterW - rr, startY);
        ctx.quadraticCurveTo(x + meterW, startY, x + meterW, startY + rr);
        ctx.lineTo(x + meterW, startY + meterH - rr);
        ctx.quadraticCurveTo(x + meterW, startY + meterH, x + meterW - rr, startY + meterH);
        ctx.lineTo(x + rr, startY + meterH);
        ctx.quadraticCurveTo(x, startY + meterH, x, startY + meterH - rr);
        ctx.lineTo(x, startY + rr);
        ctx.quadraticCurveTo(x, startY, x + rr, startY);
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = 'rgba(5,217,232,0.22)';
        ctx.lineWidth = 1;
        ctx.stroke();
        if (barH > 1) {
            const y1 = startY + meterH - barH;
            const gbar = ctx.createLinearGradient(0, y1, 0, startY + meterH);
            gbar.addColorStop(0, 'rgba(57,255,20,0.92)');
            gbar.addColorStop(0.55, 'rgba(249,240,2,0.88)');
            gbar.addColorStop(0.82, 'rgba(255,107,53,0.88)');
            gbar.addColorStop(1, 'rgba(255,7,58,0.95)');
            _vizFillRoundTopBar(ctx, x + 2, y1, meterW - 4, barH, gbar);
        }
        ctx.fillStyle = 'rgba(224,240,255,0.88)';
        ctx.font = `${Math.max(10, h / 30)}px Orbitron, sans-serif`;
        ctx.textAlign = 'center';
        ctx.fillText(label, x + meterW / 2, startY - 6);
        ctx.font = `${Math.max(9, h / 35)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.fillText(db.toFixed(1) + ' dB', x + meterW / 2, startY + meterH + 16);
    };

    drawMeter(w / 2 - meterW - 15, rmsDb, 'RMS');
    drawMeter(w / 2 + 15, peakDb, 'PEAK');

    // Peak hold indicator
    if (_vizParams.levelsHold && _vizPeakHold > -96) {
        const holdPct = Math.max(0, Math.min(1, (_vizPeakHold + 60) / 60));
        const holdY = startY + meterH - holdPct * meterH;
        ctx.strokeStyle = 'rgba(255,7,58,0.9)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(w / 2 + 15, holdY);
        ctx.lineTo(w / 2 + 15 + meterW, holdY);
        ctx.stroke();
    }

    // dB scale
    ctx.fillStyle = 'rgba(122,139,168,0.38)';
    ctx.font = '8px "Share Tech Mono", ui-monospace, monospace';
    ctx.textAlign = 'right';
    for (let db = 0; db >= -60; db -= 6) {
        const y = startY + meterH * (1 - (db + 60) / 60);
        ctx.fillText(db + '', w / 2 - meterW - 20, y + 3);
    }
}

function _drawLevelsEngine(ctx, w, h, u8) {
    let sumSq = 0;
    for (let i = 0; i < u8.length; i++) {
        const n = u8[i] / 255;
        sumSq += n * n;
    }
    const rms = Math.sqrt(sumSq / u8.length);
    const rmsDb = rms > 0 ? 20 * Math.log10(rms) : -96;
    const pk =
        typeof window._enginePlaybackPeak === 'number' && !Number.isNaN(window._enginePlaybackPeak)
            ? window._enginePlaybackPeak
            : rms;
    const peakDb = pk > 0 ? 20 * Math.log10(pk) : -96;

    if (_vizParams.levelsHold) {
        if (peakDb > _vizPeakHold) {
            _vizPeakHold = peakDb;
            clearTimeout(_vizPeakTimer);
            _vizPeakTimer = setTimeout(() => {
                _vizPeakHold = -96;
            }, 2000);
        }
    }

    _vizHudBackdrop(ctx, w, h);
    const meterW = Math.min(80, w / 4);
    const meterH = h - 50;
    const startY = 25;
    const rr = 5;

    const drawMeter = (x, db, label) => {
        const pct = Math.max(0, Math.min(1, (db + 60) / 60));
        const barH = pct * meterH;
        ctx.fillStyle = 'rgba(6,8,22,0.75)';
        ctx.beginPath();
        ctx.moveTo(x + rr, startY);
        ctx.lineTo(x + meterW - rr, startY);
        ctx.quadraticCurveTo(x + meterW, startY, x + meterW, startY + rr);
        ctx.lineTo(x + meterW, startY + meterH - rr);
        ctx.quadraticCurveTo(x + meterW, startY + meterH, x + meterW - rr, startY + meterH);
        ctx.lineTo(x + rr, startY + meterH);
        ctx.quadraticCurveTo(x, startY + meterH, x, startY + meterH - rr);
        ctx.lineTo(x, startY + rr);
        ctx.quadraticCurveTo(x, startY, x + rr, startY);
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = 'rgba(5,217,232,0.22)';
        ctx.lineWidth = 1;
        ctx.stroke();
        if (barH > 1) {
            const y1 = startY + meterH - barH;
            const gbar = ctx.createLinearGradient(0, y1, 0, startY + meterH);
            gbar.addColorStop(0, 'rgba(57,255,20,0.92)');
            gbar.addColorStop(0.55, 'rgba(249,240,2,0.88)');
            gbar.addColorStop(0.82, 'rgba(255,107,53,0.88)');
            gbar.addColorStop(1, 'rgba(255,7,58,0.95)');
            _vizFillRoundTopBar(ctx, x + 2, y1, meterW - 4, barH, gbar);
        }
        ctx.fillStyle = 'rgba(224,240,255,0.88)';
        ctx.font = `${Math.max(10, h / 30)}px Orbitron, sans-serif`;
        ctx.textAlign = 'center';
        ctx.fillText(label, x + meterW / 2, startY - 6);
        ctx.font = `${Math.max(9, h / 35)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.fillText(db.toFixed(1) + ' dB', x + meterW / 2, startY + meterH + 16);
    };

    drawMeter(w / 2 - meterW - 15, rmsDb, 'RMS');
    drawMeter(w / 2 + 15, peakDb, 'PEAK');

    if (_vizParams.levelsHold && _vizPeakHold > -96) {
        const holdPct = Math.max(0, Math.min(1, (_vizPeakHold + 60) / 60));
        const holdY = startY + meterH - holdPct * meterH;
        ctx.strokeStyle = 'rgba(255,7,58,0.9)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(w / 2 + 15, holdY);
        ctx.lineTo(w / 2 + 15 + meterW, holdY);
        ctx.stroke();
    }

    ctx.fillStyle = 'rgba(122,139,168,0.38)';
    ctx.font = '8px "Share Tech Mono", ui-monospace, monospace';
    ctx.textAlign = 'right';
    for (let db = 0; db >= -60; db -= 6) {
        const y = startY + meterH * (1 - (db + 60) / 60);
        ctx.fillText(db + '', w / 2 - meterW - 20, y + 3);
    }
}

function _drawLevelsEngineScope(ctx, w, h, sl, sr, nSc) {
    let sumSq = 0;
    let peak = 0;
    for (let i = 0; i < nSc; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (sr[i] - 128) / 128;
        const m = 0.5 * (l + r);
        sumSq += m * m;
        peak = Math.max(peak, Math.abs(l), Math.abs(r));
    }
    const rms = Math.sqrt(sumSq / nSc);
    const rmsDb = rms > 0 ? 20 * Math.log10(rms) : -96;
    const peakDb = peak > 0 ? 20 * Math.log10(peak) : -96;

    if (_vizParams.levelsHold) {
        if (peakDb > _vizPeakHold) {
            _vizPeakHold = peakDb;
            clearTimeout(_vizPeakTimer);
            _vizPeakTimer = setTimeout(() => {
                _vizPeakHold = -96;
            }, 2000);
        }
    }

    _vizHudBackdrop(ctx, w, h);
    const meterW = Math.min(80, w / 4);
    const meterH = h - 50;
    const startY = 25;
    const rr = 5;

    const drawMeter = (x, db, label) => {
        const pct = Math.max(0, Math.min(1, (db + 60) / 60));
        const barH = pct * meterH;
        ctx.fillStyle = 'rgba(6,8,22,0.75)';
        ctx.beginPath();
        ctx.moveTo(x + rr, startY);
        ctx.lineTo(x + meterW - rr, startY);
        ctx.quadraticCurveTo(x + meterW, startY, x + meterW, startY + rr);
        ctx.lineTo(x + meterW, startY + meterH - rr);
        ctx.quadraticCurveTo(x + meterW, startY + meterH, x + meterW - rr, startY + meterH);
        ctx.lineTo(x + rr, startY + meterH);
        ctx.quadraticCurveTo(x, startY + meterH, x, startY + meterH - rr);
        ctx.lineTo(x, startY + rr);
        ctx.quadraticCurveTo(x, startY, x + rr, startY);
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = 'rgba(5,217,232,0.22)';
        ctx.lineWidth = 1;
        ctx.stroke();
        if (barH > 1) {
            const y1 = startY + meterH - barH;
            const gbar = ctx.createLinearGradient(0, y1, 0, startY + meterH);
            gbar.addColorStop(0, 'rgba(57,255,20,0.92)');
            gbar.addColorStop(0.55, 'rgba(249,240,2,0.88)');
            gbar.addColorStop(0.82, 'rgba(255,107,53,0.88)');
            gbar.addColorStop(1, 'rgba(255,7,58,0.95)');
            _vizFillRoundTopBar(ctx, x + 2, y1, meterW - 4, barH, gbar);
        }
        ctx.fillStyle = 'rgba(224,240,255,0.88)';
        ctx.font = `${Math.max(10, h / 30)}px Orbitron, sans-serif`;
        ctx.textAlign = 'center';
        ctx.fillText(label, x + meterW / 2, startY - 6);
        ctx.font = `${Math.max(9, h / 35)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.fillText(db.toFixed(1) + ' dB', x + meterW / 2, startY + meterH + 16);
    };

    drawMeter(w / 2 - meterW - 15, rmsDb, 'RMS');
    drawMeter(w / 2 + 15, peakDb, 'PEAK');

    if (_vizParams.levelsHold && _vizPeakHold > -96) {
        const holdPct = Math.max(0, Math.min(1, (_vizPeakHold + 60) / 60));
        const holdY = startY + meterH - holdPct * meterH;
        ctx.strokeStyle = 'rgba(255,7,58,0.9)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(w / 2 + 15, holdY);
        ctx.lineTo(w / 2 + 15 + meterW, holdY);
        ctx.stroke();
    }

    ctx.fillStyle = 'rgba(122,139,168,0.38)';
    ctx.font = '8px "Share Tech Mono", ui-monospace, monospace';
    ctx.textAlign = 'right';
    for (let db = 0; db >= -60; db -= 6) {
        const y = startY + meterH * (1 - (db + 60) / 60);
        ctx.fillText(db + '', w / 2 - meterW - 20, y + 3);
    }
}

// ── Octave Bands ──
function _drawBands(ctx, w, h, analyser) {
    const bufLen = analyser.frequencyBinCount;
    if (!_vizBandsFrozenCopy || _vizBandsFrozenCopy.length !== bufLen) {
        _vizBandsFrozenCopy = new Uint8Array(bufLen);
    }
    if (_vizMode !== 'all') {
        if (!_vizTileFrozen('bands')) {
            if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
            analyser.getByteFrequencyData(_vizFreqData);
            _vizBandsFrozenCopy.set(_vizFreqData);
        }
    } else if (!_vizTileFrozen('bands')) {
        _vizBandsFrozenCopy.set(_vizFreqData);
    }
    const data = _vizBandsFrozenCopy;

    const sr = _vizOutputSampleRateHz();
    const fftSize = analyser.fftSize;
    const engineBands = _vizEngineSpectrumOk();
    const bands = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    const labels = ['31', '63', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];

    _vizHudBackdrop(ctx, w, h);
    const bandW = (w - 30) / bands.length;
    const maxH = h - 35;

    for (let i = 0; i < bands.length; i++) {
        const cf = bands[i];
        let lo;
        let hi;
        if (engineBands && typeof window !== 'undefined' && typeof window.engineSpectrumBundledBinF === 'function') {
            const bLo = window.engineSpectrumBundledBinF(cf / Math.sqrt(2), sr, fftSize, bufLen);
            const bHi = window.engineSpectrumBundledBinF(cf * Math.sqrt(2), sr, fftSize, bufLen);
            lo = Math.max(0, Math.floor(Math.min(bLo, bHi)));
            hi = Math.min(bufLen - 1, Math.ceil(Math.max(bLo, bHi)));
        } else {
            const adj = engineBands ? 1 : 0;
            lo = Math.floor(((cf / Math.sqrt(2)) * fftSize) / sr - adj);
            hi = Math.ceil(((cf * Math.sqrt(2)) * fftSize) / sr - adj);
        }
        let sum = 0, cnt = 0;
        for (let k = Math.max(0, lo); k <= Math.min(hi, bufLen - 1); k++) {
            sum += data[k];
            cnt++;
        }
        const avg = cnt > 0 ? sum / cnt : 0;
        const barH = (avg / 255) * maxH;
        const x = 15 + i * bandW;
        const t = i / bands.length;
        const bw = bandW - 6;
        const bx = x + 3;
        const by = h - 20 - barH;
        const grad = _vizNeonBarGradient(ctx, bx, by, h - 20, t);
        _vizFillRoundTopBar(ctx, bx, by, bw, barH, grad);

        ctx.fillStyle = 'rgba(224,240,255,0.65)';
        ctx.font = `${Math.max(8, h / 35)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText(labels[i], x + bandW / 2, h - 5);
    }
}

// ── Context menus for visualizer tiles (export / copy / fullscreen + mode tools — single handler; was duplicated in context-menu.js) ──
const _vizMenuNoEcho = {skipEchoToast: true};
function _vizShortcutTip(id) {
    return typeof shortcutTip === 'function' ? shortcutTip(id) : {};
}
document.addEventListener('contextmenu', (e) => {
    const tile = e.target.closest('.viz-tile');
    if (!tile) return;
    e.preventDefault();
    const mode = tile.dataset.vizTile;
    const label = tile.querySelector('.viz-tile-label')?.textContent?.trim() || appFmt('menu.tab_visualizer');
    const canvas = tile.querySelector('canvas');
    const gf = _vizFreezeIdForTileMode(mode);
    const graphAnimOn = !(gf && typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(gf));
    const items = [
        {
            icon: '&#128247;', label: appFmt('menu.export_snapshot_png'), action: () => {
                if (canvas && typeof exportCanvasSnapshotPng === 'function') {
                    void exportCanvasSnapshotPng(canvas, label);
                }
            }, disabled: !canvas
        },
        {
            icon: '&#128203;',
            label: appFmt('menu.copy_tile_name'), ..._vizMenuNoEcho, ..._vizShortcutTip('copyPath'),
            action: () => typeof copyToClipboard === 'function' && copyToClipboard(label)
        },
        '---',
        {
            icon: '&#128260;', label: appFmt('menu.toggle_fullscreen'), action: () => {
                tile.classList.toggle('viz-fullscreen');
                if (tile.classList.contains('viz-fullscreen')) {
                    tile.requestFullscreen?.().catch(err => {
                        if (typeof showToast === 'function') showToast(String(err), 4000, 'error');
                    });
                } else {
                    document.exitFullscreen?.().catch(err => {
                        if (typeof showToast === 'function') showToast(String(err), 4000, 'error');
                    });
                }
            }
        },
        '---',
        {
            icon: '&#9974;', label: appFmt('menu.viz_view_fullscreen'), action: () => {
                _vizMode = mode;
                document.querySelector(`.viz-mode-btn[data-viz-mode="${mode}"]`)?.click();
                document.querySelector('[data-action="vizFullscreen"]')?.click();
            }
        },
        {
            icon: '&#9650;',
            label: appFmt('menu.viz_show_only_this'),
            action: () => document.querySelector(`.viz-mode-btn[data-viz-mode="${mode}"]`)?.click()
        },
        {
            icon: '&#9632;',
            label: appFmt('menu.viz_show_all'),
            action: () => document.querySelector('.viz-mode-btn[data-viz-mode="all"]')?.click()
        },
        '---',
        {
            icon: graphAnimOn ? '&#10003;' : '&#9634;',
            label: graphAnimOn
                ? appFmt('menu.viz_graph_freeze')
                : appFmt('menu.viz_graph_unfreeze'),
            action: () => {
                if (gf && typeof window.toggleGraphFrozen === 'function') window.toggleGraphFrozen(gf);
            },
            ..._vizMenuNoEcho,
        },
    ];

    if (mode === 'fft') {
        items.push({
            icon: _vizParams.fftLogScale ? '&#10003;' : '&#9634;',
            label: appFmt('menu.viz_log_frequency_scale'),
            action: () => {
                _vizParams.fftLogScale = !_vizParams.fftLogScale;
            }
        });
    }
    if (mode === 'oscilloscope') {
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_cyan'), action: () => {
                _vizParams.oscilloscopeTraceColor = 'cyan';
            }
        });
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_magenta'), action: () => {
                _vizParams.oscilloscopeTraceColor = 'magenta';
            }
        });
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_green'), action: () => {
                _vizParams.oscilloscopeTraceColor = 'green';
            }
        });
        items.push({
            icon: _vizParams.oscilloscopeTriggeredSweep ? '&#10003;' : '&#9634;',
            label: appFmt('menu.viz_oscilloscope_triggered_sweep'),
            action: () => {
                _vizParams.oscilloscopeTriggeredSweep = !_vizParams.oscilloscopeTriggeredSweep;
            },
            ..._vizMenuNoEcho,
        });
    }
    if (mode === 'levels') {
        items.push({
            icon: _vizParams.levelsHold ? '&#10003;' : '&#9634;',
            label: appFmt('menu.viz_peak_hold'),
            action: () => {
                _vizParams.levelsHold = !_vizParams.levelsHold;
                _vizPeakHold = -96;
            }
        });
    }
    if (mode === 'spectrogram') {
        items.push({
            icon: '&#9654;', label: appFmt('menu.viz_speed_normal'), action: () => {
                _vizParams.spectrogramSpeed = 1;
                _vizSpectrogramData = [];
            }
        });
        items.push({
            icon: '&#9654;&#9654;', label: appFmt('menu.viz_speed_fast'), action: () => {
                _vizParams.spectrogramSpeed = 2;
                _vizSpectrogramData = [];
            }
        });
        items.push({
            icon: '&#9654;&#9654;&#9654;', label: appFmt('menu.viz_speed_slow'), action: () => {
                _vizParams.spectrogramSpeed = 0.5;
                _vizSpectrogramData = [];
            }
        });
    }

    if (typeof showContextMenu === 'function') showContextMenu(e, items);
});

// ── Trello drag to rearrange tiles ──
document.addEventListener('DOMContentLoaded', () => {
    if (typeof prefs !== 'undefined' && typeof prefs.getObject === 'function' && typeof prefs.setItem === 'function') {
        const saved = prefs.getObject('vizTileOrder', null);
        if (Array.isArray(saved) && saved.includes('waveform')) {
            prefs.setItem(
                'vizTileOrder',
                saved.map((k) => (k === 'waveform' ? 'oscilloscope' : k))
            );
        }
    }
    const grid = document.getElementById('vizGrid');
    if (grid && typeof initDragReorder === 'function') {
        initDragReorder(grid, '.viz-tile', 'vizTileOrder', {
            getKey: (el) => el.dataset.vizTile || '',
        });
    }
});

// ── Auto start/stop ──
document.addEventListener('click', (e) => {
    const tab = e.target.closest('[data-action="switchTab"]');
    if (tab && tab.dataset.tab === 'visualizer') startVisualizer();
    if (e.target.closest('[data-action="toggleAudioPlayback"], [data-action="previewAudio"], [data-action="playRecent"]')) {
        setTimeout(() => {
            const vizTab = document.getElementById('tabVisualizer');
            if (vizTab && vizTab.classList.contains('active')) startVisualizer();
        }, 100);
    }
});

// Resize canvases when window resizes (debounced — canvas reset is expensive)
window.addEventListener('resize', typeof debounce === 'function' ? debounce(_resizeCanvases, 150) : _resizeCanvases);

if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        const idle = e.detail && e.detail.idle;
        if (!idle) {
            /* Idle ended — force immediate full-speed restart (cancel any pending 500 ms slow-poll
             * so `startVisualizer`'s guard sees both IDs null and re-enters the loop). */
            const t = document.getElementById('tabVisualizer');
            if (t && t.classList.contains('active') && typeof startVisualizer === 'function') {
                stopVisualizer();
                startVisualizer();
            }
        }
        /* idle === true: _vizLoop self-throttles to 500 ms; no need to kill the loop here.
         * Leaving the slow poll alive lets the loop self-recover if the idle state was
         * transient and the event system missed the false→true→false cycle. */
    });
}
