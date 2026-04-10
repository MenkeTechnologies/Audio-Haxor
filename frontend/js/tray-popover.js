/**
 * Menu-bar tray popover (WebView window `tray-popover`): Apple-style layout without artwork.
 * Window size is synced to `#shell` via `tray_popover_resize` (logical/CSS px — matches HiDPI layout).
 */
(function () {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    const listen = tauri && tauri.event && typeof tauri.event.listen === 'function' ? tauri.event.listen : null;
    const invoke =
        tauri && tauri.core && typeof tauri.core.invoke === 'function' ? tauri.core.invoke : null;
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
        return a && typeof a === 'object' ? a : null;
    }

    /** Main-window scheme vars (`--cyan`, …) → popover `document.documentElement` (feeds `--cp-*` aliases in CSS). */
    function applyTrayAppearanceFromPayload(map) {
        if (!map || typeof map !== 'object') return;
        const root = document.documentElement.style;
        for (const [k, v] of Object.entries(map)) {
            if (typeof k === 'string' && k.startsWith('--') && typeof v === 'string' && v.length > 0) {
                root.setProperty(k, v);
            }
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
    const btnPrev = document.getElementById('btnPrev');
    const btnPlay = document.getElementById('btnPlay');
    const btnNext = document.getElementById('btnNext');

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
        console.info('[tray-popover] applyState', {
            idle,
            playing,
            ui_theme: ui,
            titleLen: title.length,
        });
    }

    function fmt(sec) {
        if (typeof sec !== 'number' || !Number.isFinite(sec) || sec < 0) return '0:00';
        const m = Math.floor(sec / 60);
        const s = Math.floor(sec % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
    }

    /** Size the window to the exact `.shell` rect — no outer glow / shadow to pad for. */
    function syncWindowSize() {
        if (!invoke) return;
        const root = document.getElementById('shell');
        if (!root) return;
        const br = root.getBoundingClientRect();
        const h = Math.ceil(Math.max(root.scrollHeight, root.offsetHeight, br.height));
        const w = Math.ceil(Math.max(root.scrollWidth, root.offsetWidth, br.width));
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
        if (!raw || typeof raw !== 'object') return null;
        const p = { ...raw };
        let elapsed = p.elapsed_sec;
        if (typeof elapsed === 'string') elapsed = parseFloat(elapsed);
        if (typeof elapsed !== 'number' || !Number.isFinite(elapsed)) elapsed = 0;
        let total = p.total_sec;
        if (typeof total === 'string') total = parseFloat(total);
        if (typeof total === 'number' && Number.isFinite(total) && total > 0) {
            p.total_sec = total;
        } else {
            p.total_sec = null;
        }
        p.elapsed_sec = elapsed;
        return p;
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
    let _dragFrac = 0;
    let _rafId = null;

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
        if (!p) return;
        const themed = extractUiTheme(p);
        if (themed) applyTrayDocumentTheme(themed);
        applyTrayAppearanceFromPayload(extractAppearance(p));
        const idle = p.idle === true;
        if (shell) shell.classList.toggle('idle', idle);
        if (elTitle) elTitle.textContent = typeof p.title === 'string' ? p.title : '';
        if (elSub) elSub.textContent = typeof p.subtitle === 'string' ? p.subtitle : '';
        if (elIdle) {
            elIdle.hidden = !idle;
            elIdle.textContent =
                idle && typeof p.idle_hint === 'string' && p.idle_hint.trim() !== ''
                    ? p.idle_hint
                    : idle
                      ? 'Nothing playing'
                      : '';
        }
        const elapsed = typeof p.elapsed_sec === 'number' && Number.isFinite(p.elapsed_sec) ? p.elapsed_sec : 0;
        const total = typeof p.total_sec === 'number' && Number.isFinite(p.total_sec) && p.total_sec > 0 ? p.total_sec : null;
        const playing = p.playing === true;
        /* Re-base the animation model from the host-reported values. `performance.now()` is the
         * zero-point for interpolation until the next push. */
        _baseElapsed = elapsed;
        _baseTime = performance.now();
        _currentTotal = total;
        _currentPlaying = playing;
        _currentIdle = idle;
        if (elTotal) elTotal.textContent = total != null ? fmt(total) : '—';
        if (!_dragging) renderProgress(elapsed, total);
        if (idle) {
            if (_rafId != null) {
                cancelAnimationFrame(_rafId);
                _rafId = null;
            }
        } else {
            ensureAnimating();
        }
        if (btnPlay) btnPlay.textContent = playing ? '⏸' : '▶';
        if (btnPlay) btnPlay.setAttribute('title', playing ? 'Pause' : 'Play');
        logTrayPopoverApplyState(p, idle, playing, themed);
        scheduleResize();
        setTimeout(() => {
            syncWindowSize();
        }, 0);
        setTimeout(() => {
            syncWindowSize();
        }, 80);
    }

    /* Drag-to-seek on the scrubber. Uses pointer capture so the drag still tracks even when the
     * cursor leaves the track bar, and blocks updates from host pushes (`applyState` honors
     * `_dragging`) so the thumb does not jitter while the user is scrubbing. */
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

    if (elTrackBar) {
        elTrackBar.addEventListener('pointerdown', (e) => {
            if (_currentIdle) return;
            if (e.button !== 0 && e.pointerType === 'mouse') return;
            e.preventDefault();
            _dragging = true;
            _dragFrac = pointerFraction(e);
            try {
                elTrackBar.setPointerCapture(e.pointerId);
            } catch (_) {
                /* ignore */
            }
            ensureAnimating();
        });
        elTrackBar.addEventListener('pointermove', (e) => {
            if (!_dragging) return;
            _dragFrac = pointerFraction(e);
        });
        const endDrag = (e) => {
            if (!_dragging) return;
            _dragging = false;
            try {
                elTrackBar.releasePointerCapture(e.pointerId);
            } catch (_) {
                /* ignore */
            }
            sendSeek(_dragFrac);
            /* Optimistically re-base to the dragged position so the thumb does not snap back before
             * the next host push arrives (engine seek + playback_status poll can be > 250 ms). */
            if (_currentTotal != null) {
                _baseElapsed = _dragFrac * _currentTotal;
                _baseTime = performance.now();
                renderProgress(_baseElapsed, _currentTotal);
            }
            ensureAnimating();
        };
        elTrackBar.addEventListener('pointerup', endDrag);
        elTrackBar.addEventListener('pointercancel', endDrag);
    }

    function send(action) {
        if (!invoke) return;
        void invoke('tray_popover_action', { action }).catch(() => {});
    }

    if (btnPrev) btnPrev.addEventListener('click', () => send('prev_track'));
    if (btnPlay) btnPlay.addEventListener('click', () => send('play_pause'));
    if (btnNext) btnNext.addEventListener('click', () => send('next_track'));

    /** `WebviewWindow.listen` / `event.listen` return Promises — await so emits are not dropped on first open. */
    async function initTrayIpc() {
        const onState = (e) => {
            const raw = e && e.payload !== undefined ? e.payload : e;
            applyState(raw);
        };
        const onTheme = (e) => {
            const raw = e && e.payload !== undefined ? e.payload : e;
            console.info('[tray-popover] event tray-popover-ui-theme', raw);
            const th = extractUiTheme(raw) || (raw && typeof raw === 'object' && typeof raw.theme === 'string' ? raw.theme : null);
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
                console.info('[tray-popover] bootstrap invoke', {
                    tray_popover_get_ui_theme: theme,
                    tray_popover_get_state: emit ? 'some' : 'null',
                });
                applyTrayDocumentTheme(typeof theme === 'string' ? theme : 'dark');
                if (emit) applyState(emit);
            } catch (err) {
                console.warn('[tray-popover] bootstrap invoke failed', err);
            }
        }
    }

    void initTrayIpc();

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
        const tw = getTrayWebviewWindow();
        if (tw && typeof tw.hide === 'function') void tw.hide().catch(() => {});
    });
})();
