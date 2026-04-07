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

    void refreshAudioEnginePanel();
}

/**
 * Reload ping, device list, and plugin stub from the sidecar.
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
            } else if (devices.length > 0) {
                const def = devices.find((x) => x.is_default === true);
                selectEl.value = def && def.id != null ? String(def.id) : String(devices[0].id);
            }
        }

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
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_applied', {id});
        }
    } catch (e) {
        const msg = e && e.message ? String(e.message) : String(e);
        if (statusEl && typeof catalogFmt === 'function') {
            statusEl.textContent = catalogFmt('ui.ae.status_error', {message: msg});
        }
    }
}
