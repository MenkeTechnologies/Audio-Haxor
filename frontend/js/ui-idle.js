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
    /* True once Tauri's `WebviewWindow` has been queried successfully (any of `isFocused`,
     * `isMinimized`, `isVisible` resolved).  Switches `recompute` to trust ONLY Tauri-reported
     * minimized/visible state, ignoring `document.hidden` — see comment on `recompute`. */
    let hasTauriWindowState = false;

    /* IDLE only on actual invisibility — hidden tab, minimized window, or `isVisible:false`.
     * Merely losing keyboard focus (e.g. the tray popover grabbing focus, the user clicking in
     * another app while the main window is still on screen) must NOT trigger idle, because the
     * main window's FFT / playhead / spectrum rAF loops and the engine `playback_status` poll
     * all stop behind this flag. The user reported that dragging the tray volume slider stopped
     * updating the main window's playhead + FFT until they clicked the main window to refocus
     * it — classic symptom of an idle-on-blur heuristic. Being visible-but-unfocused is NOT idle.
     * `winFocused` is still tracked for diagnostics but is not part of the idle condition.
     *
     * `document.hidden` is NOT load-bearing on macOS WKWebView: when the tray popover panel
     * is `alwaysOnTop` and overlaps the main window, WebKit's occlusion API flips
     * `visibilityState` to `hidden` even though the main window is still on screen and the
     * Tauri-side `isVisible` / `isMinimized` say otherwise.  Trusting `document.hidden` in
     * that case freezes the main window the moment the user clicks the tray.  When Tauri's
     * window state is available, trust ONLY `winMinimized` / `winVisible` — those reflect
     * native window-on-screen reality, not WebKit occlusion. */
    function recompute() {
        if (hasTauriWindowState) return winMinimized || !winVisible;
        return docHidden || winMinimized || !winVisible;
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
            /* Going idle (BG): dispatch SYNCHRONOUSLY so rAF / setInterval / FFT loops
             * stop the moment the WebView is hidden — every frame of work past this point
             * is wasted CPU under macOS suspension and risks WindowServer queue backup.
             *
             * Coming OUT of idle (FG resume): defer the dispatch by one animation frame
             * so the user's queued click / keypress events are processed by the JS event
             * loop *before* the 14 listeners run their burst of polling-restart, FFT-rAF,
             * playback-status setInterval rewires, etc. Without this, the first click
             * after focus return can appear to "not register" — the click event lands in
             * the queue behind the synchronous listener burst, and the row click handler
             * can't fetch waveform / spectrogram until tens of ms of foreground sync work
             * completes. CSS-side animations (scanlines, spinners) still resume the same
             * frame because the `ui-idle-heavy-cpu` class on `documentElement` was toggled
             * synchronously above. */
            const evt = new CustomEvent('ui-idle-heavy-cpu', {detail: {idle}});
            if (idle) {
                document.dispatchEvent(evt);
            } else if (typeof requestAnimationFrame === 'function') {
                requestAnimationFrame(() => {
                    try { document.dispatchEvent(evt); } catch (_) { /* ignore */ }
                });
            } else {
                setTimeout(() => {
                    try { document.dispatchEvent(evt); } catch (_) { /* ignore */ }
                }, 0);
            }
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
        return Promise.all(ps).then(() => {
            hasTauriWindowState = true;
        });
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
