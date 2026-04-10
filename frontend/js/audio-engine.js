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

/** Live plugin list from the last `plugin_chain` response (populated by `aePopulateInsertSlotSelects`). */
let aePluginCatalog = [];

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
        if (aeAudioEngineTabIsActive()) void refreshAeProcessStats();
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
}

/**
 * @param {string} raw
 * @returns {number|undefined} positive integer frame count, or undefined to use driver default
 */
/** Matches AudioEngine `MAX_BUFFER_FRAMES` — typos like 144000 are ~3s @ 48 kHz and sound like delayed mute after stop. */
const AE_MAX_BUFFER_FRAMES = 8192;

/**
 * @param {HTMLSelectElement|null} sel
 * @returns {number|undefined} integer Hz for `sample_rate_hz` IPC, or undefined for driver default
 */
function parseAeSampleRateHzFromSelect(sel) {
    if (!sel || typeof sel.value !== 'string') return undefined;
    const s = sel.value.trim();
    if (s === '') return undefined;
    const n = Number.parseInt(s, 10);
    if (!Number.isFinite(n) || n < 1000) return undefined;
    return n;
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
    input.placeholder = 'Search plugins\u2026';
    input.autocomplete = 'off';
    input.spellcheck = false;
    const clear = document.createElement('span');
    clear.className = 'ae-picker-clear';
    clear.textContent = '\u00d7';
    clear.title = 'Clear';
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

function aePopulateInsertSlotSelects(chain) {
    const plugins = chain && Array.isArray(chain.plugins) ? chain.plugins : [];
    aePluginCatalog = plugins.map((p) => ({
        path: p && p.path != null ? String(p.path) : '',
        name: p && p.name != null ? String(p.name) : String(p && p.path || '').split('/').pop(),
        format: p && p.format != null ? String(p.format) : '',
    })).filter((p) => p.path);

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

async function aeOpenInsertEditor(uiSlotIndex) {
    const inv = getAeAudioEngineInvoke();
    if (!inv) { aeNotifyNoAudioEngineIpc(); return; }
    if (!aeInsertPickers[uiSlotIndex] || !aeInsertPickers[uiSlotIndex].getValue()) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_insert_editor_no_plugin'), 4000, 'warning');
        return;
    }
    try {
        await applyAePlaybackInserts({ showAppliedToast: false });
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
 */
async function applyAePlaybackInserts(opts) {
    const showAppliedToast = opts == null || opts.showAppliedToast !== false;
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
        if (typeof showToast === 'function' && typeof toastFmt === 'function')
            showToast(toastFmt('toast.ae_inserts_failed', {err}), 5000, 'error');
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
        void resumeAeInputPeakPollIfNeeded();
        void refreshAeProcessStats();
        return;
    }
    root.dataset.aeInit = '1';
    bindAeInputPeakVisibilityOnce();
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
            const st = await inv({cmd: 'playback_status'});
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

async function stopAeOutputStream() {
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
    }
}

// ── Library playback via AudioEngine (PCM + EQ in engine; WebView stays silent) ──

/** @type {ReturnType<typeof setInterval> | null} */
let _enginePlaybackPollTimer = null;

function stopEnginePlaybackPoll() {
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

function applyPlaybackStatusSpectrum(st) {
    if (!st || st.ok !== true) return;
    if (Array.isArray(st.spectrum) && st.spectrum.length >= 1024) {
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

function startEnginePlaybackPoll() {
    stopEnginePlaybackPoll();
    try {
        const u = typeof window !== 'undefined' ? window.vstUpdater : undefined;
        if (u && typeof u.audioEngineEofWatchdogStart === 'function') {
            void u.audioEngineEofWatchdogStart().catch(() => {});
        }
    } catch (_) {
        /* non-Tauri */
    }
    const tick = async () => {
        const inv = getAeAudioEngineInvoke();
        if (!inv) return;
        try {
            const st = await inv({cmd: 'playback_status'});
            if (st && st.ok === true) {
                applyPlaybackStatusSpectrum(st);
                if (st.loaded === true) {
                    window._enginePlaybackPosSec = typeof st.position_sec === 'number' ? st.position_sec : 0;
                    window._enginePlaybackDurSec = typeof st.duration_sec === 'number' ? st.duration_sec : 0;
                    window._enginePlaybackPaused = st.paused === true;
                    window._enginePlaybackPeak = typeof st.peak === 'number' ? st.peak : 0;
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
    };
    void tick();
    _enginePlaybackPollTimer = setInterval(() => void tick(), 100);
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
    void inv({
        cmd: 'playback_set_dsp',
        gain: g,
        pan,
        eq_low_db: low,
        eq_mid_db: mid,
        eq_high_db: high,
    });
}

/** Now-playing speed (0.25–2×) → AudioEngine `playback_set_speed` (`ResamplingAudioSource`; pitch follows speed like `<audio>.playbackRate`). */
function syncEnginePlaybackSpeedFromPrefs() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    const sel = document.getElementById('npSpeed');
    const v = parseFloat(sel && typeof sel.value === 'string' ? sel.value : '1');
    const s = Number.isFinite(v) ? Math.max(0.25, Math.min(2, v)) : 1;
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
    /* Seek math (`seekPlaybackToPercent`) needs duration before the first `playback_status` poll (~100ms). */
    window._enginePlaybackPosSec = 0;
    window._enginePlaybackDurSec =
        typeof r.duration_sec === 'number' && !Number.isNaN(r.duration_sec) ? r.duration_sec : 0;
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
    syncEnginePlaybackDspFromPrefs();
    syncEnginePlaybackSpeedFromPrefs();
    startEnginePlaybackPoll();
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
}

if (typeof window !== 'undefined') {
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
    window.syncAeTransportFromPlayback = syncAeTransportFromPlayback;
}
