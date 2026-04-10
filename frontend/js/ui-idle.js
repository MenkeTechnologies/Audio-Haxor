/**
 * Hybrid “heavy UI idle” detection: Page Visibility (`document.hidden`), **`document.hasFocus()`**
 * (desktop WebViews often keep `document.hidden` false when another app is foreground), **`window`**
 * **blur/focus**, plus Tauri `WebviewWindow` focus / minimize / visible when available.
 *
 * **macOS Spaces:** switching to another Space does not reliably fire `visibilitychange`, `blur`, or
 * even `onFocusChanged` for the WebView, so we **poll** `isFocused` / `isVisible` / `isMinimized` on a
 * timer and on **`onMoved`** (cheap vs leaving rAF + `playback_status` loops running off-screen).
 *
 * Paused work is rAF-driven visualization and idle-gated **`playback_status`** polling — background
 * BPM/Key/LUFS batch analysis runs only when **`autoAnalysis`** is **`on`** (`audio.js`).
 * **`document.documentElement`** gets **`ui-idle-heavy-cpu`** so **`index.html`** can pause infinite CSS
 * animations (scanlines, spinners) while idle — same signal as **`isUiIdleHeavyCpu()`**.
 */
(function initUiIdleHeavyCpu() {
    let docHidden = typeof document !== 'undefined' && document.hidden;
    let winFocused = true;
    let winMinimized = false;
    let winVisible = true;

    function recompute() {
        const noDocFocus =
            typeof document !== 'undefined' &&
            typeof document.hasFocus === 'function' &&
            !document.hasFocus();
        return docHidden || !winFocused || winMinimized || !winVisible || noDocFocus;
    }

    let idle = recompute();
    if (typeof document !== 'undefined' && document.documentElement) {
        document.documentElement.classList.toggle('ui-idle-heavy-cpu', idle);
    }

    function setState() {
        const next = recompute();
        if (next === idle) return;
        idle = next;
        try {
            if (typeof document !== 'undefined' && document.documentElement) {
                document.documentElement.classList.toggle('ui-idle-heavy-cpu', idle);
            }
            document.dispatchEvent(new CustomEvent('ui-idle-heavy-cpu', {detail: {idle}}));
        } catch (_) {
            /* ignore */
        }
    }

    /**
     * @returns {boolean} true when rAF-heavy UI (spectrum, visualizer, playhead) should throttle
     */
    window.isUiIdleHeavyCpu = function isUiIdleHeavyCpu() {
        return idle;
    };

    if (typeof document !== 'undefined') {
        document.addEventListener('visibilitychange', () => {
            docHidden = document.hidden;
            setState();
        });
    }

    if (typeof window !== 'undefined') {
        window.addEventListener('blur', () => setState());
        window.addEventListener('focus', () => setState());
    }

    function syncFromTauriWindow(win) {
        if (!win) return Promise.resolve();
        const ps = [];
        if (typeof win.isFocused === 'function') {
            ps.push(win.isFocused().then((v) => {
                winFocused = !!v;
            }));
        }
        if (typeof win.isMinimized === 'function') {
            ps.push(win.isMinimized().then((v) => {
                winMinimized = !!v;
            }));
        }
        if (typeof win.isVisible === 'function') {
            ps.push(win.isVisible().then((v) => {
                winVisible = !!v;
            }));
        }
        if (ps.length === 0) return Promise.resolve();
        return Promise.all(ps);
    }

    (async function setupTauri() {
        try {
            const TW = window.__TAURI__ && window.__TAURI__.webviewWindow;
            if (!TW || typeof TW.getCurrentWebviewWindow !== 'function') return;
            const win = TW.getCurrentWebviewWindow();
            try {
                await syncFromTauriWindow(win);
                setState();
            } catch (_) {
                setState();
            }

            if (typeof win.onFocusChanged === 'function') {
                await win.onFocusChanged((evt) => {
                    const p = evt && (evt.payload !== undefined ? evt.payload : evt);
                    winFocused = p === true;
                    setState();
                });
            }
            if (typeof win.onResized === 'function') {
                await win.onResized(() => {
                    void syncFromTauriWindow(win).then(() => setState()).catch(() => setState());
                });
            }
            if (typeof win.onMoved === 'function') {
                await win.onMoved(() => {
                    void syncFromTauriWindow(win).then(() => setState()).catch(() => setState());
                });
            }
            /* Spaces / occlusion: events are unreliable; re-sync native window state periodically. */
            setInterval(() => {
                void syncFromTauriWindow(win).then(() => setState()).catch(() => setState());
            }, 1200);
        } catch (_) {
            /* non-Tauri or older API */
        }
    })();
})();
