// ── Audio Engine tab: separate `audio-engine` process (JUCE devices + playback; VST3/AU scan; insert chain + native editor windows) ──

const AE_PREFS_DEVICE = 'audioEngineOutputDeviceId';
const AE_PREFS_INPUT_DEVICE = 'audioEngineInputDeviceId';
const AE_PREFS_DEVICE_TYPE = 'audioEngineJuceDeviceType';
const AE_PREFS_SAMPLE_RATE_HZ = 'audioEngineSampleRateHz';
const AE_PREFS_TONE = 'audioEngineTestTone';
const AE_PREFS_BUFFER_FRAMES_OUTPUT = 'audioEngineBufferFramesOutput';
const AE_PREFS_BUFFER_FRAMES_INPUT = 'audioEngineBufferFramesInput';
/** JSON array of insert host paths for `playback_set_inserts` (UI mirror). */
const AE_PREFS_INSERT_PATHS_JSON = 'audioEngineInsertPathsJson';
/** @deprecated Legacy single pref; migrated once to output/input */
const AE_LEGACY_BUFFER_FRAMES = 'audioEngineBufferFrames';
/** @deprecated Migrated once into `AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER` + `AE_PREFS_PLAYBACK_SPECTRUM_BANDS`. */
const AE_PREFS_PLAYBACK_SPECTRUM_QUALITY = 'audioEnginePlaybackSpectrumQuality';
/** `playback_status` FFT order8–15 (`off` = skip FFT). */
const AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER = 'audioEnginePlaybackSpectrumFftOrder';
/** Output band count: integer string or `max` (engine clamps to Nyquist; sub-Nyquist values are max-binned across the full range). */
const AE_PREFS_PLAYBACK_SPECTRUM_BANDS = 'audioEnginePlaybackSpectrumBands';
/** Lower clamp for `playback_status.spectrum` length; upper bound is FFT Nyquist (see `Engine::appendPlaybackSpectrumJson`, min 64). */
const ENGINE_PLAYBACK_SPECTRUM_MIN_BINS = 64;

/** Legacy single-dropdown presets → [`spectrum_fft_order`, `spectrum_bins`] (read only by `migrateAePlaybackSpectrumSplitPrefs`). */
const AE_LEGACY_PLAYBACK_SPECTRUM_PRESETS = Object.freeze({
    'pt32k-max': [15, 16383],
    'pt32k-8k': [15, 8192],
    'pt32k-4k': [15, 4096],
    'pt32k-2k': [15, 2048],
    'pt32k-1k': [15, 1024],
    'pt32k-512': [15, 512],
    'pt32k-256': [15, 256],
    'pt16k-max': [14, 8191],
    'pt16k-4k': [14, 4096],
    'pt16k-2k': [14, 2048],
    'pt16k-1k': [14, 1024],
    'pt16k-512': [14, 512],
    'pt8k-max': [13, 4095],
    'pt8k-2k': [13, 2048],
    'pt8k-1k': [13, 1024],
    'pt8k-512': [13, 512],
    maximum: [12, 2047],
    'very-high': [12, 1536],
    'very-high-1k': [12, 1024],
    high: [11, 1024],
    'high-balanced': [11, 896],
    'medium-high': [11, 768],
    'high-512': [11, 512],
    medium: [10, 512],
    'medium-plus': [10, 384],
    'medium-256': [10, 256],
    low: [9, 256],
    'low-128': [9, 128],
    minimal: [8, 127],
    'minimal-64': [8, 64],
});

/** UI / pref normalization: band counts offered in `#aePlaybackSpectrumBands` (ascending). */
const AE_PLAYBACK_SPECTRUM_BAND_STEPS = Object.freeze([
    64, 128, 256, 384, 512, 768, 896, 1024, 1536, 2048, 4096, 8192, 16384,
]);

function aeMaxSpectrumBinsForFftOrder(ord) {
    const o = typeof ord === 'string' ? parseInt(ord, 10) : ord;
    if (!Number.isFinite(o)) {
        return ENGINE_PLAYBACK_SPECTRUM_MIN_BINS;
    }
    const clamped = Math.max(8, Math.min(15, o));
    const fftSize = 1 << clamped;
    return Math.max(ENGINE_PLAYBACK_SPECTRUM_MIN_BINS, (fftSize / 2) | 0);
}

function normalizeAePlaybackSpectrumFftOrderPref(v) {
    const s = v != null ? String(v).trim() : '';
    if (s === 'off') {
        return 'off';
    }
    const o = parseInt(s, 10);
    if (Number.isFinite(o) && o >= 8 && o <= 15) {
        return String(o);
    }
    return '11';
}

function aeParseSpectrumBinsPref(s, maxBins) {
    const t = s != null ? String(s).trim().toLowerCase() : '';
    if (t === 'max') {
        return maxBins;
    }
    if (t === '') {
        return Math.min(1024, maxBins);
    }
    const n = parseInt(t, 10);
    if (!Number.isFinite(n)) {
        return Math.min(1024, maxBins);
    }
    return Math.max(ENGINE_PLAYBACK_SPECTRUM_MIN_BINS, Math.min(maxBins, n));
}

/** Picks a valid `#aePlaybackSpectrumBands` value at or below `maxBins`. */
function aeNormalizeBandsSelectValue(s, maxBins) {
    const t = s != null ? String(s).trim().toLowerCase() : '';
    if (t === 'max') {
        return 'max';
    }
    const n = aeParseSpectrumBinsPref(s, maxBins);
    if (n >= maxBins) {
        return 'max';
    }
    for (let i = AE_PLAYBACK_SPECTRUM_BAND_STEPS.length - 1; i >= 0; i--) {
        const step = AE_PLAYBACK_SPECTRUM_BAND_STEPS[i];
        if (step <= maxBins && step <= n) {
            return String(step);
        }
    }
    return String(ENGINE_PLAYBACK_SPECTRUM_MIN_BINS);
}

/** English when SQLite `app_i18n` has not been re-seeded yet (`catalogFmt` returns the key). */
const AE_PLAYBACK_SPECTRUM_CAP_HINT_OFF_EN = 'FFT off — playback_status omits spectrum.';
const AE_PLAYBACK_SPECTRUM_CAP_HINT_EN = 'At most {max} bands for this FFT size (Nyquist = half the window in points).';

function aePlaybackSpectrumCapHintText(off, maxB) {
    const keyOff = 'ui.ae.playback_spectrum_cap_hint_off';
    const keyOn = 'ui.ae.playback_spectrum_cap_hint';
    if (off) {
        let s = typeof catalogFmt === 'function' ? catalogFmt(keyOff) : keyOff;
        if (s === keyOff) {
            s = AE_PLAYBACK_SPECTRUM_CAP_HINT_OFF_EN;
        }
        return s;
    }
    let s = typeof catalogFmt === 'function' ? catalogFmt(keyOn, {max: String(maxB)}) : keyOn;
    if (s === keyOn) {
        s = AE_PLAYBACK_SPECTRUM_CAP_HINT_EN.replace(/\{max\}/g, String(maxB));
    }
    return s;
}

function aeUpdatePlaybackSpectrumBandsUiState(fftEl, bandsEl) {
    if (!fftEl || !bandsEl) {
        return;
    }
    const hintEl = document.getElementById('aePlaybackSpectrumCapHint');
    const off = fftEl.value === 'off';
    bandsEl.disabled = off;
    if (off) {
        if (hintEl) {
            hintEl.textContent = aePlaybackSpectrumCapHintText(true, 0);
        }
        return;
    }
    const maxB = aeMaxSpectrumBinsForFftOrder(fftEl.value);
    if (hintEl) {
        hintEl.textContent = aePlaybackSpectrumCapHintText(false, maxB);
    }
    for (let i = 0; i < bandsEl.options.length; i++) {
        const opt = bandsEl.options[i];
        const v = opt.value;
        if (v === 'max') {
            opt.disabled = false;
            continue;
        }
        const n = parseInt(v, 10);
        opt.disabled = Number.isFinite(n) && n > maxB;
    }
    const sel = bandsEl.selectedOptions[0];
    if (sel && sel.disabled) {
        bandsEl.value = aeNormalizeBandsSelectValue(bandsEl.value, maxB);
    }
}

function migrateAePlaybackSpectrumSplitPrefs() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function' || typeof prefs.setItem !== 'function') {
        return;
    }
    const newFft = prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER);
    if (newFft != null && String(newFft) !== '') {
        return;
    }
    const leg = prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_QUALITY);
    if (leg != null && String(leg) !== '') {
        const q = String(leg);
        if (q === 'off') {
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER, 'off');
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS, '1024');
        } else if (AE_LEGACY_PLAYBACK_SPECTRUM_PRESETS[q]) {
            const pair = AE_LEGACY_PLAYBACK_SPECTRUM_PRESETS[q];
            const ord = pair[0];
            const bins = pair[1];
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER, String(ord));
            const maxB = aeMaxSpectrumBinsForFftOrder(ord);
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS, bins >= maxB ? 'max' : String(bins));
        } else {
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER, '11');
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS, '1024');
        }
        prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_QUALITY, '');
        return;
    }
    prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER, '11');
    prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS, '1024');
}

/** Live plugin list from the last `plugin_chain` response (populated by `aePopulateInsertSlotSelects`). */
let aePluginCatalog = [];
/** Last `plugin_chain` payload (for re-filtering when “show instruments” toggles). */
let aeLastPluginChain = null;

/** Active picker instances (one per insert row in the UI). */
let aeInsertPickers = [];

/** After first successful `list_audio_device_types`, restore saved JUCE driver from prefs when safe (and again after AudioEngine restart). */
let aeInitialDeviceTypeRestored = false;

/** Incremented at the start of each `refreshAudioEnginePanel` so in-flight plugin-scan polls do not apply stale results. */
let aePluginChainPollGeneration = 0;

/** Poll AudioEngine subprocess stats (same cadence as main header `updateHeaderInfo`). */
let aeProcStatsInterval = null;

/** One live toast for plugin scan (`#aePluginScanProgressToast`); updated each poll, not stacked. */
const AE_PLUGIN_SCAN_PROGRESS_TOAST_ID = 'aePluginScanProgressToast';
/** When `scan_done` / current plug-in / skipped unchanged, we still show elapsed seconds on the toast. */
let aeScanProgressToastKey = '';
let aeScanProgressToastKeyAt = 0;

function dismissAePluginScanProgressToast() {
    const el = document.getElementById(AE_PLUGIN_SCAN_PROGRESS_TOAST_ID);
    if (el) el.remove();
}

/**
 * Single non-expiring info toast for scan progress; text updated in place (avoids duplicate identical toasts).
 * @param {string} message — already localized (e.g. `toastFmt('toast.ae_plugin_scan_progress', { line })`)
 */
function ensureAePluginScanProgressToast(message) {
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) {
        dismissAePluginScanProgressToast();
        return;
    }
    const container = document.getElementById('toastContainer');
    if (!container) return;
    let el = document.getElementById(AE_PLUGIN_SCAN_PROGRESS_TOAST_ID);
    if (!el) {
        el = document.createElement('div');
        el.id = AE_PLUGIN_SCAN_PROGRESS_TOAST_ID;
        el.className = 'toast toast-info';
        el.style.animation = 'toast-in 0.3s ease-out forwards';
        container.appendChild(el);
    }
    el.textContent = message;
}

/**
 * JSON body for `playback_status` — merges user quality pref with optional future overrides.
 * @returns {{ cmd: 'playback_status', spectrum?: boolean, spectrum_fft_order?: number, spectrum_bins?: number }}
 */
function buildEnginePlaybackStatusRequest() {
    const out = {cmd: 'playback_status', scope: true, scope_samples: 1024};
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') {
        return out;
    }
    migrateAePlaybackSpectrumSplitPrefs();
    const ord = normalizeAePlaybackSpectrumFftOrderPref(prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER));
    if (ord === 'off') {
        out.spectrum = false;
        return out;
    }
    const o = parseInt(ord, 10);
    const maxB = aeMaxSpectrumBinsForFftOrder(o);
    const bins = aeParseSpectrumBinsPref(prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS), maxB);
    out.spectrum = true;
    out.spectrum_fft_order = o;
    out.spectrum_bins = bins;
    return out;
}

function migrateAeBufferPrefs() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function' || typeof prefs.setItem !== 'function') {
        return;
    }
    const leg = prefs.getItem(AE_LEGACY_BUFFER_FRAMES);
    if (leg == null || String(leg) === '') return;
    const out = prefs.getItem(AE_PREFS_BUFFER_FRAMES_OUTPUT);
    const inp = prefs.getItem(AE_PREFS_BUFFER_FRAMES_INPUT);
    if (out == null || String(out) === '') {
        prefs.setItem(AE_PREFS_BUFFER_FRAMES_OUTPUT, String(leg));
    }
    if (inp == null || String(inp) === '') {
        prefs.setItem(AE_PREFS_BUFFER_FRAMES_INPUT, String(leg));
    }
}

/**
 * Preload bridge: `audioEngineInvoke` may be absent before shell ready or outside Tauri.
 * @returns {function|null}
 */
function getAeAudioEngineInvoke() {
    const u = typeof window !== 'undefined' ? window.vstUpdater : undefined;
    return u && typeof u.audioEngineInvoke === 'function' ? u.audioEngineInvoke : null;
}

function aeStartProcessStatsPollingOnce() {
    if (aeProcStatsInterval != null) return;
    aeProcStatsInterval = setInterval(() => {
        if (!aeAudioEngineTabIsActive()) return;
        if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return;
        void refreshAeProcessStats();
    }, 3000);
}

/**
 * Fetches `get_audio_engine_process_stats` (RSS/VIRT/sysinfo CPU/threads/FDs/uptime) and fills `#aeProcessStats`.
 */
async function refreshAeProcessStats() {
    const u = typeof window !== 'undefined' && window.vstUpdater ? window.vstUpdater : null;
    if (!u || typeof u.getAudioEngineProcessStats !== 'function') return;
    const inactive = document.getElementById('aeProcessStatsInactive');
    const strip = document.getElementById('aeProcessStats');
    if (!strip) return;
    try {
        const s = await u.getAudioEngineProcessStats();
        if (!s || typeof s !== 'object') return;
        const set = (id, val) => {
            const el = document.getElementById(id);
            if (el) el.textContent = val;
        };
        if (!s.running || !s.pid) {
            if (inactive) {
                inactive.style.display = '';
                inactive.textContent =
                    typeof catalogFmt === 'function'
                        ? catalogFmt('ui.ae.process_stats_not_running')
                        : 'AudioEngine not running.';
            }
            strip.style.display = 'none';
            return;
        }
        if (inactive) inactive.style.display = 'none';
        strip.style.display = '';
        set('aeStatCores', s.numCpus != null ? String(s.numCpus) : '?');
        set('aeStatCpu', (Number(s.cpuPercent) || 0).toFixed(1) + '%');
        const fmtB = typeof formatBytes === 'function' ? formatBytes : (b) => String(b);
        set('aeStatRss', fmtB(s.rssBytes || 0));
        set('aeStatVirt', fmtB(s.virtualBytes || 0));
        set('aeStatThr', s.threads != null ? String(s.threads) : '—');
        set('aeStatFd', s.openFds != null ? String(s.openFds) : '—');
        const fmtU = typeof formatUptime === 'function' ? formatUptime : (sec) => String(sec) + 's';
        set('aeStatUp', fmtU(s.uptimeSecs || 0));
        set('aeStatPid', s.pid != null ? String(s.pid) : '—');
    } catch (_) {
        /* ignore */
    }
}

/** @type {ReturnType<typeof setInterval> | null} */
let aeInputPeakPollTimer = null;
let aeInputPeakPollInFlight = false;
let aeInputPeakVisibilityBound = false;
const AE_INPUT_PEAK_POLL_MS = 100;

/** `playback_status` poll when the Audio Engine tab is open and the output stream runs without library `startEnginePlaybackPoll` (same cadence as `ENGINE_PLAYBACK_POLL_MS`). */
const AE_TAB_METER_POLL_MS = 33;
/** @type {ReturnType<typeof setInterval> | null} */
let aeTabMeterPollTimer = null;
let aeTabMeterPollInFlight = false;
let aeOutputGraphIdleBound = false;
/** @type {ResizeObserver | null} */
let aeGraphResizeObs = null;
/** @type {number} */
let aeGraphRafId = 0;

function aeAudioEngineTabIsActive() {
    const root = document.getElementById('tabAudioEngine');
    return root != null && root.classList.contains('active');
}

function stopAeInputPeakPoll() {
    if (aeInputPeakPollTimer != null) {
        clearInterval(aeInputPeakPollTimer);
        aeInputPeakPollTimer = null;
    }
}

/**
 * @param {object|null|undefined} es — `engine_state` payload (or `{ input_stream }` from status)
 */
function syncAeInputPeakPollFromEngineState(es) {
    if (!aeAudioEngineTabIsActive()) {
        stopAeInputPeakPoll();
        return;
    }
    if (es && es.input_stream && es.input_stream.running === true) {
        startAeInputPeakPoll();
    } else {
        stopAeInputPeakPoll();
    }
}

function startAeInputPeakPoll() {
    stopAeInputPeakPoll();
    const tick = () => {
        void tickAeInputPeakPoll();
    };
    aeInputPeakPollTimer = setInterval(tick, AE_INPUT_PEAK_POLL_MS);
    void tickAeInputPeakPoll();
}

async function tickAeInputPeakPoll() {
    if (!aeAudioEngineTabIsActive()) {
        stopAeInputPeakPoll();
        return;
    }
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) {
        stopAeInputPeakPoll();
        return;
    }
    if (typeof document !== 'undefined' && document.hidden) {
        stopAeInputPeakPoll();
        return;
    }
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        stopAeInputPeakPoll();
        return;
    }
    if (aeInputPeakPollInFlight) return;
    aeInputPeakPollInFlight = true;
    try {
        const st = await inv({cmd: 'input_stream_status'});
        const el = document.getElementById('aeInputStreamStatus');
        if (el && st && st.ok === true) {
            fillAeInputStreamLineFromPayload(st, el);
            if (st.running !== true) {
                stopAeInputPeakPoll();
            }
        } else {
            stopAeInputPeakPoll();
        }
    } catch {
        stopAeInputPeakPoll();
    } finally {
        aeInputPeakPollInFlight = false;
    }
}

/** When the tab was already initialized, re-sync input line + peak poll (e.g. user left tab and returned). */
async function resumeAeInputPeakPollIfNeeded() {
    if (!aeAudioEngineTabIsActive()) return;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return;
    if (typeof document !== 'undefined' && document.hidden) return;
    syncAePlaybackControlsFromPrefs();
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    try {
        const st = await inv({cmd: 'input_stream_status'});
        const el = document.getElementById('aeInputStreamStatus');
        if (el) fillAeInputStreamLineFromPayload(st, el);
        syncAeInputPeakPollFromEngineState({ input_stream: st });
    } catch {
        /* ignore */
    }
}

function bindAeInputPeakVisibilityOnce() {
    if (aeInputPeakVisibilityBound) return;
    aeInputPeakVisibilityBound = true;
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            stopAeInputPeakPoll();
        } else {
            void resumeAeInputPeakPollIfNeeded();
        }
    });
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        const idle = e.detail && e.detail.idle;
        if (idle) {
            stopAeInputPeakPoll();
        } else {
            void resumeAeInputPeakPollIfNeeded();
        }
    });
}

/**
 * @param {string} raw
 * @returns {number|undefined} positive integer frame count, or undefined to use driver default
 */
/** Matches AudioEngine `MAX_BUFFER_FRAMES` — typos like 144000 are ~3s @ 48 kHz and sound like delayed mute after stop. */
const AE_MAX_BUFFER_FRAMES = 8192;

/**
 * @param {string|null|undefined} raw — saved pref or select value
 * @returns {number|undefined} integer Hz for `sample_rate_hz` IPC, or undefined for driver default
 */
function parseAeSampleRateHzFromPrefString(raw) {
    if (raw == null) return undefined;
    const s = String(raw).trim();
    if (s === '') return undefined;
    const n = Number.parseInt(s, 10);
    if (!Number.isFinite(n) || n < 1000) return undefined;
    return n;
}

/**
 * @param {HTMLSelectElement|null} sel
 * @returns {number|undefined} integer Hz for `sample_rate_hz` IPC, or undefined for driver default
 */
function parseAeSampleRateHzFromSelect(sel) {
    if (!sel || typeof sel.value !== 'string') return undefined;
    return parseAeSampleRateHzFromPrefString(sel.value);
}

/**
 * @param {HTMLSelectElement|null} selectEl
 * @param {object|null} info — `get_output_device_info` / `get_input_device_info` payload
 * @param {string} [preferredHz] — saved pref string (numeric or empty)
 */
function aePopulateSampleRateSelect(selectEl, info, preferredHz) {
    if (!selectEl || typeof selectEl.replaceChildren !== 'function' || typeof catalogFmt !== 'function') return;
    selectEl.replaceChildren();
    const defOpt = document.createElement('option');
    defOpt.value = '';
    defOpt.textContent = catalogFmt('ui.ae.sample_rate_driver_default');
    selectEl.appendChild(defOpt);
    const rates = info && Array.isArray(info.sample_rates) ? info.sample_rates : [];
    const nums = [];
    for (const x of rates) {
        const n = typeof x === 'number' ? x : Number.parseFloat(String(x));
        if (Number.isFinite(n)) nums.push(n);
    }
    nums.sort((a, b) => a - b);
    for (const hz of nums) {
        const opt = document.createElement('option');
        const r = Math.round(hz);
        opt.value = String(r);
        opt.textContent = catalogFmt('ui.ae.sample_rate_option_hz', {hz: String(r)});
        selectEl.appendChild(opt);
    }
    const pref = preferredHz != null ? String(preferredHz).trim() : '';
    if (pref !== '' && [...selectEl.options].some((o) => o.value === pref)) {
        selectEl.value = pref;
    } else {
        selectEl.value = '';
    }
}

/**
 * @param {HTMLSelectElement|null} selectEl
 * @param {object|null} info — `get_output_device_info` / `get_input_device_info` payload
 * @param {string} [preferredFrames] — saved pref string (numeric or empty for driver default)
 */
function aePopulateBufferFramesSelect(selectEl, info, preferredFrames) {
    if (!selectEl || typeof selectEl.replaceChildren !== 'function' || typeof catalogFmt !== 'function') return;
    selectEl.replaceChildren();
    const defOpt = document.createElement('option');
    defOpt.value = '';
    defOpt.textContent = catalogFmt('ui.ae.buffer_frames_driver_default');
    selectEl.appendChild(defOpt);
    const raw = info && Array.isArray(info.buffer_sizes) ? info.buffer_sizes : [];
    const nums = [];
    for (const x of raw) {
        const n = typeof x === 'number' ? x : Number.parseInt(String(x), 10);
        if (Number.isFinite(n) && n > 0) nums.push(Math.min(n >>> 0, AE_MAX_BUFFER_FRAMES));
    }
    nums.sort((a, b) => a - b);
    const seen = new Set();
    for (const frames of nums) {
        if (seen.has(frames)) continue;
        seen.add(frames);
        const opt = document.createElement('option');
        opt.value = String(frames);
        opt.textContent = catalogFmt('ui.ae.buffer_frames_option', {frames: String(frames)});
        selectEl.appendChild(opt);
    }
    const pref = preferredFrames != null ? String(preferredFrames).trim() : '';
    if (pref !== '' && [...selectEl.options].some((o) => o.value === pref)) {
        selectEl.value = pref;
    } else if (pref !== '') {
        const opt = document.createElement('option');
        opt.value = pref;
        opt.textContent = catalogFmt('ui.ae.buffer_frames_saved_option', {frames: pref});
        selectEl.appendChild(opt);
        selectEl.value = pref;
    } else {
        selectEl.value = '';
    }
}

/**
 * @param {HTMLSelectElement|null} selectEl
 * @param {object} typeRes — `list_audio_device_types` payload (`ok`, `types`, `current`)
 */
function aePopulateAudioDeviceTypeSelect(selectEl, typeRes) {
    if (!selectEl || typeof selectEl.replaceChildren !== 'function') return;
    if (!typeRes || typeRes.ok !== true) {
        selectEl.replaceChildren();
        return;
    }
    const rows = Array.isArray(typeRes.types) ? typeRes.types : [];
    selectEl.replaceChildren();
    for (const row of rows) {
        let t = '';
        if (typeof row === 'string') {
            t = row.trim();
        } else if (row && typeof row === 'object' && row.type != null) {
            t = String(row.type).trim();
        }
        if (t === '') continue;
        const opt = document.createElement('option');
        opt.value = t;
        opt.textContent = t;
        selectEl.appendChild(opt);
    }
    const cur = typeRes.current != null ? String(typeRes.current) : '';
    if (cur !== '' && [...selectEl.options].some((o) => o.value === cur)) {
        selectEl.value = cur;
    } else if (selectEl.options.length > 0) {
        selectEl.selectedIndex = 0;
    }
}

function parseAeBufferFramesPref(raw) {
    const s = String(raw ?? '').trim();
    if (s === '') return undefined;
    const n = Number.parseInt(s, 10);
    if (!Number.isFinite(n) || n < 1) return undefined;
    return Math.min(n >>> 0, AE_MAX_BUFFER_FRAMES);
}

/**
 * @param {unknown} buf — `buffer_size` from AudioEngine (object or legacy string)
 * @returns {string}
 */
function formatAeBufferSize(buf) {
    if (buf == null) return '';
    if (typeof buf === 'string') return buf;
    if (typeof buf === 'object' && buf.kind === 'range' && buf.min != null && buf.max != null) {
        return `${buf.min}–${buf.max} frames`;
    }
    if (typeof buf === 'object' && buf.kind === 'unknown') return 'unknown';
    try {
        return JSON.stringify(buf);
    } catch {
        return '';
    }
}

/**
 * Appends `ui.ae.stream_buffer_fixed` when `st.stream_buffer_frames` is a finite number.
 * @param {string} line
 * @param {object} st — stream status fragment (`stream` / `input_stream` / status payloads)
 * @returns {string}
 */
function appendAeStreamBufferFixedSuffix(line, st) {
    if (typeof catalogFmt !== 'function') return line;
    const sbf = st.stream_buffer_frames;
    if (sbf != null && typeof sbf === 'number' && Number.isFinite(sbf)) {
        line += catalogFmt('ui.ae.stream_buffer_fixed', {frames: String(sbf)});
    }
    return line;
}

/**
 * Running-stream line (detail vs simple). Caller must ensure `catalogFmt` and a valid `device_id` on `st`.
 * @param {object} st
 * @param {'ui.ae.output_stream_on_detail'|'ui.ae.input_stream_on_detail'} detailKey
 * @param {'ui.ae.output_stream_on'|'ui.ae.input_stream_on'} simpleKey
 * @returns {string}
 */
function buildAeStreamStatusLineCore(st, detailKey, simpleKey) {
    const name = st.device_name != null ? String(st.device_name) : String(st.device_id);
    const rate = st.sample_rate_hz != null ? String(st.sample_rate_hz) : null;
    const ch = st.channels != null ? String(st.channels) : null;
    const fmt = st.sample_format != null ? String(st.sample_format) : '';
    const buf = formatAeBufferSize(st.buffer_size);
    if (rate != null && ch != null) {
        return catalogFmt(detailKey, {
            name,
            device: String(st.device_id),
            rate,
            channels: ch,
            format: fmt,
            buffer: buf,
        });
    }
    return catalogFmt(simpleKey, {device: String(st.device_id)});
}

/**
 * Shared line for `get_output_device_info` / `get_input_device_info` payloads (same JSON shape).
 * @param {object|null} info
 * @returns {string|null}
 */
function buildAeDeviceCapsLine(info) {
    if (!info || info.ok !== true || typeof catalogFmt !== 'function') return null;
    const ch = info.channels != null ? String(info.channels) : '?';
    const fmt = info.sample_format != null ? String(info.sample_format) : '?';
    const rate = info.sample_rate_hz != null ? String(info.sample_rate_hz) : '?';
    let rateLabel = rate;
    const r = info.sample_rate_range_hz;
    if (r && r.min != null && r.max != null && String(r.min) !== String(r.max)) {
        rateLabel = `${r.min}–${r.max}`;
    }
    const buf = formatAeBufferSize(info.buffer_size);
    const bufPart = buf ? ` · ${buf}` : '';
    return catalogFmt('ui.ae.device_caps', {
        rate: rateLabel,
        channels: ch,
        format: fmt,
    }) + bufPart;
}

/**
 * Read now-playing prefs into the Audio Engine tab sliders (no `ensureAudioGraph` — safe on tab open).
 */
function syncAePlaybackControlsFromPrefs() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return;
    const v = prefs.getItem('audioVolume') || '100';
    const aeVol = document.getElementById('aeVolume');
    const aePct = document.getElementById('aeVolumePct');
    if (aeVol) aeVol.value = v;
    if (aePct) aePct.textContent = v + '%';
    const sp = prefs.getItem('audioSpeed') || '1';
    const aeSp = document.getElementById('aePlaybackSpeed');
    if (aeSp) aeSp.value = sp;
    if (typeof setEqBand === 'function') {
        for (const band of ['low', 'mid', 'high']) {
            const cap = band.charAt(0).toUpperCase() + band.slice(1);
            const raw = prefs.getItem('eq' + cap);
            if (raw != null && raw !== '') setEqBand(band, raw);
        }
    }
    const pg = prefs.getItem('preampGain');
    if (pg != null) {
        const g = parseFloat(pg);
        const sl = document.getElementById('aeGainSlider');
        const lab = document.getElementById('aeGainVal');
        if (sl && !Number.isNaN(g)) sl.value = String(g);
        if (lab && !Number.isNaN(g)) lab.textContent = (g * 100).toFixed(0) + '%';
    }
    const pan = prefs.getItem('audioPan');
    if (pan != null) {
        const p = parseFloat(pan);
        const sl = document.getElementById('aePanSlider');
        const lab = document.getElementById('aePanVal');
        if (sl && !Number.isNaN(p)) sl.value = String(p);
        if (lab && !Number.isNaN(p)) {
            lab.textContent =
                Math.abs(p) < 0.05 ? 'C' : p < 0 ? Math.round(Math.abs(p) * 100) + 'L' : Math.round(p * 100) + 'R';
        }
    }
    syncAeTransportFromPlayback();
}

/**
 * Keep Audio Engine transport buttons aligned with floating player / rodio (`playback_pause`, reverse pref).
 */
function syncAeTransportFromPlayback() {
    const playBtn = document.getElementById('aePlaybackPlayPause');
    const revBtn = document.getElementById('aePlaybackReverse');
    const sb = document.getElementById('aeSkipBack5');
    const sf = document.getElementById('aeSkipForward5');
    const hasPath =
        typeof audioPlayerPath !== 'undefined' && audioPlayerPath != null && String(audioPlayerPath) !== '';
    for (const b of [revBtn, sb, sf]) {
        if (b) b.disabled = !hasPath;
    }
    if (revBtn && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        revBtn.classList.toggle('active', prefs.getItem('audioReverse') === 'on');
    }
    const monoBtn = document.getElementById('aePlaybackMono');
    if (monoBtn && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        monoBtn.classList.toggle('active', prefs.getItem('audioMono') === 'on');
    }
    const aePanTransport = document.getElementById('aePanSlider');
    if (aePanTransport && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        aePanTransport.disabled = prefs.getItem('audioMono') === 'on';
    }
    if (playBtn && typeof isAudioPlaying === 'function') {
        const playing = isAudioPlaying();
        const prev = playBtn.dataset.aePlaying === '1';
        if (playing !== prev) {
            playBtn.dataset.aePlaying = playing ? '1' : '0';
            playBtn.innerHTML = playing ? '&#9646;&#9646;' : '&#9654;';
            playBtn.classList.toggle('playing', playing);
        }
    }
}

function bindAePlaybackControls() {
    const vol = document.getElementById('aeVolume');
    if (vol && typeof vol.addEventListener === 'function' && typeof setAudioVolume === 'function') {
        vol.addEventListener('input', () => setAudioVolume(vol.value));
    }
    const sp = document.getElementById('aePlaybackSpeed');
    if (sp && typeof sp.addEventListener === 'function' && typeof setPlaybackSpeed === 'function') {
        sp.addEventListener('change', () => setPlaybackSpeed(sp.value));
    }
    const gain = document.getElementById('aeGainSlider');
    if (gain && typeof gain.addEventListener === 'function' && typeof setPreampGain === 'function') {
        gain.addEventListener('input', () => setPreampGain(gain.value));
    }
    const pan = document.getElementById('aePanSlider');
    if (pan && typeof pan.addEventListener === 'function' && typeof setPan === 'function') {
        pan.addEventListener('input', () => setPan(pan.value));
    }
    const playBtn = document.getElementById('aePlaybackPlayPause');
    if (playBtn && typeof playBtn.addEventListener === 'function') {
        playBtn.addEventListener('click', () => {
            if (typeof toggleAudioPlayback === 'function') toggleAudioPlayback();
            syncAeTransportFromPlayback();
        });
    }
    const revBtn = document.getElementById('aePlaybackReverse');
    if (revBtn && typeof revBtn.addEventListener === 'function') {
        revBtn.addEventListener('click', () => {
            if (typeof toggleReversePlayback === 'function') {
                void toggleReversePlayback().then(() => syncAeTransportFromPlayback());
            }
        });
    }
    const monoBtn = document.getElementById('aePlaybackMono');
    if (monoBtn && typeof monoBtn.addEventListener === 'function' && typeof toggleMono === 'function') {
        monoBtn.addEventListener('click', () => {
            toggleMono();
            syncAeTransportFromPlayback();
        });
    }
    const sb = document.getElementById('aeSkipBack5');
    if (sb && typeof sb.addEventListener === 'function') {
        sb.addEventListener('click', () => {
            if (typeof skipPlaybackSeconds === 'function') void skipPlaybackSeconds(-5);
        });
    }
    const sf = document.getElementById('aeSkipForward5');
    if (sf && typeof sf.addEventListener === 'function') {
        sf.addEventListener('click', () => {
            if (typeof skipPlaybackSeconds === 'function') void skipPlaybackSeconds(5);
        });
    }
}

// ── Fuzzy matching (fzf-style subsequence with scoring) ──

function aeFuzzyMatch(query, text) {
    const q = query.toLowerCase();
    const t = text.toLowerCase();
    if (!q) return {score: 0, indices: []};
    let qi = 0;
    const indices = [];
    for (let ti = 0; ti < t.length && qi < q.length; ti++) {
        if (t[ti] === q[qi]) {
            indices.push(ti);
            qi++;
        }
    }
    if (qi < q.length) return null;
    let score = 0;
    let prev = -2;
    for (const idx of indices) {
        if (idx === prev + 1) score += 3;
        if (idx === 0 || ' /\\._-:'.includes(t[idx - 1])) score += 2;
        prev = idx;
    }
    score -= indices.length > 0 ? indices[0] : 0;
    return {score, indices};
}

function aeHighlightMatch(text, indices) {
    if (!indices || !indices.length) return document.createTextNode(text);
    const frag = document.createDocumentFragment();
    const idxSet = new Set(indices);
    let run = '';
    let inMatch = false;
    for (let i = 0; i < text.length; i++) {
        const m = idxSet.has(i);
        if (m !== inMatch) {
            if (run) {
                if (inMatch) {
                    const sp = document.createElement('span');
                    sp.className = 'ae-match';
                    sp.textContent = run;
                    frag.appendChild(sp);
                } else {
                    frag.appendChild(document.createTextNode(run));
                }
            }
            run = '';
            inMatch = m;
        }
        run += text[i];
    }
    if (run) {
        if (inMatch) {
            const sp = document.createElement('span');
            sp.className = 'ae-match';
            sp.textContent = run;
            frag.appendChild(sp);
        } else {
            frag.appendChild(document.createTextNode(run));
        }
    }
    return frag;
}

// ── Plugin picker widget ──

function aeCreatePluginPicker() {
    const state = {selectedPath: '', selectedName: '', selectedFormat: ''};
    const wrap = document.createElement('div');
    wrap.className = 'ae-picker';
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'ae-picker-input';
    input.placeholder = catalogFmt('ui.ae.picker_search_placeholder');
    input.autocomplete = 'off';
    input.spellcheck = false;
    const clear = document.createElement('span');
    clear.className = 'ae-picker-clear';
    clear.textContent = '\u00d7';
    clear.title = catalogFmt('menu.clear');
    const dropdown = document.createElement('div');
    dropdown.className = 'ae-picker-dropdown';
    wrap.appendChild(input);
    wrap.appendChild(clear);
    wrap.appendChild(dropdown);

    let activeIdx = -1;

    function renderDropdown(query) {
        dropdown.innerHTML = '';
        activeIdx = -1;
        const q = (query || '').trim();
        let items = aePluginCatalog.map((p) => {
            const m = q ? aeFuzzyMatch(q, p.name) : {score: 0, indices: []};
            if (!m && q) {
                const mPath = aeFuzzyMatch(q, p.path);
                if (mPath) return {...p, score: mPath.score - 5, indices: []};
            }
            return m ? {...p, score: m.score, indices: m.indices} : null;
        }).filter(Boolean);
        if (q) items.sort((a, b) => b.score - a.score);
        if (!items.length) {
            const d = document.createElement('div');
            d.className = 'ae-picker-no-match';
            d.textContent = q ? 'No matches' : 'No plugins loaded';
            dropdown.appendChild(d);
            return;
        }
        for (const it of items) {
            const row = document.createElement('div');
            row.className = 'ae-picker-option';
            row.dataset.path = it.path;
            row.dataset.name = it.name;
            row.dataset.format = it.format;
            const badge = document.createElement('span');
            badge.className = 'ae-badge ' + (it.format === 'VST3' ? 'ae-badge-vst3' : 'ae-badge-au');
            badge.textContent = it.format || '?';
            row.appendChild(badge);
            const nameSpan = document.createElement('span');
            nameSpan.appendChild(aeHighlightMatch(it.name, it.indices));
            row.appendChild(nameSpan);
            row.addEventListener('mousedown', (e) => {
                e.preventDefault();
                selectItem(it.path, it.name, it.format);
            });
            dropdown.appendChild(row);
        }
    }

    function selectItem(path, name, format) {
        state.selectedPath = path;
        state.selectedName = name;
        state.selectedFormat = format;
        input.value = name;
        wrap.classList.toggle('has-value', !!path);
        closeDropdown();
    }

    function openDropdown() {
        wrap.classList.add('open');
        renderDropdown(input.value === state.selectedName ? '' : input.value);
    }

    function closeDropdown() {
        wrap.classList.remove('open');
        activeIdx = -1;
    }

    function scrollActiveIntoView() {
        const opts = dropdown.querySelectorAll('.ae-picker-option');
        if (activeIdx >= 0 && activeIdx < opts.length) {
            opts[activeIdx].scrollIntoView({block: 'nearest'});
        }
    }

    input.addEventListener('focus', () => {
        input.select();
        openDropdown();
    });

    input.addEventListener('input', () => {
        if (!wrap.classList.contains('open')) wrap.classList.add('open');
        renderDropdown(input.value);
    });

    input.addEventListener('keydown', (e) => {
        const opts = dropdown.querySelectorAll('.ae-picker-option');
        if (e.key === 'ArrowDown') {
            e.preventDefault();
            if (!wrap.classList.contains('open')) { openDropdown(); return; }
            activeIdx = Math.min(activeIdx + 1, opts.length - 1);
            opts.forEach((o, i) => o.classList.toggle('active', i === activeIdx));
            scrollActiveIntoView();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            activeIdx = Math.max(activeIdx - 1, 0);
            opts.forEach((o, i) => o.classList.toggle('active', i === activeIdx));
            scrollActiveIntoView();
        } else if (e.key === 'Enter') {
            e.preventDefault();
            if (activeIdx >= 0 && activeIdx < opts.length) {
                const o = opts[activeIdx];
                selectItem(o.dataset.path, o.dataset.name, o.dataset.format);
            }
        } else if (e.key === 'Escape') {
            e.preventDefault();
            if (state.selectedPath) input.value = state.selectedName;
            closeDropdown();
            input.blur();
        }
    });

    input.addEventListener('blur', () => {
        setTimeout(() => {
            if (state.selectedPath) input.value = state.selectedName;
            else input.value = '';
            closeDropdown();
        }, 150);
    });

    clear.addEventListener('mousedown', (e) => {
        e.preventDefault();
        state.selectedPath = '';
        state.selectedName = '';
        state.selectedFormat = '';
        input.value = '';
        wrap.classList.remove('has-value');
        input.focus();
    });

    return {
        el: wrap,
        getValue() { return state.selectedPath; },
        setValue(path) {
            const p = aePluginCatalog.find((x) => x.path === path);
            if (p) selectItem(p.path, p.name, p.format);
            else { state.selectedPath = path; state.selectedName = path; state.selectedFormat = ''; input.value = path; wrap.classList.toggle('has-value', !!path); }
        },
        refresh() {
            if (state.selectedPath) {
                const p = aePluginCatalog.find((x) => x.path === state.selectedPath);
                if (p) { state.selectedName = p.name; state.selectedFormat = p.format; input.value = p.name; }
            }
        },
    };
}

// ── Dynamic insert chain rows ──

function aeRebuildInsertChainUI(paths) {
    const container = document.getElementById('aeInsertChainContainer');
    if (!container) return;
    container.innerHTML = '';
    aeInsertPickers = [];
    if (!Array.isArray(paths)) paths = [];
    const count = Math.max(paths.length, 1);
    for (let i = 0; i < count; i++) {
        aeAddInsertRow(container, paths[i] || '');
    }
}

function aeAddInsertRow(container, initialPath) {
    if (!container) container = document.getElementById('aeInsertChainContainer');
    if (!container) return;
    const idx = aeInsertPickers.length;
    const row = document.createElement('div');
    row.className = 'ae-insert-row';

    const num = document.createElement('span');
    num.className = 'ae-slot-num';
    num.textContent = String(idx + 1);
    row.appendChild(num);

    const picker = aeCreatePluginPicker();
    row.appendChild(picker.el);

    const edBtn = document.createElement('button');
    edBtn.className = 'ae-insert-editor-btn';
    edBtn.textContent = 'Editor';
    edBtn.type = 'button';
    edBtn.addEventListener('click', () => {
        const slotIdx = aeInsertPickers.indexOf(picker);
        if (slotIdx >= 0) void aeOpenInsertEditor(slotIdx);
    });
    row.appendChild(edBtn);

    const rmBtn = document.createElement('button');
    rmBtn.className = 'ae-insert-remove-btn';
    rmBtn.textContent = '\u00d7';
    rmBtn.type = 'button';
    rmBtn.addEventListener('click', () => {
        const slotIdx = aeInsertPickers.indexOf(picker);
        if (slotIdx >= 0) aeRemoveInsertRow(slotIdx);
    });
    row.appendChild(rmBtn);

    container.appendChild(row);
    aeInsertPickers.push(picker);
    if (initialPath) picker.setValue(initialPath);
}

function aeRemoveInsertRow(idx) {
    const container = document.getElementById('aeInsertChainContainer');
    if (!container) return;
    if (idx < 0 || idx >= aeInsertPickers.length) return;
    if (aeInsertPickers.length <= 1) {
        aeInsertPickers[0].setValue('');
        return;
    }
    container.children[idx].remove();
    aeInsertPickers.splice(idx, 1);
    for (let i = 0; i < container.children.length; i++) {
        const num = container.children[i].querySelector('.ae-slot-num');
        if (num) num.textContent = String(i + 1);
    }
}

function aePluginEntriesFromChain(chain) {
    const plugins = chain && Array.isArray(chain.plugins) ? chain.plugins : [];
    return plugins
        .map((p) => ({
            path: p && p.path != null ? String(p.path) : '',
            name: p && p.name != null ? String(p.name) : String((p && p.path) || '').split('/').pop(),
            format: p && p.format != null ? String(p.format) : '',
            isInstrument: !!(p && p.isInstrument === true),
        }))
        .filter((p) => p.path);
}

function aeInsertPickerShowInstrumentsEnabled() {
    return typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
        && prefs.getItem('aeInsertPickerShowInstruments') === 'on';
}

/* AU plugins are loaded out-of-process by `audiocomponentd` via `_RemoteAUv2ViewFactory`,
 * which only delivers its populated NSView over an XPC connection that requires the host to
 * be signed with a real Apple Developer ID. Our ad-hoc signed helper bundle gets `not set`
 * as `TeamIdentifier`, so the XPC view-controller delivery never completes and AU editor
 * windows render as a permanent 1×1 placeholder (blank/white). Until the project ships with
 * a real Developer ID signing identity, AU plugin editors are unusable — so the picker
 * defaults to hiding them. The user can flip this off if they want to see/select AUs anyway
 * (e.g. for an instance whose audio works fine without ever opening its UI).
 * See `audio-engine/README.md` "Helper .app architecture" for the full story. */
function aeInsertPickerHideAudioUnitsEnabled() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return true;
    const v = prefs.getItem('aeInsertPickerHideAudioUnits');
    // Default is ON (hide) when the pref has never been set.
    return v == null || v === '' || v === 'on';
}

/** Compose the active filter set against an entry list — single source of truth so the
 *  catalog rebuild and the live refresh path stay in sync as filters are added. */
function aeApplyInsertPickerFilters(entries) {
    let out = entries;
    if (!aeInsertPickerShowInstrumentsEnabled())
        out = out.filter((p) => !p.isInstrument);
    if (aeInsertPickerHideAudioUnitsEnabled())
        out = out.filter((p) => p.format !== 'AudioUnit');
    return out;
}

/** Rebuild `aePluginCatalog` from `aeLastPluginChain` without rebuilding rows (picker filter toggle). */
function aeRefreshInsertPickerCatalogOnly() {
    const full = aePluginEntriesFromChain(aeLastPluginChain);
    aePluginCatalog = aeApplyInsertPickerFilters(full);
    for (const picker of aeInsertPickers) {
        if (picker && typeof picker.refresh === 'function') picker.refresh();
    }
}

function syncAeInsertPickerShowInstrumentsCheckbox() {
    const cb = document.getElementById('aeInsertPickerShowInstruments');
    if (!cb || typeof cb !== 'object') return;
    cb.checked = aeInsertPickerShowInstrumentsEnabled();
}

function syncAeInsertPickerHideAudioUnitsCheckbox() {
    const cb = document.getElementById('aeInsertPickerHideAudioUnits');
    if (!cb || typeof cb !== 'object') return;
    cb.checked = aeInsertPickerHideAudioUnitsEnabled();
}

function aePopulateInsertSlotSelects(chain) {
    aeLastPluginChain = chain || null;
    const full = aePluginEntriesFromChain(chain);
    aePluginCatalog = aeApplyInsertPickerFilters(full);

    const fromServer = chain && Array.isArray(chain.insert_paths) ? chain.insert_paths.map((x) => String(x)) : [];
    let pick = [];
    if (fromServer.length > 0) {
        pick = fromServer;
    } else if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        try {
            const raw = prefs.getItem(AE_PREFS_INSERT_PATHS_JSON);
            if (typeof raw === 'string' && raw.trim() !== '') pick = JSON.parse(raw);
        } catch {
            pick = [];
        }
    }
    if (!Array.isArray(pick)) pick = [];
    aeRebuildInsertChainUI(pick);
}

/**
 * Poll `plugin_chain` until the AudioEngine finishes scanning (`phase` !== `scanning`) or attempts exhausted.
 * @param {function} inv — `audioEngineInvoke`
 * @param {object} [initialChain] — if set, skip the first `plugin_chain` IPC (caller already fetched it).
 * @param {number} [expectedGen] — if set, stop when `aePluginChainPollGeneration` changes (stale panel refresh).
 * @returns {Promise<object>}
 */
async function fetchPluginChainUntilSettled(inv, initialChain, expectedGen) {
    const delay = (ms) => new Promise((r) => setTimeout(r, ms));
    const toast = (key, params, ms, kind) => {
        if (typeof showToast !== 'function' || typeof toastFmt !== 'function') return;
        showToast(toastFmt(key, params || {}), ms, kind);
    };

    dismissAePluginScanProgressToast();
    aeScanProgressToastKey = '';
    aeScanProgressToastKeyAt = 0;

    let chain =
        initialChain !== undefined && initialChain !== null
            ? initialChain
            : await inv({cmd: 'plugin_chain'});
    const sawScanning = chain && chain.phase === 'scanning';
    if (sawScanning) {
        toast('toast.ae_plugin_scan_started', {}, 4200, 'info');
    }

    let attempts = 0;
    /* ~69h at 250ms — large libraries exceed the old cap (600 ≈ 2.5min), so the UI stayed on
     * "scanning" forever after the engine finished (`plugin_chain` never polled to `phase: juce`). */
    const maxAttempts = 1_000_000;
    while (chain && chain.phase === 'scanning' && attempts < maxAttempts) {
        if (expectedGen != null && aePluginChainPollGeneration !== expectedGen) {
            dismissAePluginScanProgressToast();
            return chain;
        }
        await delay(250);
        chain = await inv({cmd: 'plugin_chain'});
        attempts++;
        if (expectedGen != null && aePluginChainPollGeneration !== expectedGen) {
            dismissAePluginScanProgressToast();
            return chain;
        }
        if (chain && chain.phase === 'scanning') {
            const now = typeof Date.now === 'function' ? Date.now() : 0;
            const key = `${chain.scan_done}|${chain.scan_current_format}|${chain.scan_current_name}|${chain.scan_skipped}`;
            if (key !== aeScanProgressToastKey) {
                aeScanProgressToastKey = key;
                aeScanProgressToastKeyAt = now;
            }
            const sec = Math.floor((now - aeScanProgressToastKeyAt) / 1000);
            const suffix = sec >= 1 ? ` · ${sec}s` : '';
            fillAePluginSection(chain, {elapsedSec: sec});
            const line = formatAePluginScanProgressLine(chain);
            if (line && typeof toastFmt === 'function') {
                ensureAePluginScanProgressToast(
                    toastFmt('toast.ae_plugin_scan_progress', {line: line + suffix}),
                );
            }
        }
    }

    dismissAePluginScanProgressToast();

    if (sawScanning && chain && chain.phase === 'juce') {
        const n = chain.plugin_count != null ? Number(chain.plugin_count) : 0;
        toast(
            'toast.ae_plugin_scan_done',
            {count: Number.isFinite(n) ? String(n) : '0'},
            4800,
            'success',
        );
    } else if (chain && chain.phase === 'failed') {
        const err = chain.error != null ? String(chain.error) : '';
        toast('toast.ae_plugin_scan_failed', {err}, 6500, 'error');
    } else if (chain && chain.phase === 'scanning' && attempts >= maxAttempts) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_plugin_scan_timeout'), 8000, 'warning');
        }
    }

    /* Last poll may leave `phase` as `juce` — inner loop only fills while `scanning`; always sync UI. */
    fillAePluginSection(chain);

    return chain;
}

/**
 * Format `plugin_chain` scanning fields (`scan_done`, `scan_total`, …) for the Audio Engine tab + toasts.
 * @param {object|null} chain
 * @returns {string}
 */
function formatAePluginScanProgressLine(chain) {
    if (!chain || chain.phase !== 'scanning') return '';
    const done = chain.scan_done != null ? Number(chain.scan_done) : 0;
    const total = chain.scan_total != null ? Number(chain.scan_total) : 0;
    const skipped = chain.scan_skipped != null ? Number(chain.scan_skipped) : 0;
    const fmt = chain.scan_current_format != null ? String(chain.scan_current_format) : '';
    let name = chain.scan_current_name != null ? String(chain.scan_current_name) : '';
    if (name.length > 80) name = name.slice(0, 77) + '…';
    const cacheOn = chain.scan_cache_loaded === true;
    if (typeof catalogFmt === 'function') {
        return catalogFmt('ui.ae.plugins_scan_progress', {
            done: String(Number.isFinite(done) ? done : 0),
            total: String(Number.isFinite(total) ? total : 0),
            skipped: String(Number.isFinite(skipped) ? skipped : 0),
            format: fmt,
            name: name || '—',
            cache: cacheOn ? catalogFmt('ui.ae.plugins_scan_cache_prefix') : '',
        });
    }
    const c = cacheOn ? '[cache] ' : '';
    return `${c}${done}/${total} · skipped ${skipped} · ${fmt}: ${name || '—'}`;
}

/**
 * @param {object|null} chain — `plugin_chain` payload
 * @param {{elapsedSec?: number}|undefined} [scanUiExtra] — wall-clock seconds on the current plug-in step (from poll loop)
 */
function fillAePluginSection(chain, scanUiExtra) {
    const stub = document.getElementById('aePluginStub');
    const prog = document.getElementById('aePluginScanProgress');
    const ul = document.getElementById('aePluginSlotList');
    if (!stub || typeof catalogFmt !== 'function') return;
    const phase = chain && chain.phase != null ? String(chain.phase) : '—';
    const fmts =
        chain && Array.isArray(chain.formats_planned) && chain.formats_planned.length
            ? chain.formats_planned.join(', ')
            : '—';
    const n =
        chain && chain.plugin_count != null && Number.isFinite(Number(chain.plugin_count))
            ? Number(chain.plugin_count)
            : chain && Array.isArray(chain.slots)
              ? chain.slots.length
              : 0;
    const note = chain && chain.note != null ? String(chain.note) : '';
    if (phase === 'failed') {
        const err =
            chain && chain.error != null ? String(chain.error) : note || '—';
        stub.textContent = catalogFmt('ui.ae.plugins_scan_failed', {err});
    } else if (phase === 'scanning') {
        stub.textContent = catalogFmt('ui.ae.plugins_scanning_note');
        if (prog) {
            prog.style.display = '';
            let line = formatAePluginScanProgressLine(chain);
            const es = scanUiExtra && scanUiExtra.elapsedSec != null ? Number(scanUiExtra.elapsedSec) : 0;
            if (Number.isFinite(es) && es >= 1) {
                line += ` · ${es}s`;
            }
            prog.textContent = line;
        }
    } else {
        stub.textContent = catalogFmt('ui.ae.plugins_stub', {
            phase,
            formats: fmts,
            count: String(n),
            note,
        });
    }
    if (prog && phase !== 'scanning') {
        prog.style.display = 'none';
        prog.textContent = '';
    }
    aePopulateInsertSlotSelects(chain);
    if (!ul || typeof ul.replaceChildren !== 'function') return;
    ul.replaceChildren();
    if (phase === 'scanning') {
        const li = document.createElement('li');
        li.textContent = catalogFmt('ui.ae.plugins_scanning_list');
        ul.appendChild(li);
        return;
    }
    if (phase === 'failed') {
        const li = document.createElement('li');
        li.textContent =
            chain && chain.error != null ? String(chain.error) : catalogFmt('ui.ae.plugins_slot_empty');
        ul.appendChild(li);
        return;
    }
    if (chain && Array.isArray(chain.slots) && chain.slots.length > 0) {
        for (const s of chain.slots) {
            const li = document.createElement('li');
            const path = s && typeof s === 'object' && s.path != null ? String(s.path) : '';
            li.textContent = path || (typeof s === 'string' ? s : JSON.stringify(s));
            ul.appendChild(li);
        }
    } else {
        const li = document.createElement('li');
        li.textContent = catalogFmt('ui.ae.plugins_slot_empty');
        ul.appendChild(li);
    }
}

/**
 * Map UI slot index to the chain index (only counting non-empty slots before it).
 * @returns {number} chain index, or -1 if slot is empty
 */
function aeChainIndexForInsertUiSlot(uiSlotIndex) {
    if (uiSlotIndex < 0 || uiSlotIndex >= aeInsertPickers.length) return -1;
    let idx = 0;
    for (let i = 0; i < uiSlotIndex; i++) {
        if (aeInsertPickers[i].getValue()) idx++;
    }
    if (!aeInsertPickers[uiSlotIndex].getValue()) return -1;
    return idx;
}

function aeCollectInsertPathsFromUi() {
    const paths = [];
    for (const pk of aeInsertPickers) {
        const v = pk.getValue();
        if (v) paths.push(String(v));
    }
    return paths;
}

function aeInsertPathArraysEqual(a, b) {
    if (!Array.isArray(a) || !Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
        if (a[i] !== b[i]) return false;
    }
    return true;
}

async function aeOpenInsertEditor(uiSlotIndex) {
    const inv = getAeAudioEngineInvoke();
    if (!inv) { aeNotifyNoAudioEngineIpc(); return; }
    if (!aeInsertPickers[uiSlotIndex] || !aeInsertPickers[uiSlotIndex].getValue()) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_insert_editor_no_plugin'), 4000, 'warning');
        return;
    }
    try {
        const uiPaths = aeCollectInsertPathsFromUi();
        const chain = await inv({cmd: 'plugin_chain'});
        throwIfAeNotOk(chain, 'plugin_chain failed');
        const enginePaths = Array.isArray(chain.insert_paths) ? chain.insert_paths.map((x) => String(x)) : [];
        const needApply = !aeInsertPathArraysEqual(uiPaths, enginePaths);

        /* `playback_set_inserts` rejects while `outputRunning`. If the chain is already loaded, skip
         * stop + apply so opening the editor does not interrupt playback. */
        if (needApply) {
            await stopAeOutputStream({ throwOnFailure: true });
            await applyAePlaybackInserts({ showAppliedToast: false, rethrowOnFailure: true });
        }
        const chainIdx = aeChainIndexForInsertUiSlot(uiSlotIndex);
        if (chainIdx < 0) return;
        const r = await inv({cmd: 'playback_open_insert_editor', slot: chainIdx});
        throwIfAeNotOk(r, 'playback_open_insert_editor failed');
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_insert_editor_failed', {err}), 5000, 'error');
    }
}

/**
 * @param {object} [opts]
 * @param {boolean} [opts.showAppliedToast=true] — `false` when syncing for open editor (Apply inserts still uses default).
 * @param {boolean} [opts.rethrowOnFailure=false] — when true, failed `playback_set_inserts` rethrows after toast (open editor must not call `playback_open_insert_editor`).
 */
async function applyAePlaybackInserts(opts) {
    const showAppliedToast = opts == null || opts.showAppliedToast !== false;
    const rethrowOnFailure = opts != null && opts.rethrowOnFailure === true;
    const inv = getAeAudioEngineInvoke();
    if (!inv) { aeNotifyNoAudioEngineIpc(); return; }
    const paths = [];
    for (const pk of aeInsertPickers) {
        const v = pk.getValue();
        if (v) paths.push(v);
    }
    try {
        const r = await inv({cmd: 'playback_set_inserts', paths});
        throwIfAeNotOk(r, 'playback_set_inserts failed');
        if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function')
            prefs.setItem(AE_PREFS_INSERT_PATHS_JSON, JSON.stringify(paths));
        if (
            showAppliedToast &&
            typeof showToast === 'function' &&
            typeof toastFmt === 'function'
        ) {
            showToast(toastFmt('toast.ae_inserts_applied'), 3000, 'success');
        }
        const chain = await fetchPluginChainUntilSettled(inv, undefined, aePluginChainPollGeneration);
        fillAePluginSection(chain);
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (
            !rethrowOnFailure &&
            typeof showToast === 'function' &&
            typeof toastFmt === 'function'
        ) {
            showToast(toastFmt('toast.ae_inserts_failed', {err}), 5000, 'error');
        }
        if (rethrowOnFailure) throw e;
    }
}

/**
 * Kill the `audio-engine` subprocess; next IPC spawns a fresh process. Clears JS engine playback state.
 */
async function restartAeAudioEngine() {
    const u = typeof window !== 'undefined' ? window.vstUpdater : undefined;
    const restart =
        u && typeof u.audioEngineRestart === 'function' ? u.audioEngineRestart.bind(u) : null;
    if (!restart) {
        aeNotifyNoAudioEngineIpc();
        return;
    }
    try {
        await restart();
        aeInitialDeviceTypeRestored = false;
        if (typeof window !== 'undefined' && typeof window.syncEnginePlaybackStoppedFromAudioEngine === 'function') {
            window.syncEnginePlaybackStoppedFromAudioEngine();
        }
        if (typeof window !== 'undefined' && typeof window.stopEnginePlaybackPoll === 'function') {
            window.stopEnginePlaybackPoll();
        }
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_audioengine_restarted'), 3000, 'success');
        }
        void refreshAudioEnginePanel();
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_audioengine_restart_failed', {err}), 5000, 'error');
        }
        void refreshAudioEnginePanel();
    }
}

async function aeWipeAndRescan() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) { aeNotifyNoAudioEngineIpc(); return; }
    try {
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_plugin_rescan_wiping'), 3000, 'info');
        const timeoutEl = document.getElementById('aeScanTimeout');
        const timeoutSec = timeoutEl ? Math.max(5, Math.min(3600, parseInt(timeoutEl.value, 10) || 30)) : 30;
        const r = await inv({cmd: 'plugin_rescan', timeout_sec: timeoutSec});
        throwIfAeNotOk(r, 'plugin_rescan failed');
        const chain = await fetchPluginChainUntilSettled(inv, undefined, ++aePluginChainPollGeneration);
        fillAePluginSection(chain);
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_plugin_rescan_complete'), 3000, 'success');
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_plugin_rescan_failed', {err}), 5000, 'error');
    }
}

/**
 * Called when the Audio Engine tab becomes active (`utils.js` `switchTab` → `runPerTabWork`).
 * Idempotent — safe if called multiple times.
 */
function initAudioEngineTab() {
    const root = document.getElementById('tabAudioEngine');
    if (!root) return;
    if (root.dataset.aeInit === '1') {
        syncAePlaybackControlsFromPrefs();
        syncAeInsertPickerShowInstrumentsCheckbox();
        void resumeAeInputPeakPollIfNeeded();
        void refreshAeProcessStats();
        layoutAeOutputGraphCanvases();
        syncAeOutputGraphsAfterStreamStateChange();
        return;
    }
    root.dataset.aeInit = '1';
    bindAeInputPeakVisibilityOnce();
    bindAeOutputGraphIdleOnce();
    bindAePlaybackControls();
    syncAePlaybackControlsFromPrefs();

    const refreshBtn = document.getElementById('aeRefreshDevices');
    if (refreshBtn && typeof refreshBtn.addEventListener === 'function') {
        refreshBtn.addEventListener('click', () => {
            void refreshAudioEnginePanel();
        });
    }
    const restartAudioEngineBtn = document.getElementById('aeRestartAudioEngine');
    if (restartAudioEngineBtn && typeof restartAudioEngineBtn.addEventListener === 'function') {
        restartAudioEngineBtn.addEventListener('click', () => {
            void restartAeAudioEngine();
        });
    }
    const applyBtn = document.getElementById('aeApplyDevice');
    if (applyBtn && typeof applyBtn.addEventListener === 'function') {
        applyBtn.addEventListener('click', () => {
            void applyAudioEngineDevice();
        });
    }
    const applyInsertsBtn = document.getElementById('aeApplyInserts');
    if (applyInsertsBtn && typeof applyInsertsBtn.addEventListener === 'function') {
        applyInsertsBtn.addEventListener('click', () => {
            void applyAePlaybackInserts();
        });
    }
    const addSlotBtn = document.getElementById('aeAddInsertSlot');
    if (addSlotBtn && typeof addSlotBtn.addEventListener === 'function') {
        addSlotBtn.addEventListener('click', () => {
            aeAddInsertRow(null, '');
        });
    }
    syncAeInsertPickerShowInstrumentsCheckbox();
    const insertShowInstrumentsCb = document.getElementById('aeInsertPickerShowInstruments');
    if (insertShowInstrumentsCb && typeof insertShowInstrumentsCb.addEventListener === 'function'
        && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        insertShowInstrumentsCb.addEventListener('change', () => {
            prefs.setItem('aeInsertPickerShowInstruments', insertShowInstrumentsCb.checked ? 'on' : 'off');
            aeRefreshInsertPickerCatalogOnly();
        });
    }
    syncAeInsertPickerHideAudioUnitsCheckbox();
    const insertHideAudioUnitsCb = document.getElementById('aeInsertPickerHideAudioUnits');
    if (insertHideAudioUnitsCb && typeof insertHideAudioUnitsCb.addEventListener === 'function'
        && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        insertHideAudioUnitsCb.addEventListener('change', () => {
            prefs.setItem('aeInsertPickerHideAudioUnits', insertHideAudioUnitsCb.checked ? 'on' : 'off');
            aeRefreshInsertPickerCatalogOnly();
        });
    }
    const wipeRescanBtn = document.getElementById('aeWipeRescan');
    if (wipeRescanBtn && typeof wipeRescanBtn.addEventListener === 'function') {
        wipeRescanBtn.addEventListener('click', () => {
            void aeWipeAndRescan();
        });
    }
    const scanTimeoutInput = document.getElementById('aeScanTimeout');
    if (scanTimeoutInput) {
        if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
            const saved = prefs.getItem('pluginScanTimeoutSec');
            if (saved != null && String(saved) !== '') scanTimeoutInput.value = String(saved);
        }
        scanTimeoutInput.addEventListener('change', () => {
            const v = Math.max(5, Math.min(3600, parseInt(scanTimeoutInput.value, 10) || 30));
            scanTimeoutInput.value = String(v);
            if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function')
                prefs.setItem('pluginScanTimeoutSec', String(v));
        });
    }
    const stopBtn = document.getElementById('aeStopStream');
    if (stopBtn && typeof stopBtn.addEventListener === 'function') {
        stopBtn.addEventListener('click', () => {
            void stopAeOutputStream();
        });
    }
    const toneCb = document.getElementById('aeTestTone');
    const bufOut = document.getElementById('aeBufferFramesOutput');
    const bufInCap = document.getElementById('aeBufferFramesInput');
    migrateAeBufferPrefs();
    if (toneCb && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        toneCb.checked = prefs.getItem(AE_PREFS_TONE) === '1';
    }
    if (bufOut && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const saved = prefs.getItem(AE_PREFS_BUFFER_FRAMES_OUTPUT);
        bufOut.value = saved != null && String(saved) !== '' ? String(saved) : '';
    }
    if (bufInCap && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const savedIn = prefs.getItem(AE_PREFS_BUFFER_FRAMES_INPUT);
        bufInCap.value = savedIn != null && String(savedIn) !== '' ? String(savedIn) : '';
    }
    const specFft = document.getElementById('aePlaybackSpectrumFftOrder');
    const specBands = document.getElementById('aePlaybackSpectrumBands');
    if (specFft && specBands && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        migrateAePlaybackSpectrumSplitPrefs();
        specFft.value = normalizeAePlaybackSpectrumFftOrderPref(prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER));
        const maxB = aeMaxSpectrumBinsForFftOrder(specFft.value);
        const bandRaw = prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS);
        specBands.value = aeNormalizeBandsSelectValue(bandRaw, maxB);
        aeUpdatePlaybackSpectrumBandsUiState(specFft, specBands);
    }
    if (toneCb && typeof toneCb.addEventListener === 'function') {
        toneCb.addEventListener('change', () => {
            if (toneCb.disabled) return;
            if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
                prefs.setItem(AE_PREFS_TONE, toneCb.checked ? '1' : '0');
            }
            void toggleAeTestTone(toneCb.checked);
        });
    }
    if (bufOut && typeof bufOut.addEventListener === 'function' && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        const saveOut = () => {
            prefs.setItem(AE_PREFS_BUFFER_FRAMES_OUTPUT, bufOut.value != null ? String(bufOut.value).trim() : '');
        };
        bufOut.addEventListener('change', saveOut);
        bufOut.addEventListener('blur', saveOut);
    }
    if (bufInCap && typeof bufInCap.addEventListener === 'function' && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        const saveIn = () => {
            prefs.setItem(AE_PREFS_BUFFER_FRAMES_INPUT, bufInCap.value != null ? String(bufInCap.value).trim() : '');
        };
        bufInCap.addEventListener('change', saveIn);
        bufInCap.addEventListener('blur', saveIn);
    }
    if (
        specFft &&
        specBands &&
        typeof specFft.addEventListener === 'function' &&
        typeof specBands.addEventListener === 'function' &&
        typeof prefs !== 'undefined' &&
        typeof prefs.setItem === 'function'
    ) {
        const saveSpectrumPrefs = () => {
            const fo = normalizeAePlaybackSpectrumFftOrderPref(specFft.value != null ? String(specFft.value) : '11');
            specFft.value = fo;
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER, fo);
            aeUpdatePlaybackSpectrumBandsUiState(specFft, specBands);
            const maxB = aeMaxSpectrumBinsForFftOrder(fo);
            let bv = specBands.value != null ? String(specBands.value) : '1024';
            bv = aeNormalizeBandsSelectValue(bv === 'max' ? 'max' : bv, maxB);
            specBands.value = bv;
            prefs.setItem(AE_PREFS_PLAYBACK_SPECTRUM_BANDS, bv);
        };
        specFft.addEventListener('change', saveSpectrumPrefs);
        specBands.addEventListener('change', saveSpectrumPrefs);
    }

    const typeSel = document.getElementById('aeAudioDeviceType');
    if (typeSel && typeof typeSel.addEventListener === 'function') {
        typeSel.addEventListener('change', () => {
            void aeApplyAudioDeviceTypeChange();
        });
    }
    const outDevSel = document.getElementById('aeOutputDevice');
    if (outDevSel && typeof outDevSel.addEventListener === 'function') {
        outDevSel.addEventListener('change', () => {
            const inv = getAeAudioEngineInvoke();
            if (!inv) return;
            const id = outDevSel.value != null ? String(outDevSel.value) : '';
            void fillAeDeviceCaps(inv, id);
        });
    }
    const srSel = document.getElementById('aeSampleRate');
    if (srSel && typeof srSel.addEventListener === 'function' && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        const saveSr = () => {
            prefs.setItem(AE_PREFS_SAMPLE_RATE_HZ, srSel.value != null ? String(srSel.value).trim() : '');
        };
        srSel.addEventListener('change', saveSr);
        srSel.addEventListener('blur', saveSr);
    }

    const inSel = document.getElementById('aeInputDevice');
    if (inSel && typeof inSel.addEventListener === 'function' && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        inSel.addEventListener('change', () => {
            prefs.setItem(AE_PREFS_INPUT_DEVICE, inSel.value != null ? String(inSel.value) : '');
            const inv = getAeAudioEngineInvoke();
            if (inv) void fillAeInputDeviceCaps(inv, inSel.value);
        });
    }

    const startInBtn = document.getElementById('aeStartInputCapture');
    if (startInBtn && typeof startInBtn.addEventListener === 'function') {
        startInBtn.addEventListener('click', () => {
            void startAeInputCapture();
        });
    }
    const stopInBtn = document.getElementById('aeStopInputCapture');
    if (stopInBtn && typeof stopInBtn.addEventListener === 'function') {
        stopInBtn.addEventListener('click', () => {
            void stopAeInputCapture();
        });
    }

    aeStartProcessStatsPollingOnce();
    void refreshAudioEnginePanel();
}

/**
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 * @param {string} deviceId — AudioEngine device id (stable name-based or legacy index)
 */
async function fillAeDeviceCaps(inv, deviceId) {
    const capsEl = document.getElementById('aeDeviceCaps');
    const srSel = document.getElementById('aeSampleRate');
    const bufOut = document.getElementById('aeBufferFramesOutput');
    if (!capsEl || typeof inv !== 'function') {
        if (capsEl) capsEl.textContent = '—';
        return;
    }
    let prefSr = '';
    let prefBuf = '';
    if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const p = prefs.getItem(AE_PREFS_SAMPLE_RATE_HZ);
        prefSr = p != null ? String(p).trim() : '';
        const b = prefs.getItem(AE_PREFS_BUFFER_FRAMES_OUTPUT);
        prefBuf = b != null ? String(b).trim() : '';
    }
    try {
        const payload = {cmd: 'get_output_device_info'};
        const id = deviceId != null ? String(deviceId).trim() : '';
        if (id !== '') {
            payload.device_id = id;
        }
        const info = await inv(payload);
        const line = buildAeDeviceCapsLine(info);
        capsEl.textContent = line != null ? line : '—';
        if (srSel) {
            aePopulateSampleRateSelect(srSel, info && info.ok === true ? info : null, prefSr);
        }
        if (bufOut) {
            aePopulateBufferFramesSelect(bufOut, info && info.ok === true ? info : null, prefBuf);
        }
    } catch {
        capsEl.textContent = '—';
        if (srSel) {
            aePopulateSampleRateSelect(srSel, null, prefSr);
        }
        if (bufOut) {
            aePopulateBufferFramesSelect(bufOut, null, prefBuf);
        }
    }
}

/**
 * `get_input_device_info`: omit `device_id` when empty for system default input.
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 * @param {string} [deviceId] — AudioEngine id or "" for default
 */
async function fillAeInputDeviceCaps(inv, deviceId) {
    const el = document.getElementById('aeInputDeviceCaps');
    const bufIn = document.getElementById('aeBufferFramesInput');
    if (!el || typeof inv !== 'function') {
        if (el) el.textContent = '—';
        return;
    }
    let prefBuf = '';
    if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const b = prefs.getItem(AE_PREFS_BUFFER_FRAMES_INPUT);
        prefBuf = b != null ? String(b).trim() : '';
    }
    try {
        const payload = {cmd: 'get_input_device_info'};
        const id = deviceId != null ? String(deviceId).trim() : '';
        if (id !== '') {
            payload.device_id = id;
        }
        const info = await inv(payload);
        const line = buildAeDeviceCapsLine(info);
        el.textContent = line != null ? line : '—';
        if (bufIn) {
            aePopulateBufferFramesSelect(bufIn, info && info.ok === true ? info : null, prefBuf);
        }
    } catch {
        el.textContent = '—';
        if (bufIn) {
            aePopulateBufferFramesSelect(bufIn, null, prefBuf);
        }
    }
}

/**
 * @param {object} st — `engine_state.stream` or `output_stream_status` payload
 * @param {HTMLElement} el
 */
function fillAeStreamLineFromPayload(st, el) {
    if (!el) return;
    if (typeof catalogFmt !== 'function') {
        el.textContent = '—';
        return;
    }
    if (!st || st.ok !== true) {
        el.textContent = '—';
        return;
    }
    if (st.running === true && st.device_id != null && st.device_id !== '') {
        let line = buildAeStreamStatusLineCore(st, 'ui.ae.output_stream_on_detail', 'ui.ae.output_stream_on');
        if (st.tone_on === true && st.tone_supported === true) {
            line += catalogFmt('ui.ae.tone_active');
        }
        line = appendAeStreamBufferFixedSuffix(line, st);
        el.textContent = line;
    } else {
        el.textContent = catalogFmt('ui.ae.output_stream_off');
    }
}

/**
 * @param {object} st — `engine_state.input_stream` or `input_stream_status`
 * @param {HTMLElement} el
 */
function fillAeInputStreamLineFromPayload(st, el) {
    if (!el) return;
    if (typeof catalogFmt !== 'function') {
        el.textContent = '—';
        return;
    }
    if (!st || st.ok !== true) {
        el.textContent = '—';
        return;
    }
    if (st.running === true && st.device_id != null && st.device_id !== '') {
        let line = buildAeStreamStatusLineCore(st, 'ui.ae.input_stream_on_detail', 'ui.ae.input_stream_on');
        line = appendAeStreamBufferFixedSuffix(line, st);
        const ipk = st.input_peak;
        if (ipk != null && typeof ipk === 'number' && Number.isFinite(ipk)) {
            line += catalogFmt('ui.ae.input_peak_suffix', {level: ipk.toFixed(2)});
        }
        el.textContent = line;
    } else {
        el.textContent = catalogFmt('ui.ae.input_stream_off');
    }
}

/**
 * @param {object} es — `engine_state` payload
 */
function fillAeStreamsFromEngineState(es) {
    if (typeof window !== 'undefined') {
        window._aeOutputStreamRunning = Boolean(es && es.stream && es.stream.running === true);
    }
    const streamEl = document.getElementById('aeStreamStatus');
    const inputStreamEl = document.getElementById('aeInputStreamStatus');
    if (streamEl) {
        if (es && es.stream) {
            fillAeStreamLineFromPayload(es.stream, streamEl);
        } else {
            streamEl.textContent = '—';
        }
    }
    if (inputStreamEl) {
        if (es && es.input_stream) {
            fillAeInputStreamLineFromPayload(es.input_stream, inputStreamEl);
        } else {
            inputStreamEl.textContent = '—';
        }
    }
    syncAeInputPeakPollFromEngineState(es);
    syncAeOutputGraphsAfterStreamStateChange();
}

/**
 * After a failed IPC action, re-read `engine_state` when possible so stream lines match the AudioEngine; else clear.
 * @returns {Promise<object|null>} last `engine_state` payload when `ok === true`, else `null`
 */
async function fillAeStreamsAfterEngineError() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        fillAeStreamsFromEngineState(null);
        return null;
    }
    try {
        const es = await inv({cmd: 'engine_state'});
        if (es && es.ok === true) {
            fillAeStreamsFromEngineState(es);
            return es;
        }
        fillAeStreamsFromEngineState(null);
        return null;
    } catch {
        fillAeStreamsFromEngineState(null);
        return null;
    }
}

/** Clear stream lines and show `ui.ae.err_no_ipc` on `#aeEngineStatus` (no `audioEngineInvoke`). */
function aeNotifyNoAudioEngineIpc() {
    fillAeStreamsFromEngineState(null);
    const statusEl = document.getElementById('aeEngineStatus');
    if (statusEl && typeof catalogFmt === 'function') {
        statusEl.textContent = catalogFmt('ui.ae.err_no_ipc');
    }
}

/**
 * @param {HTMLElement|null} statusEl
 * @param {object} es — `engine_state` with `ok`, `version`, `host`
 */
function fillAeEngineStatusOkFromState(statusEl, es) {
    if (!statusEl || !es || es.ok !== true || typeof catalogFmt !== 'function') return;
    const ver = es.version != null ? String(es.version) : '?';
    const host = es.host != null ? String(es.host) : '?';
    statusEl.textContent = catalogFmt('ui.ae.status_ok', {version: ver, host});
}

/**
 * @param {HTMLElement|null} statusEl
 * @param {unknown} err
 */
function fillAeEngineStatusFromError(statusEl, err) {
    if (!statusEl || typeof catalogFmt !== 'function') return;
    const msg = err && err.message ? String(err.message) : String(err);
    statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
}

/**
 * @param {object|null|undefined} r — IPC JSON response with optional `error`
 * @param {string} fallback — when `r` missing or `r.error` absent
 */
function throwIfAeNotOk(r, fallback) {
    if (r && r.ok === true) return;
    const err = (r && r.error) ? String(r.error) : fallback;
    throw new Error(err);
}

/**
 * @param {HTMLInputElement|null|undefined} toneCb
 * @param {object|null|undefined} stream — `engine_state.stream`
 */
function syncAeToneCheckboxFromStream(toneCb, stream) {
    if (!toneCb || !stream) return;
    toneCb.disabled = !(stream.running === true && stream.tone_supported === true);
    if (stream.tone_on != null) toneCb.checked = stream.tone_on === true;
}

/**
 * Rebuild `#aeInputDevice` options (system-default row + devices) and apply `inPick` with fallback.
 * @param {HTMLSelectElement} selectEl
 * @param {object[]} devices
 * @param {string} inPick — desired value; `''` = system default
 */
function aePopulateInputDeviceSelectOptions(selectEl, devices, inPick) {
    if (!selectEl || typeof selectEl.replaceChildren !== 'function' || typeof catalogFmt !== 'function') return;
    const list = Array.isArray(devices) ? devices : [];
    selectEl.replaceChildren();
    const defOpt = document.createElement('option');
    defOpt.value = '';
    defOpt.textContent = catalogFmt('ui.ae.input_device_default_option');
    selectEl.appendChild(defOpt);
    for (const d of list) {
        const id = d.id != null ? String(d.id) : '';
        const name = d.name != null ? String(d.name) : id;
        const opt = document.createElement('option');
        opt.value = id;
        opt.textContent = name;
        if (d.is_default === true) {
            opt.dataset.default = '1';
        }
        selectEl.appendChild(opt);
    }
    if (inPick !== '') {
        selectEl.value = inPick;
    }
    const valid = inPick === '' || [...selectEl.options].some((o) => o.value === inPick);
    if (!valid && list.length > 0) {
        const defD = list.find((x) => x.is_default === true);
        selectEl.value = defD && defD.id != null ? String(defD.id) : String(list[0].id);
    } else if (!valid) {
        selectEl.value = '';
    }
}

/**
 * JUCE audio device type (CoreAudio, ASIO, etc.): applies to both managers; stops streams.
 */
async function aeApplyAudioDeviceTypeChange() {
    const inv = getAeAudioEngineInvoke();
    const typeSel = document.getElementById('aeAudioDeviceType');
    const statusEl = document.getElementById('aeEngineStatus');
    if (!inv || !typeSel) {
        if (!inv) aeNotifyNoAudioEngineIpc();
        return;
    }
    const t = typeSel.value != null ? String(typeSel.value) : '';
    if (t === '') return;
    try {
        const r = await inv({cmd: 'set_audio_device_type', type: t});
        throwIfAeNotOk(r, 'set_audio_device_type failed');
        if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
            prefs.setItem(AE_PREFS_DEVICE_TYPE, t);
        }
        await refreshAudioEnginePanel();
    } catch (e) {
        await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
        await refreshAudioEnginePanel();
    }
}

/**
 * Reload engine_state (ping + stream), device list, caps, plugin stub.
 */
async function refreshAudioEnginePanel() {
    const statusEl = document.getElementById('aeEngineStatus');
    const selectEl = document.getElementById('aeOutputDevice');
    const typeSelectEl = document.getElementById('aeAudioDeviceType');
    const toneCb = document.getElementById('aeTestTone');
    const inv = getAeAudioEngineInvoke();

    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        void refreshAeProcessStats();
        return;
    }

    aePluginChainPollGeneration++;
    const pollGen = aePluginChainPollGeneration;

    if (statusEl && typeof catalogFmt === 'function') {
        statusEl.textContent = catalogFmt('ui.ae.status_loading');
    }

    try {
        const es = await inv({cmd: 'engine_state'});
        throwIfAeNotOk(es, 'engine_state failed');
        fillAeEngineStatusOkFromState(statusEl, es);
        fillAeStreamsFromEngineState(es);
        syncAeToneCheckboxFromStream(toneCb, es.stream);

        let typeRes = await inv({cmd: 'list_audio_device_types'});
        throwIfAeNotOk(typeRes, 'list_audio_device_types failed');
        if (!aeInitialDeviceTypeRestored) {
            const savedType =
                typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                    ? prefs.getItem(AE_PREFS_DEVICE_TYPE)
                    : null;
            const cur = typeRes.current != null ? String(typeRes.current) : '';
            const shouldRestore =
                savedType != null && String(savedType).trim() !== '' && String(savedType) !== cur;
            const streamsActive =
                (es && es.stream && es.stream.running === true) ||
                (es && es.input_stream && es.input_stream.running === true);
            if (shouldRestore && !streamsActive) {
                try {
                    const sr = await inv({cmd: 'set_audio_device_type', type: String(savedType).trim()});
                    throwIfAeNotOk(sr, 'set_audio_device_type failed');
                    typeRes = await inv({cmd: 'list_audio_device_types'});
                    throwIfAeNotOk(typeRes, 'list_audio_device_types failed');
                } catch {
                    /* keep engine driver */
                }
            }
            /* Defer completion while saved driver still differs but output/input is running — `set_audio_device_type` stops all streams (would cut library playback). Retry on later refresh. */
            if (!shouldRestore || !streamsActive) {
                aeInitialDeviceTypeRestored = true;
            }
        }
        if (typeSelectEl) {
            aePopulateAudioDeviceTypeSelect(typeSelectEl, typeRes);
        }

        const list = await inv({cmd: 'list_output_devices'});
        throwIfAeNotOk(list, 'list_output_devices failed');
        const devices = Array.isArray(list.devices) ? list.devices : [];
        const saved = typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
            ? prefs.getItem(AE_PREFS_DEVICE)
            : null;
        let pick = saved || (list.default_device_id != null ? String(list.default_device_id) : null);

        if (selectEl && typeof selectEl.replaceChildren === 'function') {
            selectEl.replaceChildren();
            for (const d of devices) {
                const id = d.id != null ? String(d.id) : '';
                const name = d.name != null ? String(d.name) : id;
                const opt = document.createElement('option');
                opt.value = id;
                opt.textContent = name;
                if (d.is_default === true) {
                    opt.dataset.default = '1';
                }
                selectEl.appendChild(opt);
            }
            if (pick != null && pick !== '') {
                selectEl.value = pick;
            }
            const valid = pick != null && pick !== '' && [...selectEl.options].some((o) => o.value === pick);
            if (!valid && devices.length > 0) {
                const def = devices.find((x) => x.is_default === true);
                selectEl.value = def && def.id != null ? String(def.id) : String(devices[0].id);
            }
        }

        const selId = selectEl && selectEl.value ? String(selectEl.value) : '';
        await fillAeDeviceCaps(inv, selId);

        const chain = await inv({cmd: 'plugin_chain'});
        fillAePluginSection(chain);
        if (chain && chain.phase === 'scanning') {
            void fetchPluginChainUntilSettled(inv, chain, pollGen).then((finalChain) => {
                if (pollGen !== aePluginChainPollGeneration) return;
                fillAePluginSection(finalChain);
            });
        }

        syncAePlaybackControlsFromPrefs();

        const inListEl = document.getElementById('aeInputDevicesList');
        const inSelectEl = document.getElementById('aeInputDevice');
        try {
            const ins = await inv({cmd: 'list_input_devices'});
            if (inListEl && ins && ins.ok === true && Array.isArray(ins.devices)) {
                const lines = ins.devices.map((d) => {
                    const id = d.id != null ? String(d.id) : '';
                    const name = d.name != null ? String(d.name) : id;
                    const def = d.is_default === true ? ' *' : '';
                    return `${name} (${id})${def}`;
                });
                inListEl.textContent = lines.length ? lines.join('\n') : '—';

                const inSaved = typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                    ? prefs.getItem(AE_PREFS_INPUT_DEVICE)
                    : null;
                /** `null`/`undefined`: never saved — follow cpal default id; `''`: user chose system default. */
                let inPick;
                if (inSaved === null || inSaved === undefined) {
                    inPick = ins.default_device_id != null ? String(ins.default_device_id) : '';
                } else {
                    inPick = String(inSaved);
                }
                if (inSelectEl && typeof inSelectEl.replaceChildren === 'function' && typeof catalogFmt === 'function') {
                    aePopulateInputDeviceSelectOptions(inSelectEl, ins.devices, inPick);
                    await fillAeInputDeviceCaps(inv, inSelectEl.value);
                } else {
                    await fillAeInputDeviceCaps(inv, inPick);
                }
            } else if (inListEl) {
                inListEl.textContent = '—';
                aePopulateInputDeviceSelectOptions(inSelectEl, [], '');
                await fillAeInputDeviceCaps(inv, '');
            }
        } catch {
            if (inListEl) inListEl.textContent = '—';
            await fillAeInputDeviceCaps(inv, '');
        }
    } catch (e) {
        fillAeStreamsFromEngineState(null);
        fillAeEngineStatusFromError(statusEl, e);
    } finally {
        void refreshAeProcessStats();
    }
}

/**
 * Toggle test tone on the live stream (F32 only).
 * @param {boolean} enabled
 */
async function toggleAeTestTone(enabled) {
    const inv = getAeAudioEngineInvoke();
    const statusEl = document.getElementById('aeEngineStatus');
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }
    try {
        const r = await inv({cmd: 'set_output_tone', tone: enabled});
        throwIfAeNotOk(r, 'set_output_tone failed');
        const es = await inv({cmd: 'engine_state'});
        fillAeStreamsFromEngineState(es);
        const toneCb = document.getElementById('aeTestTone');
        syncAeToneCheckboxFromStream(toneCb, es.stream);
    } catch (e) {
        const es = await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
        const toneCb = document.getElementById('aeTestTone');
        if (es && es.stream) {
            syncAeToneCheckboxFromStream(toneCb, es.stream);
        } else if (toneCb) {
            toneCb.checked = !enabled;
        }
    }
}

/**
 * Best-effort: open cpal/JUCE output once prefs are loaded so spectrum/visualizer and tone
 * work without visiting Audio Engine → Apply. Uses saved `audioEngineOutputDeviceId` or `""` (driver default).
 * Idempotent if `engine_state.stream.running` is already true.
 */
async function ensureAeOutputStreamOnStartup() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    try {
        let es = await inv({cmd: 'engine_state'});
        throwIfAeNotOk(es, 'engine_state failed');
        if (es.stream && es.stream.running === true) {
            fillAeStreamsFromEngineState(es);
            return;
        }

        let typeRes = await inv({cmd: 'list_audio_device_types'});
        throwIfAeNotOk(typeRes, 'list_audio_device_types failed');
        if (!aeInitialDeviceTypeRestored) {
            const savedType =
                typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                    ? prefs.getItem(AE_PREFS_DEVICE_TYPE)
                    : null;
            const cur = typeRes.current != null ? String(typeRes.current) : '';
            const shouldRestore =
                savedType != null && String(savedType).trim() !== '' && String(savedType) !== cur;
            const streamsActive =
                (es && es.stream && es.stream.running === true) ||
                (es && es.input_stream && es.input_stream.running === true);
            if (shouldRestore && !streamsActive) {
                try {
                    const sr = await inv({cmd: 'set_audio_device_type', type: String(savedType).trim()});
                    throwIfAeNotOk(sr, 'set_audio_device_type failed');
                    typeRes = await inv({cmd: 'list_audio_device_types'});
                    throwIfAeNotOk(typeRes, 'list_audio_device_types failed');
                } catch {
                    /* keep engine driver */
                }
            }
            if (!shouldRestore || !streamsActive) {
                aeInitialDeviceTypeRestored = true;
            }
        }

        let playbackLoaded = false;
        try {
            const st = await inv(buildEnginePlaybackStatusRequest());
            playbackLoaded = Boolean(st && st.ok && st.loaded === true);
        } catch {
            /* ignore */
        }
        const resumePath =
            typeof window !== 'undefined' && typeof window._enginePlaybackResumePath === 'string' && window._enginePlaybackResumePath.length > 0
                ? window._enginePlaybackResumePath
                : '';
        let didReloadLibrary = false;
        /** @type {object | null} */
        let loadMeta = null;
        if (!playbackLoaded && resumePath) {
            const lr = await inv({cmd: 'playback_load', path: resumePath});
            throwIfAeNotOk(lr, 'playback_load failed');
            playbackLoaded = true;
            didReloadLibrary = true;
            loadMeta = lr;
        }

        const deviceId =
            typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                ? prefs.getItem(AE_PREFS_DEVICE) || ''
                : '';
        const toneOn =
            typeof prefs !== 'undefined' && typeof prefs.getItem === 'function' && prefs.getItem(AE_PREFS_TONE) === '1';
        const bfRaw =
            typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                ? prefs.getItem(AE_PREFS_BUFFER_FRAMES_OUTPUT)
                : '';
        const bufferFrames = parseAeBufferFramesPref(bfRaw != null ? String(bfRaw) : '');
        const srRaw =
            typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                ? prefs.getItem(AE_PREFS_SAMPLE_RATE_HZ)
                : '';
        const srHz = parseAeSampleRateHzFromPrefString(srRaw);

        const startPayload = {cmd: 'start_output_stream', device_id: deviceId, tone: toneOn};
        if (bufferFrames !== undefined) {
            startPayload.buffer_frames = bufferFrames;
        }
        if (srHz !== undefined) {
            startPayload.sample_rate_hz = srHz;
        }
        if (playbackLoaded) {
            startPayload.start_playback = true;
        }
        const start = await inv(startPayload);
        throwIfAeNotOk(start, 'start_output_stream failed');
        if (didReloadLibrary && typeof window !== 'undefined' && typeof window.resumeEnginePlaybackAfterApply === 'function') {
            window.resumeEnginePlaybackAfterApply(loadMeta);
        }
        if (playbackLoaded) {
            if (typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
                window.syncEnginePlaybackDspFromPrefs();
            }
            if (typeof window.syncEnginePlaybackSpeedFromPrefs === 'function') {
                window.syncEnginePlaybackSpeedFromPrefs();
            }
            if (typeof startEnginePlaybackPoll === 'function') {
                startEnginePlaybackPoll();
            }
        }
        es = await inv({cmd: 'engine_state'});
        fillAeStreamsFromEngineState(es);
        if (typeof window !== 'undefined' && typeof window.applyAeEqCanvasHeightFromPrefs === 'function') {
            window.applyAeEqCanvasHeightFromPrefs();
        }
    } catch {
        try {
            const inv2 = getAeAudioEngineInvoke();
            if (inv2) {
                const es = await inv2({cmd: 'engine_state'});
                if (es && es.ok === true) {
                    fillAeStreamsFromEngineState(es);
                }
            }
        } catch {
            /* ignore */
        }
    }
}

async function applyAudioEngineDevice() {
    const selectEl = document.getElementById('aeOutputDevice');
    const statusEl = document.getElementById('aeEngineStatus');
    const toneCb = document.getElementById('aeTestTone');
    const bufOut = document.getElementById('aeBufferFramesOutput');
    const inv = getAeAudioEngineInvoke();
    if (!inv || !selectEl) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_debug_device_missing', {inv: !!inv, selectEl: !!selectEl}), 3000, 'error');
        if (!inv) aeNotifyNoAudioEngineIpc();
        return;
    }

    const id = selectEl.value;
    const toneOn = toneCb && toneCb.checked === true;
    const bfRaw = bufOut && typeof bufOut.value === 'string' ? bufOut.value : '';
    const bufferFrames = parseAeBufferFramesPref(bfRaw);
    if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        prefs.setItem(AE_PREFS_DEVICE, id);
        prefs.setItem(AE_PREFS_TONE, toneOn ? '1' : '0');
        prefs.setItem(AE_PREFS_BUFFER_FRAMES_OUTPUT, bfRaw.trim());
        const srSel = document.getElementById('aeSampleRate');
        if (srSel) {
            prefs.setItem(AE_PREFS_SAMPLE_RATE_HZ, srSel.value != null ? String(srSel.value).trim() : '');
        }
    }

    try {
        /* `start_output_stream` validates `device_id` and calls `stopOutputLocked` first — no separate `set_output_device` round-trip. */
        /* If a library track is loaded (`playback_load`), reconnect file PCM to the new stream.
         * Omitting `start_playback` leaves silence/tone-only output while the session still exists — breaks preview.
         * After Stop stream, `playback_stop` clears the session — reload from `window._enginePlaybackResumePath` if set. */
        let playbackLoaded = false;
        try {
            const st = await inv(buildEnginePlaybackStatusRequest());
            playbackLoaded = Boolean(st && st.ok && st.loaded === true);
        } catch {
            /* ignore */
        }
        const resumePath =
            typeof window !== 'undefined' && typeof window._enginePlaybackResumePath === 'string' && window._enginePlaybackResumePath.length > 0
                ? window._enginePlaybackResumePath
                : '';
        let didReloadLibrary = false;
        /** @type {object | null} */
        let loadMeta = null;
        if (!playbackLoaded && resumePath) {
            const lr = await inv({cmd: 'playback_load', path: resumePath});
            throwIfAeNotOk(lr, 'playback_load failed');
            playbackLoaded = true;
            didReloadLibrary = true;
            loadMeta = lr;
        }
        const startPayload = {cmd: 'start_output_stream', device_id: id, tone: toneOn};
        if (bufferFrames !== undefined) {
            startPayload.buffer_frames = bufferFrames;
        }
        const srSelApply = document.getElementById('aeSampleRate');
        const srHz = parseAeSampleRateHzFromSelect(srSelApply);
        if (srHz !== undefined) {
            startPayload.sample_rate_hz = srHz;
        }
        if (playbackLoaded) {
            startPayload.start_playback = true;
        }
        const start = await inv(startPayload);
        throwIfAeNotOk(start, 'start_output_stream failed');
        if (didReloadLibrary && typeof window !== 'undefined' && typeof window.resumeEnginePlaybackAfterApply === 'function') {
            window.resumeEnginePlaybackAfterApply(loadMeta);
        }
        if (playbackLoaded && typeof window !== 'undefined') {
            if (typeof window.syncEnginePlaybackDspFromPrefs === 'function') {
                window.syncEnginePlaybackDspFromPrefs();
            }
            if (typeof window.syncEnginePlaybackSpeedFromPrefs === 'function') {
                window.syncEnginePlaybackSpeedFromPrefs();
            }
        }
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_applied_stream', {id});
        }
        await fillAeDeviceCaps(inv, id);
        const es = await inv({cmd: 'engine_state'});
        fillAeStreamsFromEngineState(es);
        syncAeToneCheckboxFromStream(toneCb, es.stream);
        if (typeof window !== 'undefined' && typeof window.applyAeEqCanvasHeightFromPrefs === 'function') {
            window.applyAeEqCanvasHeightFromPrefs();
        }
        if (es && es.stream && es.stream.running === true && typeof startEnginePlaybackPoll === 'function') {
            startEnginePlaybackPoll();
        }
    } catch (e) {
        const es = await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
        if (es && es.stream) syncAeToneCheckboxFromStream(toneCb, es.stream);
    }
}

async function startAeInputCapture() {
    const statusEl = document.getElementById('aeEngineStatus');
    const inSel = document.getElementById('aeInputDevice');
    const bufInCap = document.getElementById('aeBufferFramesInput');
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }

    const id = inSel && inSel.value != null ? String(inSel.value) : '';
    const bfRaw = bufInCap && typeof bufInCap.value === 'string' ? bufInCap.value : '';
    const bufferFrames = parseAeBufferFramesPref(bfRaw);
    if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        prefs.setItem(AE_PREFS_BUFFER_FRAMES_INPUT, bfRaw.trim());
    }

    try {
        const payload = {cmd: 'start_input_stream'};
        if (id !== '') {
            payload.device_id = id;
        }
        if (bufferFrames !== undefined) {
            payload.buffer_frames = bufferFrames;
        }
        const srSelIn = document.getElementById('aeSampleRate');
        const srHzIn = parseAeSampleRateHzFromSelect(srSelIn);
        if (srHzIn !== undefined) {
            payload.sample_rate_hz = srHzIn;
        }
        const r = await inv(payload);
        throwIfAeNotOk(r, 'start_input_stream failed');
        const es = await inv({cmd: 'engine_state'});
        fillAeStreamsFromEngineState(es);
        fillAeEngineStatusOkFromState(statusEl, es);
    } catch (e) {
        await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
    }
}

async function stopAeInputCapture() {
    const statusEl = document.getElementById('aeEngineStatus');
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }

    try {
        const r = await inv({cmd: 'stop_input_stream'});
        throwIfAeNotOk(r, 'stop_input_stream failed');
        const es = await inv({cmd: 'engine_state'});
        fillAeStreamsFromEngineState(es);
        fillAeEngineStatusOkFromState(statusEl, es);
    } catch (e) {
        await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
    }
}

/**
 * @param {object} [opts]
 * @param {boolean} [opts.throwOnFailure=false] — if true, rethrow after UI error state (used before `playback_set_inserts`).
 */
async function stopAeOutputStream(opts) {
    const throwOnFailure = opts != null && opts.throwOnFailure === true;
    const statusEl = document.getElementById('aeEngineStatus');
    const toneCb = document.getElementById('aeTestTone');
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }

    try {
        const r = await inv({cmd: 'stop_output_stream'});
        throwIfAeNotOk(r, 'stop_output_stream failed');
        try {
            await inv({cmd: 'playback_stop'});
        } catch {
            /* session may already be clear */
        }
        if (typeof window.syncEnginePlaybackStoppedFromAudioEngine === 'function') {
            window.syncEnginePlaybackStoppedFromAudioEngine();
        }
        const es = await inv({cmd: 'engine_state'});
        fillAeEngineStatusOkFromState(statusEl, es);
        fillAeStreamsFromEngineState(es);
        if (toneCb) {
            toneCb.disabled = true;
            toneCb.checked = false;
        }
    } catch (e) {
        const es = await fillAeStreamsAfterEngineError();
        fillAeEngineStatusFromError(statusEl, e);
        if (es && es.stream) syncAeToneCheckboxFromStream(toneCb, es.stream);
        if (throwOnFailure) throw e;
    }
}

// ── Library playback via AudioEngine (PCM + EQ in engine; WebView stays silent) ──

/** WebView `playback_status` interval — the poll also carries the spectrum frame data for
 * the main window's FFT render, so the effective FFT refresh rate is capped at `1000 / this`
 * (the rAF render loop re-reads the same `_engineSpectrumU8` buffer between polls). 250 ms
 * (4 Hz) looked visibly choppy for spectrum analysis; 33 ms gives ~30 Hz which is smooth.
 * The previous CPU concern that drove it up to 250 was actually traced to PDF metadata
 * generation, not this poll. */
const ENGINE_PLAYBACK_POLL_MS = 33;

/** @type {ReturnType<typeof setInterval> | null} */
let _enginePlaybackPollTimer = null;

/** True between **`startEnginePlaybackPoll`** and **`stopEnginePlaybackPoll`** (timer may be cleared while idle). */
let _enginePlaybackPollSessionActive = false;

/** @type {boolean} */
let _enginePlaybackIdleHooked = false;

/**
 * True when the WebView should not run the **`playback_status`** **`setInterval`** — hidden tab,
 * unfocused window, minimized, etc. (`isUiIdleHeavyCpu` in **`ui-idle.js`**). Host EOF watchdog covers
 * EOF while idle so the engine is not polled twice.
 *
 * **Exception:** while a sample loop region / A-B loop is active we *must* keep the JS poll running
 * even when idle — the wrap-back `playback_seek` lives inside `updatePlaybackTime()`, which is
 * called from this very poll (and from the rAF loop, which `ui-idle.js` also cancels). Without
 * the poll, playback runs past the loop end until the window becomes active again.
 */
function shouldDeferPlaybackPollToHostWatchdog() {
    if (typeof window !== 'undefined' && typeof window.isAbLoopActive === 'function') {
        try { if (window.isAbLoopActive()) return false; } catch {}
    }
    if (typeof window.isUiIdleHeavyCpu === 'function') {
        return window.isUiIdleHeavyCpu();
    }
    return typeof document !== 'undefined' && document.hidden;
}

/**
 * Host EOF watchdog runs while **`shouldDeferPlaybackPollToHostWatchdog()`** so foreground-focused
 * playback does not double `playback_status` IPC with the WebView poll.
 */
function syncEnginePlaybackEofWatchdog() {
    if (!_enginePlaybackPollSessionActive) {
        return;
    }
    try {
        const u = typeof window !== 'undefined' ? window.vstUpdater : undefined;
        if (!u) {
            return;
        }
        if (shouldDeferPlaybackPollToHostWatchdog()) {
            if (typeof u.audioEngineEofWatchdogStart === 'function') {
                void u.audioEngineEofWatchdogStart().catch(() => {});
            }
        } else if (typeof u.audioEngineEofWatchdogStop === 'function') {
            void u.audioEngineEofWatchdogStop();
        }
    } catch (_) {
        /* non-Tauri */
    }
}

function syncEnginePlaybackPollForUiIdle() {
    if (!_enginePlaybackPollSessionActive) {
        return;
    }
    if (shouldDeferPlaybackPollToHostWatchdog()) {
        if (_enginePlaybackPollTimer != null) {
            clearInterval(_enginePlaybackPollTimer);
            _enginePlaybackPollTimer = null;
        }
        syncEnginePlaybackEofWatchdog();
    } else {
        if (_enginePlaybackPollTimer == null) {
            _enginePlaybackPollTimer = setInterval(() => void runEnginePlaybackStatusTick(), ENGINE_PLAYBACK_POLL_MS);
            void runEnginePlaybackStatusTick();
        }
        syncEnginePlaybackEofWatchdog();
    }
}

function _haltLibraryPlaybackPollIntervalAndWatchdog() {
    if (_enginePlaybackPollTimer != null) {
        clearInterval(_enginePlaybackPollTimer);
        _enginePlaybackPollTimer = null;
    }
    try {
        const u = typeof window !== 'undefined' ? window.vstUpdater : undefined;
        if (u && typeof u.audioEngineEofWatchdogStop === 'function') {
            void u.audioEngineEofWatchdogStop();
        }
    } catch (_) {
        /* non-Tauri */
    }
}

function stopEnginePlaybackPoll() {
    _enginePlaybackPollSessionActive = false;
    _haltLibraryPlaybackPollIntervalAndWatchdog();
    startAeTabMeterPollIfNeeded();
    scheduleAeGraphRafLoop();
}

function applyPlaybackStatusScope(st) {
    if (!st || st.ok !== true) return;
    const n = typeof st.scope_len === 'number' ? st.scope_len : 0;
    if (
        n >= 16 &&
        Array.isArray(st.scope_l) &&
        Array.isArray(st.scope_r) &&
        st.scope_l.length === n &&
        st.scope_r.length === n
    ) {
        if (!window._engineScopeL || window._engineScopeL.length !== n) {
            window._engineScopeL = new Uint8Array(n);
            window._engineScopeR = new Uint8Array(n);
        }
        const L = window._engineScopeL;
        const R = window._engineScopeR;
        for (let i = 0; i < n; i++) {
            const a = st.scope_l[i];
            const b = st.scope_r[i];
            L[i] = typeof a === 'number' ? Math.max(0, Math.min(255, Math.round(a))) : 128;
            R[i] = typeof b === 'number' ? Math.max(0, Math.min(255, Math.round(b))) : 128;
        }
        window._engineScopeLen = n;
    } else if (st.loaded !== true) {
        window._engineScopeL = null;
        window._engineScopeR = null;
        window._engineScopeLen = 0;
    }
}

function applyPlaybackStatusSpectrum(st) {
    if (!st || st.ok !== true) return;
    if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        migrateAePlaybackSpectrumSplitPrefs();
        if (normalizeAePlaybackSpectrumFftOrderPref(prefs.getItem(AE_PREFS_PLAYBACK_SPECTRUM_FFT_ORDER)) === 'off') {
            window._engineSpectrumU8 = null;
            return;
        }
    }
    if (Array.isArray(st.spectrum) && st.spectrum.length >= ENGINE_PLAYBACK_SPECTRUM_MIN_BINS) {
        const n = st.spectrum.length;
        if (!window._engineSpectrumU8 || window._engineSpectrumU8.length !== n) {
            window._engineSpectrumU8 = new Uint8Array(n);
        }
        const u8 = window._engineSpectrumU8;
        for (let i = 0; i < n; i++) {
            const v = st.spectrum[i];
            u8[i] = typeof v === 'number' ? Math.max(0, Math.min(255, Math.round(v))) : 0;
        }
        window._engineSpectrumFftSize = typeof st.spectrum_fft_size === 'number' ? st.spectrum_fft_size : 2048;
        window._engineSpectrumSrHz = typeof st.spectrum_sr_hz === 'number' ? st.spectrum_sr_hz : 44100;
    } else {
        // Engine often omits or zeroes spectrum while the ring warms up, or between polls. Do not
        // wipe bins during an active loaded session — stereo/levels tiles need `_engineSpectrumU8`
        // even when Web Audio `_analyserL`/`_analyserR` were never wired (AudioEngine-only playback).
        if (st.loaded !== true) {
            window._engineSpectrumU8 = null;
        }
    }
}

function stopAeTabMeterPoll() {
    if (aeTabMeterPollTimer != null) {
        clearInterval(aeTabMeterPollTimer);
        aeTabMeterPollTimer = null;
    }
}

function stopAeGraphRaf() {
    if (aeGraphRafId !== 0) {
        cancelAnimationFrame(aeGraphRafId);
        aeGraphRafId = 0;
    }
}

function stopAeTabMeterAndGraph() {
    stopAeTabMeterPoll();
    stopAeGraphRaf();
}

function shouldRunAeTabMeterPoll() {
    if (!aeAudioEngineTabIsActive()) return false;
    if (typeof window === 'undefined' || !window._aeOutputStreamRunning) return false;
    if (_enginePlaybackPollSessionActive) return false;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return false;
    if (typeof document !== 'undefined' && document.hidden) return false;
    return getAeAudioEngineInvoke() != null;
}

const _aeDiagGraphIds = [
    'aeGraphMidSide',
    'aeGraphBalance',
    'aeGraphCorrelation',
    'aeGraphWidth',
    'aeGraphCrest',
    'aeGraphLMinusR',
    'aeGraphEnergy',
    'aeGraphGonio',
    'aeGraphDcOffset',
    'aeGraphMagHist',
    'aeGraphPeakSample',
    'aeGraphMonoWave',
    'aeGraphSideWave',
    'aeGraphLrOverlay',
    'aeGraphAbsDiffHist',
    'aeGraphLissajous',
];

/** True if at least one visible diagnostic canvas is not per-graph frozen (otherwise rAF can stop). */
function aeDiagAnyGraphUnfrozen() {
    let saw = false;
    for (let i = 0; i < _aeDiagGraphIds.length; i++) {
        const el = document.getElementById(_aeDiagGraphIds[i]);
        if (!el) continue;
        saw = true;
        const fid =
            typeof window.aeCanvasIdToGraphFreezeId === 'function'
                ? window.aeCanvasIdToGraphFreezeId(_aeDiagGraphIds[i])
                : null;
        if (!fid) return true;
        if (typeof window.isGraphFrozen !== 'function' || !window.isGraphFrozen(fid)) return true;
    }
    return false;
}

function shouldDrawAeOutputGraphs() {
    if (!aeAudioEngineTabIsActive()) return false;
    if (typeof window === 'undefined' || !window._aeOutputStreamRunning) return false;
    if (typeof document !== 'undefined' && document.hidden) return false;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return false;
    if (!aeDiagAnyGraphUnfrozen()) return false;
    return true;
}

function layoutAeOutputGraphCanvases() {
    const ids = _aeDiagGraphIds;
    const dpr =
        typeof window !== 'undefined' && typeof window.devicePixelRatio === 'number' && window.devicePixelRatio > 0
            ? window.devicePixelRatio
            : 1;
    for (let i = 0; i < ids.length; i++) {
        const c = document.getElementById(ids[i]);
        if (!c || typeof c.getBoundingClientRect !== 'function') continue;
        const wrap = c.parentElement;
        const rw = wrap ? wrap.clientWidth : 0;
        const rh = wrap ? wrap.clientHeight : 0;
        if (rw < 2 || rh < 2) continue;
        const w = Math.max(2, Math.floor(rw * dpr));
        const h = Math.max(2, Math.floor(rh * dpr));
        if (c.width !== w || c.height !== h) {
            c.width = w;
            c.height = h;
        }
    }
}

function bindAeOutputGraphIdleOnce() {
    if (aeOutputGraphIdleBound) return;
    aeOutputGraphIdleBound = true;
    if (typeof document === 'undefined' || typeof document.addEventListener !== 'function') return;
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            stopAeTabMeterPoll();
            stopAeGraphRaf();
        } else {
            layoutAeOutputGraphCanvases();
            startAeTabMeterPollIfNeeded();
            scheduleAeGraphRafLoop();
        }
    });
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        const idle = e.detail && e.detail.idle;
        if (idle) {
            stopAeTabMeterPoll();
            stopAeGraphRaf();
        } else {
            startAeTabMeterPollIfNeeded();
            scheduleAeGraphRafLoop();
        }
    });
    const aeStack = document.querySelector('#tabAudioEngine .ae-main-stack');
    if (aeStack && typeof ResizeObserver === 'function') {
        aeGraphResizeObs = new ResizeObserver(() => {
            if (!aeAudioEngineTabIsActive()) return;
            layoutAeOutputGraphCanvases();
        });
        aeGraphResizeObs.observe(aeStack);
    }
    document.addEventListener('graph-freeze-changed', () => {
        if (!aeAudioEngineTabIsActive()) return;
        layoutAeOutputGraphCanvases();
        if (shouldDrawAeOutputGraphs()) scheduleAeGraphRafLoop();
        else {
            aeDrawOutputGraphs();
            stopAeGraphRaf();
        }
    });
}

/** Last engine L/R scope per diagnostic graph id when live; used when that graph is frozen. */
const _aeGraphScopeSnapByFreezeId = Object.create(null);

/**
 * @param {string|null} freezeId
 * @param {Uint8Array|null} liveL
 * @param {Uint8Array|null} liveR
 * @param {number} liveN
 * @returns {{sl: Uint8Array|null, sr: Uint8Array|null, n: number}}
 */
function aeGraphResolveScopeForFreezeId(freezeId, liveL, liveR, liveN) {
    const frozen =
        !!freezeId && typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(freezeId);
    const n = typeof liveN === 'number' ? liveN : 0;
    const liveOk = n >= 16 && liveL && liveR;
    if (!frozen) {
        if (liveOk && freezeId) {
            let ent = _aeGraphScopeSnapByFreezeId[freezeId];
            if (!ent || ent.n !== n) {
                ent = {sl: new Uint8Array(liveL), sr: new Uint8Array(liveR), n};
                _aeGraphScopeSnapByFreezeId[freezeId] = ent;
            } else {
                ent.sl.set(liveL);
                ent.sr.set(liveR);
            }
        }
        return {sl: liveL, sr: liveR, n: liveN};
    }
    const ent = freezeId ? _aeGraphScopeSnapByFreezeId[freezeId] : null;
    if (ent && ent.n >= 16 && ent.sl && ent.sr) {
        return {sl: ent.sl, sr: ent.sr, n: ent.n};
    }
    return {sl: liveL, sr: liveR, n: liveN};
}

/**
 * @param {string} canvasId
 * @param {function(): void} draw
 */
function aeGraphWithScopeForCanvasId(canvasId, draw) {
    const origL = typeof window !== 'undefined' ? window._engineScopeL : null;
    const origR = typeof window !== 'undefined' ? window._engineScopeR : null;
    const origLen =
        typeof window !== 'undefined' && typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const fid =
        typeof window.aeCanvasIdToGraphFreezeId === 'function' ? window.aeCanvasIdToGraphFreezeId(canvasId) : null;
    const sc = aeGraphResolveScopeForFreezeId(fid, origL, origR, origLen);
    window._engineScopeL = sc.sl;
    window._engineScopeR = sc.sr;
    window._engineScopeLen = sc.n;
    try {
        draw();
    } finally {
        window._engineScopeL = origL;
        window._engineScopeR = origR;
        window._engineScopeLen = origLen;
    }
}

function aeGraphFillBackdrop(ctx, w, h) {
    ctx.fillStyle = 'rgba(6,8,22,0.88)';
    ctx.fillRect(0, 0, w, h);
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, 'rgba(5,217,232,0.06)');
    g.addColorStop(1, 'rgba(211,0,197,0.04)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
}

/**
 * Pearson L/R correlation in [-1, 1] and stereo width RMS(side)/RMS(mid) on engine scope samples.
 * @param {Uint8Array} sl
 * @param {Uint8Array} sr
 * @param {number} n
 * @returns {{ corr: number|null, width: number|null }}
 */
function aeEngineScopeStereoMetrics(sl, sr, n) {
    if (n < 16 || !sl || !sr) {
        return {corr: null, width: null};
    }
    const step = Math.max(1, Math.floor(n / 420));
    let cnt = 0;
    let sumL = 0;
    let sumR = 0;
    for (let i = 0; i < n; i += step) {
        sumL += (sl[i] - 128) / 128;
        sumR += (sr[i] - 128) / 128;
        cnt++;
    }
    const meanL = sumL / cnt;
    const meanR = sumR / cnt;
    let vL = 0;
    let vR = 0;
    let cLR = 0;
    let sqMid = 0;
    let sqSide = 0;
    for (let i = 0; i < n; i += step) {
        const l = (sl[i] - 128) / 128;
        const r = (sr[i] - 128) / 128;
        const dl = l - meanL;
        const dr = r - meanR;
        vL += dl * dl;
        vR += dr * dr;
        cLR += dl * dr;
        const mid = (l + r) * 0.5;
        const side = (l - r) * 0.5;
        sqMid += mid * mid;
        sqSide += side * side;
    }
    const den = Math.sqrt(vL * vR);
    const corr = den > 1e-12 ? Math.max(-1, Math.min(1, cLR / den)) : null;
    const rmsMid = Math.sqrt(sqMid / cnt);
    const rmsSide = Math.sqrt(sqSide / cnt);
    const width = rmsMid > 1e-8 ? rmsSide / rmsMid : null;
    return {corr, width};
}

/**
 * RMS and peak in dBFS for one engine scope channel (uint8 centered at 128).
 * @param {Uint8Array} u8
 * @returns {{ rmsDb: number, peakDb: number }}
 */
function aeScopeChannelDbStats(u8) {
    let sumSq = 0;
    let peak = 0;
    const n = u8.length;
    for (let i = 0; i < n; i++) {
        const s = (u8[i] - 128) / 128;
        sumSq += s * s;
        const a = Math.abs(s);
        if (a > peak) peak = a;
    }
    const denom = Math.max(1, n);
    const rms = Math.sqrt(sumSq / denom);
    const rmsDb = rms > 1e-8 ? 20 * Math.log10(rms) : -96;
    const peakDb = peak > 1e-8 ? 20 * Math.log10(peak) : -96;
    return {
        rmsDb: Math.max(-96, Math.min(0, rmsDb)),
        peakDb: Math.max(-96, Math.min(0, peakDb)),
    };
}

function aeDrawMidSideGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const sliceW = w / n;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const mid = (l + r) * 0.5;
        const x = i * sliceW;
        const y = (0.5 - mid * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(120, 220, 255,0.92)';
    ctx.lineWidth = 1.75;
    ctx.shadowColor = 'rgba(120, 220, 255, 0.35)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const side = (l - r) * 0.5;
        const x = i * sliceW;
        const y = (0.5 - side * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(255, 180, 90, 0.92)';
    ctx.shadowColor = 'rgba(255, 180, 90, 0.35)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('Mid', 6, 14);
    ctx.fillText('Side', 6, 28);
}

function aeDrawBalanceGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const L = aeScopeChannelDbStats(sl);
    const R = aeScopeChannelDbStats(srDat);
    const diffDb = L.rmsDb - R.rmsDb;
    const maxDb = 9;
    const t = Math.max(-1, Math.min(1, -(diffDb / maxDb)));
    const cy = h * 0.52;
    const x0 = w * 0.1;
    const x1 = w * 0.9;
    const trackH = Math.max(10, h * 0.14);
    ctx.fillStyle = 'rgba(6,8,22,0.85)';
    ctx.fillRect(x0, cy - trackH / 2, x1 - x0, trackH);
    ctx.strokeStyle = 'rgba(122,139,168,0.35)';
    ctx.lineWidth = 1;
    ctx.strokeRect(x0, cy - trackH / 2, x1 - x0, trackH);
    const cx = (x0 + x1) * 0.5 + t * (x1 - x0) * 0.42;
    ctx.fillStyle = 'rgba(5,217,232,0.95)';
    ctx.beginPath();
    ctx.moveTo(cx, cy - trackH * 0.65);
    ctx.lineTo(cx + trackH * 0.35, cy);
    ctx.lineTo(cx, cy + trackH * 0.65);
    ctx.lineTo(cx - trackH * 0.35, cy);
    ctx.closePath();
    ctx.fill();
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(9, h / 32)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText(`Δ ${diffDb >= 0 ? '+' : ''}${diffDb.toFixed(1)} dB (L − R RMS)`, w / 2, cy - trackH * 1.15);
    ctx.textAlign = 'left';
    ctx.fillText('L hotter', x0, h - 10);
    ctx.textAlign = 'right';
    ctx.fillText('R hotter', x1, h - 10);
}

function aeDrawCorrelationGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    const {corr} = aeEngineScopeStereoMetrics(sl, srDat, n);
    if (corr == null) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const y0 = h * 0.38;
    const barH = Math.max(14, h * 0.2);
    const x0 = w * 0.08;
    const x1 = w * 0.92;
    const mid = (x0 + x1) * 0.5;
    ctx.fillStyle = 'rgba(6,8,22,0.85)';
    ctx.fillRect(x0, y0, x1 - x0, barH);
    ctx.strokeStyle = 'rgba(122,139,168,0.4)';
    ctx.strokeRect(x0, y0, x1 - x0, barH);
    const fillW = ((corr + 1) / 2) * (x1 - x0);
    const g = ctx.createLinearGradient(x0, 0, x1, 0);
    g.addColorStop(0, 'rgba(255,90,90,0.85)');
    g.addColorStop(0.5, 'rgba(122,139,168,0.5)');
    g.addColorStop(1, 'rgba(57,255,120,0.9)');
    ctx.fillStyle = g;
    ctx.fillRect(x0, y0, Math.max(0, fillW), barH);
    ctx.strokeStyle = 'rgba(255,255,255,0.75)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(mid, y0 - 3);
    ctx.lineTo(mid, y0 + barH + 3);
    ctx.stroke();
    ctx.fillStyle = 'rgba(224,240,255,0.9)';
    ctx.font = `${Math.max(10, h / 28)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText(`ρ = ${corr.toFixed(2)}`, w / 2, y0 - 10);
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.fillStyle = 'rgba(122,139,168,0.65)';
    ctx.fillText('−1 (wide) ← → +1 (mono)', w / 2, y0 + barH + 22);
}

function aeDrawWidthGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    const {width} = aeEngineScopeStereoMetrics(sl, srDat, n);
    if (width == null) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const wMax = 2.5;
    const pct = Math.max(0, Math.min(1, width / wMax));
    const y0 = h * 0.38;
    const barH = Math.max(14, h * 0.2);
    const x0 = w * 0.08;
    const x1 = w * 0.92;
    ctx.fillStyle = 'rgba(6,8,22,0.85)';
    ctx.fillRect(x0, y0, x1 - x0, barH);
    ctx.strokeStyle = 'rgba(211,0,197,0.35)';
    ctx.strokeRect(x0, y0, x1 - x0, barH);
    ctx.fillStyle = 'rgba(211,0,197,0.75)';
    ctx.fillRect(x0, y0, (x1 - x0) * pct, barH);
    ctx.fillStyle = 'rgba(224,240,255,0.9)';
    ctx.font = `${Math.max(10, h / 28)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText(`RMS(side) / RMS(mid) = ${width.toFixed(2)}`, w / 2, y0 - 10);
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.fillStyle = 'rgba(122,139,168,0.65)';
    ctx.fillText(`0 (mono) — ${wMax.toFixed(1)} (wide)`, w / 2, y0 + barH + 22);
}

function aeDrawCrestGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const cL = aeScopeChannelDbStats(sl);
    const cR = aeScopeChannelDbStats(srDat);
    const crestL = Math.pow(10, (cL.peakDb - cL.rmsDb) / 20);
    const crestR = Math.pow(10, (cR.peakDb - cR.rmsDb) / 20);
    const crestMax = 12;
    const row = (yBase, label, crest, rgb) => {
        const pct = Math.max(0, Math.min(1, crest / crestMax));
        const x0 = w * 0.14;
        const x1 = w * 0.94;
        const bh = Math.max(10, h * 0.1);
        ctx.fillStyle = 'rgba(6,8,22,0.85)';
        ctx.fillRect(x0, yBase, x1 - x0, bh);
        ctx.strokeStyle = 'rgba(122,139,168,0.3)';
        ctx.strokeRect(x0, yBase, x1 - x0, bh);
        ctx.fillStyle = `rgba(${rgb[0]},${rgb[1]},${rgb[2]},0.88)`;
        ctx.fillRect(x0, yBase, (x1 - x0) * pct, bh);
        ctx.fillStyle = 'rgba(224,240,255,0.9)';
        ctx.font = `${Math.max(9, h / 30)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'left';
        ctx.fillText(`${label}  ${crest.toFixed(2)} :1`, x0, yBase - 6);
    };
    row(h * 0.28, 'L', crestL, [5, 217, 232]);
    row(h * 0.58, 'R', crestR, [211, 0, 197]);
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 38)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText(`peak / RMS · scale 0–${crestMax}`, w / 2, h - 8);
}

function aeDrawLMinusRGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const sliceW = w / n;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const d = l - r;
        const x = i * sliceW;
        const y = (0.5 - d * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(57, 255, 120, 0.92)';
    ctx.lineWidth = 1.75;
    ctx.shadowColor = 'rgba(57, 255, 120, 0.35)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('L − R', 6, 14);
}

function aeDrawEnergyEnvelopeGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    let peak = 1e-8;
    const e = new Float64Array(n);
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const v = Math.sqrt(0.5 * (l * l + r * r));
        e[i] = v;
        if (v > peak) peak = v;
    }
    const sliceW = w / n;
    const yBase = h * 0.92;
    const plotH = h * 0.78;
    ctx.beginPath();
    ctx.moveTo(0, yBase);
    for (let i = 0; i < n; i++) {
        const x = (i + 0.5) * sliceW;
        const yn = yBase - (e[i] / peak) * plotH;
        ctx.lineTo(x, yn);
    }
    ctx.lineTo(w, yBase);
    ctx.closePath();
    const g = ctx.createLinearGradient(0, yBase - plotH, 0, yBase);
    g.addColorStop(0, 'rgba(5, 217, 232, 0.55)');
    g.addColorStop(1, 'rgba(211, 0, 197, 0.08)');
    ctx.fillStyle = g;
    ctx.fill();
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const x = (i + 0.5) * sliceW;
        const yn = yBase - (e[i] / peak) * plotH;
        if (i === 0) ctx.moveTo(x, yn);
        else ctx.lineTo(x, yn);
    }
    ctx.strokeStyle = 'rgba(5, 217, 232, 0.9)';
    ctx.lineWidth = 1.5;
    ctx.shadowColor = 'rgba(5, 217, 232, 0.25)';
    ctx.shadowBlur = 6;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('√(½(L²+R²)) · frame peak', 6, 14);
}

function aeDrawGoniometerGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const cx = w * 0.5;
    const cy = h * 0.5;
    const rad = Math.min(w, h) * 0.42;
    ctx.strokeStyle = 'rgba(122,139,168,0.22)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(cx, cy, rad, 0, Math.PI * 2);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(cx, cy, rad * 0.5, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([3, 5]);
    ctx.beginPath();
    ctx.moveTo(cx - rad * 1.05, cy);
    ctx.lineTo(cx + rad * 1.05, cy);
    ctx.moveTo(cx, cy - rad * 1.05);
    ctx.lineTo(cx, cy + rad * 1.05);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(cx - rad * 0.74, cy - rad * 0.74);
    ctx.lineTo(cx + rad * 0.74, cy + rad * 0.74);
    ctx.moveTo(cx - rad * 0.74, cy + rad * 0.74);
    ctx.lineTo(cx + rad * 0.74, cy - rad * 0.74);
    ctx.stroke();
    const step = Math.max(1, Math.floor(n / 720));
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < n; i += step) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const x = cx + l * rad * 0.95;
        const y = cy - r * rad * 0.95;
        if (!started) {
            ctx.moveTo(x, y);
            started = true;
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.strokeStyle = 'rgba(255, 200, 120, 0.85)';
    ctx.lineWidth = 1.35;
    ctx.shadowColor = 'rgba(255, 200, 120, 0.3)';
    ctx.shadowBlur = 7;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('L → R ↑', 6, 14);
}

function aeDrawDcOffsetGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    let sumL = 0;
    let sumR = 0;
    for (let i = 0; i < n; i++) {
        sumL += (sl[i] - 128) / 128;
        sumR += (srDat[i] - 128) / 128;
    }
    const inv = 1 / n;
    const mL = sumL * inv;
    const mR = sumR * inv;
    const xMid = w * 0.5;
    const maxShown = 0.22;
    const half = w * 0.44;
    const scale = half / maxShown;
    const row = (y, label, mean, rgb) => {
        const x0 = Math.min(xMid, xMid + mean * scale);
        const x1 = Math.max(xMid, xMid + mean * scale);
        const y0 = y - h * 0.08;
        const hh = h * 0.11;
        ctx.fillStyle = 'rgba(6,8,22,0.85)';
        ctx.fillRect(xMid - half, y0, half * 2, hh);
        ctx.strokeStyle = 'rgba(122,139,168,0.35)';
        ctx.strokeRect(xMid - half, y0, half * 2, hh);
        ctx.strokeStyle = 'rgba(255,255,255,0.35)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(xMid, y0 - 2);
        ctx.lineTo(xMid, y0 + hh + 2);
        ctx.stroke();
        ctx.fillStyle = `rgba(${rgb[0]},${rgb[1]},${rgb[2]},0.88)`;
        ctx.fillRect(x0, y0, Math.max(1, x1 - x0), hh);
        ctx.fillStyle = 'rgba(224,240,255,0.92)';
        ctx.font = `${Math.max(9, h / 32)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'left';
        ctx.fillText(`${label}  μ=${mean >= 0 ? '+' : ''}${mean.toFixed(4)}`, w * 0.06, y0 - 6);
    };
    row(h * 0.34, 'L', mL, [5, 217, 232]);
    row(h * 0.64, 'R', mR, [211, 0, 197]);
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 38)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'center';
    ctx.fillText(`±${maxShown.toFixed(2)} linear ·0 = mid`, w / 2, h - 6);
}

function aeDrawMagHistogramGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const binN = 48;
    const hist = new Uint32Array(binN);
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        let mag = Math.sqrt(0.5 * (l * l + r * r));
        if (mag < 0) mag = 0;
        if (mag > 1) mag = 1;
        let b = (mag * binN) | 0;
        if (b >= binN) b = binN - 1;
        hist[b]++;
    }
    let peakC = 1;
    for (let b = 0; b < binN; b++) {
        if (hist[b] > peakC) peakC = hist[b];
    }
    const padL = w * 0.06;
    const padR = w * 0.04;
    const padB = h * 0.14;
    const padT = h * 0.12;
    const plotW = w - padL - padR;
    const plotH = h - padB - padT;
    const bw = plotW / binN;
    for (let b = 0; b < binN; b++) {
        const bh = (hist[b] / peakC) * plotH;
        const x = padL + b * bw;
        const g = ctx.createLinearGradient(0, padT + plotH - bh, 0, padT + plotH);
        g.addColorStop(0, 'rgba(120, 200, 255, 0.75)');
        g.addColorStop(1, 'rgba(211, 0, 197, 0.35)');
        ctx.fillStyle = g;
        ctx.fillRect(x + 0.5, padT + plotH - bh, Math.max(1, bw - 1), bh);
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.25)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padL, padT + plotH);
    ctx.lineTo(padL + plotW, padT + plotH);
    ctx.stroke();
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('√(½(L²+R²)) · 0 →1', 6, 14);
    ctx.textAlign = 'center';
    ctx.fillText('quiet ← → hot', w / 2, h - 5);
}

function aeDrawPeakSampleGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    let pkMax = 1e-8;
    const pk = new Float64Array(n);
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const v = Math.max(Math.abs(l), Math.abs(r));
        pk[i] = v;
        if (v > pkMax) pkMax = v;
    }
    const sliceW = w / n;
    const yBase = h * 0.9;
    const plotH = h * 0.76;
    ctx.beginPath();
    ctx.moveTo(0, yBase);
    for (let i = 0; i < n; i++) {
        const x = (i + 0.5) * sliceW;
        const yn = yBase - (pk[i] / pkMax) * plotH;
        ctx.lineTo(x, yn);
    }
    ctx.lineTo(w, yBase);
    ctx.closePath();
    const g = ctx.createLinearGradient(0, yBase - plotH, 0, yBase);
    g.addColorStop(0, 'rgba(255, 160, 70, 0.45)');
    g.addColorStop(1, 'rgba(255, 90, 120, 0.06)');
    ctx.fillStyle = g;
    ctx.fill();
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const x = (i + 0.5) * sliceW;
        const yn = yBase - (pk[i] / pkMax) * plotH;
        if (i === 0) ctx.moveTo(x, yn);
        else ctx.lineTo(x, yn);
    }
    ctx.strokeStyle = 'rgba(255, 170, 95, 0.92)';
    ctx.lineWidth = 1.5;
    ctx.shadowColor = 'rgba(255, 140, 80, 0.28)';
    ctx.shadowBlur = 6;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('max(|L|,|R|) · frame peak', 6, 14);
}

function aeDrawMonoWaveGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const sliceW = w / n;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const mid = (l + r) * 0.5;
        const x = i * sliceW;
        const y = (0.5 - mid * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(120, 220, 255, 0.92)';
    ctx.lineWidth = 1.75;
    ctx.shadowColor = 'rgba(120, 220, 255, 0.35)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('M = (L + R) / 2', 6, 14);
}

function aeDrawSideWaveGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const sliceW = w / n;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const side = (l - r) * 0.5;
        const x = i * sliceW;
        const y = (0.5 - side * 0.5) * h;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(255, 180, 90, 0.92)';
    ctx.lineWidth = 1.75;
    ctx.shadowColor = 'rgba(255, 180, 90, 0.35)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('S = (L − R) / 2', 6, 14);
}

function aeDrawLrOverlayGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.14)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const sliceW = w / n;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    const strokeCh = (getS, rgbaStroke, rgbaGlow) => {
        ctx.beginPath();
        for (let i = 0; i < n; i++) {
            const x = i * sliceW;
            const y = (0.5 - getS(i) * 0.5) * h;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = rgbaStroke;
        ctx.lineWidth = 1.65;
        ctx.shadowColor = rgbaGlow;
        ctx.shadowBlur = 6;
        ctx.stroke();
        ctx.shadowBlur = 0;
    };
    strokeCh((i) => (sl[i] - 128) / 128, 'rgba(5, 217, 232, 0.88)', 'rgba(5, 217, 232, 0.28)');
    strokeCh((i) => (srDat[i] - 128) / 128, 'rgba(211, 0, 197, 0.88)', 'rgba(211, 0, 197, 0.28)');
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('L cyan · R magenta', 6, 14);
}

function aeDrawAbsDiffHistogramGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    const binN = 48;
    const hist = new Uint32Array(binN);
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        let d = Math.abs(l - r);
        if (d > 2) d = 2;
        let b = ((d / 2) * binN) | 0;
        if (b >= binN) b = binN - 1;
        hist[b]++;
    }
    let peakC = 1;
    for (let b = 0; b < binN; b++) {
        if (hist[b] > peakC) peakC = hist[b];
    }
    const padL = w * 0.06;
    const padR = w * 0.04;
    const padB = h * 0.14;
    const padT = h * 0.12;
    const plotW = w - padL - padR;
    const plotH = h - padB - padT;
    const bw = plotW / binN;
    for (let b = 0; b < binN; b++) {
        const bh = (hist[b] / peakC) * plotH;
        const x = padL + b * bw;
        const g = ctx.createLinearGradient(0, padT + plotH - bh, 0, padT + plotH);
        g.addColorStop(0, 'rgba(57, 255, 120, 0.78)');
        g.addColorStop(1, 'rgba(5, 217, 232, 0.35)');
        ctx.fillStyle = g;
        ctx.fillRect(x + 0.5, padT + plotH - bh, Math.max(1, bw - 1), bh);
    }
    ctx.strokeStyle = 'rgba(122,139,168,0.25)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padL, padT + plotH);
    ctx.lineTo(padL + plotW, padT + plotH);
    ctx.stroke();
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('|L − R| · 0 → 2 linear', 6, 14);
    ctx.textAlign = 'center';
    ctx.fillText('mono-ish ← → wide', w / 2, h - 5);
}

function aeDrawLissajousGraph(ctx, w, h) {
    aeGraphFillBackdrop(ctx, w, h);
    const n = typeof window._engineScopeLen === 'number' ? window._engineScopeLen : 0;
    const sl = typeof window !== 'undefined' ? window._engineScopeL : null;
    const srDat = typeof window !== 'undefined' ? window._engineScopeR : null;
    if (n < 16 || !sl || !srDat) {
        ctx.fillStyle = 'rgba(122,139,168,0.5)';
        ctx.font = `${Math.max(10, h / 22)}px "Share Tech Mono", ui-monospace, monospace`;
        ctx.textAlign = 'center';
        ctx.fillText('—', w / 2, h / 2);
        return;
    }
    let pk = 1e-8;
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const a = Math.max(Math.abs(l), Math.abs(r));
        if (a > pk) pk = a;
    }
    const cx = w * 0.5;
    const cy = h * 0.5;
    const scale = (Math.min(w, h) * 0.42) / Math.max(pk, 1e-6);
    ctx.strokeStyle = 'rgba(122,139,168,0.12)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
        const l = (sl[i] - 128) / 128;
        const r = (srDat[i] - 128) / 128;
        const x = cx + l * scale;
        const y = cy - r * scale;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = 'rgba(180, 120, 255, 0.85)';
    ctx.lineWidth = 1.35;
    ctx.shadowColor = 'rgba(140, 90, 255, 0.25)';
    ctx.shadowBlur = 8;
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = 'rgba(122,139,168,0.55)';
    ctx.font = `${Math.max(8, h / 36)}px "Share Tech Mono", ui-monospace, monospace`;
    ctx.textAlign = 'left';
    ctx.fillText('L → horizontal · R → vertical', 6, 14);
}

function aeDrawOutputGraphs() {
    const ms = document.getElementById('aeGraphMidSide');
    const bal = document.getElementById('aeGraphBalance');
    const cor = document.getElementById('aeGraphCorrelation');
    const wid = document.getElementById('aeGraphWidth');
    const cr = document.getElementById('aeGraphCrest');
    const dlr = document.getElementById('aeGraphLMinusR');
    const ene = document.getElementById('aeGraphEnergy');
    const gon = document.getElementById('aeGraphGonio');
    const dc = document.getElementById('aeGraphDcOffset');
    const mh = document.getElementById('aeGraphMagHist');
    const pk = document.getElementById('aeGraphPeakSample');
    const mw = document.getElementById('aeGraphMonoWave');
    const sw = document.getElementById('aeGraphSideWave');
    const lro = document.getElementById('aeGraphLrOverlay');
    const adh = document.getElementById('aeGraphAbsDiffHist');
    const lis = document.getElementById('aeGraphLissajous');
    if (ms && ms.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphMidSide', () => {
            const c2 = ms.getContext('2d');
            if (c2 && ms.width > 0 && ms.height > 0) aeDrawMidSideGraph(c2, ms.width, ms.height);
        });
    }
    if (bal && bal.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphBalance', () => {
            const c2 = bal.getContext('2d');
            if (c2 && bal.width > 0 && bal.height > 0) aeDrawBalanceGraph(c2, bal.width, bal.height);
        });
    }
    if (cor && cor.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphCorrelation', () => {
            const c2 = cor.getContext('2d');
            if (c2 && cor.width > 0 && cor.height > 0) aeDrawCorrelationGraph(c2, cor.width, cor.height);
        });
    }
    if (wid && wid.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphWidth', () => {
            const c2 = wid.getContext('2d');
            if (c2 && wid.width > 0 && wid.height > 0) aeDrawWidthGraph(c2, wid.width, wid.height);
        });
    }
    if (cr && cr.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphCrest', () => {
            const c2 = cr.getContext('2d');
            if (c2 && cr.width > 0 && cr.height > 0) aeDrawCrestGraph(c2, cr.width, cr.height);
        });
    }
    if (dlr && dlr.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphLMinusR', () => {
            const c2 = dlr.getContext('2d');
            if (c2 && dlr.width > 0 && dlr.height > 0) aeDrawLMinusRGraph(c2, dlr.width, dlr.height);
        });
    }
    if (ene && ene.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphEnergy', () => {
            const c2 = ene.getContext('2d');
            if (c2 && ene.width > 0 && ene.height > 0) aeDrawEnergyEnvelopeGraph(c2, ene.width, ene.height);
        });
    }
    if (gon && gon.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphGonio', () => {
            const c2 = gon.getContext('2d');
            if (c2 && gon.width > 0 && gon.height > 0) aeDrawGoniometerGraph(c2, gon.width, gon.height);
        });
    }
    if (dc && dc.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphDcOffset', () => {
            const c2 = dc.getContext('2d');
            if (c2 && dc.width > 0 && dc.height > 0) aeDrawDcOffsetGraph(c2, dc.width, dc.height);
        });
    }
    if (mh && mh.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphMagHist', () => {
            const c2 = mh.getContext('2d');
            if (c2 && mh.width > 0 && mh.height > 0) aeDrawMagHistogramGraph(c2, mh.width, mh.height);
        });
    }
    if (pk && pk.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphPeakSample', () => {
            const c2 = pk.getContext('2d');
            if (c2 && pk.width > 0 && pk.height > 0) aeDrawPeakSampleGraph(c2, pk.width, pk.height);
        });
    }
    if (mw && mw.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphMonoWave', () => {
            const c2 = mw.getContext('2d');
            if (c2 && mw.width > 0 && mw.height > 0) aeDrawMonoWaveGraph(c2, mw.width, mw.height);
        });
    }
    if (sw && sw.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphSideWave', () => {
            const c2 = sw.getContext('2d');
            if (c2 && sw.width > 0 && sw.height > 0) aeDrawSideWaveGraph(c2, sw.width, sw.height);
        });
    }
    if (lro && lro.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphLrOverlay', () => {
            const c2 = lro.getContext('2d');
            if (c2 && lro.width > 0 && lro.height > 0) aeDrawLrOverlayGraph(c2, lro.width, lro.height);
        });
    }
    if (adh && adh.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphAbsDiffHist', () => {
            const c2 = adh.getContext('2d');
            if (c2 && adh.width > 0 && adh.height > 0) aeDrawAbsDiffHistogramGraph(c2, adh.width, adh.height);
        });
    }
    if (lis && lis.getContext) {
        aeGraphWithScopeForCanvasId('aeGraphLissajous', () => {
            const c2 = lis.getContext('2d');
            if (c2 && lis.width > 0 && lis.height > 0) aeDrawLissajousGraph(c2, lis.width, lis.height);
        });
    }
}

function aeClearOutputGraphCanvases() {
    const ids = _aeDiagGraphIds;
    for (let i = 0; i < ids.length; i++) {
        const el = document.getElementById(ids[i]);
        if (!el || !el.getContext) continue;
        const c2 = el.getContext('2d');
        if (!c2 || el.width < 2 || el.height < 2) continue;
        aeGraphFillBackdrop(c2, el.width, el.height);
    }
}

function scheduleAeGraphRafLoop() {
    if (!shouldDrawAeOutputGraphs()) {
        stopAeGraphRaf();
        return;
    }
    if (aeGraphRafId !== 0) return;
    const loop = () => {
        aeGraphRafId = 0;
        if (!shouldDrawAeOutputGraphs()) return;
        aeDrawOutputGraphs();
        aeGraphRafId = requestAnimationFrame(loop);
    };
    aeGraphRafId = requestAnimationFrame(loop);
}

function startAeTabMeterPollIfNeeded() {
    if (!shouldRunAeTabMeterPoll()) {
        stopAeTabMeterPoll();
        return;
    }
    stopAeTabMeterPoll();
    aeTabMeterPollTimer = setInterval(() => void tickAeTabMeterPoll(), AE_TAB_METER_POLL_MS);
    void tickAeTabMeterPoll();
}

async function tickAeTabMeterPoll() {
    if (!shouldRunAeTabMeterPoll()) {
        stopAeTabMeterPoll();
        return;
    }
    if (aeTabMeterPollInFlight) return;
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        stopAeTabMeterPoll();
        return;
    }
    aeTabMeterPollInFlight = true;
    try {
        const st = await inv(buildEnginePlaybackStatusRequest());
        if (st && st.ok === true) {
            applyPlaybackStatusSpectrum(st);
            applyPlaybackStatusScope(st);
            if (typeof st.peak === 'number' && !Number.isNaN(st.peak)) {
                window._enginePlaybackPeak = st.peak;
            }
        }
    } catch {
        /* ignore */
    } finally {
        aeTabMeterPollInFlight = false;
    }
}

function syncAeOutputGraphsAfterStreamStateChange() {
    if (!aeAudioEngineTabIsActive()) {
        stopAeTabMeterAndGraph();
        return;
    }
    layoutAeOutputGraphCanvases();
    if (typeof window === 'undefined' || !window._aeOutputStreamRunning) {
        stopAeTabMeterAndGraph();
        aeClearOutputGraphCanvases();
        return;
    }
    startAeTabMeterPollIfNeeded();
    scheduleAeGraphRafLoop();
}

async function runEnginePlaybackStatusTick() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    try {
        const st = await inv(buildEnginePlaybackStatusRequest());
        if (st && st.ok === true) {
            applyPlaybackStatusSpectrum(st);
            applyPlaybackStatusScope(st);
            if (typeof st.peak === 'number' && !Number.isNaN(st.peak)) {
                window._enginePlaybackPeak = st.peak;
            }
            if (st.loaded === true) {
                window._enginePlaybackPosSec = typeof st.position_sec === 'number' ? st.position_sec : 0;
                /* `playback_load` often has duration before the first few `playback_status` polls; some
                 * sessions report `duration_sec: 0` while the decoder is still settling. Overwriting a
                 * positive `_enginePlaybackDurSec` with 0 hides the NP / tray waveform cursor until
                 * a later poll — and if every poll keeps sending 0, the cursor never returns (common
                 * on the second open of the same file). */
                const incomingDur =
                    typeof st.duration_sec === 'number' && !Number.isNaN(st.duration_sec) ? st.duration_sec : null;
                const prevDur =
                    typeof window._enginePlaybackDurSec === 'number' && window._enginePlaybackDurSec > 0
                        ? window._enginePlaybackDurSec
                        : 0;
                if (incomingDur != null) {
                    if (incomingDur > 0) {
                        window._enginePlaybackDurSec = incomingDur;
                    } else if (!prevDur) {
                        window._enginePlaybackDurSec = 0;
                    }
                }
                window._enginePlaybackPaused = st.paused === true;
                /* Anchor the local interpolation model so `updatePlaybackTime()` can compute
                 * `posSec + (now - anchor) * speed` between polls at rAF rate. Without this the
                 * playhead visibly steps in 30 ms chunks (poll interval) even though the rAF
                 * loop runs at 60 Hz — each frame reads the same stale `_enginePlaybackPosSec`. */
                window._enginePlaybackPosAnchorMs = performance.now();
                if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
                if (st.eof !== true && typeof window.resetEnginePlaybackEofFlag === 'function') {
                    window.resetEnginePlaybackEofFlag();
                }
                if (
                    st.eof === true &&
                    typeof window.handleEnginePlaybackEofFromPoll === 'function'
                ) {
                    window.handleEnginePlaybackEofFromPoll();
                }
            }
        }
        if (typeof window.ensureEnginePlaybackFftRaf === 'function') {
            window.ensureEnginePlaybackFftRaf();
        }
    } catch {
        /* ignore */
    }
}

function startEnginePlaybackPoll() {
    stopAeTabMeterPoll();
    stopAeGraphRaf();
    _enginePlaybackPollSessionActive = false;
    _haltLibraryPlaybackPollIntervalAndWatchdog();
    _enginePlaybackPollSessionActive = true;
    if (typeof document !== 'undefined' && !_enginePlaybackIdleHooked) {
        _enginePlaybackIdleHooked = true;
        document.addEventListener('visibilitychange', syncEnginePlaybackPollForUiIdle);
        document.addEventListener('ui-idle-heavy-cpu', syncEnginePlaybackPollForUiIdle);
    }
    void runEnginePlaybackStatusTick();
    if (shouldDeferPlaybackPollToHostWatchdog()) {
        syncEnginePlaybackEofWatchdog();
    } else {
        _enginePlaybackPollTimer = setInterval(() => void runEnginePlaybackStatusTick(), ENGINE_PLAYBACK_POLL_MS);
        syncEnginePlaybackEofWatchdog();
    }
    scheduleAeGraphRafLoop();
}

/**
 * Push now-playing EQ / gain / pan prefs to the engine DSP path.
 */
function syncEnginePlaybackDspFromPrefs() {
    const inv = getAeAudioEngineInvoke();
    if (!inv || typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return;
    const volPct = parseInt(prefs.getItem('audioVolume') || '100', 10);
    const vol = Math.max(0, Math.min(1, volPct / 100));
    const g = (parseFloat(prefs.getItem('preampGain') || '1') || 1) * vol;
    const pan = parseFloat(prefs.getItem('audioPan') || '0') || 0;
    const low = parseFloat(prefs.getItem('eqLow') || '0') || 0;
    const mid = parseFloat(prefs.getItem('eqMid') || '0') || 0;
    const high = parseFloat(prefs.getItem('eqHigh') || '0') || 0;
    const mono = prefs.getItem('audioMono') === 'on';
    void inv({
        cmd: 'playback_set_dsp',
        gain: g,
        pan,
        eq_low_db: low,
        eq_mid_db: mid,
        eq_high_db: high,
        mono,
    });
}

/** Now-playing speed (0.25–4×) → AudioEngine `playback_set_speed` (`ResamplingAudioSource`; pitch follows speed like `<audio>.playbackRate`). */
function syncEnginePlaybackSpeedFromPrefs() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    const sel = document.getElementById('npSpeed');
    const v = parseFloat(sel && typeof sel.value === 'string' ? sel.value : '1');
    const s = Number.isFinite(v) ? Math.max(0.25, Math.min(4, v)) : 1;
    void inv({cmd: 'playback_set_speed', speed: s});
}

/** Full-file loop → `playback_set_loop` (forward: `AudioFormatReaderSource`; reverse: RAM buffer wraps). */
function syncEnginePlaybackLoop(loop) {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    void inv({cmd: 'playback_set_loop', loop: !!loop});
}

function syncEnginePlaybackLoopFromPrefs() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    const on =
        typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
            ? prefs.getItem('audioLoop') === 'on'
            : false;
    void inv({cmd: 'playback_set_loop', loop: on});
}

/**
 * Reopen output with `start_playback: true` (AudioEngine `start_output_stream` stops any prior stream first).
 * Used when reverse mode toggles (new rodio source).
 */
async function enginePlaybackRestartStream() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) throw new Error('audio engine IPC unavailable');
    const deviceId =
        typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
            ? prefs.getItem(AE_PREFS_DEVICE) || ''
            : '';
    const bufOut = document.getElementById('aeBufferFramesOutput');
    const bfRaw = bufOut && typeof bufOut.value === 'string' ? bufOut.value : '';
    const bufferFrames = parseAeBufferFramesPref(bfRaw);
    const payload = {
        cmd: 'start_output_stream',
        device_id: deviceId,
        tone: false,
        start_playback: true,
    };
    if (bufferFrames !== undefined) {
        payload.buffer_frames = bufferFrames;
    }
    const r = await inv(payload);
    throwIfAeNotOk(r, 'start_output_stream failed');
    syncEnginePlaybackDspFromPrefs();
    syncEnginePlaybackSpeedFromPrefs();
    syncEnginePlaybackLoopFromPrefs();
}

/** Pref `audioReverse` on at load: decode reversed PCM in AudioEngine and reopen stream (see `playback_set_reverse`). */
async function engineApplyReversePrefPlayback() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    try {
        await inv({cmd: 'playback_set_reverse', reverse: true});
        await enginePlaybackRestartStream();
        await inv({cmd: 'playback_seek', position_sec: 0});
    } catch {
        /* best-effort */
    }
}

/**
 * Load file + start cpal output with `start_playback` (see audio-engine README).
 * @param {string} filePath — absolute host path
 */
async function enginePlaybackStart(filePath) {
    if (typeof window !== 'undefined' && typeof window.resetEnginePlaybackEofFlag === 'function') {
        window.resetEnginePlaybackEofFlag();
    }
    const inv = getAeAudioEngineInvoke();
    if (!inv) throw new Error('audio engine IPC unavailable');
    let r = await inv({cmd: 'playback_load', path: filePath});
    throwIfAeNotOk(r, 'playback_load failed');
    /* Seek math (`seekPlaybackToPercent`) needs duration before the first `playback_status` poll (see `ENGINE_PLAYBACK_POLL_MS`). */
    window._enginePlaybackPosSec = 0;
    window._enginePlaybackDurSec =
        typeof r.duration_sec === 'number' && !Number.isNaN(r.duration_sec) ? r.duration_sec : 0;
    /* Apply this sample's persisted loop region (from the expanded-row braces) to `_abLoop`
     * now that duration is known — the shared `_playbackRafLoop` A-B enforcement handles seeks. */
    if (typeof window.syncAbLoopFromSampleRegion === 'function') {
        window.syncAbLoopFromSampleRegion(filePath);
    }
    if (typeof window.refreshNpLoopRegionUI === 'function') {
        window.refreshNpLoopRegionUI();
    }
    const deviceId =
        typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
            ? prefs.getItem(AE_PREFS_DEVICE) || ''
            : '';
    const bufOut = document.getElementById('aeBufferFramesOutput');
    const bfRaw = bufOut && typeof bufOut.value === 'string' ? bufOut.value : '';
    const bufferFrames = parseAeBufferFramesPref(bfRaw);
    /* `start_output_stream` stops any existing stream and validates `device_id` — avoid extra IPC round-trips. */
    const payload = {
        cmd: 'start_output_stream',
        device_id: deviceId,
        tone: false,
        start_playback: true,
    };
    if (bufferFrames !== undefined) {
        payload.buffer_frames = bufferFrames;
    }
    r = await inv(payload);
    throwIfAeNotOk(r, 'start_output_stream failed');
    if (typeof window !== 'undefined') {
        window._aeOutputStreamRunning = true;
    }
    /* Before first `playback_status` poll — matches `audio.js` `_enginePlaybackActive` so tray / `isAudioPlaying` see engine transport. */
    if (typeof window.setEnginePlaybackActive === 'function') {
        window.setEnginePlaybackActive(true);
    }
    syncEnginePlaybackDspFromPrefs();
    syncEnginePlaybackSpeedFromPrefs();
    syncEnginePlaybackLoopFromPrefs();
    startEnginePlaybackPoll();
    /* Next `playback_status` poll may be ~250 ms later; avoid stale `paused` from a prior session skewing `isAudioPlaying()`. */
    window._enginePlaybackPaused = false;
}

async function enginePlaybackStop() {
    stopEnginePlaybackPoll();
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    try {
        await inv({cmd: 'stop_output_stream'});
    } catch {
        /* ignore */
    }
    try {
        await inv({cmd: 'playback_stop'});
    } catch {
        /* ignore */
    }
    window._enginePlaybackPosSec = 0;
    window._enginePlaybackDurSec = 0;
    window._enginePlaybackPaused = false;
    window._engineSpectrumU8 = null;
    window._aeOutputStreamRunning = false;
    if (typeof window.stopEnginePlaybackFftRaf === 'function') window.stopEnginePlaybackFftRaf();
    stopAeTabMeterAndGraph();
    aeClearOutputGraphCanvases();
}

if (typeof window !== 'undefined') {
    window.buildEnginePlaybackStatusRequest = buildEnginePlaybackStatusRequest;
    window.ENGINE_PLAYBACK_SPECTRUM_MIN_BINS = ENGINE_PLAYBACK_SPECTRUM_MIN_BINS;
    window.ensureAeOutputStreamOnStartup = ensureAeOutputStreamOnStartup;
    window.enginePlaybackStart = enginePlaybackStart;
    window.enginePlaybackStop = enginePlaybackStop;
    window.enginePlaybackRestartStream = enginePlaybackRestartStream;
    window.syncEnginePlaybackDspFromPrefs = syncEnginePlaybackDspFromPrefs;
    window.syncEnginePlaybackSpeedFromPrefs = syncEnginePlaybackSpeedFromPrefs;
    window.syncEnginePlaybackLoop = syncEnginePlaybackLoop;
    window.syncEnginePlaybackLoopFromPrefs = syncEnginePlaybackLoopFromPrefs;
    window.engineApplyReversePrefPlayback = engineApplyReversePrefPlayback;
    window.stopEnginePlaybackPoll = stopEnginePlaybackPoll;
    window.startEnginePlaybackPoll = startEnginePlaybackPoll;
    window.syncEnginePlaybackPollForUiIdle = syncEnginePlaybackPollForUiIdle;
    window.syncAeTransportFromPlayback = syncAeTransportFromPlayback;
    window.stopAeTabMeterAndGraph = stopAeTabMeterAndGraph;
    window.stopAeGraphRaf = stopAeGraphRaf;
    window.scheduleAeGraphRafLoop = scheduleAeGraphRafLoop;
}
