/**
 * Hybrid “heavy UI idle” detection: Page Visibility (`document.hidden`) plus Tauri
 * `WebviewWindow` focus / minimize / visible when available.
 *
 * Paused work is rAF-driven visualization only — background BPM/Key/LUFS analysis (`startBackgroundAnalysis`
 * in `audio.js`) is unrelated and keeps running.
 */
(function initUiIdleHeavyCpu() {
    let docHidden = typeof document !== 'undefined' && document.hidden;
    let winFocused = true;
    let winMinimized = false;
    let winVisible = true;

    function recompute() {
        return docHidden || !winFocused || winMinimized || !winVisible;
    }

    let idle = recompute();

    function setState() {
        const next = recompute();
        if (next === idle) return;
        idle = next;
        try {
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
        } catch (_) {
            /* non-Tauri or older API */
        }
    })();
})();
