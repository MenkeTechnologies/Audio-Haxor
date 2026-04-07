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
function parseAeBufferFramesPref(raw) {
    const s = String(raw ?? '').trim();
    if (s === '') return undefined;
    const n = Number.parseInt(s, 10);
    if (!Number.isFinite(n) || n < 1) return undefined;
    return n >>> 0;
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
 * Called when the Audio Engine tab becomes active (`utils.js` `switchTab` → `runPerTabWork`).
 * Idempotent — safe if called multiple times.
 */
function initAudioEngineTab() {
    const root = document.getElementById('tabAudioEngine');
    if (!root) return;
    if (root.dataset.aeInit === '1') {
        void resumeAeInputPeakPollIfNeeded();
        return;
    }
    root.dataset.aeInit = '1';
    bindAeInputPeakVisibilityOnce();

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
        const name = st.device_name != null ? String(st.device_name) : String(st.device_id);
        const rate = st.sample_rate_hz != null ? String(st.sample_rate_hz) : null;
        const ch = st.channels != null ? String(st.channels) : null;
        const fmt = st.sample_format != null ? String(st.sample_format) : '';
        const buf = formatAeBufferSize(st.buffer_size);
        let line;
        if (rate != null && ch != null) {
            line = catalogFmt('ui.ae.output_stream_on_detail', {
                name,
                device: String(st.device_id),
                rate,
                channels: ch,
                format: fmt,
                buffer: buf,
            });
        } else {
            line = catalogFmt('ui.ae.output_stream_on', {device: String(st.device_id)});
        }
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
        const name = st.device_name != null ? String(st.device_name) : String(st.device_id);
        const rate = st.sample_rate_hz != null ? String(st.sample_rate_hz) : null;
        const ch = st.channels != null ? String(st.channels) : null;
        const fmt = st.sample_format != null ? String(st.sample_format) : '';
        const buf = formatAeBufferSize(st.buffer_size);
        let line;
        if (rate != null && ch != null) {
            line = catalogFmt('ui.ae.input_stream_on_detail', {
                name,
                device: String(st.device_id),
                rate,
                channels: ch,
                format: fmt,
                buffer: buf,
            });
        } else {
            line = catalogFmt('ui.ae.input_stream_on', {device: String(st.device_id)});
        }
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
 * Reload engine_state (ping + stream), device list, caps, plugin stub.
 */
async function refreshAudioEnginePanel() {
    const statusEl = document.getElementById('aeEngineStatus');
    const selectEl = document.getElementById('aeOutputDevice');
    const pluginEl = document.getElementById('aePluginStub');
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
        if (pluginEl && typeof catalogFmt === 'function') {
            const n = chain && Array.isArray(chain.slots) ? chain.slots.length : 0;
            pluginEl.textContent = catalogFmt('ui.ae.plugins_stub', {count: String(n)});
        }

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
                    inSelectEl.replaceChildren();
                    const defOpt = document.createElement('option');
                    defOpt.value = '';
                    defOpt.textContent = catalogFmt('ui.ae.input_device_default_option');
                    inSelectEl.appendChild(defOpt);
                    for (const d of ins.devices) {
                        const id = d.id != null ? String(d.id) : '';
                        const name = d.name != null ? String(d.name) : id;
                        const opt = document.createElement('option');
                        opt.value = id;
                        opt.textContent = name;
                        if (d.is_default === true) {
                            opt.dataset.default = '1';
                        }
                        inSelectEl.appendChild(opt);
                    }
                    if (inPick !== '') {
                        inSelectEl.value = inPick;
                    }
                    const valid = inPick === '' || [...inSelectEl.options].some((o) => o.value === inPick);
                    if (!valid && ins.devices.length > 0) {
                        const defD = ins.devices.find((x) => x.is_default === true);
                        inSelectEl.value = defD && defD.id != null ? String(defD.id) : String(ins.devices[0].id);
                    } else if (!valid) {
                        inSelectEl.value = '';
                    }
                    await fillAeInputDeviceCaps(inv, inSelectEl.value);
                } else {
                    await fillAeInputDeviceCaps(inv, inPick);
                }
            } else if (inListEl) {
                inListEl.textContent = '—';
                if (inSelectEl && typeof inSelectEl.replaceChildren === 'function' && typeof catalogFmt === 'function') {
                    inSelectEl.replaceChildren();
                    const defOpt = document.createElement('option');
                    defOpt.value = '';
                    defOpt.textContent = catalogFmt('ui.ae.input_device_default_option');
                    inSelectEl.appendChild(defOpt);
                }
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
        const startPayload = {cmd: 'start_output_stream', device_id: id, tone: toneOn};
        if (bufferFrames !== undefined) {
            startPayload.buffer_frames = bufferFrames;
        }
        const start = await inv(startPayload);
        throwIfAeNotOk(start, 'start_output_stream failed');
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
