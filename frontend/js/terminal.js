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

    // Show dock zones
    let overlay = document.getElementById('termDockZoneOverlay');
    if (!overlay) {
        overlay = document.createElement('div');
        overlay.id = 'termDockZoneOverlay';
        overlay.className = 'dock-zone-overlay';
        overlay.innerHTML =
            '<div class="dock-zone" style="top:4px;left:4px;width:calc(50% - 8px);height:calc(50% - 8px)">TL</div>' +
            '<div class="dock-zone" style="top:4px;right:4px;width:calc(50% - 8px);height:calc(50% - 8px)">TR</div>' +
            '<div class="dock-zone" style="bottom:4px;left:4px;width:calc(50% - 8px);height:calc(50% - 8px)">BL</div>' +
            '<div class="dock-zone" style="bottom:4px;right:4px;width:calc(50% - 8px);height:calc(50% - 8px)">BR</div>';
        document.body.appendChild(overlay);
    }
    overlay.classList.add('visible');

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

    // Highlight nearest dock zone
    const nearest = _termNearestDock(e.clientX, e.clientY);
    const overlay = document.getElementById('termDockZoneOverlay');
    if (overlay) {
        const zones = overlay.querySelectorAll('.dock-zone');
        const map = ['dock-tl', 'dock-tr', 'dock-bl', 'dock-br'];
        zones.forEach((z, i) => z.classList.toggle('active', map[i] === nearest));
    }
});

document.addEventListener('mouseup', (e) => {
    if (!_termDragState) return;
    const pane = document.getElementById('terminalPane');
    _termDragState = null;
    document.body.style.userSelect = '';
    document.body.style.cursor = '';

    const overlay = document.getElementById('termDockZoneOverlay');
    if (overlay) overlay.classList.remove('visible');

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

    // Initial fit
    const dims = _termFit(term, container);

    // Subscribe to PTY events BEFORE spawning so nothing is lost
    const {listen} = window.__TAURI__.event;
    const {invoke} = window.__TAURI__.core;

    _termUnlistenOutput = await listen('terminal-output', (event) => {
        if (_termInstance) _termInstance.write(event.payload);
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
