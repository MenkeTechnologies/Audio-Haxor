// ── Audio Engine tab: separate `audio-engine` process (cpal devices, future plugin graph) ──

const AE_PREFS_DEVICE = 'audioEngineOutputDeviceId';
const AE_PREFS_TONE = 'audioEngineTestTone';
const AE_PREFS_BUFFER_FRAMES = 'audioEngineBufferFrames';

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
    if (!root || root.dataset.aeInit === '1') return;
    root.dataset.aeInit = '1';

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
    const bufIn = document.getElementById('aeBufferFrames');
    if (toneCb && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        toneCb.checked = prefs.getItem(AE_PREFS_TONE) === '1';
    }
    if (bufIn && typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const saved = prefs.getItem(AE_PREFS_BUFFER_FRAMES);
        bufIn.value = saved != null && String(saved) !== '' ? String(saved) : '';
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
    if (bufIn && typeof bufIn.addEventListener === 'function' && typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        const saveBufPref = () => {
            prefs.setItem(AE_PREFS_BUFFER_FRAMES, bufIn.value != null ? String(bufIn.value).trim() : '');
        };
        bufIn.addEventListener('change', saveBufPref);
        bufIn.addEventListener('blur', saveBufPref);
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
 * Default input device (`get_input_device_info` with no `device_id`).
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 */
async function fillAeInputDeviceCaps(inv) {
    const el = document.getElementById('aeInputDeviceCaps');
    if (!el || typeof inv !== 'function') {
        if (el) el.textContent = '—';
        return;
    }
    try {
        const info = await inv({cmd: 'get_input_device_info'});
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
        if (st.tone_on === true && st.tone_supported === true && typeof catalogFmt === 'function') {
            line += catalogFmt('ui.ae.tone_active');
        }
        const sbf = st.stream_buffer_frames;
        if (sbf != null && typeof sbf === 'number' && Number.isFinite(sbf) && typeof catalogFmt === 'function') {
            line += catalogFmt('ui.ae.stream_buffer_fixed', {frames: String(sbf)});
        }
        el.textContent = line;
    } else {
        el.textContent = catalogFmt('ui.ae.output_stream_off');
    }
}

/**
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 */
async function fillAeStreamStatus(inv) {
    const el = document.getElementById('aeStreamStatus');
    if (!el || typeof inv !== 'function') {
        if (el) el.textContent = '—';
        return;
    }
    try {
        const st = await inv({cmd: 'output_stream_status'});
        fillAeStreamLineFromPayload(st, el);
    } catch {
        el.textContent = '—';
    }
}

/**
 * Reload engine_state (ping + stream), device list, caps, plugin stub.
 */
async function refreshAudioEnginePanel() {
    const statusEl = document.getElementById('aeEngineStatus');
    const streamEl = document.getElementById('aeStreamStatus');
    const selectEl = document.getElementById('aeOutputDevice');
    const pluginEl = document.getElementById('aePluginStub');
    const toneCb = document.getElementById('aeTestTone');
    const inv = window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
        ? window.vstUpdater.audioEngineInvoke
        : null;

    if (!inv) {
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.err_no_ipc');
        }
        return;
    }

    if (statusEl && typeof catalogFmt === 'function') {
        statusEl.textContent = catalogFmt('ui.ae.status_loading');
    }

    try {
        const es = await inv({cmd: 'engine_state'});
        if (!es || es.ok !== true) {
            const err = (es && es.error) ? String(es.error) : 'engine_state failed';
            throw new Error(err);
        }
        const ver = es.version != null ? String(es.version) : '?';
        const host = es.host != null ? String(es.host) : '?';
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_ok', {version: ver, host});
        }
        if (streamEl && es.stream) {
            fillAeStreamLineFromPayload(es.stream, streamEl);
        }
        if (toneCb && typeof toneCb.disabled === 'boolean') {
            const ts = es.stream;
            const canTone = ts && ts.running === true && ts.tone_supported === true;
            toneCb.disabled = !canTone;
            if (canTone && ts.tone_on != null) {
                toneCb.checked = ts.tone_on === true;
            }
        }

        const list = await inv({cmd: 'list_output_devices'});
        if (!list || list.ok !== true) {
            const err = (list && list.error) ? String(list.error) : 'list_output_devices failed';
            throw new Error(err);
        }
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
            } else if (inListEl) {
                inListEl.textContent = '—';
            }
        } catch {
            if (inListEl) inListEl.textContent = '—';
        }
        await fillAeInputDeviceCaps(inv);
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}

/**
 * Toggle test tone on the live stream (F32 only).
 * @param {boolean} enabled
 */
async function toggleAeTestTone(enabled) {
    const inv = window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
        ? window.vstUpdater.audioEngineInvoke
        : null;
    const streamEl = document.getElementById('aeStreamStatus');
    if (!inv) return;
    try {
        const r = await inv({cmd: 'set_output_tone', tone: enabled});
        if (!r || r.ok !== true) {
            const err = (r && r.error) ? String(r.error) : 'set_output_tone failed';
            throw new Error(err);
        }
        const es = await inv({cmd: 'engine_state'});
        if (es && es.stream && streamEl) {
            fillAeStreamLineFromPayload(es.stream, streamEl);
        }
        const toneCb = document.getElementById('aeTestTone');
        if (toneCb && es && es.stream && es.stream.tone_on != null) {
            toneCb.checked = es.stream.tone_on === true;
        }
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (streamEl && typeof catalogFmt === 'function') {
            streamEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
        const toneCb = document.getElementById('aeTestTone');
        if (toneCb) toneCb.checked = !enabled;
    }
}

async function applyAudioEngineDevice() {
    const selectEl = document.getElementById('aeOutputDevice');
    const statusEl = document.getElementById('aeEngineStatus');
    const streamEl = document.getElementById('aeStreamStatus');
    const toneCb = document.getElementById('aeTestTone');
    const bufIn = document.getElementById('aeBufferFrames');
    const inv = window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
        ? window.vstUpdater.audioEngineInvoke
        : null;
    if (!inv || !selectEl) return;

    const id = selectEl.value;
    const toneOn = toneCb && toneCb.checked === true;
    const bfRaw = bufIn && typeof bufIn.value === 'string' ? bufIn.value : '';
    const bufferFrames = parseAeBufferFramesPref(bfRaw);
    if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        prefs.setItem(AE_PREFS_DEVICE, id);
        prefs.setItem(AE_PREFS_TONE, toneOn ? '1' : '0');
        prefs.setItem(AE_PREFS_BUFFER_FRAMES, bfRaw.trim());
    }

    try {
        const r = await inv({cmd: 'set_output_device', device_id: id});
        if (!r || r.ok !== true) {
            const err = (r && r.error) ? String(r.error) : 'set_output_device failed';
            throw new Error(err);
        }
        const startPayload = {cmd: 'start_output_stream', device_id: id, tone: toneOn};
        if (bufferFrames !== undefined) {
            startPayload.buffer_frames = bufferFrames;
        }
        const start = await inv(startPayload);
        if (!start || start.ok !== true) {
            const err = (start && start.error) ? String(start.error) : 'start_output_stream failed';
            throw new Error(err);
        }
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_applied_stream', {id});
        }
        await fillAeDeviceCaps(inv, id);
        const es = await inv({cmd: 'engine_state'});
        if (es && es.stream && streamEl) {
            fillAeStreamLineFromPayload(es.stream, streamEl);
        }
        if (toneCb && es && es.stream) {
            toneCb.disabled = !(es.stream.running === true && es.stream.tone_supported === true);
            if (es.stream.tone_on != null) toneCb.checked = es.stream.tone_on === true;
        }
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}

async function stopAeOutputStream() {
    const statusEl = document.getElementById('aeEngineStatus');
    const streamEl = document.getElementById('aeStreamStatus');
    const toneCb = document.getElementById('aeTestTone');
    const inv = window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
        ? window.vstUpdater.audioEngineInvoke
        : null;
    if (!inv) return;

    try {
        const r = await inv({cmd: 'stop_output_stream'});
        if (!r || r.ok !== true) {
            const err = (r && r.error) ? String(r.error) : 'stop_output_stream failed';
            throw new Error(err);
        }
        const es = await inv({cmd: 'engine_state'});
        if (es && es.ok === true && statusEl && typeof catalogFmt === 'function') {
            const ver = es.version != null ? String(es.version) : '?';
            const host = es.host != null ? String(es.host) : '?';
            statusEl.textContent = catalogFmt('ui.ae.status_ok', {version: ver, host});
        }
        if (streamEl && es && es.stream) {
            fillAeStreamLineFromPayload(es.stream, streamEl);
        }
        if (toneCb) {
            toneCb.disabled = true;
            toneCb.checked = false;
        }
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}
