// ── Audio Visualizer Tab ──
// 6 real-time displays: FFT, waveform, spectrogram, stereo, levels, bands.
// Primary data: audio-engine `playback_status.spectrum` (see `applyPlaybackStatusSpectrum` in
// audio-engine.js). Web Audio `AnalyserNode` is only used when the sidecar is not supplying spectrum.
// Grid (all) or single mode. Fullscreen. Trello drag to rearrange. Context menus.

let _vizMode = 'all';
let _vizRAF = null;
let _vizSpectrogramData = [];
let _vizSpectrogramIdx = 0; // ring buffer write index
let _vizLastFrame = 0;
const _VIZ_FPS_SINGLE = 30;
const _VIZ_FPS_ALL = 20;
// Pre-allocated buffers (set once when analyser is available)
let _vizFreqData = null;
let _vizTimeData = null;
let _vizParams = {
    fftSmoothing: 0.8,
    fftLogScale: true,
    waveformColor: 'cyan',
    spectrogramSpeed: 1,
    levelsHold: true,
    bandsCount: 10,
};
let _vizPeakHold = -96;
let _vizPeakTimer = null;

/** Sidecar is sending usable spectrum bins (`playback_status.spectrum`). */
function _vizEngineSpectrumOk() {
    if (typeof window === 'undefined' || !window._engineSpectrumU8 || window._engineSpectrumU8.length < 1024) {
        return false;
    }
    /* Library / floating player playback through JUCE */
    if (window._enginePlaybackActive === true && typeof isAudioPlaying === 'function' && isAudioPlaying()) {
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
 * Waveform is synthetic; stereo/levels tiles use dedicated engine paths when spectrum is live.
 */
const _vizEngineShim = {
    frequencyBinCount: 1024,
    fftSize: 2048,
    getByteFrequencyData(arr) {
        const u8 = window._engineSpectrumU8;
        if (!u8 || u8.length < 1024) {
            arr.fill(0);
            return;
        }
        const n = Math.min(arr.length, u8.length);
        arr.set(u8.subarray(0, n));
        if (n < arr.length) arr.fill(0, n);
    },
    getFloatTimeDomainData(arr) {
        const u8 = window._engineSpectrumU8;
        const len = arr.length;
        if (!u8 || u8.length < 1024) {
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

/** Cached canvas + 2D context per tile mode — avoids `querySelector` every frame. */
const _VIZ_TILE_MODES = ['fft', 'waveform', 'spectrogram', 'stereo', 'levels', 'bands'];
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
function startVisualizer() {
    _resizeCanvases();
    if (_vizRAF) return;
    const empty = document.getElementById('vizEmpty');
    if (empty) empty.style.display = 'none';
    _vizLoop();
}

function stopVisualizer() {
    if (_vizRAF) {
        cancelAnimationFrame(_vizRAF);
        _vizRAF = null;
    }
}

// ── Main render loop ──
function _vizLoop(timestamp) {
    const tab = document.getElementById('tabVisualizer');
    if (!tab || !tab.classList.contains('active')) {
        _vizRAF = null;
        return;
    }

    _vizRAF = requestAnimationFrame(_vizLoop);

    // Throttle: 20fps in grid, 30fps in single
    const interval = _vizMode === 'all' ? (1000 / _VIZ_FPS_ALL) : (1000 / _VIZ_FPS_SINGLE);
    if (timestamp - _vizLastFrame < interval) return;
    _vizLastFrame = timestamp;

    const vizAnalyser = _vizResolveAnalyser();
    const isPlaying = typeof isAudioPlaying === 'function' ? isAudioPlaying() : typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused;
    const engineOut = typeof window !== 'undefined' && window._aeOutputStreamRunning === true;
    /* Engine tone/preview has no `<audio>` “playing” — still animate from sidecar spectrum. */
    const vizActive = isPlaying || engineOut;
    const empty = document.getElementById('vizEmpty');
    if (empty) empty.style.display = vizAnalyser && vizActive ? 'none' : '';

    if (!vizAnalyser || !vizActive) return;

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
        // Read data once, share across all tiles
        vizAnalyser.getByteFrequencyData(_vizFreqData);
        vizAnalyser.getFloatTimeDomainData(_vizTimeData);
        _drawTile('fft', vizAnalyser);
        _drawTile('waveform', vizAnalyser);
        _drawTile('spectrogram', vizAnalyser);
        _drawTile('stereo', vizAnalyser);
        _drawTile('levels', vizAnalyser);
        _drawTile('bands', vizAnalyser);
    } else {
        _drawTile(_vizMode, vizAnalyser);
    }
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

    switch (mode) {
        case 'fft':
            _drawFFT(ctx, w, h, analyser);
            break;
        case 'waveform':
            _drawWaveform(ctx, w, h, analyser);
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
}

// ── FFT Spectrum ──
function _drawFFT(ctx, w, h, analyser) {
    const bufLen = analyser.frequencyBinCount;
    if (_vizMode !== 'all') {
        if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
        analyser.getByteFrequencyData(_vizFreqData);
    }
    const data = _vizFreqData;
    _vizHudBackdrop(ctx, w, h);

    if (_vizParams.fftLogScale) {
        // Log-frequency display — cap columns so retina-wide canvases do not do O(width) pow/log per frame
        const minF = 20, maxF = 20000;
        const logMin = Math.log10(minF), logMax = Math.log10(maxF);
        const sr = _vizOutputSampleRateHz();
        const maxCols = Math.min(w, 1024);
        const colW = w / maxCols;
        for (let c = 0; c < maxCols; c++) {
            const t = (c + 0.5) / maxCols;
            const logF = logMin + t * (logMax - logMin);
            const freq = Math.pow(10, logF);
            const bin = Math.round((freq / (sr / 2)) * bufLen);
            if (bin >= bufLen) continue;
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
        const barW = w / bufLen;
        for (let i = 0; i < bufLen; i++) {
            const barH = (data[i] / 255) * h;
            const t = i / bufLen;
            const x = i * barW;
            const grad = _vizNeonBarGradient(ctx, x, h - barH, h, t);
            _vizFillRoundTopBar(ctx, x, h - barH, barW - 0.5, barH, grad);
        }
    }
}

// ── Waveform / Oscilloscope ──
function _drawWaveform(ctx, w, h, analyser) {
    const bufLen = analyser.fftSize;
    if (_vizMode !== 'all') {
        if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen);
        analyser.getFloatTimeDomainData(_vizTimeData);
    }
    const data = _vizTimeData;
    _vizHudBackdrop(ctx, w, h);

    const color = _vizParams.waveformColor === 'magenta' ? 'rgba(211,0,197,0.92)' :
        _vizParams.waveformColor === 'green' ? 'rgba(57,255,20,0.9)' : 'rgba(5,217,232,0.92)';
    const glow = _vizParams.waveformColor === 'magenta' ? 'rgba(211,0,197,0.35)' :
        _vizParams.waveformColor === 'green' ? 'rgba(57,255,20,0.35)' : 'rgba(5,217,232,0.4)';
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    const sliceW = w / bufLen;
    for (let i = 0; i < bufLen; i++) {
        const x = i * sliceW;
        const y = (0.5 - data[i] * 0.5) * h;
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
        if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
        analyser.getByteFrequencyData(_vizFreqData);
    }
    const data = _vizFreqData;

    const maxCols = Math.floor(w / _vizParams.spectrogramSpeed);

    // Ring buffer: pre-allocate Uint8Arrays, overwrite in-place (no Array.from)
    if (_vizSpectrogramData.length !== maxCols) {
        _vizSpectrogramData = new Array(maxCols);
        for (let i = 0; i < maxCols; i++) _vizSpectrogramData[i] = new Uint8Array(bufLen);
        _vizSpectrogramIdx = 0;
    }
    _vizSpectrogramData[_vizSpectrogramIdx].set(data);
    _vizSpectrogramIdx = (_vizSpectrogramIdx + 1) % maxCols;

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
            const t = bin / bufLen;
            const gcol = ctx.createLinearGradient(x, y - binH, x + cw, y);
            gcol.addColorStop(0, `rgba(${r},${g},${b},${mag * 0.55 + 0.08})`);
            gcol.addColorStop(0.5, `rgba(${Math.min(255, r + 40)},${g},${b},${mag * 0.75 + 0.1})`);
            gcol.addColorStop(1, `rgba(${r},${g},${b},${mag * 0.35})`);
            ctx.fillStyle = gcol;
            ctx.fillRect(x, y - binH, cw, binH);
        }
    }
}

// ── Stereo Field ──
// Pre-allocated stereo buffers
let _vizLeftData = null;
let _vizRightData = null;

function _drawStereo(ctx, w, h, analyser) {
    if (_vizEngineSpectrumOk()) {
        _drawStereoEngine(ctx, w, h, window._engineSpectrumU8);
        return;
    }
    const aL = window._analyserL;
    const aR = window._analyserR;
    if (!aL || !aR) return;

    const bufLen = aL.fftSize;
    if (!_vizLeftData || _vizLeftData.length !== bufLen) {
        _vizLeftData = new Float32Array(bufLen);
        _vizRightData = new Float32Array(bufLen);
    }
    aL.getFloatTimeDomainData(_vizLeftData);
    aR.getFloatTimeDomainData(_vizRightData);

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
 * Sidecar spectrum is a mono FFT tap — no true L/R. Split each frequency slice into two
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
        const px = cx + side * scale * 0.48;
        const py = cy - bandT * scale * 0.55 * (0.35 + mag * 1.15);
        const a = 0.1 + Math.min(0.52, mag * 1.1);
        const rad = baseR + Math.min(minDim * 0.07, mag * minDim * 0.09);
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

// ── Level Meters ──
function _drawLevels(ctx, w, h, analyser) {
    if (_vizEngineSpectrumOk()) {
        _drawLevelsEngine(ctx, w, h, window._engineSpectrumU8);
        return;
    }
    const bufLen = analyser.fftSize;
    if (_vizMode !== 'all') {
        if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen);
        analyser.getFloatTimeDomainData(_vizTimeData);
    }
    const data = _vizTimeData;

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

// ── Octave Bands ──
function _drawBands(ctx, w, h, analyser) {
    const bufLen = analyser.frequencyBinCount;
    if (_vizMode !== 'all') {
        if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen);
        analyser.getByteFrequencyData(_vizFreqData);
    }
    const data = _vizFreqData;

    const sr = _vizOutputSampleRateHz();
    const binFreq = sr / analyser.fftSize;
    const bands = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    const labels = ['31', '63', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];

    _vizHudBackdrop(ctx, w, h);
    const bandW = (w - 30) / bands.length;
    const maxH = h - 35;

    for (let i = 0; i < bands.length; i++) {
        const cf = bands[i];
        const lo = Math.floor((cf / Math.sqrt(2)) / binFreq);
        const hi = Math.ceil((cf * Math.sqrt(2)) / binFreq);
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
document.addEventListener('contextmenu', (e) => {
    const tile = e.target.closest('.viz-tile');
    if (!tile) return;
    e.preventDefault();
    const mode = tile.dataset.vizTile;
    const label = tile.querySelector('.viz-tile-label')?.textContent?.trim() || appFmt('menu.tab_visualizer');
    const canvas = tile.querySelector('canvas');
    const items = [
        {
            icon: '&#128247;', label: appFmt('menu.export_snapshot_png'), action: () => {
                if (canvas) {
                    const link = document.createElement('a');
                    link.download = `${label.replace(/\s+/g, '_').toLowerCase()}_${Date.now()}.png`;
                    link.href = canvas.toDataURL('image/png');
                    link.click();
                    if (typeof showToast === 'function') showToast(toastFmt('toast.snapshot_exported'));
                }
            }, disabled: !canvas
        },
        {
            icon: '&#128203;',
            label: appFmt('menu.copy_tile_name'), ..._vizMenuNoEcho,
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
    if (mode === 'waveform') {
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_cyan'), action: () => {
                _vizParams.waveformColor = 'cyan';
            }
        });
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_magenta'), action: () => {
                _vizParams.waveformColor = 'magenta';
            }
        });
        items.push({
            icon: '&#127912;', label: appFmt('menu.viz_color_green'), action: () => {
                _vizParams.waveformColor = 'green';
            }
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
