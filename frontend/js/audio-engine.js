// ── Audio Engine tab: separate `audio-engine` process (JUCE devices + playback; VST3/AU scan; insert chain + native editor windows) ──

const AE_PREFS_DEVICE = 'audioEngineOutputDeviceId';
const AE_PREFS_INPUT_DEVICE = 'audioEngineInputDeviceId';
const AE_PREFS_DEVICE_TYPE = 'audioEngineJuceDeviceType';
const AE_PREFS_SAMPLE_RATE_HZ = 'audioEngineSampleRateHz';
const AE_PREFS_TONE = 'audioEngineTestTone';
const AE_PREFS_BUFFER_FRAMES_OUTPUT = 'audioEngineBufferFramesOutput';
const AE_PREFS_BUFFER_FRAMES_INPUT = 'audioEngineBufferFramesInput';
/** JSON array of up to four host paths for `playback_set_inserts` (UI mirror). */
const AE_PREFS_INSERT_PATHS_JSON = 'audioEngineInsertPathsJson';
/** @deprecated Legacy single pref; migrated once to output/input */
const AE_LEGACY_BUFFER_FRAMES = 'audioEngineBufferFrames';

const AE_INSERT_SLOT_IDS = ['aeInsertSlot0', 'aeInsertSlot1', 'aeInsertSlot2', 'aeInsertSlot3'];
const AE_INSERT_EDITOR_IDS = ['aeInsertEditor0', 'aeInsertEditor1', 'aeInsertEditor2', 'aeInsertEditor3'];

/** After first successful `list_audio_device_types`, restore saved driver from prefs once per page load (and again after sidecar restart). */
let aeInitialDeviceTypeRestored = false;

/** Incremented at the start of each `refreshAudioEnginePanel` so in-flight plugin-scan polls do not apply stale results. */
let aePluginChainPollGeneration = 0;

/** Throttle `toast.ae_plugin_scan_progress` during long scans (`fetchPluginChainUntilSettled`). */
let aePluginScanProgressToastAt = 0;

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

/**
 * @param {object|null} chain — `plugin_chain` payload
 */
function aePopulateInsertSlotSelects(chain) {
    const plugins = chain && Array.isArray(chain.plugins) ? chain.plugins : [];
    const emptyLabel =
        typeof catalogFmt === 'function' ? catalogFmt('ui.ae.insert_slot_empty_opt') : '—';
    for (const id of AE_INSERT_SLOT_IDS) {
        const sel = document.getElementById(id);
        if (!sel || typeof sel.replaceChildren !== 'function') continue;
        sel.replaceChildren();
        const opt0 = document.createElement('option');
        opt0.value = '';
        opt0.textContent = emptyLabel;
        sel.appendChild(opt0);
        for (const p of plugins) {
            const path = p && p.path != null ? String(p.path) : '';
            if (!path) continue;
            const name = p && p.name != null ? String(p.name) : path.split('/').pop();
            const opt = document.createElement('option');
            opt.value = path;
            opt.textContent = name;
            sel.appendChild(opt);
        }
    }
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
    for (let i = 0; i < AE_INSERT_SLOT_IDS.length; i++) {
        const sel = document.getElementById(AE_INSERT_SLOT_IDS[i]);
        if (!sel) continue;
        const v = pick[i] != null ? String(pick[i]).trim() : '';
        if (v && [...sel.options].some((o) => o.value === v)) sel.value = v;
        else sel.value = '';
    }
}

/**
 * Poll `plugin_chain` until the sidecar finishes scanning (`phase` !== `scanning`) or attempts exhausted.
 * @param {function} inv — `audioEngineInvoke`
 * @param {object} [initialChain] — if set, skip the first `plugin_chain` IPC (caller already fetched it).
 * @returns {Promise<object>}
 */
async function fetchPluginChainUntilSettled(inv, initialChain) {
    const delay = (ms) => new Promise((r) => setTimeout(r, ms));
    const toast = (key, params, ms, kind) => {
        if (typeof showToast !== 'function' || typeof toastFmt !== 'function') return;
        showToast(toastFmt(key, params || {}), ms, kind);
    };

    let chain =
        initialChain !== undefined && initialChain !== null
            ? initialChain
            : await inv({cmd: 'plugin_chain'});
    const sawScanning = chain && chain.phase === 'scanning';
    if (sawScanning) {
        toast('toast.ae_plugin_scan_started', {}, 4200, 'info');
    }

    let attempts = 0;
    const maxAttempts = 600;
    aePluginScanProgressToastAt = typeof Date.now === 'function' ? Date.now() - 2600 : 0;
    while (chain && chain.phase === 'scanning' && attempts < maxAttempts) {
        await delay(250);
        chain = await inv({cmd: 'plugin_chain'});
        attempts++;
        const now = typeof Date.now === 'function' ? Date.now() : 0;
        if (chain && chain.phase === 'scanning' && now - aePluginScanProgressToastAt >= 2500) {
            aePluginScanProgressToastAt = now;
            const line = formatAePluginScanProgressLine(chain);
            if (line && typeof showToast === 'function' && typeof toastFmt === 'function') {
                showToast(toastFmt('toast.ae_plugin_scan_progress', {line}), 2200, 'info');
            }
        }
    }

    if (chain && chain.phase === 'scanning' && attempts >= maxAttempts) {
        toast('toast.ae_plugin_scan_timeout', {}, 6500, 'warning');
    } else if (sawScanning && chain && chain.phase === 'juce') {
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
    }

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
 */
function fillAePluginSection(chain) {
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
            prog.textContent = formatAePluginScanProgressLine(chain);
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
 * Map UI insert slot index (0–3) to serial chain index for `playback_open_insert_editor`.
 * @returns {number} chain index, or -1 if this UI slot has no plug-in selected
 */
function aeChainIndexForInsertUiSlot(uiSlotIndex) {
    if (uiSlotIndex < 0 || uiSlotIndex >= AE_INSERT_SLOT_IDS.length) return -1;
    let idx = 0;
    for (let i = 0; i < uiSlotIndex; i++) {
        const sel = document.getElementById(AE_INSERT_SLOT_IDS[i]);
        const v = sel && sel.value != null ? String(sel.value).trim() : '';
        if (v) idx++;
    }
    const sel = document.getElementById(AE_INSERT_SLOT_IDS[uiSlotIndex]);
    const v = sel && sel.value != null ? String(sel.value).trim() : '';
    if (!v) return -1;
    return idx;
}

/**
 * @param {number} uiSlotIndex 0–3 matching `AE_INSERT_SLOT_IDS`
 */
async function openAeInsertEditorForUiSlot(uiSlotIndex) {
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }
    const chainIdx = aeChainIndexForInsertUiSlot(uiSlotIndex);
    if (chainIdx < 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_insert_editor_no_plugin'), 4000, 'warning');
        }
        return;
    }
    try {
        const r = await inv({cmd: 'playback_open_insert_editor', slot: chainIdx});
        throwIfAeNotOk(r, 'playback_open_insert_editor failed');
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_insert_editor_failed', {err}), 5000, 'error');
        }
    }
}

async function applyAePlaybackInserts() {
    const inv = getAeAudioEngineInvoke();
    if (!inv) {
        aeNotifyNoAudioEngineIpc();
        return;
    }
    const paths = [];
    for (const id of AE_INSERT_SLOT_IDS) {
        const sel = document.getElementById(id);
        const v = sel && sel.value != null ? String(sel.value).trim() : '';
        if (v) paths.push(v);
    }
    try {
        const r = await inv({cmd: 'playback_set_inserts', paths});
        throwIfAeNotOk(r, 'playback_set_inserts failed');
        if (typeof prefs !== 'undefined' && typeof prefs.setItem === 'function') {
            prefs.setItem(AE_PREFS_INSERT_PATHS_JSON, JSON.stringify(paths));
        }
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_inserts_applied'), 3000, 'success');
        }
        const chain = await fetchPluginChainUntilSettled(inv);
        fillAePluginSection(chain);
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_inserts_failed', {err}), 5000, 'error');
        }
    }
}

/**
 * Kill the `audio-engine` subprocess; next IPC spawns a fresh process. Clears JS engine playback state.
 */
async function restartAeSidecar() {
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
        if (typeof window !== 'undefined' && typeof window.syncEnginePlaybackStoppedFromSidecar === 'function') {
            window.syncEnginePlaybackStoppedFromSidecar();
        }
        if (typeof window !== 'undefined' && typeof window.stopEnginePlaybackPoll === 'function') {
            window.stopEnginePlaybackPoll();
        }
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_sidecar_restarted'), 3000, 'success');
        }
        void refreshAudioEnginePanel();
    } catch (e) {
        const err = e && e.message ? String(e.message) : String(e);
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.ae_sidecar_restart_failed', {err}), 5000, 'error');
        }
        void refreshAudioEnginePanel();
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
    const restartSidecarBtn = document.getElementById('aeRestartSidecar');
    if (restartSidecarBtn && typeof restartSidecarBtn.addEventListener === 'function') {
        restartSidecarBtn.addEventListener('click', () => {
            void restartAeSidecar();
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
    for (let i = 0; i < AE_INSERT_EDITOR_IDS.length; i++) {
        const edBtn = document.getElementById(AE_INSERT_EDITOR_IDS[i]);
        if (edBtn && typeof edBtn.addEventListener === 'function') {
            const slot = i;
            edBtn.addEventListener('click', () => {
                void openAeInsertEditorForUiSlot(slot);
            });
        }
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

    void refreshAudioEnginePanel();
}

/**
 * @param {function} inv — `window.vstUpdater.audioEngineInvoke`
 * @param {string} deviceId — sidecar device id (stable name-based or legacy index)
 */
async function fillAeDeviceCaps(inv, deviceId) {
    const capsEl = document.getElementById('aeDeviceCaps');
    const srSel = document.getElementById('aeSampleRate');
    if (!capsEl || typeof inv !== 'function') {
        if (capsEl) capsEl.textContent = '—';
        return;
    }
    let prefSr = '';
    if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
        const p = prefs.getItem(AE_PREFS_SAMPLE_RATE_HZ);
        prefSr = p != null ? String(p).trim() : '';
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
    } catch {
        capsEl.textContent = '—';
        if (srSel) {
            aePopulateSampleRateSelect(srSel, null, prefSr);
        }
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
            aeInitialDeviceTypeRestored = true;
            const savedType =
                typeof prefs !== 'undefined' && typeof prefs.getItem === 'function'
                    ? prefs.getItem(AE_PREFS_DEVICE_TYPE)
                    : null;
            const cur = typeRes.current != null ? String(typeRes.current) : '';
            if (savedType != null && String(savedType).trim() !== '' && String(savedType) !== cur) {
                try {
                    const sr = await inv({cmd: 'set_audio_device_type', type: String(savedType).trim()});
                    throwIfAeNotOk(sr, 'set_audio_device_type failed');
                    typeRes = await inv({cmd: 'list_audio_device_types'});
                    throwIfAeNotOk(typeRes, 'list_audio_device_types failed');
                } catch {
                    /* keep engine driver */
                }
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
            void fetchPluginChainUntilSettled(inv, chain).then((finalChain) => {
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
        const srSel = document.getElementById('aeSampleRate');
        if (srSel) {
            prefs.setItem(AE_PREFS_SAMPLE_RATE_HZ, srSel.value != null ? String(srSel.value).trim() : '');
        }
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
        window._engineSpectrumU8 = null;
    }
}

function startEnginePlaybackPoll() {
    stopEnginePlaybackPoll();
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

/** Now-playing speed (0.25–2×) → sidecar `playback_set_speed` (`ResamplingAudioSource`; pitch follows speed like `<audio>.playbackRate`). */
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
    window._engineSpectrumU8 = null;
    if (typeof window.stopEnginePlaybackFftRaf === 'function') window.stopEnginePlaybackFftRaf();
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
    window.syncAeTransportFromPlayback = syncAeTransportFromPlayback;
}
