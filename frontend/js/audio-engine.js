// ── Audio Engine tab: separate `audio-engine` process (cpal devices, future plugin graph) ──

const AE_PREFS_DEVICE = 'audioEngineOutputDeviceId';
const AE_PREFS_INPUT_DEVICE = 'audioEngineInputDeviceId';
const AE_PREFS_TONE = 'audioEngineTestTone';
const AE_PREFS_BUFFER_FRAMES_OUTPUT = 'audioEngineBufferFramesOutput';
const AE_PREFS_BUFFER_FRAMES_INPUT = 'audioEngineBufferFramesInput';
/** @deprecated Legacy single pref; migrated once to output/input */
const AE_LEGACY_BUFFER_FRAMES = 'audioEngineBufferFrames';

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
/** Matches sidecar `MAX_BUFFER_FRAMES` — typos like 144000 are ~3s @ 48 kHz and sound like delayed mute after stop. */
const AE_MAX_BUFFER_FRAMES = 8192;

function parseAeBufferFramesPref(raw) {
    const s = String(raw ?? '').trim();
    if (s === '') return undefined;
    const n = Number.parseInt(s, 10);
    if (!Number.isFinite(n) || n < 1) return undefined;
    return Math.min(n >>> 0, AE_MAX_BUFFER_FRAMES);
}

/**
 * @param {unknown} buf — `buffer_size` from sidecar (object or legacy string)
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
    const fmtDb = (x) => {
        const db = parseFloat(x);
        if (Number.isNaN(db)) return '0 dB';
        return (db >= 0 ? '+' : '') + db.toFixed(0) + ' dB';
    };
    for (const band of ['Low', 'Mid', 'High']) {
        const key = 'eq' + band;
        const raw = prefs.getItem(key);
        const sl = document.getElementById('aeEq' + band);
        const lab = document.getElementById('aeEq' + band + 'Val');
        if (raw != null && sl) sl.value = raw;
        if (raw != null && lab) lab.textContent = fmtDb(raw);
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
    for (const band of ['low', 'mid', 'high']) {
        const cap = band.charAt(0).toUpperCase() + band.slice(1);
        const el = document.getElementById('aeEq' + cap);
        if (el && typeof el.addEventListener === 'function' && typeof setEqBand === 'function') {
            el.addEventListener('input', () => setEqBand(band, el.value));
        }
    }
    const gain = document.getElementById('aeGainSlider');
    if (gain && typeof gain.addEventListener === 'function' && typeof setPreampGain === 'function') {
        gain.addEventListener('input', () => setPreampGain(gain.value));
    }
    const pan = document.getElementById('aePanSlider');
    if (pan && typeof pan.addEventListener === 'function' && typeof setPan === 'function') {
        pan.addEventListener('input', () => setPan(pan.value));
    }
}

/**
 * @param {object|null} chain — `plugin_chain` payload
 */
function fillAePluginSection(chain) {
    const stub = document.getElementById('aePluginStub');
    const ul = document.getElementById('aePluginSlotList');
    if (!stub || typeof catalogFmt !== 'function') return;
    const phase = chain && chain.phase != null ? String(chain.phase) : '—';
    const fmts =
        chain && Array.isArray(chain.formats_planned) && chain.formats_planned.length
            ? chain.formats_planned.join(', ')
            : '—';
    const n = chain && Array.isArray(chain.slots) ? chain.slots.length : 0;
    const note = chain && chain.note != null ? String(chain.note) : '';
    stub.textContent = catalogFmt('ui.ae.plugins_stub', {
        phase,
        formats: fmts,
        count: String(n),
        note,
    });
    if (!ul || typeof ul.replaceChildren !== 'function') return;
    ul.replaceChildren();
    if (chain && Array.isArray(chain.slots) && chain.slots.length > 0) {
        for (const s of chain.slots) {
            const li = document.createElement('li');
            li.textContent = typeof s === 'string' ? s : JSON.stringify(s);
            ul.appendChild(li);
        }
    } else {
        const li = document.createElement('li');
        li.textContent = catalogFmt('ui.ae.plugins_slot_empty');
        ul.appendChild(li);
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
    const applyBtn = document.getElementById('aeApplyDevice');
    if (applyBtn && typeof applyBtn.addEventListener === 'function') {
        applyBtn.addEventListener('click', () => {
            void applyAudioEngineDevice();
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

    void refreshAudioEnginePanel();
}

/**
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 * @param {string} deviceId — sidecar device id (stable name-based or legacy index)
 */
async function fillAeDeviceCaps(inv, deviceId) {
    const capsEl = document.getElementById('aeDeviceCaps');
    if (!capsEl || typeof inv !== 'function' || !deviceId) {
        if (capsEl) capsEl.textContent = '—';
        return;
    }
    try {
        const info = await inv({cmd: 'get_output_device_info', device_id: deviceId});
        const line = buildAeDeviceCapsLine(info);
        capsEl.textContent = line != null ? line : '—';
    } catch {
        capsEl.textContent = '—';
    }
}

/**
 * `get_input_device_info`: omit `device_id` when empty for system default input.
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 * @param {string} [deviceId] — sidecar id or "" for default
 */
async function fillAeInputDeviceCaps(inv, deviceId) {
    const el = document.getElementById('aeInputDeviceCaps');
    if (!el || typeof inv !== 'function') {
        if (el) el.textContent = '—';
        return;
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
    } catch {
        el.textContent = '—';
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
 * After a failed IPC action, re-read `engine_state` when possible so stream lines match the sidecar; else clear.
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
 * Reload engine_state (ping + stream), device list, caps, plugin stub.
 */
async function refreshAudioEnginePanel() {
    const statusEl = document.getElementById('aeEngineStatus');
    const selectEl = document.getElementById('aeOutputDevice');
    const toneCb = document.getElementById('aeTestTone');
    const inv = getAeAudioEngineInvoke();

    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }

    if (statusEl && typeof catalogFmt === 'function') {
        statusEl.textContent = catalogFmt('ui.ae.status_loading');
    }

    try {
        const es = await inv({cmd: 'engine_state'});
        throwIfAeNotOk(es, 'engine_state failed');
        fillAeEngineStatusOkFromState(statusEl, es);
        fillAeStreamsFromEngineState(es);
        syncAeToneCheckboxFromStream(toneCb, es.stream);

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
    }

    try {
        const r = await inv({cmd: 'set_output_device', device_id: id});
        throwIfAeNotOk(r, 'set_output_device failed');
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
        if (typeof window.syncEnginePlaybackStoppedFromSidecar === 'function') {
            window.syncEnginePlaybackStoppedFromSidecar();
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

// ── Library playback via sidecar (PCM + EQ in engine; WebView stays silent) ──

/** @type {ReturnType<typeof setInterval> | null} */
let _enginePlaybackPollTimer = null;

function stopEnginePlaybackPoll() {
    if (_enginePlaybackPollTimer != null) {
        clearInterval(_enginePlaybackPollTimer);
        _enginePlaybackPollTimer = null;
    }
}

function startEnginePlaybackPoll() {
    stopEnginePlaybackPoll();
    const tick = async () => {
        const inv = getAeAudioEngineInvoke();
        if (!inv) return;
        try {
            const st = await inv({cmd: 'playback_status'});
            if (st && st.ok === true && st.loaded === true) {
                window._enginePlaybackPosSec = typeof st.position_sec === 'number' ? st.position_sec : 0;
                window._enginePlaybackDurSec = typeof st.duration_sec === 'number' ? st.duration_sec : 0;
                window._enginePlaybackPaused = st.paused === true;
                window._enginePlaybackPeak = typeof st.peak === 'number' ? st.peak : 0;
                if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
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

/** Now-playing speed (0.25–2x) → rodio `Player::set_speed` (same pitch behavior as `<audio>.playbackRate`). */
function syncEnginePlaybackSpeedFromPrefs() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) return;
    const sel = document.getElementById('npSpeed');
    const v = parseFloat(sel && typeof sel.value === 'string' ? sel.value : '1');
    const s = Number.isFinite(v) ? Math.max(0.25, Math.min(2, v)) : 1;
    void inv({cmd: 'playback_set_speed', speed: s});
}

/**
 * After `stop_output_stream`, reconnect the same device and restart library PCM (`start_playback: true`).
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
    await inv({cmd: 'stop_output_stream'});
    await inv({cmd: 'set_output_device', device_id: deviceId});
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
}

/** Pref `audioReverse` on at load: decode reversed PCM in sidecar and reopen stream (see `playback_set_reverse`). */
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
    await inv({cmd: 'stop_output_stream'});
    await inv({cmd: 'set_output_device', device_id: deviceId});
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
}

if (typeof window !== 'undefined') {
    window.enginePlaybackStart = enginePlaybackStart;
    window.enginePlaybackStop = enginePlaybackStop;
    window.enginePlaybackRestartStream = enginePlaybackRestartStream;
    window.syncEnginePlaybackDspFromPrefs = syncEnginePlaybackDspFromPrefs;
    window.syncEnginePlaybackSpeedFromPrefs = syncEnginePlaybackSpeedFromPrefs;
    window.engineApplyReversePrefPlayback = engineApplyReversePrefPlayback;
    window.stopEnginePlaybackPoll = stopEnginePlaybackPoll;
    window.startEnginePlaybackPoll = startEnginePlaybackPoll;
}
