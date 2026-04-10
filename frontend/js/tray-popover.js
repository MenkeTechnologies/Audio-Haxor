/**
 * Menu-bar tray popover (WebView window `tray-popover`): Apple-style layout without artwork.
 * Window size is synced to `#shell` via `tray_popover_resize` (logical/CSS px — matches HiDPI layout).
 */
(function () {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    const listen = tauri && tauri.event && typeof tauri.event.listen === 'function' ? tauri.event.listen : null;
    const invoke =
        tauri && tauri.core && typeof tauri.core.invoke === 'function' ? tauri.core.invoke : null;
    const TW = tauri && tauri.webviewWindow && typeof tauri.webviewWindow.getCurrentWebviewWindow === 'function'
        ? tauri.webviewWindow.getCurrentWebviewWindow()
        : null;
    /* Do not hide on blur: focus moves to the tray before the tray Click event, which would fight Rust toggle. */

    const shell = document.getElementById('shell');
    const elTitle = document.getElementById('title');
    const elSub = document.getElementById('subtitle');
    const elIdle = document.getElementById('idleHint');
    const elFill = document.getElementById('fill');
    const elElapsed = document.getElementById('elapsed');
    const elTotal = document.getElementById('total');
    const btnPrev = document.getElementById('btnPrev');
    const btnPlay = document.getElementById('btnPlay');
    const btnNext = document.getElementById('btnNext');

    function fmt(sec) {
        if (typeof sec !== 'number' || !Number.isFinite(sec) || sec < 0) return '0:00';
        const m = Math.floor(sec / 60);
        const s = Math.floor(sec % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
    }

    /** `.shell` box-shadow blur extends outside the border box; layout metrics do not include it. */
    const SHADOW_PAD_X = 22;
    const SHADOW_PAD_Y = 32;
    const LAYOUT_PAD = 8;

    function syncWindowSize() {
        if (!invoke) return;
        const root = document.getElementById('shell');
        if (!root) return;
        const br = root.getBoundingClientRect();
        const innerH = Math.max(root.scrollHeight, root.offsetHeight, br.height);
        const innerW = Math.max(root.scrollWidth, root.offsetWidth, br.width);
        const h = Math.ceil(innerH + LAYOUT_PAD + SHADOW_PAD_Y);
        const w = Math.ceil(innerW + LAYOUT_PAD + SHADOW_PAD_X);
        void invoke('tray_popover_resize', { width: w, height: h }).catch(() => {});
    }

    function scheduleResize() {
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                syncWindowSize();
            });
        });
    }

    function applyState(p) {
        if (!p || typeof p !== 'object') return;
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
        let pct = 0;
        if (total != null && total > 0) pct = Math.min(100, Math.max(0, (elapsed / total) * 100));
        if (elFill) elFill.style.width = `${pct}%`;
        if (elElapsed) elElapsed.textContent = fmt(elapsed);
        if (elTotal) elTotal.textContent = total != null ? fmt(total) : '—';
        const playing = p.playing === true;
        if (btnPlay) btnPlay.textContent = playing ? '⏸' : '▶';
        if (btnPlay) btnPlay.setAttribute('title', playing ? 'Pause' : 'Play');
        scheduleResize();
        setTimeout(() => {
            syncWindowSize();
        }, 0);
        setTimeout(() => {
            syncWindowSize();
        }, 80);
    }

    function send(action) {
        if (!invoke) return;
        void invoke('tray_popover_action', { action }).catch(() => {});
    }

    if (btnPrev) btnPrev.addEventListener('click', () => send('prev_track'));
    if (btnPlay) btnPlay.addEventListener('click', () => send('play_pause'));
    if (btnNext) btnNext.addEventListener('click', () => send('next_track'));

    if (listen) {
        void listen('tray-popover-state', (e) => {
            const p = e && e.payload !== undefined ? e.payload : e;
            applyState(p);
        }).catch(() => {});
    }

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
        if (e.key === 'Escape' && TW && typeof TW.hide === 'function') {
            void TW.hide().catch(() => {});
        }
    });
})();
