/**
 * Menu-bar tray popover (WebView window `tray-popover`): Apple-style layout without artwork.
 * Window size is synced to `#shell` via `tray_popover_resize` (logical/CSS px — matches HiDPI layout).
 */
(function () {
    const TRAY_LOG_VERBOSE =
        typeof window !== 'undefined' && window.__TRAY_POPOVER_DEBUG === true;

    function trayDbg(...args) {
        if (TRAY_LOG_VERBOSE) console.info('[tray-popover]', ...args);
    }

    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    const listen = tauri && tauri.event && typeof tauri.event.listen === 'function' ? tauri.event.listen : null;
    const invoke =
        tauri && tauri.core && typeof tauri.core.invoke === 'function' ? tauri.core.invoke : null;

    /** Tray window does not load `ipc.js` — mirror minimal `appFmt` + SQLite strings for tooltips. */
    window.__appStr = window.__appStr || {};
    if (typeof window.appFmt !== 'function') {
        window.appFmt = function (key, vars) {
            const map = window.__appStr;
            let s = map && map[key];
            if (s == null || s === '') return key;
            if (vars && typeof vars === 'object') {
                s = s.replace(/\{(\w+)\}/g, (_, name) =>
                    vars[name] != null && vars[name] !== '' ? String(vars[name]) : ''
                );
            }
            return s;
        };
    }
    const _trayI18nReady = invoke
        ? invoke('get_app_strings', {locale: null})
              .then((m) => {
                  window.__appStr = m || {};
              })
              .catch(() => {})
        : Promise.resolve();

    /** First resolved translation: `appFmt(k) !== k`, trying `primary` then `alts`. */
    function appFmtResolved(primary, ...alts) {
        const pick = (key) => {
            if (!key) return '';
            const s = window.appFmt(key);
            return s && s !== key ? s : '';
        };
        let v = pick(primary);
        if (v) return v;
        for (let i = 0; i < alts.length; i++) {
            v = pick(alts[i]);
            if (v) return v;
        }
        return '';
    }

    /* `getCurrentWebviewWindow()` must run after the webview exists — call in init, not at parse time. */
    function getTrayWebviewWindow() {
        return tauri && tauri.webviewWindow && typeof tauri.webviewWindow.getCurrentWebviewWindow === 'function'
            ? tauri.webviewWindow.getCurrentWebviewWindow()
            : null;
    }

    /** Tray IPC may use snake_case (`ui_theme`) or camelCase (`uiTheme`) depending on serializer. */
    function extractUiTheme(obj) {
        if (!obj || typeof obj !== 'object') return null;
        if (typeof obj.ui_theme === 'string') return obj.ui_theme;
        if (typeof obj.uiTheme === 'string') return obj.uiTheme;
        return null;
    }

    function extractAppearance(obj) {
        if (!obj || typeof obj !== 'object') return null;
        const a = obj.appearance;
        return a && typeof a === 'object' && !Array.isArray(a) ? a : null;
    }

    /**
     * `emit_to` delivers the tray struct as **`event.payload`** (Tauri `Event<T>`). Some bridges also pass
     * JSON strings. Rarely, a mistaken double-wrap `{ payload: { payload: state } }` appears — unwrap
     * up to a few levels. This is **not** the same as `invoke('cmd', { payload: … })` — events never use
     * that outer key; only the Event wrapper’s `.payload` holds the HUD state.
     */
    function trayListenUnwrap(arg) {
        if (arg == null) return null;
        let cur = arg;
        if (typeof cur === 'string') {
            try {
                cur = JSON.parse(cur);
            } catch {
                return null;
            }
        }
        let depth = 0;
        while (
            depth < 5 &&
            cur &&
            typeof cur === 'object' &&
            !Array.isArray(cur) &&
            Object.prototype.hasOwnProperty.call(cur, 'payload') &&
            cur.payload != null
        ) {
            const next = cur.payload;
            if (typeof next === 'string') {
                try {
                    cur = JSON.parse(next);
                } catch {
                    break;
                }
            } else {
                cur = next;
            }
            depth++;
        }
        return cur && typeof cur === 'object' ? cur : null;
    }

    /** Main-window scheme vars (`--cyan`, …) → popover `document.documentElement` (feeds `--cp-*` aliases in CSS). */
    function applyTrayAppearanceFromPayload(map, source) {
        if (!map || typeof map !== 'object') return;
        const root = document.documentElement.style;
        let applied = 0;
        const keys = [];
        for (const [k, v] of Object.entries(map)) {
            if (typeof k === 'string' && k.startsWith('--') && typeof v === 'string' && v.length > 0) {
                root.setProperty(k, v);
                applied++;
                keys.push(k);
            }
        }
        if (applied > 0) {
            console.info('[tray-popover] colorscheme applied', {
                source: source || 'payload',
                css_var_count: applied,
                keys_sample: keys.slice(0, 8),
                cyan: map['--cyan'],
            });
        }
    }

    /* Do not hide on blur: focus moves to the tray before the tray Click event, which would fight Rust toggle. */

    const shell = document.getElementById('shell');
    const elTitle = document.getElementById('trayPopoverTitle');
    const elSub = document.getElementById('subtitle');
    const elIdle = document.getElementById('idleHint');
    const elFill = document.getElementById('fill');
    const elThumb = document.getElementById('trackThumb');
    const elTrackBar = document.getElementById('trackBar');
    const elElapsed = document.getElementById('elapsed');
    const elTotal = document.getElementById('total');
    const btnShuffle = document.getElementById('btnShuffle');
    const btnPrev = document.getElementById('btnPrev');
    const btnPlay = document.getElementById('btnPlay');
    const btnNext = document.getElementById('btnNext');
    const btnLoop = document.getElementById('btnLoop');
    const elTrayVol = document.getElementById('trayVol');
    const elTrayVolPct = document.getElementById('trayVolPct');
    const elTrayVolLabel = document.getElementById('trayVolLabel');
    const elTraySpeed = document.getElementById('traySpeed');
    const elTraySpeedLabel = document.getElementById('traySpeedLabel');
    const trayCtx = document.getElementById('trayCtxMenu');
    const elProgressWrap = document.getElementById('trayProgressWrap');
    const elTrayExtras = document.getElementById('trayExtras');
    const elTransport = shell && typeof shell.querySelector === 'function' ? shell.querySelector('.transport') : null;

    /** Filesystem path for the playing file — from host `reveal_path` (copy / reveal / click subtitle). */
    let _trayRevealPath = '';

    /** `subtitle` + `reveal_path` — skip rebuilding `#subtitle` on every tray tick (~500 ms) so clicks/toggles work. */
    let _traySubtitleSig = null;

    /** User-expanded path panel; preserved until track/meta path changes. */
    let _trayPathUserExpanded = false;

    /** True briefly after local volume `input` — ignore host volume so poll ticks do not overwrite the slider. */
    let _trayVolUserActive = false;
    let _trayVolUserTimer = null;
    /* Must outlive one full Rust `start_tray_host_poll` cycle (500 ms) plus the main window's
     * debounced `syncTrayNowPlayingFromPlayback` (150 ms) plus IPC round-trip. 400 ms was too
     * tight — a host poll firing mid-drag at t≈450 ms landed with a stale `volume_pct` just
     * after the guard expired, snapping the slider back. 1200 ms covers two poll cycles. */
    const TRAY_VOL_USER_SETTLE_MS = 1200;

    /** Same values as main `#npSpeed` — tray window does not load the full index bundle. */
    const TRAY_SPEED_OPTIONS = [
        { value: '0.25', i18n: 'ui.opt.0_25x' },
        { value: '0.5', i18n: 'ui.opt.0_5x' },
        { value: '0.75', i18n: 'ui.opt.0_75x' },
        { value: '1', i18n: 'ui.opt.1x' },
        { value: '1.25', i18n: 'ui.opt.1_25x' },
        { value: '1.5', i18n: 'ui.opt.1_5x' },
        { value: '2', i18n: 'ui.opt.2x' },
    ];

    let _trayApplyingHostControls = false;
    let _trayPopoverDomTheme = '';

    function applyTrayDocumentTheme(theme) {
        const t = theme === 'light' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', t);
        if (t !== _trayPopoverDomTheme) {
            _trayPopoverDomTheme = t;
            console.info('[tray-popover] documentElement data-theme ->', t);
        }
        scheduleResize();
    }

    let _trayPopoverApplyLog = { idle: null, playing: null, ui: null };

    function logTrayPopoverApplyState(p, idle, playing, themed) {
        const ui = themed || document.documentElement.getAttribute('data-theme') || '';
        if (_trayPopoverApplyLog.idle === idle && _trayPopoverApplyLog.playing === playing && _trayPopoverApplyLog.ui === ui) {
            return;
        }
        _trayPopoverApplyLog = { idle, playing, ui };
        const title = typeof p.title === 'string' ? p.title : '';
        const sub = typeof p.subtitle === 'string' ? p.subtitle : '';
        const appMap = extractAppearance(p);
        const appKeys = appMap ? Object.keys(appMap).filter((k) => k.startsWith('--')) : [];
        console.info('[tray-popover] applyState', {
            idle,
            playing,
            ui_theme: ui,
            titleLen: title.length,
            subtitleLen: sub.length,
            appearance_in_payload: appKeys.length,
        });
    }

    function fmt(sec) {
        if (typeof sec !== 'number' || !Number.isFinite(sec) || sec < 0) return '0:00';
        const m = Math.floor(sec / 60);
        const s = Math.floor(sec % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
    }

    /** Logical size for `#shell` + document root; extra padding avoids WebKit clipping range rows / descenders. */
    const TRAY_WIN_PAD_W = 8;
    const TRAY_WIN_PAD_H = 18;

    function syncWindowSize() {
        if (!invoke) return;
        const root = document.getElementById('shell');
        if (!root) return;
        /* Measure the `#shell` intrinsic size ONLY. The previous implementation took
         * `Math.max(shellH, bodyH, htmlH)` which looked safe but is a positive feedback loop:
         * `body` has `min-height: 100%` in CSS (see `tray-popover.html`), so `bodyH` always
         * equals the current window height — once the window is large for any reason (initial
         * `TRAY_POPOVER_H = 480`, a prior expanded state), the `max` picks up the window size
         * instead of the content and the window can only grow, never shrink. The result was a
         * huge transparent popover window swallowing clicks far beyond the visible frame. */
        const br = root.getBoundingClientRect();
        const shellH = Math.ceil(Math.max(root.scrollHeight, root.offsetHeight, br.height));
        const shellW = Math.ceil(Math.max(root.scrollWidth, root.offsetWidth, br.width));
        const h = shellH + TRAY_WIN_PAD_H;
        const w = shellW + TRAY_WIN_PAD_W;
        void invoke('tray_popover_resize', { width: w, height: h }).catch(() => {});
    }

    function scheduleResize() {
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                syncWindowSize();
            });
        });
    }

    function normalizePayload(raw) {
        if (raw == null) return null;
        let o = raw;
        if (typeof o === 'string') {
            try {
                o = JSON.parse(o);
            } catch {
                return null;
            }
        }
        if (!o || typeof o !== 'object') return null;
        const p = { ...o };
        let elapsed = p.elapsed_sec ?? p.elapsedSec;
        if (typeof elapsed === 'string') elapsed = parseFloat(elapsed);
        if (typeof elapsed !== 'number' || !Number.isFinite(elapsed)) elapsed = 0;
        let total = p.total_sec ?? p.totalSec;
        if (typeof total === 'string') total = parseFloat(total);
        if (typeof total === 'number' && Number.isFinite(total) && total > 0) {
            p.total_sec = total;
        } else {
            p.total_sec = null;
        }
        p.elapsed_sec = elapsed;
        if (p.idle_hint == null && p.idleHint != null) p.idle_hint = p.idleHint;
        if (p.ui_theme == null && p.uiTheme != null) p.ui_theme = p.uiTheme;
        let vol = p.volume_pct ?? p.volumePct;
        if (typeof vol === 'string') vol = parseInt(vol, 10);
        if (typeof vol !== 'number' || !Number.isFinite(vol)) vol = 100;
        p.volume_pct = Math.max(0, Math.min(100, Math.round(vol)));
        let pSpeed = p.playback_speed ?? p.playbackSpeed;
        if (typeof pSpeed === 'string') pSpeed = parseFloat(pSpeed);
        if (typeof pSpeed !== 'number' || !Number.isFinite(pSpeed)) pSpeed = 1;
        p.playback_speed = Math.max(0.25, Math.min(2, pSpeed));
        let rp = p.reveal_path ?? p.revealPath;
        if (rp == null || typeof rp !== 'string') {
            p.reveal_path = '';
        } else {
            p.reveal_path = String(rp).trim();
        }
        return p;
    }

    function applyTrayExtrasFromState(volumePct, playbackSpeed) {
        _trayApplyingHostControls = true;
        try {
            if (!_trayVolUserActive) {
                if (elTrayVol) elTrayVol.value = String(volumePct);
                if (elTrayVolPct) elTrayVolPct.textContent = `${volumePct}%`;
            }
            if (elTraySpeed && elTraySpeed.options.length > 0) {
                const sp = playbackSpeed;
                let bestIdx = 0;
                let bestDiff = Infinity;
                for (let i = 0; i < elTraySpeed.options.length; i++) {
                    const ov = parseFloat(elTraySpeed.options[i].value);
                    if (!Number.isFinite(ov)) continue;
                    const d = Math.abs(ov - sp);
                    if (d < bestDiff) {
                        bestDiff = d;
                        bestIdx = i;
                    }
                }
                elTraySpeed.selectedIndex = bestIdx;
            }
        } finally {
            _trayApplyingHostControls = false;
        }
    }

    function populateTraySpeedSelect() {
        if (!elTraySpeed) return;
        elTraySpeed.textContent = '';
        for (const row of TRAY_SPEED_OPTIONS) {
            const opt = document.createElement('option');
            opt.value = row.value;
            const label = window.appFmt(row.i18n);
            opt.textContent = label && label !== row.i18n ? label : `${row.value}×`;
            elTraySpeed.appendChild(opt);
        }
    }
    populateTraySpeedSelect();

    function trayPopoverCopyText(text) {
        const s = text != null ? String(text).trim() : '';
        if (!s || !navigator.clipboard || typeof navigator.clipboard.writeText !== 'function') return;
        void navigator.clipboard.writeText(s).catch(() => {});
    }

    function hideTrayCtxMenu() {
        if (!trayCtx) return;
        trayCtx.classList.remove('visible');
        trayCtx.replaceChildren();
        trayCtx.setAttribute('aria-hidden', 'true');
        trayCtx._actions = null;
    }

    function buildTrayCtxItems() {
        const items = [];
        items.push({
            label: window.appFmt('tray.show'),
            action: () => {
                if (!invoke) return;
                void invoke('show_main_window').catch(() => {});
            },
        });
        items.push('---');
        if (!_currentIdle) {
            items.push({
                label: window.appFmt('tray.previous_track'),
                action: () => send('prev_track'),
            });
            items.push({
                label: window.appFmt('tray.play_pause'),
                action: () => send('play_pause'),
            });
            items.push({
                label: window.appFmt('tray.next_track'),
                action: () => send('next_track'),
            });
            items.push('---');
        }
        items.push({
            label: window.appFmt('tray.scan_all'),
            action: () => send('scan_all'),
        });
        items.push({
            label: window.appFmt('tray.stop_all'),
            action: () => send('stop_all'),
        });
        const subLine = trayPopoverSubtitleUiSummary();
        if (_trayRevealPath) {
            items.push('---');
            items.push({
                label: window.appFmt('menu.reveal_in_finder'),
                action: () => {
                    if (!invoke || !_trayRevealPath) return;
                    void invoke('open_audio_folder', { filePath: _trayRevealPath }).catch(() => {});
                },
            });
            items.push({
                label: window.appFmt('menu.copy_file_path'),
                action: () => trayPopoverCopyText(_trayRevealPath),
            });
        } else if (subLine) {
            items.push('---');
            items.push({
                label: window.appFmt('menu.copy_file_path'),
                action: () => trayPopoverCopyText(subLine),
            });
        }
        items.push('---');
        items.push({
            label: window.appFmt('menu.close'),
            action: () => {
                const tw = getTrayWebviewWindow();
                if (tw && typeof tw.hide === 'function') void tw.hide().catch(() => {});
            },
        });
        return items;
    }

    function showTrayCtxMenu(e) {
        if (!trayCtx) return;
        hideTrayCtxMenu();
        const items = buildTrayCtxItems();
        const actions = [];
        for (const item of items) {
            if (item === '---') {
                const sep = document.createElement('div');
                sep.className = 'tray-ctx-sep';
                trayCtx.appendChild(sep);
                continue;
            }
            const div = document.createElement('div');
            div.className = 'tray-ctx-item';
            div.textContent = item.label;
            const idx = actions.length;
            actions.push(item.action);
            div.dataset.trayCtxIdx = String(idx);
            trayCtx.appendChild(div);
        }
        trayCtx._actions = actions;
        trayCtx.classList.add('visible');
        trayCtx.setAttribute('aria-hidden', 'false');
        let x = e.clientX;
        let y = e.clientY;
        trayCtx.style.left = '0px';
        trayCtx.style.top = '0px';
        const rw = trayCtx.offsetWidth;
        const rh = trayCtx.offsetHeight;
        const vw = window.innerWidth;
        const vh = window.innerHeight;
        if (x + rw > vw - 4) x = Math.max(4, vw - rw - 4);
        if (y + rh > vh - 4) y = Math.max(4, vh - rh - 4);
        trayCtx.style.left = `${x}px`;
        trayCtx.style.top = `${y}px`;
    }

    if (trayCtx) {
        trayCtx.addEventListener('click', (e) => {
            const it = e.target && e.target.closest ? e.target.closest('.tray-ctx-item') : null;
            if (!it || !trayCtx._actions) return;
            const idx = parseInt(it.dataset.trayCtxIdx, 10);
            const act = trayCtx._actions[idx];
            hideTrayCtxMenu();
            if (typeof act === 'function') act();
        });
    }

    document.addEventListener(
        'click',
        (e) => {
            if (!trayCtx || !trayCtx.classList.contains('visible')) return;
            if (trayCtx.contains(e.target)) return;
            hideTrayCtxMenu();
        },
        true
    );

    document.addEventListener('contextmenu', (e) => {
        const t = e.target;
        if (t && t.closest && t.closest('input, textarea, select, option')) return;
        e.preventDefault();
        showTrayCtxMenu(e);
    });

    function syncTrayPopoverTooltips() {
        if (elTitle) {
            const t = elTitle.textContent.trim();
            elTitle.title = t;
        }
        if (elSub) {
            elSub.removeAttribute('role');
            const pathSpan = elSub.querySelector('.tray-subtitle-path');
            const copyT = window.appFmt('menu.copy_file_path');
            if (pathSpan && _trayRevealPath && !_currentIdle) {
                pathSpan.title = window.appFmt('menu.reveal_in_finder');
                elSub.removeAttribute('title');
            } else {
                if (pathSpan) {
                    pathSpan.removeAttribute('title');
                    pathSpan.removeAttribute('tabIndex');
                    pathSpan.removeAttribute('role');
                    pathSpan.classList.remove('tray-subtitle-reveal');
                }
                const hasPathPanel = !!elSub.querySelector('.tray-subtitle-path-panel');
                if (!hasPathPanel) {
                    const plain = elSub.textContent.trim();
                    elSub.title = plain ? copyT : '';
                } else {
                    elSub.removeAttribute('title');
                }
            }
        }
        if (elProgressWrap) {
            elProgressWrap.title = appFmtResolved(
                'ui.audio.meta_waveform_canvas_tt',
                'ui.audio.meta_waveform_seek_title'
            );
        }
    }

    function revealFromTraySubtitle() {
        if (!_trayRevealPath || _currentIdle || !invoke) return;
        /* Hide the popover BEFORE revealing in Finder. The popover window is created with
         * `alwaysOnTop: true` + `visibleOnAllWorkspaces: true` + `transparent: true`, so if we
         * leave it visible while Finder activates, the popover stays layered over Finder and
         * every click the user makes to interact with Finder lands on the popover's transparent
         * region instead. That traps the user in what looks like a "focus recursion" — clicks
         * either re-fire the reveal handler (if they hit the path span) or silently sink into
         * the popover's hit region. Hiding first gets the overlay out of the way so Finder is
         * actually interactable. User can reopen the popover from the menubar icon. */
        const tw = getTrayWebviewWindow();
        const path = _trayRevealPath;
        if (tw && typeof tw.hide === 'function') {
            void tw.hide().catch(() => {});
        }
        void invoke('open_audio_folder', { filePath: path }).catch(() => {});
    }

    /** Meta + toggle label only — avoids pulling hidden path text into context menu / tooltips. */
    function trayPopoverSubtitleUiSummary() {
        if (!elSub) return '';
        const bits = [];
        const meta = elSub.querySelector('.tray-subtitle-meta');
        if (meta && meta.textContent.trim()) bits.push(meta.textContent.trim());
        const lab = elSub.querySelector('.tray-subtitle-path-toggle-label');
        if (lab && lab.textContent.trim()) bits.push(lab.textContent.trim());
        return bits.join(' \u2022 ');
    }

    function trayPathToggleLabels(expanded) {
        const btn = elSub && elSub.querySelector('.tray-subtitle-path-toggle');
        if (!btn) return;
        const lab = btn.querySelector('.tray-subtitle-path-toggle-label');
        if (!lab) return;
        const hide = appFmtResolved('tray.path_collapse', 'tray.path_toggle_collapse_tt');
        const show = appFmtResolved('tray.path_expand', 'tray.path_toggle_expand_tt');
        lab.textContent = expanded ? hide : show;
        btn.title = expanded
            ? appFmtResolved('tray.path_toggle_collapse_tt', 'tray.path_collapse')
            : appFmtResolved('tray.path_toggle_expand_tt', 'tray.path_expand');
    }

    function refreshTrayPathToggleI18n() {
        const btn = elSub && elSub.querySelector('.tray-subtitle-path-toggle');
        if (!btn) return;
        trayPathToggleLabels(btn.getAttribute('aria-expanded') === 'true');
    }

    function applyTrayPathPanelDom(expanded) {
        if (!elSub) return;
        const btn = elSub.querySelector('.tray-subtitle-path-toggle');
        const panel = elSub.querySelector('.tray-subtitle-path-panel');
        const pathSpan = elSub.querySelector('.tray-subtitle-path');
        if (!btn || !panel) return;
        btn.setAttribute('aria-expanded', expanded ? 'true' : 'false');
        panel.hidden = !expanded;
        panel.setAttribute('aria-hidden', expanded ? 'false' : 'true');
        if (pathSpan) pathSpan.tabIndex = expanded ? 0 : -1;
        trayPathToggleLabels(expanded);
        syncTrayPopoverTooltips();
    }

    function setTrayPathPanelExpanded(expanded) {
        _trayPathUserExpanded = expanded;
        applyTrayPathPanelDom(expanded);
        scheduleResize();
    }

    function makeTrayPathToggleAndPanel(pathDisplay) {
        const toggle = document.createElement('button');
        toggle.type = 'button';
        toggle.className = 'tray-subtitle-path-toggle';
        toggle.setAttribute('aria-expanded', 'false');
        const panel = document.createElement('div');
        panel.className = 'tray-subtitle-path-panel';
        panel.id = 'traySubtitlePathPanel';
        panel.hidden = true;
        panel.setAttribute('aria-hidden', 'true');
        toggle.setAttribute('aria-controls', 'traySubtitlePathPanel');

        const chev = document.createElement('span');
        chev.className = 'tray-subtitle-path-chevron';
        chev.setAttribute('aria-hidden', 'true');
        chev.textContent = '\u25BC';
        const lab = document.createElement('span');
        lab.className = 'tray-subtitle-path-toggle-label';
        toggle.appendChild(chev);
        toggle.appendChild(lab);

        const pathSpan = document.createElement('span');
        pathSpan.className = 'tray-subtitle-path tray-subtitle-reveal';
        pathSpan.textContent = pathDisplay;
        pathSpan.tabIndex = -1;
        pathSpan.setAttribute('role', 'button');
        panel.appendChild(pathSpan);
        const panAl = appFmtResolved('tray.path_panel_accessible_name', 'tray.path_expand');
        if (panAl) panel.setAttribute('aria-label', panAl);
        return { toggle, panel };
    }

    /** Meta row + collapsible path (`reveal_path`); path hidden by default. */
    function renderTraySubtitle(metaRaw, revealPathTrimmed) {
        if (!elSub) return;
        elSub.replaceChildren();
        const meta = metaRaw != null ? String(metaRaw).trim() : '';
        const pathDisp =
            revealPathTrimmed && typeof revealPathTrimmed === 'string' && revealPathTrimmed.trim() !== ''
                ? revealPathTrimmed.replace(/\\/g, '/')
                : '';
        if (!meta && !pathDisp) {
            elSub.replaceChildren();
            return;
        }

        if (pathDisp) {
            const { toggle, panel } = makeTrayPathToggleAndPanel(pathDisp);
            const row = document.createElement('div');
            row.className = 'tray-subtitle-row';
            if (meta) {
                const metaSpan = document.createElement('span');
                metaSpan.className = 'tray-subtitle-meta';
                metaSpan.textContent = meta;
                row.appendChild(metaSpan);
                row.appendChild(document.createTextNode(' \u2022 '));
            }
            row.appendChild(toggle);
            elSub.appendChild(row);
            elSub.appendChild(panel);
        } else {
            const metaSpan = document.createElement('span');
            metaSpan.className = 'tray-subtitle-meta';
            metaSpan.textContent = meta;
            elSub.appendChild(metaSpan);
        }
    }

    if (elSub) {
        elSub.addEventListener('click', (e) => {
            const tgl = e.target && e.target.closest ? e.target.closest('.tray-subtitle-path-toggle') : null;
            if (tgl) {
                e.preventDefault();
                e.stopPropagation();
                const exp = tgl.getAttribute('aria-expanded') === 'true';
                setTrayPathPanelExpanded(!exp);
                return;
            }
            if (!_trayRevealPath || _currentIdle) return;
            const hit = e.target && e.target.closest ? e.target.closest('.tray-subtitle-path') : null;
            if (!hit) return;
            revealFromTraySubtitle();
        });
        elSub.addEventListener('keydown', (e) => {
            if (e.key !== 'Enter' && e.key !== ' ') return;
            const t = e.target;
            const tgl = t && t.closest ? t.closest('.tray-subtitle-path-toggle') : null;
            if (tgl) {
                e.preventDefault();
                const exp = tgl.getAttribute('aria-expanded') === 'true';
                setTrayPathPanelExpanded(!exp);
                return;
            }
            if (!_trayRevealPath || _currentIdle) return;
            const onPath = t && t.closest && t.closest('.tray-subtitle-path') === t;
            if (!onPath) return;
            e.preventDefault();
            revealFromTraySubtitle();
        });
    }

    /**
     * Local playback model — between host pushes (every ~500 ms), a **`requestAnimationFrame`** loop
     * interpolates elapsed time against `performance.now()` so the slider animates smoothly at 60 fps
     * instead of stepping in 500 ms increments. The model is paused while the user drags the thumb and
     * resumes from the new base after `pointerup`.
     */
    let _baseElapsed = 0;
    let _baseTime = performance.now();
    let _currentTotal = null;
    let _currentPlaying = false;
    let _currentIdle = true;
    let _dragging = false;
    /** Active scrub pointer — window-level `pointermove`/`pointerup` (no `setPointerCapture`; capture broke hit testing vs `#trayVol`). */
    let _trayScrubPointerId = null;
    /** True once the pointer moves over vol/times/transport/etc.; `pointerup` must not `sendSeek`. */
    let _trayScrubCancelled = false;
    let _dragFrac = 0;
    let _rafId = null;
    /** Last `applyState` progress tuple from host — when unchanged, ignore drift (shuffle/loop-only
     * emits while main is hidden can carry a stale `elapsed_sec` vs local rAF interpolation). */
    let _trayLastApplyProgressKey = null;

    function renderProgress(elapsed, total) {
        const tot = typeof total === 'number' && Number.isFinite(total) && total > 0 ? total : null;
        let pct = 0;
        if (tot != null) pct = Math.min(100, Math.max(0, (elapsed / tot) * 100));
        if (elFill) elFill.style.width = `${pct}%`;
        if (elThumb) elThumb.style.left = `${pct}%`;
        if (elElapsed) elElapsed.textContent = fmt(Math.max(0, tot != null ? Math.min(elapsed, tot) : elapsed));
    }

    function animationTick() {
        _rafId = null;
        if (_currentIdle) return;
        if (_dragging) {
            /* Drag preview: render whatever the pointer is pointing at, elapsed text follows. */
            renderProgress(_dragFrac * (_currentTotal || 0), _currentTotal);
            _rafId = requestAnimationFrame(animationTick);
            return;
        }
        const now = performance.now();
        const elapsed = _currentPlaying
            ? _baseElapsed + (now - _baseTime) / 1000
            : _baseElapsed;
        renderProgress(elapsed, _currentTotal);
        _rafId = requestAnimationFrame(animationTick);
    }

    function ensureAnimating() {
        if (_rafId == null && !_currentIdle) {
            _rafId = requestAnimationFrame(animationTick);
        }
    }

    function applyState(raw) {
        const p = normalizePayload(raw);
        if (!p) {
            console.warn('[tray-popover] applyState skipped — not a tray state object', {
                type: raw === null ? 'null' : typeof raw,
                sample: typeof raw === 'string' ? raw.slice(0, 200) : raw,
            });
            return;
        }
        const themed = extractUiTheme(p);
        if (themed) applyTrayDocumentTheme(themed);
        applyTrayAppearanceFromPayload(extractAppearance(p), 'tray-popover-state');
        const idle = p.idle === true;
        if (shell) shell.classList.toggle('idle', idle);
        if (elTitle) elTitle.textContent = typeof p.title === 'string' ? p.title : '';
        _trayRevealPath =
            idle || typeof p.reveal_path !== 'string' || !p.reveal_path.trim() ? '' : p.reveal_path.trim();
        const popSub = typeof p.subtitle === 'string' ? p.subtitle : '';
        const subtitleSig = `${popSub}\0${_trayRevealPath}`;
        const subtitleDirty = _traySubtitleSig !== subtitleSig;
        if (subtitleDirty) {
            _traySubtitleSig = subtitleSig;
            _trayPathUserExpanded = false;
            if (elSub) {
                renderTraySubtitle(popSub, _trayRevealPath);
                applyTrayPathPanelDom(_trayPathUserExpanded);
            }
        }
        if (elIdle) {
            elIdle.hidden = !idle;
            let idleLabel = '';
            if (idle) {
                if (typeof p.idle_hint === 'string' && p.idle_hint.trim() !== '') {
                    idleLabel = p.idle_hint;
                } else {
                    idleLabel = window.appFmt('tray.popover_idle');
                }
            }
            elIdle.textContent = idleLabel;
        }
        const elapsed = typeof p.elapsed_sec === 'number' && Number.isFinite(p.elapsed_sec) ? p.elapsed_sec : 0;
        const total = typeof p.total_sec === 'number' && Number.isFinite(p.total_sec) && p.total_sec > 0 ? p.total_sec : null;
        const playing = p.playing === true;
        const durKey = total != null ? Math.round(total * 1000) : -1;
        const elapsedMs = Math.round(Math.max(0, elapsed) * 1000);
        const progressKey = `${idle ? 1 : 0}|${playing ? 1 : 0}|${durKey}|${elapsedMs}`;
        const progressPayloadUnchanged =
            _trayLastApplyProgressKey !== null && progressKey === _trayLastApplyProgressKey;
        _trayLastApplyProgressKey = progressKey;
        /* Re-base the animation model ONLY on discontinuities: idle toggle, play/pause, total
         * change, or a large elapsed jump (user seek / track change). Routine 500 ms polls AND
         * sibling pushes from volume / speed updates (see `setAudioVolume` →
         * `syncTrayNowPlayingFromPlayback`) must NOT re-base, otherwise the 60 fps local
         * interpolation snaps back to the slightly stale host value on every volume input event
         * and the progress thumb visibly yanks backward while the user is dragging the tray volume
         * slider. Keep the smooth interpolation when host elapsed and interpolated elapsed agree. */
        const nowMs = performance.now();
        const interpolated = _currentIdle
            ? elapsed
            : _currentPlaying
                ? _baseElapsed + (nowMs - _baseTime) / 1000
                : _baseElapsed;
        const drift = Math.abs(elapsed - interpolated);
        /* While the user is actively dragging the volume slider, suppress drift-based re-base
         * entirely. Engine DSP IPC flooding during drag can stall the `start_tray_host_poll`
         * `playback_status` response (shared audio-engine stdin/stdout mutex with the per-tick
         * `playback_set_dsp` commands), so the polled `position_sec` comes back stale by several
         * hundred ms. Without this guard the stale position breaches the 0.75 s drift threshold
         * and re-base yanks the progress thumb backward — exactly the symptom the user reports.
         * Track changes (`total` change), play/pause, and idle toggles still bypass the guard.
         * When the main window is hidden, `elapsed_sec` in the tray payload can lag behind local
         * interpolation (no fresh JS sync; HTML5 playback skips the engine poll). Shuffle/loop
         * toggles re-emit tray state with that stale elapsed — drift looks like a discontinuity.
         * If the progress tuple matches the previous `applyState`, treat the push as transport-only
         * metadata and keep interpolating. */
        const discontinuity =
            idle !== _currentIdle ||
            playing !== _currentPlaying ||
            total !== _currentTotal ||
            (!progressPayloadUnchanged && !_trayVolUserActive && drift > 0.75);
        if (discontinuity) {
            _baseElapsed = elapsed;
            _baseTime = nowMs;
        }
        _currentTotal = total;
        _currentPlaying = playing;
        _currentIdle = idle;
        if (elTotal) elTotal.textContent = total != null ? fmt(total) : '—';
        if (!_dragging && discontinuity) renderProgress(elapsed, total);
        if (idle) {
            if (_rafId != null) {
                cancelAnimationFrame(_rafId);
                _rafId = null;
            }
        } else {
            ensureAnimating();
        }
        if (btnPlay) btnPlay.textContent = playing ? '⏸' : '▶';
        if (btnPlay) {
            const playT = playing
                ? appFmtResolved('menu.pause')
                : appFmtResolved('menu.play');
            if (playT) btnPlay.setAttribute('title', playT);
        }
        const shuf = p.shuffle_on === true;
        const loopOn = p.loop_on === true;
        if (btnShuffle) btnShuffle.classList.toggle('active', shuf);
        if (btnLoop) btnLoop.classList.toggle('active', loopOn);
        applyTrayExtrasFromState(p.volume_pct, p.playback_speed);
        logTrayPopoverApplyState(p, idle, playing, themed);
        syncTrayPopoverTooltips();
        scheduleResize();
        setTimeout(() => {
            syncWindowSize();
        }, 0);
        setTimeout(() => {
            syncWindowSize();
        }, 80);
        setTimeout(() => {
            syncWindowSize();
        }, 260);
    }

    /* Drag-to-seek: window-level pointer listeners (no `setPointerCapture`). Capture retargeting on
     * macOS WebKit made `clientY`/rect checks unreliable vs the real node under the cursor; volume
     * drags still updated seek. `elementFromPoint` + `closest` matches the interactive stack. */
    function pointerFraction(e) {
        if (!elTrackBar) return 0;
        const rect = elTrackBar.getBoundingClientRect();
        if (rect.width <= 0) return 0;
        const x = e.clientX - rect.left;
        return Math.max(0, Math.min(1, x / rect.width));
    }

    function sendSeek(frac) {
        if (!invoke) return;
        void invoke('tray_popover_action', {
            action: `seek:${frac.toFixed(4)}`,
        }).catch(() => {});
    }

    const TRAY_SCRUB_BLOCK_SEEK =
        '#trayExtras, #trayVol, #traySpeed, .times, .transport, .subtitle, .title, .idle-hint, #subtitle, #trayPopoverTitle, #idleHint';

    function trayScrubPointerBlocksSeek(clientX, clientY) {
        const n = document.elementFromPoint(clientX, clientY);
        if (!n) return false;
        return n.closest(TRAY_SCRUB_BLOCK_SEEK) != null;
    }

    /** Narrower than move-blocking: release over `.times` / title must still commit seek; only
     * transport + extras were falsely taking stray scrub commits from quick cross-drags. */
    function trayScrubReleaseShouldCancelSeek(clientX, clientY) {
        const n = document.elementFromPoint(clientX, clientY);
        if (!n) return false;
        return n.closest('#trayExtras, #trayVol, #traySpeed, .transport') != null;
    }

    /** `elementFromPoint` can mis-hit when the main app window is hidden / non-key; `e.target` is
     * the real pointer-up target (e.g. shuffle) from the hit test. */
    function trayScrubReleaseTargetBlocksSeek(target) {
        if (!target || typeof target.closest !== 'function') return false;
        return target.closest('#trayExtras, #trayVol, #traySpeed, .transport') != null;
    }

    function removeTrayScrubWindowListeners() {
        window.removeEventListener('pointermove', trayScrubWindowMove, true);
        window.removeEventListener('pointerup', trayScrubWindowUp, true);
        window.removeEventListener('pointercancel', trayScrubWindowUp, true);
    }

    function trayScrubWindowMove(e) {
        if (!_dragging || e.pointerId !== _trayScrubPointerId) return;
        if (_trayScrubCancelled) return;
        if (trayScrubPointerBlocksSeek(e.clientX, e.clientY)) {
            _trayScrubCancelled = true;
            return;
        }
        _dragFrac = pointerFraction(e);
    }

    function trayScrubWindowUp(e) {
        if (!_dragging || e.pointerId !== _trayScrubPointerId) return;
        removeTrayScrubWindowListeners();
        /* If `pointermove` never ran before release (quick drag to transport), we would still seek
         * at the scrub start fraction — looks like shuffle/loop moved the playback bar. */
        let cancelled = _trayScrubCancelled;
        if (!cancelled &&
            (trayScrubReleaseTargetBlocksSeek(e.target) ||
                trayScrubReleaseShouldCancelSeek(e.clientX, e.clientY))) {
            cancelled = true;
        }
        _dragging = false;
        _trayScrubPointerId = null;
        _trayScrubCancelled = false;
        if (!cancelled) {
            sendSeek(_dragFrac);
            /* Optimistically re-base to the dragged position so the thumb does not snap back before
             * the next host push arrives (engine seek + playback_status poll can be > 250 ms). */
            if (_currentTotal != null) {
                _baseElapsed = _dragFrac * _currentTotal;
                _baseTime = performance.now();
                renderProgress(_baseElapsed, _currentTotal);
            }
        }
        ensureAnimating();
    }

    /** Abort an in-flight scrub (second pointer on extras, etc.) — no seek, drop window listeners. */
    function cancelTrackScrubWithoutSeek() {
        if (!_dragging) return;
        removeTrayScrubWindowListeners();
        _dragging = false;
        _trayScrubPointerId = null;
        _trayScrubCancelled = false;
        ensureAnimating();
    }

    if (elTrackBar) {
        elTrackBar.addEventListener('pointerdown', (e) => {
            if (_currentIdle) return;
            if (e.button !== 0 && e.pointerType === 'mouse') return;
            e.preventDefault();
            _trayScrubCancelled = false;
            _trayScrubPointerId = e.pointerId;
            _dragging = true;
            _dragFrac = pointerFraction(e);
            window.addEventListener('pointermove', trayScrubWindowMove, true);
            window.addEventListener('pointerup', trayScrubWindowUp, true);
            window.addEventListener('pointercancel', trayScrubWindowUp, true);
            ensureAnimating();
        });
    }

    function send(action) {
        if (!invoke) return;
        void invoke('tray_popover_action', { action }).catch(() => {});
    }

    if (btnShuffle) btnShuffle.addEventListener('click', () => send('toggle_shuffle'));
    if (btnPrev) btnPrev.addEventListener('click', () => send('prev_track'));
    if (btnPlay) btnPlay.addEventListener('click', () => send('play_pause'));
    if (btnNext) btnNext.addEventListener('click', () => send('next_track'));
    if (btnLoop) btnLoop.addEventListener('click', () => send('toggle_loop'));

    /* Force key window on click inside `#shell`. On macOS, `NonactivatingPanel`-style popovers
     * can receive mouse without becoming key; `setFocus` makes `onFocusChanged(false)` fire when
     * the user clicks outside (deferred blur dismiss in Rust + JS). */
    if (shell) {
        shell.addEventListener(
            'pointerdown',
            () => {
                const tw = getTrayWebviewWindow();
                if (tw && typeof tw.setFocus === 'function') {
                    void tw.setFocus().catch(() => {});
                }
            },
            true
        );
    }

    /* Dismiss-on-outside-click: the popover window rect is larger than the visible `.shell`
     * frame (TRAY_WIN_PAD_W/H padding + first-render auto-height lag before `syncWindowSize`
     * catches up), so clicks on the transparent padding that looks like "outside the popover"
     * were being swallowed by the window with no handler and felt like frozen-input. Standard
     * popover behavior is to dismiss on outside clicks — match that by hiding the webview when
     * a pointerdown lands outside `#shell` and outside the tray context menu (which is a sibling
     * of `#shell`, not a descendant). Capture phase so it runs before the shell-internal
     * handlers and can't be stopped by them. */
    document.addEventListener(
        'pointerdown',
        (e) => {
            const t = e.target;
            if (!t || typeof t.closest !== 'function') return;
            if (t.closest('#shell')) return;
            if (t.closest('#trayCtxMenu')) return;
            const tw = getTrayWebviewWindow();
            if (tw && typeof tw.hide === 'function') {
                void tw.hide().catch(() => {});
            }
        },
        true
    );

    if (elTrayExtras) {
        elTrayExtras.addEventListener(
            'pointerdown',
            (e) => {
                if (e.button !== 0 && e.pointerType === 'mouse') return;
                cancelTrackScrubWithoutSeek();
            },
            true
        );
    }
    if (elTransport) {
        elTransport.addEventListener(
            'pointerdown',
            (e) => {
                if (e.button !== 0 && e.pointerType === 'mouse') return;
                cancelTrackScrubWithoutSeek();
            },
            true
        );
    }
    if (elTrayVol) {
        elTrayVol.addEventListener('input', () => {
            if (_trayApplyingHostControls) return;
            _trayVolUserActive = true;
            if (_trayVolUserTimer != null) clearTimeout(_trayVolUserTimer);
            _trayVolUserTimer = setTimeout(() => {
                _trayVolUserTimer = null;
                _trayVolUserActive = false;
            }, TRAY_VOL_USER_SETTLE_MS);
            const v = parseInt(elTrayVol.value, 10);
            const n = Number.isFinite(v) ? Math.max(0, Math.min(100, v)) : 100;
            if (elTrayVolPct) elTrayVolPct.textContent = `${n}%`;
            void invoke('tray_popover_action', { action: `volume:${n}` }).catch(() => {});
        });
    }
    if (elTraySpeed) {
        elTraySpeed.addEventListener('change', () => {
            if (_trayApplyingHostControls) return;
            const sp = parseFloat(elTraySpeed.value);
            if (!Number.isFinite(sp)) return;
            void invoke('tray_popover_action', { action: `speed:${sp}` }).catch(() => {});
        });
    }

    /** `WebviewWindow.listen` / `event.listen` return Promises — await so emits are not dropped on first open. */
    async function initTrayIpc() {
        await _trayI18nReady;
        const prevT = appFmtResolved('tray.previous_track');
        const nextT = appFmtResolved('tray.next_track');
        if (btnPrev && prevT) btnPrev.setAttribute('title', prevT);
        if (btnNext && nextT) btnNext.setAttribute('title', nextT);
        const playPauseT = appFmtResolved('tray.play_pause');
        if (btnPlay && playPauseT) btnPlay.setAttribute('title', playPauseT);
        const shuffleTt = appFmtResolved('menu.toggle_shuffle', 'ui.tt.shuffle');
        if (btnShuffle && shuffleTt) btnShuffle.setAttribute('title', shuffleTt);
        const loopTt = appFmtResolved('menu.toggle_loop', 'ui.tt.toggle_loop_l');
        if (btnLoop && loopTt) btnLoop.setAttribute('title', loopTt);
        populateTraySpeedSelect();
        if (elTrayVolLabel) {
            const vLabel = appFmtResolved('ui.ae.playback_volume_label');
            elTrayVolLabel.textContent = vLabel || 'Vol';
        }
        if (elTraySpeedLabel) {
            const sLabel = appFmtResolved('ui.np.label_speed');
            elTraySpeedLabel.textContent = sLabel || 'Speed';
        }
        const volTt = appFmtResolved('ui.tt.volume_cmd_up_down');
        if (elTrayVol && volTt) elTrayVol.setAttribute('title', volTt);
        const speedTt = appFmtResolved('ui.tt.playback_speed', 'ui.np.label_speed');
        if (elTraySpeed && speedTt) {
            elTraySpeed.setAttribute('title', speedTt);
            elTraySpeed.setAttribute('aria-label', speedTt);
        }
        syncTrayPopoverTooltips();
        refreshTrayPathToggleI18n();

        const tw0 = getTrayWebviewWindow();
        console.info('[tray-popover] boot', {
            href: typeof location !== 'undefined' ? location.href : '',
            has_global_listen: !!listen,
            has_invoke: !!invoke,
            webview_label: tw0 && typeof tw0.label === 'string' ? tw0.label : undefined,
            webview_has_listen: !!(tw0 && typeof tw0.listen === 'function'),
        });
        const onState = (e) => {
            const raw = trayListenUnwrap(e);
            const top = e && typeof e === 'object' && !Array.isArray(e) ? e : null;
            console.info('[tray-popover] tray-popover-state ← host', {
                event_is_wrapper: !!(top && 'payload' in top && top.payload != null),
                event_name: top && typeof top.event === 'string' ? top.event : undefined,
                state_keys: raw && typeof raw === 'object' ? Object.keys(raw) : [],
                appearance_keys:
                    raw && raw.appearance && typeof raw.appearance === 'object'
                        ? Object.keys(raw.appearance).filter((k) => k.startsWith('--')).length
                        : 0,
            });
            applyState(raw);
        };
        const onTheme = (e) => {
            const raw = trayListenUnwrap(e);
            const th =
                extractUiTheme(raw) ||
                (raw && typeof raw === 'object' && typeof raw.theme === 'string' ? raw.theme : null);
            console.info('[tray-popover] tray-popover-ui-theme ← host', {
                event_is_wrapper: !!(e && typeof e === 'object' && 'payload' in e),
                raw,
                ui_theme: th || '(none)',
            });
            if (th) applyTrayDocumentTheme(th);
        };
        const scoped = { target: 'tray-popover' };
        try {
            const tw = getTrayWebviewWindow();
            if (tw && typeof tw.listen === 'function') {
                await tw.listen('tray-popover-state', onState);
                await tw.listen('tray-popover-ui-theme', onTheme);
                console.info('[tray-popover] IPC listeners registered (WebviewWindow.listen)', {
                    label: typeof tw.label === 'string' ? tw.label : '(unknown)',
                });
            } else if (listen) {
                try {
                    await listen('tray-popover-state', onState, scoped);
                    await listen('tray-popover-ui-theme', onTheme, scoped);
                    console.info('[tray-popover] IPC listeners registered (event.listen + target)', scoped);
                } catch (_) {
                    /* Older/global bundles may omit the `target` option. */
                    await listen('tray-popover-state', onState);
                    await listen('tray-popover-ui-theme', onTheme);
                    console.info('[tray-popover] IPC listeners registered (event.listen, no target)');
                }
            } else {
                console.warn('[tray-popover] no listen API — tray-popover-state events will not apply');
            }
        } catch (err) {
            console.warn('[tray-popover] IPC listen failed', err);
        }

        if (invoke) {
            try {
                const [theme, emit] = await Promise.all([
                    invoke('tray_popover_get_ui_theme').catch(() => 'dark'),
                    invoke('tray_popover_get_state').catch(() => null),
                ]);
                const bootState = emit ? trayListenUnwrap(emit) : null;
                console.info('[tray-popover] bootstrap invoke', {
                    tray_popover_get_ui_theme: theme,
                    tray_popover_get_state: bootState ? Object.keys(bootState) : null,
                });
                applyTrayDocumentTheme(typeof theme === 'string' ? theme : 'dark');
                if (bootState) applyState(bootState);
                document.title = 'AUDIO_HAXOR';
            } catch (err) {
                console.warn('[tray-popover] bootstrap invoke failed', err);
            }
        }
    }

    void initTrayIpc();

    /* Click-outside-window dismiss: hide when this webview loses focus. */
    (async function installClickOutsideDismiss() {
        const tw = getTrayWebviewWindow();
        if (!tw || typeof tw.onFocusChanged !== 'function') return;
        try {
            await tw.onFocusChanged((evt) => {
                const focused = evt && evt.payload !== undefined ? evt.payload : evt;
                if (focused === true) return;
                void (async () => {
                    try {
                        const vis = typeof tw.isVisible === 'function' ? await tw.isVisible() : true;
                        if (vis && typeof tw.hide === 'function') await tw.hide();
                    } catch (_) {
                        /* ignore */
                    }
                })();
            });
        } catch (_) {
            /* non-Tauri or older API */
        }
    })();

    function initSizeAfterFonts() {
        const run = () => scheduleResize();
        if (typeof document !== 'undefined' && document.fonts && typeof document.fonts.ready !== 'undefined') {
            void document.fonts.ready.then(run).catch(run);
        } else {
            run();
        }
    }
    if (typeof ResizeObserver === 'function' && shell) {
        const ro = new ResizeObserver(() => {
            scheduleResize();
        });
        ro.observe(shell);
    }

    if (document.readyState === 'complete') {
        initSizeAfterFonts();
    } else {
        window.addEventListener('load', () => initSizeAfterFonts(), { once: true });
    }

    document.addEventListener('keydown', (e) => {
        if (e.key !== 'Escape') return;
        if (trayCtx && trayCtx.classList.contains('visible')) {
            hideTrayCtxMenu();
            e.preventDefault();
            return;
        }
        const tw = getTrayWebviewWindow();
        if (tw && typeof tw.hide === 'function') void tw.hide().catch(() => {});
    });
})();
