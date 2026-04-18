// ── Modal Drag & Resize ──
// Makes all .modal-content elements draggable (via header) and resizable (via edges).
// Persists size/position to user prefs keyed by modal ID.
// Applies automatically to any modal inserted into the DOM.

(function () {
    let _dragState = null;
    let _resizeState = null;

    const EDGE_CURSOR = {
        n: 'ns-resize', s: 'ns-resize',
        e: 'ew-resize', w: 'ew-resize',
        ne: 'nesw-resize', sw: 'nesw-resize',
        nw: 'nwse-resize', se: 'nwse-resize',
    };

    /** Dock classes pin the player with bottom/right (or top/left) using !important — conflicts with resize math that uses left/top + width/height. */
    const PLAYER_DOCK_CLASSES = ['dock-tl', 'dock-tr', 'dock-bl', 'dock-br'];

    /** IDs that use the dock system (player + terminal). */
    const DOCK_IDS = ['audioNowPlaying', 'terminalPane'];

    function isDockable(modal) {
        return DOCK_IDS.includes(modal.id);
    }

    function stripPlayerDockForResize(modal) {
        if (!isDockable(modal)) return;
        PLAYER_DOCK_CLASSES.forEach((c) => modal.classList.remove(c));
    }

    function restorePlayerDockAfterResize(modal) {
        if (!isDockable(modal)) return;
        const dockKey = modal.id === 'terminalPane' ? 'terminalDock' : 'playerDock';
        let dock = 'dock-br';
        if (typeof prefs !== 'undefined') {
            const saved = prefs.getItem(dockKey);
            if (saved && PLAYER_DOCK_CLASSES.includes(saved)) {
                dock = saved;
            }
        }
        modal.style.left = '';
        modal.style.top = '';
        modal.style.right = '';
        modal.style.bottom = '';
        PLAYER_DOCK_CLASSES.forEach((c) => modal.classList.remove(c));
        modal.classList.add(dock);
    }

    function getModalKey(modal) {
        const overlay = modal.closest('.modal-overlay');
        return overlay?.id || modal.id || modal.closest('[id]')?.id || '';
    }

    function saveGeometry(modal) {
        const key = getModalKey(modal);
        if (!key || typeof prefs === 'undefined') return;
        const rect = modal.getBoundingClientRect();
        prefs.setItem('modal_' + key, JSON.stringify({
            left: Math.round(rect.left),
            top: Math.round(rect.top),
            width: Math.round(rect.width),
            height: Math.round(rect.height),
        }));
    }

    function restoreGeometry(modal) {
        const key = getModalKey(modal);
        if (!key || typeof prefs === 'undefined') return;
        const saved = prefs.getItem('modal_' + key);
        if (!saved) return;
        try {
            const geo = JSON.parse(saved);
            // Validate geometry is within current viewport
            if (geo.left < 0 || geo.top < 0 || geo.left > window.innerWidth - 100 || geo.top > window.innerHeight - 50) return;
            if (geo.width < 200 || geo.height < 100) return;

            const overlay = modal.closest('.modal-overlay');
            if (overlay) {
                overlay.style.alignItems = 'flex-start';
                overlay.style.justifyContent = 'flex-start';
            }
            modal.style.position = 'fixed';
            modal.style.left = geo.left + 'px';
            modal.style.top = geo.top + 'px';
            modal.style.width = geo.width + 'px';
            modal.style.height = geo.height + 'px';
            modal.style.margin = '0';
            modal.style.maxWidth = 'none';
            modal.style.maxHeight = 'none';

            const body = modal.querySelector('.modal-body');
            if (body) {
                const headerH = modal.querySelector('.modal-header')?.offsetHeight || 50;
                body.style.maxHeight = (geo.height - headerH - 10) + 'px';
            }
        } catch (e) {
            if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
        }
    }

    // Observe DOM for new modals
    const observer = new MutationObserver((mutations) => {
        for (const m of mutations) {
            for (const node of m.addedNodes) {
                if (node.nodeType !== 1) continue;
                const modal = node.classList?.contains('modal-content') ? node : node.querySelector?.('.modal-content');
                if (modal && !modal._dragInit) initModalDragResize(modal);
            }
        }
    });
    observer.observe(document.body, {childList: true, subtree: true});

    window.initModalDragResize = initModalDragResize;

    function initModalDragResize(modal) {
        modal._dragInit = true;
        // Don't override position for elements already using fixed positioning (like audio player)
        if (getComputedStyle(modal).position !== 'fixed') {
            modal.style.position = 'relative';
        }

        // Add resize handles
        const edges = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'];
        for (const edge of edges) {
            const handle = document.createElement('div');
            handle.className = 'modal-resize modal-resize-' + edge;
            handle.dataset.modalResize = edge;
            modal.appendChild(handle);
        }

        // Dockable panes have their own dock drag system — skip modal drag & geometry restore
        const isDock = isDockable(modal);

        // Restore saved geometry (skip for dockable panes — dock position managed separately)
        if (!isDock) restoreGeometry(modal);

        // Drag via modal header (skip for dockable panes — have custom dock drag)
        const header = !isDock ? modal.querySelector('.modal-header') : null;
        if (header) {
            header.style.cursor = 'move';
            header.addEventListener('mousedown', (e) => {
                if (e.target.closest('.modal-close, button, input, select')) return;
                if (e.button !== 0) return;
                e.preventDefault();
                const rect = modal.getBoundingClientRect();
                const overlay = modal.closest('.modal-overlay');

                if (overlay) {
                    overlay.style.alignItems = 'flex-start';
                    overlay.style.justifyContent = 'flex-start';
                }

                modal.style.position = 'fixed';
                modal.style.left = rect.left + 'px';
                modal.style.top = rect.top + 'px';
                modal.style.margin = '0';
                modal.style.width = rect.width + 'px';

                document.body.style.userSelect = 'none';
                document.body.style.cursor = 'move';
                _dragState = {modal, startX: e.clientX, startY: e.clientY, origLeft: rect.left, origTop: rect.top};
            });
        }

        // Resize via edge handles
        modal.addEventListener('mousedown', (e) => {
            const handle = e.target.closest('[data-modal-resize]');
            if (!handle) return;
            e.preventDefault();
            e.stopPropagation();
            const rect = modal.getBoundingClientRect();
            const overlay = modal.closest('.modal-overlay');

            if (overlay) {
                overlay.style.alignItems = 'flex-start';
                overlay.style.justifyContent = 'flex-start';
            }

            stripPlayerDockForResize(modal);

            modal.style.position = 'fixed';
            modal.style.left = rect.left + 'px';
            modal.style.top = rect.top + 'px';
            modal.style.margin = '0';
            modal.style.width = rect.width + 'px';
            modal.style.height = rect.height + 'px';
            modal.style.maxWidth = 'none';
            modal.style.maxHeight = 'none';

            if (isDockable(modal)) {
                modal.style.right = 'auto';
                modal.style.bottom = 'auto';
            }

            document.body.style.userSelect = 'none';
            document.body.style.cursor = EDGE_CURSOR[handle.dataset.modalResize] || '';
            _resizeState = {
                modal, edge: handle.dataset.modalResize,
                startX: e.clientX, startY: e.clientY,
                origLeft: rect.left, origTop: rect.top,
                origWidth: rect.width, origHeight: rect.height,
            };
        });
    }

    document.addEventListener('mousemove', (e) => {
        if (_dragState) {
            const {modal, startX, startY, origLeft, origTop} = _dragState;
            modal.style.left = (origLeft + e.clientX - startX) + 'px';
            modal.style.top = (origTop + e.clientY - startY) + 'px';
        }

        if (_resizeState) {
            const s = _resizeState;
            const dx = e.clientX - s.startX;
            const dy = e.clientY - s.startY;
            const minW = 300, minH = 200;

            let left = s.origLeft, top = s.origTop, w = s.origWidth, h = s.origHeight;

            if (s.edge.includes('e')) w = Math.max(minW, s.origWidth + dx);
            if (s.edge.includes('w')) {
                w = Math.max(minW, s.origWidth - dx);
                left = s.origLeft + s.origWidth - w;
            }
            if (s.edge.includes('s')) h = Math.max(minH, s.origHeight + dy);
            if (s.edge.includes('n')) {
                h = Math.max(minH, s.origHeight - dy);
                top = s.origTop + s.origHeight - h;
            }

            s.modal.style.left = left + 'px';
            s.modal.style.top = top + 'px';
            s.modal.style.width = w + 'px';
            s.modal.style.height = h + 'px';

            const body = s.modal.querySelector('.modal-body');
            if (body) {
                const headerH = s.modal.querySelector('.modal-header')?.offsetHeight || 50;
                body.style.maxHeight = (h - headerH - 10) + 'px';
            }
        }
    });

    document.addEventListener('mouseup', () => {
        if (_dragState) {
            saveGeometry(_dragState.modal);
        }
        if (_resizeState) {
            saveGeometry(_resizeState.modal);
            restorePlayerDockAfterResize(_resizeState.modal);
        }
        if (_dragState || _resizeState) {
            document.body.style.userSelect = '';
            document.body.style.cursor = '';
        }
        _dragState = null;
        _resizeState = null;
    });
})();
