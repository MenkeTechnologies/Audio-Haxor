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
const _AUDIO_DECODE_TIMEOUT_MS = 30_000;

function _audioDecodeWorkerOnMessage(ev) {
    const d = ev.data;
    if (!d || typeof d.id !== 'number') return;
    const pending = _audioDecodeWorkerPending.get(d.id);
    if (!pending) return;
    _audioDecodeWorkerPending.delete(d.id);
    clearTimeout(pending.timer);
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
                clearTimeout(p.timer);
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
        const timer = setTimeout(() => {
            _audioDecodeWorkerPending.delete(id);
            reject(new Error('audio decode worker timeout'));
        }, _AUDIO_DECODE_TIMEOUT_MS);
        _audioDecodeWorkerPending.set(id, { resolve, reject, timer });
        try {
            w.postMessage({ ...payload, id }, transfer || []);
        } catch (e) {
            clearTimeout(timer);
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

/** Coalesce concurrent `waveform_preview` calls (same path + clamped width) — video row hits NP + expanded UI together. */
const _wfEngineInflight = new Map();

async function fetchWaveformPreviewFromEngine(absPath, widthPx) {
    const invoke = audioEngineInvokeMetaVisuals();
    if (!invoke) return null;
    const w = Math.max(32, Math.min(8192, Math.floor(Number(widthPx)) || 800));
    const key = absPath + '\0' + w;
    const inflight = _wfEngineInflight.get(key);
    if (inflight) {
        return inflight;
    }
    const task = (async () => {
        try {
            const wfRes = await invoke({ cmd: 'waveform_preview', path: absPath, width_px: w });
            if (!wfRes || wfRes.ok !== true || !wfRes.peaks) return null;
            return wfRes.peaks;
        } finally {
            _wfEngineInflight.delete(key);
        }
    })();
    _wfEngineInflight.set(key, task);
    return task;
}

/** Shared with `file-browser.js` + `video.js`: evict, immediate SQLite row upsert, debounced full snapshot. */
function storeWaveformPeaksInCache(filePath, peaks) {
    if (typeof _waveformCache === 'undefined' || !peaks) return;
    _waveformCache[filePath] = peaks;
    _evictCache(_waveformCache);
    if (typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.upsertWaveformCacheEntry === 'function') {
        void window.vstUpdater.upsertWaveformCacheEntry(filePath, peaks).catch(() => {});
    }
    _debounceWfSave();
    if (typeof notifyWaveformCacheUpdatedForTray === 'function') {
        notifyWaveformCacheUpdatedForTray(filePath);
    }
}

function storeSpectrogramDataInCache(filePath, sgData) {
    if (typeof _spectrogramCache === 'undefined' || !sgData) return;
    _spectrogramCache[filePath] = sgData;
    _evictCache(_spectrogramCache);
    if (typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.upsertSpectrogramCacheEntry === 'function') {
        void window.vstUpdater.upsertSpectrogramCacheEntry(filePath, sgData).catch(() => {});
    }
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
/** Lazily created `createMediaElementSource(#videoPlayerEl)` — MP4/MKV… use `<video>` audio, not AudioEngine. */
let _videoMediaElementSource = null;
let _eqLow = null;
let _eqMid = null;
let _eqHigh = null;
let _gainNode = null;
let _panNode = null;
let _analyser = null;
let _monoMode = false;
let _abLoop = null; // { start, end } in seconds, or null

/** Per-sample loop regions, keyed by absolute host path.
 *  `{ [path]: { enabled: bool, startFrac: 0-1, endFrac: 0-1 } }`.
 *  Persisted to `prefs('sampleLoopRegions')`; applied to `_abLoop` when that path plays. */
let _sampleLoopRegions = {};

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

if (typeof window !== 'undefined') {
    window.setEnginePlaybackActive = setEnginePlaybackActive;
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
    if (
        typeof videoPlayerPath !== 'undefined' &&
        videoPlayerPath &&
        typeof audioPlayerPath !== 'undefined' &&
        audioPlayerPath === videoPlayerPath
    ) {
        const vid = document.getElementById('videoPlayerEl');
        if (vid && !vid.paused && !vid.ended) return true;
    }
    if (audioReverseMode && _bufPlaying) return true;
    return typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused;
}

/**
 * `playback_status` poll (~250 ms with AudioEngine library playback) sets `_enginePlaybackPaused`. After `playback_pause` IPC, apply the
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
 * last bins) independently of global play/pause; per-graph freeze (`graphFreezeMap` / `isGraphFrozen`) stops updates for that canvas only.
 */
function engineSpectrumLive() {
    const minBins =
        typeof window !== 'undefined' && typeof window.ENGINE_PLAYBACK_SPECTRUM_MIN_BINS === 'number'
            ? window.ENGINE_PLAYBACK_SPECTRUM_MIN_BINS
            : 64;
    if (typeof window === 'undefined' || !window._engineSpectrumU8 || window._engineSpectrumU8.length < minBins) {
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

/** At least one parametric EQ canvas (np / ae) should animate its spectrum overlay — honors per-graph freeze. */
function parametricEqNeedsLiveSpectrumAnimation() {
    if (typeof window.isGraphFrozen !== 'function') return true;
    const eqSec = typeof document !== 'undefined' ? document.getElementById('npEqSection') : null;
    const npVis = eqSec && eqSec.classList.contains('visible');
    const aeTab = typeof document !== 'undefined' ? document.getElementById('tabAudioEngine') : null;
    const aeVis = aeTab && aeTab.classList.contains('active');
    const eng = engineSpectrumLive();
    const playing = isAudioPlaying();
    if (!eng && !playing) return false;
    if (npVis && !window.isGraphFrozen('np:eq')) return true;
    if (aeVis && !window.isGraphFrozen('ae:eq')) return true;
    return false;
}

function enginePlaybackStatusInvokePayload() {
    if (typeof window !== 'undefined' && typeof window.buildEnginePlaybackStatusRequest === 'function') {
        return window.buildEnginePlaybackStatusRequest();
    }
    return {cmd: 'playback_status'};
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
    try {
        if (_videoMediaElementSource) {
            _videoMediaElementSource.disconnect();
        }
    } catch (_) {
    }
}

function connectMediaToEq() {
    ensureAudioGraph();
    disconnectMediaFromEq();
    _sourceNode.connect(_eqLow);
}

/**
 * Route container video (`<video>`) audio through the same EQ / NP FFT graph as `<audio>`.
 * Without this, HTML5 video playback never touches `_analyser` — the spectrum stays flat.
 */
function connectVideoHtml5AudioThroughWebAudio(vid) {
    if (!vid || vid.tagName !== 'VIDEO') return;
    ensureAudioGraph();
    if (!_videoMediaElementSource) {
        try {
            _videoMediaElementSource = _playbackCtx.createMediaElementSource(vid);
        } catch (_) {
            return;
        }
    }
    disconnectMediaFromEq();
    _videoMediaElementSource.connect(_eqLow);
    if (_gainNode && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const v = parseInt(prefs.getItem('audioVolume') || '100', 10);
        const vol = Math.max(0, Math.min(1, v / 100));
        const pre = parseFloat(document.getElementById('npGainSlider')?.value || '1');
        _gainNode.gain.value = vol * (Number.isFinite(pre) ? pre : 1);
    }
    if (
        _playbackCtx.state === 'suspended' &&
        typeof isAudioPlaying === 'function' &&
        isAudioPlaying()
    ) {
        void _playbackCtx.resume();
    }
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
        if (canAutoplayAdvanceTrack()) {
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
    if (typeof window.scheduleParametricEqFrame === 'function') window.scheduleParametricEqFrame();
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
    const aeMono = document.getElementById('aePlaybackMono');
    if (aeMono) aeMono.classList.toggle('active', _monoMode);
    prefs.setItem('audioMono', _monoMode ? 'on' : 'off');
    if (_monoMode) {
        setPan(0);
        const slider = document.getElementById('npPanSlider');
        if (slider) {
            slider.value = 0;
            slider.disabled = true;
        }
        const aePan = document.getElementById('aePanSlider');
        if (aePan) aePan.disabled = true;
    } else {
        const slider = document.getElementById('npPanSlider');
        if (slider) slider.disabled = false;
        const aePan = document.getElementById('aePanSlider');
        if (aePan) aePan.disabled = false;
    }
    /* Always push `audioMono` to AudioEngine — `setPan` only syncs when `_enginePlaybackActive`, but the
     * native stream can be live while that flag is false (reconnect paths), and mono must still apply. */
    if (typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        window.syncEnginePlaybackDspFromPrefs();
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
    prefs.setItem('audioMono', 'off');
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
    const aeMono = document.getElementById('aePlaybackMono');
    if (aeMono) aeMono.classList.remove('active');
    const aePanEl = document.getElementById('aePanSlider');
    if (aePanEl) aePanEl.disabled = false;
    document.getElementById('npEqLowVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npEqMidVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npEqHighVal').textContent = catalogFmt('ui.audio.eq_val_db');
    document.getElementById('npGainVal').textContent = catalogFmt('ui.audio.eq_gain_pct');
    document.getElementById('npPanVal').textContent = catalogFmt('ui.audio.pan_center');
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        window.syncEnginePlaybackDspFromPrefs();
    }
    showToast(toastFmt('toast.eq_reset'));
}

// A-B loop — use effective path/time/duration so AudioEngine + reverse-buffer paths work (HTML `<audio>` is often idle).
function _abLoopPlaybackPath() {
    if (audioPlayerPath) return audioPlayerPath;
    if (
        typeof window !== 'undefined' &&
        typeof window._enginePlaybackResumePath === 'string' &&
        window._enginePlaybackResumePath.length > 0
    ) {
        return window._enginePlaybackResumePath;
    }
    return null;
}

function _abLoopEffectiveDurationSec() {
    if (_enginePlaybackActive && typeof enginePlaybackDurationSec === 'function') {
        const d = enginePlaybackDurationSec();
        if (Number.isFinite(d) && d > 0) return d;
    }
    if (audioReverseMode && _reversedBuf) {
        const d = _reversedBuf.duration;
        if (Number.isFinite(d) && d > 0) return d;
    }
    const ad = audioPlayer.duration;
    if (Number.isFinite(ad) && ad > 0) return ad;
    const p = _abLoopPlaybackPath();
    if (p && typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined') {
        const s = findByPath(allAudioSamples, p);
        if (s && typeof s.duration === 'number' && s.duration > 0) return s.duration;
    }
    return 0;
}

function _abLoopEffectiveCurrentTimeSec() {
    if (_enginePlaybackActive && typeof window !== 'undefined' && typeof window._enginePlaybackPosSec === 'number') {
        const basePos = window._enginePlaybackPosSec;
        const anchor =
            typeof window._enginePlaybackPosAnchorMs === 'number'
                ? window._enginePlaybackPosAnchorMs
                : performance.now();
        const paused = window._enginePlaybackPaused === true;
        let speed = 1;
        if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
            const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
            if (Number.isFinite(raw)) speed = Math.max(0.25, Math.min(4, raw));
        }
        const elapsedSinceAnchor = paused ? 0 : (performance.now() - anchor) / 1000;
        let cur = basePos + elapsedSinceAnchor * speed;
        const dur = _abLoopEffectiveDurationSec();
        if (Number.isFinite(dur) && dur > 0 && cur > dur) cur = dur;
        return Math.max(0, cur);
    }
    if (audioReverseMode && _reversedBuf && _bufPlaying && _playbackCtx) {
        const dur = _reversedBuf.duration;
        const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
        const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
        return Math.max(0, dur - posInRev);
    }
    const ct = audioPlayer.currentTime;
    return Number.isFinite(ct) ? ct : 0;
}

function setAbLoopStart() {
    const path = _abLoopPlaybackPath();
    const dur = _abLoopEffectiveDurationSec();
    if (!path || !Number.isFinite(dur) || dur <= 0) return;
    const t = _abLoopEffectiveCurrentTimeSec();
    if (!_abLoop) _abLoop = {start: t, end: dur};
    else _abLoop.start = Math.min(t, _abLoop.end - 0.05); // keep start < end
    updateAbLoopUI();
    showToast(toastFmt('toast.ab_point_a', {time: formatTime(_abLoop.start)}));
}

function setAbLoopEnd() {
    const path = _abLoopPlaybackPath();
    const dur = _abLoopEffectiveDurationSec();
    if (!path || !Number.isFinite(dur) || dur <= 0) return;
    const t = _abLoopEffectiveCurrentTimeSec();
    if (!_abLoop) _abLoop = {start: 0, end: t};
    else _abLoop.end = Math.max(t, _abLoop.start + 0.05); // keep end > start
    updateAbLoopUI();
    showToast(toastFmt('toast.ab_point_b', {time: formatTime(_abLoop.end)}));
}

function clearAbLoop() {
    _abLoop = null;
    updateAbLoopUI();
}

/**
 * Keyboard / palette `toggleABLoop`: set A at playhead → set B at playhead → clear.
 * (`_abLoop` lives in `audio.js` scope only; `shortcuts.js` used `typeof _abLoop`, which never saw the real loop.)
 */
function toggleAbLoopShortcut() {
    const path = _abLoopPlaybackPath();
    const dur = _abLoopEffectiveDurationSec();
    if (!path || !Number.isFinite(dur) || dur <= 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.sample_loop_no_target'), 2500);
        }
        return;
    }
    if (!_abLoop) {
        setAbLoopStart();
        return;
    }
    const bIsStillEof = Math.abs(_abLoop.end - dur) < 0.05;
    if (bIsStillEof) {
        setAbLoopEnd();
        return;
    }
    clearAbLoop();
}

function updateAbLoopUI() {
    // Keep the AudioEngine `playback_status` JS poll running whenever a loop is active
    // (even while the window is idle) so the A-B wrap-back seek in `updatePlaybackTime`
    // still fires when hidden/minimized. The poll scheduler re-checks `isAbLoopActive()`
    // inside `shouldDeferPlaybackPollToHostWatchdog`.
    if (typeof window !== 'undefined' && typeof window.syncEnginePlaybackPollForUiIdle === 'function') {
        try { window.syncEnginePlaybackPollForUiIdle(); } catch {}
    }
    // Push the region fractions into the tray popover emit so the tray progress bar repaints
    // its brace overlay on every toggle/drag. The sig includes the region so the debounced
    // `_traySyncSig` gate doesn't swallow the update.
    if (typeof syncTrayNowPlayingFromPlayback === 'function') {
        try { syncTrayNowPlayingFromPlayback(); } catch {}
    }
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
    const dur = _abLoopEffectiveDurationSec() || 1;
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

// ── Per-sample loop region (expanded-row waveform braces) ──
/** Load persisted per-sample loop regions from prefs. Called once at startup. */
function loadSampleLoopRegions() {
    try {
        const obj = typeof prefs !== 'undefined' && typeof prefs.getObject === 'function'
            ? prefs.getObject('sampleLoopRegions', {})
            : {};
        _sampleLoopRegions = (obj && typeof obj === 'object') ? obj : {};
    } catch {
        _sampleLoopRegions = {};
    }
}

function saveSampleLoopRegions() {
    try {
        if (typeof prefs !== 'undefined' && typeof prefs.setObject === 'function') {
            prefs.setObject('sampleLoopRegions', _sampleLoopRegions);
        }
    } catch {}
}

/** Return a normalized region for `path`, falling back to a default inner-half selection. */
function getSampleLoopRegion(path) {
    const r = _sampleLoopRegions[path];
    if (r && typeof r === 'object') {
        let s = typeof r.startFrac === 'number' ? r.startFrac : 0.25;
        let e = typeof r.endFrac === 'number' ? r.endFrac : 0.75;
        s = Math.max(0, Math.min(1, s));
        e = Math.max(0, Math.min(1, e));
        if (e < s + 0.01) e = Math.min(1, s + 0.01);
        return { enabled: !!r.enabled, startFrac: s, endFrac: e };
    }
    return { enabled: false, startFrac: 0.25, endFrac: 0.75 };
}

function setSampleLoopRegion(path, region) {
    if (!path) return;
    const s = Math.max(0, Math.min(1, typeof region.startFrac === 'number' ? region.startFrac : 0.25));
    let e = Math.max(0, Math.min(1, typeof region.endFrac === 'number' ? region.endFrac : 0.75));
    if (e < s + 0.01) e = Math.min(1, s + 0.01);
    _sampleLoopRegions[path] = {
        enabled: !!region.enabled,
        startFrac: s,
        endFrac: e,
    };
    saveSampleLoopRegions();
}

/** Paint the loop-region overlay (region band + two brace handles + optional toggle) into one container. */
function _paintLoopRegionOverlay(box, region) {
    if (!box) return;
    const startEl = box.querySelector('.waveform-loop-brace-start');
    const endEl = box.querySelector('.waveform-loop-brace-end');
    const regionEl = box.querySelector('.waveform-loop-region');
    const toggleEl = box.querySelector('.waveform-loop-toggle');
    const show = region.enabled;
    const disp = show ? '' : 'none';
    if (startEl) {
        startEl.style.display = disp;
        startEl.style.left = (region.startFrac * 100) + '%';
    }
    if (endEl) {
        endEl.style.display = disp;
        endEl.style.left = (region.endFrac * 100) + '%';
    }
    if (regionEl) {
        regionEl.style.display = disp;
        regionEl.style.left = (region.startFrac * 100) + '%';
        regionEl.style.width = ((region.endFrac - region.startFrac) * 100) + '%';
    }
    if (toggleEl) toggleEl.classList.toggle('active', show);
}

/** Update all on-screen loop overlays (expanded row + now-playing player) to match persisted state for `filePath`. */
function applyMetaLoopRegionUI(filePath) {
    if (!filePath) return;
    const region = getSampleLoopRegion(filePath);
    const metaBox = document.getElementById('metaWaveformBox');
    if (metaBox && metaBox.dataset.path === filePath) {
        _paintLoopRegionOverlay(metaBox, region);
    }
    if (filePath === audioPlayerPath) {
        const npBox = document.getElementById('npWaveform');
        if (npBox) _paintLoopRegionOverlay(npBox, region);
    }
    const videoBox = document.getElementById('videoWaveformBox');
    if (videoBox && videoBox.dataset.path === filePath) {
        _paintLoopRegionOverlay(videoBox, region);
    }
}

/** Refresh the now-playing loop overlay from the current `audioPlayerPath` — used on track change. */
function refreshNpLoopRegionUI() {
    const npBox = document.getElementById('npWaveform');
    if (!npBox) return;
    if (!audioPlayerPath) {
        _paintLoopRegionOverlay(npBox, { enabled: false, startFrac: 0, endFrac: 1 });
        return;
    }
    _paintLoopRegionOverlay(npBox, getSampleLoopRegion(audioPlayerPath));
}
if (typeof window !== 'undefined') window.refreshNpLoopRegionUI = refreshNpLoopRegionUI;

/** Resolve the duration (sec) that `_abLoop` should be computed against for the active playback path. */
function _durationSecForActivePlayback() {
    if (_enginePlaybackActive && typeof enginePlaybackDurationSec === 'function') {
        const d = enginePlaybackDurationSec();
        if (Number.isFinite(d) && d > 0) return d;
    }
    if (typeof audioPlayer !== 'undefined' && audioPlayer && Number.isFinite(audioPlayer.duration) && audioPlayer.duration > 0) {
        return audioPlayer.duration;
    }
    const vd = videoHostDurationFallbackSec();
    if (vd > 0) return vd;
    return 0;
}

/** If `filePath` is the current playback, push its stored region into `_abLoop` (or clear it). */
function syncAbLoopFromSampleRegion(filePath) {
    if (!filePath || audioPlayerPath !== filePath) return;
    const region = getSampleLoopRegion(filePath);
    const dur = _durationSecForActivePlayback();
    if (region.enabled && dur > 0) {
        _abLoop = {
            start: region.startFrac * dur,
            end: region.endFrac * dur,
            _fromSampleRegion: true,
        };
    } else if (_abLoop && _abLoop._fromSampleRegion) {
        _abLoop = null;
    }
    if (typeof updateAbLoopUI === 'function') updateAbLoopUI();
}

/** Toggle the loop region on/off for the currently expanded row. */
function toggleMetaLoopRegion() {
    const box = document.getElementById('metaWaveformBox');
    if (!box) return;
    const filePath = box.dataset.path || '';
    if (!filePath) return;
    const region = getSampleLoopRegion(filePath);
    region.enabled = !region.enabled;
    setSampleLoopRegion(filePath, region);
    applyMetaLoopRegionUI(filePath);
    syncAbLoopFromSampleRegion(filePath);
}

/** Resolve the live playback position (sec) for the active path — interpolates between engine polls. */
function _getCurrentPlaybackTimeSec() {
    if (_enginePlaybackActive && typeof window !== 'undefined') {
        const basePos = typeof window._enginePlaybackPosSec === 'number' ? window._enginePlaybackPosSec : 0;
        const anchor = typeof window._enginePlaybackPosAnchorMs === 'number'
            ? window._enginePlaybackPosAnchorMs
            : performance.now();
        const paused = window._enginePlaybackPaused === true;
        let speed = 1;
        if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
            const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
            if (Number.isFinite(raw)) speed = Math.max(0.25, Math.min(4, raw));
        }
        const elapsed = paused ? 0 : (performance.now() - anchor) / 1000;
        let cur = basePos + elapsed * speed;
        const dur = typeof enginePlaybackDurationSec === 'function' ? enginePlaybackDurationSec() : 0;
        if (Number.isFinite(dur) && dur > 0 && cur > dur) cur = dur;
        return cur < 0 ? 0 : cur;
    }
    if (audioReverseMode && _reversedBuf && _bufPlaying && _playbackCtx) {
        const dur = _reversedBuf.duration || 0;
        const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
        const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
        return Math.max(0, dur - posInRev);
    }
    if (audioPlayer && Number.isFinite(audioPlayer.currentTime)) return audioPlayer.currentTime;
    return 0;
}

/** Resolve the loop-region target path: the expanded row if any, otherwise the currently playing path. */
function _sampleLoopRegionTargetPath() {
    const activeTab = document.querySelector('.tab-content.active')?.id;
    if (activeTab === 'tabVideos') {
        const vBox = document.getElementById('videoWaveformBox');
        if (vBox && vBox.dataset && vBox.dataset.path) return vBox.dataset.path;
        if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath) return videoPlayerPath;
        return '';
    }
    const box = document.getElementById('metaWaveformBox');
    if (box && box.dataset && box.dataset.path) return box.dataset.path;
    return audioPlayerPath || '';
}

/** Compute the current playhead as a 0..1 fraction of `filePath`'s duration (requires live playback on that path). */
function _playbackFracForPath(filePath) {
    if (!filePath || audioPlayerPath !== filePath) return null;
    const dur = _durationSecForActivePlayback();
    if (!Number.isFinite(dur) || dur <= 0) return null;
    const cur = _getCurrentPlaybackTimeSec();
    return Math.max(0, Math.min(1, cur / dur));
}

/** Set the sample loop region start at the live playhead (or 0 when not playing this path). Enables the region. */
function setSampleLoopRegionStartAtPlayhead() {
    const filePath = _sampleLoopRegionTargetPath();
    if (!filePath) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.sample_loop_no_target'), 2500);
        }
        return;
    }
    const MIN_GAP = 0.005;
    const region = getSampleLoopRegion(filePath);
    const frac = _playbackFracForPath(filePath);
    const newStart = frac != null ? frac : 0;
    region.startFrac = Math.min(newStart, 1 - MIN_GAP);
    if (region.endFrac <= region.startFrac + MIN_GAP) {
        region.endFrac = Math.min(1, region.startFrac + MIN_GAP);
    }
    region.enabled = true;
    setSampleLoopRegion(filePath, region);
    applyMetaLoopRegionUI(filePath);
    syncAbLoopFromSampleRegion(filePath);
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        const dur = _durationSecForActivePlayback();
        const tSec = dur > 0 ? region.startFrac * dur : 0;
        showToast(toastFmt('toast.sample_loop_start_set', { time: formatTime(tSec) }), 1500);
    }
}

/** Set the sample loop region end at the live playhead (or full-duration when not playing this path). Enables the region. */
function setSampleLoopRegionEndAtPlayhead() {
    const filePath = _sampleLoopRegionTargetPath();
    if (!filePath) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.sample_loop_no_target'), 2500);
        }
        return;
    }
    const MIN_GAP = 0.005;
    const region = getSampleLoopRegion(filePath);
    const frac = _playbackFracForPath(filePath);
    const newEnd = frac != null ? frac : 1;
    region.endFrac = Math.max(newEnd, MIN_GAP);
    if (region.endFrac <= region.startFrac + MIN_GAP) {
        region.startFrac = Math.max(0, region.endFrac - MIN_GAP);
    }
    region.enabled = true;
    setSampleLoopRegion(filePath, region);
    applyMetaLoopRegionUI(filePath);
    syncAbLoopFromSampleRegion(filePath);
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        const dur = _durationSecForActivePlayback();
        const tSec = dur > 0 ? region.endFrac * dur : 0;
        showToast(toastFmt('toast.sample_loop_end_set', { time: formatTime(tSec) }), 1500);
    }
}

if (typeof window !== 'undefined') {
    window.syncAbLoopFromSampleRegion = syncAbLoopFromSampleRegion;
    window.applyMetaLoopRegionUI = applyMetaLoopRegionUI;
    window.toggleMetaLoopRegion = toggleMetaLoopRegion;
    window.setSampleLoopRegionStartAtPlayhead = setSampleLoopRegionStartAtPlayhead;
    window.setSampleLoopRegionEndAtPlayhead = setSampleLoopRegionEndAtPlayhead;
    /* Used by `audio-engine.js` `shouldDeferPlaybackPollToHostWatchdog` to keep the
     * 33ms `playback_status` poll alive while a loop is active, so `updatePlaybackTime`'s
     * A-B wrap-back seek still fires when the window is hidden/minimized/unfocused
     * (otherwise `ui-idle.js` cancels both the rAF loop and the poll — playback runs
     * past the loop end forever until the app becomes active again). */
    window.isAbLoopActive = () => _abLoop != null;
    window.toggleAbLoopShortcut = toggleAbLoopShortcut;
}

function loadRecentlyPlayed() {
    recentlyPlayed = prefs.getObject('recentlyPlayed', []);
    loadSampleLoopRegions();
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

    {
        const rawSp = prefs.getItem('audioSpeed');
        const parsed = rawSp != null && String(rawSp).trim() !== '' ? parseFloat(String(rawSp)) : NaN;
        const sp = Number.isFinite(parsed) ? String(Math.max(0.25, Math.min(4, parsed))) : '1';
        setPlaybackSpeed(sp);
    }

    audioReverseMode = prefs.getItem('audioReverse') === 'on';
    syncReversePlaybackButtons(audioReverseMode);

    _monoMode = prefs.getItem('audioMono') === 'on';
    const monoBtn = document.getElementById('npBtnMono');
    if (monoBtn) monoBtn.classList.toggle('active', _monoMode);
    const aeMonoBtn = document.getElementById('aePlaybackMono');
    if (aeMonoBtn) aeMonoBtn.classList.toggle('active', _monoMode);

    const panSlider = document.getElementById('npPanSlider');
    const aePanSlider = document.getElementById('aePanSlider');
    const savedPan = prefs.getItem('audioPan');
    if (_monoMode) {
        if (panSlider) {
            panSlider.value = '0';
            panSlider.disabled = true;
        }
        if (aePanSlider) aePanSlider.disabled = true;
        setPan(0);
    } else if (savedPan) {
        if (panSlider) {
            panSlider.value = savedPan;
            panSlider.disabled = false;
        }
        if (aePanSlider) aePanSlider.disabled = false;
        setPan(savedPan);
    } else if (panSlider) {
        panSlider.disabled = false;
        if (aePanSlider) aePanSlider.disabled = false;
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
        if (canAutoplayAdvanceTrack()) {
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
    if (canAutoplayAdvanceTrack()) {
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
    if (typeof window.isGraphFrozen === 'function') {
        const npFftLive = !window.isGraphFrozen('np:fft');
        const eqLive = parametricEqNeedsLiveSpectrumAnimation();
        if (!npFftLive && !eqLive) return;
    }
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
    window.connectVideoHtml5AudioThroughWebAudio = connectVideoHtml5AudioThroughWebAudio;
    window.ensureEnginePlaybackFftRaf = ensureEnginePlaybackFftRaf;
    window.stopEnginePlaybackFftRaf = stopEnginePlaybackFftRaf;
    /** Kick the audio playback RAF loop if not already running (used by video player for NP bar/tray sync). */
    window.kickPlaybackRafLoop = function () {
        if (!_playbackRafId && shouldContinuePlaybackUiRaf()) {
            _playbackRafId = requestAnimationFrame(_playbackRafLoop);
        }
    };
}

/** NP / tray / waveform cursors need ~60 Hz updates while transport is loaded, even when paused. */
function shouldContinuePlaybackUiRaf() {
    if (typeof isAudioPlaying === 'function' && isAudioPlaying()) return true;
    if (_enginePlaybackActive) return true;
    if (typeof audioPlayerPath !== 'undefined' && audioPlayerPath) return true;
    return false;
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
    /* When the engine spectrum RAF loop is active it already calls `_renderNpFft` — skip duplicate ~60 Hz work. */
    const fftDelegated =
        typeof engineSpectrumLive === 'function' &&
        engineSpectrumLive() &&
        _enginePlaybackFftRafId != null;
    if (!fftDelegated) {
        _renderNpFft();
    }
    /* Web Audio path: parametric EQ spectrum only animates while playing — engine playback uses `ensureEnginePlaybackFftRaf` + `scheduleParametricEqFrame`. */
    if (
        !_enginePlaybackActive &&
        typeof window.scheduleParametricEqFrame === 'function' &&
        typeof parametricEqNeedsLiveSpectrumAnimation === 'function' &&
        parametricEqNeedsLiveSpectrumAnimation() &&
        typeof isAudioPlaying === 'function' &&
        isAudioPlaying()
    ) {
        window.scheduleParametricEqFrame();
    }
    if (shouldContinuePlaybackUiRaf()) {
        _playbackRafId = requestAnimationFrame(_playbackRafLoop);
    }
}

/** Right edge of log-frequency axis for mini FFT + parametric EQ (must match `FREQ_MAX` below). */
const NP_SPECTRUM_DISPLAY_MAX_HZ = 20000;

/**
 * Fractional index into AudioEngine `playback_status.spectrum` (DC omitted). When the buffer is shorter
 * than the Nyquist bin count, indices match `appendPlaybackSpectrumJson` max-per-subband buckets.
 */
function engineSpectrumBundledBinF(freqHz, sampleRate, fftSize, bufLen) {
    const fullBins = Math.max(64, (fftSize / 2) | 0);
    const k = (freqHz * fftSize) / sampleRate;
    if (bufLen >= fullBins) {
        return k - 1;
    }
    const denom = Math.max(1, fullBins - 1);
    const kClamped = Math.max(1, Math.min(fullBins, k));
    return ((kClamped - 1) / denom) * (bufLen - 1);
}
if (typeof window !== 'undefined') {
    window.engineSpectrumBundledBinF = engineSpectrumBundledBinF;
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
 * Bin→Hz mapping for spectrum draws uses `spectrum_sr_hz` + `spectrum_fft_size`. Those change when
 * the user switches playback-spectrum presets; we keep pins in sync with the latest engine values
 * so the curve stays aligned with the log axis and EQ band Hz positions. Switching between engine
 * and web spectrum clears pins (see `syncNpFftSpectrumAxisPins`).
 */
let _npFftEngineAxisSrHz = null;
let _npFftEngineAxisFftSize = null;
/** Web Audio analyser/context sr + fftSize; refreshed when either changes (FFT preset / graph rebuild). */
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
    const liveSr = typeof window._engineSpectrumSrHz === 'number' ? window._engineSpectrumSrHz : 44100;
    const liveFft = typeof window._engineSpectrumFftSize === 'number' ? window._engineSpectrumFftSize : 2048;
    if (
        _npFftEngineAxisSrHz == null ||
        _npFftEngineAxisFftSize == null ||
        _npFftEngineAxisSrHz !== liveSr ||
        _npFftEngineAxisFftSize !== liveFft
    ) {
        _npFftEngineAxisSrHz = liveSr;
        _npFftEngineAxisFftSize = liveFft;
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
    const ctx = _npFftCtx || canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    /* Bitmap size follows `ResizeObserver` + DPR sizing — avoid `getBoundingClientRect` (~layout) every rAF. */
    if (w < 2 || h < 2) return;
    if (typeof window.isGraphFrozen === 'function' && window.isGraphFrozen('np:fft')) return;
    let sampleRate = 44100;
    let fftSize = 2048;
    let binCount = 1024;
    if (useEngineSpectrum) {
        const axis = getPinnedEngineSpectrumAxis();
        if (!axis) return;
        binCount = window._engineSpectrumU8.length;
        if (!_npFftBuf || _npFftBuf.length < binCount) _npFftBuf = new Uint8Array(binCount);
        _npFftBuf.set(window._engineSpectrumU8.subarray(0, binCount));
        sampleRate = axis.sr;
        fftSize = axis.fft;
    } else {
        const binNeed = _analyser.frequencyBinCount;
        if (!_npFftBuf || _npFftBuf.length !== binNeed) _npFftBuf = new Uint8Array(binNeed);
        _analyser.getByteFrequencyData(_npFftBuf);
        const liveSr = _playbackCtx ? _playbackCtx.sampleRate : 44100;
        const liveFft = _analyser.fftSize;
        if (
            _npFftWebAxisSrHz == null ||
            _npFftWebAxisFftSize == null ||
            _npFftWebAxisSrHz !== liveSr ||
            _npFftWebAxisFftSize !== liveFft
        ) {
            _npFftWebAxisSrHz = liveSr;
            _npFftWebAxisFftSize = liveFft;
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
    const fMax = Math.min(sampleRate / 2, NP_SPECTRUM_DISPLAY_MAX_HZ);
    const logMin = Math.log10(fMin);
    const logMax = Math.log10(fMax);
    const specH = h - 10;

    /** AudioEngine: bundled or 1:1 bins (DC omitted). Web: `binF` matches `getByteFrequencyData` indexing. */
    function magAtFreq(freqHz) {
        const binF = useEngineSpectrum
            ? engineSpectrumBundledBinF(freqHz, sampleRate, fftSize, binCount)
            : (freqHz * fftSize) / sampleRate - 1;
        if (binCount < 1) return 0;
        if (binF <= 0) return _npFftBuf[0] / 255;
        if (binF >= binCount - 1) return _npFftBuf[binCount - 1] / 255;
        const i0 = Math.floor(binF);
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
    updatePlaybackTime();
    if (shouldContinuePlaybackUiRaf()) {
        if (!_playbackRafId) {
            _playbackRafId = requestAnimationFrame(_playbackRafLoop);
        }
        return;
    }
    if (_playbackRafId) {
        cancelAnimationFrame(_playbackRafId);
        _playbackRafId = null;
    }
});
audioPlayer.addEventListener('seeked', updatePlaybackTime);
/**
 * HTMLMediaElement `timeupdate` fires ~4×/sec even when the window is unfocused, minimized, or on
 * another Space — where the `_playbackRafLoop` idle gate (`isUiIdleHeavyCpu`) has already exited.
 * This is the only HTML5 hook that reliably keeps the menu-bar title and tray popover alive while
 * the user is looking at the menu bar instead of the main window.
 */
audioPlayer.addEventListener('timeupdate', () => {
    if (audioReverseMode && _bufPlaying) return;
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
});

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
      <span class="sim-toolbar-title" title="${escapeHtml(_audioFmt('menu.find_similar_samples'))}">&#128270; ${escapeHtml(_audioFmt('ui.audio.similar_toolbar_title', {name}))}</span>
      <div class="sim-toolbar-actions">
        <button class="sim-toolbar-btn" data-action="minimizeSimilar" title="${escapeHtml(_audioFmt('ui.tt.minimize'))}">&#9866;</button>
        <button class="sim-toolbar-btn btn-close" data-action="closeSimilar" title="${escapeHtml(_audioFmt('menu.close'))}">&#10005;</button>
      </div>
    </div>
    <div class="sim-body" id="simBody">
      <div style="text-align:center;padding:24px;">
        <div class="spinner" style="width:20px;height:20px;margin:0 auto 8px;"></div>
        <div id="similarStatusText" style="color:var(--text-muted);font-size:11px;">${escapeHtml(_audioFmt('ui.audio.similar_loading_analyzing'))}</div>
        <div id="similarStatusDetail" style="color:var(--text-dim);font-size:9px;margin-top:4px;">${escapeHtml(_audioFmt('ui.audio.similar_loading_cache_check'))}</div>
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

// ── Fingerprint cache build: stats-bar badge (`fingerprint-build-progress`) ──
let _fingerprintBuildUnlisten = null;
let _fingerprintBuildLastProgress = null;
let _fingerprintBuildRunning = false;

function _shouldUpdateFingerprintBuildBadgeUi() {
    return !(typeof isUiIdleHeavyCpu === 'function' && isUiIdleHeavyCpu());
}

function clearFingerprintBuildBadge() {
    _fingerprintBuildLastProgress = null;
    const badge = document.getElementById('bgFingerprintBadge');
    if (!badge) return;
    badge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function applyFingerprintBuildBadgePayload(pl) {
    if (!pl || typeof pl !== 'object') return;
    const badge = document.getElementById('bgFingerprintBadge');
    const phase = pl.phase;
    if (phase === 'done') {
        clearFingerprintBuildBadge();
        return;
    }
    if (phase === 'start') {
        _fingerprintBuildLastProgress = pl;
        if (!badge || !_shouldUpdateFingerprintBuildBadgeUi()) {
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            return;
        }
        const t = pl.total;
        if (t != null && Number.isFinite(t) && t > 0) {
            badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_progress', {done: 0, total: t});
        } else {
            badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_working');
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
        return;
    }
    if (phase === 'progress') {
        _fingerprintBuildLastProgress = pl;
        if (!badge || !_shouldUpdateFingerprintBuildBadgeUi()) {
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            return;
        }
        if (pl.done != null && pl.total != null && Number.isFinite(pl.total)) {
            badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_progress', {
                done: pl.done,
                total: pl.total
            });
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    }
}

async function ensureFingerprintBuildProgressListener() {
    if (_fingerprintBuildUnlisten || !window.__TAURI__?.event?.listen) return;
    _fingerprintBuildUnlisten = await window.__TAURI__.event.listen('fingerprint-build-progress', (e) => {
        applyFingerprintBuildBadgePayload(e.payload);
    });
}

/**
 * Toast after `buildFingerprintCache` / `invokeBuildFingerprintCacheWithBadge` (handles user stop).
 * @param {'builtOnly'|'withCachedTotals'} [summary]
 */
function showFingerprintCacheBuildOutcomeToast(res, summary = 'builtOnly') {
    if (!res || typeof res !== 'object') return;
    if (typeof showToast !== 'function' || typeof toastFmt !== 'function') return;
    const built = (res.built ?? 0).toLocaleString();
    const cached = (res.cached ?? 0).toLocaleString();
    if (res.stopped === true) {
        showToast(
            toastFmt('toast.fingerprint_build_stopped', {
                built,
                cached
            })
        );
        return;
    }
    if (summary === 'withCachedTotals') {
        showToast(
            toastFmt('toast.fingerprint_build_complete_n_cached', {
                built,
                cached
            })
        );
        return;
    }
    showToast(toastFmt('toast.fingerprint_build_complete_n', {n: built}));
}

/** Command palette / native menu: fingerprint cache build for the full audio library. */
async function triggerStartFingerprintCacheBuild() {
    const paths = typeof fetchAudioLibraryPathsForFingerprint === 'function'
        ? await fetchAudioLibraryPathsForFingerprint()
        : [];
    if (paths.length === 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.no_audio_samples_scan_first'), 4000, 'error');
        }
        return;
    }
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.fingerprint_building_n', {n: paths.length.toLocaleString()}), 4000);
    }
    const vu = window.vstUpdater;
    if (!vu || typeof vu.buildFingerprintCache !== 'function') return;
    try {
        const res =
            typeof invokeBuildFingerprintCacheWithBadge === 'function'
                ? await invokeBuildFingerprintCacheWithBadge(paths)
                : await vu.buildFingerprintCache(paths);
        if (typeof showFingerprintCacheBuildOutcomeToast === 'function') {
            showFingerprintCacheBuildOutcomeToast(res, 'builtOnly');
        }
    } catch (e) {
        if (window.vstUpdater?.appendLog) {
            window.vstUpdater.appendLog(`FINGERPRINT CACHE ERROR — UI: ${e.message || e}`);
        }
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.fingerprint_build_failed', {err: e.message || e}), 4000, 'error');
        }
    }
}

/** Wraps `buildFingerprintCache` with stats-bar progress and cleanup on completion or error. */
async function invokeBuildFingerprintCacheWithBadge(paths) {
    if (!window.vstUpdater || typeof window.vstUpdater.buildFingerprintCache !== 'function') {
        throw new Error('buildFingerprintCache unavailable');
    }
    await ensureFingerprintBuildProgressListener();
    _fingerprintBuildRunning = true;
    if (typeof window !== 'undefined') window.__statusBarFingerprintJob = true;
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    try {
        return await window.vstUpdater.buildFingerprintCache(paths);
    } finally {
        _fingerprintBuildRunning = false;
        if (typeof window !== 'undefined') window.__statusBarFingerprintJob = false;
        clearFingerprintBuildBadge();
    }
}

document.addEventListener('ui-idle-heavy-cpu', (ev) => {
    try {
        if (!ev.detail || ev.detail.idle !== false) return;
        if (!_fingerprintBuildRunning) return;
        const badge = document.getElementById('bgFingerprintBadge');
        if (!badge) return;
        const pl = _fingerprintBuildLastProgress;
        if (pl && pl.phase === 'progress' && pl.done != null && pl.total != null && Number.isFinite(pl.total)) {
            if (!_shouldUpdateFingerprintBuildBadgeUi()) return;
            badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_progress', {
                done: pl.done,
                total: pl.total
            });
        } else if (pl && pl.phase === 'start') {
            if (!_shouldUpdateFingerprintBuildBadgeUi()) return;
            const t = pl.total;
            if (t != null && Number.isFinite(t) && t > 0) {
                badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_progress', {done: 0, total: t});
            } else {
                badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_working');
            }
        } else if (_shouldUpdateFingerprintBuildBadgeUi()) {
            badge.textContent = formatBgJobBadgeLine('fingerprint', 'ui.stats.fingerprint_bg_working');
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    } catch {
        /* ignore */
    }
});

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

    if (typeof btnLoading === 'function') btnLoading(btn, true);
    btn.disabled = true;
    btn.innerHTML = '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn');
    resumeBtn.style.display = 'none';
    stopBtn.style.display = '';
    progressBar.classList.add('active');
    progressFill.style.width = '0%';

    const excludePaths = resume ? allAudioSamples.map(s => s.path) : null;

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
        if (!_bgAnalysisRunning && prefs.getItem('autoAnalysis') === 'on') startBackgroundAnalysis();

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

    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

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
        if (prefs.getItem('autoAnalysis') === 'on') startBackgroundAnalysis();
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

function clearAudioScanButtonSpinnerImmediate() {
    const btn = document.getElementById('btnScanAudio');
    const stopBtn = document.getElementById('btnStopAudio');
    const progressBar = document.getElementById('audioProgressBar');
    const progressFill = document.getElementById('audioProgressFill');
    if (typeof btnLoading === 'function') btnLoading(btn, false);
    if (btn) {
        btn.disabled = false;
        btn.innerHTML = catalogFmt('ui.btn.127925_scan_samples');
    }
    if (stopBtn) stopBtn.style.display = 'none';
    if (progressBar) progressBar.classList.remove('active');
    if (progressFill) {
        progressFill.style.width = '0%';
        progressFill.style.animation = '';
    }
}

async function stopAudioScan() {
    clearAudioScanButtonSpinnerImmediate();
    try {
        await window.vstUpdater.stopAudioScan();
    } catch (e) {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    }
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
        <th class="col-actions" style="width: 152px;"></th>
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

function sortAudio(key, forceAsc) {
    if (typeof forceAsc === 'boolean') {
        audioSortKey = key;
        audioSortAsc = forceAsc;
    } else if (audioSortKey === key) {
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
      <span class="table-row-actions">
      <button type="button" class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${hp}" title="${previewBtnT}">
        ${isPlaying && isAudioPlaying() ? '&#9646;&#9646;' : '&#9654;'}
      </button>
      <button type="button" class="btn-small btn-loop${isPlaying && audioLooping ? ' active' : ''}" data-action="toggleRowLoop" data-path="${hp}" title="${loopBtnT}">&#8634;</button>
      <button type="button" class="btn-small btn-folder" data-action="openAudioFolder" data-path="${hp}" title="${revealBtnT}">&#128193;</button>
      </span>
    </td>
    </tr>`;
}

async function showEngineUnplayablePreview(filePath, keepFloatingPlayerHidden = false) {
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
    if (!keepFloatingPlayerHidden) {
        np.classList.add('active');
        if (prefs.getItem('playerExpanded') === 'on') {
            np.classList.add('expanded');
            renderRecentlyPlayed();
        }
    } else {
        np.classList.remove('active');
        const pill = document.getElementById('audioRestorePill');
        if (pill) pill.classList.remove('active');
    }
    np.classList.remove('np-playing');
    const sample = findByPath(allAudioSamples, filePath);
    const displayName = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
    _clearNpVideoSourceFlags();
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
/**
 * @param {string} filePath
 * @param { { skipRecentReorder?: boolean, minimizeFloatingPlayer?: boolean } } [opts] — When **`skipRecentReorder`**, **`addToRecentlyPlayed`** updates or appends without moving the row to the top (autoplay, prev/next, and clicks on the player history list keep stable order). When **`minimizeFloatingPlayer`**, do not auto-open the floating bar if it is already hidden (restore pill); if the bar is already visible, leave it visible. Pref **`playerPaneHidden`** (`on` after **Hide player**) keeps the pane minimized across launches until **Show player** sets it off.
 */
async function previewAudio(filePath, opts) {
    const o = opts && typeof opts === 'object' ? opts : {};
    const np0 = document.getElementById('audioNowPlaying');
    /** User hid the floating player (`hidePlayer`) while a path was still loaded — keep it hidden on new previews. */
    const userHidPlayerWithPathLoaded =
        !!(np0 && !np0.classList.contains('active') && audioPlayerPath != null);
    /** Row/keyboard play: stay minimized only when the bar is not already shown. */
    const rowPlayKeepBarHidden =
        o.minimizeFloatingPlayer === true && np0 && !np0.classList.contains('active');
    /** Persisted from `hidePlayer` / `showPlayer` (`playerPaneHidden` pref). */
    const playerPaneHiddenByPref =
        typeof prefs !== 'undefined' &&
        typeof prefs.getItem === 'function' &&
        prefs.getItem('playerPaneHidden') === 'on';
    const keepFloatingPlayerHidden =
        userHidPlayerWithPathLoaded || rowPlayKeepBarHidden || playerPaneHiddenByPref;

    const ext = filePath.split('.').pop().toLowerCase();
    if (isEngineUnplayablePath(filePath)) {
        const extU = (filePath.split('.').pop() || '').toLowerCase();
        const extDisplay = extU ? extU.toUpperCase() : '?';
        const unplayableErr =
            typeof toastFmt === 'function'
                ? toastFmt('toast.preview_not_supported_format', { ext: extDisplay })
                : undefined;
        if (await tryPreviewAutoplayNextOnFailureAsync(filePath, unplayableErr, o)) return;
        await showEngineUnplayablePreview(filePath, keepFloatingPlayerHidden);
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
            try {
                await audioPlayer.play();
            } catch (e) {
                if (await tryPreviewAutoplayNextOnFailureAsync(filePath, e, o)) return;
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }
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
            /* Stop any active video playback that is driving the engine — keep the frozen frame in the panel. */
            if (typeof stopVideoPlayback === 'function') stopVideoPlayback({ keepVideoFrame: true });
            stopReverseBufferPlayback();
            _decodedBuf = null;
            _reversedBuf = null;
            _decodedBufPath = null;
            _pausedOffsetInRev = 0;
            /* `enginePlaybackStart` runs `playback_status` immediately — path + engine flag must be set first or tray / time stay idle / HTML5. */
            audioPlayerPath = filePath;
            audioPlayer.loop = false;
            if (typeof window !== 'undefined') {
                window._enginePlaybackResumePath = filePath;
            }
            try {
                await window.enginePlaybackStart(filePath);
            } catch (e) {
                setEnginePlaybackActive(false);
                if (typeof window.stopEnginePlaybackPoll === 'function') window.stopEnginePlaybackPoll();
                audioPlayerPath = null;
                if (typeof window !== 'undefined') {
                    window._enginePlaybackResumePath = '';
                }
                throw e;
            }
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
            if (typeof window.kickPlaybackRafLoop === 'function') window.kickPlaybackRafLoop();
        } else {
            if (typeof stopVideoPlayback === 'function') stopVideoPlayback({ keepVideoFrame: true });
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

        // Show now-playing bar unless the user hid it while another track was loaded (`hidePlayer`).
        const np = document.getElementById('audioNowPlaying');
        if (!keepFloatingPlayerHidden) {
            np.classList.add('active');
            if (prefs.getItem('playerExpanded') === 'on') {
                np.classList.add('expanded');
                renderRecentlyPlayed();
            }
        } else {
            np.classList.remove('active');
            const pill = document.getElementById('audioRestorePill');
            if (pill && isAudioPlaying()) pill.classList.add('active');
        }
        const sample = findByPath(allAudioSamples, filePath);
        const displayName = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
        _clearNpVideoSourceFlags();
        const npNameEl2 = document.getElementById('npName');
        if (npNameEl2) npNameEl2.textContent = displayName;

        // Track recently played
        addToRecentlyPlayed(filePath, sample, o);

        updatePlayBtnStates();
        updateNowPlayingBtn();
        updateFavBtn();
        updateNoteBtn();
        updateMetaLine();
        // Deferred one task — layout for the waveform flex child is often 0×0 until after paint (WKWebView).
        scheduleNowPlayingWaveform(filePath);
        // Apply expanded-row loop braces to live `_abLoop` (engine path does this in `enginePlaybackStart`).
        syncAbLoopFromSampleRegion(filePath);
        refreshNpLoopRegionUI();
        if (typeof window.syncAeTransportFromPlayback === 'function') window.syncAeTransportFromPlayback();
        /* Fire-and-forget BPM / Key / LUFS analysis for the newly-loaded track. Runs on every
         * play path (row click, tray prev/next, menu bar, keyboard shortcut, autoplay-next EOF,
         * history resume) so the analysis pipeline isn't gated behind expanding the row. Results
         * are cached in memory + persisted to SQLite via `persistAnalysisRowToDb`, so subsequent
         * expansion / display is an instant read. The `*ForMeta` helpers no-op their DOM updates
         * when the metadata row isn't open (all DOM gets are null-guarded as of this pass). */
        void ensureAudioAnalysisForPath(filePath);
    } catch (err) {
        setEnginePlaybackActive(false);
        if (typeof window.stopEnginePlaybackPoll === 'function') window.stopEnginePlaybackPoll();
        if (await tryPreviewAutoplayNextOnFailureAsync(filePath, err, o)) return;
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
        // Sync muted <video> element when toggling via NP bar
        if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath) {
            const vid = document.getElementById('videoPlayerEl');
            if (vid) { playing ? vid.pause() : vid.play().catch(() => {}); }
            if (typeof _updateVideoPlayBtn === 'function') _updateVideoPlayBtn();
            if (playing) {
                if (typeof _videoRafId !== 'undefined' && _videoRafId) { cancelAnimationFrame(_videoRafId); _videoRafId = null; }
            } else {
                if (typeof _startVideoRaf === 'function') _startVideoRaf();
            }
        }
        /* `previewAudio` schedules the waveform on load / same-track replay; play/pause toggles only
         * hit this path — resuming must redraw (or fill cache) so the floating player waveform is not blank. */
        if (!playing && typeof audioPlayerPath === 'string' && audioPlayerPath !== '') {
            scheduleNowPlayingWaveform(audioPlayerPath);
        }
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
    if (
        typeof videoPlayerPath !== 'undefined' &&
        videoPlayerPath &&
        typeof audioPlayerPath !== 'undefined' &&
        audioPlayerPath === videoPlayerPath
    ) {
        const vid = document.getElementById('videoPlayerEl');
        if (vid) vid.loop = audioLooping;
    }
    updateLoopBtnStates();
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackLoop === 'function') {
        window.syncEnginePlaybackLoop(audioLooping);
    }
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

function toggleRowLoop(filePath, event) {
    event.stopPropagation();
    // If this sample isn't playing yet, start it with loop on
    if (audioPlayerPath !== filePath) {
        audioLooping = true;
        audioPlayer.loop = true;
        prefs.setItem('audioLoop', 'on');
        document.getElementById('npBtnLoop').classList.add('active');
        previewAudio(filePath, { minimizeFloatingPlayer: true });
        updateLoopBtnStates();
        return;
    }
    // Toggle loop for the currently playing sample
    toggleAudioLoop();
}

/**
 * Videos tab row loop — same pref / engine / `<video>.loop` wiring as `toggleRowLoop`, transport is `previewVideo`.
 */
function toggleVideoRowLoop(filePath, event) {
    if (event && typeof event.stopPropagation === 'function') event.stopPropagation();
    if (typeof previewVideo !== 'function') return;
    const v = typeof videoPlayerPath !== 'undefined' && videoPlayerPath === filePath;
    const a = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === filePath;
    if (!v || !a) {
        audioLooping = true;
        audioPlayer.loop = true;
        prefs.setItem('audioLoop', 'on');
        const loopBtn = document.getElementById('npBtnLoop');
        if (loopBtn) loopBtn.classList.add('active');
        void previewVideo(filePath, { minimizeFloatingPlayer: true });
        updateLoopBtnStates();
        updatePlayBtnStates();
        return;
    }
    toggleAudioLoop();
}

function updateLoopBtnStates() {
    if (!audioPlayerPath) return;
    const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(audioPlayerPath)}"]`);
    if (row) {
        const btn = row.querySelector('.btn-loop');
        if (btn) btn.classList.toggle('active', audioLooping);
    }
    if (
        typeof videoPlayerPath !== 'undefined' &&
        videoPlayerPath &&
        videoPlayerPath === audioPlayerPath
    ) {
        const vRow = document.querySelector(`#videoTableBody tr[data-video-path="${CSS.escape(audioPlayerPath)}"]`);
        if (vRow) {
            const vb = vRow.querySelector('.btn-loop');
            if (vb) vb.classList.toggle('active', audioLooping);
        }
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
    if (np) {
        np.classList.remove('active');
        np.classList.remove('expanded');
        np.classList.remove('np-playing');
    }
    const npProgress = document.getElementById('npProgress');
    if (npProgress) npProgress.style.width = '0%';
    const npTime = document.getElementById('npTime');
    if (npTime) npTime.textContent = catalogFmt('ui.audio.player_time_zero');
    const npCursor = document.getElementById('npCursor');
    if (npCursor) npCursor.style.display = 'none';
    const pill = document.getElementById('audioRestorePill');
    if (pill) pill.classList.remove('active');
    updatePlayBtnStates();
    updateNowPlayingBtn();
    updateFavBtn();
    updateNoteBtn();
    _traySyncSig = '';
    _traySyncKeyNoApp = '';
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

let _prevPlayingRow = null;
let _prevVideoPlayingRow = null;

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
    if (_prevVideoPlayingRow) {
        const vbtn = _prevVideoPlayingRow.querySelector('.btn-play');
        if (vbtn) {
            vbtn.classList.remove('playing');
            vbtn.innerHTML = '&#9654;';
        }
        _prevVideoPlayingRow.classList.remove('row-playing');
        const vloop = _prevVideoPlayingRow.querySelector('.btn-loop');
        if (vloop) vloop.classList.remove('active');
    }
    _prevPlayingRow = null;
    _prevVideoPlayingRow = null;
    // Set current playing row(s)
    if (audioPlayerPath) {
        const playing = isAudioPlaying();
        const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(audioPlayerPath)}"]`);
        if (row) {
            row.classList.add('row-playing');
            const btn = row.querySelector('.btn-play');
            if (btn) {
                btn.classList.toggle('playing', playing);
                btn.innerHTML = playing ? '&#9646;&#9646;' : '&#9654;';
            }
            const loop = row.querySelector('.btn-loop');
            if (loop) loop.classList.toggle('active', audioLooping);
            _prevPlayingRow = row;
        }
        if (
            typeof videoPlayerPath !== 'undefined' &&
            videoPlayerPath &&
            videoPlayerPath === audioPlayerPath
        ) {
            const vRow = document.querySelector(`#videoTableBody tr[data-video-path="${CSS.escape(audioPlayerPath)}"]`);
            if (vRow) {
                vRow.classList.add('row-playing');
                const vbtn = vRow.querySelector('.btn-play');
                if (vbtn) {
                    vbtn.classList.toggle('playing', playing);
                    vbtn.innerHTML = playing ? '&#9646;&#9646;' : '&#9654;';
                }
                const vloop = vRow.querySelector('.btn-loop');
                if (vloop) vloop.classList.toggle('active', audioLooping);
                _prevVideoPlayingRow = vRow;
            }
        }
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

/** Re-resolve now-playing transport nodes if a prior reference is detached (e.g. after modal/tab DOM churn). */
function _ensureNpPlaybackDomRefs() {
    if (!_npTimeEl || !_npTimeEl.isConnected || !_npProgressEl || !_npProgressEl.isConnected || !_npCursorEl || !_npCursorEl.isConnected) {
        _cachePlaybackEls();
    }
}

/** Cached `#videoPlayerEl` when `videoPlayerPath === audioPlayerPath` — `updatePlaybackTime` / tray sync (~60 Hz). */
let _playbackNpVideoPath = '';
let _playbackNpVideoEl = null;

function npVideoElementIfDrivingRow() {
    if (typeof videoPlayerPath === 'undefined' || !videoPlayerPath || audioPlayerPath !== videoPlayerPath) {
        _playbackNpVideoPath = '';
        _playbackNpVideoEl = null;
        return null;
    }
    if (_playbackNpVideoPath !== audioPlayerPath) {
        _playbackNpVideoPath = audioPlayerPath;
        _playbackNpVideoEl = document.getElementById('videoPlayerEl');
    } else if (!_playbackNpVideoEl || !_playbackNpVideoEl.isConnected) {
        /* If the first lookup raced before `#videoPlayerEl` existed, `_playbackNpVideoEl` stays null forever
         * unless we retry — shows up most often on the next video open (second playback). */
        _playbackNpVideoEl = document.getElementById('videoPlayerEl');
    }
    return _playbackNpVideoEl;
}

let _npMetaWaveformBox = null;
let _npMetaWaveformCachedPath = null;
/** Cached `.querySelector` targets under `#metaWaveformBox` — updated when the box or path changes. */
let _npMetaWfFill = null;
let _npMetaWfCursor = null;
let _npMetaWfTimeLabel = null;
let _npAeTransportTabEl = null;

/** `<video>` duration when the floating player is driving a video row (not in `allAudioSamples`). */
function videoHostDurationFallbackSec() {
    if (typeof videoPlayerPath === 'undefined' || !videoPlayerPath || audioPlayerPath !== videoPlayerPath) return 0;
    const vid = npVideoElementIfDrivingRow();
    if (!vid) return 0;
    const d = vid.duration;
    if (Number.isFinite(d) && d > 0) return d;
    try {
        if (vid.seekable && vid.seekable.length > 0) {
            const end = vid.seekable.end(vid.seekable.length - 1);
            if (Number.isFinite(end) && end > 0) return end;
        }
    } catch (_) {}
    return 0;
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
    if (dur <= 0) {
        const vd = videoHostDurationFallbackSec();
        if (vd > 0) dur = vd;
    }
    return dur;
}

/** Dedupes `invoke('update_tray_now_playing')` — includes duration so tray updates when total length loads. */
let _traySyncSig = '';
/** Tray payload identity without CSS `appearance` — skips `getComputedStyle` on every ~60 Hz `updatePlaybackTime` tick when nothing user-visible changed. */
let _traySyncKeyNoApp = '';

/** Last path we pushed waveform peaks to the tray for — avoids re-sending ~1.6 KB of peaks on every poll tick. */
let _traySyncLastWaveformPath = '';
/** True once peaks for `_traySyncLastWaveformPath` were actually sent (i.e., they were cached at send time). */
let _traySyncLastWaveformSent = false;

/** Hook called after any `_waveformCache[path] = peaks` write. When `path` matches the currently
 *  playing track, re-run `syncTrayNowPlayingFromPlayback` so the tray popover receives the freshly
 *  decoded peaks (track change → empty-peaks placeholder first, async worker finishes → real peaks). */
function notifyWaveformCacheUpdatedForTray(filePath) {
    if (!filePath) return;
    if (filePath !== audioPlayerPath) return;
    // Invalidate the tray-sync waveform flag so the next sync re-reads `_waveformCache[sigPath]`.
    if (_traySyncLastWaveformPath === filePath) {
        _traySyncLastWaveformSent = false;
    }
    // Invalidate the debounce sig so syncTrayNowPlayingFromPlayback actually fires this call
    // (otherwise the unchanged time tuple would make it short-circuit).
    _traySyncSig = '';
    _traySyncKeyNoApp = '';
    if (typeof syncTrayNowPlayingFromPlayback === 'function') {
        try { syncTrayNowPlayingFromPlayback(); } catch {}
    }
}

/** DevTools: compact summary of `appearance` sent with `update_tray_now_playing` (CSS custom properties). */
function trayAppearanceForLog(appearance) {
    if (!appearance || typeof appearance !== 'object') {
        return { appearance_var_count: 0 };
    }
    const keys = Object.keys(appearance).filter((k) => typeof k === 'string' && k.startsWith('--'));
    const cyan = appearance['--cyan'];
    return {
        appearance_var_count: keys.length,
        appearance_keys_sample: keys.slice(0, 10),
        '--cyan': typeof cyan === 'string' ? cyan : undefined,
    };
}

/** Scheme vars for tray popover HUD — matches `SCHEME_VAR_KEYS` from `settings.js`. */
function trayAppearanceForTraySync() {
    const keys =
        typeof window !== 'undefined' && Array.isArray(window.SCHEME_VAR_KEYS) ? window.SCHEME_VAR_KEYS : null;
    if (!keys || keys.length === 0 || typeof document === 'undefined') return { appearance: null, sig: '' };
    const cs = getComputedStyle(document.documentElement);
    const appearance = {};
    const parts = [];
    for (const k of keys) {
        const v = cs.getPropertyValue(k).trim();
        parts.push(v);
        if (v) appearance[k] = v;
    }
    return { appearance: Object.keys(appearance).length ? appearance : null, sig: parts.join('|') };
}

/**
 * Library row for meta / tray — SQLite UI keeps only a page in `filteredAudioSamples`; `allAudioSamples` is often empty.
 * Fall back to play history (format/size/dir) so `#npMetaLine` and the tray match what the user hears.
 */
function resolveAudioSampleForMeta(path) {
    if (!path || typeof findByPath !== 'function') return null;
    if (typeof allAudioSamples !== 'undefined') {
        const a = findByPath(allAudioSamples, path);
        if (a) return a;
    }
    if (typeof filteredAudioSamples !== 'undefined') {
        const f = findByPath(filteredAudioSamples, path);
        if (f) return f;
    }
    if (typeof recentlyPlayed !== 'undefined' && Array.isArray(recentlyPlayed)) {
        const r = recentlyPlayed.find((x) => x && x.path === path);
        if (r) {
            const norm = path.replace(/\\/g, '/');
            const slash = norm.lastIndexOf('/');
            const directory = slash > 0 ? norm.slice(0, slash) : '';
            return {
                path,
                name: r.name,
                format: r.format,
                sizeFormatted: r.size || '',
                directory,
            };
        }
    }
    return null;
}

/** Same string as `#npMetaLine` / `updateMetaLine` — shared so tray sync does not read stale DOM or miss `resumePath`. */
function npMetaLineTextForPath(path) {
    if (!path) return '';
    if (isEngineUnplayablePath(path)) {
        const x = path.split('.').pop().toLowerCase();
        return _audioFmt('ui.audio.not_playable_in_audio_engine', {ext: x.toUpperCase()});
    }
    const sample = resolveAudioSampleForMeta(path);
    if (!sample) {
        const base = path.replace(/\\/g, '/').split('/').pop();
        return base || '';
    }
    const parts = [sample.format, sample.sizeFormatted];
    const bpmShow = sample.bpm || (typeof _bpmCache !== 'undefined' ? _bpmCache[path] : undefined);
    const keyShow = sample.key || (typeof _keyCache !== 'undefined' ? _keyCache[path] : undefined);
    const lufsShow =
        sample.lufs != null
            ? sample.lufs
            : typeof _lufsCache !== 'undefined' && _lufsCache[path] != null ? _lufsCache[path]
              : null;
    if (bpmShow) parts.push(bpmShow + ' BPM');
    if (keyShow) parts.push(keyShow);
    if (lufsShow != null) parts.push(lufsShow + ' LUFS');
    if (sample.directory) parts.push(sample.directory);
    return parts.join(' \u2022 ');
}

/**
 * Tray popover subtitle: format / BPM / key / LUFS only — the file path is a separate clickable span in `tray-popover.js`.
 */
function npTrayPopoverSubtitleMetaOnly(path) {
    if (!path) return '';
    if (isEngineUnplayablePath(path)) {
        const x = path.split('.').pop().toLowerCase();
        return _audioFmt('ui.audio.not_playable_in_audio_engine', {ext: x.toUpperCase()});
    }
    const sample = resolveAudioSampleForMeta(path);
    if (!sample) return '';
    const parts = [sample.format, sample.sizeFormatted];
    const bpmShow = sample.bpm || (typeof _bpmCache !== 'undefined' ? _bpmCache[path] : undefined);
    const keyShow = sample.key || (typeof _keyCache !== 'undefined' ? _keyCache[path] : undefined);
    const lufsShow =
        sample.lufs != null
            ? sample.lufs
            : typeof _lufsCache !== 'undefined' && _lufsCache[path] != null ? _lufsCache[path]
              : null;
    if (bpmShow) parts.push(bpmShow + ' BPM');
    if (keyShow) parts.push(keyShow);
    if (lufsShow != null) parts.push(lufsShow + ' LUFS');
    return parts.join(' \u2022 ');
}

/** Tray popover + menu line: `#npName`, else library sample basename, else path basename. */
function trayNowPlayingDisplayName() {
    const np = document.getElementById('npName');
    let track = np && typeof np.textContent === 'string' ? np.textContent.trim() : '';
    if (track) return track;
    const resumePath =
        typeof window !== 'undefined' &&
        typeof window._enginePlaybackResumePath === 'string' &&
        window._enginePlaybackResumePath.length > 0
            ? window._enginePlaybackResumePath
            : '';
    const pathForMeta = audioPlayerPath || resumePath || null;
    if (pathForMeta && typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined') {
        const s = findByPath(allAudioSamples, pathForMeta);
        if (s && typeof s.name === 'string' && s.name.trim() !== '') {
            const ext = s.format ? String(s.format).toLowerCase() : '';
            return ext ? `${s.name}.${ext}` : s.name;
        }
    }
    if (pathForMeta) {
        const base = pathForMeta.split('/').pop();
        if (base) return base;
    }
    return '';
}

/** Matches `document.documentElement` `data-theme` for `update_tray_now_playing` → `TrayNowPlayingPayload.ui_theme`. */
function traySyncUiTheme() {
    return typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'light'
        ? 'light'
        : 'dark';
}

/** Prefs `audioSpeed` for tray HUD — same clamp as `setPlaybackSpeed` (0.25..4). */
function trayPlaybackSpeedForSync() {
    let v = 1;
    if (typeof prefs !== 'undefined' && prefs.getItem) {
        const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
        if (Number.isFinite(raw)) v = raw;
    }
    return Math.max(0.25, Math.min(4, v));
}

/** Prefs `audioVolume` for tray HUD — same range as `setAudioVolume` (0..100). */
function trayVolumeForSync() {
    let v = 100;
    if (typeof prefs !== 'undefined' && prefs.getItem) {
        const raw = parseInt(prefs.getItem('audioVolume') || '100', 10);
        if (Number.isFinite(raw)) v = raw;
    }
    return Math.max(0, Math.min(100, v));
}

/** Same string as `wf:` in `keyNoApp` — no array allocation (used to short-circuit ~60 Hz tray prep). */
function trayWaveformKeyForSync(sigPath) {
    if (!sigPath) return 'n';
    const cached = _waveformCache && _waveformCache[sigPath];
    const pathChanged = _traySyncLastWaveformPath !== sigPath;
    if (Array.isArray(cached) && cached.length > 0 && (pathChanged || !_traySyncLastWaveformSent)) {
        return 'p' + cached.length;
    }
    if (pathChanged && (!Array.isArray(cached) || cached.length === 0)) {
        return 'e';
    }
    return 'n';
}

/** Build `waveform_peaks` payload + update `_traySyncLastWaveformPath` — only when an IPC emit will run. */
function materializeTrayWaveformPeaksForSync(sigPath) {
    let trayWaveformPeaks = null;
    if (sigPath) {
        const cached = _waveformCache && _waveformCache[sigPath];
        const pathChanged = _traySyncLastWaveformPath !== sigPath;
        if (Array.isArray(cached) && cached.length > 0 && (pathChanged || !_traySyncLastWaveformSent)) {
            const n = cached.length;
            const flat = new Array(n * 2);
            if (typeof cached[0] === 'object' && cached[0] !== null && 'max' in cached[0]) {
                for (let i = 0; i < n; i++) {
                    flat[i * 2] = Number(cached[i].max) || 0;
                    flat[i * 2 + 1] = Number(cached[i].min) || 0;
                }
            } else {
                for (let i = 0; i < n; i++) {
                    const v = Number(cached[i]) || 0;
                    flat[i * 2] = v;
                    flat[i * 2 + 1] = -v;
                }
            }
            trayWaveformPeaks = flat;
            _traySyncLastWaveformPath = sigPath;
            _traySyncLastWaveformSent = true;
        } else if (pathChanged && (!Array.isArray(cached) || cached.length === 0)) {
            trayWaveformPeaks = [];
            _traySyncLastWaveformPath = sigPath;
            _traySyncLastWaveformSent = false;
        }
    } else if (_traySyncLastWaveformPath) {
        _traySyncLastWaveformPath = '';
        _traySyncLastWaveformSent = false;
    }
    return trayWaveformPeaks;
}

/**
 * @param {{ cur: number, dur: number } | undefined} [playbackTimeHint] — when `updatePlaybackTime` passes this, skip
 * re-deriving position/duration (avoids duplicating transport math at ~60 Hz).
 */
function syncTrayNowPlayingFromPlayback(playbackTimeHint) {
    const inv =
        typeof window !== 'undefined' &&
        window.__TAURI__ &&
        window.__TAURI__.core &&
        typeof window.__TAURI__.core.invoke === 'function'
            ? window.__TAURI__.core.invoke
            : null;
    if (!inv) return;
    /* `!audioPlayerPath` alone misses AudioEngine sessions where `resumeEnginePlaybackAfterApply` did not
     * restore `audioPlayerPath` while transport is still active. Match engine / reverse transport. */
    const resumePath =
        typeof window !== 'undefined' &&
        typeof window._enginePlaybackResumePath === 'string' &&
        window._enginePlaybackResumePath.length > 0
            ? window._enginePlaybackResumePath
            : '';
    const idle =
        !audioPlayerPath &&
        !_enginePlaybackActive &&
        !(audioReverseMode && _reversedBuf && _bufPlaying);
    const tooltipBase = catalogFmt('tray.tooltip');
    const uiTheme = traySyncUiTheme();
    const traySp = trayPlaybackSpeedForSync();
    const trayVol = trayVolumeForSync();
    if (idle) {
        const { appearance: trayAppearance, sig: trayAppSig } = trayAppearanceForTraySync();
        const idleSig = `idle|${uiTheme}|${trayAppSig}|sp:${traySp}|vol:${trayVol}|sh:${audioShuffling ? 1 : 0}|lp:${audioLooping ? 1 : 0}|fav:0`;
        if (_traySyncSig === idleSig) return;
        _traySyncSig = idleSig;
        _traySyncKeyNoApp = '';
        if (typeof window !== 'undefined' && window.__TRAY_SYNC_DEBUG__) {
            console.info('[tray-main] update_tray_now_playing → Rust', {
                idle: true,
                ui_theme: uiTheme,
                colorscheme: trayAppearanceForLog(trayAppearance),
            });
        }
        /* Tauri v2 requires struct command args to be wrapped in the Rust parameter name (`payload`),
         * not passed flat. Passing flat fails with "missing required key payload" and the tray stays
         * frozen at its last known state — this was the silent root cause of the broken tray updates. */
        void inv('update_tray_now_playing', {
            payload: {
                title_bar: null,
                tooltip: tooltipBase,
                idle: true,
                popover_title: '',
                popover_subtitle: '',
                popover_reveal_path: null,
                elapsed_sec: 0,
                total_sec: null,
                popover_playing: false,
                popover_idle_label: catalogFmt('tray.popover_idle'),
                playback_speed: traySp,
                volume_pct: trayVol,
                ui_theme: uiTheme,
                appearance: trayAppearance,
                shuffle_on: audioShuffling,
                loop_on: audioLooping,
                favorite_on: false,
            },
        }).catch(() => {});
        return;
    }
    let cur;
    let dur;
    const ptHint = playbackTimeHint && typeof playbackTimeHint === 'object' ? playbackTimeHint : null;
    if (ptHint && typeof ptHint.cur === 'number' && Number.isFinite(ptHint.cur)) {
        cur = ptHint.cur;
        dur = typeof ptHint.dur === 'number' ? ptHint.dur : NaN;
    } else if (_enginePlaybackActive && typeof window !== 'undefined') {
        cur =
            typeof window._enginePlaybackPosSec === 'number' && Number.isFinite(window._enginePlaybackPosSec)
                ? window._enginePlaybackPosSec
                : 0;
        dur = enginePlaybackDurationSec();
    } else if (audioReverseMode && _reversedBuf && _bufPlaying) {
        dur = _reversedBuf.duration;
        const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
        const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
        cur = Math.max(0, dur - posInRev);
    } else if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath && audioPlayerPath === videoPlayerPath) {
        const vid = npVideoElementIfDrivingRow();
        if (vid) {
            cur = vid.currentTime || 0;
        } else {
            cur = typeof audioPlayer !== 'undefined' && audioPlayer ? audioPlayer.currentTime || 0 : 0;
        }
        dur = typeof enginePlaybackDurationSec === 'function' ? enginePlaybackDurationSec() : 0;
        if ((!Number.isFinite(dur) || dur <= 0) && vid && Number.isFinite(vid.duration) && vid.duration > 0) {
            dur = vid.duration;
        }
        if ((!Number.isFinite(dur) || dur <= 0) && typeof audioPlayer !== 'undefined' && audioPlayer) {
            const ad = audioPlayer.duration;
            if (Number.isFinite(ad) && ad > 0) dur = ad;
        }
        const pathForMeta = audioPlayerPath || resumePath || null;
        if ((!Number.isFinite(dur) || dur <= 0) && pathForMeta && typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined') {
            const s = findByPath(allAudioSamples, pathForMeta);
            if (s && typeof s.duration === 'number' && s.duration > 0) dur = s.duration;
        }
    } else {
        cur = audioPlayer.currentTime;
        dur = audioPlayer.duration;
        const pathForMeta = audioPlayerPath || resumePath || null;
        if ((!Number.isFinite(dur) || dur <= 0) && pathForMeta && typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined') {
            const s = findByPath(allAudioSamples, pathForMeta);
            if (s && typeof s.duration === 'number' && s.duration > 0) dur = s.duration;
        }
    }
    let track = trayNowPlayingDisplayName();
    const playing = typeof isAudioPlaying === 'function' && isAudioPlaying();
    const ft = typeof formatTime === 'function' ? formatTime : (x) => String(x);
    const totalStr = Number.isFinite(dur) && dur > 0 ? ft(dur) : '—';
    const timeLine = `${ft(cur)} / ${totalStr}`;
    const status = playing ? catalogFmt('tray.status_playing') : catalogFmt('tray.status_paused');
    /* Single line: macOS status-item tooltips often drop or truncate after \n */
    const tooltip = `${track} — ${timeLine} • ${status}`;
    /* Menu-bar title is track name only — elapsed/total stay in the popover + tooltip. */
    const title_bar = track.length > 44 ? `${track.slice(0, 41)}…` : track;
    const pathForTrayMeta = audioPlayerPath || resumePath || null;
    let popover_subtitle = pathForTrayMeta ? npTrayPopoverSubtitleMetaOnly(pathForTrayMeta).trim() : '';
    /* Tray HUD: avoid repeating the title when meta is only the basename (non-library file). */
    if (popover_subtitle && track) {
        if (popover_subtitle === track) {
            popover_subtitle = '';
        } else if (!popover_subtitle.includes('\u2022')) {
            const stem = (s) => {
                const t = s.trim();
                const i = t.lastIndexOf('.');
                if (i > 0 && i < t.length - 1) return t.slice(0, i).toLowerCase();
                return t.toLowerCase();
            };
            if (stem(popover_subtitle) === stem(track)) popover_subtitle = '';
        }
    }
    const durKey = Number.isFinite(dur) && dur > 0 ? Math.floor(dur) : -1;
    const sigPath = audioPlayerPath || resumePath || '';
    const trayFavOn = !!(sigPath && typeof isFavorite === 'function' && isFavorite(sigPath));
    /* Per-sample loop region for the currently playing path — draws braces on the tray progress bar. */
    const trayLoopRegionRaw = sigPath && typeof getSampleLoopRegion === 'function'
        ? getSampleLoopRegion(sigPath)
        : { enabled: false, startFrac: 0, endFrac: 1 };
    const trayLoopRegionEnabled = !!trayLoopRegionRaw.enabled;
    const trayLoopDurSec = Number.isFinite(dur) && dur > 0 ? dur : 0;
    const trayLoopStartSec = trayLoopRegionEnabled ? trayLoopRegionRaw.startFrac * trayLoopDurSec : 0;
    const trayLoopEndSec = trayLoopRegionEnabled ? trayLoopRegionRaw.endFrac * trayLoopDurSec : 0;
    const trayLoopSigKey = `${trayLoopRegionEnabled ? 1 : 0}:${Math.round(trayLoopStartSec * 1000)}:${Math.round(trayLoopEndSec * 1000)}`;
    const wfKey = trayWaveformKeyForSync(sigPath);
    const keyNoApp = `${sigPath}|${track}|${popover_subtitle}|${Math.floor(cur)}|${durKey}|${playing ? 1 : 0}|${uiTheme}|sp:${traySp}|vol:${trayVol}|sh:${audioShuffling ? 1 : 0}|lp:${audioLooping ? 1 : 0}|fav:${trayFavOn ? 1 : 0}|lr:${trayLoopSigKey}|wf:${wfKey}`;
    if (keyNoApp === _traySyncKeyNoApp) return;
    const { appearance: trayAppearancePlaying, sig: trayAppSigPlaying } = trayAppearanceForTraySync();
    /* Include title + subtitle: first ticks often have empty `#npName` / meta; dedupe must not block later updates. */
    /* `wfKey` must be part of `sig`, not only `keyNoApp`, or peaks can load while time/title are unchanged and
     * this branch skips IPC forever (regressed after waveform flatten moved past `keyNoApp` dedupe). */
    const sig = `${sigPath}|${track}|${popover_subtitle}|${Math.floor(cur)}|${durKey}|${playing ? 1 : 0}|${uiTheme}|${trayAppSigPlaying}|sp:${traySp}|vol:${trayVol}|sh:${audioShuffling ? 1 : 0}|lp:${audioLooping ? 1 : 0}|fav:${trayFavOn ? 1 : 0}|lr:${trayLoopSigKey}|wf:${wfKey}`;
    if (sig === _traySyncSig) {
        _traySyncKeyNoApp = keyNoApp;
        return;
    }
    const trayWaveformPeaks = materializeTrayWaveformPeaksForSync(sigPath);
    _traySyncSig = sig;
    _traySyncKeyNoApp = keyNoApp;
    if (typeof window !== 'undefined' && window.__TRAY_SYNC_DEBUG__) {
        console.info('[tray-main] update_tray_now_playing → Rust', {
            idle: false,
            ui_theme: uiTheme,
            colorscheme: trayAppearanceForLog(trayAppearancePlaying),
            path: sigPath,
            popover_title: track,
            popover_subtitle: popover_subtitle,
            subtitle_len: popover_subtitle.length,
            playing,
            elapsed_sec: cur,
            total_sec: Number.isFinite(dur) && dur > 0 ? dur : null,
        });
    }
    void inv('update_tray_now_playing', {
        payload: {
            title_bar,
            tooltip,
            idle: false,
            popover_title: track,
            popover_subtitle,
            popover_reveal_path: pathForTrayMeta || null,
            elapsed_sec: cur,
            total_sec: Number.isFinite(dur) && dur > 0 ? dur : null,
            popover_playing: playing,
            playback_speed: traySp,
            volume_pct: trayVol,
            ui_theme: uiTheme,
            appearance: trayAppearancePlaying,
            shuffle_on: audioShuffling,
            loop_on: audioLooping,
            favorite_on: trayFavOn,
            loop_region_enabled: trayLoopRegionEnabled,
            loop_region_start_sec: trayLoopStartSec,
            loop_region_end_sec: trayLoopEndSec,
            waveform_peaks: trayWaveformPeaks,
        },
    }).catch(() => {});
}

function updatePlaybackTime() {
    let cur;
    let dur;
    if (_enginePlaybackActive && typeof window !== 'undefined') {
        /* Interpolate between the 30 Hz engine `playback_status` polls so the playhead /
         * waveform cursor animates at the full rAF rate (~60 Hz). Without interpolation the
         * cursor visibly steps in poll-interval chunks because every rAF tick reads the same
         * stale `_enginePlaybackPosSec` until the next poll writes a new one. `_enginePlaybackPosAnchorMs`
         * is set in `runEnginePlaybackStatusTick` whenever the position is refreshed.
         * Stop advancing on pause; honor current `audioSpeed` pref for non-1x playback. */
        const basePos =
            typeof window._enginePlaybackPosSec === 'number' && Number.isFinite(window._enginePlaybackPosSec)
                ? window._enginePlaybackPosSec
                : 0;
        const anchor = typeof window._enginePlaybackPosAnchorMs === 'number'
            ? window._enginePlaybackPosAnchorMs
            : performance.now();
        const paused = window._enginePlaybackPaused === true;
        let speed = 1;
        if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
            const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
            if (Number.isFinite(raw)) speed = Math.max(0.25, Math.min(4, raw));
        }
        const elapsedSinceAnchor = paused ? 0 : (performance.now() - anchor) / 1000;
        cur = basePos + elapsedSinceAnchor * speed;
        dur = enginePlaybackDurationSec();
        if (Number.isFinite(dur) && dur > 0 && cur > dur) cur = dur;
        if (cur < 0) cur = 0;
    } else if (audioReverseMode && _reversedBuf && _bufPlaying) {
        dur = _reversedBuf.duration;
        const elapsed = _playbackCtx.currentTime - _bufSegStartCtx;
        const posInRev = _bufOffsetInRev + elapsed * _bufPlaybackRate;
        cur = Math.max(0, dur - posInRev);
    } else if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath && audioPlayerPath === videoPlayerPath) {
        const vid = npVideoElementIfDrivingRow();
        if (vid) {
            cur = vid.currentTime || 0;
        } else {
            cur = typeof audioPlayer !== 'undefined' && audioPlayer ? audioPlayer.currentTime || 0 : 0;
        }
        /* Video rows are not in `allAudioSamples`; `audioPlayer` is usually idle here. Prefer engine /
         * `<video>` fallbacks so the NP waveform cursor is not stuck hidden at 0:00 / 0:00. */
        dur = typeof enginePlaybackDurationSec === 'function' ? enginePlaybackDurationSec() : 0;
        if ((!Number.isFinite(dur) || dur <= 0) && vid && Number.isFinite(vid.duration) && vid.duration > 0) {
            dur = vid.duration;
        }
        if ((!Number.isFinite(dur) || dur <= 0) && typeof audioPlayer !== 'undefined' && audioPlayer) {
            const ad = audioPlayer.duration;
            if (Number.isFinite(ad) && ad > 0) dur = ad;
        }
    } else {
        cur = audioPlayer.currentTime;
        dur = audioPlayer.duration;
    }
    // A-B loop enforcement (forward playback only)
    if (!audioReverseMode && _abLoop && dur > 0 && cur >= _abLoop.end) {
        const vidLoop =
            typeof videoPlayerPath !== 'undefined' && videoPlayerPath === audioPlayerPath
                ? npVideoElementIfDrivingRow()
                : null;
        if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
            void window.vstUpdater.audioEngineInvoke({cmd: 'playback_seek', position_sec: _abLoop.start});
            if (vidLoop) vidLoop.currentTime = _abLoop.start;
        } else {
            if (vidLoop) vidLoop.currentTime = _abLoop.start;
            else audioPlayer.currentTime = _abLoop.start;
        }
    }
    _ensureNpPlaybackDomRefs();
    if (_npTimeEl) _npTimeEl.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
    const durOk = Number.isFinite(dur) && dur > 0;
    if (durOk) {
        const pct = (cur / dur) * 100;
        if (_npProgressEl) _npProgressEl.style.width = pct + '%';
        if (_npCursorEl) {
            _npCursorEl.style.display = '';
            _npCursorEl.style.left = pct + '%';
        }
        // Playback cursor — metadata panel (cache `#metaWaveformBox`; path switch, DOM detach, or null ref refreshes)
        if (
            _npMetaWaveformCachedPath !== audioPlayerPath ||
            !_npMetaWaveformBox ||
            !_npMetaWaveformBox.isConnected
        ) {
            _npMetaWaveformCachedPath = audioPlayerPath;
            _npMetaWaveformBox = audioPlayerPath ? document.getElementById('metaWaveformBox') : null;
            _npMetaWfFill = null;
            _npMetaWfCursor = null;
            _npMetaWfTimeLabel = null;
        }
        const metaWaveform = _npMetaWaveformBox;
        if (metaWaveform && metaWaveform.isConnected && metaWaveform.dataset.path === audioPlayerPath) {
            if (
                !_npMetaWfFill ||
                !_npMetaWfFill.isConnected ||
                !_npMetaWfCursor ||
                !_npMetaWfCursor.isConnected ||
                !_npMetaWfTimeLabel ||
                !_npMetaWfTimeLabel.isConnected
            ) {
                _npMetaWfFill = metaWaveform.querySelector('.waveform-progress-fill');
                _npMetaWfCursor = metaWaveform.querySelector('.waveform-cursor');
                _npMetaWfTimeLabel = metaWaveform.querySelector('.waveform-time-label');
            }
            if (_npMetaWfFill) _npMetaWfFill.style.width = pct + '%';
            if (_npMetaWfCursor) {
                _npMetaWfCursor.style.display = '';
                _npMetaWfCursor.style.left = pct + '%';
            }
            if (_npMetaWfTimeLabel) _npMetaWfTimeLabel.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
        } else if (metaWaveform && metaWaveform.isConnected) {
            const hideMetaC = metaWaveform.querySelector('.waveform-cursor');
            if (hideMetaC) hideMetaC.style.display = 'none';
        }
        // Expanded video row — same `cur` / `dur` / `pct` as NP (engine + `<video>` stay aligned)
        const vidWfBox = document.getElementById('videoWaveformBox');
        if (
            vidWfBox &&
            vidWfBox.isConnected &&
            typeof videoPlayerPath !== 'undefined' &&
            videoPlayerPath &&
            audioPlayerPath === videoPlayerPath &&
            vidWfBox.dataset.path === audioPlayerPath
        ) {
            const vFill = vidWfBox.querySelector('.waveform-progress-fill');
            const vCursor = vidWfBox.querySelector('.waveform-cursor');
            const vLab = vidWfBox.querySelector('.waveform-time-label');
            if (vFill) vFill.style.width = pct + '%';
            if (vCursor) {
                vCursor.style.display = '';
                vCursor.style.left = pct + '%';
            }
            if (vLab) vLab.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
        } else if (vidWfBox && vidWfBox.isConnected) {
            const hideVC = vidWfBox.querySelector('.waveform-cursor');
            if (hideVC) hideVC.style.display = 'none';
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
    } else {
        if (_npCursorEl) _npCursorEl.style.display = 'none';
        const metaHideBox =
            (_npMetaWaveformBox && _npMetaWaveformBox.isConnected && _npMetaWaveformBox) ||
            document.getElementById('metaWaveformBox');
        if (metaHideBox) {
            const mhc = metaHideBox.querySelector('.waveform-cursor');
            if (mhc) mhc.style.display = 'none';
        }
        const vidHideBox = document.getElementById('videoWaveformBox');
        if (vidHideBox) {
            const vhc = vidHideBox.querySelector('.waveform-cursor');
            if (vhc) vhc.style.display = 'none';
        }
        if (window._fbCursorEl) window._fbCursorEl.style.display = 'none';
    }
    if (typeof window.syncAeTransportFromPlayback === 'function') {
        let aeTab = _npAeTransportTabEl;
        if (!aeTab || !aeTab.isConnected) {
            aeTab = document.getElementById('tabAudioEngine');
            _npAeTransportTabEl = aeTab;
        }
        if (aeTab && aeTab.classList.contains('active')) {
            window.syncAeTransportFromPlayback();
        }
    }
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback({ cur, dur });
}

/** Seek current playback to a normalized position [0, 1]. Used by now-playing and metadata waveforms. */
function seekPlaybackToPercent(pct) {
    const p = Math.max(0, Math.min(1, pct));
    const hasInvoke =
        typeof window !== 'undefined' &&
        window.vstUpdater &&
        typeof window.vstUpdater.audioEngineInvoke === 'function';
    /* Engine-playback sessions don't set `audioPlayerPath` — the path lives in
     * `window._enginePlaybackResumePath`. Any seek path that checks `audioPlayerPath` first
     * would bail out before reaching the engine seek branch, which is exactly why seeks from
     * the tray popover (`seek:<frac>` → `seekPlaybackToPercent`) silently did nothing while the
     * engine was driving transport. Resolve the effective path from both sources up front. */
    const resumePath =
        typeof window !== 'undefined' &&
        typeof window._enginePlaybackResumePath === 'string' &&
        window._enginePlaybackResumePath.length > 0
            ? window._enginePlaybackResumePath
            : '';
    const effectivePath = audioPlayerPath || (_enginePlaybackActive ? resumePath : '');
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
    if (!effectivePath) {
        logWaveformSeek('abort', { reason: 'no_audioPlayerPath' });
        return;
    }
    if (isEngineUnplayablePath(effectivePath)) {
        logWaveformSeek('abort', { reason: 'engine_unplayable_format' });
        return;
    }
    if (_enginePlaybackActive && hasInvoke) {
        let dur = enginePlaybackDurationSec();
        const vd = dur <= 0 ? videoHostDurationFallbackSec() : 0;
        if (vd > 0) {
            window._enginePlaybackDurSec = vd;
            dur = vd;
        }
        if (dur <= 0) {
            logWaveformSeek('engine_seek_async_duration', { dur, pct: p });
            void (async () => {
                try {
                    const st = await window.vstUpdater.audioEngineInvoke(enginePlaybackStatusInvokePayload());
                    logWaveformSeek('engine_playback_status', { st });
                    if (st && st.ok === true && typeof st.duration_sec === 'number' && st.duration_sec > 0) {
                        window._enginePlaybackDurSec = st.duration_sec;
                        const pos = p * st.duration_sec;
                        const seekRes = await window.vstUpdater.audioEngineInvoke({
                            cmd: 'playback_seek',
                            position_sec: pos,
                        });
                        logWaveformSeek('engine_playback_seek_result', { position_sec: pos, seekRes });
                        window._enginePlaybackPosSec = pos;
                        window._enginePlaybackPosAnchorMs = performance.now();
                        const vid = document.getElementById('videoPlayerEl');
                        if (
                            vid
                            && typeof videoPlayerPath !== 'undefined'
                            && videoPlayerPath
                            && audioPlayerPath === videoPlayerPath
                        ) {
                            vid.currentTime = pos;
                        }
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
        window._enginePlaybackPosSec = pos;
        window._enginePlaybackPosAnchorMs = performance.now();
        const vidSeek = document.getElementById('videoPlayerEl');
        if (
            vidSeek
            && typeof videoPlayerPath !== 'undefined'
            && videoPlayerPath
            && audioPlayerPath === videoPlayerPath
        ) {
            vidSeek.currentTime = pos;
        }
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
        const vid = document.getElementById('videoPlayerEl');
        if (
            vid
            && typeof videoPlayerPath !== 'undefined'
            && videoPlayerPath === audioPlayerPath
            && Number.isFinite(vid.duration)
            && vid.duration > 0
        ) {
            const t = p * vid.duration;
            logWaveformSeek('html5_video_seek', { currentTime: t, duration: vid.duration });
            vid.currentTime = t;
            return;
        }
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
            const st = await window.vstUpdater.audioEngineInvoke(enginePlaybackStatusInvokePayload());
            if (!st || st.ok !== true || st.loaded !== true) return;
            let dur = typeof st.duration_sec === 'number' ? st.duration_sec : 0;
            if (dur <= 0) {
                const vd = videoHostDurationFallbackSec();
                if (vd > 0) dur = vd;
            }
            if (dur <= 0) return;
            const cur = typeof st.position_sec === 'number' ? st.position_sec : 0;
            const next = Math.max(0, Math.min(dur, cur + d));
            await window.vstUpdater.audioEngineInvoke({cmd: 'playback_seek', position_sec: next});
            window._enginePlaybackPosSec = next;
            window._enginePlaybackPosAnchorMs = performance.now();
            const vid = document.getElementById('videoPlayerEl');
            if (vid && typeof videoPlayerPath !== 'undefined' && videoPlayerPath === audioPlayerPath) {
                vid.currentTime = next;
            }
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
    } else if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath === audioPlayerPath) {
        const vid = document.getElementById('videoPlayerEl');
        if (vid && Number.isFinite(vid.duration) && vid.duration > 0) {
            dur = vid.duration;
            cur = vid.currentTime || 0;
        } else {
            dur = audioPlayer.duration;
            if (!dur || Number.isNaN(dur)) return;
            cur = audioPlayer.currentTime;
        }
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

function setAudioVolume(value, opts) {
    const vol = parseInt(value, 10) / 100;
    const npSlider = document.getElementById('npVolume');
    if (npSlider) npSlider.value = String(value);
    const npPct = document.getElementById('npVolumePct');
    if (npPct) npPct.textContent = value + '%';
    const vfsPct = document.getElementById('videoFsVolumePct');
    if (vfsPct) vfsPct.textContent = value + '%';
    const vfs = document.getElementById('videoFsVolume');
    if (vfs && document.activeElement !== vfs) vfs.value = String(value);
    const eqVs = document.getElementById('npEqVolSlider');
    if (eqVs) eqVs.value = String(value);
    const eqVp = document.getElementById('npEqVolVal');
    if (eqVp) eqVp.textContent = value + '%';
    const aeV = document.getElementById('aeVolume');
    if (aeV) aeV.value = String(value);
    const aePct = document.getElementById('aeVolumePct');
    if (aePct) aePct.textContent = value + '%';
    prefs.setItem('audioVolume', value);
    /* Debounced tray-popover sync MUST run when the change originated in the MAIN window so
     * Rust's `TrayState.last_popover_emit.volume_pct` catches up (otherwise `start_tray_host_poll`
     * re-emits the stale cached value and the tray slider snaps back after `_trayVolUserActive`
     * expires).
     *
     * BUT when the change originated in the TRAY popover (`ipc.js` menu-action handler for
     * `volume:N`), Rust already updated `last_popover_emit.volume_pct` synchronously inside
     * `tray_popover_action` BEFORE emitting the menu-action to main — so the sync here is
     * redundant. Worse, while the main window is minimized on macOS, WebKit freezes
     * `<audio>` element state updates to background windows, so `audioPlayer.currentTime` read
     * inside `syncTrayNowPlayingFromPlayback` is stuck at the value it held when the window
     * lost visibility. That stale `elapsed_sec` then gets pushed through `update_tray_now_playing`
     * → `tray-popover-state`, and the popover's drift-rebase yanks the progress thumb backward
     * to "the last point main was visible" on every tray volume drag. Skip the sync for
     * tray-sourced changes. */
    const fromTray = !!(opts && opts.fromTray);
    if (!fromTray && typeof window !== 'undefined') {
        if (window._trayVolSyncTimer) clearTimeout(window._trayVolSyncTimer);
        window._trayVolSyncTimer = setTimeout(() => {
            window._trayVolSyncTimer = null;
            if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
        }, 150);
    }
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
        audioPlayer.volume = 0;
        audioPlayer.muted = true;
        if (_gainNode) {
            _gainNode.gain.value = 0;
        }
        /* Coalesce engine DSP IPC to one call per animation frame (≤60 Hz) when the main window
         * is foreground-visible. Tray volume drag fires `input` events at ~120 Hz on macOS
         * WebKit; without coalescing, every tick sends a full `playback_set_dsp` round-trip over
         * the audio-engine's shared stdin/stdout mutex, which `start_tray_host_poll`'s
         * `playback_status` also uses — a saturated DSP queue stalls the host poll and the
         * progress thumb snaps backward.
         *
         * BUT `requestAnimationFrame` is paused by WebKit when the window is minimized /
         * unfocused / hidden, so rAF coalescing would silently drop every drag IPC when the user
         * is driving the tray popover with the main window minimized. Fall back to immediate
         * dispatch in that case — the user can't see the progress bar anyway, and the IPC flood
         * is bounded by the ~120 Hz drag rate. */
        const idle =
            typeof window !== 'undefined' &&
            typeof window.isUiIdleHeavyCpu === 'function' &&
            window.isUiIdleHeavyCpu();
        if (idle || typeof requestAnimationFrame !== 'function') {
            window.syncEnginePlaybackDspFromPrefs();
        } else {
            if (window._engineDspCoalesceRaf == null) {
                window._engineDspCoalesceRaf = requestAnimationFrame(() => {
                    window._engineDspCoalesceRaf = null;
                    if (typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
                        window.syncEnginePlaybackDspFromPrefs();
                    }
                });
            }
        }
        return;
    }
    audioPlayer.volume = Math.max(0, Math.min(1, vol));
    if (_gainNode) {
        _gainNode.gain.value = vol * parseFloat(document.getElementById('npGainSlider')?.value || '1');
    }
    /* No-engine video fallback only: loudness comes from `<video>`, not `audioPlayer`. Engine video is muted here. */
    if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath && audioPlayerPath === videoPlayerPath) {
        const vid = document.getElementById('videoPlayerEl');
        if (vid) {
            vid.volume = Math.max(0, Math.min(1, vol));
            vid.muted = vol <= 0;
        }
    }
}

/**
 * Align `#videoPlayerEl` with `prefs('audioSpeed')` or an explicit clamped rate. AudioEngine video uses
 * `playback_set_speed` for PCM; the muted `<video>` stays at 1× unless this runs — picture lags audio.
 * @param {number} [clampedSpeed] — 0.25…4; omit to read prefs.
 */
function syncVideoPlayerElPlaybackRate(clampedSpeed) {
    if (typeof videoPlayerPath === 'undefined' || !videoPlayerPath) return;
    if (typeof audioPlayerPath === 'undefined' || audioPlayerPath !== videoPlayerPath) return;
    const vid = document.getElementById('videoPlayerEl');
    if (!vid) return;
    let s = clampedSpeed;
    if (!Number.isFinite(s)) {
        if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return;
        const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
        s = Number.isFinite(raw) ? Math.max(0.25, Math.min(4, raw)) : 1;
    } else {
        s = Math.max(0.25, Math.min(4, s));
    }
    try {
        /* WebKit often resets `playbackRate` after `load()` / `loadedmetadata`; `defaultPlaybackRate`
         * survives some of those transitions better than assigning `playbackRate` alone. */
        vid.defaultPlaybackRate = s;
        vid.playbackRate = s;
    } catch (_) {}
}

if (typeof window !== 'undefined') {
    window.syncVideoPlayerElPlaybackRate = syncVideoPlayerElPlaybackRate;
}

function setPlaybackSpeed(value, opts) {
    const v = parseFloat(value);
    const clamped = Number.isFinite(v) ? Math.max(0.25, Math.min(4, v)) : 1;
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
        syncVideoPlayerElPlaybackRate(clamped);
        const fromTray = !!(opts && opts.fromTray);
        if (!fromTray && typeof syncTrayNowPlayingFromPlayback === 'function') {
            syncTrayNowPlayingFromPlayback();
        }
        return;
    }
    if (audioReverseMode && _bufSrc && _bufPlaying) {
        const bufClamped = Math.max(0.0625, Math.min(16, clamped));
        _bufSrc.playbackRate.value = bufClamped;
        _bufPlaybackRate = bufClamped;
    } else {
        audioPlayer.playbackRate = clamped;
    }
    syncVideoPlayerElPlaybackRate(clamped);
    /* Tray-sourced speed changes: Rust already set `last_popover_emit.playback_speed` inside
     * `tray_popover_action` before dispatching the menu-action to main, so syncing back here
     * is redundant. Doing it anyway would push a stale `audioPlayer.currentTime` (frozen by
     * WebKit while main is minimized on macOS) to the popover and snap the progress thumb. See
     * the matching note in `setAudioVolume`. */
    const fromTray = !!(opts && opts.fromTray);
    if (!fromTray && typeof syncTrayNowPlayingFromPlayback === 'function') {
        syncTrayNowPlayingFromPlayback();
    }
}

/**
 * Set the speed algorithm mode: 'resample' (pitch follows rate) or 'timestretch' (pitch preserved).
 * Persists to prefs, syncs all dropdowns, and forwards to AudioEngine if active.
 */
function setSpeedMode(value) {
    const mode = value === 'timestretch' ? 'timestretch' : 'resample';
    prefs.setItem('audioSpeedMode', mode);
    const npSm = document.getElementById('npSpeedMode');
    if (npSm) npSm.value = mode;
    const aeSm = document.getElementById('aeSpeedMode');
    if (aeSm) aeSm.value = mode;
    if (_enginePlaybackActive && typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
        void window.vstUpdater.audioEngineInvoke({cmd: 'playback_set_speed_mode', mode});
    }
    if (typeof window.syncEngineSpeedModeFromPrefs === 'function') {
        window.syncEngineSpeedModeFromPrefs();
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
        items += metaItem(_audioFmt('ui.audio.meta_label_file_name'), meta.fileName, true);
        items += metaItem(_audioFmt('ui.audio.meta_label_format'), meta.format);
        items += metaItem(_audioFmt('ui.audio.meta_label_size'), formatAudioSize(meta.sizeBytes));
        items += metaItem(_audioFmt('ui.audio.meta_label_full_path'), meta.fullPath, true);

        if (meta.sampleRate) {
            items += metaItem(
                _audioFmt('ui.audio.meta_label_sample_rate'),
                _audioFmt('ui.audio.meta_sample_rate_hz', {rate: meta.sampleRate.toLocaleString()})
            );
        }
        if (meta.bitsPerSample) {
            items += metaItem(
                _audioFmt('ui.audio.meta_label_bit_depth'),
                _audioFmt('ui.audio.meta_bit_depth_bits', {n: meta.bitsPerSample})
            );
        }
        if (meta.channels) {
            const chVal = meta.channels === 1
                ? _audioFmt('ui.tt.mono')
                : meta.channels === 2
                    ? _audioFmt('ui.btn.stereo')
                    : _audioFmt('ui.audio.meta_channels_multichannel', {n: meta.channels});
            items += metaItem(_audioFmt('ui.audio.meta_label_channels'), chVal);
        }
        if (meta.duration) items += metaItem(_audioFmt('ui.audio.meta_label_duration'), formatTime(meta.duration));
        if (meta.byteRate) {
            items += metaItem(
                _audioFmt('ui.audio.meta_label_byte_rate'),
                _audioFmt('ui.audio.meta_byte_rate_per_sec', {size: formatAudioSize(meta.byteRate)})
            );
        }

        const ttBpm = escapeHtml(_audioFmt('ui.audio.meta_tt_bpm'));
        const ttKey = escapeHtml(_audioFmt('ui.audio.meta_tt_key'));
        const ttLufs = escapeHtml(_audioFmt('ui.audio.meta_tt_lufs'));
        const lblBpm = escapeHtml(_audioFmt('ui.audio.meta_label_bpm'));
        const lblKey = escapeHtml(_audioFmt('ui.audio.meta_label_key'));
        const lblLufs = escapeHtml(_audioFmt('ui.audio.meta_label_lufs'));
        // BPM and Key placeholders — filled async
        items += `<div class="meta-item" id="metaBpmItem" title="${ttBpm}"><span class="meta-label">${lblBpm}</span><span class="meta-value" id="metaBpmValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;
        items += `<div class="meta-item" id="metaKeyItem" title="${ttKey}"><span class="meta-label">${lblKey}</span><span class="meta-value" id="metaKeyValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;
        items += `<div class="meta-item" id="metaLufsItem" title="${ttLufs}"><span class="meta-label">${lblLufs}</span><span class="meta-value" id="metaLufsValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;

        const fmtDate = (v) => {
            if (!v) return _audioFmt('ui.audio.meta_na');
            const d = new Date(v);
            return isNaN(d) ? _audioFmt('ui.audio.meta_na') : d.toLocaleString();
        };
        items += metaItem(_audioFmt('ui.audio.meta_label_created'), fmtDate(meta.created));
        items += metaItem(_audioFmt('ui.audio.meta_label_modified'), fmtDate(meta.modified));
        items += metaItem(_audioFmt('ui.audio.meta_label_accessed'), fmtDate(meta.accessed));
        items += metaItem(_audioFmt('ui.audio.meta_label_permissions'), meta.permissions);

        const wfSeekT = escapeHtml(_audioFmt('ui.audio.meta_waveform_seek_title'));
        const wfCanT = escapeHtml(_audioFmt('ui.audio.meta_waveform_canvas_tt'));
        const sgBoxT = escapeHtml(_audioFmt('ui.audio.meta_spectrogram_box_tt'));
        const sgCanT = escapeHtml(_audioFmt('ui.audio.meta_spectrogram_canvas_tt'));
        const sgBadge = escapeHtml(_audioFmt('ui.audio.meta_spectrogram_badge'));
        // Waveform preview with seek support + per-sample loop region braces (toggle + drag)
        const waveformHtml = `<div class="meta-waveform" id="metaWaveformBox" data-path="${escapeHtml(filePath)}" title="${wfSeekT}">
      <canvas id="metaWaveformCanvas" title="${wfCanT}"></canvas>
      <div class="waveform-progress-fill"></div>
      <div class="waveform-loop-region" style="display:none;"></div>
      <div class="waveform-loop-brace waveform-loop-brace-start" data-loop-brace="start" style="display:none;left:25%;" title="Drag to set loop start"></div>
      <div class="waveform-loop-brace waveform-loop-brace-end" data-loop-brace="end" style="display:none;left:75%;" title="Drag to set loop end"></div>
      <button type="button" class="waveform-loop-toggle" data-action="toggleMetaLoopRegion" title="Toggle loop region">L</button>
      <div class="waveform-cursor" style="left:0;"></div>
      <div class="waveform-time-label">${meta.duration ? formatTime(meta.duration) : ''}</div>
    </div>
    <div class="meta-waveform" style="height:80px;cursor:default;" title="${sgBoxT}">
      <canvas id="metaSpectrogramCanvas" width="800" height="80" style="position:absolute;top:0;left:0;width:100%;height:100%;" title="${sgCanT}"></canvas>
      <span style="position:absolute;top:2px;left:4px;font-size:8px;color:var(--text-dim);pointer-events:none;">${sgBadge}</span>
    </div>`;

        const _closeT = typeof escapeHtml === 'function' ? escapeHtml(_audioFmt('ui.audio.meta_close_title')) : _audioFmt('ui.audio.meta_close_title');
        metaRow.innerHTML = `<td colspan="12"><div class="audio-meta-panel"><span class="meta-close-btn" data-action="closeMetaRow" title="${_closeT}">&#10005;</span>${waveformHtml}${items}</div></td>`;

        // Hydrate loop-region braces/toggle from persisted state for this path
        applyMetaLoopRegionUI(filePath);

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

        // Sync playhead if this row is already the loaded transport (engine, `<audio>`, or video NP path)
        if (audioPlayerPath === filePath) {
            requestAnimationFrame(() => {
                if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
            });
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
            const na = _audioFmt('ui.audio.meta_na');
            const bpmEl = document.getElementById('metaBpmValue');
            if (bpmEl) bpmEl.textContent = na;
            const keyEl = document.getElementById('metaKeyValue');
            if (keyEl) keyEl.textContent = na;
            const lufsEl = document.getElementById('metaLufsValue');
            if (lufsEl) lufsEl.textContent = na;
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
            await previewAudio(filePath, { minimizeFloatingPlayer: true });
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

// BPM / key / LUFS — in-memory maps; `dbGetAnalysis` + meta pipelines fill per path. Debounced
// `writeCacheFile('…-cache.json')` snapshots to SQLite — no startup bulk read (large libraries hung the UI).
let _bpmCache = {};
let _bpmCacheDirty = false;

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
    /* `bpmEl` may be null when this is called from `previewAudio` rather than
     * `expandMetaForPath` — the function still needs to compute and cache the result for the
     * main table row + DB, so the no-DOM case is valid. DOM updates below are all null-guarded. */
    const bpmEl = document.getElementById('metaBpmValue');

    // Check in-memory cache
    if (_bpmCache[filePath] !== undefined) {
        if (bpmEl) {
            bpmEl.textContent = _bpmCache[filePath]
                ? _audioFmt('ui.audio.meta_bpm_value', {n: _bpmCache[filePath]})
                : _audioFmt('ui.audio.meta_na');
        }
        return;
    }

    // Check SQLite (analysis data stored on audio_samples row)
    try {
        const analysis = await window.vstUpdater.dbGetAnalysis(filePath);
        if (analysis && analysis.bpm) {
            _bpmCache[filePath] = analysis.bpm;
            if (bpmEl) bpmEl.textContent = _audioFmt('ui.audio.meta_bpm_value', {n: analysis.bpm});
            // Also fill key and LUFS from same query
            if (analysis.key) {
                _keyCache[filePath] = analysis.key;
                const keyEl = document.getElementById('metaKeyValue');
                if (keyEl) keyEl.textContent = analysis.key;
            }
            if (analysis.lufs != null) {
                _lufsCache[filePath] = analysis.lufs;
                const lufsEl = document.getElementById('metaLufsValue');
                if (lufsEl) lufsEl.textContent = _audioFmt('ui.audio.meta_lufs_value', {n: analysis.lufs});
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
            currentBpmEl.textContent = bpm ? _audioFmt('ui.audio.meta_bpm_value', {n: bpm}) : _audioFmt('ui.audio.meta_na');
        }
        // Update table row cell
        const tableRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow) {
            const cell = tableRow.querySelector('.col-bpm');
            if (cell) cell.textContent = bpm || '';
        }
    } catch {
        _bpmCache[filePath] = null;
        if (bpmEl) bpmEl.textContent = _audioFmt('ui.audio.meta_na');
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

    if (_keyCache[filePath] !== undefined) {
        if (keyEl) keyEl.textContent = _keyCache[filePath] || _audioFmt('ui.audio.meta_na');
        return;
    }

    try {
        const key = await window.vstUpdater.detectAudioKey(filePath);
        _keyCache[filePath] = key;
        _debounceKeySave();
        const currentKeyEl = document.getElementById('metaKeyValue');
        const metaRow = document.getElementById('audioMetaRow');
        if (currentKeyEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
            currentKeyEl.textContent = key || _audioFmt('ui.audio.meta_na');
        }
        // Update table row cell
        const tableRow2 = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow2) {
            const cell = tableRow2.querySelector('.col-key');
            if (cell) cell.textContent = key || '';
        }
    } catch {
        _keyCache[filePath] = null;
        if (keyEl) keyEl.textContent = _audioFmt('ui.audio.meta_na');
    }
}

async function measureLufsForMeta(filePath) {
    const lufsEl = document.getElementById('metaLufsValue');

    if (_lufsCache[filePath] !== undefined) {
        if (lufsEl) {
            lufsEl.textContent = _lufsCache[filePath] != null
                ? _audioFmt('ui.audio.meta_lufs_value', {n: _lufsCache[filePath]})
                : _audioFmt('ui.audio.meta_na');
        }
        return;
    }

    try {
        const lufs = await window.vstUpdater.measureLufs(filePath);
        _lufsCache[filePath] = lufs;
        _debounceLufsSave();
        const currentEl = document.getElementById('metaLufsValue');
        const metaRow = document.getElementById('audioMetaRow');
        if (currentEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
            currentEl.textContent = lufs != null ? _audioFmt('ui.audio.meta_lufs_value', {n: lufs}) : _audioFmt('ui.audio.meta_na');
        }
        const tableRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(filePath)}"]`);
        if (tableRow) {
            const cell = tableRow.querySelector('.col-lufs');
            if (cell) cell.textContent = lufs != null ? lufs : '';
        }
    } catch {
        _lufsCache[filePath] = null;
        if (lufsEl) lufsEl.textContent = _audioFmt('ui.audio.meta_na');
    }
}

/**
 * Run BPM / Key / LUFS analysis for a file without requiring the metadata row to be expanded.
 * Called from `previewAudio` so every play path (row click, tray prev/next, menu bar, keyboard
 * shortcut, autoplay EOF, history resume) populates the analysis caches + SQLite row.
 * - Checks the file extension against the supported list (same as `expandMetaForPath`).
 * - Delegates to the existing `*ForMeta` helpers, which are now no-DOM-safe (guarded gets).
 * - Fire-and-forget; caller does `void ensureAudioAnalysisForPath(path)`.
 */
async function ensureAudioAnalysisForPath(filePath) {
    if (!filePath || typeof filePath !== 'string') return;
    const ext = (filePath.split('.').pop() || '').toLowerCase();
    const supported = ['wav', 'aiff', 'aif', 'mp3', 'flac', 'ogg', 'm4a', 'aac', 'opus'];
    if (!supported.includes(ext)) return;
    try {
        await Promise.all([
            estimateBpmForMeta(filePath),
            detectKeyForMeta(filePath),
            measureLufsForMeta(filePath),
        ]);
        await persistAnalysisRowToDb(filePath);
    } catch (_) {
        /* ignore — individual helpers handle their own errors and write null to the caches */
    }
    /* Tray popover subtitle bakes BPM / Key / LUFS from `_bpmCache`/`_keyCache`/`_lufsCache` in
     * `npTrayPopoverSubtitleMetaOnly`. Those caches are only populated now, so the initial tray
     * push at `previewAudio` time had no analysis values. Push the refreshed subtitle directly
     * to Rust via a dedicated lightweight command rather than `syncTrayNowPlayingFromPlayback`:
     * the full sync path reads `audioPlayer.currentTime`, which is frozen by WebKit when the
     * main window is minimized on macOS, and replaying that stale elapsed through a full state
     * emit would snap the tray progress thumb backward (same root cause as the shuffle/loop
     * bug). Only push for the currently playing track — background batch analysis of unrelated
     * rows must not thrash the tray IPC. */
    const resumePath =
        typeof window !== 'undefined' && typeof window._enginePlaybackResumePath === 'string'
            ? window._enginePlaybackResumePath
            : '';
    const curPath =
        (typeof audioPlayerPath === 'string' && audioPlayerPath) || resumePath || '';
    if (curPath && filePath === curPath) {
        const inv =
            typeof window !== 'undefined' &&
            window.__TAURI__ &&
            window.__TAURI__.core &&
            typeof window.__TAURI__.core.invoke === 'function'
                ? window.__TAURI__.core.invoke
                : null;
        if (inv) {
            let subtitle = '';
            try {
                subtitle = npTrayPopoverSubtitleMetaOnly(filePath).trim();
            } catch (_) {
                subtitle = '';
            }
            /* Avoid repeating the track name when meta is just the basename (same rule as
             * `syncTrayNowPlayingFromPlayback`). */
            const track = typeof trayNowPlayingDisplayName === 'function' ? trayNowPlayingDisplayName() : '';
            if (subtitle && track) {
                if (subtitle === track) {
                    subtitle = '';
                } else if (!subtitle.includes('\u2022')) {
                    const stem = (s) => {
                        const t = s.trim();
                        const i = t.lastIndexOf('.');
                        if (i > 0 && i < t.length - 1) return t.slice(0, i).toLowerCase();
                        return t.toLowerCase();
                    };
                    if (stem(subtitle) === stem(track)) subtitle = '';
                }
            }
            void inv('tray_popover_push_subtitle', { subtitle }).catch(() => {});
        }
    }
}

/** Merge in-memory analysis caches with existing DB row and persist (same rules as `batch_analyze`). */
async function persistAnalysisRowToDb(filePath) {
    /* Previously gated on `expandedMetaPath === filePath` to avoid racing against a user
     * expanding a different row. The race doesn't actually corrupt anything (we read + merge
     * the existing DB row before writing), and removing the gate lets `previewAudio` trigger
     * analysis persistence on any play regardless of whether a row is expanded. */
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
// Pause bg analysis when user interacts (resume after prefs `analysisPause` seconds — same as Settings slider)
let _bgIdleTimer = null;

function getBgAnalysisPauseMs() {
    try {
        if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return 3000;
        const raw = prefs.getItem('analysisPause');
        const n = parseInt(raw, 10);
        if (!Number.isFinite(n)) return 3000;
        return Math.max(1000, Math.min(10000, n * 1000));
    } catch {
        return 3000;
    }
}

document.addEventListener('mousedown', () => {
    _bgPaused = true;
    clearTimeout(_bgIdleTimer);
    _bgIdleTimer = setTimeout(() => {
        _bgPaused = false;
    }, getBgAnalysisPauseMs());
}, true);
document.addEventListener('keydown', () => {
    _bgPaused = true;
    clearTimeout(_bgIdleTimer);
    _bgIdleTimer = setTimeout(() => {
        _bgPaused = false;
    }, getBgAnalysisPauseMs());
}, true);

/** Skip badge DOM writes while the window is hidden / unfocused / minimized (`ui-idle.js`). Analysis keeps running. */
function _shouldUpdateBgAnalysisBadgeUi() {
    return !(typeof isUiIdleHeavyCpu === 'function' && isUiIdleHeavyCpu());
}

function _setBgAnalysisBadgeRunning(badge) {
    if (!badge) return;
    if (!_shouldUpdateBgAnalysisBadgeUi()) return;
    badge.textContent = formatBgJobBadgeLine('bpm', 'ui.stats.bpm_bg_working');
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function _setBgAnalysisBadgeProgress(badge, n) {
    if (!badge) return;
    if (!_shouldUpdateBgAnalysisBadgeUi()) return;
    badge.textContent = formatBgJobBadgeLine('bpm', 'ui.stats.bpm_bg_progress', {n});
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

document.addEventListener('ui-idle-heavy-cpu', (ev) => {
    try {
        if (!ev.detail || ev.detail.idle !== false) return;
        const badge = document.getElementById('bgAnalysisBadge');
        if (!badge) return;
        if (_bgAnalysisRunning) {
            if (_bgDone > 0) _setBgAnalysisBadgeProgress(badge, _bgDone);
            else _setBgAnalysisBadgeRunning(badge);
        } else {
            badge.innerHTML = '';
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    } catch (_) {
        /* ignore */
    }
});

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
    if (typeof window !== 'undefined') window.__statusBarBpmJob = true;
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    let analyzedThisSession = 0;

    const badge = document.getElementById('bgAnalysisBadge');
    const BATCH = 50; // 50 files analyzed in parallel per rayon
    if (badge) {
        if (_bgDone > 0) _setBgAnalysisBadgeProgress(badge, _bgDone);
        else _setBgAnalysisBadgeRunning(badge);
    }

    if (window.vstUpdater?.appendLog) {
        window.vstUpdater.appendLog('BPM/LUFS BG — session start');
    }

    try {
    while (!_bgAnalysisAbort) {
        while (_bgPaused && !_bgAnalysisAbort) await new Promise(r => setTimeout(r, 200));
        if (_bgAnalysisAbort) break;

        let paths;
        try {
            paths = await window.vstUpdater.dbUnanalyzedPaths(BATCH);
        } catch (e) {
            if (window.vstUpdater?.appendLog) {
                window.vstUpdater.appendLog(
                    `BPM/LUFS BG — ERROR queue: ${e && e.message ? e.message : e}`
                );
            }
            if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                showToast(
                    toastFmt('toast.bpm_key_lufs_analysis_queue_failed', {err: e && e.message ? e.message : e}),
                    5000,
                    'error'
                );
            }
            break;
        }
        if (!paths || paths.length === 0) break;

        // Single IPC call → Rust processes all in parallel (rayon) → saves to SQLite
        // Returns results directly so we skip N individual dbGetAnalysis roundtrips.
        let analysisResult;
        try {
            // Do not switch to "working" here — it flashes between count and label every batch.
            analysisResult = await window.vstUpdater.batchAnalyze(paths);
            const n = analysisResult.count || 0;
            _bgDone += n;
            analyzedThisSession += n;
        } catch (e) {
            if (window.vstUpdater?.appendLog) {
                window.vstUpdater.appendLog(`BPM/LUFS BG — ERROR batch: ${e.message || e}`);
            }
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
    if (badge && _shouldUpdateBgAnalysisBadgeUi()) badge.innerHTML = '';
    if (
        !_bgAnalysisAbort &&
        analyzedThisSession > 0 &&
        typeof showToast === 'function' &&
        typeof toastFmt === 'function'
    ) {
        showToast(
            toastFmt('toast.bpm_key_lufs_analysis_complete', {n: analyzedThisSession.toLocaleString()}),
            4000
        );
    }
    } finally {
        if (window.vstUpdater?.appendLog) {
            window.vstUpdater.appendLog(
                `BPM/LUFS BG — session end | analyzed=${analyzedThisSession} user_stopped=${_bgAnalysisAbort}`
            );
        }
        _bgAnalysisRunning = false;
        if (typeof window !== 'undefined') window.__statusBarBpmJob = false;
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    }
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
    const na = _audioFmt('ui.audio.meta_na');
    const val = (value == null || value === '') ? na : String(value);
    const escL = escapeHtml(label);
    return `<div class="${cls}" title="${escL}: ${escapeHtml(val)}"><span class="meta-label">${escL}</span><span class="meta-value">${escapeHtml(val)}</span></div>`;
}

function openAudioFolder(filePath) {
    window.vstUpdater.openAudioFolder(filePath).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}

/**
 * Videos table row order (`#videoTableBody`), same semantics as sample rows for autoplay/prev-next.
 * @returns {Array<{ path: string, name: string, format: string, size: string }>}
 */
function getVideoTablePlaybackListItems() {
    const tbody = document.getElementById('videoTableBody');
    if (!tbody) return [];
    const rows = tbody.querySelectorAll('tr[data-video-path]');
    const items = [];
    for (const tr of rows) {
        const path = tr.getAttribute('data-video-path');
        if (!path) continue;
        const rec =
            typeof findByPath === 'function' && typeof allVideos !== 'undefined'
                ? findByPath(allVideos, path)
                : null;
        if (rec) {
            items.push({
                path: rec.path,
                name: rec.name,
                format: rec.format || '',
                size: rec.sizeFormatted || '',
            });
        } else {
            const fmtEl = tr.querySelector('.col-format');
            const fmt = fmtEl ? fmtEl.textContent.trim() : '';
            const nameCell = tr.querySelector('.col-name');
            const nameRaw = nameCell ? nameCell.textContent.trim().replace(/\s+/g, ' ') : '';
            const name = nameRaw || path.split('/').pop().replace(/\.[^.]+$/, '');
            const sizeCell = tr.querySelector('.col-size');
            const size = sizeCell ? sizeCell.textContent.trim() : '';
            items.push({ path, name, format: fmt, size });
        }
    }
    return items;
}

/** True when EOF / prev-next should walk `#videoTableBody` instead of `#audioTableBody`. */
function shouldUseVideoTableForLibraryPlaybackList() {
    if (typeof document === 'undefined') return false;
    const activeTab = document.querySelector('.tab-content.active')?.id;
    if (activeTab === 'tabVideos') return true;
    const cur = typeof audioPlayerPath !== 'undefined' && audioPlayerPath ? String(audioPlayerPath) : '';
    if (!cur) return false;
    try {
        return !!document.querySelector(`#videoTableBody tr[data-video-path="${CSS.escape(cur)}"]`);
    } catch {
        return false;
    }
}

/**
 * Samples or Videos table row order (visible library list — sort, filter, loaded page).
 * Uses `#videoTableBody` when the Videos tab is active or the current track is a row there
 * (so autoplay still works after switching tabs while a video plays).
 * @returns {Array<{ path: string, name: string, format: string, size: string }>}
 */
function getTablePlaybackListItems() {
    if (shouldUseVideoTableForLibraryPlaybackList()) {
        const vItems = getVideoTablePlaybackListItems();
        if (vItems.length > 0) return vItems;
    }
    const tbody = document.getElementById('audioTableBody');
    if (!tbody) return [];
    const rows = tbody.querySelectorAll('tr[data-audio-path]');
    const items = [];
    for (const tr of rows) {
        const path = tr.getAttribute('data-audio-path');
        if (!path) continue;
        const sample =
            typeof findByPath === 'function' && typeof allAudioSamples !== 'undefined'
                ? findByPath(allAudioSamples, path)
                : null;
        if (sample) {
            items.push({
                path: sample.path,
                name: sample.name,
                format: sample.format,
                size: sample.sizeFormatted || '',
            });
        } else {
            const fmt = tr.getAttribute('data-audio-format') || '';
            const nameCell = tr.querySelector('.col-name');
            const nameRaw = nameCell ? nameCell.textContent.trim().replace(/\s+/g, ' ') : '';
            const name = nameRaw || path.split('/').pop().replace(/\.[^.]+$/, '');
            items.push({ path, name, format: fmt, size: '' });
        }
    }
    return items;
}

/**
 * Which list EOF autoplay advances through (`prefs.autoplayNextSource`).
 * @returns {'player' | 'samples'}
 */
function getAutoplayNextSource() {
    if (typeof prefs === 'undefined') return 'samples';
    return prefs.getItem('autoplayNextSource') === 'player' ? 'player' : 'samples';
}

window.getAutoplayNextSource = getAutoplayNextSource;

/** Whether autoplay-after-EOF may run (Settings on + non-empty chosen list). */
function canAutoplayAdvanceTrack() {
    if (typeof prefs === 'undefined' || prefs.getItem('autoplayNext') === 'off') return false;
    if (getAutoplayNextSource() === 'player') {
        return getPlayerHistoryListItems().length >= 1;
    }
    return getTablePlaybackListItems().length >= 1;
}

/**
 * Next path after `currentPath` in the same ordering as {@link nextTrack}.
 * @param {string} currentPath
 * @param { { autoplay?: boolean, respectAutoplaySource?: boolean } } [opts]
 * @returns {string | null}
 */
function getAutoplayNextPathAfter(currentPath, opts) {
    const o = opts || {};
    /** EOF autoplay (`autoplay`) or tray / menu-bar transport (`respectAutoplaySource`) use `autoplayNextSource`; floating-player buttons omit both.
     *  `sourceList` explicit override ('player' | 'samples') takes precedence — used by the tray popover's own
     *  independent `trayTransportSource` pref. */
    const useSourceList = o.autoplay === true || o.respectAutoplaySource === true;
    let items;
    if (o.sourceList === 'player') {
        items = getPlayerHistoryListItems();
    } else if (o.sourceList === 'samples') {
        items = getTablePlaybackListItems();
    } else if (useSourceList) {
        items = getAutoplayNextSource() === 'player' ? getPlayerHistoryListItems() : getTablePlaybackListItems();
    } else {
        items = getPlayerHistoryListItems();
    }
    if (items.length === 0) return null;
    if (audioShuffling) {
        return items[Math.floor(Math.random() * items.length)].path;
    }
    /* Resolve engine-playback resume path if the caller passed a null/empty `currentPath`.
     * `audioPlayerPath` is null during AudioEngine sessions and callers sometimes pass it
     * directly — fall back to `window._enginePlaybackResumePath` so `findIndex` can locate
     * the actual current track instead of returning -1 and wrapping to `items[0]`. */
    let effectiveCurrent = currentPath;
    if (!effectiveCurrent && _enginePlaybackActive && typeof window !== 'undefined') {
        const rp = window._enginePlaybackResumePath;
        if (typeof rp === 'string' && rp.length > 0) effectiveCurrent = rp;
    }
    const idx = items.findIndex(s => s.path === effectiveCurrent);
    if (idx < 0) {
        return items[0].path;
    }
    const nextIdx = (idx + 1) % items.length;
    return items[nextIdx].path;
}

/**
 * When autoplay-next is on, skip to the following sample after a failed preview (decode/play/unplayable).
 * Shows an error toast with the failure reason, then awaits **`previewAudio`** for the next path so callers
 * (including **`nextTrack`**) do not resolve before the chain finishes and play/pause state stays consistent.
 * @param {string} failedPath
 * @param {string | Error | undefined} [errDetail]
 * @returns {Promise<boolean>}
 */
async function tryPreviewAutoplayNextOnFailureAsync(failedPath, errDetail, previewOpts) {
    if (!canAutoplayAdvanceTrack()) return false;
    const nextPath = getAutoplayNextPathAfter(failedPath, { autoplay: true });
    if (!nextPath || nextPath === failedPath) return false;
    /** Same as {@link nextTrack}: keep expanded metadata under the playing row; only when this hop is the one that stuck (see chained failures below). */
    const hadExpanded = expandedMetaPath !== null;
    const chainOpts = previewOpts && typeof previewOpts === 'object' ? previewOpts : {};
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        const extRaw = (failedPath.split('.').pop() || '').toLowerCase();
        const ext = extRaw ? extRaw.toUpperCase() : '?';
        let errMsg;
        if (errDetail != null && errDetail !== '') {
            errMsg = typeof errDetail === 'string' ? errDetail : (errDetail.message || String(errDetail));
        } else {
            errMsg = typeof catalogFmt === 'function' ? catalogFmt('toast.unknown_error') : 'Unknown error';
        }
        showToast(toastFmt('toast.playback_failed_autoplay_next', { ext, err: errMsg }), 4000, 'error');
    }
    await previewAudio(nextPath, { skipRecentReorder: true, ...chainOpts });
    if (hadExpanded && audioPlayerPath === nextPath) {
        await expandMetaForPath(nextPath);
    }
    if (typeof window.syncAeTransportFromPlayback === 'function') {
        window.syncAeTransportFromPlayback();
    }
    return true;
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
/**
 * @param {string} filePath
 * @param {*} sample — from **`findByPath`**, or null for unknown paths
 * @param { { skipRecentReorder?: boolean } } [opts] — When set (autoplay / in-player list navigation), do not move the entry to the top — otherwise EOF **`nextTrack`** sees a reshuffled list and ping-pongs between the top two items.
 */
function addToRecentlyPlayed(filePath, sample, opts) {
    const skipReorder = opts && opts.skipRecentReorder === true;
    const entry = {
        path: filePath,
        name: sample ? sample.name : filePath.split('/').pop().replace(/\.[^.]+$/, ''),
        format: sample ? sample.format : filePath.split('.').pop().toUpperCase(),
        size: sample ? sample.sizeFormatted : '',
    };
    if (skipReorder) {
        const idx = recentlyPlayed.findIndex(r => r.path === filePath);
        if (idx >= 0) {
            recentlyPlayed[idx] = entry;
        } else {
            recentlyPlayed.push(entry);
            while (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.shift();
        }
    } else {
        recentlyPlayed = recentlyPlayed.filter(r => r.path !== filePath);
        recentlyPlayed.unshift(entry);
        if (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.length = MAX_RECENT;
    }
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
    const np = document.getElementById('audioNowPlaying');
    if (np) np.classList.remove('expanded');
    prefs.setItem('playerExpanded', 'off');
}

function hidePlayer() {
    const np = document.getElementById('audioNowPlaying');
    if (np) {
        prefs.setItem('playerExpanded', np.classList.contains('expanded') ? 'on' : 'off');
        np.classList.remove('active');
    }
    prefs.setItem('playerPaneHidden', 'on');
    // Hide player but keep audio playing
    const pill = document.getElementById('audioRestorePill');
    if (pill && audioPlayerPath && isAudioPlaying()) {
        pill.classList.add('active');
    }
}

function showPlayer() {
    const pill = document.getElementById('audioRestorePill');
    if (pill) pill.classList.remove('active');
    prefs.setItem('playerPaneHidden', 'off');
    const np = document.getElementById('audioNowPlaying');
    if (!np) return;
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

/**
 * After `prefs.load()`: if the user last hid the player pane, ensure it stays dismissed
 * (guards stray `.active` from devtools or partial init). Full visibility rules apply again
 * when `previewAudio` / video playback runs (reads `playerPaneHidden`).
 */
function restorePlayerPaneVisibilityFromPrefs() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return;
    if (prefs.getItem('playerPaneHidden') !== 'on') return;
    const np = document.getElementById('audioNowPlaying');
    if (!np) return;
    np.classList.remove('active');
    const pill = document.getElementById('audioRestorePill');
    if (pill) pill.classList.remove('active');
}

if (typeof window !== 'undefined') {
    window.restorePlayerPaneVisibilityFromPrefs = restorePlayerPaneVisibilityFromPrefs;
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
        previewAudio(item.dataset.path, { skipRecentReorder: true });
    }
});

// ── Previous / Next / Shuffle ──
/**
 * @param { { respectAutoplaySource?: boolean } } [opts] — Tray + menu bar: set true to use **`autoplayNextSource`** (player list vs Samples table). Floating player passes nothing (history list only).
 */
function prevTrack(opts) {
    const hadExpanded = expandedMetaPath !== null;
    const o = opts || {};
    /* `sourceList` explicit override ('player' | 'samples') takes precedence over
     * `respectAutoplaySource` + the shared `autoplayNextSource` pref. Used by the tray popover
     * so it has its own independent source setting (`trayTransportSource` pref) without
     * affecting EOF autoplay. */
    const useSourceList = o.respectAutoplaySource === true;
    const resolvedSource =
        o.sourceList === 'player' || o.sourceList === 'samples'
            ? o.sourceList
            : (useSourceList ? getAutoplayNextSource() : 'player');
    const items = resolvedSource === 'player'
        ? getPlayerHistoryListItems()
        : getTablePlaybackListItems();
    /* Resolve the effective current path — during AudioEngine playback, `audioPlayerPath` is
     * null and the real path lives in `window._enginePlaybackResumePath`. Using the null
     * value below made `items.findIndex(...)` return -1, which fell through to "wrap to last
     * item", so prev/next from the tray popover jumped to arbitrary tracks instead of stepping
     * relative to what's actually playing. Mirrors the same fix in `seekPlaybackToPercent`. */
    const resumePath =
        typeof window !== 'undefined' &&
        typeof window._enginePlaybackResumePath === 'string' &&
        window._enginePlaybackResumePath.length > 0
            ? window._enginePlaybackResumePath
            : '';
    const currentPath = audioPlayerPath || (_enginePlaybackActive ? resumePath : '');
    let prevPath = null;
    if (audioShuffling) {
        if (items.length === 0) return;
        prevPath = items[Math.floor(Math.random() * items.length)].path;
    } else {
        if (items.length === 0) return;
        const idx = items.findIndex(s => s.path === currentPath);
        if (idx < 0) {
            prevPath = items[items.length - 1].path;
        } else {
            const prevIdx = idx <= 0 ? items.length - 1 : idx - 1;
            prevPath = items[prevIdx].path;
        }
    }
    void (async () => {
        await previewAudio(prevPath, { skipRecentReorder: true });
        /* `previewAudio` may chain through `tryPreviewAutoplayNextOnFailureAsync` if `prevPath`
         * is unplayable — in that case the chain has already set `audioPlayerPath` (and
         * `expandedMetaPath`) to whatever it skipped to, so expanding the intended hop would
         * overwrite the correct expansion with the failed row. Only expand when the chain
         * actually landed on `prevPath`.
         * `opts.expand === true` forces expansion even when no row was previously expanded —
         * used by the tray popover so prev/next there runs the BPM/Key/LUFS analysis pipeline
         * (via `expandMetaForPath`) the same way a row click does in the main window. */
        if ((hadExpanded || o.expand === true) && audioPlayerPath === prevPath) {
            await expandMetaForPath(prevPath);
        }
    })();
}

/**
 * @param { { autoplay?: boolean, respectAutoplaySource?: boolean } } [opts] — **`autoplay`**: EOF / failure-chain — uses **`autoplayNextSource`**.
 * **`respectAutoplaySource`**: tray + menu bar — same. Omit both (floating player): **`getPlayerHistoryListItems`** only.
 */
function nextTrack(opts) {
    const hadExpanded = expandedMetaPath !== null;
    const o = opts || {};
    const nextPath = getAutoplayNextPathAfter(audioPlayerPath, {
        autoplay: o.autoplay === true,
        respectAutoplaySource: o.respectAutoplaySource === true,
        sourceList: o.sourceList,
    });
    if (!nextPath) return;
    void (async () => {
        await previewAudio(nextPath, { skipRecentReorder: true });
        /* `previewAudio` may chain through `tryPreviewAutoplayNextOnFailureAsync` if `nextPath`
         * is unplayable — in that case the chain has already set `audioPlayerPath` (and
         * `expandedMetaPath`) to whatever it skipped to, so expanding the intended hop would
         * overwrite the correct expansion with the failed row. Only expand when the chain
         * actually landed on `nextPath`.
         * `opts.expand === true` forces expansion even when no row was previously expanded —
         * tray popover passes it so BPM/Key/LUFS analysis runs via `expandMetaForPath` the
         * same way a row click in the main window does. */
        if ((hadExpanded || o.expand === true) && audioPlayerPath === nextPath) {
            await expandMetaForPath(nextPath);
        }
    })();
}

function toggleShuffle() {
    audioShuffling = !audioShuffling;
    prefs.setItem('shuffleMode', audioShuffling ? 'on' : 'off');
    const btn = document.getElementById('npBtnShuffle');
    if (btn) btn.classList.toggle('active', audioShuffling);
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

/** After tray popover toggles shuffle/loop in Rust (main webview may have been suspended).
 *
 * CRITICAL: do NOT call `syncTrayNowPlayingFromPlayback` here. Rust already owns the authoritative
 * shuffle/loop state (it was set directly inside `tray_popover_toggle_shuffle`/`_loop` before
 * this event fired), so the round-trip push is redundant. Worse, when the main window is
 * minimized on macOS, WebKit freezes `<audio>` element state updates to background windows —
 * `audioPlayer.currentTime` gets stuck at the value it held when the window lost visibility.
 * Pushing that stale elapsed back through `update_tray_now_playing` then re-emits
 * `tray-popover-state` to the popover with the stale value, and the popover's drift-rebase
 * yanks the progress thumb backward to the "last point where main app was visible" on every
 * shuffle/loop click. The tray popover gets its own lightweight `tray-popover-shuffle-loop`
 * event for the button highlights, so nothing here needs to drive it. */
function applyTrayPlaybackFlagsFromHost(shuffleOn, loopOn) {
    audioShuffling = !!shuffleOn;
    audioLooping = !!loopOn;
    audioPlayer.loop = audioLooping;
    if (typeof prefs !== 'undefined' && prefs.setItem) {
        prefs.setItem('shuffleMode', audioShuffling ? 'on' : 'off');
        prefs.setItem('audioLoop', audioLooping ? 'on' : 'off');
    }
    const shuffleBtn = document.getElementById('npBtnShuffle');
    const loopBtn = document.getElementById('npBtnLoop');
    if (shuffleBtn) shuffleBtn.classList.toggle('active', audioShuffling);
    if (loopBtn) loopBtn.classList.toggle('active', audioLooping);
    updateLoopBtnStates();
    if (_enginePlaybackActive && typeof window.syncEnginePlaybackLoop === 'function') {
        window.syncEnginePlaybackLoop(audioLooping);
    }
}

if (typeof window !== 'undefined') {
    window.applyTrayPlaybackFlagsFromHost = applyTrayPlaybackFlagsFromHost;
}

function toggleMute() {
    const btn = document.getElementById('npBtnMute');
    const slider = document.getElementById('npVolume');
    if (audioMuted) {
        const pct =
            typeof savedMuteVolumePct === 'number' && !Number.isNaN(savedMuteVolumePct)
                ? savedMuteVolumePct
                : Math.round((savedVolume || 0) * 100);
        const restored = Math.max(0, Math.min(100, pct));
        if (typeof setAudioVolume === 'function') {
            setAudioVolume(String(restored));
        }
        audioMuted = false;
        if (btn) btn.innerHTML = '&#128264;';
    } else {
        let prePct;
        if (slider) {
            const v = parseInt(String(slider.value), 10);
            if (Number.isFinite(v)) prePct = Math.max(0, Math.min(100, v));
        }
        if (prePct === undefined && typeof prefs !== 'undefined' && prefs.getItem) {
            const raw = parseInt(prefs.getItem('audioVolume') || '100', 10);
            prePct = Number.isNaN(raw) ? 100 : Math.max(0, Math.min(100, raw));
        }
        if (prePct === undefined && typeof audioPlayer !== 'undefined' && audioPlayer) {
            prePct = Math.max(0, Math.min(100, Math.round(audioPlayer.volume * 100)));
        }
        if (prePct === undefined) prePct = 100;
        savedMuteVolumePct = prePct;
        savedVolume = prePct / 100;
        if (typeof setAudioVolume === 'function') {
            setAudioVolume('0');
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

function _evictCache(cache) {
    const keys = Object.keys(cache);
    if (keys.length > _WF_CACHE_MAX) {
        for (const k of keys.slice(0, keys.length - _WF_CACHE_MAX)) delete cache[k];
    }
}

const _wfSqliteHydrateInflight = new Map();

function _isUsableWaveformPeaks(peaks) {
    if (!Array.isArray(peaks) || peaks.length === 0) return false;
    const first = peaks[0];
    if (typeof first === 'number' && Number.isFinite(first)) return true;
    if (first && typeof first === 'object' && Number.isFinite(first.min) && Number.isFinite(first.max)) return true;
    return false;
}

/** Fills `_waveformCache[path]` from SQLite on demand (`read_waveform_cache_entry`); no full-table load at startup. */
async function hydrateWaveformPeaksFromSqlite(filePath) {
    if (!filePath || typeof _waveformCache === 'undefined') return;
    if (_waveformCache[filePath]) return;
    let inflight = _wfSqliteHydrateInflight.get(filePath);
    if (inflight) return inflight;
    inflight = (async () => {
        try {
            const vu = window.vstUpdater;
            if (!vu || typeof vu.readWaveformCacheEntry !== 'function') return;
            const raw = await vu.readWaveformCacheEntry(filePath);
            if (!raw || !_isUsableWaveformPeaks(raw)) return;
            _waveformCache[filePath] = raw;
            _evictCache(_waveformCache);
        } catch {
            /* ignore */
        } finally {
            _wfSqliteHydrateInflight.delete(filePath);
        }
    })();
    _wfSqliteHydrateInflight.set(filePath, inflight);
    return inflight;
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

function _clearNpVideoSourceFlags() {
    const el = document.getElementById('npName');
    if (!el) return;
    el.dataset.videoSource = '';
    delete el.dataset.videoSizeBytes;
}

async function drawWaveform(filePath, wfSeq) {
    const canvas = document.getElementById('npWaveformCanvas');
    if (!canvas) return;
    if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
    const container = canvas.parentElement;
    const npNameEl = document.getElementById('npName');
    const isVideoNowPlaying = !!(npNameEl && npNameEl.dataset.videoSource === 'true');

    await hydrateWaveformPeaksFromSqlite(filePath);
    if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;

    if (_waveformCache[filePath]) {
        const { w: cw, h: ch } = await resolveWaveformBoxSize(container, 280, 24);
        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.max(1, Math.round(cw * dpr));
        canvas.height = Math.max(1, Math.round(ch * dpr));
        const ctx = canvas.getContext('2d');
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        renderWaveformData(ctx, canvas, _waveformCache[filePath]);
        return;
    }

    let cwHintNp = 280;
    if (container) {
        try {
            const r = container.getBoundingClientRect();
            if (r.width >= 2) cwHintNp = Math.min(2000, Math.round(r.width));
            else if (container.clientWidth >= 2) cwHintNp = Math.min(2000, container.clientWidth);
        } catch {
            /* ignore */
        }
    }
    const barsEarlyVideo = Math.max(1, Math.min(Math.max(1, Math.floor(cwHintNp)), 800));
    const videoEnginePromise =
        isVideoNowPlaying && typeof fetchWaveformPreviewFromEngine === 'function'
            ? fetchWaveformPreviewFromEngine(filePath, barsEarlyVideo)
            : null;

    const { w: cw, h: ch } = await resolveWaveformBoxSize(container, 280, 24);
    if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(cw * dpr));
    canvas.height = Math.max(1, Math.round(ch * dpr));
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const bars = Math.max(1, Math.min(Math.max(1, Math.floor(cw)), 800));
    const src = fileSrcForDecode(filePath);
    try {
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (_npWaveformDrawStale(wfSeq) || filePath !== audioPlayerPath) return;
        let peaks = null;
        if (isVideoNowPlaying) {
            // Video: never decodePeaksViaWorker — it would fetch the entire container (multi‑GB UI freeze).
            // Host transcodes before JUCE (bounded extract) — call engine for any file size.
            // `fetchWaveformPreviewFromEngine` is started before `resolveWaveformBoxSize` so MP4 IPC overlaps layout waits.
            try {
                peaks = videoEnginePromise ? await videoEnginePromise : null;
            } catch {
                peaks = null;
            }
        } else {
            try {
                peaks = await decodePeaksViaWorker(src, bars);
            } catch {
                peaks = null;
            }
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
        storeWaveformPeaksInCache(filePath, peaks);
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

    await hydrateWaveformPeaksFromSqlite(filePath);
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
            storeWaveformPeaksInCache(filePath, peaks);
            storeSpectrogramDataInCache(filePath, sgData);
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
            storeSpectrogramDataInCache(filePath, sgData);
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
            storeWaveformPeaksInCache(filePath, peaks);
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
    await hydrateWaveformPeaksFromSqlite(filePath);
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
        storeWaveformPeaksInCache(filePath, peaks);
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
        storeSpectrogramDataInCache(filePath, sgData);
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
    el.textContent = npMetaLineTextForPath(audioPlayerPath);
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
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

    let _eqSpectrumBufNp = null;
    let _eqSpectrumBufAe = null;

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

    const FREQ_MIN = 20, FREQ_MAX = NP_SPECTRUM_DISPLAY_MAX_HZ;
    const GAIN_MIN = -12, GAIN_MAX = 12;
    /** Log-spaced samples for Ableton-style smooth spectrum (not one vertex per FFT bin). */
    const EQ_SPECTRUM_POINTS = 512;
    const EQ_MARGIN_BOTTOM = 22;

    /**
     * Interpolate spectrum magnitude0–1. `omitDc`: AudioEngine `spectrum[i]` is FFT bin i+1; WebAudio
     * `getByteFrequencyData` is DC-inclusive (bin Hz i·sr/fftSize).
     */
    function sampleSpectrumMag01(freqHz, dataArr, bufLen, sampleRate, fftSize, omitDc) {
        const binF = omitDc
            ? engineSpectrumBundledBinF(freqHz, sampleRate, fftSize, bufLen)
            : (freqHz * fftSize) / sampleRate;
        if (bufLen < 1) return 0;
        if (bufLen === 1) return dataArr[0] / 255;
        if (binF <= 0) return dataArr[0] / 255;
        if (binF >= bufLen - 1) return dataArr[bufLen - 1] / 255;
        const i0 = Math.floor(binF);
        const i1 = Math.min(bufLen - 1, i0 + 1);
        const frac = binF - i0;
        const v0 = dataArr[i0] / 255;
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

    /** Reused for `getFrequencyResponse` — was 6× `new Float32Array(480)` per frame at 60fps (GC + CPU). */
    const EQ_FREQ_RESPONSE_POINTS = 480;
    let _eqRespFreqs = null;
    let _eqRespMagLow = null;
    let _eqRespPhaseLow = null;
    let _eqRespMagMid = null;
    let _eqRespPhaseMid = null;
    let _eqRespMagHigh = null;
    let _eqRespPhaseHigh = null;

    function ensureEqFreqResponseBuffers() {
        const n = EQ_FREQ_RESPONSE_POINTS;
        if (_eqRespFreqs && _eqRespFreqs.length === n) return;
        _eqRespFreqs = new Float32Array(n);
        for (let i = 0; i < n; i++) {
            _eqRespFreqs[i] = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, i / (n - 1));
        }
        _eqRespMagLow = new Float32Array(n);
        _eqRespPhaseLow = new Float32Array(n);
        _eqRespMagMid = new Float32Array(n);
        _eqRespPhaseMid = new Float32Array(n);
        _eqRespMagHigh = new Float32Array(n);
        _eqRespPhaseHigh = new Float32Array(n);
    }

    /** Keep parametric EQ rAF only while spectrum animates, audio plays, or the user drags a band — not 24/7 when the panel is open. */
    function needsParametricEqRafContinuation() {
        if (_dragState) return true;
        return typeof parametricEqNeedsLiveSpectrumAnimation === 'function' && parametricEqNeedsLiveSpectrumAnimation();
    }

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
    function drawEqSpectrumFill(ctx, w, h, dataArr, bufLen, sampleRate, fftSize, omitDc) {
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
            const mag = sampleSpectrumMag01(freq, dataArr, bufLen, sampleRate, fftSize, omitDc);
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
            const mag = sampleSpectrumMag01(freq, dataArr, bufLen, sampleRate, fftSize, omitDc);
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
        const eqFreezeId = canvas === aeCanvas ? 'ae:eq' : 'np:eq';
        const eqSpectrumFrozen = typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(eqFreezeId);
        let _eqSpectrumBuf = canvas === aeCanvas ? _eqSpectrumBufAe : _eqSpectrumBufNp;
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
                const bufLen = dataArr.length;
                if (!eqSpectrumFrozen) {
                    if (!_eqSpectrumBuf || _eqSpectrumBuf.length !== bufLen) _eqSpectrumBuf = new Uint8Array(bufLen);
                    _eqSpectrumBuf.set(dataArr.subarray(0, bufLen));
                }
                if (canvas === aeCanvas) _eqSpectrumBufAe = _eqSpectrumBuf;
                else _eqSpectrumBufNp = _eqSpectrumBuf;
                if (_eqSpectrumBuf && _eqSpectrumBuf.length === bufLen) {
                    const sampleRate = axis.sr;
                    const fftSize = axis.fft;
                    drawEqSpectrumFill(ctx, w, h, _eqSpectrumBuf, bufLen, sampleRate, fftSize, true);
                }
                }
            }
        } else if (_analyser && _playbackCtx && typeof isAudioPlaying === 'function' && isAudioPlaying()) {
            const bufLen = _analyser.frequencyBinCount;
            if (!eqSpectrumFrozen) {
                if (!_eqSpectrumBuf || _eqSpectrumBuf.length !== bufLen) _eqSpectrumBuf = new Uint8Array(bufLen);
                _analyser.getByteFrequencyData(_eqSpectrumBuf);
            }
            if (canvas === aeCanvas) _eqSpectrumBufAe = _eqSpectrumBuf;
            else _eqSpectrumBufNp = _eqSpectrumBuf;
            if (_eqSpectrumBuf && _eqSpectrumBuf.length === bufLen) {
                const liveSr = _playbackCtx.sampleRate;
                const liveFft = _analyser.fftSize;
                if (
                    _npFftWebAxisSrHz == null ||
                    _npFftWebAxisFftSize == null ||
                    _npFftWebAxisSrHz !== liveSr ||
                    _npFftWebAxisFftSize !== liveFft
                ) {
                    _npFftWebAxisSrHz = liveSr;
                    _npFftWebAxisFftSize = liveFft;
                }
                const sampleRate = _npFftWebAxisSrHz;
                const fftSizeForBins = _npFftWebAxisFftSize;
                drawEqSpectrumFill(ctx, w, h, _eqSpectrumBuf, bufLen, sampleRate, fftSizeForBins, false);
            }
        }

        if (_eqLow && _eqMid && _eqHigh) {
            ensureEqFreqResponseBuffers();
            const nPoints = EQ_FREQ_RESPONSE_POINTS;
            const freqs = _eqRespFreqs;
            _eqLow.getFrequencyResponse(freqs, _eqRespMagLow, _eqRespPhaseLow);
            _eqMid.getFrequencyResponse(freqs, _eqRespMagMid, _eqRespPhaseMid);
            _eqHigh.getFrequencyResponse(freqs, _eqRespMagHigh, _eqRespPhaseHigh);

            ctx.beginPath();
            ctx.strokeStyle = 'rgba(5,217,232,0.6)';
            ctx.lineWidth = 2;
            for (let i = 0; i < nPoints; i++) {
                const totalDb = 20 * Math.log10(_eqRespMagLow[i] * _eqRespMagMid[i] * _eqRespMagHigh[i]);
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
        if (needsParametricEqRafContinuation()) {
            _paramEqRafId = requestAnimationFrame(parametricEqTick);
        }
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

    if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
        document.addEventListener('graph-freeze-changed', () => {
            scheduleParametricEqFrame();
            if (typeof window.ensureEnginePlaybackFftRaf === 'function') window.ensureEnginePlaybackFftRaf();
        });
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
window.hydrateWaveformPeaksFromSqlite = hydrateWaveformPeaksFromSqlite;

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

/** Resolve the loop-region path for a waveform container (meta uses dataset.path, np follows audioPlayerPath). */
function _loopRegionPathForBox(box) {
    if (!box) return '';
    if (box.id === 'metaWaveformBox') return box.dataset ? box.dataset.path || '' : '';
    if (box.id === 'npWaveform') return audioPlayerPath || '';
    if (box.id === 'videoWaveformBox') {
        const fromDs = box.dataset ? box.dataset.path || '' : '';
        if (fromDs) return fromDs;
        return (typeof videoPlayerPath !== 'undefined' && videoPlayerPath) ? videoPlayerPath : '';
    }
    return '';
}

/** Now-playing + expanded-row waveforms: pointerdown seeks (click delegation can miss in some WebViews; canvas is pointer-events:none). */
(function initWaveformPointerSeek() {
    function maybeExitLoopOnRightClickFrac(box, e) {
        const filePath = _loopRegionPathForBox(box);
        if (!filePath) return;
        const region = getSampleLoopRegion(filePath);
        if (!region.enabled) return;
        const rect = box.getBoundingClientRect();
        if (rect.width <= 0) return;
        const frac = (e.clientX - rect.left) / rect.width;
        if (frac > region.endFrac + 0.001) {
            region.enabled = false;
            setSampleLoopRegion(filePath, region);
            applyMetaLoopRegionUI(filePath);
            syncAbLoopFromSampleRegion(filePath);
        }
    }
    function onPointerDown(e) {
        if (e.button !== 0) return;
        const t = e.target;
        if (!t || (t.closest && t.closest('button, input, select, textarea'))) return;
        // Loop brace handles consume their own pointerdown (drag); don't treat as seek.
        if (t.closest && t.closest('.waveform-loop-brace')) return;
        // Shift+click on a loop-capable waveform starts the region paint (rubber band); skip seek.
        if (e.shiftKey && t.closest && t.closest('#metaWaveformBox, #npWaveform, #videoWaveformBox')) return;
        const meta = typeof t.closest === 'function' ? t.closest('#metaWaveformBox') : null;
        if (meta && typeof seekMetaWaveform === 'function') {
            // Click to the right of the loop end brace exits the loop region — lets playback continue past `end`.
            maybeExitLoopOnRightClickFrac(meta, e);
            e.preventDefault();
            seekMetaWaveform(e);
            return;
        }
        const np = typeof t.closest === 'function' ? t.closest('#npWaveform') : null;
        if (np && typeof seekAudio === 'function') {
            maybeExitLoopOnRightClickFrac(np, e);
            e.preventDefault();
            seekAudio(e);
            return;
        }
        // Video waveform seek
        const videoWf = typeof t.closest === 'function' ? t.closest('#videoWaveformBox') : null;
        if (videoWf && typeof seekVideoWaveform === 'function') {
            maybeExitLoopOnRightClickFrac(videoWf, e);
            e.preventDefault();
            seekVideoWaveform(e);
        }
    }
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('pointerdown', onPointerDown, true);
})();

/** Shift+drag on the expanded-row OR now-playing waveform paints a new loop region (rubber-band). */
(function initMetaLoopPaintDrag() {
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    let paint = null; // { box, filePath, rect, anchorFrac, pointerId }
    function onPointerDown(e) {
        if (e.button !== 0) return;
        if (!e.shiftKey) return;
        const t = e.target;
        if (!t || typeof t.closest !== 'function') return;
        // Don't start paint when the user shift-clicks a brace or the toggle button.
        if (t.closest('.waveform-loop-brace')) return;
        if (t.closest('button, input, select, textarea')) return;
        const box = t.closest('#metaWaveformBox, #npWaveform, #videoWaveformBox');        if (!box) return;
        const filePath = _loopRegionPathForBox(box);
        if (!filePath) return;
        const rect = box.getBoundingClientRect();
        if (rect.width <= 0) return;
        e.preventDefault();
        e.stopPropagation();
        const anchorFrac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        paint = {
            box,
            filePath,
            rect,
            anchorFrac,
            pointerId: e.pointerId,
        };
        try { box.setPointerCapture(e.pointerId); } catch {}
        // Seed a zero-width region at the anchor so the highlight band appears on first move.
        const region = getSampleLoopRegion(paint.filePath);
        region.enabled = true;
        region.startFrac = anchorFrac;
        region.endFrac = Math.min(1, anchorFrac + 0.005);
        setSampleLoopRegion(paint.filePath, region);
        applyMetaLoopRegionUI(paint.filePath);
        syncAbLoopFromSampleRegion(paint.filePath);
        document.addEventListener('pointermove', onPointerMove, true);
        document.addEventListener('pointerup', onPointerUp, true);
        document.addEventListener('pointercancel', onPointerUp, true);
    }
    function onPointerMove(e) {
        if (!paint) return;
        const { filePath, rect, anchorFrac } = paint;
        let frac = (e.clientX - rect.left) / rect.width;
        if (!Number.isFinite(frac)) return;
        frac = Math.max(0, Math.min(1, frac));
        const region = getSampleLoopRegion(filePath);
        const MIN_GAP = 0.005;
        if (frac >= anchorFrac) {
            region.startFrac = anchorFrac;
            region.endFrac = Math.max(frac, anchorFrac + MIN_GAP);
        } else {
            region.startFrac = frac;
            region.endFrac = Math.max(anchorFrac, frac + MIN_GAP);
        }
        region.enabled = true;
        setSampleLoopRegion(filePath, region);
        applyMetaLoopRegionUI(filePath);
        syncAbLoopFromSampleRegion(filePath);
    }
    function onPointerUp() {
        if (paint) {
            try { paint.box.releasePointerCapture(paint.pointerId); } catch {}
            syncAbLoopFromSampleRegion(paint.filePath);
        }
        paint = null;
        document.removeEventListener('pointermove', onPointerMove, true);
        document.removeEventListener('pointerup', onPointerUp, true);
        document.removeEventListener('pointercancel', onPointerUp, true);
    }
    document.addEventListener('pointerdown', onPointerDown, true);
})();

/** Drag loop braces on the expanded-row OR now-playing waveform. Updates per-sample region + live `_abLoop` when playing. */
(function initMetaLoopBraceDrag() {
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    let dragging = null; // { kind, box, filePath, rect, pointerId, handle }
    function onPointerDown(e) {
        if (e.button !== 0) return;
        const t = e.target;
        if (!t || typeof t.closest !== 'function') return;
        const handle = t.closest('.waveform-loop-brace');
        if (!handle) return;
        const box = handle.closest('#metaWaveformBox, #npWaveform, #videoWaveformBox');        if (!box) return;
        const kind = handle.dataset.loopBrace;
        if (kind !== 'start' && kind !== 'end') return;
        const filePath = _loopRegionPathForBox(box);
        if (!filePath) return;
        e.preventDefault();
        e.stopPropagation();
        const rect = box.getBoundingClientRect();
        if (rect.width <= 0) return;
        dragging = {
            kind,
            box,
            filePath,
            rect,
            pointerId: e.pointerId,
            handle,
        };
        try { handle.setPointerCapture(e.pointerId); } catch {}
        document.addEventListener('pointermove', onPointerMove, true);
        document.addEventListener('pointerup', onPointerUp, true);
        document.addEventListener('pointercancel', onPointerUp, true);
    }
    function onPointerMove(e) {
        if (!dragging) return;
        const { kind, filePath, rect } = dragging;
        let frac = (e.clientX - rect.left) / rect.width;
        if (!Number.isFinite(frac)) return;
        frac = Math.max(0, Math.min(1, frac));
        const region = getSampleLoopRegion(filePath);
        const MIN_GAP = 0.005;
        if (kind === 'start') {
            region.startFrac = Math.min(frac, region.endFrac - MIN_GAP);
        } else {
            region.endFrac = Math.max(frac, region.startFrac + MIN_GAP);
        }
        // Dragging implies the user wants the region visible/active.
        region.enabled = true;
        setSampleLoopRegion(filePath, region);
        applyMetaLoopRegionUI(filePath);
        syncAbLoopFromSampleRegion(filePath);
    }
    function onPointerUp() {
        if (dragging) {
            try { dragging.handle.releasePointerCapture(dragging.pointerId); } catch {}
        }
        dragging = null;
        document.removeEventListener('pointermove', onPointerMove, true);
        document.removeEventListener('pointerup', onPointerUp, true);
        document.removeEventListener('pointercancel', onPointerUp, true);
    }
    document.addEventListener('pointerdown', onPointerDown, true);
})();

/** Click on the `L` toggle on meta / video expanded waveform (NP bar has no L button). */
(function initMetaLoopToggleClick() {
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('click', (e) => {
        const t = e.target;
        if (!t || typeof t.closest !== 'function') return;
        const btn = t.closest('.waveform-loop-toggle');
        if (!btn) return;
        const host = btn.closest('#metaWaveformBox, #videoWaveformBox');
        if (!host) return;
        e.preventDefault();
        e.stopPropagation();
        if (host.id === 'videoWaveformBox') {
            if (typeof toggleVideoLoopRegionFn === 'function') toggleVideoLoopRegionFn();
        } else {
            toggleMetaLoopRegion();
        }
    }, true);
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

/** After `__appStr` loads, align idle tray tooltip with localized `tray.tooltip`. */
(function syncTrayAfterAppStrings() {
    if (typeof window === 'undefined') return;
    const run = () => {
        if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
    };
    const p = window.__appReady;
    if (p && typeof p.then === 'function') {
        void p.then(run).catch(() => {});
    } else {
        run();
    }
})();
