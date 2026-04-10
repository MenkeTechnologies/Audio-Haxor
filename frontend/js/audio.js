// ── Audio Samples (paginated via SQLite) ──
/** Same directory as this script (for `audio-decode-worker.js`); set while this file executes. */
const _AUDIO_JS_BASE =
    typeof document !== 'undefined' && document.currentScript && document.currentScript.src
        ? document.currentScript.src
        : '';

function _audioDecodeWorkerScriptUrl() {
    if (_AUDIO_JS_BASE) {
        try {
            return new URL('audio-decode-worker.js', _AUDIO_JS_BASE).href;
        } catch (_) {
            /* fall through */
        }
    }
    try {
        const base = typeof location !== 'undefined' && location.href ? location.href : '';
        return base ? new URL('js/audio-decode-worker.js', base).href : 'js/audio-decode-worker.js';
    } catch (_) {
        return 'js/audio-decode-worker.js';
    }
}

let _audioDecodeWorker = null;
let _audioDecodeWorkerJobId = 0;
const _audioDecodeWorkerPending = new Map();

function _audioDecodeWorkerOnMessage(ev) {
    const d = ev.data;
    if (!d || typeof d.id !== 'number') return;
    const pending = _audioDecodeWorkerPending.get(d.id);
    if (!pending) return;
    _audioDecodeWorkerPending.delete(d.id);
    if (d.ok) pending.resolve(d);
    else pending.reject(new Error(d.error || 'worker decode failed'));
}

function getAudioDecodeWorker() {
    if (typeof Worker !== 'function') return null;
    if (_audioDecodeWorker) return _audioDecodeWorker;
    try {
        _audioDecodeWorker = new Worker(_audioDecodeWorkerScriptUrl());
        _audioDecodeWorker.onmessage = _audioDecodeWorkerOnMessage;
        _audioDecodeWorker.onerror = () => {
            for (const [, p] of _audioDecodeWorkerPending) {
                p.reject(new Error('audio decode worker failed'));
            }
            _audioDecodeWorkerPending.clear();
            try {
                _audioDecodeWorker.terminate();
            } catch (_) {
                /* ignore */
            }
            _audioDecodeWorker = null;
        };
    } catch (_) {
        _audioDecodeWorker = null;
    }
    return _audioDecodeWorker;
}

function postAudioDecodeWorker(payload, transfer) {
    const w = getAudioDecodeWorker();
    if (!w) return Promise.reject(new Error('no worker'));
    return new Promise((resolve, reject) => {
        const id = ++_audioDecodeWorkerJobId;
        _audioDecodeWorkerPending.set(id, { resolve, reject });
        try {
            w.postMessage({ ...payload, id }, transfer || []);
        } catch (e) {
            _audioDecodeWorkerPending.delete(id);
            reject(e);
        }
    });
}

/** Main-thread fetch of file bytes (async I/O only); decode runs in [`audio-decode-worker.js`]. */
async function fetchAudioArrayBuffer(url) {
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`fetch ${resp.status}`);
    return await resp.arrayBuffer();
}

async function decodePeaksFromArrayBuffer(ab, bars) {
    const res = await postAudioDecodeWorker({ type: 'peaksFromBuffer', ab, bars }, [ab]);
    return res.peaks;
}

async function decodeMetaFromArrayBuffer(ab, bars) {
    const res = await postAudioDecodeWorker({ type: 'metaFromBuffer', ab, bars }, [ab]);
    return { peaks: res.peaks, sgData: res.sgData };
}

async function decodeSpectrogramFromArrayBuffer(ab) {
    const res = await postAudioDecodeWorker({ type: 'spectrogramFromBuffer', ab, bars: 0 }, [ab]);
    return res.sgData;
}

async function decodeChannelsFromArrayBuffer(ab) {
    const res = await postAudioDecodeWorker({ type: 'channelsFromBuffer', ab, bars: 0 }, [ab]);
    return {
        sampleRate: res.sampleRate,
        length: res.length,
        channels: res.channels,
    };
}

async function decodePeaksInWorker(url, bars) {
    const res = await postAudioDecodeWorker({ type: 'peaks', url, bars });
    return res.peaks;
}

function peaksFromChannelData(raw, nBars) {
    const step = Math.floor(raw.length / nBars);
    const peaks = [];
    for (let i = 0; i < nBars; i++) {
        let max = 0;
        let min = 0;
        const start = i * step;
        for (let j = start; j < start + step && j < raw.length; j++) {
            if (raw[j] > max) max = raw[j];
            if (raw[j] < min) min = raw[j];
        }
        peaks.push({ max, min });
    }
    return peaks;
}

/** Main-thread `decodeAudioData` + peak envelope (last resort when worker decode fails). */
async function decodePeaksMainThreadFromUrl(url, nBars) {
    const ab = await fetchAudioArrayBuffer(url);
    if (!_audioCtx) _audioCtx = new AudioContext();
    const audioBuf = await _audioCtx.decodeAudioData(ab.slice(0));
    return peaksFromChannelData(audioBuf.getChannelData(0), nBars);
}

/**
 * Prefer worker `fetch`+decode; if that fails, main-thread fetch + transfer to worker; if that fails
 * too (worker broken, bad decode, etc.), decode on the main thread so waveforms never stay blank.
 */
async function decodePeaksViaWorker(url, bars) {
    const nBars = Math.max(1, Math.min(Math.floor(Number(bars)) || 1, 800));
    try {
        return await decodePeaksInWorker(url, nBars);
    } catch {
        /* fall through */
    }
    if (getAudioDecodeWorker()) {
        try {
            const ab = await fetchAudioArrayBuffer(url);
            return await decodePeaksFromArrayBuffer(ab, nBars);
        } catch {
            /* fall through */
        }
    }
    return await decodePeaksMainThreadFromUrl(url, nBars);
}

async function decodeMetaVisualsInWorker(url, bars) {
    const res = await postAudioDecodeWorker({ type: 'meta', url, bars });
    return { peaks: res.peaks, sgData: res.sgData };
}

async function decodeMetaVisualsViaWorker(url, bars) {
    const nBars = Math.max(1, Math.min(Math.floor(Number(bars)) || 1, 800));
    try {
        return await decodeMetaVisualsInWorker(url, nBars);
    } catch {
        /* fall through */
    }
    if (!getAudioDecodeWorker()) {
        throw new Error('no worker');
    }
    const ab = await fetchAudioArrayBuffer(url);
    return await decodeMetaFromArrayBuffer(ab, nBars);
}

async function decodeSpectrogramInWorker(url) {
    const res = await postAudioDecodeWorker({ type: 'spectrogram', url, bars: 0 });
    return res.sgData;
}

async function decodeSpectrogramViaWorker(url) {
    try {
        return await decodeSpectrogramInWorker(url);
    } catch {
        if (!getAudioDecodeWorker()) throw new Error('no worker');
        const ab = await fetchAudioArrayBuffer(url);
        return await decodeSpectrogramFromArrayBuffer(ab);
    }
}

/** @returns {null | ((req: object) => Promise<unknown>)} */
function audioEngineInvokeMetaVisuals() {
    if (
        typeof window !== 'undefined' &&
        window.vstUpdater &&
        typeof window.vstUpdater.audioEngineInvoke === 'function'
    ) {
        return window.vstUpdater.audioEngineInvoke.bind(window.vstUpdater);
    }
    return null;
}

/**
 * `spectrogram_preview` returns `rows`[freq][time] in dB; `renderSpectrogramData` expects
 * `sgData`[time][freq] as linear magnitudes (same post-processing as worker FFT path).
 */
function spectrogramEngineRowsToSgData(rows) {
    if (!rows || !rows.length || !rows[0] || !rows[0].length) return [];
    const nFreq = rows.length;
    const nTime = rows[0].length;
    const sgData = [];
    for (let t = 0; t < nTime; t++) {
        const col = new Array(nFreq);
        for (let r = 0; r < nFreq; r++) {
            const db = Number(rows[r][t]);
            col[r] = Number.isFinite(db) ? Math.pow(10, db / 20) : 0;
        }
        sgData.push(col);
    }
    return sgData;
}

/** Modest `width_px` / `height_px` for AudioEngine spectrogram JSON (smaller payloads on weak devices). */
function metaSpectrogramEnginePixelDims() {
    const cores = typeof navigator !== 'undefined' && navigator.hardwareConcurrency ? navigator.hardwareConcurrency : 8;
    const memGb = typeof navigator !== 'undefined' && navigator.deviceMemory ? navigator.deviceMemory : 8;
    if (cores <= 4 || memGb <= 4) return { width_px: 192, height_px: 48 };
    return { width_px: 256, height_px: 64 };
}

async function fetchWaveformPreviewFromEngine(absPath, widthPx) {
    const invoke = audioEngineInvokeMetaVisuals();
    if (!invoke) return null;
    const wfRes = await invoke({ cmd: 'waveform_preview', path: absPath, width_px: widthPx });
    if (!wfRes || wfRes.ok !== true || !wfRes.peaks) return null;
    return wfRes.peaks;
}

/** Shared with `file-browser.js`: evict + debounced persist after inserting waveform peaks. */
function storeWaveformPeaksInCache(filePath, peaks) {
    if (typeof _waveformCache === 'undefined' || !peaks) return;
    _waveformCache[filePath] = peaks;
    _evictCache(_waveformCache);
    _debounceWfSave();
}

async function fetchSpectrogramPreviewFromEngine(absPath, dims) {
    const invoke = audioEngineInvokeMetaVisuals();
    if (!invoke) return null;
    const sgRes = await invoke({
        cmd: 'spectrogram_preview',
        path: absPath,
        width_px: dims.width_px,
        height_px: dims.height_px,
    });
    if (!sgRes || sgRes.ok !== true || !sgRes.rows) return null;
    const sgData = spectrogramEngineRowsToSgData(sgRes.rows);
    return sgData.length ? sgData : null;
}

async function fetchMetaVisualsFromAudioEngine(filePath, wfWidthPx, sgDims) {
    const invoke = audioEngineInvokeMetaVisuals();
    if (!invoke) return null;
    const [wfRes, sgRes] = await Promise.all([
        invoke({ cmd: 'waveform_preview', path: filePath, width_px: wfWidthPx }),
        invoke({
            cmd: 'spectrogram_preview',
            path: filePath,
            width_px: sgDims.width_px,
            height_px: sgDims.height_px,
        }),
    ]);
    if (!wfRes || wfRes.ok !== true || !wfRes.peaks) return null;
    if (!sgRes || sgRes.ok !== true || !sgRes.rows) return null;
    const sgData = spectrogramEngineRowsToSgData(sgRes.rows);
    if (!sgData.length) return null;
    return { peaks: wfRes.peaks, sgData };
}

async function decodeChannelsInWorker(url) {
    const res = await postAudioDecodeWorker({ type: 'channels', url, bars: 0 });
    return {
        sampleRate: res.sampleRate,
        length: res.length,
        channels: res.channels,
    };
}

async function decodeChannelsViaWorker(url) {
    try {
        return await decodeChannelsInWorker(url);
    } catch {
        if (!getAudioDecodeWorker()) throw new Error('no worker');
        const ab = await fetchAudioArrayBuffer(url);
        return await decodeChannelsFromArrayBuffer(ab);
    }
}

let allAudioSamples = []; // kept for export/compatibility — lazily populated
let filteredAudioSamples = []; // current visible page from DB
let audioTotalCount = 0; // total matching rows in DB
/** True when the backend stopped counting at ~100k FTS hits (exact total unknown). */
let audioTotalCountCapped = false;
let audioTotalUnfiltered = 0; // total rows in scan
let audioCurrentOffset = 0; // pagination offset
let audioSortKey = 'name';
let audioSortAsc = true;
let audioScanProgressCleanup = null;
/** User ran a SQLite filter during scan — skip streaming row appends until scan ends. */
let _audioScanDbView = false;
let _audioScanActive = false;
/** Monotonic id so stale `dbQueryAudio` results never overwrite a newer filter/sort. */
let _audioQuerySeq = 0;
/** Cancels in-flight now-playing waveform decode when the user switches tracks (full-file `decodeAudioData` is expensive for MP3). */
let _npWaveformDrawSeq = 0;
/** Cancels meta-panel waveform/spectrogram work when another row is expanded. */
let _metaPanelDrawSeq = 0;
/** One `AudioBuffer` from the meta waveform decode, reused by spectrogram in the same expand (avoids double full-file decode). */
let _metaSharedDecoded = { path: null, buffer: null };
/** Pending `requestIdleCallback` / `setTimeout` id for now-playing waveform (cancelled on track change). */
let _npWaveformIdleId = null;
/** Pending idle schedule for expanded-row waveform + spectrogram. */
let _metaPanelIdleId = null;

/**
 * Defer waveform/spectrogram work to after the current task. Do not use `requestIdleCallback`:
 * in Tauri/WKWebView it is often starved so callbacks never run and `drawWaveform` /
 * `drawMetaPanelVisuals` never execute.
 */
function scheduleIdleVisualWork(fn, opts) {
    const ms = opts && typeof opts.delayMs === 'number' ? opts.delayMs : 0;
    return setTimeout(fn, ms);
}

function cancelIdleSchedule(id) {
    if (id == null) return;
    clearTimeout(id);
}

/** Asset URL for fetch/decode — prefers `__TAURI__.core` (always correct in app), then `ipc.js` `window` shim. */
function fileSrcForDecode(path) {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    if (tauri?.core?.convertFileSrc && typeof tauri.core.convertFileSrc === 'function') {
        return tauri.core.convertFileSrc(path);
    }
    if (typeof window !== 'undefined' && typeof window.convertFileSrc === 'function') {
        return window.convertFileSrc(path);
    }
    if (typeof convertFileSrc === 'function') return convertFileSrc(path);
    return path;
}

/**
 * WKWebView often reports 0×0 for the waveform flex child until reflow; `bars === 0` yields empty peaks and a blank canvas.
 */
async function resolveWaveformBoxSize(container, fallbackW, fallbackH) {
    const np = document.getElementById('audioNowPlaying');
    const fw = fallbackW != null ? fallbackW : 400;
    const fh = fallbackH != null ? fallbackH : 24;
    for (let i = 0; i < 8; i++) {
        if (np) void np.offsetWidth;
        if (container) void container.offsetWidth;
        let w = container ? container.offsetWidth : 0;
        let h = container ? container.offsetHeight : 0;
        if (w < 2 && container) {
            const br = container.getBoundingClientRect();
            w = br.width;
            h = br.height;
        }
        if (w >= 2 && h >= 2) return { w, h };
        await new Promise((r) => requestAnimationFrame(r));
    }
    const br = container ? container.getBoundingClientRect() : null;
    return {
        w: br && br.width >= 2 ? br.width : fw,
        h: br && br.height >= 2 ? br.height : fh,
    };
}

function scheduleNowPlayingWaveform(filePath) {
    cancelIdleSchedule(_npWaveformIdleId);
    _npWaveformIdleId = null;
    _npWaveformDrawSeq++;
    const wfSeq = _npWaveformDrawSeq;
    _npWaveformIdleId = scheduleIdleVisualWork(() => {
        _npWaveformIdleId = null;
        void drawWaveform(filePath, wfSeq);
    }, { delayMs: 0 });
}

/** `appFmt` wrapper — same pattern as `plugins.js` `_ui`. */
function _audioFmt(key, vars) {
    if (typeof appFmt !== 'function') return key;
    return vars ? appFmt(key, vars) : appFmt(key);
}

// Playback state
let audioPlayer = new Audio();
let audioPlayerPath = null;
let audioLooping = false;
let audioPlaybackRAF = null;
let expandedMetaPath = null;
let recentlyPlayed = [];
const MAX_RECENT = 50;
let audioShuffling = false;
let audioMuted = false;
let savedVolume = 1;
/** Volume % (0–100) saved when muting — engine path uses prefs, so `savedVolume` alone is wrong. */
let savedMuteVolumePct = 100;

// ── Web Audio processing chain ──
let _playbackCtx = null;
let _sourceNode = null;
let _eqLow = null;
let _eqMid = null;
let _eqHigh = null;
let _gainNode = null;
let _panNode = null;
let _analyser = null;
let _monoMode = false;
let _abLoop = null; // { start, end } in seconds, or null

// Reverse playback (decoded buffer played backwards through the same EQ chain; HTMLAudioElement has no negative playbackRate)
let audioReverseMode = false;
let _decodedBuf = null;
let _decodedBufPath = null;
let _reversedBuf = null;
let _bufSrc = null;
let _bufPlaying = false;
let _bufSegStartCtx = 0;
let _bufOffsetInRev = 0;
let _bufPlaybackRate = 1;
let _pausedOffsetInRev = 0;
let _reverseDecodeBusy = false;

/** Library playback through `audio-engine` AudioEngine (no Web Audio output). */
let _enginePlaybackActive = false;

function setEnginePlaybackActive(value) {
    _enginePlaybackActive = value;
    if (typeof window !== 'undefined') {
        window._enginePlaybackActive = value;
    }
}

/**
 * Waveform click-to-seek diagnostics. In DevTools, filter by `[audio-haxor] waveform-seek`.
 * Set `window.__AUDIO_HAXOR_WAVEFORM_SEEK_LOG = false` to silence (default is on).
 */
function logWaveformSeek(phase, detail) {
    try {
        if (typeof window !== 'undefined' && window.__AUDIO_HAXOR_WAVEFORM_SEEK_LOG === false) return;
        if (typeof console !== 'undefined' && typeof console.warn === 'function') {
            console.warn('[audio-haxor] waveform-seek', phase, detail == null ? '' : detail);
        }
    } catch (_) {
        /* ignore */
    }
}

/** Audio Engine tab "Stop stream" calls `stop_output_stream` + `playback_stop`; sync JS so `isAudioPlaying()` matches. */
function syncEnginePlaybackStoppedFromAudioEngine() {
    setEnginePlaybackActive(false);
    if (typeof window.stopEnginePlaybackPoll === 'function') {
        window.stopEnginePlaybackPoll();
    }
    window._enginePlaybackPosSec = 0;
    window._enginePlaybackDurSec = 0;
    window._enginePlaybackPaused = false;
    window._engineSpectrumU8 = null;
    if (typeof window !== 'undefined') window._aeOutputStreamRunning = false;
    if (typeof stopEnginePlaybackFftRaf === 'function') stopEnginePlaybackFftRaf();
    if (typeof updatePlayBtnStates === 'function') {
        updatePlayBtnStates();
    }
    if (typeof updateNowPlayingBtn === 'function') {
        updateNowPlayingBtn();
    }
}
/**
 * After Apply reconnects the output stream with `playback_load` (e.g. Stop stream then Apply),
 * restore JS engine playback state and polling.
 * @param {object|null} loadMeta — optional `playback_load` response (`duration_sec`, …)
 */
function resumeEnginePlaybackAfterApply(loadMeta) {
    setEnginePlaybackActive(true);
    if (typeof window !== 'undefined') window._aeOutputStreamRunning = true;
    if (loadMeta && typeof loadMeta.duration_sec === 'number' && !Number.isNaN(loadMeta.duration_sec) && loadMeta.duration_sec > 0) {
        window._enginePlaybackDurSec = loadMeta.duration_sec;
    }
    window._enginePlaybackPosSec = 0;
    window._enginePlaybackPaused = false;
    if (typeof window.startEnginePlaybackPoll === 'function') {
        window.startEnginePlaybackPoll();
    }
    if (typeof updatePlayBtnStates === 'function') {
        updatePlayBtnStates();
    }
    if (typeof updateNowPlayingBtn === 'function') {
        updateNowPlayingBtn();
    }
}

if (typeof window !== 'undefined') {
    window.syncEnginePlaybackStoppedFromAudioEngine = syncEnginePlaybackStoppedFromAudioEngine;
    window.resumeEnginePlaybackAfterApply = resumeEnginePlaybackAfterApply;
}

/** Neither WebView `<audio>` nor JUCE `registerBasicFormats` can decode — preview shows meta line only. */
const ENGINE_UNPLAYABLE_EXT = new Set(['sf2', 'sfz', 'rex', 'rx2', 'wma', 'ape', 'opus', 'mid', 'midi']);
function isEngineUnplayablePath(filePath) {
    if (!filePath) return false;
    const x = filePath.split('.').pop().toLowerCase();
    return ENGINE_UNPLAYABLE_EXT.has(x);
}

function isAudioPlaying() {
    if (_enginePlaybackActive) {
        return window._enginePlaybackPaused !== true;
    }
    if (audioReverseMode && _bufPlaying) return true;
    return typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused;
}

/**
 * `playback_status` poll (~100ms) sets `_enginePlaybackPaused`. After `playback_pause` IPC, apply the
 * same value immediately so `isAudioPlaying()` matches buttons (context menu → `previewAudio`, main bar,
 * sample rows) before the next poll.
 */
function applyEnginePlaybackPausedFromTransport(paused) {
    if (typeof window !== 'undefined') {
        window._enginePlaybackPaused = paused === true;
    }
}

/**
 * True when `window._engineSpectrumU8` should drive the floating mini FFT, parametric EQ fill, etc.
 * Matches `visualizer.js` `_vizEngineSpectrumOk`: library playback through the AudioEngine, or any
 * Audio Engine output with an FFT tap (`_aeOutputStreamRunning`).
 * Transport pause (`playback_pause`) does **not** turn this off — spectrum still updates (or holds
 * last bins) independently of global play/pause; only `fftAnimationPaused` freezes the curves.
 */
function engineSpectrumLive() {
    if (typeof window === 'undefined' || !window._engineSpectrumU8 || window._engineSpectrumU8.length < 1024) {
        return false;
    }
    if (window._enginePlaybackActive === true) {
        return true;
    }
    if (window._aeOutputStreamRunning === true) {
        return true;
    }
    return false;
}

if (typeof window !== 'undefined') {
    window.engineSpectrumLive = engineSpectrumLive;
}

/** Reverse PCM in chunks with event-loop yields so multi-hour WAVs do not freeze the WebView. */
async function reverseAudioBufferAsync(ctx, buf) {
    const len = buf.length;
    const ch = buf.numberOfChannels;
    const out = ctx.createBuffer(ch, len, buf.sampleRate);
    const yieldEvery = 250000;
    let op = 0;
    for (let c = 0; c < ch; c++) {
        const s = buf.getChannelData(c);
        const d = out.getChannelData(c);
        for (let i = 0; i < len; i++) {
            d[i] = s[len - 1 - i];
            op++;
            if (op % yieldEvery === 0 && typeof yieldToBrowser === 'function') {
                await yieldToBrowser();
            }
        }
    }
    return out;
}

/** Copy decoded PCM into an `AudioBuffer` in slices so one huge `set` does not freeze the WebView on multi‑GB WAVs. */
async function copyFloat32ToBufferChannelAsync(buf, channelIndex, src) {
    const dst = buf.getChannelData(channelIndex);
    const len = Math.min(src.length, dst.length);
    const chunk = 524288;
    for (let i = 0; i < len; i += chunk) {
        const n = Math.min(chunk, len - i);
        dst.set(src.subarray(i, i + n), i);
        if (i + n < len && typeof yieldToBrowser === 'function') {
            await yieldToBrowser();
        }
    }
}

async function ensureReversedBufferForPath(path) {
    if (_reversedBuf && _decodedBufPath === path) return _reversedBuf;
    ensureAudioGraph();
    const url = fileSrcForDecode(path);
    let buf = null;
    try {
        const dec = await decodeChannelsViaWorker(url);
        buf = _playbackCtx.createBuffer(dec.channels.length, dec.length, dec.sampleRate);
        for (let c = 0; c < dec.channels.length; c++) {
            await copyFloat32ToBufferChannelAsync(buf, c, dec.channels[c]);
        }
    } catch (e) {
        if (!getAudioDecodeWorker()) {
            const resp = await fetch(url);
            const arr = await resp.arrayBuffer();
            buf = await _playbackCtx.decodeAudioData(arr.slice(0));
        } else {
            throw e;
        }
    }
    _decodedBuf = buf;
    _decodedBufPath = path;
    _reversedBuf = await reverseAudioBufferAsync(_playbackCtx, buf);
    return _reversedBuf;
}

function disconnectMediaFromEq() {
    ensureAudioGraph();
    try {
        _sourceNode.disconnect();
    } catch (_) {
    }
}

function connectMediaToEq() {
    ensureAudioGraph();
    try {
        _sourceNode.disconnect();
    } catch (_) {
    }
    _sourceNode.connect(_eqLow);
}

/** AudioEngine playback: keep `<audio>` disconnected from Web Audio + muted so nothing doubles through the WebView. */
function silenceWebViewAudioForEngine() {
    if (typeof audioPlayer === 'undefined' || !audioPlayer) return;
    try {
        audioPlayer.pause();
    } catch (_) {}
    audioPlayer.muted = true;
    audioPlayer.volume = 0;
    try {
        audioPlayer.removeAttribute('src');
        audioPlayer.src = '';
    } catch (_) {}
    disconnectMediaFromEq();
    if (_gainNode) {
        _gainNode.gain.value = 0;
    }
}

/** Restore HTML / Web Audio path after engine stops or when falling back to `<audio>` decode. */
function restoreWebViewAudioAfterEngine() {
    if (typeof audioPlayer === 'undefined' || !audioPlayer) return;
    audioPlayer.muted = false;
    let vol = 1;
    if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const v = parseInt(prefs.getItem('audioVolume') || '100', 10);
        vol = Math.max(0, Math.min(1, v / 100));
    }
    audioPlayer.volume = vol;
    connectMediaToEq();
    if (_gainNode) {
        const pre = parseFloat(document.getElementById('npGainSlider')?.value || '1');
        _gainNode.gain.value = vol * pre;
    }
}

function getPlaybackSpeedValue() {
    const sel = document.getElementById('npSpeed');
    const v = parseFloat(sel?.value || '1');
    return Number.isFinite(v) ? v : 1;
}

function getOriginalTimeFromReverseBuffer() {
    if (!_reversedBuf || !_bufPlaying) return 0;
    const dur = _reversedBuf.duration;
    const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
    const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
    return Math.max(0, dur - posInRev);
}

function stopReverseBufferPlayback() {
    if (_bufSrc) {
        try {
            _bufSrc.stop(0);
        } catch (_) {
        }
        try {
            _bufSrc.disconnect();
        } catch (_) {
        }
        _bufSrc = null;
    }
    _bufPlaying = false;
    if (_playbackRafId) {
        cancelAnimationFrame(_playbackRafId);
        _playbackRafId = null;
    }
}

function pauseReverseBufferPlayback() {
    if (!audioReverseMode || !_reversedBuf || !_bufPlaying) return;
    const dur = _reversedBuf.duration;
    const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
    const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
    _pausedOffsetInRev = Math.max(0, Math.min(posInRev, dur - 0.001));
    stopReverseBufferPlayback();
}

function startReverseBufferFromOffset(offsetInRev) {
    if (!_reversedBuf || !audioPlayerPath) return;
    stopReverseBufferPlayback();
    ensureAudioGraph();
    if (_playbackCtx.state === 'suspended') {
        _playbackCtx.resume().catch(() => {
        });
    }
    const buf = _reversedBuf;
    const dur = buf.duration;
    const rate = Math.max(0.0625, Math.min(16, getPlaybackSpeedValue()));
    const off = Math.max(0, Math.min(offsetInRev, dur - 0.001));
    if (off >= dur) return;
    disconnectMediaFromEq();
    _bufSrc = _playbackCtx.createBufferSource();
    _bufSrc.buffer = buf;
    _bufSrc.playbackRate.value = rate;
    _bufSrc.connect(_eqLow);
    _bufSegStartCtx = _playbackCtx.currentTime;
    _bufOffsetInRev = off;
    _bufPlaybackRate = rate;
    _bufSrc.onended = () => {
        _bufSrc = null;
        _bufPlaying = false;
        if (_playbackRafId) {
            cancelAnimationFrame(_playbackRafId);
            _playbackRafId = null;
        }
        if (!audioPlayerPath) return;
        if (audioLooping) {
            _pausedOffsetInRev = 0;
            startReverseBufferFromOffset(0);
            return;
        }
        if (typeof recentlyPlayed !== 'undefined' && recentlyPlayed.length > 1 && prefs.getItem('autoplayNext') !== 'off') {
            nextTrack({ autoplay: true });
        } else {
            updatePlayBtnStates();
            updateNowPlayingBtn();
        }
    };
    _bufSrc.start(0, off);
    _bufPlaying = true;
    if (!_playbackRafId) _playbackRafId = requestAnimationFrame(_playbackRafLoop);
}

function syncReversePlaybackButtons(active) {
    for (const id of ['npBtnReverse', 'npEqBtnReverse']) {
        const el = document.getElementById(id);
        if (el) el.classList.toggle('active', !!active);
    }
}

async function toggleReversePlayback() {
    if (!audioPlayerPath) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.reverse_no_track'), 3000, 'error');
        return;
    }
    if (_enginePlaybackActive) {
        const inv =
            typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
                ? window.vstUpdater.audioEngineInvoke.bind(window.vstUpdater)
                : null;
        const restart =
            typeof window !== 'undefined' && typeof window.enginePlaybackRestartStream === 'function'
                ? window.enginePlaybackRestartStream
                : null;
        if (!inv || !restart) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.reverse_no_track'), 3000, 'error');
            return;
        }
        if (_reverseDecodeBusy) return;
        if (audioReverseMode) {
            audioReverseMode = false;
            prefs.setItem('audioReverse', 'off');
            syncReversePlaybackButtons(false);
            const cur =
                typeof window._enginePlaybackPosSec === 'number' && !Number.isNaN(window._enginePlaybackPosSec)
                    ? window._enginePlaybackPosSec
                    : 0;
            _reverseDecodeBusy = true;
            try {
                await inv({cmd: 'playback_set_reverse', reverse: false});
                await restart();
                await inv({cmd: 'playback_seek', position_sec: cur});
            } catch (e) {
                if (typeof showToast === 'function') {
                    showToast(toastFmt('toast.reverse_playback_failed', {err: e.message || String(e)}), 4000, 'error');
                }
            } finally {
                _reverseDecodeBusy = false;
            }
            updatePlayBtnStates();
            updateNowPlayingBtn();
            return;
        }
        audioReverseMode = true;
        prefs.setItem('audioReverse', 'on');
        syncReversePlaybackButtons(true);
        const cur =
            typeof window._enginePlaybackPosSec === 'number' && !Number.isNaN(window._enginePlaybackPosSec)
                ? window._enginePlaybackPosSec
                : 0;
        _reverseDecodeBusy = true;
        try {
            await inv({cmd: 'playback_set_reverse', reverse: true});
            await restart();
            await inv({cmd: 'playback_seek', position_sec: cur});
        } catch (e) {
            audioReverseMode = false;
            prefs.setItem('audioReverse', 'off');
            syncReversePlaybackButtons(false);
            if (typeof showToast === 'function') {
                showToast(toastFmt('toast.reverse_playback_failed', {err: e.message || String(e)}), 4000, 'error');
            }
        } finally {
            _reverseDecodeBusy = false;
        }
        updatePlayBtnStates();
        updateNowPlayingBtn();
        return;
    }
    if (_reverseDecodeBusy) return;
    if (audioReverseMode) {
        audioReverseMode = false;
        prefs.setItem('audioReverse', 'off');
        syncReversePlaybackButtons(false);
        let origT = 0;
        if (_reversedBuf) {
            const dur = _reversedBuf.duration;
            if (_bufPlaying) origT = getOriginalTimeFromReverseBuffer();
            else origT = Math.max(0, dur - _pausedOffsetInRev);
        }
        stopReverseBufferPlayback();
        connectMediaToEq();
        if (audioPlayer.duration && !Number.isNaN(audioPlayer.duration)) {
            audioPlayer.currentTime = Math.min(origT, Math.max(0, audioPlayer.duration - 0.01));
        }
        try {
            await audioPlayer.play();
        } catch (_) {
        }
        updatePlayBtnStates();
        updateNowPlayingBtn();
        return;
    }
    audioReverseMode = true;
    prefs.setItem('audioReverse', 'on');
    syncReversePlaybackButtons(true);
    audioPlayer.pause();
    _reverseDecodeBusy = true;
    try {
        await ensureReversedBufferForPath(audioPlayerPath);
        const dur = _reversedBuf.duration;
        let origT = audioPlayer.currentTime || 0;
        if (origT <= 0 && _pausedOffsetInRev > 0) origT = Math.max(0, dur - _pausedOffsetInRev);
        const off = Math.max(0, dur - origT);
        _pausedOffsetInRev = off;
        startReverseBufferFromOffset(off);
    } catch (e) {
        audioReverseMode = false;
        prefs.setItem('audioReverse', 'off');
        syncReversePlaybackButtons(false);
        if (typeof showToast === 'function') showToast(toastFmt('toast.reverse_playback_failed', {err: e.message || e}), 4000, 'error');
    } finally {
        _reverseDecodeBusy = false;
    }
}

function ensureAudioGraph() {
    if (_playbackCtx) return;
    _playbackCtx = new AudioContext();
    _sourceNode = _playbackCtx.createMediaElementSource(audioPlayer);

    // 3-band EQ
    _eqLow = _playbackCtx.createBiquadFilter();
    _eqLow.type = 'lowshelf';
    _eqLow.frequency.value = 200;
    _eqLow.gain.value = 0;

    _eqMid = _playbackCtx.createBiquadFilter();
    _eqMid.type = 'peaking';
    _eqMid.frequency.value = 1000;
    _eqMid.Q.value = 1;
    _eqMid.gain.value = 0;

    _eqHigh = _playbackCtx.createBiquadFilter();
    _eqHigh.type = 'highshelf';
    _eqHigh.frequency.value = 8000;
    _eqHigh.gain.value = 0;

    // Gain (preamp)
    _gainNode = _playbackCtx.createGain();
    _gainNode.gain.value = 1;

    // Stereo pan
    _panNode = _playbackCtx.createStereoPanner();
    _panNode.pan.value = 0;

    // FFT analyser for parametric EQ visualization
    _analyser = _playbackCtx.createAnalyser();
    _analyser.fftSize = 8192;
    _analyser.smoothingTimeConstant = 0.8;

    // Stereo split analysers for Lissajous/stereo field
    window._splitter = _playbackCtx.createChannelSplitter(2);
    window._analyserL = _playbackCtx.createAnalyser();
    window._analyserR = _playbackCtx.createAnalyser();
    window._analyserL.fftSize = 2048;
    window._analyserR.fftSize = 2048;
    window._analyserL.smoothingTimeConstant = 0.5;
    window._analyserR.smoothingTimeConstant = 0.5;

    // Chain: source → eqLow → eqMid → eqHigh → gain → analyser → pan → destination
    //                                                  ↘ splitter → analyserL/R
    _sourceNode.connect(_eqLow);
    _eqLow.connect(_eqMid);
    _eqMid.connect(_eqHigh);
    _eqHigh.connect(_gainNode);
    _gainNode.connect(_analyser);
    _gainNode.connect(window._splitter);
    window._splitter.connect(window._analyserL, 0);
    window._splitter.connect(window._analyserR, 1);
    _analyser.connect(_panNode);
    _panNode.connect(_playbackCtx.destination);
}

function setEqBand(band, value) {
    ensureAudioGraph();
    const db = parseFloat(value);
    if (band === 'low') _eqLow.gain.value = db;
    else if (band === 'mid') _eqMid.gain.value = db;
    else if (band === 'high') _eqHigh.gain.value = db;
    const cap = band.charAt(0).toUpperCase() + band.slice(1);
    const label = document.getElementById('npEq' + cap + 'Val');
    if (label) label.textContent = (db >= 0 ? '+' : '') + db.toFixed(0) + ' dB';
    prefs.setItem('eq' + cap, String(value));
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        window.syncEnginePlaybackDspFromPrefs();
    }
}

function setPreampGain(value) {
    ensureAudioGraph();
    const g = parseFloat(value);
    _gainNode.gain.value = g;
    const label = document.getElementById('npGainVal');
    if (label) label.textContent = (g * 100).toFixed(0) + '%';
    const aeG = document.getElementById('aeGainSlider');
    if (aeG) aeG.value = String(g);
    const aeLab = document.getElementById('aeGainVal');
    if (aeLab) aeLab.textContent = (g * 100).toFixed(0) + '%';
    prefs.setItem('preampGain', String(value));
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        window.syncEnginePlaybackDspFromPrefs();
    }
}

function setPan(value) {
    ensureAudioGraph();
    const p = parseFloat(value);
    _panNode.pan.value = p;
    const label = document.getElementById('npPanVal');
    const panTxt =
        Math.abs(p) < 0.05 ? 'C' : p < 0 ? Math.round(Math.abs(p) * 100) + 'L' : Math.round(p * 100) + 'R';
    if (label) label.textContent = panTxt;
    const aeP = document.getElementById('aePanSlider');
    if (aeP) aeP.value = String(p);
    const aeLab = document.getElementById('aePanVal');
    if (aeLab) aeLab.textContent = panTxt;
    prefs.setItem('audioPan', String(value));
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        window.syncEnginePlaybackDspFromPrefs();
    }
}

function toggleEqSection() {
    const section = document.getElementById('npEqSection');
    const btn = document.getElementById('npEqToggle');
    if (!section || !btn) return;
    section.classList.toggle('visible');
    const vis = section.classList.contains('visible');
    btn.classList.toggle('active', vis);
    /* MutationObserver + ResizeObserver alone can miss in release WebView; kick after layout settles. */
    if (vis) {
        if (typeof window.applyNpEqCanvasHeightFromPrefs === 'function') window.applyNpEqCanvasHeightFromPrefs();
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
            });
        });
    }
}

function toggleMono() {
    _monoMode = !_monoMode;
    const btn = document.getElementById('npBtnMono');
    if (btn) btn.classList.toggle('active', _monoMode);
    // Mono via pan automation isn't possible with StereoPanner alone,
    // so we use a ChannelMerger approach. Simpler: just set pan to center
    // and note the state. Full mono requires a splitter/merger which is
    // heavy — for a preview player, center-pan is the practical equivalent.
    prefs.setItem('audioMono', _monoMode ? 'on' : 'off');
    if (_monoMode) {
        setPan(0);
        const slider = document.getElementById('npPanSlider');
        if (slider) {
            slider.value = 0;
            slider.disabled = true;
        }
    } else {
        const slider = document.getElementById('npPanSlider');
        if (slider) slider.disabled = false;
    }
}

function resetEq() {
    ensureAudioGraph();
    _eqLow.gain.value = 0;
    _eqMid.gain.value = 0;
    _eqHigh.gain.value = 0;
    _gainNode.gain.value = 1;
    _panNode.pan.value = 0;
    _monoMode = false;
    // Update UI
    ['npEqLow', 'npEqMid', 'npEqHigh'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.value = 0;
    });
    const gain = document.getElementById('npGainSlider');
    if (gain) gain.value = 1;
    const pan = document.getElementById('npPanSlider');
    if (pan) {
        pan.value = 0;
        pan.disabled = false;
    }
    const mono = document.getElementById('npBtnMono');
    if (mono) mono.classList.remove('active');
    document.getElementById('npEqLowVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npEqMidVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npEqHighVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npGainVal').textContent = catalogFmt('ui.audio.eq_gain_pct');
    document.getElementById('npPanVal').textContent = catalogFmt('ui.audio.pan_center');
    showToast(toastFmt('toast.eq_reset'));
}

// A-B loop
function setAbLoopStart() {
    if (!audioPlayerPath || !audioPlayer.duration) return;
    const t = audioPlayer.currentTime;
    if (!_abLoop) _abLoop = {start: t, end: audioPlayer.duration};
    else _abLoop.start = Math.min(t, _abLoop.end - 0.05); // keep start < end
    updateAbLoopUI();
    showToast(toastFmt('toast.ab_point_a', {time: formatTime(_abLoop.start)}));
}

function setAbLoopEnd() {
    if (!audioPlayerPath || !audioPlayer.duration) return;
    const t = audioPlayer.currentTime;
    if (!_abLoop) _abLoop = {start: 0, end: t};
    else _abLoop.end = Math.max(t, _abLoop.start + 0.05); // keep end > start
    updateAbLoopUI();
    showToast(toastFmt('toast.ab_point_b', {time: formatTime(_abLoop.end)}));
}

function clearAbLoop() {
    _abLoop = null;
    updateAbLoopUI();
}

function updateAbLoopUI() {
    const aBtn = document.getElementById('npAbA');
    const bBtn = document.getElementById('npAbB');
    const clearBtn = document.getElementById('npAbClear');
    if (aBtn) aBtn.classList.toggle('active', !!_abLoop);
    if (bBtn) bBtn.classList.toggle('active', !!_abLoop);
    if (clearBtn) clearBtn.style.display = _abLoop ? '' : 'none';
    // Show markers on waveform
    const wf = document.getElementById('npWaveform');
    let markerA = document.getElementById('npAbMarkerA');
    let markerB = document.getElementById('npAbMarkerB');
    if (!_abLoop) {
        if (markerA) markerA.style.display = 'none';
        if (markerB) markerB.style.display = 'none';
        return;
    }
    const dur = audioPlayer.duration || 1;
    if (!markerA) {
        markerA = document.createElement('div');
        markerA.id = 'npAbMarkerA';
        markerA.className = 'ab-marker ab-marker-a';
        wf.appendChild(markerA);
    }
    if (!markerB) {
        markerB = document.createElement('div');
        markerB.id = 'npAbMarkerB';
        markerB.className = 'ab-marker ab-marker-b';
        wf.appendChild(markerB);
    }
    markerA.style.display = '';
    markerB.style.display = '';
    markerA.style.left = ((_abLoop.start / dur) * 100) + '%';
    markerB.style.left = ((_abLoop.end / dur) * 100) + '%';
}

function loadRecentlyPlayed() {
    recentlyPlayed = prefs.getObject('recentlyPlayed', []);
    // Restore playback settings
    audioLooping = prefs.getItem('audioLoop') === 'on';
    audioPlayer.loop = audioLooping;
    const loopBtn = document.getElementById('npBtnLoop');
    if (loopBtn) loopBtn.classList.toggle('active', audioLooping);

    audioShuffling = prefs.getItem('shuffleMode') === 'on';
    const shuffleBtn = document.getElementById('npBtnShuffle');
    if (shuffleBtn) shuffleBtn.classList.toggle('active', audioShuffling);

    const savedVol = prefs.getItem('audioVolume');
    if (savedVol) {
        const slider = document.getElementById('npVolume');
        if (slider) {
            slider.value = savedVol;
            setAudioVolume(savedVol);
        }
    }

    const savedSpeed = prefs.getItem('audioSpeed');
    if (savedSpeed) {
        const sel = document.getElementById('npSpeed');
        if (sel) {
            sel.value = savedSpeed;
        }
        setPlaybackSpeed(savedSpeed);
    }

    audioReverseMode = prefs.getItem('audioReverse') === 'on';
    syncReversePlaybackButtons(audioReverseMode);

    _monoMode = prefs.getItem('audioMono') === 'on';
    const monoBtn = document.getElementById('npBtnMono');
    if (monoBtn) monoBtn.classList.toggle('active', _monoMode);

    const savedPan = prefs.getItem('audioPan');
    if (savedPan) {
        const panSlider = document.getElementById('npPanSlider');
        if (panSlider) {
            panSlider.value = savedPan;
        }
        setPan(savedPan);
    }

    // Restore EQ bands + preamp gain
    const savedEqLow = prefs.getItem('eqLow');
    const savedEqMid = prefs.getItem('eqMid');
    const savedEqHigh = prefs.getItem('eqHigh');
    const savedGain = prefs.getItem('preampGain');
    if (savedEqLow) {
        const el = document.getElementById('npEqLow');
        if (el) el.value = savedEqLow;
        if (typeof setEqBand === 'function') setEqBand('low', savedEqLow);
    }
    if (savedEqMid) {
        const el = document.getElementById('npEqMid');
        if (el) el.value = savedEqMid;
        if (typeof setEqBand === 'function') setEqBand('mid', savedEqMid);
    }
    if (savedEqHigh) {
        const el = document.getElementById('npEqHigh');
        if (el) el.value = savedEqHigh;
        if (typeof setEqBand === 'function') setEqBand('high', savedEqHigh);
    }
    if (savedGain) {
        const el = document.getElementById('npGainSlider');
        if (el) el.value = savedGain;
        if (typeof setPreampGain === 'function') setPreampGain(savedGain);
    }
}

function saveRecentlyPlayed() {
    prefs.setItem('recentlyPlayed', recentlyPlayed);
}

function clearRecentlyPlayed() {
    recentlyPlayed = [];
    saveRecentlyPlayed();
    renderRecentlyPlayed();
    showToast(toastFmt('toast.play_history_cleared'));
}

function exportRecentlyPlayed() {
    if (recentlyPlayed.length === 0) {
        showToast(toastFmt('toast.no_play_history_export'));
        return;
    }
    _exportCtx = {
        title: catalogFmt('ui.dialog.play_history'),
        defaultName: exportFileName('play-history', recentlyPlayed.length),
        exportFn: async (fmt, filePath) => {
            if (fmt === 'pdf') {
                const headers = [
                    catalogFmt('ui.export.col_name'),
                    catalogFmt('ui.export.col_format'),
                    catalogFmt('ui.export.col_size'),
                    catalogFmt('ui.export.col_path'),
                ];
                const rows = recentlyPlayed.map(r => [r.name, r.format, r.size || '', r.path]);
                await window.vstUpdater.exportPdf(catalogFmt('ui.dialog.play_history'), headers, rows, filePath);
            } else if (fmt === 'csv' || fmt === 'tsv') {
                const sep = fmt === 'tsv' ? '\t' : ',';
                const esc = (v) => {
                    const s = String(v || '');
                    return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s;
                };
                const lines = [
                    catalogFmt('ui.export.col_name') +
                        sep +
                        catalogFmt('ui.export.col_format') +
                        sep +
                        catalogFmt('ui.export.col_size') +
                        sep +
                        catalogFmt('ui.export.col_path'),
                ];
                for (const r of recentlyPlayed) lines.push([r.name, r.format, r.size || '', r.path].map(esc).join(sep));
                await window.__TAURI__.core.invoke('write_text_file', {filePath, contents: lines.join('\n')});
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({history: recentlyPlayed}, filePath);
            } else {
                const json = JSON.stringify(recentlyPlayed, null, 2);
                await window.__TAURI__.core.invoke('write_text_file', {filePath, contents: json});
            }
        }
    };
    showExportModal('history', catalogFmt('ui.dialog.play_history'), recentlyPlayed.length);
}

async function importRecentlyPlayed() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: catalogFmt('ui.dialog.import_play_history'),
        multiple: false,
        filters: ALL_IMPORT_FILTERS,
    });
    if (!selected) return;
    const filePath = typeof selected === 'string' ? selected : selected.path;
    if (!filePath) return;
    try {
        let imported;
        if (filePath.endsWith('.toml')) {
            const data = await window.vstUpdater.importToml(filePath);
            imported = data.history || data;
        } else {
            const text = await window.__TAURI__.core.invoke('read_text_file', {filePath});
            imported = JSON.parse(text);
        }
        if (!Array.isArray(imported)) throw new Error('Expected an array');
        const existing = new Set(recentlyPlayed.map(r => r.path));
        let added = 0;
        for (const item of imported) {
            if (item.path && !existing.has(item.path)) {
                recentlyPlayed.push(item);
                existing.add(item.path);
                added++;
            }
        }
        if (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.length = MAX_RECENT;
        saveRecentlyPlayed();
        renderRecentlyPlayed();
        showToast(toastFmt('toast.imported_tracks', {added, dup: imported.length - added}));
    } catch (e) {
        showToast(toastFmt('toast.import_failed', {err: e.message || e}), 4000, 'error');
    }
}

audioPlayer.addEventListener('ended', () => {
    if (!audioLooping) {
        if (typeof recentlyPlayed !== 'undefined' && recentlyPlayed.length > 1 && prefs.getItem('autoplayNext') !== 'off') {
            nextTrack({ autoplay: true });
        } else {
            updatePlayBtnStates();
            updateNowPlayingBtn();
        }
    }
});

/** AudioEngine path mutes `<audio>` — no `ended` event; `playback_status.eof` drives the same logic. */
let _enginePlaybackEofHandled = false;

function resetEnginePlaybackEofFlag() {
    _enginePlaybackEofHandled = false;
}

function handleEnginePlaybackEofFromPoll() {
    if (_enginePlaybackEofHandled) return;
    if (!_enginePlaybackActive) return;
    if (audioLooping) return;
    _enginePlaybackEofHandled = true;
    if (typeof recentlyPlayed !== 'undefined' && recentlyPlayed.length > 1 && prefs.getItem('autoplayNext') !== 'off') {
        nextTrack({ autoplay: true });
    } else {
        updatePlayBtnStates();
        updateNowPlayingBtn();
    }
}

if (typeof window !== 'undefined') {
    window.resetEnginePlaybackEofFlag = resetEnginePlaybackEofFlag;
    window.handleEnginePlaybackEofFromPoll = handleEnginePlaybackEofFromPoll;
}
// Use rAF loop instead of timeupdate for smooth 60fps playhead
let _playbackRafId = null;

/** AudioEngine output spectrum (np FFT + parametric EQ) when Web Audio analyser has no signal. */
let _enginePlaybackFftRafId = null;

function shouldRunEngineSpectrumRaf() {
    if (typeof window === 'undefined') return false;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return false;
    if (typeof engineSpectrumLive === 'function' && engineSpectrumLive()) return true;
    return false;
}

function _enginePlaybackFftLoop() {
    _enginePlaybackFftRafId = null;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return;
    if (typeof _renderNpFft === 'function') _renderNpFft();
    if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
    if (!shouldRunEngineSpectrumRaf()) return;
    if (typeof isFftAnimationPaused === 'function' && isFftAnimationPaused()) return;
    _enginePlaybackFftRafId = requestAnimationFrame(_enginePlaybackFftLoop);
}

function ensureEnginePlaybackFftRaf() {
    if (!shouldRunEngineSpectrumRaf()) return;
    if (_enginePlaybackFftRafId != null) return;
    _enginePlaybackFftRafId = requestAnimationFrame(_enginePlaybackFftLoop);
}

function stopEnginePlaybackFftRaf() {
    if (_enginePlaybackFftRafId != null) {
        cancelAnimationFrame(_enginePlaybackFftRafId);
        _enginePlaybackFftRafId = null;
    }
}

if (typeof window !== 'undefined') {
    window.ensureEnginePlaybackFftRaf = ensureEnginePlaybackFftRaf;
    window.stopEnginePlaybackFftRaf = stopEnginePlaybackFftRaf;
}

/** Prefs: `fftAnimationPaused` — `1` freezes spectrum curves (mini FFT, visualizer FFT tile, EQ fill). */
const FFT_ANIM_PREF_KEY = 'fftAnimationPaused';

function isFftAnimationPaused() {
    try {
        return typeof prefs !== 'undefined' && prefs.getItem && prefs.getItem(FFT_ANIM_PREF_KEY) === '1';
    } catch {
        return false;
    }
}

function setFftAnimationPaused(on) {
    if (typeof prefs === 'undefined' || !prefs.setItem) return;
    prefs.setItem(FFT_ANIM_PREF_KEY, on ? '1' : '0');
    if (on) {
        if (typeof stopEnginePlaybackFftRaf === 'function') stopEnginePlaybackFftRaf();
    } else if (typeof ensureEnginePlaybackFftRaf === 'function') {
        ensureEnginePlaybackFftRaf();
    }
}

function toggleFftAnimationPaused() {
    setFftAnimationPaused(!isFftAnimationPaused());
}

if (typeof window !== 'undefined') {
    window.isFftAnimationPaused = isFftAnimationPaused;
    window.setFftAnimationPaused = setFftAnimationPaused;
    window.toggleFftAnimationPaused = toggleFftAnimationPaused;
}

function _playbackRafLoop() {
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) {
        if (_playbackRafId) {
            cancelAnimationFrame(_playbackRafId);
            _playbackRafId = null;
        }
        return;
    }
    updatePlaybackTime();
    _renderNpFft();
    if (isAudioPlaying()) {
        _playbackRafId = requestAnimationFrame(_playbackRafLoop);
    }
}

// Real-time FFT spectrum curve in the player's visualizer section.
// Magenta→cyan gradient + cyan outline (matches parametric EQ spectrum fill).
let _npFftBuf = null;
let _npFftGrad = null;
let _npFftCanvas = null;
let _npFftCtx = null;
/** Reused point list for spectrum outline (one Web Audio bin pass, then fill + stroke). */
let _npFftPts = null;

/**
 * IPC `playback_status` updates `spectrum_sr_hz` / `spectrum_fft_size` frequently; using those
 * values directly would shift the log-frequency x-axis every poll (visible "scaling"). Pin once
 * per engine-playback session for stable bin→pixel mapping (mini FFT + parametric EQ fill).
 */
let _npFftEngineAxisSrHz = null;
let _npFftEngineAxisFftSize = null;
/** Pinned once per Web Audio session so fMax / bin→Hz mapping does not jitter frame-to-frame. */
let _npFftWebAxisSrHz = null;
let _npFftWebAxisFftSize = null;
/** `'engine'` | `'web'` — changing source resets both pin sets (see `syncNpFftSpectrumAxisPins`). */
let _npFftSpectrumSourceMode = null;

function syncNpFftSpectrumAxisPins(useEngineSpectrum) {
    const mode = useEngineSpectrum ? 'engine' : 'web';
    if (_npFftSpectrumSourceMode !== mode) {
        _npFftSpectrumSourceMode = mode;
        _npFftEngineAxisSrHz = null;
        _npFftEngineAxisFftSize = null;
        _npFftWebAxisSrHz = null;
        _npFftWebAxisFftSize = null;
    }
}

function getPinnedEngineSpectrumAxis() {
    if (typeof window === 'undefined' || !engineSpectrumLive()) {
        return null;
    }
    if (_npFftEngineAxisSrHz == null) {
        _npFftEngineAxisSrHz = typeof window._engineSpectrumSrHz === 'number' ? window._engineSpectrumSrHz : 44100;
        _npFftEngineAxisFftSize = typeof window._engineSpectrumFftSize === 'number' ? window._engineSpectrumFftSize : 2048;
    }
    return {sr: _npFftEngineAxisSrHz, fft: _npFftEngineAxisFftSize};
}

// ResizeObserver syncs canvas pixel buffer to container size on resize —
// NOT in the render loop (which would reset the bitmap every frame).
(function initFftCanvasResize() {
    const canvas = document.getElementById('npFftCanvas');
    if (!canvas) return;
    _npFftCanvas = canvas;
    _npFftCtx = canvas.getContext('2d');
    const parent = canvas.parentElement || canvas;
    let resizeRaf = null;
    function applyFftCanvasSize() {
        resizeRaf = null;
        const br = parent.getBoundingClientRect();
        let cw = Math.round(br.width);
        let ch = Math.round(br.height);
        if (cw < 2 || ch < 2) {
            cw = Math.max(2, parent.clientWidth || parseInt(canvas.getAttribute('width'), 10) || 600);
            ch = Math.max(2, parent.clientHeight || parseInt(canvas.getAttribute('height'), 10) || 48);
        }
        if (canvas.width === cw && canvas.height === ch) return;
        canvas.width = cw;
        canvas.height = ch;
        _npFftGrad = null; // rebuild gradient for new height
    }
    function scheduleFftCanvasSize() {
        if (resizeRaf != null) return;
        resizeRaf = requestAnimationFrame(applyFftCanvasSize);
    }
    /* Defer past the observer microtask so canvas dimension writes do not nest ResizeObserver loops (Chromium warning). */
    const ro = new ResizeObserver(() => requestAnimationFrame(scheduleFftCanvasSize));
    ro.observe(parent);
    if (typeof requestAnimationFrame === 'function') requestAnimationFrame(applyFftCanvasSize);
})();

function _renderNpFft() {
    const useEngineSpectrum = engineSpectrumLive();
    syncNpFftSpectrumAxisPins(useEngineSpectrum);
    if (!useEngineSpectrum) {
        ensureAudioGraph();
        if (
            _playbackCtx &&
            _playbackCtx.state === 'suspended' &&
            typeof isAudioPlaying === 'function' &&
            isAudioPlaying()
        ) {
            void _playbackCtx.resume();
        }
    }
    if (!useEngineSpectrum && !_analyser) return;
    const canvas = _npFftCanvas || document.getElementById('npFftCanvas');
    if (!canvas) return;
    const fftBox = canvas.getBoundingClientRect();
    if (fftBox.width < 2 || fftBox.height < 2) return;
    const ctx = _npFftCtx || canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    if (w === 0 || h === 0) return;
    if (isFftAnimationPaused()) return;
    let sampleRate = 44100;
    let fftSize = 2048;
    let binCount = 1024;
    if (useEngineSpectrum) {
        const axis = getPinnedEngineSpectrumAxis();
        if (!axis) return;
        if (!_npFftBuf || _npFftBuf.length < 1024) _npFftBuf = new Uint8Array(1024);
        _npFftBuf.set(window._engineSpectrumU8.subarray(0, 1024));
        sampleRate = axis.sr;
        fftSize = axis.fft;
        binCount = Math.min(1024, window._engineSpectrumU8.length);
    } else {
        if (!_npFftBuf) _npFftBuf = new Uint8Array(_analyser.frequencyBinCount);
        _analyser.getByteFrequencyData(_npFftBuf);
        if (_npFftWebAxisSrHz == null) {
            _npFftWebAxisSrHz = _playbackCtx ? _playbackCtx.sampleRate : 44100;
            _npFftWebAxisFftSize = _analyser.fftSize;
        }
        sampleRate = _npFftWebAxisSrHz;
        fftSize = _npFftWebAxisFftSize;
        binCount = _npFftBuf.length;
    }
    ctx.clearRect(0, 0, w, h);

    if (!_npFftGrad) {
        _npFftGrad = ctx.createLinearGradient(0, 0, 0, h);
        _npFftGrad.addColorStop(0, 'rgba(211,0,197,0.35)');
        _npFftGrad.addColorStop(0.5, 'rgba(5,217,232,0.18)');
        _npFftGrad.addColorStop(1, 'rgba(5,217,232,0.03)');
    }

    const fMin = 20;
    const fMax = sampleRate / 2;
    const logMin = Math.log10(fMin);
    const logMax = Math.log10(fMax);
    const specH = h - 10;

    function magAtFreq(freqHz) {
        const binF = (freqHz * fftSize) / sampleRate;
        const i0 = Math.floor(binF);
        if (i0 < 0) return 0;
        if (i0 >= binCount) return _npFftBuf[binCount - 1] / 255;
        const frac = binF - i0;
        const i1 = Math.min(binCount - 1, i0 + 1);
        const v0 = _npFftBuf[i0];
        const v1 = _npFftBuf[i1];
        return (v0 + (v1 - v0) * frac) / 255;
    }

    const maxCols = Math.min(Math.max(Math.floor(w * 2.5), 1), 3200);
    let nPts = 0;
    const maxPts = maxCols * 2 + 8;
    if (!_npFftPts || _npFftPts.length < maxPts) _npFftPts = new Float32Array(maxPts);
    const pts = _npFftPts;
    for (let c = 0; c < maxCols; c++) {
        const t = (c + 0.5) / maxCols;
        const logF = logMin + t * (logMax - logMin);
        const freq = Math.pow(10, logF);
        if (freq < fMin || freq > fMax) continue;
        const x = ((Math.log10(freq) - logMin) / (logMax - logMin)) * w;
        const mag = magAtFreq(freq);
        const y = specH - mag * (specH - 2);
        pts[nPts++] = x;
        pts[nPts++] = y;
    }

    if (nPts >= 2) {
        ctx.beginPath();
        ctx.moveTo(0, specH);
        ctx.lineTo(pts[0], pts[1]);
        for (let p = 2; p < nPts; p += 2) ctx.lineTo(pts[p], pts[p + 1]);
        ctx.lineTo(w, specH);
        ctx.closePath();
        ctx.fillStyle = _npFftGrad;
        ctx.fill();

        ctx.beginPath();
        ctx.moveTo(0, specH);
        for (let p = 0; p < nPts; p += 2) ctx.lineTo(pts[p], pts[p + 1]);
        ctx.strokeStyle = 'rgba(5,217,232,0.5)';
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    // Frequency scale labels along the bottom
    ctx.fillStyle = 'rgba(255,255,255,0.3)';
    ctx.font = '8px sans-serif';
    ctx.textAlign = 'center';
    for (const f of [50, 100, 200, 500, '1k', '2k', '5k', '10k', '20k']) {
        const hz = typeof f === 'string' ? parseFloat(f) * 1000 : f;
        if (hz < fMin || hz > fMax) continue;
        const x = ((Math.log10(hz) - logMin) / (logMax - logMin)) * w;
        ctx.fillText(typeof f === 'string' ? f : f + '', x, h - 1);
    }
    ctx.textAlign = 'start';
}

audioPlayer.addEventListener('play', () => {
    if (audioReverseMode && _bufPlaying) return;
    if (!_playbackRafId) _playbackRafId = requestAnimationFrame(_playbackRafLoop);
});
audioPlayer.addEventListener('pause', () => {
    if (audioReverseMode && _bufPlaying) return;
    if (_playbackRafId) {
        cancelAnimationFrame(_playbackRafId);
        _playbackRafId = null;
    }
    updatePlaybackTime(); // final position
});
audioPlayer.addEventListener('seeked', updatePlaybackTime);

// formatAudioSize and formatTime moved to utils.js

// ── Audio Similarity Search ──
async function findSimilarSamples(filePath) {
    showToast(toastFmt('toast.finding_similar_samples'));
    const name = filePath.split('/').pop().replace(/\.[^.]+$/, '');
    let existing = document.getElementById('similarPanel');
    if (existing) existing.remove();

    // Show floating panel (non-blocking, like audio player)
    const simDock = prefs.getItem('similarDock') || 'dock-bl';
    const simW = prefs.getItem('similarWidth');
    const simH = prefs.getItem('similarHeight');
    const simSizeStyle = (simW && simH) ? ` style="width:${simW}px;height:${simH}px;"` : '';
    const loadHtml = `<div class="similar-panel ${simDock}" id="similarPanel"${simSizeStyle}>
    <div class="sim-resize sim-resize-n" data-sim-resize="n"></div>
    <div class="sim-resize sim-resize-s" data-sim-resize="s"></div>
    <div class="sim-resize sim-resize-e" data-sim-resize="e"></div>
    <div class="sim-resize sim-resize-w" data-sim-resize="w"></div>
    <div class="sim-resize sim-resize-se" data-sim-resize="se"></div>
    <div class="sim-resize sim-resize-sw" data-sim-resize="sw"></div>
    <div class="sim-resize sim-resize-ne" data-sim-resize="ne"></div>
    <div class="sim-resize sim-resize-nw" data-sim-resize="nw"></div>
    <div class="sim-toolbar" id="simToolbar">
      <span class="sim-toolbar-title" title="Find Similar Samples">&#128270; Similar to "${escapeHtml(name)}"</span>
      <div class="sim-toolbar-actions">
        <button class="sim-toolbar-btn" data-action="minimizeSimilar" title="Minimize">&#9866;</button>
        <button class="sim-toolbar-btn btn-close" data-action="closeSimilar" title="Close">&#10005;</button>
      </div>
    </div>
    <div class="sim-body" id="simBody">
      <div style="text-align:center;padding:24px;">
        <div class="spinner" style="width:20px;height:20px;margin:0 auto 8px;"></div>
        <div id="similarStatusText" style="color:var(--text-muted);font-size:11px;">Analyzing fingerprints...</div>
        <div id="similarStatusDetail" style="color:var(--text-dim);font-size:9px;margin-top:4px;">Checking cache...</div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', loadHtml);
    initSimilarPanelDrag();

    let progressCleanup = null;
    try {
        if (window.__TAURI__?.event?.listen) {
            progressCleanup = await window.__TAURI__.event.listen('similarity-progress', (event) => {
                const d = event.payload;
                const statusText = document.getElementById('similarStatusText');
                const statusDetail = document.getElementById('similarStatusDetail');
                if (d.phase === 'computing' && statusText && statusDetail) {
                    const cached = d.cached_count ?? d.cached;
                    const uncached = d.uncached_count ?? d.total;
                    const total =
                        d.candidate_count ??
                        (typeof cached === 'number' && typeof uncached === 'number'
                            ? cached + uncached
                            : uncached);
                    statusText.textContent = _audioFmt('ui.audio.similar_fp_status', {
                        total,
                        uncached
                    });
                    statusDetail.textContent = _audioFmt('ui.audio.similar_fp_detail', {
                        cached,
                        uncached
                    });
                }
            });
        }
        const candidates = (typeof allAudioSamples !== 'undefined' ? allAudioSamples : []).map(s => s.path);
        const results = await window.vstUpdater.findSimilarSamples(filePath, candidates, 20);

        const panel = document.getElementById('similarPanel');
        if (!panel) return;
        const body = document.getElementById('simBody');

        if (results.length === 0) {
            body.innerHTML = `<div style="text-align:center;color:var(--text-muted);padding:16px;font-size:11px;">${escapeHtml(_audioFmt('ui.audio.similar_empty'))}</div>`;
            return;
        }

        body.innerHTML = `<div style="margin-bottom:6px;color:var(--text-muted);font-size:10px;padding:0 8px;">${escapeHtml(_audioFmt('ui.audio.similar_count', {n: results.length}))}</div>` +
            results.map(r => {
                const sampleName = r.path.split('/').pop().replace(/\.[^.]+$/, '');
                const ext = r.path.split('.').pop().toUpperCase();
                const sim = Math.round(r.similarity);
                const barColor = sim > 70 ? 'var(--green)' : sim > 40 ? 'var(--yellow)' : 'var(--red)';
                return `<div class="sim-result-row" data-similar-path="${escapeHtml(r.path)}" title="${escapeHtml(r.path)}">
          <span class="sim-result-name">${escapeHtml(sampleName)}</span>
          <span class="sim-result-ext">${ext}</span>
          <div class="sim-result-bar">
            <div class="sim-result-bar-fill" data-bar-pct="${sim}" style="width:0;background:${barColor};"></div>
          </div>
          <span class="sim-result-pct" style="color:${barColor};">${sim}%</span>
        </div>`;
            }).join('');
        // Defer bar widths until layout resolves
        requestAnimationFrame(() => {
            body.querySelectorAll('[data-bar-pct]').forEach(el => {
                el.style.width = el.dataset.barPct + '%';
                el.style.transition = 'width 0.3s ease-out';
            });
        });
    } catch (err) {
        const body = document.getElementById('simBody');
        if (body) body.innerHTML = `<div style="padding:16px;color:var(--red);font-size:11px;">${escapeHtml(_audioFmt('ui.audio.similar_error_prefix'))} ${escapeHtml(err.message || String(err))}</div>`;
    } finally {
        if (typeof progressCleanup === 'function') {
            try {
                progressCleanup();
            } catch (_unused) {
            }
        }
    }
}

function closeSimilarPanel() {
    if (_simDragAbort) {
        _simDragAbort.abort();
        _simDragAbort = null;
    }
    const panel = document.getElementById('similarPanel');
    if (panel) panel.remove();
}

function minimizeSimilarPanel() {
    const panel = document.getElementById('similarPanel');
    if (!panel) return;
    const body = document.getElementById('simBody');
    if (!body) return;
    body.style.display = body.style.display === 'none' ? '' : 'none';
}

// Similar panel drag + resize + snap (same pattern as audio player)
// AbortController kills all document-level listeners when the panel closes.
let _simDragAbort = null;

function initSimilarPanelDrag() {
    const panel = document.getElementById('similarPanel');
    if (!panel) return;
    const toolbar = document.getElementById('simToolbar');
    let dragging = false, startX, startY, origX, origY;

    // Abort previous listeners if panel was re-opened without closing
    if (_simDragAbort) _simDragAbort.abort();
    _simDragAbort = new AbortController();
    const sig = {signal: _simDragAbort.signal};

    function nearestDock(x, y) {
        const cx = window.innerWidth / 2, cy = window.innerHeight / 2;
        if (x < cx && y < cy) return 'dock-tl';
        if (x >= cx && y < cy) return 'dock-tr';
        if (x < cx && y >= cy) return 'dock-bl';
        return 'dock-br';
    }

    toolbar.addEventListener('mousedown', (e) => {
        if (e.target.closest('.sim-toolbar-actions')) return;
        if (e.button !== 0) return;
        e.preventDefault();
        dragging = true;
        const rect = panel.getBoundingClientRect();
        startX = e.clientX;
        startY = e.clientY;
        origX = rect.left;
        origY = rect.top;
        panel.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
        panel.style.left = origX + 'px';
        panel.style.top = origY + 'px';
        panel.style.right = 'auto';
        panel.style.bottom = 'auto';
        panel.classList.add('dragging');
        document.body.style.userSelect = 'none';
    }, sig);

    document.addEventListener('mousemove', (e) => {
        if (!dragging) return;
        panel.style.left = (origX + e.clientX - startX) + 'px';
        panel.style.top = (origY + e.clientY - startY) + 'px';
    }, sig);

    document.addEventListener('mouseup', (e) => {
        if (!dragging) return;
        dragging = false;
        panel.classList.remove('dragging');
        document.body.style.userSelect = '';
        const dock = nearestDock(e.clientX, e.clientY);
        panel.style.left = '';
        panel.style.top = '';
        panel.style.right = '';
        panel.style.bottom = '';
        panel.classList.add(dock);
        prefs.setItem('similarDock', dock);
    }, sig);

    // Resize via edge handles
    let resizing = null;
    panel.addEventListener('mousedown', (e) => {
        const handle = e.target.closest('[data-sim-resize]');
        if (!handle) return;
        e.preventDefault();
        e.stopPropagation();
        const rect = panel.getBoundingClientRect();
        panel.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
        panel.style.left = rect.left + 'px';
        panel.style.top = rect.top + 'px';
        panel.style.right = 'auto';
        panel.style.bottom = 'auto';
        panel.style.width = rect.width + 'px';
        panel.style.height = rect.height + 'px';
        document.body.style.userSelect = 'none';
        resizing = {
            edge: handle.dataset.simResize,
            startX: e.clientX,
            startY: e.clientY,
            origLeft: rect.left,
            origTop: rect.top,
            origW: rect.width,
            origH: rect.height
        };
    }, sig);

    document.addEventListener('mousemove', (e) => {
        if (!resizing) return;
        const s = resizing, dx = e.clientX - s.startX, dy = e.clientY - s.startY;
        let l = s.origLeft, t = s.origTop, w = s.origW, h = s.origH;
        if (s.edge.includes('e')) w = Math.max(240, s.origW + dx);
        if (s.edge.includes('w')) {
            w = Math.max(240, s.origW - dx);
            l = s.origLeft + s.origW - w;
        }
        if (s.edge.includes('s')) h = Math.max(150, s.origH + dy);
        if (s.edge.includes('n')) {
            h = Math.max(150, s.origH - dy);
            t = s.origTop + s.origH - h;
        }
        panel.style.left = l + 'px';
        panel.style.top = t + 'px';
        panel.style.width = w + 'px';
        panel.style.height = h + 'px';
    }, sig);

    document.addEventListener('mouseup', () => {
        if (resizing) {
            const rect = panel.getBoundingClientRect();
            prefs.setItem('similarWidth', Math.round(rect.width));
            prefs.setItem('similarHeight', Math.round(rect.height));
            resizing = null;
            document.body.style.userSelect = '';
        }
    }, sig);
}

// Similar panel event delegation
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="closeSimilar"]')) {
        closeSimilarPanel();
        return;
    }
    if (e.target.closest('[data-action="minimizeSimilar"]')) {
        minimizeSimilarPanel();
        return;
    }
    const row = e.target.closest('[data-similar-path]');
    if (row && document.getElementById('similarPanel')) {
        const path = row.dataset.similarPath;
        if (path && typeof previewAudio === 'function') previewAudio(path);
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('similarPanel')) {
        closeSimilarPanel();
    }
});

function closeMetaRow() {
    const meta = document.getElementById('audioMetaRow');
    if (meta) meta.remove();
    const expanded = document.querySelector('tr.row-expanded');
    if (expanded) expanded.classList.remove('row-expanded');
    expandedMetaPath = null;
}

function getFormatClass(format) {
    const f = format.toLowerCase();
    if (
        [
            'wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'aac', 'opus', 'wma', 'rex', 'rx2', 'sf2', 'sfz',
        ].includes(f)
    ) {
        return 'format-' + f;
    }
    return 'format-default';
}

// `unifiedResult` is an optional promise provided by scanAll() that resolves
// to `{ samples, roots, stopped }` from a shared scan_unified backend call.
// When provided, this function skips its own Tauri invoke and reuses the
// shared result — so the filesystem is walked once instead of 4 times.
async function scanAudioSamples(resume = false, unifiedResult = null, overrideRoots = null) {
    stopBackgroundAnalysis();
    showGlobalProgress();
    const btn = document.getElementById('btnScanAudio');
    const resumeBtn = document.getElementById('btnResumeAudio');
    const stopBtn = document.getElementById('btnStopAudio');
    const progressBar = document.getElementById('audioProgressBar');
    const progressFill = document.getElementById('audioProgressFill');
    const tableWrap = document.getElementById('audioTableWrap');

    const excludePaths = resume ? allAudioSamples.map(s => s.path) : null;

    if (typeof btnLoading === 'function') btnLoading(btn, true);
    btn.disabled = true;
    btn.innerHTML = '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn');
    resumeBtn.style.display = 'none';
    stopBtn.style.display = '';
    progressBar.classList.add('active');
    progressFill.style.width = '0%';

    if (!resume) {
        _audioScanDbView = false;
        allAudioSamples = [];
        filteredAudioSamples = [];
        resetAudioStats();
    }

    /** Stream DOM only while table was empty at scan start; otherwise DB-only (preserves selection). */
    let scanStreamDomActive = false;
    let pendingScanClear = !resume;
    let firstAudioBatch = true;
    let pendingSamples = [];
    let pendingFound = 0;
    _audioScanActive = true;
    const audioEta = createETA();
    audioEta.start();
    const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);

    function flushPendingSamples() {
        if (pendingSamples.length === 0) return;

        const audioElapsed = audioEta.elapsed();
        btn.innerHTML = catalogFmt('ui.audio.scan_progress_line', {
            n: pendingFound.toLocaleString(),
            elapsed: audioElapsed ? ' — ' + audioElapsed : ''
        });
        progressFill.style.width = '';
        progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

        const allowDom =
            scanStreamDomActive ||
            (typeof isAudioScanTableEmpty === 'function' && isAudioScanTableEmpty());
        if (!allowDom) {
            pendingSamples = [];
            return;
        }
        scanStreamDomActive = true;

        if (pendingScanClear && pendingSamples.length > 0) {
            pendingScanClear = false;
            allAudioSamples = [];
            filteredAudioSamples = [];
            expandedMetaPath = null;
            resetAudioStats();
            const statsEl = document.getElementById('audioStats');
            if (statsEl) statsEl.style.display = 'none';
        }

        if (firstAudioBatch) {
            firstAudioBatch = false;
            tableWrap.innerHTML = '';
            initAudioTable();
        }

        const toAdd = pendingSamples;
        pendingSamples = [];

        allAudioSamples.push(...toAdd);
        if (allAudioSamples.length > 100000) allAudioSamples.length = 100000;
        accumulateAudioStats(toAdd);
        if (!_bgAnalysisRunning && prefs.getItem('autoAnalysis') !== 'off') startBackgroundAnalysis();

        const search = document.getElementById('audioSearchInput').value || '';
        const scanFmtSet = getMultiFilterValues('audioFormatFilter');
        const scanMode = getSearchMode('regexAudio');
        const matching = toAdd.filter(s => {
            if (scanFmtSet && !scanFmtSet.has(s.format)) return false;
            if (search && !searchMatch(search, [s.name, s.path, s.format], scanMode)) return false;
            return true;
        });
        if (matching.length > 0) {
            filteredAudioSamples.push(...matching);
            if (filteredAudioSamples.length > 100000) filteredAudioSamples.length = 100000;
            if (!_audioScanDbView) {
                const tbody = document.getElementById('audioTableBody');
                if (tbody && audioRenderCount < 2000) {
                    const loadMore = document.getElementById('audioLoadMore');
                    if (loadMore) loadMore.remove();
                    const toRender = matching.slice(0, 2000 - audioRenderCount);
                    tbody.insertAdjacentHTML('beforeend', toRender.map(buildAudioRow).join(''));
                    if (typeof reorderNewTableRows === 'function') reorderNewTableRows('audioTable');
                    audioRenderCount += toRender.length;
                }
            }
        }

        updateAudioStats();
    }

    const scheduleFlush = createScanFlusher(flushPendingSamples, FLUSH_INTERVAL);

    if (audioScanProgressCleanup) audioScanProgressCleanup();
    audioScanProgressCleanup = await window.vstUpdater.onAudioScanProgress((data) => {
        if (data.phase === 'status') {
            // status message
        } else if (data.phase === 'scanning') {
            pendingSamples.push(...data.samples);
            pendingFound = data.found;
            window.__audioScanPendingFound = pendingFound;
            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({samples: pendingFound});
            else document.getElementById('sampleCount').textContent = pendingFound.toLocaleString();
            scheduleFlush();
        }
    });

    try {
        const audioRoots = (overrideRoots && overrideRoots.length > 0)
            ? overrideRoots
            : (prefs.getItem('audioScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
        const result = unifiedResult
            ? await unifiedResult
            : await window.vstUpdater.scanAudioSamples(audioRoots.length ? audioRoots : undefined, excludePaths);
        if (audioScanProgressCleanup) {
            audioScanProgressCleanup();
            audioScanProgressCleanup = null;
        }
        flushPendingSamples();
        if (pendingScanClear) {
            pendingScanClear = false;
            allAudioSamples = [];
            filteredAudioSamples = [];
            expandedMetaPath = null;
            resetAudioStats();
        }
        scanStreamDomActive = false;
        // Save scan results to SQLite (backend already streamed-saved when result.streamed)
        if (!result.streamed) {
            try {
                await window.vstUpdater.saveAudioScan(result.samples || [], result.roots);
            } catch (e) {
                showToast(toastFmt('toast.failed_save_audio_history', {err: e.message || e}), 4000, 'error');
            }
        }
        // Fetch first page from DB (no in-memory array needed)
        audioCurrentOffset = 0;
        await rebuildAudioStats(true);
        await fetchAudioPage();
        if (prefs.getItem('autoAnalysis') !== 'off') startBackgroundAnalysis();
        if (result.stopped && audioTotalUnfiltered > 0) {
            resumeBtn.style.display = '';
        }
        if (typeof postScanCompleteToast === 'function') {
            const n = audioTotalUnfiltered || 0;
            postScanCompleteToast(
                !!result.stopped,
                'toast.post_scan_samples_complete',
                'toast.post_scan_samples_stopped',
                {n: n.toLocaleString()},
            );
        }
    } catch (err) {
        if (audioScanProgressCleanup) {
            audioScanProgressCleanup();
            audioScanProgressCleanup = null;
        }
        _audioScanDbView = false;
        flushPendingSamples();
        if (pendingScanClear) {
            pendingScanClear = false;
            allAudioSamples = [];
            filteredAudioSamples = [];
            expandedMetaPath = null;
            resetAudioStats();
        }
        scanStreamDomActive = false;
        const errMsg = err.message || err || catalogFmt('toast.unknown_error');
        tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>${typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.scan_error_title')) : _audioFmt('ui.audio.scan_error_title')}</h2><p>${typeof escapeHtml === 'function' ? escapeHtml(errMsg) : errMsg}</p></div>`;
        showToast(toastFmt('toast.audio_scan_failed', {errMsg}), 4000, 'error');
    }

    window.__audioScanPendingFound = 0;
    _audioScanActive = false;
    scanStreamDomActive = false;
    _audioScanDbView = false;
    hideGlobalProgress();
    btn.disabled = false;
    if (typeof btnLoading === 'function') btnLoading(btn, false);
    btn.innerHTML = catalogFmt('ui.btn.127925_scan_samples');
    stopBtn.style.display = 'none';
    progressBar.classList.remove('active');
    progressFill.style.width = '0%';
    progressFill.style.animation = '';
}

async function stopAudioScan() {
    await window.vstUpdater.stopAudioScan();
}

// Running stat counters — avoid re-scanning the full array every flush
let audioStatCounts = {};
let audioStatBytes = 0;

function resetAudioStats() {
    audioStatCounts = {};
    audioStatBytes = 0;
}

function accumulateAudioStats(samples) {
    for (const s of samples) {
        if (!s) continue;
        audioStatCounts[s.format] = (audioStatCounts[s.format] || 0) + 1;
        audioStatBytes += s.size || 0;
    }
}

function updateAudioStats() {
    const stats = document.getElementById('audioStats');
    stats.style.display = 'flex';
    const wav = audioStatCounts['WAV'] || 0;
    const mp3 = audioStatCounts['MP3'] || 0;
    const aiff = (audioStatCounts['AIFF'] || 0) + (audioStatCounts['AIF'] || 0);
    const flac = audioStatCounts['FLAC'] || 0;
    const mainFormats = wav + mp3 + aiff + flac;
    const total = audioTotalCount || audioTotalUnfiltered || 0;
    const unfiltered = audioTotalUnfiltered || 0;
    const isFiltered = unfiltered > 0 && total > 0 && total < unfiltered;
    const totalPart = audioTotalCountCapped ? total.toLocaleString() + '+' : total.toLocaleString();
    const totalStr = isFiltered ? totalPart + ' / ' + unfiltered.toLocaleString() : totalPart;
    document.getElementById('audioTotalCount').textContent = totalStr;
    document.getElementById('audioWavCount').textContent = wav.toLocaleString();
    document.getElementById('audioMp3Count').textContent = mp3.toLocaleString();
    document.getElementById('audioAiffCount').textContent = aiff.toLocaleString();
    document.getElementById('audioFlacCount').textContent = flac.toLocaleString();
    document.getElementById('audioOtherCount').textContent = Math.max(0, total - mainFormats).toLocaleString();
    document.getElementById('audioTotalSize').textContent = formatAudioSize(audioStatBytes);
    if (!_audioScanActive) {
        const sc = unfiltered || total;
        if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({samples: sc});
        else document.getElementById('sampleCount').textContent = sc.toLocaleString();
    }
    document.getElementById('btnExportAudio').style.display = total > 0 ? '' : 'none';
    if (typeof updateAudioDiskUsage === 'function') updateAudioDiskUsage();
}

let _lastAudioAggKey = null;
let _pendingAudioAggKey = '';
/** Debounce heavy `GROUP BY format` stats IPC so typing a filter does not block the UI on 200k+ rows. */
let _audioStatsDebounceTimer = null;
const AUDIO_STATS_DEBOUNCE_MS = 450;

async function rebuildAudioStats(force) {
    try {
        const search = _lastAudioSearch || '';
        const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('audioFormatFilter') : null;
        const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
        const key = search + '|' + (formatFilter || '');
        // Skip the aggregate query if filter/search hasn't changed (e.g. load-more, sort).
        if (!force && key === _lastAudioAggKey) {
            updateAudioStats();
            return;
        }
        _pendingAudioAggKey = key;
        updateAudioStats();

        if (force) {
            if (_audioStatsDebounceTimer !== null) {
                clearTimeout(_audioStatsDebounceTimer);
                _audioStatsDebounceTimer = null;
            }
            await _runAudioFilterStatsAgg();
            return;
        }
        if (_audioStatsDebounceTimer !== null) {
            clearTimeout(_audioStatsDebounceTimer);
        }
        _audioStatsDebounceTimer = setTimeout(() => {
            _audioStatsDebounceTimer = null;
            void _runAudioFilterStatsAgg();
        }, AUDIO_STATS_DEBOUNCE_MS);
    } catch {
        resetAudioStats();
        updateAudioStats();
    }
}

async function _runAudioFilterStatsAgg() {
    try {
        const search = _lastAudioSearch || '';
        const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('audioFormatFilter') : null;
        const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
        const key = search + '|' + (formatFilter || '');
        if (key !== _pendingAudioAggKey) {
            return;
        }
        const agg = await window.vstUpdater.dbAudioFilterStats(
            search,
            formatFilter,
            _lastAudioMode === 'regex',
        );
        if (key !== _pendingAudioAggKey) {
            return;
        }
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (key !== _pendingAudioAggKey) {
            return;
        }
        _lastAudioAggKey = key;
        audioStatCounts = agg.byType || {};
        audioStatBytes = agg.totalBytes || 0;
        audioTotalCount = agg.count || 0;
        audioTotalCountCapped = agg.countCapped === true;
        audioTotalUnfiltered = agg.totalUnfiltered || 0;
        _audioBytesByType = agg.bytesByType || {};
        updateAudioStats();
    } catch {
        resetAudioStats();
        updateAudioStats();
    }
}

async function backfillAudioMeta(samples) {
    if (!samples || !samples.length) return;
    const missing = samples.filter(s => s.duration == null && s.channels == null).map(s => s.path);
    if (!missing.length) return;
    try {
        const updated = await window.vstUpdater.dbBackfillAudioMeta(missing);
        if (!updated || !Object.keys(updated).length) return;
        let changed = false;
        for (const s of filteredAudioSamples) {
            const u = updated[s.path];
            if (!u) continue;
            if (u.duration != null) s.duration = u.duration;
            if (u.channels != null) s.channels = u.channels;
            if (u.sampleRate != null) s.sampleRate = u.sampleRate;
            if (u.bitsPerSample != null) s.bitsPerSample = u.bitsPerSample;
            changed = true;
        }
        if (changed) renderAudioTable();
    } catch { /* backfill is best-effort */
    }
}

function initAudioTable() {
    const tableWrap = document.getElementById('audioTableWrap');
    const t = _audioFmt;
    const tAttr = (key) => {
        const s = t(key);
        return typeof escapeHtml === 'function' ? escapeHtml(s) : s;
    };
    tableWrap.innerHTML = `<table class="audio-table" id="audioTable">
    <thead>
      <tr>
        <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${tAttr('ui.audio.th_select_all')}"></th>
        <th data-action="sortAudio" data-key="name" style="width: 22%;" title="${tAttr('ui.audio.tt_sort_name')}">${t('ui.audio.th_name')} <span class="sort-arrow" id="sortArrowName">&#9660;</span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="format" class="col-format" style="width: 60px;" title="${tAttr('ui.audio.tt_sort_format')}">${t('ui.audio.th_format')} <span class="sort-arrow" id="sortArrowFormat"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="size" class="col-size" style="width: 75px;" title="${tAttr('ui.audio.tt_sort_size')}">${t('ui.audio.th_size')} <span class="sort-arrow" id="sortArrowSize"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="bpm" class="col-bpm" style="width: 55px;" title="${tAttr('ui.audio.tt_sort_bpm')}">${t('ui.audio.th_bpm')} <span class="sort-arrow" id="sortArrowBpm"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="key" class="col-key" style="width: 75px;" title="${tAttr('ui.audio.tt_sort_key')}">${t('ui.audio.th_key')} <span class="sort-arrow" id="sortArrowKey"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="duration" class="col-dur" style="width: 55px;" title="${tAttr('ui.audio.tt_sort_duration')}">${t('ui.audio.th_dur')} <span class="sort-arrow" id="sortArrowDuration"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="channels" class="col-ch" style="width: 40px;" title="${tAttr('ui.audio.tt_sort_channels')}">${t('ui.audio.th_ch')} <span class="sort-arrow" id="sortArrowChannels"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="lufs" class="col-lufs" style="width: 55px;" title="${tAttr('ui.audio.tt_sort_lufs')}">${t('ui.audio.th_lufs')} <span class="sort-arrow" id="sortArrowLufs"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="modified" class="col-date" style="width: 90px;" title="${tAttr('ui.audio.tt_sort_modified')}">${t('ui.audio.th_modified')} <span class="sort-arrow" id="sortArrowModified"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="directory" style="width: 22%;" title="${tAttr('ui.audio.tt_sort_path')}">${t('ui.audio.th_path')} <span class="sort-arrow" id="sortArrowDirectory"></span><span class="col-resize"></span></th>
        <th class="col-actions" style="width: 130px;"></th>
      </tr>
    </thead>
    <tbody id="audioTableBody"></tbody>
  </table>`;
    initColumnResize(document.getElementById('audioTable'));
    if (typeof initTableColumnReorder === 'function') initTableColumnReorder('audioTable', 'audioColumnOrder');
}

let _lastAudioSearch = '';
let _lastAudioMode = 'fuzzy';

registerFilter('filterAudioSamples', {
    inputId: 'audioSearchInput',
    regexToggleId: 'regexAudio',
    formatDropdownId: 'audioFormatFilter',
    // Slightly longer than default 250ms: at 3+ chars the backend uses FTS5 MATCH (heavier than LIKE).
    debounceMs: 400,
    resetOffset() {
        audioCurrentOffset = 0;
    },
    fetchFn() {
        _lastAudioSearch = this.lastSearch || '';
        _lastAudioMode = this.lastMode || 'fuzzy';
        fetchAudioPage();
    },
});

function filterAudioSamples() {
    applyFilter('filterAudioSamples');
}

function showAudioQueryLoading(isLoadMore) {
    if (!document.getElementById('audioTableBody')) return;
    const label = typeof _audioFmt === 'function' ? _audioFmt('ui.audio.query_loading') : queryLoadingLabel();
    showTableQueryLoadingRow({
        tbodyId: 'audioTableBody',
        rowId: 'audioQueryLoadingRow',
        tableId: 'audioTable',
        colspan: 12,
        append: isLoadMore,
        label,
    });
}

function clearAudioQueryLoadingRow() {
    clearTableQueryLoadingRow('audioQueryLoadingRow', 'audioTable');
}

/** Full list for export when SQLite-backed UI has left `allAudioSamples` empty (paginated DB model). */
const _AUDIO_EXPORT_MAX = 100000;

async function fetchAudioSamplesForExport() {
    const search = _lastAudioSearch || '';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('audioFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    let total = audioTotalCount || 0;
    if (total <= 0) {
        try {
            const probe = await window.vstUpdater.dbQueryAudio({
                search: search || null,
                search_regex: _lastAudioMode === 'regex',
                format_filter: formatFilter,
                sort_key: audioSortKey,
                sort_asc: audioSortAsc,
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch {
            return [];
        }
    }
    const n = Math.min(total, _AUDIO_EXPORT_MAX);
    if (n <= 0) return [];
    const result = await window.vstUpdater.dbQueryAudio({
        search: search || null,
        search_regex: _lastAudioMode === 'regex',
        format_filter: formatFilter,
        sort_key: audioSortKey,
        sort_asc: audioSortAsc,
        offset: 0,
        limit: n,
    });
    let samples = result.samples || [];
    if (search && samples.length > 1 && typeof searchScore === 'function') {
        const scored = samples.map((s) => ({s, score: searchScore(search, [s.name], _lastAudioMode)}));
        scored.sort((a, b) => b.score - a.score);
        samples = scored.map((x) => x.s);
    }
    return samples;
}

async function fetchAudioPage() {
    const search = _lastAudioSearch || '';
    const fmtSet = getMultiFilterValues('audioFormatFilter');
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    const seq = ++_audioQuerySeq;
    const isLoadMore = audioCurrentOffset > 0;
    showAudioQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('audioSearchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        const result = await window.vstUpdater.dbQueryAudio({
            search: search || null,
            search_regex: _lastAudioMode === 'regex',
            format_filter: formatFilter,
            sort_key: audioSortKey,
            sort_asc: audioSortAsc,
            offset: audioCurrentOffset,
            limit: AUDIO_PAGE_SIZE,
        });
        if (seq !== _audioQuerySeq) return;
        filteredAudioSamples = result.samples || [];
        audioTotalCount = result.totalCount || 0;
        audioTotalCountCapped = result.totalCountCapped === true;
        audioTotalUnfiltered = result.totalUnfiltered || 0;
        // Re-sort by fzf relevance score
        if (search && filteredAudioSamples.length > 1) {
            const scored = filteredAudioSamples.map(s => ({s, score: searchScore(search, [s.name], _lastAudioMode)}));
            scored.sort((a, b) => b.score - a.score);
            filteredAudioSamples = scored.map(x => x.s);
        }
        // Let the browser process pending input/paint before `innerHTML` + row work (can take tens of ms).
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _audioQuerySeq) return;
        renderAudioTable();
        if (audioScanProgressCleanup) _audioScanDbView = true;
        // Header totals from paginated query (fast); per-format breakdown debounced.
        updateAudioStats();
        if (typeof requestIdleCallback === 'function') {
            requestIdleCallback(() => {
                void rebuildAudioStats();
            });
        } else {
            setTimeout(() => {
                void rebuildAudioStats();
            }, 0);
        }
        // Backfill duration/channels for rows missing metadata (legacy scans)
        backfillAudioMeta(filteredAudioSamples);
    } catch (e) {
        if (seq !== _audioQuerySeq) return;
        clearAudioQueryLoadingRow();
        if (typeof showToast === 'function') showToast(toastFmt('toast.audio_query_failed', {err: e.message || e}), 4000, 'error');
        if (audioCurrentOffset === 0) {
            renderAudioTable();
        }
    } finally {
        if (seq === _audioQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('audioSearchInput', false);
    }
}

function sortAudio(key) {
    if (audioSortKey === key) {
        audioSortAsc = !audioSortAsc;
    } else {
        audioSortKey = key;
        audioSortAsc = true;
    }
    ['Name', 'Format', 'Size', 'Bpm', 'Key', 'Duration', 'Channels', 'Lufs', 'Modified', 'Directory'].forEach(k => {
        const el = document.getElementById('sortArrow' + k);
        if (el) {
            const isActive = k.toLowerCase() === audioSortKey;
            el.innerHTML = isActive ? (audioSortAsc ? '&#9650;' : '&#9660;') : '';
            el.closest('th')?.classList.toggle('sort-active', isActive);
        }
    });
    audioCurrentOffset = 0;
    fetchAudioPage();
    if (typeof saveSortState === 'function') saveSortState('audio', audioSortKey, audioSortAsc);
}

// Legacy — no longer needed, sort happens in SQL
function sortAudioArray() {
}

let audioRenderCount = 0;

function renderAudioTable() {
    if (!document.getElementById('audioTable')) initAudioTable();
    const tbody = document.getElementById('audioTableBody');
    if (!tbody) return;
    const loadingRow = document.getElementById('audioQueryLoadingRow');
    if (loadingRow) loadingRow.remove();
    const tblBusy = document.getElementById('audioTable');
    if (tblBusy) tblBusy.removeAttribute('aria-busy');
    audioRenderCount = audioCurrentOffset + filteredAudioSamples.length;
    if (audioCurrentOffset === 0) {
        tbody.innerHTML = filteredAudioSamples.map(buildAudioRow).join('');
    } else {
        const loadMore = document.getElementById('audioLoadMore');
        if (loadMore) loadMore.remove();
        tbody.insertAdjacentHTML('beforeend', filteredAudioSamples.map(buildAudioRow).join(''));
    }
    if (typeof reorderNewTableRows === 'function') reorderNewTableRows('audioTable');
    const hasMore = audioTotalCountCapped
        ? (filteredAudioSamples.length === AUDIO_PAGE_SIZE)
        : (audioRenderCount < audioTotalCount);
    if (hasMore) {
        appendLoadMore(tbody);
    }
}

function appendLoadMore(tbody) {
    const totalShown = audioTotalCountCapped ? audioTotalCount.toLocaleString() + '+' : audioTotalCount.toLocaleString();
    const line = catalogFmt('ui.audio.load_more_hint', {
        shown: audioRenderCount.toLocaleString(),
        total: totalShown,
    });
    tbody.insertAdjacentHTML('beforeend',
        `<tr id="audioLoadMore"><td colspan="12" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreAudio">
      ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
    </td></tr>`);
}

function loadMoreAudio() {
    audioCurrentOffset = audioRenderCount;
    fetchAudioPage();
}

/** BPM column text + title; when `bpmExhausted`, shows localized N/A and ignores stale BPM cache. */
function bpmCellDisplayAndTitle(s) {
    const bpmExhausted = s.bpmExhausted === true;
    const raw = !bpmExhausted
        ? (s.bpm != null && s.bpm !== ''
            ? s.bpm
            : (typeof _bpmCache !== 'undefined' && _bpmCache[s.path] != null && _bpmCache[s.path] !== ''
                ? _bpmCache[s.path]
                : null))
        : null;
    const display = bpmExhausted
        ? (typeof _audioFmt === 'function' ? _audioFmt('ui.audio.bpm_na') : 'N/A')
        : (raw != null && raw !== '' ? String(raw) : '');
    const titleRaw = bpmExhausted
        ? (typeof _audioFmt === 'function' ? _audioFmt('ui.audio.tt_cell_bpm_exhausted') : '')
        : (display !== ''
            ? _audioFmt('ui.audio.tt_cell_bpm', {bpm: raw})
            : _audioFmt('ui.audio.tt_cell_click_analyze'));
    return { display, titleRaw };
}

function buildAudioRow(s) {
    const fmtClass = getFormatClass(s.format);
    const hp = escapeHtml(s.path);
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (x) => String(x);
    const isPlaying = audioPlayerPath === s.path;
    const rowClass = isPlaying ? ' class="row-playing"' : '';
    const checked =
        typeof batchSetForTabId === 'function' && batchSetForTabId('tabSamples').has(s.path) ? ' checked' : '';
    const { display: bpmDisplay, titleRaw: bpmTitleRaw } = bpmCellDisplayAndTitle(s);
    const bpmTitle = esc(bpmTitleRaw);
    const key = s.key || (typeof _keyCache !== 'undefined' && _keyCache[s.path]) || '';
    const dur = s.duration ? (typeof formatTime === 'function' ? formatTime(s.duration) : s.duration.toFixed(1) + 's') : '';
    const ch = s.channels ? (s.channels === 1 ? 'M' : s.channels === 2 ? 'S' : s.channels + 'ch') : (s.sampleRate ? '?' : '');
    const lufs = s.lufs != null ? s.lufs : (typeof _lufsCache !== 'undefined' && _lufsCache[s.path] != null) ? _lufsCache[s.path] : '';
    const keyTitle = key ? esc(key) : esc(_audioFmt('ui.audio.tt_cell_click_analyze'));
    const lufsTitle = lufs !== ''
        ? (lufs < -25
            ? esc(_audioFmt('ui.audio.tt_lufs_quiet', {lufs}))
            : esc(_audioFmt('ui.audio.tt_lufs_line', {lufs})))
        : esc(_audioFmt('ui.audio.tt_cell_click_analyze'));
    const chTitle = ch === 'M' ? esc(_audioFmt('ui.tt.mono')) : ch === 'S' ? esc(_audioFmt('ui.btn.stereo')) : esc(String(ch));
    const previewBtnT = esc(_audioFmt('ui.audio.row_btn_preview'));
    const loopBtnT = esc(_audioFmt('ui.audio.row_btn_loop'));
    const revealBtnT = esc(_audioFmt('menu.reveal_in_finder'));
    return `<tr${rowClass} data-audio-path="${hp}" data-audio-format="${escapeHtml(s.format)}" data-audio-name="${escapeHtml((s.name || '').toLowerCase())}" data-action="toggleMetadata" data-path="${hp}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(s.name)}">${_lastAudioSearch ? highlightBasenameFromPath(s.path, s.name, _lastAudioSearch, _lastAudioMode) : escapeHtml(s.name)}${typeof rowBadges === 'function' ? rowBadges(s.path) : ''}</td>
    <td class="col-format"><span class="format-badge ${fmtClass}">${_lastAudioSearch ? highlightMatch(s.format, _lastAudioSearch, _lastAudioMode) : escapeHtml(s.format)}</span></td>
    <td class="col-size">${s.sizeFormatted}</td>
    <td class="col-bpm" title="${bpmTitle}">${typeof escapeHtml === 'function' ? escapeHtml(bpmDisplay) : bpmDisplay}</td>
    <td class="col-key" title="${keyTitle}">${escapeHtml(key)}</td>
    <td class="col-dur" title="${dur ? esc(dur) : ''}">${dur}</td>
    <td class="col-ch" title="${chTitle}">${ch}</td>
    <td class="col-lufs${lufs !== '' && lufs < -25 ? ' lufs-low' : ''}" title="${lufsTitle}">${lufs}</td>
    <td class="col-date">${s.modified}</td>
    <td class="col-path" title="${hp}">${_lastAudioSearch ? highlightPathPrefixFromPath(s.path, s.directory, _lastAudioSearch, _lastAudioMode) : escapeHtml(s.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${hp}" title="${previewBtnT}">
        ${isPlaying && isAudioPlaying() ? '&#9646;&#9646;' : '&#9654;'}
      </button>
      <button class="btn-small btn-loop${isPlaying && audioLooping ? ' active' : ''}" data-action="toggleRowLoop" data-path="${hp}" title="${loopBtnT}">&#8634;</button>
      <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${hp}" title="${revealBtnT}">&#128193;</button>
    </td>
    </tr>`;
}

async function showEngineUnplayablePreview(filePath) {
    if (_enginePlaybackActive && typeof window !== 'undefined' && typeof window.enginePlaybackStop === 'function') {
        await window.enginePlaybackStop();
        setEnginePlaybackActive(false);
    }
    stopReverseBufferPlayback();
    restoreWebViewAudioAfterEngine();
    audioReverseMode = false;
    syncReversePlaybackButtons(false);
    _decodedBuf = null;
    _reversedBuf = null;
    _decodedBufPath = null;
    _pausedOffsetInRev = 0;
    if (typeof audioPlayer !== 'undefined' && audioPlayer) {
        audioPlayer.pause();
        audioPlayer.src = '';
    }
    if (typeof window !== 'undefined') {
        window._enginePlaybackResumePath = '';
    }
    audioPlayerPath = filePath;
    audioPlayer.loop = false;
    const np = document.getElementById('audioNowPlaying');
    if (!np) return;
    np.classList.add('active');
    np.classList.remove('np-playing');
    if (prefs.getItem('playerExpanded') === 'on') {
        np.classList.add('expanded');
        renderRecentlyPlayed();
    }
    const sample = findByPath(allAudioSamples, filePath);
    const displayName = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
    const npName = document.getElementById('npName');
    if (npName) npName.textContent = displayName;
    const npTime = document.getElementById('npTime');
    if (npTime) npTime.textContent = catalogFmt('ui.audio.player_time_zero');
    const npProgress = document.getElementById('npProgress');
    if (npProgress) npProgress.style.width = '0%';
    const npCursor = document.getElementById('npCursor');
    if (npCursor) npCursor.style.display = 'none';
    cancelIdleSchedule(_npWaveformIdleId);
    _npWaveformIdleId = null;
    _npWaveformDrawSeq++;
    const canvas = document.getElementById('npWaveformCanvas');
    if (canvas) {
        try {
            const ctx = canvas.getContext('2d');
            if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
        } catch (_) {
        }
    }
    updateMetaLine();
    updatePlayBtnStates();
    updateNowPlayingBtn();
    updateFavBtn();
    updateNoteBtn();
}

// ── Audio Preview / Playback ──
async function previewAudio(filePath) {
    const ext = filePath.split('.').pop().toLowerCase();
    if (isEngineUnplayablePath(filePath)) {
        await showEngineUnplayablePreview(filePath);
        return;
    }

    if (audioPlayerPath === filePath && isAudioPlaying()) {
        if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
            applyEnginePlaybackPausedFromTransport(true);
            void window.vstUpdater.audioEngineInvoke({cmd: 'playback_pause', paused: true});
        } else if (audioReverseMode) pauseReverseBufferPlayback();
        else audioPlayer.pause();
        updatePlayBtnStates();
        updateNowPlayingBtn();
        if (typeof window.syncAeTransportFromPlayback === 'function') window.syncAeTransportFromPlayback();
        return;
    }

    if (audioPlayerPath === filePath && !isAudioPlaying()) {
        if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
            await window.vstUpdater.audioEngineInvoke({cmd: 'playback_pause', paused: false});
            applyEnginePlaybackPausedFromTransport(false);
        } else if (audioReverseMode) {
            startReverseBufferFromOffset(_pausedOffsetInRev);
        } else {
            if (_playbackCtx && _playbackCtx.state === 'suspended') {
                await _playbackCtx.resume().catch(e => {
                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                });
            }
            await audioPlayer.play().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            });
        }
        updatePlayBtnStates();
        updateNowPlayingBtn();
        if (typeof window.syncAeTransportFromPlayback === 'function') window.syncAeTransportFromPlayback();
        scheduleNowPlayingWaveform(filePath);
        return;
    }

    // New file
    try {
        const canEngine =
            typeof window !== 'undefined' &&
            window.vstUpdater &&
            typeof window.vstUpdater.audioEngineInvoke === 'function' &&
            typeof window.enginePlaybackStart === 'function';
        if (canEngine) {
            /* Mute / disconnect `<audio>` before AudioEngine audio starts so WebView path cannot overlap. */
            silenceWebViewAudioForEngine();
            await window.enginePlaybackStart(filePath);
            setEnginePlaybackActive(true);
            if (typeof window !== 'undefined') {
                window._enginePlaybackResumePath = filePath;
            }
            stopReverseBufferPlayback();
            _decodedBuf = null;
            _reversedBuf = null;
            _decodedBufPath = null;
            _pausedOffsetInRev = 0;
            audioPlayerPath = filePath;
            audioPlayer.loop = false;
            if (prefs.getItem('audioReverse') === 'on' && typeof window.engineApplyReversePrefPlayback === 'function') {
                await window.engineApplyReversePrefPlayback();
                audioReverseMode = true;
                syncReversePlaybackButtons(true);
            } else {
                audioReverseMode = false;
                syncReversePlaybackButtons(false);
            }
            if (typeof window.syncEnginePlaybackLoop === 'function') {
                window.syncEnginePlaybackLoop(audioLooping);
            }
        } else {
            if (_enginePlaybackActive && typeof window.enginePlaybackStop === 'function') {
                void window.enginePlaybackStop();
                setEnginePlaybackActive(false);
            }
            if (typeof window !== 'undefined') {
                window._enginePlaybackResumePath = '';
            }
            restoreWebViewAudioAfterEngine();
            ensureAudioGraph();
            if (_playbackCtx.state === 'suspended') await _playbackCtx.resume().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            });
            stopReverseBufferPlayback();
            _decodedBuf = null;
            _reversedBuf = null;
            _decodedBufPath = null;
            _pausedOffsetInRev = 0;
            connectMediaToEq();
            audioPlayer.src = fileSrcForDecode(filePath);
            audioPlayer.loop = audioLooping;
            audioPlayerPath = filePath;
        }
        if (_enginePlaybackActive) {
            /* `enginePlaybackStart` already opened the stream. */
        } else if (audioReverseMode) {
            audioPlayer.pause();
            await ensureReversedBufferForPath(filePath);
            startReverseBufferFromOffset(0);
        } else {
            try {
                await audioPlayer.play();
            } catch (playErr) {
                if (_playbackCtx.state === 'suspended') await _playbackCtx.resume().catch(e => {
                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                });
                await audioPlayer.play();
            }
        }

        // Show now-playing bar, restore expanded state from prefs
        const np = document.getElementById('audioNowPlaying');
        np.classList.add('active');
        if (prefs.getItem('playerExpanded') === 'on') {
            np.classList.add('expanded');
            renderRecentlyPlayed();
        }
        const sample = findByPath(allAudioSamples, filePath);
        const displayName = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
        document.getElementById('npName').textContent = displayName;

        // Track recently played
        addToRecentlyPlayed(filePath, sample);

        updatePlayBtnStates();
        updateNowPlayingBtn();
        updateFavBtn();
        updateNoteBtn();
        updateMetaLine();
        // Deferred one task — layout for the waveform flex child is often 0×0 until after paint (WKWebView).
        scheduleNowPlayingWaveform(filePath);
    } catch (err) {
        setEnginePlaybackActive(false);
        if (typeof window.stopEnginePlaybackPoll === 'function') window.stopEnginePlaybackPoll();
        showToast(toastFmt('toast.playback_failed', {
            ext: ext.toUpperCase(),
            err: err.message || err || 'Unknown error'
        }), 4000, 'error');
    }
}

function toggleAudioPlayback() {
    if (!audioPlayerPath) {
        // No track loaded — play most recent from history
        if (typeof recentlyPlayed !== 'undefined' && recentlyPlayed.length > 0 && recentlyPlayed[0]?.path) {
            previewAudio(recentlyPlayed[0].path);
        }
        return;
    }
    if (isEngineUnplayablePath(audioPlayerPath)) {
        return;
    }
    if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
        const playing = isAudioPlaying();
        applyEnginePlaybackPausedFromTransport(playing);
        void window.vstUpdater.audioEngineInvoke({cmd: 'playback_pause', paused: playing});
        updatePlayBtnStates();
        updateNowPlayingBtn();
        if (typeof window.syncAeTransportFromPlayback === 'function') window.syncAeTransportFromPlayback();
        return;
    }
    if (audioReverseMode) {
        if (_bufPlaying) pauseReverseBufferPlayback();
        else startReverseBufferFromOffset(_pausedOffsetInRev);
        updatePlayBtnStates();
        updateNowPlayingBtn();
        return;
    }
    if (audioPlayer.paused) {
        audioPlayer.play();
    } else {
        audioPlayer.pause();
    }
    updatePlayBtnStates();
    updateNowPlayingBtn();
}

function toggleAudioLoop() {
    audioLooping = !audioLooping;
    audioPlayer.loop = audioLooping;
    prefs.setItem('audioLoop', audioLooping ? 'on' : 'off');
    document.getElementById('npBtnLoop').classList.toggle('active', audioLooping);
    updateLoopBtnStates();
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackLoop === 'function') {
        window.syncEnginePlaybackLoop(audioLooping);
    }
}

function toggleRowLoop(filePath, event) {
    event.stopPropagation();
    // If this sample isn't playing yet, start it with loop on
    if (audioPlayerPath !== filePath) {
        audioLooping = true;
        audioPlayer.loop = true;
        prefs.setItem('audioLoop', 'on');
        document.getElementById('npBtnLoop').classList.add('active');
        previewAudio(filePath);
        updateLoopBtnStates();
        return;
    }
    // Toggle loop for the currently playing sample
    toggleAudioLoop();
}

function updateLoopBtnStates() {
    if (!audioPlayerPath) return;
    const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(audioPlayerPath)}"]`);
    if (row) {
        const btn = row.querySelector('.btn-loop');
        if (btn) btn.classList.toggle('active', audioLooping);
    }
}

function stopAudioPlayback() {
    if (_enginePlaybackActive && typeof window !== 'undefined' && typeof window.enginePlaybackStop === 'function') {
        void window.enginePlaybackStop();
        setEnginePlaybackActive(false);
    }
    stopReverseBufferPlayback();
    restoreWebViewAudioAfterEngine();
    _decodedBuf = null;
    _reversedBuf = null;
    _decodedBufPath = null;
    _pausedOffsetInRev = 0;
    audioPlayer.pause();
    audioPlayer.currentTime = 0;
    audioPlayer.src = '';
    audioPlayerPath = null;
    if (typeof window !== 'undefined') {
        window._enginePlaybackResumePath = '';
    }
    clearAudioPlaybackUI();
}

function clearAudioPlaybackUI() {
    const np = document.getElementById('audioNowPlaying');
    np.classList.remove('active');
    np.classList.remove('expanded');
    np.classList.remove('np-playing');
    document.getElementById('npProgress').style.width = '0%';
    document.getElementById('npTime').textContent = catalogFmt('ui.audio.player_time_zero');
    const npCursor = document.getElementById('npCursor');
    if (npCursor) npCursor.style.display = 'none';
    const pill = document.getElementById('audioRestorePill');
    if (pill) pill.classList.remove('active');
    updatePlayBtnStates();
    updateNowPlayingBtn();
    updateFavBtn();
    updateNoteBtn();
}

let _prevPlayingRow = null;

function updatePlayBtnStates() {
    // Clear previous playing row
    if (_prevPlayingRow) {
        const btn = _prevPlayingRow.querySelector('.btn-play');
        if (btn) {
            btn.classList.remove('playing');
            btn.innerHTML = '&#9654;';
        }
        _prevPlayingRow.classList.remove('row-playing');
        const loop = _prevPlayingRow.querySelector('.btn-loop');
        if (loop) loop.classList.remove('active');
    }
    // Set current playing row
    if (audioPlayerPath) {
        const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(audioPlayerPath)}"]`);
        if (row) {
            const btn = row.querySelector('.btn-play');
            const playing = isAudioPlaying();
            if (btn) {
                btn.classList.toggle('playing', playing);
                btn.innerHTML = playing ? '&#9646;&#9646;' : '&#9654;';
            }
            row.classList.toggle('row-playing', playing);
            const loop = row.querySelector('.btn-loop');
            if (loop) loop.classList.toggle('active', playing && audioLooping);
            _prevPlayingRow = row;
        }
    } else {
        _prevPlayingRow = null;
    }
    // Lightweight history update — just toggle active/icon classes, skip full re-render
    const histItems = document.querySelectorAll('#npHistoryList .np-history-item');
    if (histItems.length > 0) {
        histItems.forEach(el => {
            const isActive = el.dataset.path === audioPlayerPath;
            const isPlaying = isActive && isAudioPlaying();
            el.classList.toggle('active', isActive);
            const icon = el.querySelector('.np-h-icon');
            if (icon) icon.innerHTML = isPlaying ? '&#9654;' : '&#9835;';
        });
    }
}

function updateNowPlayingBtn() {
    const btn = document.getElementById('npBtnPlay');
    const np = document.getElementById('audioNowPlaying');
    if (!btn || !np) return;
    if (!isAudioPlaying()) {
        btn.innerHTML = '&#9654;';
        btn.classList.remove('playing');
        np.classList.remove('np-playing');
    } else {
        btn.innerHTML = '&#9646;&#9646;';
        btn.classList.add('playing');
        np.classList.add('np-playing');
    }
}

// Cache DOM elements used every animation frame
let _npTimeEl, _npProgressEl, _npCursorEl;

function _cachePlaybackEls() {
    _npTimeEl = document.getElementById('npTime');
    _npProgressEl = document.getElementById('npProgress');
    _npCursorEl = document.getElementById('npCursor');
}

/** Effective duration (seconds) for engine-routed playback — poll + load may lag or return 0 for some files. */
function enginePlaybackDurationSec() {
    let dur =
        typeof window._enginePlaybackDurSec === 'number' && window._enginePlaybackDurSec > 0
            ? window._enginePlaybackDurSec
            : 0;
    if (dur <= 0 && typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined' && audioPlayerPath) {
        const s = findByPath(allAudioSamples, audioPlayerPath);
        if (s && typeof s.duration === 'number' && s.duration > 0) dur = s.duration;
    }
    return dur;
}

function updatePlaybackTime() {
    let cur;
    let dur;
    if (_enginePlaybackActive && typeof window !== 'undefined' && typeof window._enginePlaybackPosSec === 'number') {
        cur = window._enginePlaybackPosSec;
        dur = enginePlaybackDurationSec();
    } else if (audioReverseMode && _reversedBuf && _bufPlaying) {
        dur = _reversedBuf.duration;
        const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
        const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
        cur = Math.max(0, dur - posInRev);
    } else {
        cur = audioPlayer.currentTime;
        dur = audioPlayer.duration;
    }
    // A-B loop enforcement (forward playback only)
    if (!audioReverseMode && _abLoop && dur > 0 && cur >= _abLoop.end) {
        if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
            void window.vstUpdater.audioEngineInvoke({cmd: 'playback_seek', position_sec: _abLoop.start});
        } else {
            audioPlayer.currentTime = _abLoop.start;
        }
    }
    if (!_npTimeEl) _cachePlaybackEls();
    if (_npTimeEl) _npTimeEl.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
    if (dur > 0) {
        const pct = (cur / dur) * 100;
        if (_npProgressEl) _npProgressEl.style.width = pct + '%';
        if (_npCursorEl) {
            _npCursorEl.style.display = '';
            _npCursorEl.style.left = pct + '%';
        }
        // Playback cursor — metadata panel
        const metaWaveform = document.getElementById('metaWaveformBox');
        if (metaWaveform && metaWaveform.dataset.path === audioPlayerPath) {
            const fill = metaWaveform.querySelector('.waveform-progress-fill');
            const cursor = metaWaveform.querySelector('.waveform-cursor');
            const timeLabel = metaWaveform.querySelector('.waveform-time-label');
            if (fill) fill.style.width = pct + '%';
            if (cursor) cursor.style.left = pct + '%';
            if (timeLabel) timeLabel.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
        }
        // Playback cursor — file browser waveform (cached lookup, not every frame)
        if (!window._fbCursorPath || window._fbCursorPath !== audioPlayerPath) {
            // Path changed — hide old cursor, find new one
            if (window._fbCursorEl) window._fbCursorEl.style.display = 'none';
            const fbRow = document.querySelector(`.file-row[data-wf-file="${CSS.escape(audioPlayerPath)}"]`);
            window._fbCursorEl = fbRow?.querySelector('.file-wf-cursor') || null;
            window._fbCursorPath = audioPlayerPath;
        }
        if (window._fbCursorEl) {
            window._fbCursorEl.style.display = '';
            window._fbCursorEl.style.left = pct + '%';
        }
    }
    if (typeof window.syncAeTransportFromPlayback === 'function') {
        const aeTab = document.getElementById('tabAudioEngine');
        if (aeTab && aeTab.classList.contains('active')) {
            window.syncAeTransportFromPlayback();
        }
    }
}

/** Seek current playback to a normalized position [0, 1]. Used by now-playing and metadata waveforms. */
function seekPlaybackToPercent(pct) {
    const p = Math.max(0, Math.min(1, pct));
    const hasInvoke =
        typeof window !== 'undefined' &&
        window.vstUpdater &&
        typeof window.vstUpdater.audioEngineInvoke === 'function';
    logWaveformSeek('seekPlaybackToPercent', {
        pct: p,
        audioPlayerPath: audioPlayerPath || null,
        enginePlaybackActive: _enginePlaybackActive,
        hasAudioEngineInvoke: !!hasInvoke,
        audioReverseMode,
        hasReversedBuf: !!_reversedBuf,
        durHintSec: typeof enginePlaybackDurationSec === 'function' ? enginePlaybackDurationSec() : null,
        html5duration:
            typeof audioPlayer !== 'undefined' && audioPlayer && Number.isFinite(audioPlayer.duration)
                ? audioPlayer.duration
                : audioPlayer?.duration,
        html5muted: typeof audioPlayer !== 'undefined' && audioPlayer ? audioPlayer.muted : null,
    });
    if (!audioPlayerPath) {
        logWaveformSeek('abort', { reason: 'no_audioPlayerPath' });
        return;
    }
    if (isEngineUnplayablePath(audioPlayerPath)) {
        logWaveformSeek('abort', { reason: 'engine_unplayable_format' });
        return;
    }
    if (_enginePlaybackActive && hasInvoke) {
        let dur = enginePlaybackDurationSec();
        if (dur <= 0) {
            logWaveformSeek('engine_seek_async_duration', { dur, pct: p });
            void (async () => {
                try {
                    const st = await window.vstUpdater.audioEngineInvoke({cmd: 'playback_status'});
                    logWaveformSeek('engine_playback_status', { st });
                    if (st && st.ok === true && typeof st.duration_sec === 'number' && st.duration_sec > 0) {
                        window._enginePlaybackDurSec = st.duration_sec;
                        const pos = p * st.duration_sec;
                        const seekRes = await window.vstUpdater.audioEngineInvoke({
                            cmd: 'playback_seek',
                            position_sec: pos,
                        });
                        logWaveformSeek('engine_playback_seek_result', { position_sec: pos, seekRes });
                    } else {
                        logWaveformSeek('engine_seek_skip', {
                            reason: 'playback_status_missing_duration',
                            st,
                        });
                    }
                } catch (err) {
                    logWaveformSeek('engine_seek_async_error', { err: err && err.message ? err.message : String(err) });
                }
            })();
            return;
        }
        const pos = p * dur;
        logWaveformSeek('engine_seek', { position_sec: pos, dur });
        void (async () => {
            try {
                const seekRes = await window.vstUpdater.audioEngineInvoke({
                    cmd: 'playback_seek',
                    position_sec: pos,
                });
                logWaveformSeek('engine_playback_seek_result', { position_sec: pos, seekRes });
            } catch (err) {
                logWaveformSeek('engine_seek_error', { err: err && err.message ? err.message : String(err) });
            }
        })();
        return;
    }
    if (audioReverseMode && _reversedBuf) {
        const d = _reversedBuf.duration;
        const origT = p * d;
        logWaveformSeek('reverse_buffer_seek', { d, origT });
        stopReverseBufferPlayback();
        startReverseBufferFromOffset(Math.max(0, d - origT));
        return;
    }
    if (!audioPlayer.duration) {
        logWaveformSeek('abort', {
            reason: 'html5_no_duration',
            enginePlaybackActive: _enginePlaybackActive,
            hasInvoke: !!hasInvoke,
            hint: 'If output is from audio-engine but _enginePlaybackActive is false, seek IPC will not run.',
        });
        return;
    }
    const t = p * audioPlayer.duration;
    logWaveformSeek('html5_seek', { currentTime: t, duration: audioPlayer.duration });
    audioPlayer.currentTime = t;
}

/**
 * Nudge playback along the forward timeline (seconds). Engine: `playback_status` + `playback_seek`.
 * Web Audio reverse buffer and &lt;audio&gt;: reuse `seekPlaybackToPercent`.
 */
async function skipPlaybackSeconds(delta) {
    const d = Number(delta);
    if (!Number.isFinite(d)) return;
    if (!audioPlayerPath) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.reverse_no_track'), 3000, 'error');
        return;
    }
    if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
        try {
            const st = await window.vstUpdater.audioEngineInvoke({cmd: 'playback_status'});
            if (!st || st.ok !== true || st.loaded !== true) return;
            const dur = typeof st.duration_sec === 'number' ? st.duration_sec : 0;
            if (dur <= 0) return;
            const cur = typeof st.position_sec === 'number' ? st.position_sec : 0;
            const next = Math.max(0, Math.min(dur, cur + d));
            await window.vstUpdater.audioEngineInvoke({cmd: 'playback_seek', position_sec: next});
        } catch {
            /* ignore */
        }
        return;
    }
    let cur = 0;
    let dur = 0;
    if (audioReverseMode && _reversedBuf) {
        dur = _reversedBuf.duration;
        if (!(dur > 0)) return;
        cur = _bufPlaying ? getOriginalTimeFromReverseBuffer() : Math.max(0, dur - _pausedOffsetInRev);
    } else {
        dur = audioPlayer.duration;
        if (!dur || Number.isNaN(dur)) return;
        cur = audioPlayer.currentTime;
    }
    const next = Math.max(0, Math.min(dur, cur + d));
    seekPlaybackToPercent(next / dur);
}

function seekAudio(event) {
    logWaveformSeek('seekAudio', { hasPath: !!audioPlayerPath });
    if (!audioPlayerPath) {
        logWaveformSeek('seekAudio_abort', { reason: 'no_audioPlayerPath' });
        return;
    }
    const bar = document.getElementById('npWaveform');
    if (!bar) {
        logWaveformSeek('seekAudio_abort', { reason: 'missing_npWaveform' });
        return;
    }
    const rect = bar.getBoundingClientRect();
    if (rect.width <= 0) {
        logWaveformSeek('seekAudio_abort', { reason: 'zero_width_rect', rect: { w: rect.width, h: rect.height } });
        return;
    }
    const pct = (event.clientX - rect.left) / rect.width;
    logWaveformSeek('seekAudio', { clientX: event.clientX, pct, rectLeft: rect.left, rectWidth: rect.width });
    seekPlaybackToPercent(pct);
}

function setAudioVolume(value) {
    const vol = parseInt(value, 10) / 100;
    const npSlider = document.getElementById('npVolume');
    if (npSlider) npSlider.value = String(value);
    const npPct = document.getElementById('npVolumePct');
    if (npPct) npPct.textContent = value + '%';
    const eqVs = document.getElementById('npEqVolSlider');
    if (eqVs) eqVs.value = String(value);
    const eqVp = document.getElementById('npEqVolVal');
    if (eqVp) eqVp.textContent = value + '%';
    const aeV = document.getElementById('aeVolume');
    if (aeV) aeV.value = String(value);
    const aePct = document.getElementById('aeVolumePct');
    if (aePct) aePct.textContent = value + '%';
    prefs.setItem('audioVolume', value);
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        audioPlayer.volume = 0;
        audioPlayer.muted = true;
        if (_gainNode) {
            _gainNode.gain.value = 0;
        }
        window.syncEnginePlaybackDspFromPrefs();
        return;
    }
    audioPlayer.volume = Math.max(0, Math.min(1, vol));
    if (_gainNode) {
        _gainNode.gain.value = vol * parseFloat(document.getElementById('npGainSlider')?.value || '1');
    }
}

function setPlaybackSpeed(value) {
    const v = parseFloat(value);
    const clamped = Number.isFinite(v) ? Math.max(0.25, Math.min(2, v)) : 1;
    prefs.setItem('audioSpeed', String(clamped));
    const npS = document.getElementById('npSpeed');
    if (npS) {
        let bestIdx = 0;
        let bestDiff = Infinity;
        for (let i = 0; i < npS.options.length; i++) {
            const ov = parseFloat(npS.options[i].value);
            const d = Math.abs(ov - clamped);
            if (d < bestDiff) {
                bestDiff = d;
                bestIdx = i;
            }
        }
        npS.selectedIndex = bestIdx;
    }
    const eqSl = document.getElementById('npEqSpeedSlider');
    if (eqSl) eqSl.value = String(clamped);
    const eqVal = document.getElementById('npEqSpeedVal');
    if (eqVal) eqVal.textContent = clamped + '×';
    const aeSp = document.getElementById('aePlaybackSpeed');
    if (aeSp) aeSp.value = String(clamped);
    if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
        void window.vstUpdater.audioEngineInvoke({cmd: 'playback_set_speed', speed: clamped});
        return;
    }
    if (audioReverseMode && _bufSrc && _bufPlaying) {
        const bufClamped = Math.max(0.0625, Math.min(16, clamped));
        _bufSrc.playbackRate.value = bufClamped;
        _bufPlaybackRate = bufClamped;
    } else {
        audioPlayer.playbackRate = clamped;
    }
}

// ── Metadata Panel ──
/** Expand the metadata panel for a given file path (no toggle, always opens). */
async function expandMetaForPath(filePath) {
    const tbody = document.getElementById('audioTableBody');
    if (!tbody) return;

    // Close any existing meta row
    const existingMeta = document.getElementById('audioMetaRow');
    if (existingMeta) {
        existingMeta.remove();
        const prevRow = tbody.querySelector('tr.row-expanded');
        if (prevRow) prevRow.classList.remove('row-expanded');
    }

    expandedMetaPath = filePath;

    const row = tbody.querySelector(`tr[data-audio-path="${CSS.escape(filePath)}"]`);
    if (!row) return;
    row.classList.add('row-expanded');

    // Insert loading row
    const metaRow = document.createElement('tr');
    metaRow.id = 'audioMetaRow';
    metaRow.className = 'audio-meta-row';
    metaRow.setAttribute('data-meta-path', filePath);
    metaRow.innerHTML = `<td colspan="12"><div class="audio-meta-panel" style="justify-items: center;"><div class="spinner" style="width: 18px; height: 18px;"></div></div></td>`;
    row.after(metaRow);

    // Scroll the expanded row into view
    row.scrollIntoView({behavior: 'smooth', block: 'nearest'});

    // Fetch metadata
    try {
        const meta = await window.vstUpdater.getAudioMetadata(filePath);
        if (expandedMetaPath !== filePath) return; // user closed it

        let items = '';
        items += metaItem('File Name', meta.fileName, true);
        items += metaItem('Format', meta.format);
        items += metaItem('Size', formatAudioSize(meta.sizeBytes));
        items += metaItem('Full Path', meta.fullPath, true);

        if (meta.sampleRate) items += metaItem('Sample Rate', meta.sampleRate.toLocaleString() + ' Hz');
        if (meta.bitsPerSample) items += metaItem('Bit Depth', meta.bitsPerSample + '-bit');
        if (meta.channels) items += metaItem('Channels', meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch');
        if (meta.duration) items += metaItem('Duration', formatTime(meta.duration));
        if (meta.byteRate) items += metaItem('Byte Rate', formatAudioSize(meta.byteRate) + '/s');

        // BPM and Key placeholders — filled async
        items += `<div class="meta-item" id="metaBpmItem" title="Estimated tempo via onset-strength autocorrelation"><span class="meta-label">BPM</span><span class="meta-value" id="metaBpmValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;
        items += `<div class="meta-item" id="metaKeyItem" title="Musical key detected via chromagram analysis"><span class="meta-label">KEY</span><span class="meta-value" id="metaKeyValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;
        items += `<div class="meta-item" id="metaLufsItem" title="Integrated loudness (ITU-R BS.1770 K-weighted)"><span class="meta-label">LUFS</span><span class="meta-value" id="metaLufsValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;

        const fmtDate = (v) => {
            if (!v) return '—';
            const d = new Date(v);
            return isNaN(d) ? '—' : d.toLocaleString();
        };
        items += metaItem('Created', fmtDate(meta.created));
        items += metaItem('Modified', fmtDate(meta.modified));
        items += metaItem('Accessed', fmtDate(meta.accessed));
        items += metaItem('Permissions', meta.permissions);

        // Waveform preview with seek support
        const waveformHtml = `<div class="meta-waveform" id="metaWaveformBox" data-path="${escapeHtml(filePath)}" title="Click to seek playback position">
      <canvas id="metaWaveformCanvas" title="Waveform — click to seek"></canvas>
      <div class="waveform-progress-fill"></div>
      <div class="waveform-cursor" style="left:0;"></div>
      <div class="waveform-time-label">${meta.duration ? formatTime(meta.duration) : ''}</div>
    </div>
    <div class="meta-waveform" style="height:80px;cursor:default;" title="Spectrogram — frequency content over time (FFT)">
      <canvas id="metaSpectrogramCanvas" width="800" height="80" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="Spectrogram — low frequencies at bottom, high at top"></canvas>
      <span style="position:absolute;top:2px;left:4px;font-size:8px;color:var(--text-dim);pointer-events:none;">SPECTROGRAM</span>
    </div>`;

        const _closeT = typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.meta_close_title')) : _audioFmt('ui.audio.meta_close_title');
        metaRow.innerHTML = `<td colspan="12"><div class="audio-meta-panel"><span class="meta-close-btn" data-action="closeMetaRow" title="${_closeT}">&#10005;</span>${waveformHtml}${items}</div></td>`;

        // Expanded-row visuals are lowest priority: idle-scheduled so they never preempt playback.
        // Run sequentially so we decode once per visual (not two parallel full-file decodes).
        // When this row is the current track, defer one rAF so the first paint after `play()` / engine
        // start lands before waveform/spectrogram IPC or worker decode.
        cancelIdleSchedule(_metaPanelIdleId);
        _metaPanelIdleId = null;
        _metaPanelDrawSeq++;
        const metaSeq = _metaPanelDrawSeq;
        const scheduleMetaDraw = () => {
            _metaPanelIdleId = scheduleIdleVisualWork(() => {
                _metaPanelIdleId = null;
                void drawMetaPanelVisuals(filePath, metaSeq);
            }, { delayMs: 0 });
        };
        if (filePath === audioPlayerPath) {
            requestAnimationFrame(scheduleMetaDraw);
        } else {
            scheduleMetaDraw();
        }

        // Sync cursor if already playing this track
        if (audioPlayerPath === filePath && audioPlayer.duration > 0) {
            const pct = (audioPlayer.currentTime / audioPlayer.duration) * 100;
            const box = document.getElementById('metaWaveformBox');
            if (box) {
                const fill = box.querySelector('.waveform-progress-fill');
                const cursor = box.querySelector('.waveform-cursor');
                const timeLabel = box.querySelector('.waveform-time-label');
                if (fill) fill.style.width = pct + '%';
                if (cursor) cursor.style.left = pct + '%';
                if (timeLabel) timeLabel.textContent = `${formatTime(audioPlayer.currentTime)} / ${formatTime(audioPlayer.duration)}`;
            }
        }

        // Estimate BPM and detect key async (all playable formats)
        const bpmFormats = ['WAV', 'AIFF', 'AIF', 'MP3', 'FLAC', 'OGG', 'M4A', 'AAC', 'OPUS'];
        if (bpmFormats.includes(meta.format)) {
            await Promise.all([
                estimateBpmForMeta(filePath),
                detectKeyForMeta(filePath),
                measureLufsForMeta(filePath),
            ]);
            await persistAnalysisRowToDb(filePath);
        } else {
            const bpmEl = document.getElementById('metaBpmValue');
            if (bpmEl) bpmEl.textContent = '—';
            const keyEl = document.getElementById('metaKeyValue');
            if (keyEl) keyEl.textContent = '—';
            const lufsEl = document.getElementById('metaLufsValue');
            if (lufsEl) lufsEl.textContent = '—';
        }
    } catch (err) {
        {
            const msg = typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.meta_load_failed')) : _audioFmt('ui.audio.meta_load_failed');
            metaRow.innerHTML = `<td colspan="12"><div class="audio-meta-panel"><span style="color: var(--red);">${msg}</span></div></td>`;
        }
    }
}

/** Called from keyboard-nav when Play on Keyboard Selection moves the highlight; keeps the metadata panel under the selected row. */
function syncExpandedMetaWithKeyboardSelection(newPath) {
    if (expandedMetaPath === null) return;
    if (expandedMetaPath === newPath) return;
    void expandMetaForPath(newPath);
}

async function toggleMetadata(filePath, event) {
    // Don't toggle if clicking buttons
    if (event.target.closest('.col-actions')) return;

    // Single-click: play unless explicitly off (null/undefined before prefs.load() → play; matches default-on)
    {
        const sc = prefs.getItem('singleClickPlay');
        if (sc !== 'off' && sc !== 'false') {
            // Await so expanded-row waveform/spectrogram IPC/decode runs after playback has started
            // (engine `playback_load` / `<audio>.play()`), not in parallel with it.
            await previewAudio(filePath);
        }
    }

    if (prefs.getItem('expandOnClick') === 'off') return;

    // If the same row is already expanded, toggle it off
    if (expandedMetaPath === filePath) {
        closeMetaRow();
        return;
    }

    await expandMetaForPath(filePath);
}

// BPM cache — persisted to prefs
let _bpmCache = {};
let _bpmCacheDirty = false;

async function loadBpmKeyCache() {
    try {
        _bpmCache = await window.vstUpdater.readCacheFile('bpm-cache.json');
    } catch {
        _bpmCache = {};
    }
    try {
        _keyCache = await window.vstUpdater.readCacheFile('key-cache.json');
    } catch {
        _keyCache = {};
    }
    try {
        _lufsCache = await window.vstUpdater.readCacheFile('lufs-cache.json');
    } catch {
        _lufsCache = {};
    }
}

let _keyCacheDirty = false;
let _lufsCacheDirty = false;

function _saveBpmCache() {
    if (!_bpmCacheDirty) return;
    _bpmCacheDirty = false;
    // Use requestIdleCallback to avoid blocking animations
    const doSave = () => window.vstUpdater.writeCacheFile('bpm-cache.json', _bpmCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
    if (typeof requestIdleCallback === 'function') requestIdleCallback(doSave); else setTimeout(doSave, 0);
}

function _saveKeyCache() {
    if (!_keyCacheDirty) return;
    _keyCacheDirty = false;
    const doSave = () => window.vstUpdater.writeCacheFile('key-cache.json', _keyCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
    if (typeof requestIdleCallback === 'function') requestIdleCallback(doSave); else setTimeout(doSave, 0);
}

function _saveLufsCache() {
    if (!_lufsCacheDirty) return;
    _lufsCacheDirty = false;
    const doSave = () => window.vstUpdater.writeCacheFile('lufs-cache.json', _lufsCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
    if (typeof requestIdleCallback === 'function') requestIdleCallback(doSave); else setTimeout(doSave, 0);
}

// Debounce cache saves — 30s during analysis to minimize main thread blocking
let _bpmSaveTimer = null;
let _keySaveTimer = null;
const _CACHE_SAVE_DELAY = 30000;

function _debounceBpmSave() {
    _bpmCacheDirty = true;
    clearTimeout(_bpmSaveTimer);
    _bpmSaveTimer = setTimeout(_saveBpmCache, _CACHE_SAVE_DELAY);
}

function _debounceKeySave() {
    _keyCacheDirty = true;
    clearTimeout(_keySaveTimer);
    _keySaveTimer = setTimeout(_saveKeyCache, _CACHE_SAVE_DELAY);
}

async function estimateBpmForMeta(filePath) {
    const bpmEl = document.getElementById('metaBpmValue');
    if (!bpmEl) return;

    // Check in-memory cache
    if (_bpmCache[filePath] !== undefined) {
        bpmEl.textContent = _bpmCache[filePath] ? _bpmCache[filePath] + ' BPM' : '—';
        return;
    }

    // Check SQLite (analysis data stored on audio_samples row)
    try {
        const analysis = await window.vstUpdater.dbGetAnalysis(filePath);
        if (analysis && analysis.bpm) {
            _bpmCache[filePath] = analysis.bpm;
            bpmEl.textContent = analysis.bpm + ' BPM';
            // Also fill key and LUFS from same query
            if (analysis.key) {
                _keyCache[filePath] = analysis.key;
                const keyEl = document.getElementById('metaKeyValue');
                if (keyEl) keyEl.textContent = analysis.key;
            }
            if (analysis.lufs != null) {
                _lufsCache[filePath] = analysis.lufs;
                const lufsEl = document.getElementById('metaLufsValue');
                if (lufsEl) lufsEl.textContent = analysis.lufs + ' LUFS';
            }
            return;
        }
    } catch (e) {
        if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
    }

    // Not in DB either — compute it
    try {
        const bpm = await window.vstUpdater.estimateBpm(filePath);
        _bpmCache[filePath] = bpm;
        _debounceBpmSave();
        const currentBpmEl = document.getElementById('metaBpmValue');
        const metaRow = document.getElementById('audioMetaRow');
        if (currentBpmEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
            currentBpmEl.textContent = bpm ? bpm + ' BPM' : '—';
        }
        // Update table row cell
        const tableRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow) {
            const cell = tableRow.querySelector('.col-bpm');
            if (cell) cell.textContent = bpm || '';
        }
    } catch {
        _bpmCache[filePath] = null;
        if (bpmEl) bpmEl.textContent = '—';
    }
}

// Key detection cache — persisted to prefs
let _keyCache = {};

// LUFS cache — persisted to prefs
let _lufsCache = {};

function _debounceLufsSave() {
    _lufsCacheDirty = true;
    clearTimeout(_lufsSaveTimer);
    _lufsSaveTimer = setTimeout(_saveLufsCache, _CACHE_SAVE_DELAY);
}

let _lufsSaveTimer = null;

async function detectKeyForMeta(filePath) {
    const keyEl = document.getElementById('metaKeyValue');
    if (!keyEl) return;

    if (_keyCache[filePath] !== undefined) {
        keyEl.textContent = _keyCache[filePath] || '—';
        return;
    }

    try {
        const key = await window.vstUpdater.detectAudioKey(filePath);
        _keyCache[filePath] = key;
        _debounceKeySave();
        const currentKeyEl = document.getElementById('metaKeyValue');
        const metaRow = document.getElementById('audioMetaRow');
        if (currentKeyEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
            currentKeyEl.textContent = key || '—';
        }
        // Update table row cell
        const tableRow2 = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow2) {
            const cell = tableRow2.querySelector('.col-key');
            if (cell) cell.textContent = key || '';
        }
    } catch {
        _keyCache[filePath] = null;
        if (keyEl) keyEl.textContent = '—';
    }
}

async function measureLufsForMeta(filePath) {
    const lufsEl = document.getElementById('metaLufsValue');
    if (!lufsEl) return;

    if (_lufsCache[filePath] !== undefined) {
        lufsEl.textContent = _lufsCache[filePath] != null ? _lufsCache[filePath] + ' LUFS' : '—';
        return;
    }

    try {
        const lufs = await window.vstUpdater.measureLufs(filePath);
        _lufsCache[filePath] = lufs;
        _debounceLufsSave();
        const currentEl = document.getElementById('metaLufsValue');
        const metaRow = document.getElementById('audioMetaRow');
        if (currentEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
            currentEl.textContent = lufs != null ? lufs + ' LUFS' : '—';
        }
        const tableRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow) {
            const cell = tableRow.querySelector('.col-lufs');
            if (cell) cell.textContent = lufs != null ? lufs : '';
        }
    } catch {
        _lufsCache[filePath] = null;
        if (lufsEl) lufsEl.textContent = '—';
    }
}

/** Merge in-memory analysis caches with existing DB row and persist (same rules as `batch_analyze`). */
async function persistAnalysisRowToDb(filePath) {
    if (expandedMetaPath !== filePath) return;
    if (!window.vstUpdater || typeof window.vstUpdater.dbUpdateAnalysis !== 'function') return;
    let base = {};
    try {
        base = await window.vstUpdater.dbGetAnalysis(filePath);
    } catch {
        /* ignore */
    }
    const bpm = _bpmCache[filePath];
    const key = _keyCache[filePath];
    const lufs = _lufsCache[filePath];
    const mergedBpm = bpm !== undefined ? bpm : (typeof base.bpm === 'number' ? base.bpm : null);
    const mergedKey = key !== undefined ? (key || null) : (typeof base.key === 'string' && base.key.length ? base.key : null);
    const mergedLufs = lufs !== undefined ? lufs : (typeof base.lufs === 'number' ? base.lufs : null);
    try {
        await window.vstUpdater.dbUpdateAnalysis(filePath, mergedBpm, mergedKey, mergedLufs);
    } catch (e) {
        if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
        return;
    }
    const exhausted = mergedBpm == null && mergedKey != null && mergedLufs != null;
    const sample = typeof findByPath === 'function' ? findByPath(allAudioSamples, filePath) : null;
    if (sample) {
        sample.bpmExhausted = exhausted;
        if (exhausted) {
            delete sample.bpm;
        } else if (mergedBpm != null) {
            sample.bpm = mergedBpm;
        }
    }
    if (exhausted && typeof _bpmCache !== 'undefined') {
        delete _bpmCache[filePath];
    }
    const forCell = sample || {
        path: filePath,
        bpmExhausted: exhausted,
        bpm: mergedBpm != null ? mergedBpm : undefined,
    };
    const { display, titleRaw } = bpmCellDisplayAndTitle(forCell);
    const tableRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
    if (tableRow) {
        const cell = tableRow.querySelector('.col-bpm');
        if (cell) {
            cell.textContent = display;
            cell.setAttribute('title', titleRaw);
        }
    }
}

// ── Background BPM/Key/LUFS batch analysis ──
let _bgAnalysisRunning = false;
let _bgAnalysisAbort = false;
let _bgQueue = []; // kept for compat but no longer primary source
let _bgDone = 0;
let _bgPaused = false;
// Pause bg analysis when user interacts (resume after 3s idle)
let _bgIdleTimer = null;
document.addEventListener('mousedown', () => {
    _bgPaused = true;
    clearTimeout(_bgIdleTimer);
    _bgIdleTimer = setTimeout(() => {
        _bgPaused = false;
    }, 3000);
}, true);
document.addEventListener('keydown', () => {
    _bgPaused = true;
    clearTimeout(_bgIdleTimer);
    _bgIdleTimer = setTimeout(() => {
        _bgPaused = false;
    }, 3000);
}, true);

function _setBgAnalysisBadgeRunning(badge) {
    if (!badge) return;
    badge.textContent = catalogFmt('ui.stats.bpm_bg_working');
}

function _setBgAnalysisBadgeProgress(badge, n) {
    if (!badge) return;
    badge.textContent = catalogFmt('ui.stats.bpm_bg_progress', {n});
}

/** Keep Settings cache stats table in sync while BPM/Key/LUFS batches run (not only on tab open). */
function _refreshCacheStatsIfSettingsTab() {
    try {
        if (typeof prefs === 'undefined' || prefs.getItem('activeTab') !== 'settings') return;
        if (typeof renderCacheStats !== 'function') return;
        void renderCacheStats();
    } catch {
        /* ignore */
    }
}

async function startBackgroundAnalysis() {
    if (_bgAnalysisRunning) return;
    _bgAnalysisRunning = true;
    _bgAnalysisAbort = false;

    const badge = document.getElementById('bgAnalysisBadge');
    const BATCH = 50; // 50 files analyzed in parallel per rayon
    if (badge) {
        if (_bgDone > 0) _setBgAnalysisBadgeProgress(badge, _bgDone);
        else _setBgAnalysisBadgeRunning(badge);
    }

    while (!_bgAnalysisAbort) {
        while (_bgPaused && !_bgAnalysisAbort) await new Promise(r => setTimeout(r, 200));
        if (_bgAnalysisAbort) break;

        let paths;
        try {
            paths = await window.vstUpdater.dbUnanalyzedPaths(BATCH);
        } catch {
            break;
        }
        if (!paths || paths.length === 0) break;

        // Single IPC call → Rust processes all in parallel (rayon) → saves to SQLite
        // Returns results directly so we skip N individual dbGetAnalysis roundtrips.
        let analysisResult;
        try {
            _setBgAnalysisBadgeRunning(badge);
            analysisResult = await window.vstUpdater.batchAnalyze(paths);
            _bgDone += analysisResult.count || 0;
        } catch (e) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.analysis_batch_failed', {err: e.message || e}), 4000, 'error');
            break; // Stop loop on persistent failure
        }

        // Update visible rows from returned results (no extra IPC needed)
        const tbody = document.getElementById('audioTableBody');
        if (tbody && analysisResult.results) {
            for (const a of analysisResult.results) {
                if (a.bpm) {
                    _bpmCache[a.path] = a.bpm;
                    _debounceBpmSave();
                }
                if (a.bpmExhausted === true && typeof _bpmCache !== 'undefined') {
                    delete _bpmCache[a.path];
                }
                if (a.key) {
                    _keyCache[a.path] = a.key;
                    _debounceKeySave();
                }
                if (a.lufs != null) {
                    _lufsCache[a.path] = a.lufs;
                    _debounceLufsSave();
                }
                const sample = typeof findByPath === 'function' ? findByPath(allAudioSamples, a.path) : null;
                if (sample) {
                    if (a.bpm) sample.bpm = a.bpm;
                    if (a.key) sample.key = a.key;
                    if (a.lufs != null) sample.lufs = a.lufs;
                    sample.bpmExhausted = a.bpmExhausted === true;
                    if (sample.bpmExhausted) {
                        delete sample.bpm;
                    }
                }
                const row = tbody.querySelector(`tr[data-audio-path="${CSS.escape(a.path)}"]`);
                if (row) {
                    const cBpm = row.querySelector('.col-bpm');
                    if (cBpm) {
                        const src = sample || { path: a.path, bpm: a.bpm, bpmExhausted: a.bpmExhausted === true };
                        const { display, titleRaw } = bpmCellDisplayAndTitle(sample || src);
                        cBpm.textContent = display;
                        cBpm.setAttribute('title', titleRaw);
                    }
                    if (a.key) {
                        const c = row.querySelector('.col-key');
                        if (c) c.textContent = a.key;
                    }
                    if (a.lufs != null) {
                        const c = row.querySelector('.col-lufs');
                        if (c) {
                            c.textContent = a.lufs;
                            c.classList.toggle('lufs-low', a.lufs < -25);
                        }
                    }
                }
                if (a.path === audioPlayerPath) updateMetaLine();
            }
        }

        _setBgAnalysisBadgeProgress(badge, _bgDone);
        _refreshCacheStatsIfSettingsTab();
        await new Promise(r => setTimeout(r, 100));
    }

    _refreshCacheStatsIfSettingsTab();
    _bgAnalysisRunning = false;
    if (badge) badge.innerHTML = '';
}

function stopBackgroundAnalysis() {
    _bgAnalysisAbort = true;
}

/** Settings / manual trigger: start background BPM/Key/LUFS batch if not already running. */
function triggerBackgroundBpmKeyLufsAnalysis() {
    if (_bgAnalysisRunning) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.bpm_key_lufs_analysis_already_running'));
        return;
    }
    startBackgroundAnalysis();
    if (typeof showToast === 'function') showToast(toastFmt('toast.bpm_key_lufs_analysis_started'));
}

/** Settings: request stop of background BPM/Key/LUFS batch (takes effect after current batch). */
function triggerStopBackgroundBpmKeyLufsAnalysis() {
    if (!_bgAnalysisRunning) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.bpm_key_lufs_analysis_not_running'));
        return;
    }
    stopBackgroundAnalysis();
    if (typeof showToast === 'function') showToast(toastFmt('toast.bpm_key_lufs_analysis_stopped'));
}

function metaItem(label, value, wide) {
    const cls = wide ? 'meta-item meta-item-wide' : 'meta-item';
    const val = String(value || '—');
    return `<div class="${cls}" title="${escapeHtml(label)}: ${escapeHtml(val)}"><span class="meta-label">${label}</span><span class="meta-value">${escapeHtml(val)}</span></div>`;
}

function openAudioFolder(filePath) {
    window.vstUpdater.openAudioFolder(filePath).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}

/** Same items/order as `#npHistoryList` (Recently Played, drag order) or search results when the player search box has text. */
function getPlayerHistoryListItems() {
    const searchInput = document.getElementById('npSearchInput');
    const query = searchInput ? searchInput.value.trim().toLowerCase() : '';
    if (!query) {
        return typeof recentlyPlayed !== 'undefined' && Array.isArray(recentlyPlayed) ? recentlyPlayed : [];
    }
    const seen = new Set();
    const scored = [];
    for (const r of recentlyPlayed) {
        const score = searchScore(query, [r.name, r.path], 'fuzzy');
        if (score > 0 && !seen.has(r.path)) {
            seen.add(r.path);
            scored.push({item: r, score: score + 1000});
        }
    }
    if (typeof allAudioSamples !== 'undefined') {
        const N = Math.min(allAudioSamples.length, 10000);
        for (let i = 0; i < N; i++) {
            const s = allAudioSamples[i];
            const score = searchScore(query, [s.name, s.path], 'fuzzy');
            if (score > 0 && !seen.has(s.path)) {
                seen.add(s.path);
                scored.push({item: {path: s.path, name: s.name, format: s.format, size: s.sizeFormatted}, score});
            }
        }
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, 100).map(s => s.item);
}

// ── Recently Played / Expanded Player ──
function addToRecentlyPlayed(filePath, sample) {
    // Remove duplicate if already in list
    recentlyPlayed = recentlyPlayed.filter(r => r.path !== filePath);
    // Add to front
    recentlyPlayed.unshift({
        path: filePath,
        name: sample ? sample.name : filePath.split('/').pop().replace(/\.[^.]+$/, ''),
        format: sample ? sample.format : filePath.split('.').pop().toUpperCase(),
        size: sample ? sample.sizeFormatted : '',
    });
    // Cap
    if (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.length = MAX_RECENT;
    saveRecentlyPlayed();
    renderRecentlyPlayed();
}

function renderRecentlyPlayed() {
    const list = document.getElementById('npHistoryList');
    if (!list) return;
    const searchInput = document.getElementById('npSearchInput');
    const query = searchInput ? searchInput.value.trim().toLowerCase() : '';
    const items = getPlayerHistoryListItems();

    if (items.length === 0 && query) {
        list.innerHTML = `<div style="text-align:center;color:var(--text-dim);font-size:11px;padding:12px;">${typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.search_no_matches')) : _audioFmt('ui.audio.search_no_matches')}</div>`;
        return;
    }

    list.innerHTML = items.map(r => {
        const isActive = r.path === audioPlayerPath;
        const isPlaying = isActive && isAudioPlaying();
        return `<div class="np-history-item${isActive ? ' active' : ''}" data-action="playRecent" data-path="${escapeHtml(r.path)}">
      <span class="np-h-icon">${isPlaying ? '&#9654;' : '&#9835;'}</span>
      <span class="np-h-name" title="${escapeHtml(r.path)}">${query ? highlightMatch(r.name, query, 'fuzzy') : escapeHtml(r.name)}</span>
      <span class="np-h-format">${r.format}</span>
      ${r.size ? `<span class="np-h-dur">${r.size}</span>` : ''}
    </div>`;
    }).join('');
    if (typeof initRecentlyPlayedDragReorder === 'function') requestAnimationFrame(initRecentlyPlayedDragReorder);
}

// Search input in player — uses unified filter system
registerFilter('filterNowPlaying', {
    inputId: 'npSearchInput',
    fetchFn() {
        const np = document.getElementById('audioNowPlaying');
        if (np && np.classList.contains('expanded')) {
            renderRecentlyPlayed();
        } else {
            renderMiniSearchResults();
        }
    },
});

function renderMiniSearchResults() {
    const container = document.getElementById('npSearchResults');
    if (!container) return;
    const searchInput = document.getElementById('npSearchInput');
    const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

    if (!query) {
        container.innerHTML = '';
        return;
    }

    const seen = new Set();
    const scored = [];
    for (const r of recentlyPlayed) {
        const score = searchScore(query, [r.name, r.path], 'fuzzy');
        if (score > 0 && !seen.has(r.path)) {
            seen.add(r.path);
            scored.push({item: r, score: score + 1000});
        }
    }
    if (typeof allAudioSamples !== 'undefined') {
        // Cap iteration for keystroke-speed search (see note in renderRecentlyPlayed).
        const N = Math.min(allAudioSamples.length, 10000);
        for (let i = 0; i < N; i++) {
            const s = allAudioSamples[i];
            const score = searchScore(query, [s.name, s.path], 'fuzzy');
            if (score > 0 && !seen.has(s.path)) {
                seen.add(s.path);
                scored.push({item: {path: s.path, name: s.name, format: s.format, size: s.sizeFormatted}, score});
            }
        }
    }
    scored.sort((a, b) => b.score - a.score);
    const items = scored.slice(0, 50).map(s => s.item);

    if (items.length === 0) {
        container.innerHTML = `<div style="text-align:center;color:var(--text-dim);font-size:11px;padding:8px;">${typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.search_no_matches')) : _audioFmt('ui.audio.search_no_matches')}</div>`;
        return;
    }

    container.innerHTML = items.map(r => {
        const isActive = r.path === audioPlayerPath;
        return `<div class="np-history-item${isActive ? ' active' : ''}" data-action="playRecent" data-path="${escapeHtml(r.path)}">
      <span class="np-h-icon">&#9835;</span>
      <span class="np-h-name" title="${escapeHtml(r.path)}">${highlightMatch(r.name, query, 'fuzzy')}</span>
      <span class="np-h-format">${r.format}</span>
    </div>`;
    }).join('');
}

function togglePlayerExpanded() {
    const np = document.getElementById('audioNowPlaying');
    np.classList.toggle('expanded');
    const ex = np.classList.contains('expanded');
    prefs.setItem('playerExpanded', ex ? 'on' : 'off');
    if (ex) {
        renderRecentlyPlayed();
        /* Expanded layout changes canvas wrap size; re-sync EQ + mini FFT after paint (tauri:// vs dev). */
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                if (typeof window.applyNpEqCanvasHeightFromPrefs === 'function') window.applyNpEqCanvasHeightFromPrefs();
                if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
                if (typeof window.ensureEnginePlaybackFftRaf === 'function') window.ensureEnginePlaybackFftRaf();
            });
        });
    }
}

function favCurrentTrack() {
    if (!audioPlayerPath) return;
    if (isFavorite(audioPlayerPath)) {
        removeFavorite(audioPlayerPath);
    } else {
        const sample = findByPath(allAudioSamples, audioPlayerPath);
        const name = sample ? sample.name : audioPlayerPath.split('/').pop().replace(/\.[^.]+$/, '');
        addFavorite('sample', audioPlayerPath, name, {format: sample ? sample.format : ''});
    }
}

// Update favorite button state when track changes (also `window.updateFavBtn` from `saveFavorites` in favorites.js)
function updateFavBtn() {
    const btn = document.getElementById('npBtnFav');
    if (!btn) return;
    const fav = !!(audioPlayerPath && isFavorite(audioPlayerPath));
    btn.classList.toggle('np-fav-active', fav);
    btn.style.color = fav ? 'var(--yellow)' : '';
}

/** Floating player note/tags button — also `window.updateNoteBtn` from `setNote` in notes.js */
function updateNoteBtn() {
    const btn = document.getElementById('npBtnTag');
    if (!btn) return;
    if (!audioPlayerPath || typeof getNote !== 'function') {
        btn.classList.remove('np-note-active');
        btn.style.color = '';
        return;
    }
    const n = getNote(audioPlayerPath);
    const active = !!(n && ((n.note && n.note.trim()) || (n.tags && n.tags.length > 0)));
    btn.classList.toggle('np-note-active', active);
    btn.style.color = active ? 'var(--green)' : '';
}

function tagCurrentTrack() {
    if (!audioPlayerPath) return;
    const sample = typeof allAudioSamples !== 'undefined' && findByPath(allAudioSamples, audioPlayerPath);
    const name = sample ? sample.name : audioPlayerPath.split('/').pop().replace(/\.[^.]+$/, '');
    if (typeof showNoteEditor === 'function') showNoteEditor(audioPlayerPath, name);
}

function collapsePlayer() {
    document.getElementById('audioNowPlaying').classList.remove('expanded');
    prefs.setItem('playerExpanded', 'off');
}

function hidePlayer() {
    const np = document.getElementById('audioNowPlaying');
    prefs.setItem('playerExpanded', np.classList.contains('expanded') ? 'on' : 'off');
    // Hide player but keep audio playing
    np.classList.remove('active');
    const pill = document.getElementById('audioRestorePill');
    if (pill && audioPlayerPath && isAudioPlaying()) {
        pill.classList.add('active');
    }
}

function showPlayer() {
    const pill = document.getElementById('audioRestorePill');
    if (pill) pill.classList.remove('active');
    const np = document.getElementById('audioNowPlaying');
    np.classList.add('active');
    if (prefs.getItem('playerExpanded') === 'on') np.classList.add('expanded');
    // Restore saved size
    const saved = prefs.getItem('modal_audioNowPlaying');
    if (saved) {
        try {
            const geo = JSON.parse(saved);
            if (geo.width > 200) np.style.width = geo.width + 'px';
            if (geo.height > 100) np.style.height = geo.height + 'px';
        } catch {
        }
    }
    // Force a synchronous reflow so the visualizer canvas has resolved
    // dimensions on the very first rAF frame. Without this, release WebView
    // defers layout and the canvas renders at 0px wide until a drag event.
    void np.offsetWidth;
    renderRecentlyPlayed();
    updateNowPlayingBtn();
}

// Double-click to expand/collapse player
document.getElementById('audioNowPlaying').addEventListener('dblclick', (e) => {
    // Don't toggle if clicking controls
    if (e.target.closest('button, input, select, .now-playing-waveform, .np-history-item')) return;
    togglePlayerExpanded();
});

// Play from recently played list
document.getElementById('npHistoryList')?.addEventListener('click', (e) => {
    const item = e.target.closest('[data-action="playRecent"]');
    if (item) {
        e.stopPropagation();
        previewAudio(item.dataset.path);
    }
});

// ── Previous / Next / Shuffle ──
function prevTrack() {
    const hadExpanded = expandedMetaPath !== null;
    const items = getPlayerHistoryListItems();
    let prevPath = null;
    if (audioShuffling) {
        if (items.length === 0) return;
        prevPath = items[Math.floor(Math.random() * items.length)].path;
    } else {
        if (items.length === 0) return;
        const idx = items.findIndex(s => s.path === audioPlayerPath);
        if (idx < 0) {
            prevPath = items[items.length - 1].path;
        } else {
            const prevIdx = idx <= 0 ? items.length - 1 : idx - 1;
            prevPath = items[prevIdx].path;
        }
    }
    void (async () => {
        await previewAudio(prevPath);
        if (hadExpanded) await expandMetaForPath(prevPath);
    })();
}

/** @param { { autoplay?: boolean } } [opts] — When `autoplay`, advance using full Recently Played order (ignores the player search box). */
function nextTrack(opts) {
    const hadExpanded = expandedMetaPath !== null;
    const autoplay = opts && opts.autoplay === true;
    const items =
        autoplay && typeof recentlyPlayed !== 'undefined' && Array.isArray(recentlyPlayed) && recentlyPlayed.length > 0
            ? recentlyPlayed
            : getPlayerHistoryListItems();
    let nextPath = null;
    if (audioShuffling) {
        if (items.length === 0) return;
        nextPath = items[Math.floor(Math.random() * items.length)].path;
    } else {
        if (items.length === 0) return;
        const idx = items.findIndex(s => s.path === audioPlayerPath);
        if (idx < 0) {
            nextPath = items[0].path;
        } else {
            const nextIdx = (idx + 1) % items.length;
            nextPath = items[nextIdx].path;
        }
    }
    void (async () => {
        await previewAudio(nextPath);
        if (hadExpanded) await expandMetaForPath(nextPath);
    })();
}

function toggleShuffle() {
    audioShuffling = !audioShuffling;
    prefs.setItem('shuffleMode', audioShuffling ? 'on' : 'off');
    const btn = document.getElementById('npBtnShuffle');
    if (btn) btn.classList.toggle('active', audioShuffling);
}

function toggleMute() {
    const btn = document.getElementById('npBtnMute');
    const slider = document.getElementById('npVolume');
    const pctEl = document.getElementById('npVolumePct');
    if (audioMuted) {
        if (_enginePlaybackActive && typeof setAudioVolume === 'function') {
            const pct =
                typeof savedMuteVolumePct === 'number' && !Number.isNaN(savedMuteVolumePct)
                    ? savedMuteVolumePct
                    : Math.round(savedVolume * 100);
            setAudioVolume(String(Math.max(0, Math.min(100, pct))));
        } else {
            audioPlayer.volume = savedVolume;
            if (_gainNode) _gainNode.gain.value = savedVolume * parseFloat(document.getElementById('npGainSlider')?.value || '1');
            if (slider) slider.value = Math.round(savedVolume * 100);
            if (pctEl) pctEl.textContent = Math.round(savedVolume * 100) + '%';
        }
        audioMuted = false;
        if (btn) btn.innerHTML = '&#128264;';
    } else {
        if (_enginePlaybackActive && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function' && typeof setAudioVolume === 'function') {
            const raw = parseInt(prefs.getItem('audioVolume') || '100', 10);
            savedMuteVolumePct = Number.isNaN(raw) ? 100 : Math.max(0, Math.min(100, raw));
            setAudioVolume('0');
        } else {
            savedVolume = audioPlayer.volume;
            audioPlayer.volume = 0;
            if (_gainNode) _gainNode.gain.value = 0;
            if (slider) slider.value = 0;
            if (pctEl) pctEl.textContent = catalogFmt('ui.audio.volume_zero');
        }
        audioMuted = true;
        if (btn) btn.innerHTML = '&#128263;';
    }
}

// ── Waveform rendering ──
let _audioCtx = null;

let _waveformCache = {};
let _spectrogramCache = {};
const _WF_CACHE_MAX = 500;
let _wfCacheDirtyTimer = null;

async function loadWaveformCache() {
    try {
        _waveformCache = await window.vstUpdater.readCacheFile('waveform-cache.json');
    } catch {
        _waveformCache = {};
    }
    try {
        _spectrogramCache = await window.vstUpdater.readCacheFile('spectrogram-cache.json');
    } catch {
        _spectrogramCache = {};
    }
}

function _evictCache(cache) {
    const keys = Object.keys(cache);
    if (keys.length > _WF_CACHE_MAX) {
        for (const k of keys.slice(0, keys.length - _WF_CACHE_MAX)) delete cache[k];
    }
}

function _saveWaveformCache() {
    _evictCache(_waveformCache);
    _evictCache(_spectrogramCache);
    // Stagger to avoid blocking
    const saveWf = () => window.vstUpdater.writeCacheFile('waveform-cache.json', _waveformCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
    const saveSg = () => window.vstUpdater.writeCacheFile('spectrogram-cache.json', _spectrogramCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
    if (typeof requestIdleCallback === 'function') {
        requestIdleCallback(saveWf);
        requestIdleCallback(saveSg);
    } else {
        setTimeout(saveWf, 0);
        setTimeout(saveSg, 2000);
    }
}

function _debounceWfSave() {
    clearTimeout(_wfCacheDirtyTimer);
    _wfCacheDirtyTimer = setTimeout(_saveWaveformCache, 30000);
}

function _npWaveformDrawStale(wfSeq) {
    return wfSeq !== undefined && wfSeq !== _npWaveformDrawSeq;
}

async function drawWaveform(filePath, wfSeq) {
    const canvas = document.getElementById('npWaveformCanvas');
    if (!canvas) return;
    if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
    const container = canvas.parentElement;
    const { w: cw, h: ch } = await resolveWaveformBoxSize(container, 280, 24);
    if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(cw * dpr));
    canvas.height = Math.max(1, Math.round(ch * dpr));
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    if (_waveformCache[filePath]) {
        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        renderWaveformData(ctx, canvas, _waveformCache[filePath]);
        return;
    }

    const bars = Math.max(1, Math.min(Math.max(1, Math.floor(cw)), 800));
    const src = fileSrcForDecode(filePath);
    try {
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        let peaks = null;
        try {
            peaks = await decodePeaksViaWorker(src, bars);
        } catch {
            peaks = null;
        }

        if (!peaks) {
            if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
            ctx.strokeStyle = 'rgba(5,217,232,0.3)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, canvas.height / 2);
            ctx.lineTo(canvas.width, canvas.height / 2);
            ctx.stroke();
            return;
        }

        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        _waveformCache[filePath] = peaks;
        _evictCache(_waveformCache);
        _debounceWfSave();
        renderWaveformData(ctx, canvas, peaks);
    } catch {
        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        ctx.strokeStyle = 'rgba(5,217,232,0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, canvas.height / 2);
        ctx.lineTo(canvas.width, canvas.height / 2);
        ctx.stroke();
    }
}

function renderWaveformData(ctx, canvas, peaks) {
    const w = canvas.width;
    const h = canvas.height;
    const mid = h / 2;

    ctx.clearRect(0, 0, w, h);

    if (!peaks || peaks.length === 0) {
        ctx.strokeStyle = 'rgba(5,217,232,0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, mid);
        ctx.lineTo(w, mid);
        ctx.stroke();
        return;
    }

    // Support both old format (number[]) and new format ({max,min}[])
    const isNewFormat = peaks.length > 0 && typeof peaks[0] === 'object';

    if (isNewFormat) {
        // Draw filled waveform shape using min/max envelope
        const barW = w / peaks.length;

        // Top half (positive)
        ctx.beginPath();
        ctx.moveTo(0, mid);
        for (let i = 0; i < peaks.length; i++) {
            const x = (i + 0.5) * barW;
            const y = mid - peaks[i].max * mid * 0.92;
            if (i === 0) ctx.lineTo(x, y); else ctx.lineTo(x, y);
        }
        // Bottom half (negative) — trace back
        for (let i = peaks.length - 1; i >= 0; i--) {
            const x = (i + 0.5) * barW;
            const y = mid - peaks[i].min * mid * 0.92;
            ctx.lineTo(x, y);
        }
        ctx.closePath();

        // Gradient fill
        const grad = ctx.createLinearGradient(0, 0, w, 0);
        grad.addColorStop(0, 'rgba(5,217,232,0.5)');
        grad.addColorStop(0.5, 'rgba(108,108,232,0.5)');
        grad.addColorStop(1, 'rgba(211,0,197,0.5)');
        ctx.fillStyle = grad;
        ctx.fill();

        // Brighter center line for detail
        ctx.beginPath();
        for (let i = 0; i < peaks.length; i++) {
            const x = (i + 0.5) * barW;
            const rms = (peaks[i].max - peaks[i].min) * 0.35;
            const y1 = mid - rms * mid;
            const y2 = mid + rms * mid;
            ctx.moveTo(x, y1);
            ctx.lineTo(x, y2);
        }
        const grad2 = ctx.createLinearGradient(0, 0, w, 0);
        grad2.addColorStop(0, 'rgba(5,217,232,0.8)');
        grad2.addColorStop(1, 'rgba(211,0,197,0.8)');
        ctx.strokeStyle = grad2;
        ctx.lineWidth = 1;
        ctx.stroke();
    } else {
        // Legacy format: simple bars
        const barW = w / peaks.length;
        for (let i = 0; i < peaks.length; i++) {
            const barH = peaks[i] * mid * 0.9;
            const x = i * barW;
            const t = i / peaks.length;
            const r = Math.round(5 + t * 250);
            const g = Math.round(217 - t * 175);
            const b = Math.round(232 - t * 23);
            ctx.fillStyle = `rgba(${r},${g},${b},0.6)`;
            ctx.fillRect(x, mid - barH, barW - 0.5, barH * 2);
        }
    }
}

function _metaPanelStale(metaSeq, filePath) {
    return (metaSeq !== undefined && metaSeq !== _metaPanelDrawSeq) || expandedMetaPath !== filePath;
}

/**
 * Expanded-row waveform + spectrogram: prefer `audio_engine_invoke` (`waveform_preview` +
 * `spectrogram_preview`) when `vstUpdater.audioEngineInvoke` is available, else worker
 * (`decodeMetaVisualsViaWorker` / related). On failure, main-thread `drawMetaWaveform` →
 * `drawSpectrogram`. Spectrogram AudioEngine size uses `metaSpectrogramEnginePixelDims` (modest JSON).
 */
async function drawMetaPanelVisuals(filePath, metaSeq) {
    const wfCanvas = document.getElementById('metaWaveformCanvas');
    const sgCanvas = document.getElementById('metaSpectrogramCanvas');
    if (!wfCanvas || !sgCanvas) return;
    if (_metaPanelStale(metaSeq, filePath)) return;

    const wfCached = _waveformCache[filePath];
    const sgCached = _spectrogramCache[filePath];

    const container = wfCanvas.parentElement;
    const { w: cw, h: ch } = await resolveWaveformBoxSize(container, 560, 56);
    if (_metaPanelStale(metaSeq, filePath)) return;
    const dpr = window.devicePixelRatio || 1;
    wfCanvas.width = Math.max(1, Math.round(cw * dpr));
    wfCanvas.height = Math.max(1, Math.round(ch * dpr));
    const wfCtx = wfCanvas.getContext('2d');
    wfCtx.clearRect(0, 0, wfCanvas.width, wfCanvas.height);
    const sgCtx = sgCanvas.getContext('2d');
    const sgW = 800;
    const sgH = 80;
    sgCtx.clearRect(0, 0, sgW, sgH);

    if (wfCached && sgCached) {
        if (_metaPanelStale(metaSeq, filePath)) return;
        renderWaveformData(wfCtx, wfCanvas, wfCached);
        _metaSharedDecoded.path = filePath;
        _metaSharedDecoded.buffer = null;
        renderSpectrogramData(sgCtx, sgW, sgH, sgCached);
        _metaSharedDecoded.path = null;
        _metaSharedDecoded.buffer = null;
        return;
    }

    const url = fileSrcForDecode(filePath);
    const bars = Math.max(1, Math.min(Math.max(1, Math.floor(cw)), 800));
    const sgDims = metaSpectrogramEnginePixelDims();

    try {
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (_metaPanelStale(metaSeq, filePath)) return;
        if (!wfCached && !sgCached) {
            let peaks;
            let sgData;
            const fromEngine = await fetchMetaVisualsFromAudioEngine(filePath, bars, sgDims);
            if (fromEngine) {
                peaks = fromEngine.peaks;
                sgData = fromEngine.sgData;
            } else {
                const decoded = await decodeMetaVisualsViaWorker(url, bars);
                peaks = decoded.peaks;
                sgData = decoded.sgData;
            }
            if (_metaPanelStale(metaSeq, filePath)) return;
            _waveformCache[filePath] = peaks;
            _spectrogramCache[filePath] = sgData;
            _evictCache(_waveformCache);
            _evictCache(_spectrogramCache);
            _debounceWfSave();
            renderWaveformData(wfCtx, wfCanvas, peaks);
            _metaSharedDecoded.path = filePath;
            _metaSharedDecoded.buffer = null;
            renderSpectrogramData(sgCtx, sgW, sgH, sgData);
            _metaSharedDecoded.path = null;
            _metaSharedDecoded.buffer = null;
            return;
        }
        if (wfCached && !sgCached) {
            let sgData = await fetchSpectrogramPreviewFromEngine(filePath, sgDims);
            if (!sgData) sgData = await decodeSpectrogramViaWorker(url);
            if (_metaPanelStale(metaSeq, filePath)) return;
            _spectrogramCache[filePath] = sgData;
            _evictCache(_spectrogramCache);
            _debounceWfSave();
            renderWaveformData(wfCtx, wfCanvas, wfCached);
            _metaSharedDecoded.path = filePath;
            _metaSharedDecoded.buffer = null;
            renderSpectrogramData(sgCtx, sgW, sgH, sgData);
            _metaSharedDecoded.path = null;
            _metaSharedDecoded.buffer = null;
            return;
        }
        if (!wfCached && sgCached) {
            let peaks = await fetchWaveformPreviewFromEngine(filePath, bars);
            if (!peaks) peaks = await decodePeaksViaWorker(url, bars);
            if (_metaPanelStale(metaSeq, filePath)) return;
            _waveformCache[filePath] = peaks;
            _evictCache(_waveformCache);
            _debounceWfSave();
            renderWaveformData(wfCtx, wfCanvas, peaks);
            _metaSharedDecoded.path = filePath;
            _metaSharedDecoded.buffer = null;
            renderSpectrogramData(sgCtx, sgW, sgH, sgCached);
            _metaSharedDecoded.path = null;
            _metaSharedDecoded.buffer = null;
            return;
        }
    } catch (err) {
        if (_metaPanelStale(metaSeq, filePath)) return;
        const msg = err && err.message ? String(err.message) : String(err);
        console.warn('[audio-haxor] drawMetaPanelVisuals: worker decode failed, main-thread fallback', {
            path: filePath,
            url,
            bars,
            error: msg,
        });
        try {
            await drawMetaWaveform(filePath, metaSeq);
            if (_metaPanelStale(metaSeq, filePath)) return;
            await drawSpectrogram(filePath, metaSeq);
        } catch (fallbackErr) {
            const fb = fallbackErr && fallbackErr.message ? String(fallbackErr.message) : String(fallbackErr);
            console.warn('[audio-haxor] drawMetaPanelVisuals: main-thread fallback also failed', {
                path: filePath,
                error: fb,
            });
            if (_metaPanelStale(metaSeq, filePath)) return;
            wfCtx.strokeStyle = 'rgba(5,217,232,0.3)';
            wfCtx.lineWidth = 1;
            wfCtx.beginPath();
            wfCtx.moveTo(0, wfCanvas.height / 2);
            wfCtx.lineTo(wfCanvas.width, wfCanvas.height / 2);
            wfCtx.stroke();
            sgCtx.fillStyle = 'var(--text-dim)';
            sgCtx.font = '9px sans-serif';
            sgCtx.fillText('Spectrogram unavailable', 10, 40);
        }
    }
}

async function drawMetaWaveform(filePath, metaSeq) {
    const canvas = document.getElementById('metaWaveformCanvas');
    if (!canvas) return;
    if (_metaPanelStale(metaSeq, filePath)) return;
    const container = canvas.parentElement;
    const { w: cw, h: ch } = await resolveWaveformBoxSize(container, 560, 56);
    if (_metaPanelStale(metaSeq, filePath)) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(cw * dpr));
    canvas.height = Math.max(1, Math.round(ch * dpr));
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    if (_waveformCache[filePath]) {
        if (_metaPanelStale(metaSeq, filePath)) return;
        renderWaveformData(ctx, canvas, _waveformCache[filePath]);
        _metaSharedDecoded.path = filePath;
        _metaSharedDecoded.buffer = null;
        return;
    }

    const bars = Math.max(1, Math.min(Math.max(1, Math.floor(cw)), 800));
    const src = fileSrcForDecode(filePath);
    try {
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (_metaPanelStale(metaSeq, filePath)) return;
        if (!_audioCtx) _audioCtx = new AudioContext();
        const resp = await fetch(src);
        if (!resp.ok) throw new Error(`fetch ${resp.status}`);
        if (_metaPanelStale(metaSeq, filePath)) return;
        const buf = await resp.arrayBuffer();
        if (_metaPanelStale(metaSeq, filePath)) return;
        const audioBuf = await _audioCtx.decodeAudioData(buf.slice(0));
        if (_metaPanelStale(metaSeq, filePath)) return;
        _metaSharedDecoded.path = filePath;
        _metaSharedDecoded.buffer = audioBuf;
        const raw = audioBuf.getChannelData(0);
        const step = Math.floor(raw.length / bars);
        const peaks = [];
        for (let i = 0; i < bars; i++) {
            let max = 0;
            let min = 0;
            const start = i * step;
            for (let j = start; j < start + step && j < raw.length; j++) {
                if (raw[j] > max) max = raw[j];
                if (raw[j] < min) min = raw[j];
            }
            peaks.push({ max, min });
        }

        if (_metaPanelStale(metaSeq, filePath)) return;
        _waveformCache[filePath] = peaks;
        _evictCache(_waveformCache);
        _debounceWfSave();
        renderWaveformData(ctx, canvas, peaks);
    } catch {
        if (_metaPanelStale(metaSeq, filePath)) return;
        _metaSharedDecoded.path = filePath;
        _metaSharedDecoded.buffer = null;
        ctx.strokeStyle = 'rgba(5,217,232,0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, canvas.height / 2);
        ctx.lineTo(canvas.width, canvas.height / 2);
        ctx.stroke();
    }
}

async function drawSpectrogram(filePath, metaSeq) {
    const canvas = document.getElementById('metaSpectrogramCanvas');
    if (!canvas) return;
    if (_metaPanelStale(metaSeq, filePath)) return;
    const ctx = canvas.getContext('2d');
    const w = 800;
    const h = 80;
    ctx.clearRect(0, 0, w, h);

    if (_spectrogramCache[filePath]) {
        if (_metaPanelStale(metaSeq, filePath)) return;
        renderSpectrogramData(ctx, w, h, _spectrogramCache[filePath]);
        _metaSharedDecoded.path = null;
        _metaSharedDecoded.buffer = null;
        return;
    }

    try {
        let audioBuf = null;
        if (_metaSharedDecoded.path === filePath && _metaSharedDecoded.buffer) {
            audioBuf = _metaSharedDecoded.buffer;
        } else {
            if (typeof yieldToBrowser === 'function') await yieldToBrowser();
            if (_metaPanelStale(metaSeq, filePath)) return;
            if (!_audioCtx) _audioCtx = new AudioContext();
            const src = fileSrcForDecode(filePath);
            const resp = await fetch(src);
            if (!resp.ok) throw new Error(`fetch ${resp.status}`);
            if (_metaPanelStale(metaSeq, filePath)) return;
            const buf = await resp.arrayBuffer();
            if (_metaPanelStale(metaSeq, filePath)) return;
            audioBuf = await _audioCtx.decodeAudioData(buf.slice(0));
        }
        _metaSharedDecoded.path = null;
        _metaSharedDecoded.buffer = null;
        if (_metaPanelStale(metaSeq, filePath)) return;
        const raw = audioBuf.getChannelData(0);

        const fftSize = 1024;
        const hop = fftSize / 2;
        const numBins = fftSize / 2;
        const numFrames = Math.floor((raw.length - fftSize) / hop);
        if (numFrames <= 0) return;

        const cols = Math.min(w, numFrames);
        const frameStep = Math.max(1, Math.floor(numFrames / cols));
        const freqBins = 64;

        const hannWindow = new Float32Array(fftSize);
        for (let i = 0; i < fftSize; i++) {
            hannWindow[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / (fftSize - 1)));
        }

        const bitRev = new Uint32Array(fftSize);
        const bits = Math.log2(fftSize);
        for (let i = 0; i < fftSize; i++) {
            let reversed = 0;
            for (let b = 0; b < bits; b++) {
                reversed = (reversed << 1) | ((i >> b) & 1);
            }
            bitRev[i] = reversed;
        }

        const twiddleRe = new Float64Array(fftSize / 2);
        const twiddleIm = new Float64Array(fftSize / 2);
        for (let i = 0; i < fftSize / 2; i++) {
            const angle = (-2 * Math.PI * i) / fftSize;
            twiddleRe[i] = Math.cos(angle);
            twiddleIm[i] = Math.sin(angle);
        }

        const re = new Float64Array(fftSize);
        const im = new Float64Array(fftSize);
        const sgData = [];

        for (let col = 0; col < cols; col++) {
            if (col > 0 && col % 4 === 0) {
                if (typeof yieldToBrowser === 'function') await yieldToBrowser();
                if (_metaPanelStale(metaSeq, filePath)) return;
            }
            const frameIdx = col * frameStep;
            const offset = frameIdx * hop;
            if (offset + fftSize > raw.length) break;

            for (let i = 0; i < fftSize; i++) {
                re[bitRev[i]] = raw[offset + i] * hannWindow[i];
                im[bitRev[i]] = 0;
            }

            for (let size = 2; size <= fftSize; size *= 2) {
                const halfSize = size / 2;
                const step = fftSize / size;
                for (let i = 0; i < fftSize; i += size) {
                    for (let j = 0; j < halfSize; j++) {
                        const idx = j * step;
                        const tRe = twiddleRe[idx] * re[i + j + halfSize] - twiddleIm[idx] * im[i + j + halfSize];
                        const tIm = twiddleRe[idx] * im[i + j + halfSize] + twiddleIm[idx] * re[i + j + halfSize];
                        re[i + j + halfSize] = re[i + j] - tRe;
                        im[i + j + halfSize] = im[i + j] - tIm;
                        re[i + j] += tRe;
                        im[i + j] += tIm;
                    }
                }
            }

            const mags = new Array(freqBins);
            for (let bin = 0; bin < freqBins; bin++) {
                const freqLo = Math.pow(bin / freqBins, 2) * numBins;
                const freqHi = Math.pow((bin + 1) / freqBins, 2) * numBins;
                const lo = Math.floor(freqLo);
                const hi = Math.max(lo + 1, Math.floor(freqHi));
                let energy = 0;
                for (let k = lo; k < hi && k < numBins; k++) {
                    energy += Math.sqrt(re[k] * re[k] + im[k] * im[k]);
                }
                mags[bin] = Math.round((energy / Math.max(1, hi - lo)) * 100) / 100;
            }
            sgData.push(mags);
        }

        if (_metaPanelStale(metaSeq, filePath)) return;
        _spectrogramCache[filePath] = sgData;
        _debounceWfSave();
        renderSpectrogramData(ctx, w, h, sgData);
    } catch {
        _metaSharedDecoded.path = null;
        _metaSharedDecoded.buffer = null;
        if (_metaPanelStale(metaSeq, filePath)) return;
        ctx.fillStyle = 'var(--text-dim)';
        ctx.font = '9px sans-serif';
        ctx.fillText('Spectrogram unavailable', 10, 40);
    }
}

function renderSpectrogramData(ctx, w, h, sgData) {
    const cols = sgData.length;
    if (cols === 0) return;
    const freqBins = sgData[0].length;
    for (let col = 0; col < cols; col++) {
        const x = (col / cols) * w;
        const colWidth = Math.ceil(w / cols);
        for (let bin = 0; bin < freqBins; bin++) {
            const mag = Math.min(1, Math.log1p(sgData[col][bin] * 4) / 3);
            const y = h - (bin / freqBins) * h;
            const binH = Math.ceil(h / freqBins);
            const r = Math.floor(mag * 211 + (1 - mag) * 5);
            const g = Math.floor(mag * mag * 50);
            const b = Math.floor(mag * 197 + (1 - mag) * 20);
            const a = mag * 0.9 + 0.05;
            ctx.fillStyle = `rgba(${r},${g},${b},${a})`;
            ctx.fillRect(x, y - binH, colWidth, binH);
        }
    }
}

function seekMetaWaveform(event) {
    const box = document.getElementById('metaWaveformBox');
    logWaveformSeek('seekMetaWaveform', {
        hasBox: !!box,
        boxPath: box?.dataset?.path || null,
        audioPlayerPath: audioPlayerPath || null,
    });
    if (!box) {
        logWaveformSeek('seekMetaWaveform_abort', { reason: 'missing_metaWaveformBox' });
        return;
    }
    if (!audioPlayerPath || audioPlayerPath !== box.dataset.path) {
        logWaveformSeek('seekMetaWaveform_abort', {
            reason: 'path_mismatch',
            expected: box.dataset.path || null,
            current: audioPlayerPath || null,
        });
        return;
    }
    const rect = box.getBoundingClientRect();
    if (rect.width <= 0) {
        logWaveformSeek('seekMetaWaveform_abort', { reason: 'zero_width_rect', rect: { w: rect.width, h: rect.height } });
        return;
    }
    const pct = (event.clientX - rect.left) / rect.width;
    logWaveformSeek('seekMetaWaveform', { clientX: event.clientX, pct, rectLeft: rect.left, rectWidth: rect.width });
    seekPlaybackToPercent(pct);
}

function updateMetaLine() {
    const el = document.getElementById('npMetaLine');
    if (!el || !audioPlayerPath) {
        if (el) el.textContent = '';
        return;
    }
    if (isEngineUnplayablePath(audioPlayerPath)) {
        const x = audioPlayerPath.split('.').pop().toLowerCase();
        el.textContent = _audioFmt('ui.audio.not_playable_in_audio_engine', {ext: x.toUpperCase()});
        return;
    }
    const sample = findByPath(allAudioSamples, audioPlayerPath);
    if (!sample) {
        el.textContent = audioPlayerPath.split('/').pop();
        return;
    }
    const parts = [sample.format, sample.sizeFormatted];
    const bpmShow = sample.bpm || _bpmCache[audioPlayerPath];
    const keyShow = sample.key || _keyCache[audioPlayerPath];
    const lufsShow = sample.lufs != null ? sample.lufs : (_lufsCache[audioPlayerPath] != null ? _lufsCache[audioPlayerPath] : null);
    if (bpmShow) parts.push(bpmShow + ' BPM');
    if (keyShow) parts.push(keyShow);
    if (lufsShow != null) parts.push(lufsShow + ' LUFS');
    if (sample.directory) parts.push(sample.directory);
    el.textContent = parts.join(' \u2022 ');
}

// ── Visualizer init (canvas-based FFT replaces random CSS bars) ──

// ── Player section drag-to-reorder (Trello-style) ──
(function initPlayerSectionDrag() {
    const body = document.querySelector('.np-body');
    if (!body) return;
    initDragReorder(body, '.np-section', 'playerSectionOrder', {
        getKey: (el) => el.dataset.npSection,
        onReorder: () => {
            const eqPanel = document.getElementById('npEqSection');
            if (eqPanel) body.appendChild(eqPanel);
        },
    });
})();

// ── Drag-to-dock ──
(function initPlayerDrag() {
    const np = document.getElementById('audioNowPlaying');
    const handle = document.getElementById('npDragHandle');
    const overlay = document.getElementById('dockOverlay');
    if (!np || !handle || !overlay) return;
    const zones = {tl: 'dockTL', tr: 'dockTR', bl: 'dockBL', br: 'dockBR'};
    let dragging = false, startX, startY, origX, origY;

    function getCurrentDock() {
        for (const cls of np.classList) {
            if (cls.startsWith('dock-')) return cls;
        }
        return 'dock-br';
    }

    function setDock(dock) {
        np.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
        np.classList.add(dock);
        prefs.setItem('playerDock', dock);
    }

    // Expose the dock restore — must be called AFTER prefs.load() (app.js does this).
    // Reading prefs here at IIFE time is too early; cache is still empty.
    window.restorePlayerDock = function () {
        const saved = prefs.getItem('playerDock');
        if (saved && ['dock-tl', 'dock-tr', 'dock-bl', 'dock-br'].includes(saved)) {
            setDock(saved);
        }
    };

    function nearestDock(x, y) {
        const cx = window.innerWidth / 2;
        const cy = window.innerHeight / 2;
        if (x < cx && y < cy) return 'dock-tl';
        if (x >= cx && y < cy) return 'dock-tr';
        if (x < cx && y >= cy) return 'dock-bl';
        return 'dock-br';
    }

    function highlightZone(dock) {
        Object.values(zones).forEach(id => document.getElementById(id).classList.remove('active'));
        const map = {'dock-tl': 'dockTL', 'dock-tr': 'dockTR', 'dock-bl': 'dockBL', 'dock-br': 'dockBR'};
        const el = document.getElementById(map[dock]);
        if (el) el.classList.add('active');
    }

    const toolbar = np.querySelector('.np-toolbar');

    function onDragStart(e) {
        if (e.button !== 0) return;
        // Don't drag if clicking toolbar buttons
        if (e.target.closest('.np-toolbar-actions')) return;
        e.preventDefault();
        e.stopPropagation(); // prevent generic drag-reorder from intercepting
        dragging = true;
        startX = e.clientX;
        startY = e.clientY;
        const rect = np.getBoundingClientRect();
        origX = rect.left;
        origY = rect.top;

        // Switch to absolute positioning for free drag
        np.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
        np.style.left = origX + 'px';
        np.style.top = origY + 'px';
        np.style.right = 'auto';
        np.style.bottom = 'auto';
        np.classList.add('dragging');

        // Position dock zones with pixel values — CSS % doesn't resolve in release WebView
        const vw = window.innerWidth, vh = window.innerHeight, gap = 4;
        const zw = Math.floor(vw / 2 - gap * 1.5) + 'px';
        const zh = Math.floor(vh / 2 - gap * 1.5) + 'px';
        const mid = Math.ceil(vw / 2 + gap / 2) + 'px';
        const midY = Math.ceil(vh / 2 + gap / 2) + 'px';
        const g = gap + 'px';
        const tl = document.getElementById('dockTL');
        const tr = document.getElementById('dockTR');
        const bl = document.getElementById('dockBL');
        const br = document.getElementById('dockBR');
        tl.style.cssText = `top:${g};left:${g};width:${zw};height:${zh}`;
        tr.style.cssText = `top:${g};left:${mid};width:${zw};height:${zh}`;
        bl.style.cssText = `top:${midY};left:${g};width:${zw};height:${zh}`;
        br.style.cssText = `top:${midY};left:${mid};width:${zw};height:${zh}`;

        overlay.classList.add('visible');
    }

    handle.addEventListener('mousedown', onDragStart, true);
    toolbar.addEventListener('mousedown', onDragStart, true);

    document.addEventListener('mousemove', (e) => {
        if (!dragging) return;
        const dx = e.clientX - startX;
        const dy = e.clientY - startY;
        np.style.left = (origX + dx) + 'px';
        np.style.top = (origY + dy) + 'px';
        highlightZone(nearestDock(e.clientX, e.clientY));
    });

    document.addEventListener('mouseup', (e) => {
        if (!dragging) return;
        dragging = false;
        np.classList.remove('dragging');
        overlay.classList.remove('visible');
        Object.values(zones).forEach(id => document.getElementById(id).classList.remove('active'));

        // Clear position styles and snap to dock (preserve width/height)
        const savedW = np.style.width;
        const savedH = np.style.height;
        np.style.left = '';
        np.style.top = '';
        np.style.right = '';
        np.style.bottom = '';
        np.style.width = savedW;
        np.style.height = savedH;

        const dock = nearestDock(e.clientX, e.clientY);
        np.classList.add('snapping');
        setDock(dock);

        // Save dimensions + dock to prefs
        prefs.setItem('modal_audioNowPlaying', JSON.stringify({
            width: np.offsetWidth,
            height: np.offsetHeight,
        }));
        setTimeout(() => np.classList.remove('snapping'), 300);
    });
})();

// ── Corner + edge resize ──
// Use the same drag/resize system as all modals
(function initPlayerResize() {
    const np = document.getElementById('audioNowPlaying');
    // Attach resize handles immediately (synchronous, no prefs needed).
    if (typeof initModalDragResize === 'function') {
        initModalDragResize(np);
    }
    // Dimension restore must wait for prefs.load() — expose it for app.js to call.
    window.restorePlayerDimensions = function () {
        const savedGeo = prefs.getItem('modal_audioNowPlaying');
        if (savedGeo) {
            try {
                const geo = JSON.parse(savedGeo);
                if (geo.width > 200) np.style.width = geo.width + 'px';
                if (geo.height > 100) np.style.height = geo.height + 'px';
            } catch {
            }
        }
        if (!np.style.width) np.style.width = '360px';
        if (typeof window.applyNpFftHeightFromPrefs === 'function') window.applyNpFftHeightFromPrefs();
        if (typeof window.applyNpEqCanvasHeightFromPrefs === 'function') window.applyNpEqCanvasHeightFromPrefs();
        if (typeof window.applyAeEqCanvasHeightFromPrefs === 'function') window.applyAeEqCanvasHeightFromPrefs();
    };
    // Set a safe default immediately so the player has a size before prefs load.
    if (!np.style.width) np.style.width = '360px';
})();

// ── FFT spectrum strip — vertical resize (prefs `npFftHeight`) ──
(function initNpFftResize() {
    const handle = document.getElementById('npFftResizeHandle');
    const viz = document.getElementById('npVisualizer');
    if (!handle || !viz) return;
    const MIN = 32;
    const MAX = 480;
    const PREF_KEY = 'npFftHeight';

    function applyNpFftHeightFromPrefs() {
        const raw = prefs.getItem(PREF_KEY);
        if (raw != null && raw !== '') {
            const h = parseInt(String(raw), 10);
            if (Number.isFinite(h) && h >= MIN && h <= MAX) {
                viz.style.height = h + 'px';
                return;
            }
        }
        viz.style.height = '';
    }
    window.applyNpFftHeightFromPrefs = applyNpFftHeightFromPrefs;

    handle.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        const startY = e.clientY;
        const startH = viz.getBoundingClientRect().height;
        handle.setPointerCapture(e.pointerId);

        function onMove(ev) {
            const dy = ev.clientY - startY;
            const nh = Math.round(Math.min(MAX, Math.max(MIN, startH + dy)));
            viz.style.height = nh + 'px';
        }
        function onUp(ev) {
            handle.removeEventListener('pointermove', onMove);
            handle.removeEventListener('pointerup', onUp);
            handle.removeEventListener('pointercancel', onUp);
            try {
                handle.releasePointerCapture(ev.pointerId);
            } catch (_) {}
            prefs.setItem(PREF_KEY, String(Math.round(viz.getBoundingClientRect().height)));
        }
        handle.addEventListener('pointermove', onMove);
        handle.addEventListener('pointerup', onUp);
        handle.addEventListener('pointercancel', onUp);
    });
})();

// ── Parametric EQ canvas height (floating player + Audio Engine tab; prefs `npEqCanvasHeight` / `aeEqCanvasHeight`) ──
(function initNpEqCanvasResize() {
    const handle = document.getElementById('npEqCanvasResizeHandle');
    const wrap = document.getElementById('npEqCanvasWrap');
    if (!handle || !wrap) return;
    const MIN = 80;
    const MAX = 480;
    const PREF_KEY = 'npEqCanvasHeight';

    function applyNpEqCanvasHeightFromPrefs() {
        const raw = prefs.getItem(PREF_KEY);
        if (raw != null && raw !== '') {
            const h = parseInt(String(raw), 10);
            if (Number.isFinite(h) && h >= MIN && h <= MAX) {
                wrap.style.height = h + 'px';
                return;
            }
        }
        wrap.style.height = '';
    }
    window.applyNpEqCanvasHeightFromPrefs = applyNpEqCanvasHeightFromPrefs;

    handle.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        const startY = e.clientY;
        const startH = wrap.getBoundingClientRect().height;
        handle.setPointerCapture(e.pointerId);

        function onMove(ev) {
            const dy = ev.clientY - startY;
            const nh = Math.round(Math.min(MAX, Math.max(MIN, startH + dy)));
            wrap.style.height = nh + 'px';
        }
        function onUp(ev) {
            handle.removeEventListener('pointermove', onMove);
            handle.removeEventListener('pointerup', onUp);
            handle.removeEventListener('pointercancel', onUp);
            try {
                handle.releasePointerCapture(ev.pointerId);
            } catch (_) {}
            prefs.setItem(PREF_KEY, String(Math.round(wrap.getBoundingClientRect().height)));
            if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
        }
        handle.addEventListener('pointermove', onMove);
        handle.addEventListener('pointerup', onUp);
        handle.addEventListener('pointercancel', onUp);
    });
    applyNpEqCanvasHeightFromPrefs();
})();

(function initAeEqCanvasResize() {
    const handle = document.getElementById('aeEqCanvasResizeHandle');
    const wrap = document.getElementById('aeEqCanvasWrap');
    if (!handle || !wrap) return;
    const MIN = 80;
    const MAX = 480;
    const PREF_KEY = 'aeEqCanvasHeight';

    function applyAeEqCanvasHeightFromPrefs() {
        const raw = prefs.getItem(PREF_KEY);
        if (raw != null && raw !== '') {
            const h = parseInt(String(raw), 10);
            if (Number.isFinite(h) && h >= MIN && h <= MAX) {
                wrap.style.height = h + 'px';
                return;
            }
        }
        wrap.style.height = '';
    }
    window.applyAeEqCanvasHeightFromPrefs = applyAeEqCanvasHeightFromPrefs;

    handle.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        const startY = e.clientY;
        const startH = wrap.getBoundingClientRect().height;
        handle.setPointerCapture(e.pointerId);

        function onMove(ev) {
            const dy = ev.clientY - startY;
            const nh = Math.round(Math.min(MAX, Math.max(MIN, startH + dy)));
            wrap.style.height = nh + 'px';
        }
        function onUp(ev) {
            handle.removeEventListener('pointermove', onMove);
            handle.removeEventListener('pointerup', onUp);
            handle.removeEventListener('pointercancel', onUp);
            try {
                handle.releasePointerCapture(ev.pointerId);
            } catch (_) {}
            prefs.setItem(PREF_KEY, String(Math.round(wrap.getBoundingClientRect().height)));
            if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
        }
        handle.addEventListener('pointermove', onMove);
        handle.addEventListener('pointerup', onUp);
        handle.addEventListener('pointercancel', onUp);
    });
    applyAeEqCanvasHeightFromPrefs();
})();

// ── Parametric EQ Visualization (floating player + Audio Engine tab; shared Web Audio graph + FFT) ──
(function initParametricEQ() {
    const npCanvas = document.getElementById('npEqCanvas');
    const aeCanvas = document.getElementById('aeEqCanvas');
    if (!npCanvas && !aeCanvas) return;

    let _eqSpectrumBuf = null;

    function eqBandLabel(id) {
        const k = id === 'low' ? 'ui.eq.band_low' : id === 'mid' ? 'ui.eq.band_mid' : 'ui.eq.band_high';
        return typeof appFmt === 'function' ? appFmt(k) : id.toUpperCase();
    }

    const bands = [
        {
            id: 'low', get filter() {
                return _eqLow;
            }, color: '#05d9e8'
        },
        {
            id: 'mid', get filter() {
                return _eqMid;
            }, color: '#d300c5'
        },
        {
            id: 'high', get filter() {
                return _eqHigh;
            }, color: '#ff2a6d'
        },
    ];

    const FREQ_MIN = 20, FREQ_MAX = 20000;
    const GAIN_MIN = -12, GAIN_MAX = 12;
    /** Log-spaced samples for Ableton-style smooth spectrum (not one vertex per FFT bin). */
    const EQ_SPECTRUM_POINTS = 512;
    const EQ_MARGIN_BOTTOM = 22;

    /** Interpolate FFT bin magnitudes in linear bin index space (0–1). */
    function sampleSpectrumMag01(freqHz, dataArr, bufLen, sampleRate, fftSize) {
        const binF = (freqHz * fftSize) / sampleRate;
        if (binF < 0 || bufLen < 2) return 0;
        const i0 = Math.floor(binF);
        const i1 = Math.min(bufLen - 1, i0 + 1);
        const frac = binF - i0;
        const v0 = dataArr[Math.max(0, i0)] / 255;
        const v1 = dataArr[i1] / 255;
        return v0 + frac * (v1 - v0);
    }

    function freqToX(freq, w) {
        return (Math.log10(freq / FREQ_MIN) / Math.log10(FREQ_MAX / FREQ_MIN)) * w;
    }

    function xToFreq(x, w) {
        return FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, x / w);
    }

    function gainToY(gain, h) {
        return h / 2 - (gain / GAIN_MAX) * (h / 2 - 10);
    }

    function yToGain(y, h) {
        return -((y - h / 2) / (h / 2 - 10)) * GAIN_MAX;
    }

    let _paramEqRafId = null;
    let _dragState = null;
    let _eqDragEngineSyncTs = 0;

    /** Light log grid + 0 dB line (transparent background; EQ response stroke stays cyan). */
    function drawEqPanelGrid(ctx, w, h, zeroY) {
        ctx.strokeStyle = 'rgba(255,255,255,0.05)';
        ctx.lineWidth = 1;
        for (const f of [100, 1000, 10000]) {
            const x = freqToX(f, w);
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, h);
            ctx.stroke();
            ctx.fillStyle = 'rgba(255,255,255,0.15)';
            ctx.font = '9px sans-serif';
            ctx.fillText(f >= 1000 ? (f / 1000) + 'k' : f, x + 2, h - 3);
        }
        ctx.strokeStyle = 'rgba(255,255,255,0.1)';
        ctx.beginPath();
        ctx.moveTo(0, zeroY);
        ctx.lineTo(w, zeroY);
        ctx.stroke();
    }

    /** Log-spaced spectrum fill + top outline (magenta→cyan; interpolated magnitudes). */
    function drawEqSpectrumFill(ctx, w, h, dataArr, bufLen, sampleRate, fftSize) {
        const plotH = h - EQ_MARGIN_BOTTOM;
        const grad = ctx.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, 'rgba(211,0,197,0.25)');
        grad.addColorStop(0.5, 'rgba(5,217,232,0.12)');
        grad.addColorStop(1, 'rgba(5,217,232,0.02)');
        ctx.beginPath();
        ctx.moveTo(0, h);
        let firstY = h;
        for (let i = 0; i < EQ_SPECTRUM_POINTS; i++) {
            const t = i / (EQ_SPECTRUM_POINTS - 1);
            const freq = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, t);
            const x = freqToX(freq, w);
            const mag = sampleSpectrumMag01(freq, dataArr, bufLen, sampleRate, fftSize);
            const y = plotH - mag * plotH;
            if (i === 0) {
                firstY = y;
                ctx.lineTo(0, y);
            } else {
                ctx.lineTo(x, y);
            }
        }
        ctx.lineTo(w, h);
        ctx.lineTo(0, h);
        ctx.closePath();
        ctx.fillStyle = grad;
        ctx.fill();
        ctx.beginPath();
        ctx.moveTo(0, firstY);
        for (let i = 0; i < EQ_SPECTRUM_POINTS; i++) {
            const t = i / (EQ_SPECTRUM_POINTS - 1);
            const freq = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, t);
            const x = freqToX(freq, w);
            const mag = sampleSpectrumMag01(freq, dataArr, bufLen, sampleRate, fftSize);
            const y = plotH - mag * plotH;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = 'rgba(5,217,232,0.45)';
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    function drawParametricEqOnCanvas(canvas) {
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const useEngineEqSpectrum = engineSpectrumLive();
        syncNpFftSpectrumAxisPins(useEngineEqSpectrum);
        /* Canvas size is synced via ResizeObserver → primeCanvasSize only — not here (per-frame
         * getBoundingClientRect + width/height writes reset the bitmap and can fight layout). */
        if ((canvas.width || 0) < 2 || (canvas.height || 0) < 2) {
            primeCanvasSize(canvas);
        }
        ensureParametricEqCanvasMinBitmap(canvas);
        const w = canvas.width || 800;
        const h = canvas.height || 120;
        ctx.clearRect(0, 0, w, h);
        const zeroY = gainToY(0, h);
        drawEqPanelGrid(ctx, w, h, zeroY);

        if (useEngineEqSpectrum) {
            const axis = getPinnedEngineSpectrumAxis();
            if (axis) {
                const dataArr = window._engineSpectrumU8;
                if (!dataArr) {
                    /* skip spectrum fill */
                } else {
                const bufLen = Math.min(1024, dataArr.length);
                const paused = typeof window.isFftAnimationPaused === 'function' && window.isFftAnimationPaused();
                if (!paused) {
                    if (!_eqSpectrumBuf || _eqSpectrumBuf.length !== bufLen) _eqSpectrumBuf = new Uint8Array(bufLen);
                    _eqSpectrumBuf.set(dataArr.subarray(0, bufLen));
                }
                if (_eqSpectrumBuf && _eqSpectrumBuf.length === bufLen) {
                    const sampleRate = axis.sr;
                    const fftSize = axis.fft;
                    drawEqSpectrumFill(ctx, w, h, _eqSpectrumBuf, bufLen, sampleRate, fftSize);
                }
                }
            }
        } else if (_analyser && _playbackCtx && typeof isAudioPlaying === 'function' && isAudioPlaying()) {
            const bufLen = _analyser.frequencyBinCount;
            const paused = typeof window.isFftAnimationPaused === 'function' && window.isFftAnimationPaused();
            if (!paused) {
                if (!_eqSpectrumBuf || _eqSpectrumBuf.length !== bufLen) _eqSpectrumBuf = new Uint8Array(bufLen);
                _analyser.getByteFrequencyData(_eqSpectrumBuf);
            }
            if (_eqSpectrumBuf && _eqSpectrumBuf.length === bufLen) {
                if (_npFftWebAxisSrHz == null) {
                    _npFftWebAxisSrHz = _playbackCtx.sampleRate;
                    _npFftWebAxisFftSize = _analyser.fftSize;
                }
                const sampleRate = _npFftWebAxisSrHz;
                const fftSizeForBins = _npFftWebAxisFftSize;
                drawEqSpectrumFill(ctx, w, h, _eqSpectrumBuf, bufLen, sampleRate, fftSizeForBins);
            }
        }

        if (_eqLow && _eqMid && _eqHigh) {
            const nPoints = 480;
            const freqs = new Float32Array(nPoints);
            for (let i = 0; i < nPoints; i++) {
                freqs[i] = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, i / (nPoints - 1));
            }
            const magLow = new Float32Array(nPoints), phaseLow = new Float32Array(nPoints);
            const magMid = new Float32Array(nPoints), phaseMid = new Float32Array(nPoints);
            const magHigh = new Float32Array(nPoints), phaseHigh = new Float32Array(nPoints);
            _eqLow.getFrequencyResponse(freqs, magLow, phaseLow);
            _eqMid.getFrequencyResponse(freqs, magMid, phaseMid);
            _eqHigh.getFrequencyResponse(freqs, magHigh, phaseHigh);

            ctx.beginPath();
            ctx.strokeStyle = 'rgba(5,217,232,0.6)';
            ctx.lineWidth = 2;
            for (let i = 0; i < nPoints; i++) {
                const totalDb = 20 * Math.log10(magLow[i] * magMid[i] * magHigh[i]);
                const x = freqToX(freqs[i], w);
                const y = gainToY(Math.max(GAIN_MIN, Math.min(GAIN_MAX, totalDb)), h);
                if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
            }
            ctx.stroke();

            const lastX = freqToX(freqs[nPoints - 1], w);
            ctx.lineTo(lastX, zeroY);
            ctx.lineTo(freqToX(freqs[0], w), zeroY);
            ctx.closePath();
            ctx.fillStyle = 'rgba(5,217,232,0.05)';
            ctx.fill();
        }

        for (const band of bands) {
            if (!band.filter) continue;
            const x = freqToX(band.filter.frequency.value, w);
            const y = gainToY(band.filter.gain.value, h);

            ctx.beginPath();
            ctx.arc(x, y, 12, 0, Math.PI * 2);
            ctx.fillStyle = band.color + '15';
            ctx.fill();

            ctx.beginPath();
            ctx.arc(x, y, 6, 0, Math.PI * 2);
            ctx.fillStyle = band.color;
            ctx.fill();
            ctx.strokeStyle = '#fff';
            ctx.lineWidth = 1.5;
            ctx.stroke();

            ctx.fillStyle = band.color;
            ctx.font = 'bold 8px Orbitron, sans-serif';
            ctx.fillText(eqBandLabel(band.id), x + 10, y - 4);
            ctx.fillStyle = 'rgba(255,255,255,0.5)';
            ctx.font = '8px sans-serif';
            ctx.fillText(Math.round(band.filter.frequency.value) + 'Hz ' + band.filter.gain.value.toFixed(1) + 'dB', x + 10, y + 8);
        }
    }

    function parametricEqTick() {
        _paramEqRafId = null;
        if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return;
        const canvases = [];
        const eqSec = document.getElementById('npEqSection');
        if (npCanvas && eqSec && eqSec.classList.contains('visible')) canvases.push(npCanvas);
        const aeTab = document.getElementById('tabAudioEngine');
        if (aeCanvas && aeTab && aeTab.classList.contains('active')) canvases.push(aeCanvas);

        if (canvases.length === 0) return;

        try {
            ensureAudioGraph();
            for (const c of canvases) {
                drawParametricEqOnCanvas(c);
            }
        } catch (err) {
            if (typeof console !== 'undefined' && typeof console.error === 'function') {
                console.error('parametricEqTick', err);
            }
        }
        _paramEqRafId = requestAnimationFrame(parametricEqTick);
    }

    function scheduleParametricEqFrame() {
        if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return;
        if (_paramEqRafId != null) return;
        _paramEqRafId = requestAnimationFrame(parametricEqTick);
    }

    function cancelParametricEqRaf() {
        if (_paramEqRafId != null) {
            cancelAnimationFrame(_paramEqRafId);
            _paramEqRafId = null;
        }
    }

    function primeCanvasSize(canvas) {
        if (!canvas) return;
        const wrap = canvas.parentElement;
        if (!wrap) return;
        const br = wrap.getBoundingClientRect();
        /* Floor dimensions to avoid subpixel oscillation (ResizeObserver ↔ canvas bitmap ↔ layout creep). */
        let w = Math.floor(br.width > 1 ? br.width : wrap.clientWidth);
        let h = Math.floor(br.height > 1 ? br.height : wrap.clientHeight);
        /* Never set a 1×1 bitmap: WKWebView can report 0×0 before layout; scaling that up looks like a blank black panel. */
        if (w < 2 || h < 2) return;
        const dw = Math.abs(w - canvas.width);
        const dh = Math.abs(h - canvas.height);
        if (dw > 1 || dh > 1) {
            canvas.width = w;
            canvas.height = h;
        }
    }

    /** When layout has not given the wrap real pixels yet, use HTML width/height attrs so the EQ is drawable. */
    function ensureParametricEqCanvasMinBitmap(canvas) {
        if (!canvas) return;
        const attrW = parseInt(canvas.getAttribute('width'), 10) || 800;
        const attrH = parseInt(canvas.getAttribute('height'), 10) || 120;
        if ((canvas.width || 0) < 2 || (canvas.height || 0) < 2) {
            canvas.width = Math.max(2, attrW);
            canvas.height = Math.max(2, attrH);
        }
    }

    function applyEqDragFromClient(canvas, clientX, clientY) {
        if (!_dragState || !_dragState.band || !_dragState.canvas) return;
        const rect = canvas.getBoundingClientRect();
        const w = canvas.width || 800, h = canvas.height || 120;
        const scaleX = w / rect.width, scaleY = h / rect.height;
        const mx = (clientX - rect.left) * scaleX, my = (clientY - rect.top) * scaleY;
        const freq = Math.max(FREQ_MIN, Math.min(FREQ_MAX, xToFreq(mx, w)));
        const gain = Math.max(GAIN_MIN, Math.min(GAIN_MAX, yToGain(my, h)));
        _dragState.band.filter.frequency.value = freq;
        _dragState.band.filter.gain.value = Math.round(gain * 10) / 10;
        const cap = _dragState.band.id.charAt(0).toUpperCase() + _dragState.band.id.slice(1);
        if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
            prefs.setItem('eq' + cap, String(Math.round(gain)));
        }
        if (_enginePlaybackActive && typeof window !== 'undefined' && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
            const t = Date.now();
            if (t - _eqDragEngineSyncTs >= 40) {
                _eqDragEngineSyncTs = t;
                window.syncEnginePlaybackDspFromPrefs();
            }
        }
        const id = _dragState.band.id;
        if (id === 'low') {
            const el = document.getElementById('npEqLow');
            const lab = document.getElementById('npEqLowVal');
            if (el) el.value = Math.round(gain);
            if (lab) lab.textContent = Math.round(gain) + ' dB';
        } else if (id === 'mid') {
            const el = document.getElementById('npEqMid');
            const lab = document.getElementById('npEqMidVal');
            if (el) el.value = Math.round(gain);
            if (lab) lab.textContent = Math.round(gain) + ' dB';
        } else if (id === 'high') {
            const el = document.getElementById('npEqHigh');
            const lab = document.getElementById('npEqHighVal');
            if (el) el.value = Math.round(gain);
            if (lab) lab.textContent = Math.round(gain) + ' dB';
        }
    }

    function onWindowEqPointerMove(e) {
        if (!_dragState || !_dragState.band || !_dragState.canvas) return;
        if (e.pointerId !== _dragState.pointerId) return;
        ensureAudioGraph();
        applyEqDragFromClient(_dragState.canvas, e.clientX, e.clientY);
    }

    function onWindowEqPointerEnd(e) {
        if (!_dragState || !_dragState.canvas) return;
        if (e.pointerId !== _dragState.pointerId) return;
        window.removeEventListener('pointermove', onWindowEqPointerMove, true);
        window.removeEventListener('pointerup', onWindowEqPointerEnd, true);
        window.removeEventListener('pointercancel', onWindowEqPointerEnd, true);
        const c = _dragState.canvas;
        const pid = _dragState.pointerId;
        if (_dragState.band && typeof setEqBand === 'function') {
            setEqBand(_dragState.band.id, String(_dragState.band.filter.gain.value));
        }
        _eqDragEngineSyncTs = 0;
        _dragState = null;
        try {
            if (c && typeof pid === 'number' && typeof c.hasPointerCapture === 'function' && c.hasPointerCapture(pid)) {
                c.releasePointerCapture(pid);
            }
        } catch (_) {}
    }

    function bindCanvasDrag(canvas) {
        if (!canvas) return;
        canvas.addEventListener('pointerdown', (e) => {
            if (e.button !== 0) return;
            ensureAudioGraph();
            const rect = canvas.getBoundingClientRect();
            const cw = canvas.width || 800, ch = canvas.height || 120;
            const scaleX = cw / rect.width, scaleY = ch / rect.height;
            const mx = (e.clientX - rect.left) * scaleX, my = (e.clientY - rect.top) * scaleY;
            for (const band of bands) {
                if (!band.filter) continue;
                const bx = freqToX(band.filter.frequency.value, cw);
                const by = gainToY(band.filter.gain.value, ch);
                if (Math.hypot(mx - bx, my - by) < 14) {
                    _dragState = {band, canvas, pointerId: e.pointerId};
                    e.preventDefault();
                    try {
                        canvas.setPointerCapture(e.pointerId);
                    } catch (_) {}
                    window.addEventListener('pointermove', onWindowEqPointerMove, true);
                    window.addEventListener('pointerup', onWindowEqPointerEnd, true);
                    window.addEventListener('pointercancel', onWindowEqPointerEnd, true);
                    return;
                }
            }
        });
    }

    if (npCanvas) bindCanvasDrag(npCanvas);
    if (aeCanvas) bindCanvasDrag(aeCanvas);

    function setupEqCanvasResizeObserver(canvas) {
        if (!canvas || typeof ResizeObserver === 'undefined') return;
        const wrap = canvas.parentElement;
        if (!wrap) return;
        let eqResizeRaf = null;
        const ro = new ResizeObserver(() => {
            if (eqResizeRaf != null) return;
            eqResizeRaf = requestAnimationFrame(() => {
                eqResizeRaf = null;
                primeCanvasSize(canvas);
                scheduleParametricEqFrame();
            });
        });
        ro.observe(wrap);
    }
    setupEqCanvasResizeObserver(npCanvas);
    setupEqCanvasResizeObserver(aeCanvas);

    const eqSection = document.getElementById('npEqSection');
    if (eqSection) {
        const observer = new MutationObserver(() => {
            if (eqSection.classList.contains('visible')) {
                setTimeout(() => {
                    if (typeof window.applyNpEqCanvasHeightFromPrefs === 'function') window.applyNpEqCanvasHeightFromPrefs();
                    primeCanvasSize(npCanvas);
                    scheduleParametricEqFrame();
                }, 50);
            }
        });
        observer.observe(eqSection, {attributes: true, attributeFilter: ['class']});
    }

    const aeTab = document.getElementById('tabAudioEngine');
    if (aeTab) {
        const observer = new MutationObserver(() => {
            if (aeTab.classList.contains('active')) {
                setTimeout(() => {
                    if (typeof window.applyAeEqCanvasHeightFromPrefs === 'function') window.applyAeEqCanvasHeightFromPrefs();
                    primeCanvasSize(aeCanvas);
                    scheduleParametricEqFrame();
                }, 50);
            }
        });
        observer.observe(aeTab, {attributes: true, attributeFilter: ['class']});
    }

    if (eqSection && eqSection.classList.contains('visible')) {
        setTimeout(() => {
            primeCanvasSize(npCanvas);
            scheduleParametricEqFrame();
        }, 50);
    }
    if (aeTab && aeTab.classList.contains('active')) {
        setTimeout(() => {
            if (typeof window.applyAeEqCanvasHeightFromPrefs === 'function') window.applyAeEqCanvasHeightFromPrefs();
            primeCanvasSize(aeCanvas);
            scheduleParametricEqFrame();
        }, 50);
    }

    if (typeof window !== 'undefined') {
        window.scheduleParametricEqFrame = scheduleParametricEqFrame;
        window.cancelParametricEqRaf = cancelParametricEqRaf;
    }
})();

window.isAudioPlaying = isAudioPlaying;
window.updateFavBtn = updateFavBtn;
window.updateNoteBtn = updateNoteBtn;
window._decodePeaksViaWorker = decodePeaksViaWorker;
window._fetchWaveformPeaksFromAudioEngine = fetchWaveformPreviewFromEngine;
window._storeWaveformPeaksInCache = storeWaveformPeaksInCache;

/**
 * Compile/load `audio-decode-worker.js` after first paint — avoids paying V8 worker script
 * compile on the first play click. Also scheduled from idle + ~650ms post-splash (`app.js`).
 */
function preloadAudioDecodeWorker() {
    try {
        getAudioDecodeWorker();
    } catch (_) {
        /* ignore */
    }
}
window.preloadAudioDecodeWorker = preloadAudioDecodeWorker;

(function schedulePrewarmAudioDecodeWorker() {
    function run() {
        preloadAudioDecodeWorker();
    }
    if (typeof window === 'undefined') return;
    if (typeof requestIdleCallback === 'function') {
        requestIdleCallback(run, { timeout: 5000 });
    } else if (typeof globalThis !== 'undefined' && typeof globalThis.setTimeout === 'function') {
        globalThis.setTimeout(run, 2500);
    }
})();

/** Now-playing + expanded-row waveforms: pointerdown seeks (click delegation can miss in some WebViews; canvas is pointer-events:none). */
(function initWaveformPointerSeek() {
    function onPointerDown(e) {
        if (e.button !== 0) return;
        const t = e.target;
        if (!t || (t.closest && t.closest('button, input, select, textarea'))) return;
        const meta = typeof t.closest === 'function' ? t.closest('#metaWaveformBox') : null;
        if (meta && typeof seekMetaWaveform === 'function') {
            e.preventDefault();
            seekMetaWaveform(e);
            return;
        }
        const np = typeof t.closest === 'function' ? t.closest('#npWaveform') : null;
        if (np && typeof seekAudio === 'function') {
            e.preventDefault();
            seekAudio(e);
        }
    }
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('pointerdown', onPointerDown, true);
})();

/** `ui-idle.js` — sync playhead / spectrum / parametric EQ rAF when hybrid idle toggles (minimize, other Space, unfocused, hidden tab). */
(function wireUiIdleHeavyCpuEvents() {
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        const idle = e.detail && e.detail.idle;
        if (idle) {
            if (_playbackRafId) {
                cancelAnimationFrame(_playbackRafId);
                _playbackRafId = null;
            }
            stopEnginePlaybackFftRaf();
            if (typeof window.cancelParametricEqRaf === 'function') window.cancelParametricEqRaf();
            return;
        }
        if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
        if (typeof isAudioPlaying === 'function' && isAudioPlaying() && !_playbackRafId) {
            _playbackRafId = requestAnimationFrame(_playbackRafLoop);
        }
        if (typeof ensureEnginePlaybackFftRaf === 'function') ensureEnginePlaybackFftRaf();
        if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
    });
})();
