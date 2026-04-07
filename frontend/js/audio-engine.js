// ── Audio Engine tab: separate `audio-engine` process (cpal devices, future plugin graph) ──

const AE_PREFS_DEVICE = 'audioEngineOutputDeviceId';

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
        if (info && info.ok === true && typeof catalogFmt === 'function') {
            const ch = info.channels != null ? String(info.channels) : '?';
            const fmt = info.sample_format != null ? String(info.sample_format) : '?';
            const rate = info.sample_rate_hz != null ? String(info.sample_rate_hz) : '?';
            let rateLabel = rate;
            const r = info.sample_rate_range_hz;
            if (r && r.min != null && r.max != null && String(r.min) !== String(r.max)) {
                rateLabel = `${r.min}–${r.max}`;
            }
            capsEl.textContent = catalogFmt('ui.ae.device_caps', {
                rate: rateLabel,
                channels: ch,
                format: fmt,
            });
        } else {
            capsEl.textContent = '—';
        }
    } catch {
        capsEl.textContent = '—';
    }
}

/**
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 */
async function fillAeStreamStatus(inv) {
    const el = document.getElementById('aeStreamStatus');
    if (!el || typeof inv !== 'function' || typeof catalogFmt !== 'function') {
        if (el) el.textContent = '—';
        return;
    }
    try {
        const st = await inv({cmd: 'output_stream_status'});
        if (st && st.ok === true && st.running === true && st.device_id != null && st.device_id !== '') {
            const name = st.device_name != null ? String(st.device_name) : String(st.device_id);
            const rate = st.sample_rate_hz != null ? String(st.sample_rate_hz) : null;
            const ch = st.channels != null ? String(st.channels) : null;
            const fmt = st.sample_format != null ? String(st.sample_format) : '';
            const buf = st.buffer_size != null ? String(st.buffer_size) : '';
            if (rate != null && ch != null) {
                el.textContent = catalogFmt('ui.ae.output_stream_on_detail', {
                    name,
                    device: String(st.device_id),
                    rate,
                    channels: ch,
                    format: fmt,
                    buffer: buf,
                });
            } else {
                el.textContent = catalogFmt('ui.ae.output_stream_on', {device: String(st.device_id)});
            }
        } else {
            el.textContent = catalogFmt('ui.ae.output_stream_off');
        }
    } catch {
        el.textContent = '—';
    }
}

/**
 * Reload ping, device list, stream status, and plugin stub from the sidecar.
 */
async function refreshAudioEnginePanel() {
    const statusEl = document.getElementById('aeEngineStatus');
    const selectEl = document.getElementById('aeOutputDevice');
    const pluginEl = document.getElementById('aePluginStub');
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
        const ping = await inv({cmd: 'ping'});
        if (!ping || ping.ok !== true) {
            const err = (ping && ping.error) ? String(ping.error) : 'ping failed';
            throw new Error(err);
        }
        const ver = ping.version != null ? String(ping.version) : '?';
        const host = ping.host != null ? String(ping.host) : '?';
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_ok', {version: ver, host});
        }

        await fillAeStreamStatus(inv);

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
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}

async function applyAudioEngineDevice() {
    const selectEl = document.getElementById('aeOutputDevice');
    const statusEl = document.getElementById('aeEngineStatus');
    const inv = window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function'
        ? window.vstUpdater.audioEngineInvoke
        : null;
    if (!inv || !selectEl) return;

    const id = selectEl.value;
    if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
        prefs.setItem(AE_PREFS_DEVICE, id);
    }

    try {
        const r = await inv({cmd: 'set_output_device', device_id: id});
        if (!r || r.ok !== true) {
            const err = (r && r.error) ? String(r.error) : 'set_output_device failed';
            throw new Error(err);
        }
        const start = await inv({cmd: 'start_output_stream', device_id: id});
        if (!start || start.ok !== true) {
            const err = (start && start.error) ? String(start.error) : 'start_output_stream failed';
            throw new Error(err);
        }
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_applied_stream', {id});
        }
        await fillAeDeviceCaps(inv, id);
        await fillAeStreamStatus(inv);
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}

async function stopAeOutputStream() {
    const statusEl = document.getElementById('aeEngineStatus');
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
        const ping = await inv({cmd: 'ping'});
        if (ping && ping.ok === true && statusEl && typeof catalogFmt === 'function') {
            const ver = ping.version != null ? String(ping.version) : '?';
            const host = ping.host != null ? String(ping.host) : '?';
            statusEl.textContent = catalogFmt('ui.ae.status_ok', {version: ver, host});
        }
        await fillAeStreamStatus(inv);
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}
