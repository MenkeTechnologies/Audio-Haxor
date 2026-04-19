// ── Embedded Terminal (PTY-backed, xterm.js) ──
// Fixed-position pane with dock-to-corner drag, geometry persistence, and
// visibility saved to prefs — mirrors the audio player popup behavior.

let _termInstance = null;
let _termUnlistenOutput = null;
let _termUnlistenExit = null;
let _termFitDebounce = null;
let _termSessionAlive = false;

const TERM_DOCK_CLASSES = ['dock-tl', 'dock-tr', 'dock-bl', 'dock-br'];

// ── Public API ──

function toggleTerminalPopup() {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    if (pane.classList.contains('active')) {
        hideTerminal();
    } else {
        showTerminal();
    }
}

function showTerminal() {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    pane.classList.add('active');
    prefs.setItem('terminalPaneHidden', 'off');

    // Restore saved dimensions
    _termRestoreDimensions();

    // Spawn PTY session if needed
    if (!_termSessionAlive) {
        _termSpawnSession();
    } else if (_termInstance) {
        _termInstance.focus();
        _termSendResize();
    }
}

function hideTerminal() {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    pane.classList.remove('active');
    prefs.setItem('terminalPaneHidden', 'on');
}

// ── Dock system (mirrors audio player) ──

function _termGetCurrentDock() {
    const pane = document.getElementById('terminalPane');
    if (!pane) return 'dock-br';
    for (const c of TERM_DOCK_CLASSES) {
        if (pane.classList.contains(c)) return c;
    }
    return 'dock-br';
}

function _termSetDock(dock) {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    TERM_DOCK_CLASSES.forEach((c) => pane.classList.remove(c));
    pane.classList.add(dock);
    prefs.setItem('terminalDock', dock);
}

function _termNearestDock(x, y) {
    const midX = window.innerWidth / 2;
    const midY = window.innerHeight / 2;
    if (x < midX) return y < midY ? 'dock-tl' : 'dock-bl';
    return y < midY ? 'dock-tr' : 'dock-br';
}

function restoreTerminalDock() {
    const saved = prefs.getItem('terminalDock');
    const dock = saved && TERM_DOCK_CLASSES.includes(saved) ? saved : 'dock-br';
    const pane = document.getElementById('terminalPane');
    if (pane) {
        TERM_DOCK_CLASSES.forEach((c) => pane.classList.remove(c));
        pane.classList.add(dock);
    }
}

function restoreTerminalDimensions() {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    const saved = prefs.getItem('modal_terminalPane');
    if (!saved) return;
    try {
        const geo = JSON.parse(saved);
        if (geo.width >= 200) pane.style.width = geo.width + 'px';
        if (geo.height >= 150) pane.style.height = geo.height + 'px';
    } catch (_) { /* ignore */ }
}

function _termRestoreDimensions() {
    if (typeof restoreTerminalDimensions === 'function') restoreTerminalDimensions();
}

function restoreTerminalPaneVisibilityFromPrefs() {
    const hidden = prefs.getItem('terminalPaneHidden');
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    if (hidden === 'on') {
        pane.classList.remove('active');
    }
}

// ── Drag-to-dock ──

let _termDragState = null;

function _termOnDragStart(e) {
    const pane = document.getElementById('terminalPane');
    if (!pane) return;

    // Don't drag from buttons, input, or the xterm canvas/textarea
    if (e.target.closest('button, input, select, textarea, canvas, .xterm')) return;
    if (e.button !== 0) return;
    e.preventDefault();

    const rect = pane.getBoundingClientRect();
    TERM_DOCK_CLASSES.forEach((c) => pane.classList.remove(c));
    pane.classList.remove('snapping');
    pane.style.position = 'fixed';
    pane.style.left = rect.left + 'px';
    pane.style.top = rect.top + 'px';
    pane.style.right = 'auto';
    pane.style.bottom = 'auto';
    pane.classList.add('dragging');

    // Reuse the shared dock overlay (same as audio player) with pixel-based positioning
    // CSS calc() with percentages doesn't resolve in release WebView
    const overlay = document.getElementById('dockOverlay');
    if (overlay) {
        const vw = window.innerWidth, vh = window.innerHeight, gap = 4;
        const zw = Math.floor(vw / 2 - gap * 1.5) + 'px';
        const zh = Math.floor(vh / 2 - gap * 1.5) + 'px';
        const mid = Math.ceil(vw / 2 + gap / 2) + 'px';
        const midY = Math.ceil(vh / 2 + gap / 2) + 'px';
        const g = gap + 'px';
        const tl = document.getElementById('dockTL');
        const tr = document.getElementById('dockTR');
        const bl = document.getElementById('dockBL');
        const br = document.getElementById('dockBR');
        if (tl) tl.style.cssText = `top:${g};left:${g};width:${zw};height:${zh}`;
        if (tr) tr.style.cssText = `top:${g};left:${mid};width:${zw};height:${zh}`;
        if (bl) bl.style.cssText = `top:${midY};left:${g};width:${zw};height:${zh}`;
        if (br) br.style.cssText = `top:${midY};left:${mid};width:${zw};height:${zh}`;
        overlay.classList.add('visible');
    }

    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'grabbing';
    _termDragState = {startX: e.clientX, startY: e.clientY, origLeft: rect.left, origTop: rect.top};
}

document.addEventListener('mousemove', (e) => {
    if (!_termDragState) return;
    const pane = document.getElementById('terminalPane');
    if (!pane) return;
    const dx = e.clientX - _termDragState.startX;
    const dy = e.clientY - _termDragState.startY;
    pane.style.left = (_termDragState.origLeft + dx) + 'px';
    pane.style.top = (_termDragState.origTop + dy) + 'px';

    // Highlight nearest dock zone (shared overlay)
    const nearest = _termNearestDock(e.clientX, e.clientY);
    const zoneMap = {
        'dock-tl': 'dockTL', 'dock-tr': 'dockTR',
        'dock-bl': 'dockBL', 'dock-br': 'dockBR',
    };
    Object.entries(zoneMap).forEach(([dock, id]) => {
        const el = document.getElementById(id);
        if (el) el.classList.toggle('active', dock === nearest);
    });
});

document.addEventListener('mouseup', (e) => {
    if (!_termDragState) return;
    const pane = document.getElementById('terminalPane');
    _termDragState = null;
    document.body.style.userSelect = '';
    document.body.style.cursor = '';

    const overlay = document.getElementById('dockOverlay');
    if (overlay) {
        overlay.classList.remove('visible');
        ['dockTL', 'dockTR', 'dockBL', 'dockBR'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.classList.remove('active');
        });
    }

    if (!pane) return;
    pane.classList.remove('dragging');

    // Snap to nearest dock
    const dock = _termNearestDock(e.clientX, e.clientY);
    pane.style.left = '';
    pane.style.top = '';
    pane.style.right = '';
    pane.style.bottom = '';
    pane.classList.add('snapping');
    _termSetDock(dock);
    setTimeout(() => pane.classList.remove('snapping'), 300);

    // Save dimensions
    const rect = pane.getBoundingClientRect();
    prefs.setItem('modal_terminalPane', JSON.stringify({
        width: Math.round(rect.width),
        height: Math.round(rect.height),
    }));

    // Re-fit after dock
    clearTimeout(_termFitDebounce);
    _termFitDebounce = setTimeout(() => _termSendResize(), 60);
});

// ── PTY session management ──

async function _termSpawnSession() {
    const pane = document.getElementById('terminalPane');
    const container = document.getElementById('terminalContainer');
    if (!pane || !container) return;

    if (typeof Terminal !== 'function') {
        container.textContent = 'xterm.js not loaded';
        return;
    }

    // Create xterm.js instance
    const term = new Terminal({
        cursorBlink: true,
        cursorStyle: 'block',
        fontSize: 13,
        fontFamily: "'Hack Nerd Font', 'Hack Nerd Font Mono', 'Hack', 'Share Tech Mono', 'Menlo', monospace",
        theme: {
            background: 'rgba(0, 0, 0, 0)',
            foreground: '#e0e0e0',
            cursor: '#00e5ff',
            cursorAccent: '#0a0a12',
            selectionBackground: 'rgba(0,229,255,0.25)',
            black: '#1a1a2e',
            red: '#ff3860',
            green: '#23d160',
            yellow: '#ffdd57',
            blue: '#3273dc',
            magenta: '#b86bff',
            cyan: '#00e5ff',
            white: '#e0e0e0',
            brightBlack: '#4a4a6a',
            brightRed: '#ff6b8a',
            brightGreen: '#5dfc8a',
            brightYellow: '#ffe27a',
            brightBlue: '#5a9cff',
            brightMagenta: '#d19cff',
            brightCyan: '#4df0ff',
            brightWhite: '#ffffff',
        },
        allowProposedApi: true,
        allowTransparency: true,
        scrollback: 10000,
    });

    term.open(container);
    _termInstance = term;

    // Release WebKit ignores setAttribute("style", ...) on dynamically-created
    // elements under tauri://localhost. xterm.js DOM renderer uses setAttribute
    // for truecolor (24-bit RGB) via _addStyle(). Monkey-patch to use the DOM
    // .style API instead, which DOES work (proven by letter-spacing).
    // Release WebKit ignores setAttribute("style",...) on dynamically-created
    // elements. xterm.js uses setAttribute for truecolor via _addStyle().
    // Monkey-patch to use DOM .style API which works (proven by letter-spacing).
    // Walk the _core tree to find _rowFactory regardless of nesting depth.
    try {
        const core = term._core;
        let rf = null;
        // Try common paths — xterm.js v5 minified structure varies
        const candidates = [
            core._renderService?._rowFactory,
            core._renderService?._renderer?._rowFactory,
        ];
        // Deep scan: find any object with _addStyle method
        if (!candidates.some(c => c)) {
            const scan = (obj, depth) => {
                if (!obj || depth > 4 || rf) return;
                if (typeof obj._addStyle === 'function') { rf = obj; return; }
                for (const k of Object.keys(obj)) {
                    if (k.startsWith('_') && typeof obj[k] === 'object' && obj[k]) {
                        scan(obj[k], depth + 1);
                    }
                }
            };
            scan(core, 0);
        } else {
            rf = candidates.find(c => c);
        }
        if (rf && typeof rf._addStyle === 'function') {
            rf._addStyle = function (el, styleStr) {
                const colorMatch = styleStr.match(/^color:(#[0-9a-fA-F]{3,8})/);
                if (colorMatch) { el.style.color = colorMatch[1]; return; }
                const bgMatch = styleStr.match(/^background-color:(#[0-9a-fA-F]{3,8})/);
                if (bgMatch) { el.style.backgroundColor = bgMatch[1]; return; }
                const parts = styleStr.split(':');
                if (parts.length === 2) {
                    const prop = parts[0].trim().replace(/-([a-z])/g, (_, c) => c.toUpperCase());
                    el.style[prop] = parts[1].trim().replace(/;$/, '');
                }
            };
        }
    } catch (_) { /* xterm internals may change — fail gracefully */ }

    // Force xterm.js to re-measure font metrics after static CSS takes effect.
    requestAnimationFrame(() => {
        if (_termInstance) {
            _termInstance.resize(_termInstance.cols, _termInstance.rows);
            _termInstance.refresh(0, _termInstance.rows - 1);
        }
    });

    // Initial fit
    const dims = _termFit(term, container);

    // Subscribe to PTY events BEFORE spawning so nothing is lost
    const {listen} = window.__TAURI__.event;
    const {invoke} = window.__TAURI__.core;

    _termUnlistenOutput = await listen('terminal-output', (event) => {
        if (!_termInstance) return;
        const payload = event.payload;
        // Detect ESC[2J (erase display) — clear xterm.js scrollback so
        // Ctrl+L (zle clear-screen) actually blanks the viewport.
        if (payload.includes('\x1b[2J')) {
            _termInstance.clear();
        }
        _termInstance.write(payload);
    });
    _termUnlistenExit = await listen('terminal-exit', () => {
        _termSessionAlive = false;
        if (_termInstance) _termInstance.write('\r\n\x1b[90m[session ended — press any key to restart]\x1b[0m\r\n');
    });

    // Spawn PTY
    try {
        await invoke('terminal_spawn', {rows: dims.rows, cols: dims.cols});
        _termSessionAlive = true;
    } catch (err) {
        term.write(`\x1b[31mFailed to spawn terminal: ${err}\x1b[0m\r\n`);
    }

    // Forward keystrokes to PTY (or restart on dead session)
    term.onData((data) => {
        if (!_termSessionAlive) {
            _termDestroyInstance();
            _termSpawnSession();
            return;
        }
        invoke('terminal_write', {data}).catch(() => {});
    });

    // Observe pane resize
    const observer = new ResizeObserver(() => {
        clearTimeout(_termFitDebounce);
        _termFitDebounce = setTimeout(() => _termSendResize(), 50);
    });
    observer.observe(pane);
    pane._termResizeObserver = observer;

    term.focus();
}

function _termDestroyInstance() {
    if (_termUnlistenOutput) { _termUnlistenOutput(); _termUnlistenOutput = null; }
    if (_termUnlistenExit) { _termUnlistenExit(); _termUnlistenExit = null; }

    const pane = document.getElementById('terminalPane');
    if (pane?._termResizeObserver) {
        pane._termResizeObserver.disconnect();
        pane._termResizeObserver = null;
    }

    if (_termInstance) {
        _termInstance.dispose();
        _termInstance = null;
    }

    const container = document.getElementById('terminalContainer');
    if (container) container.innerHTML = '';

    _termSessionAlive = false;
}

/** Kill the backend PTY and tear down the frontend instance. */
function killTerminal() {
    const {invoke} = window.__TAURI__.core;
    invoke('terminal_kill').catch(() => {});
    _termDestroyInstance();
}

// ── Fit helpers ──

function _termFit(term, container) {
    if (!term || !container) return {rows: 24, cols: 80};
    const core = term._core;
    if (!core) return {rows: 24, cols: 80};

    const dims = core._renderService?.dimensions;
    if (!dims || !dims.css || !dims.css.cell || !dims.css.cell.width || !dims.css.cell.height) {
        return {rows: term.rows, cols: term.cols};
    }

    const cellW = dims.css.cell.width;
    const cellH = dims.css.cell.height;
    const availW = container.clientWidth;
    const availH = container.clientHeight;

    if (availW <= 0 || availH <= 0) return {rows: term.rows, cols: term.cols};

    const cols = Math.max(2, Math.floor(availW / cellW));
    const rows = Math.max(1, Math.floor(availH / cellH));

    if (cols !== term.cols || rows !== term.rows) {
        term.resize(cols, rows);
    }
    return {rows, cols};
}

function _termSendResize() {
    if (!_termInstance) return;
    const container = document.getElementById('terminalContainer');
    if (!container) return;
    const dims = _termFit(_termInstance, container);
    const {invoke} = window.__TAURI__.core;
    invoke('terminal_resize', {rows: dims.rows, cols: dims.cols}).catch(() => {});
}

// ── Toolbar button handlers + drag init ──

// Drag-to-dock via toolbar header
document.addEventListener('mousedown', (e) => {
    const handle = e.target.closest('#termDragHandle');
    if (!handle) return;
    _termOnDragStart(e);
});

// Init resize handles via shared modal-drag system (same pattern as audio player)
{
    const tp = document.getElementById('terminalPane');
    if (tp && typeof initModalDragResize === 'function') {
        initModalDragResize(tp);
    }
}
