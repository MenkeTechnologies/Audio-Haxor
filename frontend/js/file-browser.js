// ── File Browser ──
const _ctxMenuNoEcho = {skipEchoToast: true};

function _ctxShortcutTip(id) {
    return typeof shortcutTip === 'function' ? shortcutTip(id) : {};
}

/** Rust `fs_list_dir` may return `\` on Windows; normalize for split/join logic. */
function normalizePathSeparators(p) {
    if (p == null || typeof p !== 'string') return '';
    return p.replace(/\\/g, '/');
}

/** Parent directory for Unix paths and Windows `C:/...` paths (after normalization). */
function parentDirectoryPath(p) {
    const n = normalizePathSeparators(p).replace(/\/+$/, '');
    if (n === '' || n === '/') return '/';
    if (/^[A-Za-z]:$/.test(n)) return n + '/';
    const i = n.lastIndexOf('/');
    if (i < 0) return '/';
    if (i === 0) return '/';
    const parent = n.slice(0, i);
    if (/^[A-Za-z]:$/.test(parent)) return parent + '/';
    return parent || '/';
}

/** Last path segment for labels (handles Windows `\\` from `fs_list_dir`). */
function pathFileName(p) {
    const n = normalizePathSeparators(p);
    const segs = n.split('/').filter(Boolean);
    return segs.length ? segs[segs.length - 1] : n;
}

// `var` (not `let`) on the module-state binding so the VM test sandbox can
// seed `_fileBrowserEntries` (and inspect path/inited state) via globalThis.
// Same rationale as the sort/select state below.
var _fileBrowserPath = null;

// ════════════════════════════════════════════════════════════════════
// ── Multi-pane (1-4 panes) ──
// Each pane has its own {path, entries, selection, scroll, navIdx}.
// Active pane is the target of all toolbar/menu/keyboard actions.
// Non-active panes still render their own data into their own DOM so
// the user can see multiple folders simultaneously (Total Commander
// style). Per-pane tabs/history land in a follow-up commit — for V1
// the global tabs/history apply to the active pane.
// ════════════════════════════════════════════════════════════════════
const FB_MAX_PANES = 4;
let _fbPanes = []; // {path, entries, selection: Set, scroll: number, navIdx: number}
let _fbActivePaneIdx = 0;
let _fbPaneCount = (() => {
    try {
        if (typeof prefs === 'undefined') return 1;
        const n = parseInt(prefs.getItem('fileBrowserPaneCount') || '1', 10);
        if (isNaN(n)) return 1;
        return Math.max(1, Math.min(FB_MAX_PANES, n));
    } catch (_) { return 1; }
})();

function _fbActivePane() { return _fbPanes[_fbActivePaneIdx]; }
function _fbPaneListEl(idx) {
    return document.querySelector(`.fb-pane[data-pane-idx="${idx}"] .file-list`);
}
function _fbActiveListEl() { return _fbPaneListEl(_fbActivePaneIdx); }

// Seed all max-pane slots up front so we can swap freely without
// null-checks downstream. Pane 0 takes over the existing globals
// (path/entries/selection); panes 1-3 start empty and lazy-load.
function _fbEnsurePanesInited() {
    for (let i = 0; i < FB_MAX_PANES; i++) {
        if (_fbPanes[i]) continue;
        _fbPanes[i] = {
            path: i === 0 ? _fileBrowserPath : null,
            entries: i === 0 ? _fileBrowserEntries : [],
            selection: i === 0 ? _fileSelected : new Set(),
            scroll: 0,
            navIdx: -1,
            // V2: per-pane tabs + nav history
            tabs: [],
            activeTabId: null,
            history: [],
            historyIdx: -1,
        };
    }
}
_fbEnsurePanesInited();

// Restore per-pane tab/path/etc state from V2 prefs (or migrate from V1
// global tabs into pane 0). Must run AFTER `_fbEnsurePanesInited` since
// it mutates `_fbPanes[i].tabs`. Then sync active pane → globals so the
// existing tab/history code sees correct initial state.
//
// NOTE: `_fbTabsRestoreFromPrefs` is defined later in the file — the
// definition is hoisted (function declaration) so this call is valid.
function _fbInitPanesFromPrefs() {
    if (typeof _fbTabsRestoreFromPrefs === 'function') _fbTabsRestoreFromPrefs();
    // Per-pane paths from prefs (fileBrowserPanesPaths = [path, …]) so
    // each pane reopens to where it was last left across launches.
    try {
        if (typeof prefs !== 'undefined') {
            const raw = prefs.getItem('fileBrowserPanesPaths');
            if (raw) {
                const arr = JSON.parse(raw);
                if (Array.isArray(arr)) {
                    for (let i = 0; i < Math.min(FB_MAX_PANES, arr.length); i++) {
                        if (_fbPanes[i] && typeof arr[i] === 'string' && arr[i]) {
                            _fbPanes[i].path = arr[i];
                        }
                    }
                }
            }
            const activeRaw = prefs.getItem('fileBrowserActivePaneIdx');
            if (activeRaw != null) {
                const n = parseInt(activeRaw, 10);
                if (!isNaN(n) && n >= 0 && n < FB_MAX_PANES) _fbActivePaneIdx = n;
            }
        }
    } catch (_) { /* ignore */ }
    // Sync active pane → globals (the existing fbTabs / fbHistory etc.
    // mirror).
    _fbLoadActivePaneIntoGlobals();
}
_fbInitPanesFromPrefs();

function _fbPersistPanePaths() {
    try {
        if (typeof prefs === 'undefined') return;
        const paths = _fbPanes.map((p) => p ? (p.path || '') : '');
        prefs.setItem('fileBrowserPanesPaths', JSON.stringify(paths));
        prefs.setItem('fileBrowserActivePaneIdx', String(_fbActivePaneIdx));
    } catch (_) { /* ignore */ }
}

// Save the current global state into the active pane (called before
// switching). After this the active pane's snapshot is up-to-date.
function _fbSaveGlobalsToActivePane() {
    const p = _fbActivePane();
    if (!p) return;
    p.path = _fileBrowserPath;
    p.entries = _fileBrowserEntries;
    p.selection = _fileSelected;
    // `_fileNavIdx` is `let`-declared further down in this file, so it's
    // in the TDZ during module-load initialization. typeof on a `let` in
    // TDZ also throws, so we use try/catch instead.
    try { p.navIdx = _fileNavIdx; } catch (_) { p.navIdx = -1; }
    const el = _fbActiveListEl();
    if (el) p.scroll = el.scrollTop;
    // V2: per-pane tabs + nav history
    p.tabs = _fbTabs;
    p.activeTabId = _fbActiveTabId;
    try { p.history = _fbHistory; } catch (_) { /* TDZ — leave as-is */ }
    try { p.historyIdx = _fbHistoryIdx; } catch (_) { /* TDZ */ }
}

// Pull the active pane's state into the globals (called after
// switching). Existing code that reads globals just sees the new
// active pane's data, no further changes needed.
function _fbLoadActivePaneIntoGlobals() {
    const p = _fbActivePane();
    if (!p) return;
    _fileBrowserPath = p.path;
    _fileBrowserEntries = p.entries || [];
    _fileSelected = p.selection || new Set();
    try { _fileNavIdx = p.navIdx; } catch (_) { /* TDZ — will sync on first activation post-init */ }
    // V2: per-pane tabs + nav history
    _fbTabs = p.tabs || [];
    _fbActiveTabId = p.activeTabId || null;
    try { _fbHistory = p.history || []; } catch (_) { /* TDZ */ }
    try { _fbHistoryIdx = (p.historyIdx == null ? -1 : p.historyIdx); } catch (_) { /* TDZ */ }
    if (typeof _updateNavButtons === 'function') _updateNavButtons();
}

function _fbSetActivePane(idx) {
    if (idx < 0 || idx >= _fbPaneCount) return;
    if (idx === _fbActivePaneIdx) return;
    _fbSaveGlobalsToActivePane();
    _fbActivePaneIdx = idx;
    _fbLoadActivePaneIntoGlobals();
    // Visual indicator
    document.querySelectorAll('.fb-pane').forEach((el, i) => {
        el.classList.toggle('fb-pane-active', i === _fbActivePaneIdx);
    });
    // Repaint top-level UI from new active pane
    if (_fileBrowserPath && typeof renderBreadcrumb === 'function') renderBreadcrumb(_fileBrowserPath);
    if (typeof updateBookmarkBtn === 'function') updateBookmarkBtn();
    if (typeof updateFileBulkBar === 'function') updateFileBulkBar();
    // Restore scroll
    const el = _fbActiveListEl();
    if (el) el.scrollTop = _fbActivePane().scroll || 0;
    // Refresh tab bar (per-pane in V2 — render all panes' bars).
    if (typeof renderFileBrowserTabs === 'function') renderFileBrowserTabs();
    _fbPersistPanePaths();
}

function _fbSetPaneCount(n) {
    n = Math.max(1, Math.min(FB_MAX_PANES, n));
    _fbPaneCount = n;
    document.querySelectorAll('.fb-pane').forEach((el, i) => {
        el.classList.toggle('fb-hidden', i >= n);
    });
    try {
        if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserPaneCount', String(n));
    } catch (_) { /* ignore */ }
    // If active pane got hidden, switch to 0.
    if (_fbActivePaneIdx >= n) _fbSetActivePane(0);
    // Newly-shown panes that have no content yet — load home into them.
    for (let i = 0; i < n; i++) {
        const p = _fbPanes[i];
        if (!p) continue;
        if (!p.path && _fbHomePath) {
            // Lazy-load home into this newly-shown pane.
            loadDirectoryIntoPane(_fbHomePath, i).catch(() => {});
        } else if (p.path && (!p.entries || p.entries.length === 0)) {
            // Already had a path but no entries (e.g. count was bumped
            // back up after being collapsed). Reload it.
            loadDirectoryIntoPane(p.path, i).catch(() => {});
        }
    }
}

function _fbCyclePaneCount() {
    _fbSetPaneCount((_fbPaneCount % FB_MAX_PANES) + 1);
}

// Light-weight loader for non-active panes — does the IPC + state
// update + targeted render WITHOUT touching active-pane globals or
// repainting active-pane-only UI (path bar, tabs, breadcrumb).
async function loadDirectoryIntoPane(dirPath, paneIdx) {
    if (paneIdx === _fbActivePaneIdx) {
        return loadDirectory(dirPath);
    }
    const pane = _fbPanes[paneIdx];
    if (!pane) return;
    pane.path = dirPath;
    pane.selection = new Set();
    pane.scroll = 0;
    pane.navIdx = -1;
    try {
        const result = await window.vstUpdater.listDirectory(dirPath, _fbShowHidden);
        pane.entries = result.entries || [];
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed_open_directory', {err: err.message || err}), 4000, 'error');
        return;
    }
    _fbPersistPanePaths();
    renderPaneList(paneIdx);
}

// Render an inactive pane's entries into its DOM. The active pane is
// rendered by the existing `renderFileList()` flow, which writes to
// pane 0's `#fileList` (or whichever pane is active via `_fbActiveListEl`).
function renderPaneList(paneIdx) {
    if (paneIdx === _fbActivePaneIdx) {
        if (typeof renderFileList === 'function') renderFileList();
        return;
    }
    const pane = _fbPanes[paneIdx];
    const listEl = _fbPaneListEl(paneIdx);
    if (!pane || !listEl) return;
    if (!pane.entries || pane.entries.length === 0) {
        listEl.innerHTML = `<div class="state-message"><div class="state-icon">&#128193;</div><h2>${escapeHtml(pane.path || '')}</h2><p>Empty</p></div>`;
        return;
    }
    // Build rows from this pane's entries + selection. We temporarily
    // swap globals so `buildFileListRowHtml` (which reads _fileSelected
    // etc.) sees the pane's state, then restore. Cheaper than
    // refactoring every helper to take a selection set parameter.
    const savedSel = _fileSelected;
    const savedEntries = _fileBrowserEntries;
    _fileSelected = pane.selection;
    _fileBrowserEntries = pane.entries;
    try {
        const html = pane.entries.map((e) => buildFileListRowHtml(e, '', null, _lastFilesMode)).join('');
        listEl.innerHTML = html;
    } finally {
        _fileSelected = savedSel;
        _fileBrowserEntries = savedEntries;
    }
}

// ── F5 (copy) / F6 (move) — active pane selection → next pane's folder ──
function _fbNextPaneIdx() {
    return (_fbActivePaneIdx + 1) % _fbPaneCount;
}
async function _fbCrossPaneOp(mode) {
    if (_fbPaneCount < 2) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'Need at least 2 panes (Cmd+\\\\ to split)'}), 4000, 'error');
        return;
    }
    const srcSel = _fbActivePane().selection;
    if (!srcSel || srcSel.size === 0) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'Nothing selected in active pane'}), 4000, 'error');
        return;
    }
    const destIdx = _fbNextPaneIdx();
    const destPane = _fbPanes[destIdx];
    if (!destPane || !destPane.path) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'Destination pane has no folder loaded'}), 4000, 'error');
        return;
    }
    let ok = 0, fail = 0;
    for (const src of srcSel) {
        const base = src.split('/').pop();
        const dest = `${destPane.path}/${base}`;
        try {
            if (mode === 'move') {
                await window.vstUpdater.renameFile(src, dest);
            } else {
                await window.vstUpdater.fsCopyPath(src, dest);
            }
            ok++;
        } catch (_) { fail++; }
    }
    if (typeof showToast === 'function') {
        const verb = mode === 'move' ? 'moved' : 'copied';
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `${verb} ${ok} item${ok === 1 ? '' : 's'} → pane ${destIdx + 1}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} ${mode}${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    // Refresh dest pane (new files appeared) and active pane (move removed them).
    await loadDirectoryIntoPane(destPane.path, destIdx);
    if (mode === 'move' && _fileBrowserPath) await loadDirectory(_fileBrowserPath);
}

// Click in any pane → make it the active pane. Capture phase so it runs
// before the row-click handler which would otherwise scope to old active.
document.addEventListener('click', (e) => {
    const pane = e.target.closest('.fb-pane[data-pane-idx]');
    if (!pane) return;
    const idx = parseInt(pane.dataset.paneIdx, 10);
    if (Number.isInteger(idx) && idx !== _fbActivePaneIdx) {
        _fbSetActivePane(idx);
    }
}, true);

if (typeof window !== 'undefined') {
    window._fbSetPaneCount = _fbSetPaneCount;
    window._fbCyclePaneCount = _fbCyclePaneCount;
    window._fbSetActivePane = _fbSetActivePane;
    window._fbCrossPaneOp = _fbCrossPaneOp;
    window.loadDirectoryIntoPane = loadDirectoryIntoPane;
    window.renderPaneList = renderPaneList;
    // Exposed for native-file-drag.js so OS drag can pick up per-pane
    // selection (a row that's in the active pane's selection drags the
    // whole selection out to Finder/Desktop, matching the in-app
    // pane-to-pane drag behavior).
    window._fbPanes = _fbPanes;
}

// ── Tabs (per-pane in V2) ──
// Each pane has its own `tabs[]` + `activeTabId` in `_fbPanes[paneIdx]`.
// The globals `_fbTabs` / `_fbActiveTabId` are kept as a live mirror of
// the active pane for code that reads them — same pattern as
// `_fileBrowserPath` and friends.
// `var` (not `let`) so hoisting makes these accessible to the pane-
// init code that runs above (they're populated from per-pane snapshots).
var _fbTabs = []; // mirror of active pane's tabs
var _fbActiveTabId = null; // mirror of active pane's activeTabId

function _fbTabsRestoreFromPrefs() {
    // V2 prefs format: per-pane tabs as `fileBrowserPanesTabs` =
    // [[{id,path}], …]. Falls back to the V1 `fileBrowserTabs` global
    // single-pane list, migrating it into pane 0 on first load.
    try {
        if (typeof prefs === 'undefined') return;
        const v2raw = prefs.getItem('fileBrowserPanesTabs');
        if (v2raw) {
            const v2 = JSON.parse(v2raw);
            if (Array.isArray(v2)) {
                for (let i = 0; i < Math.min(FB_MAX_PANES, v2.length); i++) {
                    const arr = Array.isArray(v2[i]) ? v2[i] : [];
                    if (_fbPanes[i]) {
                        _fbPanes[i].tabs = arr.filter((t) => t && typeof t.id === 'string' && typeof t.path === 'string');
                    }
                }
                const activeRaw = prefs.getItem('fileBrowserPanesActiveTabIds');
                if (activeRaw) {
                    const active = JSON.parse(activeRaw);
                    if (Array.isArray(active)) {
                        for (let i = 0; i < Math.min(FB_MAX_PANES, active.length); i++) {
                            if (_fbPanes[i]) _fbPanes[i].activeTabId = active[i] || null;
                        }
                    }
                }
                return;
            }
        }
        // V1 migration — single global tab list goes into pane 0.
        const v1raw = prefs.getItem('fileBrowserTabs');
        if (v1raw) {
            const parsed = JSON.parse(v1raw);
            if (Array.isArray(parsed) && _fbPanes[0]) {
                _fbPanes[0].tabs = parsed.filter((t) => t && typeof t.id === 'string' && typeof t.path === 'string');
                _fbPanes[0].activeTabId = prefs.getItem('fileBrowserActiveTabId') || null;
            }
        }
    } catch (_) { /* ignore */ }
}

function _fbTabsPersist() {
    try {
        if (typeof prefs === 'undefined') return;
        // Sync globals into active pane first so the snapshot is current.
        if (_fbActivePane()) {
            _fbActivePane().tabs = _fbTabs;
            _fbActivePane().activeTabId = _fbActiveTabId;
        }
        const tabsByPane = _fbPanes.map((p) => p ? (p.tabs || []) : []);
        const activeByPane = _fbPanes.map((p) => p ? (p.activeTabId || null) : null);
        prefs.setItem('fileBrowserPanesTabs', JSON.stringify(tabsByPane));
        prefs.setItem('fileBrowserPanesActiveTabIds', JSON.stringify(activeByPane));
    } catch (_) { /* ignore */ }
}

function _fbActiveTab() {
    return _fbTabs.find((t) => t.id === _fbActiveTabId) || _fbTabs[0] || null;
}
function _fbTabId() {
    return 'tab-' + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 6);
}
function _fbTabDisplayName(path) {
    if (!path) return 'untitled';
    if (path === '/') return '/';
    return path.split('/').filter(Boolean).pop() || path;
}

// Render every visible pane's tab bar. Each pane has its own
// `[data-fb-tab-list]` inside its `.fb-pane` container; we draw from
// the per-pane state (or the global mirror for the active pane).
function renderFileBrowserTabs() {
    for (let i = 0; i < FB_MAX_PANES; i++) {
        const pane = _fbPanes[i];
        if (!pane) continue;
        // Active pane reads from globals (live state); others from snapshot.
        const tabs = (i === _fbActivePaneIdx) ? _fbTabs : (pane.tabs || []);
        const activeId = (i === _fbActivePaneIdx) ? _fbActiveTabId : (pane.activeTabId || null);
        const list = document.querySelector(`.fb-pane[data-pane-idx="${i}"] [data-fb-tab-list]`);
        if (!list) continue;
        list.innerHTML = tabs.map((t) => {
            const active = t.id === activeId ? ' fb-tab-active' : '';
            return `<button class="fb-tab${active}" data-fb-tab-id="${escapeHtml(t.id)}" data-fb-tab-pane="${i}" title="${escapeHtml(t.path)}">`
                + `<span class="fb-tab-name">${escapeHtml(_fbTabDisplayName(t.path))}</span>`
                + `<span class="fb-tab-close" data-fb-tab-close="${escapeHtml(t.id)}" data-fb-tab-close-pane="${i}" title="Close tab (Cmd+Shift+W)">&times;</span>`
                + `</button>`;
        }).join('');
        const activeEl = list.querySelector('.fb-tab-active');
        if (activeEl && typeof activeEl.scrollIntoView === 'function') {
            activeEl.scrollIntoView({block: 'nearest', inline: 'nearest'});
        }
    }
}

// New tab on active pane (default) or specific pane.
function fbNewTab(path, paneIdx) {
    if (paneIdx == null) paneIdx = _fbActivePaneIdx;
    if (paneIdx !== _fbActivePaneIdx) _fbSetActivePane(paneIdx);
    const tabPath = path || (_fbActiveTab() ? _fbActiveTab().path : _fileBrowserPath) || _fbHomePath || '/';
    const tab = {id: _fbTabId(), path: tabPath};
    _fbTabs.push(tab);
    _fbActiveTabId = tab.id;
    _fbTabsPersist();
    renderFileBrowserTabs();
    if (tabPath) loadDirectory(tabPath);
}

function fbCloseTab(id, paneIdx) {
    if (paneIdx == null) paneIdx = _fbActivePaneIdx;
    if (paneIdx !== _fbActivePaneIdx) _fbSetActivePane(paneIdx);
    const idx = _fbTabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    _fbTabs.splice(idx, 1);
    if (_fbTabs.length === 0) {
        const fresh = {id: _fbTabId(), path: _fbHomePath || '/'};
        _fbTabs.push(fresh);
        _fbActiveTabId = fresh.id;
        _fbTabsPersist();
        renderFileBrowserTabs();
        loadDirectory(fresh.path);
        return;
    }
    if (_fbActiveTabId === id) {
        const next = _fbTabs[Math.min(idx, _fbTabs.length - 1)];
        _fbActiveTabId = next.id;
        _fbTabsPersist();
        renderFileBrowserTabs();
        loadDirectory(next.path);
    } else {
        _fbTabsPersist();
        renderFileBrowserTabs();
    }
}

function fbSwitchTab(id, paneIdx) {
    if (paneIdx == null) paneIdx = _fbActivePaneIdx;
    if (paneIdx !== _fbActivePaneIdx) _fbSetActivePane(paneIdx);
    const tab = _fbTabs.find((t) => t.id === id);
    if (!tab || tab.id === _fbActiveTabId) return;
    _fbActiveTabId = tab.id;
    _fbTabsPersist();
    renderFileBrowserTabs();
    loadDirectory(tab.path);
}

function fbCycleTab(delta) {
    if (_fbTabs.length === 0) return;
    const idx = _fbTabs.findIndex((t) => t.id === _fbActiveTabId);
    const next = (idx + delta + _fbTabs.length) % _fbTabs.length;
    fbSwitchTab(_fbTabs[next].id);
}

// Click handlers — read the pane the tab belongs to so clicking a tab
// in pane 2 activates pane 2 AND switches its tab.
document.addEventListener('click', (e) => {
    const close = e.target.closest('[data-fb-tab-close]');
    if (close) {
        e.stopPropagation();
        const paneIdx = parseInt(close.dataset.fbTabClosePane, 10);
        fbCloseTab(close.dataset.fbTabClose, Number.isInteger(paneIdx) ? paneIdx : undefined);
        return;
    }
    const tab = e.target.closest('[data-fb-tab-id]');
    if (tab) {
        e.stopPropagation();
        const paneIdx = parseInt(tab.dataset.fbTabPane, 10);
        fbSwitchTab(tab.dataset.fbTabId, Number.isInteger(paneIdx) ? paneIdx : undefined);
        return;
    }
    const add = e.target.closest('[data-fb-tab-add]');
    if (add) {
        e.stopPropagation();
        const paneEl = add.closest('.fb-pane[data-pane-idx]');
        const paneIdx = paneEl ? parseInt(paneEl.dataset.paneIdx, 10) : _fbActivePaneIdx;
        fbNewTab(null, Number.isInteger(paneIdx) ? paneIdx : undefined);
    }
});
if (typeof window !== 'undefined') {
    window.fbNewTab = fbNewTab;
    window.fbCloseTab = fbCloseTab;
    window.fbSwitchTab = fbSwitchTab;
    window.fbCycleTab = fbCycleTab;
    window.renderFileBrowserTabs = renderFileBrowserTabs;
}

// Git porcelain status for the current folder, loaded async per
// loadDirectory. Empty map outside a git repo. Read by
// `buildFileListRowHtml` to render per-row M/A/?/U badges.
var _fbGitStatus = {};

// Whether dotfiles (`.bashrc`, `.git`, etc.) are visible. Persisted in
// prefs so the toggle survives across sessions. Default false matches
// Nautilus + Finder defaults.
var _fbShowHidden = (() => {
    try { return typeof prefs !== 'undefined' && prefs.getItem('fileBrowserShowHidden') === '1'; }
    catch (_) { return false; }
})();
function fileBrowserToggleHidden() {
    _fbShowHidden = !_fbShowHidden;
    try { if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserShowHidden', _fbShowHidden ? '1' : '0'); }
    catch (_) { /* ignore */ }
    if (typeof showToast === 'function') {
        showToast(toastFmt('toast.deleted_name', {name: _fbShowHidden ? 'showing hidden files' : 'hiding hidden files'}));
    }
    if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
}
if (typeof window !== 'undefined') window.fileBrowserToggleHidden = fileBrowserToggleHidden;
var _fileBrowserEntries = [];
var _fileBrowserInited = false;
/** Invalidate in-flight chunked file list renders when directory or filter changes. */
var _fileListRenderSeq = 0;
/** Search mode for `filterFiles` (`registerFilter`); used by `renderFileList`. */
var _lastFilesMode = 'fuzzy';
const FILE_LIST_CHUNK = 80;

// ── Favorite Directories ──
function getFavDirs() {
    return prefs.getObject('favDirs', []);
}

function saveFavDirs(dirs) {
    prefs.setItem('favDirs', dirs);
}

function isFavDir(dirPath) {
    return getFavDirs().some(d => d.path === dirPath);
}

function addFavDir(dirPath) {
    const dirs = getFavDirs();
    if (dirs.some(d => d.path === dirPath)) return;
    const name = pathFileName(dirPath) || dirPath;
    dirs.push({path: dirPath, name});
    saveFavDirs(dirs);
    renderFavDirs();
    updateBookmarkBtn();
    showToast(toastFmt('toast.bookmarked_name', {name}));
}

function removeFavDir(dirPath) {
    saveFavDirs(getFavDirs().filter(d => d.path !== dirPath));
    renderFavDirs();
    updateBookmarkBtn();
    showToast(toastFmt('toast.bookmark_removed'));
}

function renderFavDirs() {
    const container = document.getElementById('fileFavs');
    const grid = document.getElementById('fileFavsGrid');
    if (!container || !grid) return;
    const dirs = getFavDirs();
    if (dirs.length === 0) {
        container.classList.add('fb-hidden');
        return;
    }
    container.classList.remove('fb-hidden');
    const rmTitle = catalogFmt('ui.tt.remove_bookmark_from_chip');
    grid.innerHTML = dirs.map(d =>
        `<div class="file-fav-chip" data-fav-dir="${escapeHtml(d.path)}" title="${escapeHtml(d.path)}">
      <span class="fav-chip-icon">&#128193;</span>
      <span class="fav-chip-name">${escapeHtml(d.name)}</span>
      <span class="fav-chip-remove" data-remove-fav-dir="${escapeHtml(d.path)}" title="${escapeHtml(rmTitle)}">&#10005;</span>
    </div>`
    ).join('');
}

function updateBookmarkBtn() {
    const btn = document.getElementById('btnFileFav');
    if (!btn || !_fileBrowserPath) return;
    const fav = isFavDir(_fileBrowserPath);
    const fmt = catalogFmt;
    const label = fav ? fmt('ui.btn.9733_unbookmark') : fmt('ui.btn.9733_bookmark');
    btn.innerHTML = `&#9733; <span>${escapeHtml(label)}</span>`;
    btn.title = fav ? fmt('ui.tt.remove_current_directory_from_bookmarks') : fmt('ui.tt.bookmark_current_directory');
}

/** Keep in sync with `AUDIO_EXTENSIONS` in `src-tauri/src/audio_extensions.rs`. */
const AUDIO_EXTS = [
    'wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'wma', 'aac', 'opus', 'rex', 'rx2', 'sf2', 'sfz',
];
/** Keep in sync with `DAW_EXTENSIONS` in `src-tauri/src/daw_scanner.rs` (lowercase, no dot). */
const DAW_EXTS = ['als', 'logicx', 'flp', 'cpr', 'npr', 'bwproject', 'rpp', 'rpp-bak', 'ptx', 'ptf', 'song', 'reason', 'aup', 'aup3', 'band', 'ardour', 'dawproject'];
const PLUGIN_EXTS = ['vst', 'vst3', 'component', 'aaxplugin'];

function fileIcon(entry) {
    if (entry.isDir) return '&#128193;';
    const ext = entry.ext;
    if (AUDIO_EXTS.includes(ext)) return '&#127925;';
    if (DAW_EXTS.includes(ext)) return '&#127911;';
    if (PLUGIN_EXTS.includes(ext)) return '&#9889;';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(ext)) return '&#128247;';
    if (['pdf'].includes(ext)) return '&#128196;';
    if (['json', 'toml', 'xml', 'yaml', 'yml'].includes(ext)) return '&#128203;';
    if (['zip', 'gz', 'tar', 'rar', '7z', 'dmg'].includes(ext)) return '&#128230;';
    return '&#128196;';
}

// ── Bulk rename modal ──
// Multi-select rows → toolbar Rename... → modal with pattern fields.
// Pure JS — backend just receives per-file `rename_file` calls in a loop.
// Live preview table updates as the user types; rows highlighted red when
// a new name would conflict with another row's new name (would overwrite).

function _fbBulkRenameSelectedEntries() {
    if (!Array.isArray(_fileBrowserEntries) || _fileSelected.size === 0) return [];
    const out = [];
    for (const p of _fileSelected) {
        const e = _fileEntryByPath ? _fileEntryByPath(p) : null;
        if (e) out.push(e);
    }
    return out;
}

/** Compute the new name for a single entry given the current pattern fields. */
function _fbBulkRenameComputeName(entry, params, index) {
    const orig = entry.name;
    const dot = orig.lastIndexOf('.');
    const hasExt = !entry.isDir && dot > 0;
    let base = hasExt ? orig.slice(0, dot) : orig;
    const ext = hasExt ? orig.slice(dot) : '';
    // Find/replace on the basename only — preserve extension.
    if (params.find) {
        try {
            if (params.regex) {
                const re = new RegExp(params.find, 'g');
                base = base.replace(re, params.replace);
            } else {
                base = base.split(params.find).join(params.replace);
            }
        } catch (_) {
            // Invalid regex — leave base unchanged; UI will surface no change.
        }
    }
    // Placeholder substitution. {n} = padded number; {name}, {ext} for templates.
    const padded = String((params.numStart || 0) + index).padStart(Math.max(1, params.numPad || 1), '0');
    const subs = (s) => String(s || '')
        .replace(/\{n\}/g, padded)
        .replace(/\{name\}/g, base)
        .replace(/\{ext\}/g, ext.replace(/^\./, ''));
    const prefix = subs(params.prefix);
    const suffix = subs(params.suffix);
    return `${prefix}${base}${suffix}${ext}`;
}

function _fbBulkRenameCollectParams() {
    const $ = (id) => document.getElementById(id);
    return {
        find: $('fbBulkRenameFind')?.value || '',
        replace: $('fbBulkRenameReplace')?.value || '',
        regex: !!$('fbBulkRenameRegex')?.checked,
        prefix: $('fbBulkRenamePrefix')?.value || '',
        suffix: $('fbBulkRenameSuffix')?.value || '',
        numStart: Number($('fbBulkRenameNumStart')?.value) || 0,
        numPad: Number($('fbBulkRenameNumPad')?.value) || 1,
    };
}

function _fbBulkRenameRenderPreview() {
    if (typeof document === 'undefined') return;
    const params = _fbBulkRenameCollectParams();
    const entries = _fbBulkRenameSelectedEntries();
    const previewEl = document.getElementById('fbBulkRenamePreview');
    const status = document.getElementById('fbBulkRenameStatus');
    const apply = document.getElementById('fbBulkRenameApply');
    if (!previewEl) return;
    const rows = [];
    const newNameCounts = new Map();
    let changed = 0;
    for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        const newName = _fbBulkRenameComputeName(e, params, i);
        newNameCounts.set(newName, (newNameCounts.get(newName) || 0) + 1);
        rows.push({orig: e.name, next: newName, path: e.path});
        if (newName !== e.name) changed++;
    }
    const tbody = rows.map((r) => {
        const conflict = newNameCounts.get(r.next) > 1;
        const unchanged = r.next === r.orig;
        const cls = conflict ? 'conflict' : (unchanged ? 'unchanged' : '');
        return `<tr class="${cls}"><td>${escapeHtml(r.orig)}</td><td>${escapeHtml(r.next)}</td></tr>`;
    }).join('');
    previewEl.innerHTML = `<table>
        <thead><tr><th>Current</th><th>New</th></tr></thead>
        <tbody>${tbody}</tbody>
    </table>`;
    const conflicts = [...newNameCounts.values()].filter((n) => n > 1).length;
    if (status) {
        status.textContent = conflicts > 0
            ? `${conflicts} conflict${conflicts === 1 ? '' : 's'} · ${changed} change${changed === 1 ? '' : 's'}`
            : `${changed} change${changed === 1 ? '' : 's'} pending`;
    }
    if (apply) {
        // Disable Apply when there are conflicts (would overwrite each other)
        // or when nothing would change.
        apply.disabled = conflicts > 0 || changed === 0;
    }
}

function showFileBulkRenameModal() {
    if (typeof document === 'undefined') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal) return;
    const entries = _fbBulkRenameSelectedEntries();
    if (entries.length === 0) return;
    const countEl = document.getElementById('fbBulkRenameCount');
    if (countEl) countEl.textContent = `(${entries.length} selected)`;
    // Reset fields to sensible defaults on open.
    const reset = (id, val) => { const el = document.getElementById(id); if (el) el.value = val; };
    reset('fbBulkRenameFind', '');
    reset('fbBulkRenameReplace', '');
    reset('fbBulkRenamePrefix', '');
    reset('fbBulkRenameSuffix', '');
    reset('fbBulkRenameNumStart', '1');
    reset('fbBulkRenameNumPad', '3');
    const regex = document.getElementById('fbBulkRenameRegex');
    if (regex) regex.checked = false;
    modal.classList.add('modal-visible');
    _fbBulkRenameRenderPreview();
    // Focus the Find field for keyboard-first workflow.
    requestAnimationFrame(() => {
        const find = document.getElementById('fbBulkRenameFind');
        if (find) find.focus();
    });
}

function hideFileBulkRenameModal() {
    if (typeof document === 'undefined') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (modal) modal.classList.remove('modal-visible');
}

async function applyFileBulkRename() {
    const params = _fbBulkRenameCollectParams();
    const entries = _fbBulkRenameSelectedEntries();
    if (entries.length === 0) return;
    let success = 0;
    let fail = 0;
    for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        const newName = _fbBulkRenameComputeName(e, params, i);
        if (newName === e.name) continue;
        const dir = e.path.replace(/\/[^/]+$/, '');
        const newPath = `${dir}/${newName}`;
        try {
            await window.vstUpdater.renameFile(e.path, newPath);
            success++;
        } catch (_) {
            fail++;
        }
    }
    if (typeof showToast === 'function') {
        if (fail === 0) showToast(toastFmt('toast.deleted_name', {name: `renamed ${success} file${success === 1 ? '' : 's'}`}));
        else showToast(toastFmt('toast.failed', {err: `${fail} of ${success + fail} rename${success + fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    hideFileBulkRenameModal();
    if (typeof clearFileSelection === 'function') clearFileSelection();
    if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
}

// ── PDF preview cache helpers (canvas ↔ PNG bytes) ──
// `canvas.toBlob` is async; wrap in a Promise that resolves to a Uint8Array
// of the PNG bytes (suitable for `pdf_preview_set` which expects Vec<u8>).
// Used after PDF.js renders a page so the result can be cached in SQLite.
async function _fbCanvasToPngBytes(canvas) {
    return new Promise((resolve, reject) => {
        try {
            canvas.toBlob(async (blob) => {
                if (!blob) { resolve(null); return; }
                try {
                    const buf = await blob.arrayBuffer();
                    resolve(new Uint8Array(buf));
                } catch (e) { reject(e); }
            }, 'image/png');
        } catch (e) { reject(e); }
    });
}

/** Paints raw PNG bytes (returned by `pdf_preview_get`) into a canvas.
 *  Used on cache HIT — avoids the PDF.js module load + render entirely. */
async function _fbPaintPngBytesIntoCanvas(canvas, bytes) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    // Construct a Blob with the right MIME → object URL → Image → drawImage.
    // ObjectURL revoked after draw to free memory.
    return new Promise((resolve, reject) => {
        let url = null;
        try {
            const blob = new Blob([u8], {type: 'image/png'});
            url = URL.createObjectURL(blob);
            const img = new Image();
            img.onload = () => {
                canvas.width = img.naturalWidth;
                canvas.height = img.naturalHeight;
                const ctx = canvas.getContext('2d');
                ctx.drawImage(img, 0, 0);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.onerror = (e) => {
                if (url) URL.revokeObjectURL(url);
                reject(e);
            };
            img.src = url;
        } catch (e) {
            if (url) URL.revokeObjectURL(url);
            reject(e);
        }
    });
}

// ── PDF.js lazy-loader (shared across preview pane + PDF inventory thumbs) ──
// Module-level promise so concurrent calls share the same load. PDF.js is
// ~350 KB (main) + ~1.4 MB (worker); deferring until the first PDF render
// keeps it out of the initial bundle. Absolute path from WebView root —
// frontend is served from `frontend/` so `/lib/...` resolves correctly.
// Exposed on `window` so `pdf.js` (PDF inventory tab) can reuse without
// duplicating the loader / canvas-blob helpers below.
var _pdfJsPromise = null;
function loadPdfJs() {
    if (_pdfJsPromise) return _pdfJsPromise;
    _pdfJsPromise = (async () => {
        const mod = await import('/lib/pdf.min.mjs');
        if (mod && mod.GlobalWorkerOptions) {
            mod.GlobalWorkerOptions.workerSrc = '/lib/pdf.worker.min.mjs';
        }
        return mod;
    })().catch((err) => {
        _pdfJsPromise = null; // allow retry on next preview
        throw err;
    });
    return _pdfJsPromise;
}
if (typeof window !== 'undefined') {
    window.loadPdfJs = loadPdfJs;
    window.canvasToPngBytes = _fbCanvasToPngBytes;
    window.paintPngBytesIntoCanvas = _fbPaintPngBytesIntoCanvas;
}

// ── Inline preview pane (right side of file list) ──
// Persistent side panel that updates whenever a file row is focused or
// clicked. Toggled via the toolbar Preview button or Cmd+I. Per-extension
// content:
//   - audio  → 800-wide waveform + grid of audio metadata
//   - image  → thumbnail (loaded as base64 data URL via `fs_read_file_base64`,
//              capped at 2 MiB to prevent the WebView stalling on huge JPEGs)
//   - text   → first 4 KiB via `fs_read_head`, in a scrollable `<pre>`
//   - other  → just the basic metadata (size / date / path)
const FB_PREVIEW_IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'ico'];
const FB_PREVIEW_TEXT_EXTS = ['txt', 'md', 'json', 'toml', 'xml', 'yaml', 'yml', 'log', 'csv', 'tsv', 'sh', 'js', 'ts', 'rs', 'py', 'rb', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'css', 'html', 'env', 'gitignore'];
const FB_PREVIEW_IMAGE_CAP = 2 * 1024 * 1024;

/** Currently-previewed path — used to skip redundant fetches when the same
 *  row is re-clicked, and to abandon in-flight previews when selection moves. */
var _fbPreviewPath = null;
var _fbPreviewSeq = 0;

function isPreviewPaneVisible() {
    if (typeof document === 'undefined') return false;
    const pane = document.getElementById('fbPreviewPane');
    return !!(pane && !pane.classList.contains('fb-hidden'));
}

function setPreviewPaneVisible(visible) {
    if (typeof document === 'undefined') return;
    const pane = document.getElementById('fbPreviewPane');
    if (!pane) return;
    pane.classList.toggle('fb-hidden', !visible);
    try { prefs.setItem('fileBrowserPreviewVisible', visible ? '1' : '0'); } catch (_) { /* ignore */ }
    if (visible && _fbPreviewPath) populatePreviewPane(_fbPreviewPath);
}

function toggleFileBrowserPreviewPane() {
    setPreviewPaneVisible(!isPreviewPaneVisible());
}

function _fbPreviewMimeFromExt(ext) {
    switch (ext) {
        case 'jpg': case 'jpeg': return 'image/jpeg';
        case 'png': return 'image/png';
        case 'gif': return 'image/gif';
        case 'webp': return 'image/webp';
        case 'bmp': return 'image/bmp';
        case 'svg': return 'image/svg+xml';
        case 'ico': return 'image/x-icon';
        default: return 'application/octet-stream';
    }
}

async function populatePreviewPane(filePath) {
    if (typeof document === 'undefined' || !filePath) return;
    if (!isPreviewPaneVisible()) {
        _fbPreviewPath = filePath; // remember; will populate when toggled on
        return;
    }
    _fbPreviewPath = filePath;
    const seq = ++_fbPreviewSeq;
    const title = document.getElementById('fbPreviewTitle');
    const body = document.getElementById('fbPreviewBody');
    if (!body) return;
    const name = filePath.split('/').pop();
    const ext = (name.split('.').pop() || '').toLowerCase();
    if (title) title.textContent = name;
    body.innerHTML = '<div class="fb-preview-empty">Loading…</div>';

    const isAudio = typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
    const isImage = FB_PREVIEW_IMAGE_EXTS.includes(ext);
    const isText = FB_PREVIEW_TEXT_EXTS.includes(ext);

    // Build basic metadata grid common to all types.
    const entry = (typeof _fileEntryByPath === 'function') ? _fileEntryByPath(filePath) : null;
    const metaRows = [
        ['Path', escapeHtml(filePath)],
        ['Type', escapeHtml(ext || '—')],
    ];
    if (entry && !entry.isDir) {
        if (entry.sizeFormatted) metaRows.push(['Size', escapeHtml(entry.sizeFormatted)]);
        if (entry.modified) metaRows.push(['Modified', escapeHtml(entry.modified)]);
        if (entry.created) metaRows.push(['Created', escapeHtml(entry.created)]);
    }
    const metaHtml = `<dl class="fb-preview-meta">${metaRows.map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${v}</dd>`).join('')}</dl>`;

    if (isAudio) {
        body.innerHTML = `
            <canvas class="fb-preview-wf" id="fbPreviewWf" data-wf-path="${escapeHtml(filePath)}" width="600" height="80"></canvas>
            ${metaHtml}
        `;
        if (typeof drawMiniWaveform === 'function') {
            const canvas = document.getElementById('fbPreviewWf');
            if (canvas) drawMiniWaveform(canvas, filePath);
        }
        return;
    }

    if (isImage) {
        try {
            const b64 = await window.vstUpdater.fsReadFileBase64(filePath, FB_PREVIEW_IMAGE_CAP);
            if (seq !== _fbPreviewSeq) return; // selection moved during the load
            const mime = _fbPreviewMimeFromExt(ext);
            body.innerHTML = `<img class="fb-preview-image" src="data:${mime};base64,${b64}" alt="${escapeHtml(name)}">${metaHtml}`;
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            // Most common failure is "File too large" (over the 2 MiB cap).
            body.innerHTML = `<div class="fb-preview-empty">Image too large to preview</div>${metaHtml}`;
        }
        return;
    }

    if (isText) {
        try {
            const head = await window.vstUpdater.fsReadHead(filePath, 4096);
            if (seq !== _fbPreviewSeq) return;
            body.innerHTML = `<pre class="fb-preview-text">${escapeHtml(head)}</pre>${metaHtml}`;
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            body.innerHTML = `<div class="fb-preview-empty">Could not read file</div>${metaHtml}`;
        }
        return;
    }

    if (ext === 'pdf') {
        // Single canonical render width shared across preview pane, Quick-
        // look, and the PDF inventory thumbnail. Cache key is just
        // (path, page, 600) — viewing a thumb pre-populates the Cmd+I
        // cache and vice versa. Display canvas CSS-resizes to whatever
        // each consumer wants (thumb shrinks to 80px, pane to ~320px,
        // quicklook stays at 600). One render per PDF, ever.
        const FB_PDF_PREVIEW_WIDTH = 600;
        const FB_PDF_PREVIEW_PAGE = 1;
        body.innerHTML = `
            <canvas class="fb-preview-pdf-canvas" id="fbPreviewPdfCanvas"></canvas>
            ${metaHtml}
        `;
        const canvas = document.getElementById('fbPreviewPdfCanvas');
        if (!canvas) return;

        // 1) Try SQLite cache first — a fresh hit skips both the PDF.js
        //    lazy module load (~50-200 ms) and the page render (~tens-of-ms
        //    for typical pages, seconds for complex ones).
        try {
            const cached = await window.vstUpdater.pdfPreviewGet(filePath, FB_PDF_PREVIEW_PAGE, FB_PDF_PREVIEW_WIDTH);
            if (seq !== _fbPreviewSeq) return;
            // Raw ArrayBuffer — check byteLength (truthiness is always true).
            if (cached && cached.byteLength > 0) {
                await _fbPaintPngBytesIntoCanvas(canvas, cached);
                if (seq !== _fbPreviewSeq) return;
                return;
            }
        } catch (_) { /* cache miss / IPC error → fall through to render */ }

        // 2) Cache miss → render via PDF.js, paint canvas, persist to cache.
        try {
            const bytes = await window.vstUpdater.fsReadFileBytes(filePath);
            if (seq !== _fbPreviewSeq) return;
            const pdfjs = await loadPdfJs();
            if (seq !== _fbPreviewSeq) return;
            const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
            const pdf = await pdfjs.getDocument({data: u8}).promise;
            if (seq !== _fbPreviewSeq) return;
            const page = await pdf.getPage(FB_PDF_PREVIEW_PAGE);
            if (seq !== _fbPreviewSeq) return;
            const baseViewport = page.getViewport({scale: 1.0});
            const scale = Math.min(1.0, FB_PDF_PREVIEW_WIDTH / baseViewport.width);
            const viewport = page.getViewport({scale});
            canvas.width = Math.round(viewport.width);
            canvas.height = Math.round(viewport.height);
            const ctx = canvas.getContext('2d');
            await page.render({canvasContext: ctx, viewport}).promise;
            if (seq !== _fbPreviewSeq) return;
            // 3) Persist render to cache — kicked off immediately on a
            //    background path (Rust side runs SQLite write on the
            //    blocking pool). Not awaited, so this function returns as
            //    soon as the visible canvas is painted; the user never
            //    waits on the cache write. Failures route to console.warn
            //    (not silent) so future serialization bugs surface.
            _fbCanvasToPngBytes(canvas).then((pngBytes) => {
                if (!pngBytes) return;
                return window.vstUpdater.pdfPreviewSet(
                    filePath, FB_PDF_PREVIEW_PAGE, FB_PDF_PREVIEW_WIDTH, pngBytes,
                );
            }).catch((err) => console.warn('pdf preview cache write failed:', err));
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            const msg = (err && (err.message || err)) ? String(err.message || err) : 'PDF preview unavailable';
            body.innerHTML = `<div class="fb-preview-empty">${escapeHtml(msg)}</div>${metaHtml}`;
        }
        return;
    }

    // No type-specific preview path matched → fall through to a hex /
    // binary dump (read the first 4 KiB so the pane shows something
    // useful even for opaque formats). Hex view is ALSO useful for files
    // that DO have a typed preview but whose type the user wants to
    // verify (e.g. checking a PDF magic byte). For now we only show it
    // as the fallback; a Cmd+Opt+H "force hex" toggle could be added
    // later if asked.
    try {
        // `fsReadHeadBytes` returns a raw ArrayBuffer of the first
        // 4 KiB (server-side bounded read, doesn't ship the whole file
        // — important for the GB-scale ALS / DAW project case the user
        // might preview-pane by accident). UTF-8-lossy `fsReadHead`
        // would scramble 0x80+ bytes into U+FFFD; this one preserves
        // every byte for the hex view.
        const ab = await window.vstUpdater.fsReadHeadBytes(filePath, 4096);
        if (seq !== _fbPreviewSeq) return;
        const u8 = new Uint8Array(ab || new ArrayBuffer(0));
        body.innerHTML = `<pre class="fb-preview-hex">${_fbBytesToHexDump(u8, 4096)}</pre>${metaHtml}`;
    } catch (err) {
        if (seq !== _fbPreviewSeq) return;
        body.innerHTML = `<div class="fb-preview-empty">Could not read file (${escapeHtml(String(err && err.message ? err.message : err))})</div>${metaHtml}`;
    }
}

/**
 * Format a byte buffer as a `xxd`-style hex+ASCII dump:
 *   00000000  48 65 6c 6c 6f 20 57 6f  72 6c 64 0a              |Hello World.|
 * Caps at `maxBytes` so a huge file can't pin the preview pane (caller
 * also reads a bounded slice on the Rust side).
 */
function _fbBytesToHexDump(u8, maxBytes) {
    const cap = Math.min(u8.length, maxBytes || 4096);
    const lines = [];
    for (let off = 0; off < cap; off += 16) {
        const end = Math.min(off + 16, cap);
        // Address column — 8 hex digits, zero-padded.
        const addr = off.toString(16).padStart(8, '0');
        // Hex bytes (16 per row, split into 2 groups of 8 for readability).
        let hex = '';
        for (let i = 0; i < 16; i++) {
            if (i === 8) hex += ' ';
            if (off + i < end) {
                hex += u8[off + i].toString(16).padStart(2, '0') + ' ';
            } else {
                hex += '   '; // pad missing bytes so the ASCII column lines up
            }
        }
        // ASCII pane — printable ASCII as-is, others as '.'.
        let ascii = '';
        for (let i = 0; i < end - off; i++) {
            const c = u8[off + i];
            ascii += (c >= 0x20 && c < 0x7f) ? String.fromCharCode(c) : '.';
        }
        lines.push(`${addr}  ${hex.trimEnd()}  |${escapeHtml(ascii)}|`);
    }
    if (u8.length > cap) {
        lines.push(`… ${u8.length - cap} more bytes`);
    }
    return lines.join('\n');
}

// ── Quick-look overlay (Space key → big preview modal) ──
// Builds and shows a centered modal with file metadata. Dismissed by Esc,
// click-outside, or pressing Space again. For audio files, embeds a larger
// waveform that reuses the existing `drawMiniWaveform` helper. For other
// types, shows path / size / dates only — richer per-type preview
// (image/PDF body) is the job of the dedicated preview pane task.
function _ensureQuickLookOverlay() {
    if (typeof document === 'undefined') return null;
    let overlay = document.getElementById('fbQuickLook');
    if (overlay) return overlay;
    overlay = document.createElement('div');
    overlay.id = 'fbQuickLook';
    overlay.className = 'fb-quicklook fb-hidden';
    overlay.innerHTML = `
        <div class="fb-quicklook-card">
            <div class="fb-quicklook-header">
                <span class="fb-quicklook-title" id="fbQuickLookTitle"></span>
                <button class="fb-quicklook-close" data-action="fbQuickLookClose" title="Close (Esc)">&times;</button>
            </div>
            <div class="fb-quicklook-body" id="fbQuickLookBody"></div>
        </div>
    `;
    document.body.appendChild(overlay);
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay || e.target.closest('[data-action="fbQuickLookClose"]')) hideQuickLook();
    });
    // Esc closes the overlay. Capture phase + stopImmediatePropagation so
    // it wins over the global shortcuts.js _handleEscape (which doesn't
    // know about .fb-quicklook). Only fires while the overlay is visible.
    document.addEventListener('keydown', (e) => {
        if (e.key !== 'Escape') return;
        if (overlay.classList.contains('fb-hidden')) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        hideQuickLook();
    }, true);
    return overlay;
}

async function showQuickLook(filePath) {
    if (typeof document === 'undefined' || !filePath) return;
    const overlay = _ensureQuickLookOverlay();
    if (!overlay) return;
    const title = document.getElementById('fbQuickLookTitle');
    const body = document.getElementById('fbQuickLookBody');
    const name = filePath.split('/').pop();
    const ext = (name.split('.').pop() || '').toLowerCase();
    if (title) title.textContent = name;
    if (!body) {
        overlay.classList.remove('fb-hidden');
        return;
    }
    const isAudio = typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
    const isPdf = ext === 'pdf';
    const wfHtml = isAudio
        ? `<canvas class="fb-quicklook-wf" id="fbQuickLookWf" data-wf-path="${escapeHtml(filePath)}" width="800" height="120"></canvas>`
        : '';
    // Wrap canvas in a `<div>` so the spinner has a positioned parent.
    // Canvas starts hidden (`fb-hidden`) — only unhidden after paint, so
    // there's no flash of white box before render. Spinner is the
    // canonical `.spinner` element (same one ALS scan progress uses) so
    // every loading state in the app looks the same.
    const pdfHtml = isPdf
        ? `<div class="fb-quicklook-pdf-wrap" id="fbQuickLookPdfWrap">`
            + `<span class="spinner fb-quicklook-pdf-spinner fb-hidden" id="fbQuickLookPdfSpinner" aria-hidden="true"></span>`
            + `<canvas class="fb-quicklook-pdf fb-hidden" id="fbQuickLookPdf"></canvas>`
            + `</div>`
        : '';
    body.innerHTML = `
        ${wfHtml}
        ${pdfHtml}
        <div class="fb-quicklook-meta">
            <div class="fb-quicklook-row"><span class="fb-quicklook-label">Path</span><span class="fb-quicklook-val">${escapeHtml(filePath)}</span></div>
            <div class="fb-quicklook-row"><span class="fb-quicklook-label">Type</span><span class="fb-quicklook-val">${escapeHtml(ext || '—')}</span></div>
        </div>
        <div class="fb-quicklook-hint">Press Esc to close</div>
    `;
    overlay.classList.remove('fb-hidden');
    if (isAudio && typeof drawMiniWaveform === 'function') {
        const canvas = document.getElementById('fbQuickLookWf');
        if (canvas) drawMiniWaveform(canvas, filePath);
        return;
    }
    if (isPdf) {
        // Big render — 600 px width. Same canonical width as preview pane
        // + PDF-tab thumb, so all three consumers share one cache row per
        // PDF (a thumb render pre-populates the Quick-look cache too).
        const canvas = document.getElementById('fbQuickLookPdf');
        const spinner = document.getElementById('fbQuickLookPdfSpinner');
        if (!canvas) return;
        const QL_WIDTH = 600;
        // Canvas starts hidden until paint completes — no flash of empty
        // white box. Spinner starts hidden too; only unhide if the cache
        // misses and we have to actually render.
        const showCanvas = () => canvas.classList.remove('fb-hidden');
        const showSpinner = () => { if (spinner) spinner.classList.remove('fb-hidden'); };
        const hideSpinner = () => { if (spinner) spinner.classList.add('fb-hidden'); };
        // Try cache FIRST without showing the spinner — hits should be
        // invisible (no flash of loading state).
        try {
            const cached = await window.vstUpdater.pdfPreviewGet(filePath, 1, QL_WIDTH);
            // Raw ArrayBuffer — check byteLength (truthiness is always true).
            if (cached && cached.byteLength > 0 && typeof _fbPaintPngBytesIntoCanvas === 'function') {
                await _fbPaintPngBytesIntoCanvas(canvas, cached);
                showCanvas();
                return;
            }
        } catch (_) { /* fall through */ }
        // Confirmed cache miss → show spinner, now do the slow render.
        showSpinner();
        try {
            const bytes = await window.vstUpdater.fsReadFileBytes(filePath);
            const pdfjs = await loadPdfJs();
            const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
            const pdf = await pdfjs.getDocument({data: u8}).promise;
            const page = await pdf.getPage(1);
            const baseViewport = page.getViewport({scale: 1.0});
            const scale = Math.min(1.0, QL_WIDTH / baseViewport.width);
            const viewport = page.getViewport({scale});
            canvas.width = Math.round(viewport.width);
            canvas.height = Math.round(viewport.height);
            await page.render({canvasContext: canvas.getContext('2d'), viewport}).promise;
            hideSpinner();
            showCanvas();
            // Fire-and-forget cache write — the modal's visible content
            // doesn't depend on the write, so don't block the JS thread
            // waiting on the IPC roundtrip. Rust writes SQLite on its
            // blocking pool; we just kick it off and surface any failure
            // via console.warn.
            _fbCanvasToPngBytes(canvas).then((png) => {
                if (!png) return;
                return window.vstUpdater.pdfPreviewSet(filePath, 1, QL_WIDTH, png);
            }).catch((err) => console.warn('quicklook pdf cache write failed:', err));
        } catch (_) {
            hideSpinner();
            // Leave canvas hidden on failure; the metadata + path are still
            // visible in the modal body so user knows what failed.
        }
    }
}

function hideQuickLook() {
    if (typeof document === 'undefined') return;
    const overlay = document.getElementById('fbQuickLook');
    if (overlay) overlay.classList.add('fb-hidden');
}

function isQuickLookVisible() {
    if (typeof document === 'undefined') return false;
    const overlay = document.getElementById('fbQuickLook');
    return !!(overlay && !overlay.classList.contains('fb-hidden'));
}

// Exposed on `window` so the PDF inventory tab (pdf.js) can trigger Quick-look
// via Cmd+I / Space for the focused row.
if (typeof window !== 'undefined') {
    window.showQuickLook = showQuickLook;
    window.hideQuickLook = hideQuickLook;
    window.isQuickLookVisible = isQuickLookVisible;
}

// ── Inline rename (F2 on focused row, or via context menu) ──
// Replaces the row's `.file-name` text node with a borderless `<input>`,
// pre-filled and selected. Enter commits via `rename_file` IPC; Esc cancels;
// blur cancels (treats unintentional click-away as cancel). After successful
// rename, reload the directory so the row's path / placement update.
async function _commitFileRename(oldPath, input, row) {
    const next = (input.value || '').trim();
    const dir = oldPath.replace(/\/[^/]+$/, '');
    const newPath = `${dir}/${next}`;
    if (!next || next === oldPath.split('/').pop()) {
        _cancelFileRename(input, row);
        return;
    }
    try {
        await window.vstUpdater.renameFile(oldPath, newPath);
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `renamed to ${next}`}));
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
        _cancelFileRename(input, row);
    }
}

function _cancelFileRename(input, row) {
    if (!row || !input) return;
    const nameCell = row.querySelector('.file-name');
    if (nameCell && nameCell._origHtml != null) {
        nameCell.innerHTML = nameCell._origHtml;
        delete nameCell._origHtml;
    }
}

function startFileRename(row) {
    if (!row) return;
    const oldPath = row.dataset.filePath;
    if (!oldPath) return;
    const nameCell = row.querySelector('.file-name');
    if (!nameCell || nameCell.querySelector('input.fb-rename-input')) return;
    const baseName = oldPath.split('/').pop();
    // Stash existing HTML so cancel can restore the highlight / badges / note pin.
    nameCell._origHtml = nameCell.innerHTML;
    nameCell.innerHTML = `<input class="fb-rename-input" type="text" autocomplete="off" autocorrect="off" spellcheck="false">`;
    const input = nameCell.querySelector('input.fb-rename-input');
    input.value = baseName;
    input.focus();
    // Select everything except the extension (matches Finder / Explorer UX).
    const dot = baseName.lastIndexOf('.');
    if (dot > 0) input.setSelectionRange(0, dot);
    else input.select();
    let done = false;
    const finish = (commit) => {
        if (done) return;
        done = true;
        if (commit) _commitFileRename(oldPath, input, row);
        else _cancelFileRename(input, row);
    };
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') { e.preventDefault(); finish(true); }
        else if (e.key === 'Escape') { e.preventDefault(); finish(false); }
        e.stopPropagation();
    });
    input.addEventListener('blur', () => { finish(false); });
}

// ── New Folder (button + Cmd+Shift+N) ──
// Uses the in-app `promptAction()` modal — native `window.prompt()` is
// silently dismissed in Tauri WKWebView release builds (same class of
// bug as `window.confirm()`, see file-browser delete path).
async function fileBrowserNewFolder() {
    if (!_fileBrowserPath) return;
    const name = typeof promptAction === 'function'
        ? await promptAction('New folder name:', 'untitled folder')
        : window.prompt('New folder name:', 'untitled folder');
    if (!name) return;
    const cleaned = name.trim();
    if (!cleaned) return;
    const newPath = `${_fileBrowserPath}/${cleaned}`;
    try {
        await window.vstUpdater.fsCreateDir(newPath);
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `created ${cleaned}`}));
        loadDirectory(_fileBrowserPath);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
    }
}

// ── New File (empty-space right-click → New File) ──
// Mirrors `fileBrowserNewFolder` but creates a zero-byte file via
// `fs_create_file` (uses `create_new` so an existing path errors instead
// of being silently truncated). Same `promptAction()` reasoning above.
async function fileBrowserNewFile() {
    if (!_fileBrowserPath) return;
    const name = typeof promptAction === 'function'
        ? await promptAction('New file name:', 'untitled.txt')
        : window.prompt('New file name:', 'untitled.txt');
    if (!name) return;
    const cleaned = name.trim();
    if (!cleaned) return;
    const newPath = `${_fileBrowserPath}/${cleaned}`;
    try {
        await window.vstUpdater.fsCreateFile(newPath);
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `created ${cleaned}`}));
        loadDirectory(_fileBrowserPath);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
    }
}

// ── File clipboard (Cmd+C / Cmd+X mark, Cmd+V paste) ──
// Pure JS state — Finder-style file ops without OS-clipboard plumbing
// (Tauri's WebView clipboard API is text-only). Each marked file
// preserves its full path; paste resolves relative to current dir and
// errors on name collision (caller can rename + retry).
//   _fbClipboard.mode: 'copy' (preserves source) | 'cut' (moves)
//   _fbClipboard.paths: array of absolute paths
window._fbClipboard = window._fbClipboard || {mode: null, paths: []};

function fileBrowserMarkClipboard(mode, paths) {
    if (!Array.isArray(paths) || paths.length === 0) return;
    window._fbClipboard = {mode, paths: paths.slice()};
    if (typeof showToast === 'function') {
        const verb = mode === 'cut' ? 'cut' : 'copied';
        const target = paths.length === 1 ? paths[0].split('/').pop() : `${paths.length} items`;
        showToast(toastFmt('toast.deleted_name', {name: `${verb} ${target} — paste in target folder`}));
    }
}

async function fileBrowserPasteClipboard() {
    if (!_fileBrowserPath) return;
    const clip = window._fbClipboard;
    if (!clip || !clip.paths || clip.paths.length === 0) return;
    let ok = 0, fail = 0;
    for (const src of clip.paths) {
        const base = src.split('/').pop();
        // Same dir + copy → auto-suffix via fsDuplicate would be ideal,
        // but `fs_copy_path` errors on collision — derive a non-colliding
        // dest here. Cut into the same dir is a no-op (Finder behavior).
        let dest = `${_fileBrowserPath}/${base}`;
        if (src === dest) continue;
        try {
            if (clip.mode === 'cut') {
                await window.vstUpdater.renameFile(src, dest);
            } else {
                // Probe up to 10 numbered suffixes for copies into the
                // same parent (matches `fs_duplicate` behavior).
                let i = 0;
                while (true) {
                    try {
                        await window.vstUpdater.fsCopyPath(src, dest);
                        break;
                    } catch (err) {
                        const m = String(err && err.message ? err.message : err);
                        if (!m.includes('already exists') || i >= 10) throw err;
                        i++;
                        const dot = base.lastIndexOf('.');
                        const stem = dot > 0 ? base.slice(0, dot) : base;
                        const ext = dot > 0 ? base.slice(dot) : '';
                        dest = `${_fileBrowserPath}/${stem} ${i + 1}${ext}`;
                    }
                }
            }
            ok++;
        } catch (_) {
            fail++;
        }
    }
    if (clip.mode === 'cut') {
        // Cut empties the clipboard once pasted — Finder-equivalent.
        window._fbClipboard = {mode: null, paths: []};
    }
    if (typeof showToast === 'function') {
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `pasted ${ok} item${ok === 1 ? '' : 's'}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} item${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    loadDirectory(_fileBrowserPath);
}

// ── Get Info / Properties modal ──
// Themed two-column grid (matches `.fb-info-grid` CSS). Built lazily on
// open, removed on close — single instance, no state to leak.
function _fmtBytes(n) {
    if (!n && n !== 0) return '—';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let i = 0, v = n;
    while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
    return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
function _fmtTs(ms) {
    if (!ms) return '—';
    try { return new Date(ms).toLocaleString(); }
    catch (_) { return String(ms); }
}
async function fileBrowserShowInfo(path) {
    document.getElementById('appFileInfoModal')?.remove();
    let info = null, errMsg = null;
    try { info = await window.vstUpdater.fsGetInfo(path); }
    catch (err) { errMsg = String(err && err.message ? err.message : err); }
    const name = info ? info.name : (path.split('/').pop() || path);
    const rows = info
        ? [
            ['Path', info.path],
            ['Kind', info.kind + (info.isSymlink ? ' (symlink)' : '')],
            ...(info.symlinkTarget ? [['Target', info.symlinkTarget]] : []),
            ['Size', info.kind === 'dir' ? `${_fmtBytes(info.size)} (recursive)` : _fmtBytes(info.size)],
            ...(info.itemCount != null ? [['Items', `${info.itemCount}${info.itemCount >= 100000 ? '+ (cap)' : ''}`]] : []),
            ['Modified', _fmtTs(info.mtimeMs)],
            ['Created', _fmtTs(info.ctimeMs)],
            ['Accessed', _fmtTs(info.atimeMs)],
            ['Permissions', info.modeString ? `${info.modeString}  (${info.modeOctal})` : (info.isReadonly ? 'read-only' : 'read-write')],
            ...(info.uid != null ? [['Owner / Group', `uid=${info.uid}  gid=${info.gid}`]] : []),
            ['xattrs', 'loading…'],
        ]
        : [];
    const gridHtml = info
        ? `<div class="fb-info-grid">${rows.map(([k, v]) => `<div class="fb-info-key">${escapeHtml(k)}</div><div class="fb-info-val">${escapeHtml(String(v))}</div>`).join('')}</div>`
        : `<p class="app-confirm-message">${escapeHtml(errMsg || 'Unable to read file info')}</p>`;
    const html = `<div class="modal-overlay modal-visible" id="appFileInfoModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Get Info — ${escapeHtml(name)}</h2>
        <button type="button" class="modal-close" data-app-info="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        ${gridHtml}
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-info="close">Close</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appFileInfoModal');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        if (e.target.closest('[data-app-info="close"]') || e.target === modal) close();
    });
    // Lazy-load xattrs after the modal paints. We placed a "loading…"
    // placeholder; patch the value cell in place when the IPC returns.
    if (info) {
        window.vstUpdater.fsXattrs(path).then((xattrs) => {
            if (!modal || !modal.isConnected) return;
            // The xattrs row is always the LAST .fb-info-val in the modal —
            // safer than indexing because the row count varies (symlink
            // target only present sometimes, etc.).
            const cells = modal.querySelectorAll('.fb-info-val');
            const cell = cells[cells.length - 1];
            if (!cell) return;
            if (!xattrs || xattrs.length === 0) {
                cell.textContent = '(none)';
                return;
            }
            cell.innerHTML = xattrs
                .map((x) => `${escapeHtml(x.name)} <span style="color:var(--text-dim)">(${x.size} B)</span>`)
                .join('<br>');
        }).catch(() => {
            const cells = modal?.querySelectorAll('.fb-info-val');
            const cell = cells && cells[cells.length - 1];
            if (cell) cell.textContent = '(unavailable)';
        });
    }
}

// ── New Folder with Selection (Finder: bundle selected → subfolder) ──
async function fileBrowserNewFolderWithSelection(paths) {
    if (!_fileBrowserPath || !Array.isArray(paths) || paths.length === 0) return;
    const dflt = paths.length === 1 ? 'New Folder With Item' : `New Folder With ${paths.length} Items`;
    const name = typeof promptAction === 'function'
        ? await promptAction('Folder name:', dflt)
        : window.prompt('Folder name:', dflt);
    if (!name) return;
    const cleaned = name.trim();
    if (!cleaned) return;
    const newDir = `${_fileBrowserPath}/${cleaned}`;
    try {
        await window.vstUpdater.fsCreateDir(newDir);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
        return;
    }
    let ok = 0, fail = 0;
    for (const p of paths) {
        const base = p.split('/').pop();
        try {
            await window.vstUpdater.renameFile(p, `${newDir}/${base}`);
            ok++;
        } catch (_) {
            fail++;
        }
    }
    if (typeof showToast === 'function') {
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `moved ${ok} item${ok === 1 ? '' : 's'} into ${cleaned}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} move${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    loadDirectory(_fileBrowserPath);
}

// ── Hash (SHA-256) — single file or selection ──
// Streams via the Rust `fs_hash` command (64 KiB chunks; bounded RAM).
// Multi-file shows per-row digests + a Copy All button.
async function fileBrowserShowHashModal(paths) {
    if (!Array.isArray(paths) || paths.length === 0) return;
    document.getElementById('appHashModal')?.remove();
    // Render an empty modal first so the user gets immediate visual
    // feedback that the action kicked off (large files take seconds).
    const rowSkel = paths.map((p) => {
        const name = p.split('/').pop();
        return `<div class="fb-info-grid"><div class="fb-info-key">${escapeHtml(name)}</div><div class="fb-info-val" data-hash-path="${escapeHtml(p)}">hashing…</div></div>`;
    }).join('');
    const html = `<div class="modal-overlay modal-visible" id="appHashModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>SHA-256 ${paths.length === 1 ? '' : `(${paths.length} files)`}</h2>
        <button type="button" class="modal-close" data-app-hash="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        ${rowSkel}
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-hash="copy-all">Copy All</button>
          <button type="button" class="btn btn-primary" data-app-hash="close">Close</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appHashModal');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', async (e) => {
        const btn = e.target.closest('[data-app-hash]');
        if (!btn) { if (e.target === modal) close(); return; }
        if (btn.dataset.appHash === 'close') { close(); return; }
        if (btn.dataset.appHash === 'copy-all') {
            const lines = [...modal.querySelectorAll('[data-hash-path]')].map((el) => {
                const p = el.dataset.hashPath;
                const v = el.textContent.trim();
                return `${v}  ${p}`;
            }).join('\n');
            try { await navigator.clipboard.writeText(lines); showToast(toastFmt('toast.copied_clipboard')); }
            catch (_) { showToast(toastFmt('toast.failed', {err: 'clipboard'}), 4000, 'error'); }
        }
    });
    // Hash one at a time (serial keeps disk + IPC pressure low; per-file
    // streaming inside Rust is already chunked).
    for (const p of paths) {
        try {
            const res = await window.vstUpdater.fsHash(p, ['sha256']);
            const cell = modal?.querySelector(`[data-hash-path="${CSS.escape(p)}"]`);
            if (cell) cell.textContent = (res.digests && res.digests.sha256) || '(no digest)';
        } catch (err) {
            const cell = modal?.querySelector(`[data-hash-path="${CSS.escape(p)}"]`);
            if (cell) cell.textContent = `error: ${err && err.message ? err.message : err}`;
        }
        if (!document.getElementById('appHashModal')) break; // user closed
    }
}

// Bulk chmod — same modal contract but applies the mode to every path
// in the selection. Pre-fills with the mode of the FIRST path (so
// homogeneous selections show their existing value).
async function fileBrowserShowBulkChmodModal(paths) {
    if (!Array.isArray(paths) || paths.length === 0) return;
    if (paths.length === 1) return fileBrowserShowChmodModal(paths[0]);
    document.getElementById('appChmodModal')?.remove();
    let curMode = '';
    try {
        const info = await window.vstUpdater.fsGetInfo(paths[0]);
        curMode = info.modeOctal || '';
    } catch (_) { /* ignore */ }
    const html = `<div class="modal-overlay modal-visible" id="appChmodModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Permissions — ${paths.length} files</h2>
        <button type="button" class="modal-close" data-app-chmod="cancel" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message">Octal mode (e.g. 0644, 755). Applies to ALL ${paths.length} selected paths. First path's current mode: <code>${escapeHtml(curMode || '?')}</code></p>
        <input type="text" id="appChmodInput" class="app-prompt-input" value="${escapeHtml(curMode)}" />
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-chmod="cancel">Cancel</button>
          <button type="button" class="btn btn-primary" data-app-chmod="ok">Apply to all</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appChmodModal');
    const input = document.getElementById('appChmodInput');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const apply = async () => {
        const mode = (input.value || '').trim();
        if (!mode) { close(); return; }
        let ok = 0, fail = 0;
        for (const p of paths) {
            try { await window.vstUpdater.fsChmod(p, mode); ok++; }
            catch (_) { fail++; }
        }
        if (typeof showToast === 'function') {
            if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `chmod ${mode} on ${ok} file${ok === 1 ? '' : 's'}`}));
            if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} chmod${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
        }
        close();
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
    };
    const esc = (e) => {
        if (e.key === 'Escape') { e.preventDefault(); close(); }
        else if (e.key === 'Enter' && document.activeElement === input) { e.preventDefault(); apply(); }
    };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-app-chmod]');
        if (btn) { if (btn.dataset.appChmod === 'ok') apply(); else close(); return; }
        if (e.target === modal) close();
    });
    requestAnimationFrame(() => { input?.focus(); input?.select(); });
}

// ── Chmod modal (Unix) ──
// Text entry for octal mode; checkbox grid would be nice but the modal
// real estate is small and power users prefer typing 644 / 755 anyway.
async function fileBrowserShowChmodModal(path) {
    document.getElementById('appChmodModal')?.remove();
    let curMode = '';
    try {
        const info = await window.vstUpdater.fsGetInfo(path);
        curMode = info.modeOctal || '';
    } catch (_) { /* fall through with empty */ }
    const name = path.split('/').pop();
    const html = `<div class="modal-overlay modal-visible" id="appChmodModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Permissions — ${escapeHtml(name)}</h2>
        <button type="button" class="modal-close" data-app-chmod="cancel" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message">Octal mode (e.g. 0644, 755). Current: <code>${escapeHtml(curMode || '?')}</code></p>
        <input type="text" id="appChmodInput" class="app-prompt-input" value="${escapeHtml(curMode)}" />
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-chmod="cancel">Cancel</button>
          <button type="button" class="btn btn-primary" data-app-chmod="ok">Apply</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appChmodModal');
    const input = document.getElementById('appChmodInput');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const apply = async () => {
        const mode = (input.value || '').trim();
        if (!mode) { close(); return; }
        try {
            await window.vstUpdater.fsChmod(path, mode);
            showToast(toastFmt('toast.deleted_name', {name: `chmod ${mode} ${name}`}));
            close();
            if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
        } catch (err) {
            showToast(toastFmt('toast.failed', {err: err && err.message ? err.message : err}), 4000, 'error');
        }
    };
    const esc = (e) => {
        if (e.key === 'Escape') { e.preventDefault(); close(); }
        else if (e.key === 'Enter' && document.activeElement === input) { e.preventDefault(); apply(); }
    };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-app-chmod]');
        if (btn) { if (btn.dataset.appChmod === 'ok') apply(); else close(); return; }
        if (e.target === modal) close();
    });
    requestAnimationFrame(() => { input?.focus(); input?.select(); });
}

// ── Pattern Select (glob → check matching rows) ──
// Glob → RegExp converter: `*` → `.*`, `?` → `.`, anything else escaped.
function _fbGlobToRegex(glob) {
    const re = glob
        .split('')
        .map((c) => (c === '*' ? '.*' : c === '?' ? '.' : c.replace(/[.+^${}()|[\]\\]/g, '\\$&')))
        .join('');
    return new RegExp('^' + re + '$', 'i');
}
async function fileBrowserPatternSelect() {
    if (!_fileBrowserPath) return;
    const glob = typeof promptAction === 'function'
        ? await promptAction('Glob pattern (e.g. *.wav, song-??.mp3):', '*.*')
        : window.prompt('Glob pattern (e.g. *.wav):', '*.*');
    if (!glob) return;
    const cleaned = glob.trim();
    if (!cleaned) return;
    let re;
    try { re = _fbGlobToRegex(cleaned); }
    catch (e) { showToast(toastFmt('toast.failed', {err: 'Invalid pattern'}), 4000, 'error'); return; }
    if (typeof clearFileSelection === 'function') clearFileSelection();
    let matched = 0;
    document.querySelectorAll('.file-row-cb').forEach((cb) => {
        const path = cb.dataset.fbCb;
        if (!path) return;
        const name = path.split('/').pop();
        if (re.test(name)) {
            cb.checked = true;
            _fileSelected.add(path);
            _setFileRowSelectedClass(path, true);
            matched++;
        }
    });
    updateFileBulkBar();
    if (typeof showToast === 'function') {
        showToast(toastFmt('toast.deleted_name', {name: `selected ${matched} matching ${cleaned}`}));
    }
}

// ── Bulk archive ops ──
// Compress selection into one .zip; Extract every selected .zip into a
// sibling dir. Both serialize on the Rust side via fsCompress/fsExtract;
// the JS side just orchestrates name selection + sequential awaits.
async function fileBrowserBulkCompress(paths) {
    if (!_fileBrowserPath || !Array.isArray(paths) || paths.length === 0) return;
    const dflt = paths.length === 1
        ? `${(paths[0].split('/').pop() || 'Archive')}.zip`
        : `Archive ${new Date().toISOString().slice(0, 10)}.zip`;
    const name = typeof promptAction === 'function'
        ? await promptAction(`Compress ${paths.length} item${paths.length === 1 ? '' : 's'} into:`, dflt)
        : window.prompt('Archive name:', dflt);
    if (!name) return;
    const cleaned = name.trim().replace(/\.zip$/i, '');
    if (!cleaned) return;
    let archive = `${_fileBrowserPath}/${cleaned}.zip`;
    for (let i = 0; i < 10; i++) {
        try {
            await window.vstUpdater.fsCompress(paths, archive);
            showToast(toastFmt('toast.deleted_name', {name: `compressed → ${archive.split('/').pop()}`}));
            loadDirectory(_fileBrowserPath);
            return;
        } catch (err) {
            const m = String(err && err.message ? err.message : err);
            if (!m.includes('already exists')) {
                showToast(toastFmt('toast.failed', {err: m}), 4000, 'error');
                return;
            }
            archive = `${_fileBrowserPath}/${cleaned} ${i + 2}.zip`;
        }
    }
}
async function fileBrowserBulkExtract(paths) {
    const ARCHIVE_RE = /\.(zip|tar|tar\.gz|tgz|7z)$/i;
    const STEM_STRIP = /\.(tar\.gz|tgz|tar|zip|7z)$/i;
    const archives = (paths || []).filter((p) => ARCHIVE_RE.test(p));
    if (archives.length === 0) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'No archive files selected (.zip / .tar / .tar.gz)'}), 4000, 'error');
        return;
    }
    let ok = 0, fail = 0;
    for (const p of archives) {
        const parent = p.replace(/\/[^/]+$/, '');
        const base = p.split('/').pop();
        const stem = base.replace(STEM_STRIP, '');
        let dest = `${parent}/${stem}`;
        let placed = false;
        for (let i = 0; i < 10; i++) {
            try {
                await window.vstUpdater.fsExtract(p, dest);
                placed = true;
                break;
            } catch (err) {
                const m = String(err && err.message ? err.message : err);
                if (!m.includes('already exists')) break;
                dest = `${parent}/${stem} ${i + 2}`;
            }
        }
        if (placed) ok++; else fail++;
    }
    if (typeof showToast === 'function') {
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `extracted ${ok} archive${ok === 1 ? '' : 's'}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} extraction${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
}

// ── Find by Content (grep current folder) ──
// Modal with text input + case toggle + results list. Click result →
// navigate to its parent dir + select the row.
async function fileBrowserShowGrepModal() {
    if (!_fileBrowserPath) return;
    document.getElementById('appGrepModal')?.remove();
    const html = `<div class="modal-overlay modal-visible" id="appGrepModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Find in Files — ${escapeHtml(_fileBrowserPath.split('/').filter(Boolean).pop() || _fileBrowserPath)}</h2>
        <button type="button" class="modal-close" data-app-grep="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message">Substring search. Skips binaries, dotdirs, and files &gt; 4 MiB.</p>
        <input type="text" id="appGrepInput" class="app-prompt-input" placeholder="search text…" />
        <label class="app-confirm-message" style="display:flex;gap:8px;align-items:center;margin-top:8px;">
          <input type="checkbox" id="appGrepCase"> Case insensitive
        </label>
        <div id="appGrepResults" class="fb-info-grid" style="max-height:320px;overflow-y:auto;margin-top:12px;"></div>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-grep="close">Close</button>
          <button type="button" class="btn btn-primary" data-app-grep="run">Search</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appGrepModal');
    const input = document.getElementById('appGrepInput');
    const cs = document.getElementById('appGrepCase');
    const results = document.getElementById('appGrepResults');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const run = async () => {
        const q = (input.value || '').trim();
        if (!q) return;
        results.innerHTML = '<div class="fb-info-val">searching…</div>';
        try {
            const matches = await window.vstUpdater.fsGrep(_fileBrowserPath, q, cs.checked, 500);
            if (!matches || matches.length === 0) {
                results.innerHTML = '<div class="fb-info-val">no matches</div>';
                return;
            }
            results.innerHTML = matches.map((m) => {
                const name = m.path.split('/').pop();
                return `<div class="fb-info-key">${escapeHtml(name)}:${m.line}</div><div class="fb-info-val" style="cursor:pointer;" data-grep-path="${escapeHtml(m.path)}">${escapeHtml(m.text)}</div>`;
            }).join('');
        } catch (err) {
            results.innerHTML = `<div class="fb-info-val">error: ${escapeHtml(String(err && err.message ? err.message : err))}</div>`;
        }
    };
    const esc = (e) => {
        if (e.key === 'Escape') { e.preventDefault(); close(); }
        else if (e.key === 'Enter' && document.activeElement === input) { e.preventDefault(); run(); }
    };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        const path = e.target.closest('[data-grep-path]')?.dataset.grepPath;
        if (path) {
            const parent = path.replace(/\/[^/]+$/, '');
            close();
            loadDirectory(parent);
            return;
        }
        const btn = e.target.closest('[data-app-grep]');
        if (btn) { if (btn.dataset.appGrep === 'run') run(); else close(); return; }
        if (e.target === modal) close();
    });
    requestAnimationFrame(() => input?.focus());
}

// ── Tree-view sidebar (Cmd+B, persisted) ──
// Power-user navigation: lazy-loaded collapsible folder tree.
//   _fbTreeExpanded: Set<path>  — every path that's currently expanded.
//   _fbTreeChildren: Map<path, [{name, path}]> — cached subdir lists.
// Each render walks from a small set of roots (favorites + current dir
// ancestors) and emits visible nodes via flat HTML — no virtual DOM.
let _fbTreeExpanded = (() => {
    try {
        const raw = typeof prefs !== 'undefined' ? prefs.getItem('fileBrowserTreeExpanded') : null;
        return new Set(raw ? JSON.parse(raw) : []);
    } catch (_) { return new Set(); }
})();
const _fbTreeChildren = new Map();
function _fbTreePersist() {
    try {
        if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserTreeExpanded', JSON.stringify([..._fbTreeExpanded]));
    } catch (_) { /* ignore */ }
}

function fileBrowserToggleTreeSidebar(forceState) {
    const sidebar = document.getElementById('fbTreeSidebar');
    if (!sidebar) return;
    const visible = !sidebar.classList.contains('fb-hidden');
    const next = typeof forceState === 'boolean' ? forceState : !visible;
    sidebar.classList.toggle('fb-hidden', !next);
    try {
        if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserTreeVisible', next ? '1' : '0');
    } catch (_) { /* ignore */ }
    if (next) renderFileBrowserTree();
}

async function _fbTreeFetchChildren(path) {
    if (_fbTreeChildren.has(path)) return _fbTreeChildren.get(path);
    try {
        const subs = await window.vstUpdater.fsListSubdirs(path, _fbShowHidden);
        _fbTreeChildren.set(path, subs || []);
        return subs || [];
    } catch (_) {
        _fbTreeChildren.set(path, []);
        return [];
    }
}

// Roots shown at the top of the tree. Pulls favorite dirs + home if
// not already a favorite. User can expand each to drill down.
function _fbTreeRoots() {
    const out = [];
    const seen = new Set();
    if (typeof getFavDirs === 'function') {
        for (const d of getFavDirs()) {
            if (!d || !d.path || seen.has(d.path)) continue;
            seen.add(d.path);
            out.push({name: d.name || d.path.split('/').pop() || d.path, path: d.path});
        }
    }
    // Home + Root always available.
    const home = (typeof _fbHomePath === 'string' && _fbHomePath) ? _fbHomePath : null;
    if (home && !seen.has(home)) { seen.add(home); out.push({name: 'Home', path: home}); }
    if (!seen.has('/')) out.push({name: '/', path: '/'});
    return out;
}

let _fbHomePath = null;
(async () => {
    try {
        if (window.vstUpdater && typeof window.vstUpdater.getHomeDir === 'function') {
            _fbHomePath = await window.vstUpdater.getHomeDir();
        }
    } catch (_) { /* ignore */ }
})();

async function renderFileBrowserTree() {
    const body = document.getElementById('fbTreeBody');
    if (!body) return;
    const roots = _fbTreeRoots();
    // Pre-warm: for every expanded root + its expanded descendants we
    // need children loaded. Walk depth-first, fetching as we go.
    const cur = _fileBrowserPath || '';
    const visible = []; // {name, path, depth}
    async function walk(node, depth) {
        visible.push({...node, depth, expanded: _fbTreeExpanded.has(node.path)});
        if (!_fbTreeExpanded.has(node.path)) return;
        const kids = await _fbTreeFetchChildren(node.path);
        for (const k of kids) await walk(k, depth + 1);
    }
    for (const r of roots) await walk(r, 0);
    body.innerHTML = visible.map((n) => {
        const isCur = n.path === cur ? ' fb-tree-active' : '';
        const twist = _fbTreeExpanded.has(n.path) ? '&#9660;' : '&#9658;';
        const pad = 6 + n.depth * 14;
        return `<div class="fb-tree-node${isCur}" style="padding-left:${pad}px" data-fb-tree-path="${escapeHtml(n.path)}">`
            + `<span class="fb-tree-twist" data-fb-tree-twist="${escapeHtml(n.path)}">${twist}</span>`
            + `<span class="fb-tree-icon">&#128193;</span>`
            + `<span class="fb-tree-name" title="${escapeHtml(n.path)}">${escapeHtml(n.name)}</span>`
            + `</div>`;
    }).join('');
}

// Click handler: clicking the twist toggles expansion; clicking the
// row loads the folder in the main pane.
document.addEventListener('click', async (e) => {
    const twist = e.target.closest('[data-fb-tree-twist]');
    if (twist) {
        e.stopPropagation();
        const p = twist.dataset.fbTreeTwist;
        if (_fbTreeExpanded.has(p)) _fbTreeExpanded.delete(p);
        else _fbTreeExpanded.add(p);
        _fbTreePersist();
        renderFileBrowserTree();
        return;
    }
    const node = e.target.closest('[data-fb-tree-path]');
    if (node) {
        e.stopPropagation();
        loadDirectory(node.dataset.fbTreePath);
    }
});

// Tree close button
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="fbTreeClose"]')) fileBrowserToggleTreeSidebar(false);
});

// Restore visibility from prefs on first load.
(function _fbTreeInit() {
    try {
        if (typeof prefs !== 'undefined' && prefs.getItem('fileBrowserTreeVisible') === '1') {
            fileBrowserToggleTreeSidebar(true);
        }
    } catch (_) { /* ignore */ }
})();

if (typeof window !== 'undefined') {
    window.fileBrowserToggleTreeSidebar = fileBrowserToggleTreeSidebar;
    window.renderFileBrowserTree = renderFileBrowserTree;
}

// ── Inline image thumbnails (lazy) ──
// IntersectionObserver on the file-list scroll container fires when a
// thumb canvas comes into view; we then fetch the cached PNG bytes via
// `fs_image_thumbnail` (server-side resize + SQLite cache) and paint
// once. Each canvas is observed at most once — the `dataset.loaded` flag
// prevents re-fetching after scroll-out / scroll-back.
const FB_THUMB_WIDTH = 64; // 2× the visual 32px slot for HiDPI sharpness
let _fbThumbObserver = null;
function _fbInitThumbObserver() {
    if (typeof IntersectionObserver === 'undefined') return;
    if (_fbThumbObserver) return;
    _fbThumbObserver = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                if (!entry.isIntersecting) continue;
                const canvas = entry.target;
                if (canvas.dataset.loaded === '1' || canvas.dataset.loading === '1') continue;
                canvas.dataset.loading = '1';
                const path = canvas.dataset.fbThumbPath;
                if (!path) continue;
                window.vstUpdater.fsImageThumbnail(path, FB_THUMB_WIDTH).then((ab) => {
                    if (!ab || ab.byteLength === 0) return;
                    if (typeof window.paintPngBytesIntoCanvas === 'function') {
                        return window.paintPngBytesIntoCanvas(canvas, ab);
                    }
                }).then(() => {
                    canvas.dataset.loaded = '1';
                    canvas.dataset.loading = '';
                    canvas.classList.add('file-image-thumb-loaded');
                    _fbThumbObserver.unobserve(canvas);
                }).catch(() => {
                    canvas.dataset.loading = '';
                });
            }
        },
        { root: null, rootMargin: '200px 0px', threshold: 0.01 }
    );
}
// Observe thumb canvases after each render. Called from a MutationObserver
// on the file list so newly-rendered chunks pick up automatically.
function _fbObserveThumbCanvases(rootEl) {
    if (!_fbThumbObserver) _fbInitThumbObserver();
    if (!_fbThumbObserver) return;
    (rootEl || document).querySelectorAll('.file-image-thumb:not([data-loaded="1"])').forEach((c) => {
        _fbThumbObserver.observe(c);
    });
}
if (typeof window !== 'undefined' && typeof MutationObserver !== 'undefined') {
    // Watch every pane's file-list for new rows being appended (chunked
    // renderer adds in batches). The observer kicks IntersectionObserver
    // on the new canvases.
    const mo = new MutationObserver(() => _fbObserveThumbCanvases());
    document.addEventListener('DOMContentLoaded', () => {
        document.querySelectorAll('.fb-pane .file-list').forEach((el) => {
            mo.observe(el, {childList: true, subtree: true});
        });
        _fbObserveThumbCanvases();
    });
}

// ── Find Duplicates modal ──
// Recursive SHA-256 scan inside the current folder. Server pre-filters
// by (size, ext) so only candidate sets are hashed. Modal groups files
// by identical content + lets the user trash all-but-first per group.
async function fileBrowserShowDuplicatesModal() {
    if (!_fileBrowserPath) return;
    document.getElementById('appDupModal')?.remove();
    const html = `<div class="modal-overlay modal-visible" id="appDupModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Find Duplicates — ${escapeHtml(_fileBrowserPath.split('/').filter(Boolean).pop() || _fileBrowserPath)}</h2>
        <button type="button" class="modal-close" data-app-dup="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message">Scans by SHA-256 content hash. Pre-filters by (size, ext) so only real candidate sets are hashed.</p>
        <label class="app-confirm-message" style="display:flex;gap:8px;align-items:center;margin-top:8px;">
          <input type="checkbox" id="appDupRecursive" checked> Recursive (walk subfolders)
        </label>
        <label class="app-confirm-message" style="display:flex;gap:8px;align-items:center;margin-top:6px;">
          Min size: <input type="number" id="appDupMinSize" value="1024" min="1" style="width:80px"> bytes
        </label>
        <div id="appDupResults" class="fb-info-grid" style="max-height:420px;overflow-y:auto;margin-top:12px;"></div>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-dup="close">Close</button>
          <button type="button" class="btn btn-primary" data-app-dup="scan">Scan</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appDupModal');
    const recursive = document.getElementById('appDupRecursive');
    const minSize = document.getElementById('appDupMinSize');
    const results = document.getElementById('appDupResults');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const fmtBytes = (n) => {
        if (typeof _fmtBytes === 'function') return _fmtBytes(n);
        return `${n} B`;
    };
    const trashGroup = async (paths) => {
        // Keep the first, trash the rest. User can sort the group order
        // before clicking if they want to keep a different copy — for
        // now first-wins is the convention.
        const toTrash = paths.slice(1);
        const ok = typeof confirmAction === 'function'
            ? await confirmAction(`Move ${toTrash.length} duplicate${toTrash.length === 1 ? '' : 's'} to Trash? (Keeping the first copy in the group.)`, 'Trash Duplicates')
            : confirm(`Move ${toTrash.length} duplicates to Trash?`);
        if (!ok) return;
        let okCount = 0, fail = 0;
        for (const p of toTrash) {
            try { await window.vstUpdater.moveToTrash(p); okCount++; }
            catch (_) { fail++; }
        }
        if (typeof showToast === 'function') {
            if (okCount > 0) showToast(toastFmt('toast.deleted_name', {name: `trashed ${okCount} duplicate${okCount === 1 ? '' : 's'}`}));
            if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} failed`}), 4000, 'error');
        }
        // Re-scan to refresh
        scan();
    };
    const scan = async () => {
        results.innerHTML = '<div class="fb-info-val">scanning…</div>';
        try {
            const groups = await window.vstUpdater.fsFindDuplicates(
                _fileBrowserPath,
                recursive.checked,
                parseInt(minSize.value, 10) || 1,
            );
            if (!groups || groups.length === 0) {
                results.innerHTML = '<div class="fb-info-val">no duplicates found</div>';
                return;
            }
            // Render each group as a block. Header: "N files, X MB each
            // (reclaim Y MB)" + per-path list with Trash All But First.
            results.innerHTML = groups.map((g, gi) => {
                const reclaim = g.size * (g.paths.length - 1);
                const pathsHtml = g.paths.map((p) => `<div class="fb-info-val" style="cursor:pointer" data-dup-path="${escapeHtml(p)}">${escapeHtml(p)}</div>`).join('');
                return `<div class="fb-info-key" style="margin-top:${gi === 0 ? 0 : 16}px">${g.paths.length} files, ${escapeHtml(fmtBytes(g.size))} each — reclaim ${escapeHtml(fmtBytes(reclaim))}</div>
                <div class="fb-info-val">
                  ${pathsHtml}
                  <button type="button" class="btn btn-stop" style="margin-top:6px;font-size:11px" data-dup-trash="${gi}">Trash all but first (${g.paths.length - 1})</button>
                </div>`;
            }).join('');
            // Wire trash buttons (need closure over groups[gi].paths).
            results.querySelectorAll('[data-dup-trash]').forEach((btn) => {
                btn.addEventListener('click', () => {
                    const gi = parseInt(btn.dataset.dupTrash, 10);
                    if (groups[gi]) trashGroup(groups[gi].paths);
                });
            });
            // Click a path → navigate to its parent + close modal.
            results.querySelectorAll('[data-dup-path]').forEach((el) => {
                el.addEventListener('click', () => {
                    const p = el.dataset.dupPath;
                    const parent = p.replace(/\/[^/]+$/, '');
                    close();
                    loadDirectory(parent);
                });
            });
        } catch (err) {
            results.innerHTML = `<div class="fb-info-val">error: ${escapeHtml(String(err && err.message ? err.message : err))}</div>`;
        }
    };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-app-dup]');
        if (btn) { if (btn.dataset.appDup === 'scan') scan(); else close(); return; }
        if (e.target === modal) close();
    });
    // Auto-scan on open so the modal isn't blank.
    requestAnimationFrame(() => scan());
}

// ── Drag and drop between panes ──
// HTML5 native drag — rows are draggable="true" via buildFileListRowHtml.
// dragstart: pick the dragged paths (selection if row is in it, else
//   just that row's path) + record source pane idx.
// dragover on .fb-pane: preventDefault + set dropEffect (move if no
//   Cmd held, copy if Cmd/Ctrl/Alt). Highlight target pane.
// drop: parse source paths + run copy / move IPC into the target
//   pane's path, then reload both panes.
//
// Custom MIME type so this doesn't clash with native filesystem drag
// (Tauri's tauri-plugin-drag handles drags FROM the app to the OS).
const FB_DND_MIME = 'application/x-audio-haxor-paths';
let _fbDragSrcPaneIdx = -1;

document.addEventListener('dragstart', (e) => {
    const row = e.target.closest('.file-row');
    if (!row || !row.dataset.filePath) return;
    const paneEl = row.closest('.fb-pane[data-pane-idx]');
    if (!paneEl) return;
    _fbDragSrcPaneIdx = parseInt(paneEl.dataset.paneIdx, 10);
    // If the dragged row IS in the selection, drag the WHOLE selection.
    // Else drag just that row. Matches Finder behavior.
    const path = row.dataset.filePath;
    const srcPane = _fbPanes[_fbDragSrcPaneIdx];
    const sel = srcPane ? srcPane.selection : null;
    const paths = (sel && sel.has(path) && sel.size > 1) ? [...sel] : [path];
    try {
        e.dataTransfer.setData(FB_DND_MIME, JSON.stringify(paths));
        e.dataTransfer.effectAllowed = 'copyMove';
    } catch (_) { /* some browsers throw on certain MIMEs */ }
});

document.addEventListener('dragover', (e) => {
    const pane = e.target.closest('.fb-pane[data-pane-idx]');
    if (!pane) return;
    if (!e.dataTransfer || !e.dataTransfer.types || !e.dataTransfer.types.includes(FB_DND_MIME)) return;
    e.preventDefault();
    // Default to MOVE; Cmd/Ctrl (Win) or Opt (Mac) flips to COPY —
    // matches Finder + Nautilus.
    e.dataTransfer.dropEffect = (e.metaKey || e.ctrlKey || e.altKey) ? 'copy' : 'move';
    pane.classList.add('fb-pane-drop-target');
});

document.addEventListener('dragleave', (e) => {
    const pane = e.target.closest('.fb-pane[data-pane-idx]');
    if (!pane) return;
    // Only remove highlight when leaving the pane itself, not when
    // moving between its children (children fire dragleave bubbling).
    if (!pane.contains(e.relatedTarget)) {
        pane.classList.remove('fb-pane-drop-target');
    }
});

document.addEventListener('drop', async (e) => {
    const pane = e.target.closest('.fb-pane[data-pane-idx]');
    if (!pane) return;
    const data = e.dataTransfer && e.dataTransfer.getData(FB_DND_MIME);
    if (!data) return;
    e.preventDefault();
    pane.classList.remove('fb-pane-drop-target');
    const destPaneIdx = parseInt(pane.dataset.paneIdx, 10);
    const destPane = _fbPanes[destPaneIdx];
    if (!destPane || !destPane.path) return;
    let paths = [];
    try { paths = JSON.parse(data); } catch (_) { return; }
    if (!Array.isArray(paths) || paths.length === 0) return;
    // Same pane + same parent dir → no-op (Finder behavior).
    const isCopy = e.metaKey || e.ctrlKey || e.altKey;
    let ok = 0, fail = 0;
    for (const src of paths) {
        const base = src.split('/').pop();
        const dest = `${destPane.path}/${base}`;
        if (src === dest) continue; // same place
        try {
            if (isCopy) {
                // Same-folder copy → use fsCopyPath; will fail on
                // collision so the user sees the error.
                await window.vstUpdater.fsCopyPath(src, dest);
            } else {
                await window.vstUpdater.renameFile(src, dest);
            }
            ok++;
        } catch (_) { fail++; }
    }
    if (typeof showToast === 'function') {
        const verb = isCopy ? 'copied' : 'moved';
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `${verb} ${ok} item${ok === 1 ? '' : 's'} → pane ${destPaneIdx + 1}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} ${verb === 'copied' ? 'copy' : 'move'}${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    // Refresh destination + source (move emptied source's selection).
    if (destPaneIdx !== _fbActivePaneIdx) {
        await loadDirectoryIntoPane(destPane.path, destPaneIdx);
    } else if (_fileBrowserPath) {
        await loadDirectory(_fileBrowserPath);
    }
    if (!isCopy && _fbDragSrcPaneIdx >= 0 && _fbDragSrcPaneIdx !== destPaneIdx) {
        const srcPane = _fbPanes[_fbDragSrcPaneIdx];
        if (srcPane && srcPane.path) {
            if (_fbDragSrcPaneIdx === _fbActivePaneIdx) {
                await loadDirectory(srcPane.path);
            } else {
                await loadDirectoryIntoPane(srcPane.path, _fbDragSrcPaneIdx);
            }
        }
    }
    _fbDragSrcPaneIdx = -1;
});

// ── Diff Two Files modal ──
// Picks the FIRST TWO selected files from the active pane (alphabetical
// order). Server returns unified diff ops; client renders side-by-side.
async function fileBrowserShowDiffModal(pathA, pathB) {
    if (!pathA || !pathB) {
        // Auto-pick from selection (need exactly 2 files).
        const sel = (typeof _fileSelected !== 'undefined' && _fileSelected instanceof Set)
            ? [..._fileSelected].sort()
            : [];
        if (sel.length !== 2) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'Select exactly 2 files to diff'}), 4000, 'error');
            return;
        }
        pathA = sel[0];
        pathB = sel[1];
    }
    document.getElementById('appDiffModal')?.remove();
    const nameA = pathA.split('/').pop();
    const nameB = pathB.split('/').pop();
    const html = `<div class="modal-overlay modal-visible" id="appDiffModal" role="dialog" aria-modal="true">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Diff — ${escapeHtml(nameA)} ⇄ ${escapeHtml(nameB)}</h2>
        <button type="button" class="modal-close" data-app-diff="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div id="appDiffBody" class="fb-diff-body">computing diff…</div>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-primary" data-app-diff="close">Close</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appDiffModal');
    const body = document.getElementById('appDiffBody');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        if (e.target.closest('[data-app-diff="close"]') || e.target === modal) close();
    });
    try {
        const ops = await window.vstUpdater.fsDiff(pathA, pathB);
        if (!ops || ops.length === 0) {
            body.innerHTML = '<div class="fb-diff-equal">files are identical</div>';
            return;
        }
        // Filter to non-equal ops + a couple lines of context. For
        // typical text diffs the output is short enough to render
        // inline without virtualization.
        body.innerHTML = ops.map((op) => {
            const cls = `fb-diff-${op.tag}`;
            const prefix = op.tag === 'delete' ? '−' : op.tag === 'insert' ? '+' : ' ';
            return `<div class="${cls}"><span class="fb-diff-gutter">${prefix}</span><span class="fb-diff-text">${escapeHtml(op.text)}</span></div>`;
        }).join('');
    } catch (err) {
        body.innerHTML = `<div class="fb-diff-error">error: ${escapeHtml(String(err && err.message ? err.message : err))}</div>`;
    }
}

// ── Color labels (Finder-style file tags 1-7) ──
// Single-tag-per-file model: each path maps to at most one label idx
// (0 = no label, 1-7 = colored). Persisted in prefs as
// `fileBrowserLabels` = {path: idx}. Rendered as a colored ring on
// the row icon cell, patched in place to avoid full re-render on label
// changes.
const FB_LABEL_COLORS = [
    null,            // 0 = no label
    '#ff5555',       // 1 red
    '#ffb86c',       // 2 orange
    '#f1fa8c',       // 3 yellow
    '#50fa7b',       // 4 green
    '#8be9fd',       // 5 cyan
    '#bd93f9',       // 6 purple
    '#bfbfbf',       // 7 gray
];
const FB_LABEL_NAMES = ['None', 'Red', 'Orange', 'Yellow', 'Green', 'Cyan', 'Purple', 'Gray'];
let _fbLabels = (() => {
    try {
        const raw = typeof prefs !== 'undefined' ? prefs.getItem('fileBrowserLabels') : null;
        return raw ? (JSON.parse(raw) || {}) : {};
    } catch (_) { return {}; }
})();
function _fbLabelsPersist() {
    try {
        if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserLabels', JSON.stringify(_fbLabels));
    } catch (_) { /* ignore */ }
}
function fileBrowserGetLabel(path) { return _fbLabels[path] || 0; }
function fileBrowserSetLabel(path, idx) {
    idx = parseInt(idx, 10) || 0;
    if (idx === 0) delete _fbLabels[path];
    else _fbLabels[path] = idx;
    _fbLabelsPersist();
    const row = document.querySelector(`.file-row[data-file-path="${CSS.escape(path)}"]`);
    if (row) {
        let ring = row.querySelector('.fb-label-ring');
        if (idx === 0) {
            if (ring) ring.remove();
        } else {
            if (!ring) {
                ring = document.createElement('span');
                ring.className = 'fb-label-ring';
                const iconCell = row.querySelector('.file-icon');
                if (iconCell) iconCell.appendChild(ring);
            }
            ring.style.background = FB_LABEL_COLORS[idx];
            ring.title = `Label: ${FB_LABEL_NAMES[idx]}`;
        }
    }
}
function fileBrowserBulkSetLabel(paths, idx) {
    for (const p of paths) fileBrowserSetLabel(p, idx);
    if (typeof showToast === 'function') {
        showToast(toastFmt('toast.deleted_name', {name: `labeled ${paths.length} item${paths.length === 1 ? '' : 's'} ${FB_LABEL_NAMES[idx] || 'None'}`}));
    }
}

// ── Touch (set mtime to now) ──
async function fileBrowserTouchPaths(paths) {
    if (!Array.isArray(paths) || paths.length === 0) return;
    let ok = 0, fail = 0;
    for (const p of paths) {
        try { await window.vstUpdater.fsTouch(p); ok++; }
        catch (_) { fail++; }
    }
    if (typeof showToast === 'function') {
        if (ok > 0) showToast(toastFmt('toast.deleted_name', {name: `touched ${ok} item${ok === 1 ? '' : 's'}`}));
        if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} touch${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
}

// ── Compare folders modal ──
// Defaults to active pane vs next pane (multi-pane required); can take
// explicit paths from caller (e.g. selection has exactly 2 folders).
async function fileBrowserShowCompareModal(dirA, dirB) {
    if (!dirA || !dirB) {
        if (_fbPaneCount < 2) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'Need ≥ 2 panes (Cmd+\\\\ to split)'}), 4000, 'error');
            return;
        }
        const a = _fbActivePane();
        const b = _fbPanes[(_fbActivePaneIdx + 1) % _fbPaneCount];
        if (!a || !b || !a.path || !b.path) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: 'One pane has no folder loaded'}), 4000, 'error');
            return;
        }
        dirA = a.path; dirB = b.path;
    }
    document.getElementById('appCmpModal')?.remove();
    const html = `<div class="modal-overlay modal-visible" id="appCmpModal" role="dialog" aria-modal="true">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Compare Folders</h2>
        <button type="button" class="modal-close" data-app-cmp="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message"><strong>A:</strong> ${escapeHtml(dirA)}<br><strong>B:</strong> ${escapeHtml(dirB)}</p>
        <div id="appCmpBody" class="fb-cmp-body">comparing…</div>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-primary" data-app-cmp="close">Close</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appCmpModal');
    const body = document.getElementById('appCmpBody');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        if (e.target.closest('[data-app-cmp="close"]') || e.target === modal) close();
    });
    try {
        const r = await window.vstUpdater.fsCompareDirs(dirA, dirB);
        const sec = (label, items, cls) => {
            if (!items || items.length === 0) return '';
            return `<div class="fb-cmp-section"><div class="fb-info-key">${escapeHtml(label)} (${items.length})</div>`
                + items.map((rel) => `<div class="fb-info-val ${cls}">${escapeHtml(rel)}</div>`).join('')
                + `</div>`;
        };
        const total = (r.onlyInA?.length || 0) + (r.onlyInB?.length || 0) + (r.different?.length || 0);
        if (total === 0) {
            body.innerHTML = '<div class="fb-cmp-equal">trees are identical</div>';
        } else {
            body.innerHTML = sec('Only in A', r.onlyInA, 'fb-cmp-only-a')
                + sec('Only in B', r.onlyInB, 'fb-cmp-only-b')
                + sec('Different content (same path)', r.different, 'fb-cmp-diff');
        }
    } catch (err) {
        body.innerHTML = `<div class="fb-diff-error">error: ${escapeHtml(String(err && err.message ? err.message : err))}</div>`;
    }
}

// ── Quick file palette (Cmd+P — VSCode-style recent file/folder jumper) ──
// Tracks every path that's been loaded via `loadDirectory` (folders)
// or opened (files via opener_open / openFileDefault). Persisted in
// prefs as a capped list (most-recent first).
const FB_RECENT_CAP = 200;
let _fbRecent = (() => {
    try {
        const raw = typeof prefs !== 'undefined' ? prefs.getItem('fileBrowserRecent') : null;
        const parsed = raw ? JSON.parse(raw) : null;
        if (Array.isArray(parsed)) {
            return parsed.filter((e) => e && typeof e.path === 'string').slice(0, FB_RECENT_CAP);
        }
    } catch (_) { /* fall through */ }
    return [];
})();
function fileBrowserRecordRecent(path, isDir) {
    if (!path) return;
    _fbRecent = _fbRecent.filter((r) => r.path !== path);
    _fbRecent.unshift({path, isDir: !!isDir, ts: Date.now()});
    if (_fbRecent.length > FB_RECENT_CAP) _fbRecent.length = FB_RECENT_CAP;
    try {
        if (typeof prefs !== 'undefined') prefs.setItem('fileBrowserRecent', JSON.stringify(_fbRecent));
    } catch (_) { /* ignore */ }
}
// Fuzzy match: walk the needle char-by-char through the haystack; the
// shorter the matched span, the better the score.
function _fbFuzzyScore(haystack, needle) {
    if (!needle) return 1;
    const hl = haystack.toLowerCase();
    const nl = needle.toLowerCase();
    let h = 0, n = 0;
    let firstHit = -1, lastHit = -1;
    while (h < hl.length && n < nl.length) {
        if (hl[h] === nl[n]) {
            if (firstHit < 0) firstHit = h;
            lastHit = h;
            n++;
        }
        h++;
    }
    if (n < nl.length) return 0;
    const span = lastHit - firstHit + 1;
    return needle.length / span;
}
async function fileBrowserShowQuickPalette() {
    document.getElementById('appQuickModal')?.remove();
    const html = `<div class="modal-overlay modal-visible" id="appQuickModal" role="dialog" aria-modal="true">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Quick Open — recent folders &amp; files</h2>
        <button type="button" class="modal-close" data-app-quick="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <input type="text" id="appQuickInput" class="app-prompt-input" placeholder="Fuzzy search…" />
        <div id="appQuickResults" class="fb-quick-results"></div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appQuickModal');
    const input = document.getElementById('appQuickInput');
    const results = document.getElementById('appQuickResults');
    let cursor = 0;
    let visible = [];
    const close = () => { modal?.remove(); document.removeEventListener('keydown', key, true); };
    const open = (entry) => {
        close();
        if (!entry) return;
        if (entry.isDir) loadDirectory(entry.path);
        else if (window.vstUpdater && typeof window.vstUpdater.openFileDefault === 'function') {
            window.vstUpdater.openFileDefault(entry.path).catch(() => {});
        }
    };
    const render = () => {
        const q = (input.value || '').trim();
        const scored = _fbRecent.map((r) => {
            const name = r.path.split('/').pop() || r.path;
            const sc = _fbFuzzyScore(name + ' ' + r.path, q);
            return {entry: r, name, score: sc};
        }).filter((s) => s.score > 0).sort((a, b) => b.score - a.score).slice(0, 50);
        visible = scored.map((s) => s.entry);
        if (cursor >= visible.length) cursor = 0;
        results.innerHTML = visible.map((r, i) => {
            const name = r.path.split('/').pop() || r.path;
            const icon = r.isDir ? '&#128193;' : '&#128196;';
            const active = i === cursor ? ' fb-quick-active' : '';
            return `<div class="fb-quick-row${active}" data-fb-quick-idx="${i}">`
                + `<span class="fb-quick-icon">${icon}</span>`
                + `<span class="fb-quick-name">${escapeHtml(name)}</span>`
                + `<span class="fb-quick-path">${escapeHtml(r.path)}</span>`
                + `</div>`;
        }).join('');
    };
    const key = (e) => {
        if (e.key === 'Escape') { e.preventDefault(); close(); return; }
        if (e.key === 'Enter') { e.preventDefault(); open(visible[cursor]); return; }
        if (e.key === 'ArrowDown') { e.preventDefault(); cursor = Math.min(cursor + 1, visible.length - 1); render(); return; }
        if (e.key === 'ArrowUp') { e.preventDefault(); cursor = Math.max(cursor - 1, 0); render(); return; }
    };
    document.addEventListener('keydown', key, true);
    input.addEventListener('input', () => { cursor = 0; render(); });
    modal?.addEventListener('click', (e) => {
        const row = e.target.closest('[data-fb-quick-idx]');
        if (row) { open(visible[parseInt(row.dataset.fbQuickIdx, 10)]); return; }
        if (e.target.closest('[data-app-quick="close"]') || e.target === modal) close();
    });
    requestAnimationFrame(() => { input.focus(); render(); });
}

// ── Spotlight-style global search (Cmd+K) ──
// Searches every populated inventory table at once (audio, DAW,
// presets, MIDI, PDFs, videos) via the FTS5 trigram tables on the Rust
// side. Modal groups results by category; click → switch tab + open or
// open file directly. Debounced 150ms on input to avoid hammering FTS.
let _fbSpotlightDebounce = null;
async function fileBrowserShowSpotlight() {
    document.getElementById('appSpotlightModal')?.remove();
    const html = `<div class="modal-overlay modal-visible" id="appSpotlightModal" role="dialog" aria-modal="true">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Spotlight — search all scanned inventory</h2>
        <button type="button" class="modal-close" data-app-spot="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <input type="text" id="appSpotInput" class="app-prompt-input" placeholder="search audio, DAW, presets, MIDI, PDFs, videos…" />
        <div id="appSpotResults" class="fb-spot-results"><div class="fb-info-val">Type ≥ 3 chars for FTS, 1-2 for LIKE fallback…</div></div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appSpotlightModal');
    const input = document.getElementById('appSpotInput');
    const results = document.getElementById('appSpotResults');
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);

    const groups = [
        {key: 'audio',  label: 'Audio',    tab: 'samples',  icon: '&#127925;'},
        {key: 'daw',    label: 'DAW',      tab: 'daw',      icon: '&#127911;'},
        {key: 'preset', label: 'Presets',  tab: 'presets',  icon: '&#127924;'},
        {key: 'midi',   label: 'MIDI',     tab: 'midi',     icon: '&#127929;'},
        {key: 'pdf',    label: 'PDFs',     tab: 'pdf',      icon: '&#128196;'},
        {key: 'video',  label: 'Videos',   tab: 'videos',   icon: '&#127909;'},
    ];

    const openHit = (hit, tab) => {
        close();
        if (tab && typeof switchTab === 'function') {
            switchTab(tab);
        }
        // Best-effort: open the file in default app. The user's already
        // on the right tab so they can find the row there too.
        if (window.vstUpdater && typeof window.vstUpdater.openFileDefault === 'function') {
            window.vstUpdater.openFileDefault(hit.path).catch(() => {});
        }
    };

    const render = (data) => {
        if (!data) { results.innerHTML = '<div class="fb-info-val">no input</div>'; return; }
        const total = groups.reduce((s, g) => s + (data[g.key]?.length || 0), 0);
        if (total === 0) {
            results.innerHTML = '<div class="fb-info-val">no matches across any inventory</div>';
            return;
        }
        results.innerHTML = groups.map((g) => {
            const hits = data[g.key] || [];
            if (hits.length === 0) return '';
            return `<div class="fb-spot-section"><div class="fb-spot-section-head">${g.icon} ${escapeHtml(g.label)} (${hits.length})</div>`
                + hits.map((h, hi) => {
                    const right = h.ext ? `<span class="fb-spot-ext">${escapeHtml(String(h.ext).toUpperCase())}</span>` : '';
                    return `<div class="fb-spot-row" data-fb-spot-key="${g.key}" data-fb-spot-idx="${hi}">`
                        + `<span class="fb-spot-name">${escapeHtml(h.name)}</span>`
                        + `<span class="fb-spot-path">${escapeHtml(h.path)}</span>`
                        + right + `</div>`;
                }).join('')
                + `</div>`;
        }).join('');
        // Wire row clicks
        let lastData = data;
        results.querySelectorAll('[data-fb-spot-key]').forEach((el) => {
            el.addEventListener('click', () => {
                const k = el.dataset.fbSpotKey;
                const i = parseInt(el.dataset.fbSpotIdx, 10);
                const g = groups.find((g) => g.key === k);
                const hit = lastData[k] && lastData[k][i];
                if (g && hit) openHit(hit, g.tab);
            });
        });
    };

    const runSearch = async () => {
        const q = (input.value || '').trim();
        if (!q) { results.innerHTML = '<div class="fb-info-val">Type ≥ 3 chars for FTS, 1-2 for LIKE fallback…</div>'; return; }
        results.innerHTML = '<div class="fb-info-val">searching…</div>';
        try {
            const data = await window.vstUpdater.fsGlobalSearch(q, 25);
            render(data);
        } catch (err) {
            results.innerHTML = `<div class="fb-diff-error">error: ${escapeHtml(String(err && err.message ? err.message : err))}</div>`;
        }
    };

    input.addEventListener('input', () => {
        if (_fbSpotlightDebounce) clearTimeout(_fbSpotlightDebounce);
        _fbSpotlightDebounce = setTimeout(runSearch, 150);
    });
    modal?.addEventListener('click', (e) => {
        if (e.target.closest('[data-app-spot="close"]') || e.target === modal) close();
    });
    requestAnimationFrame(() => input.focus());
}

// ── Bookmarks management modal ──
// Rename / reorder / delete favorite directories in one place. Pulls
// from `getFavDirs()` / writes back via the existing setter.
async function fileBrowserShowBookmarksModal() {
    if (typeof getFavDirs !== 'function') return;
    document.getElementById('appBmModal')?.remove();
    let dirs = (getFavDirs() || []).slice();
    const render = (body) => {
        body.innerHTML = dirs.length === 0
            ? '<div class="fb-info-val">no bookmarks yet — star folders from the file list to add them</div>'
            : dirs.map((d, i) => {
                return `<div class="fb-bm-row" data-bm-idx="${i}">
                    <button class="fb-bm-up" data-bm-up="${i}" title="Move up">&#9650;</button>
                    <button class="fb-bm-down" data-bm-down="${i}" title="Move down">&#9660;</button>
                    <input type="text" class="fb-bm-name app-prompt-input" data-bm-name="${i}" value="${escapeHtml(d.name || '')}">
                    <span class="fb-bm-path" data-bm-go="${i}" title="${escapeHtml(d.path)}">${escapeHtml(d.path)}</span>
                    <button class="fb-bm-del" data-bm-del="${i}" title="Remove bookmark">&times;</button>
                </div>`;
            }).join('');
    };
    const html = `<div class="modal-overlay modal-visible" id="appBmModal" role="dialog" aria-modal="true">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Bookmarks (${dirs.length})</h2>
        <button type="button" class="modal-close" data-app-bm="close" aria-label="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <p class="app-confirm-message">Rename inline, reorder via ▲▼, click path to jump, × to remove.</p>
        <div id="appBmBody" class="fb-bm-body"></div>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-bm="close">Close</button>
          <button type="button" class="btn btn-primary" data-app-bm="save">Save</button>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const modal = document.getElementById('appBmModal');
    const body = document.getElementById('appBmBody');
    render(body);
    const close = () => { modal?.remove(); document.removeEventListener('keydown', esc, true); };
    const save = () => {
        // Persist via prefs directly (`getFavDirs` reads `prefs.favDirs`).
        try {
            if (typeof prefs !== 'undefined') prefs.setItem('favDirs', JSON.stringify(dirs));
        } catch (_) { /* ignore */ }
        if (typeof renderFavDirs === 'function') renderFavDirs();
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `saved ${dirs.length} bookmark${dirs.length === 1 ? '' : 's'}`}));
        close();
    };
    const esc = (e) => { if (e.key === 'Escape') { e.preventDefault(); close(); } };
    document.addEventListener('keydown', esc, true);
    modal?.addEventListener('click', (e) => {
        const up = e.target.closest('[data-bm-up]');
        if (up) {
            const i = parseInt(up.dataset.bmUp, 10);
            if (i > 0) { [dirs[i - 1], dirs[i]] = [dirs[i], dirs[i - 1]]; render(body); }
            return;
        }
        const down = e.target.closest('[data-bm-down]');
        if (down) {
            const i = parseInt(down.dataset.bmDown, 10);
            if (i < dirs.length - 1) { [dirs[i + 1], dirs[i]] = [dirs[i], dirs[i + 1]]; render(body); }
            return;
        }
        const del = e.target.closest('[data-bm-del]');
        if (del) {
            const i = parseInt(del.dataset.bmDel, 10);
            dirs.splice(i, 1); render(body);
            return;
        }
        const go = e.target.closest('[data-bm-go]');
        if (go) {
            const i = parseInt(go.dataset.bmGo, 10);
            if (dirs[i]) { close(); loadDirectory(dirs[i].path); }
            return;
        }
        const btn = e.target.closest('[data-app-bm]');
        if (btn) { if (btn.dataset.appBm === 'save') save(); else close(); return; }
        if (e.target === modal) close();
    });
    modal?.addEventListener('input', (e) => {
        const nameInput = e.target.closest('[data-bm-name]');
        if (nameInput) {
            const i = parseInt(nameInput.dataset.bmName, 10);
            if (dirs[i]) dirs[i].name = nameInput.value;
        }
    });
}

// Expose helpers to context-menu.js (separate file, separate scope).
if (typeof window !== 'undefined') {
    window.fileBrowserMarkClipboard = fileBrowserMarkClipboard;
    window.fileBrowserPasteClipboard = fileBrowserPasteClipboard;
    window.fileBrowserShowInfo = fileBrowserShowInfo;
    window.fileBrowserNewFolderWithSelection = fileBrowserNewFolderWithSelection;
    window.fileBrowserShowHashModal = fileBrowserShowHashModal;
    window.fileBrowserShowChmodModal = fileBrowserShowChmodModal;
    window.fileBrowserPatternSelect = fileBrowserPatternSelect;
    window.fileBrowserBulkCompress = fileBrowserBulkCompress;
    window.fileBrowserBulkExtract = fileBrowserBulkExtract;
    window.fileBrowserShowGrepModal = fileBrowserShowGrepModal;
    window.fileBrowserShowDuplicatesModal = fileBrowserShowDuplicatesModal;
    window.fileBrowserShowDiffModal = fileBrowserShowDiffModal;
    window.fileBrowserShowQuickPalette = fileBrowserShowQuickPalette;
    window.fileBrowserShowBookmarksModal = fileBrowserShowBookmarksModal;
    window.fileBrowserShowBulkChmodModal = fileBrowserShowBulkChmodModal;
    window.fileBrowserRecordRecent = fileBrowserRecordRecent;
    window.fileBrowserGetLabel = fileBrowserGetLabel;
    window.fileBrowserSetLabel = fileBrowserSetLabel;
    window.fileBrowserBulkSetLabel = fileBrowserBulkSetLabel;
    window.fileBrowserTouchPaths = fileBrowserTouchPaths;
    window.fileBrowserShowCompareModal = fileBrowserShowCompareModal;
    window.fileBrowserShowSpotlight = fileBrowserShowSpotlight;
    // Cmd+K — global Spotlight modal. Capture phase so it wins over
    // per-tab keydown handlers; works from any inventory tab, not just
    // Files. Skip while a text input has focus (Cmd+K could be a
    // shortcut inside an editor).
    document.addEventListener('keydown', (e) => {
        if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return;
        if (e.key !== 'k' && e.key !== 'K') return;
        const t = e.target;
        if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        fileBrowserShowSpotlight();
    }, true);
    window.FB_LABEL_COLORS = FB_LABEL_COLORS;
    window.FB_LABEL_NAMES = FB_LABEL_NAMES;
}

// ── Move-to-bookmark (right-click → Move to → <bookmark>) ──
// Returns an array of context-menu items, one per saved favorite dir, each
// of which moves the path to that bookmark when clicked. Empty array when
// no bookmarks exist — the caller can omit the submenu in that case.
function buildMoveToBookmarkMenuItems(srcPath) {
    if (typeof getFavDirs !== 'function') return [];
    const dirs = getFavDirs();
    if (!Array.isArray(dirs) || dirs.length === 0) return [];
    return dirs.map((d) => ({
        icon: '&#128193;',
        label: `${d.name}`,
        action: async () => {
            const base = srcPath.split('/').pop();
            const target = `${d.path}/${base}`;
            try {
                await window.vstUpdater.renameFile(srcPath, target);
                if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `moved ${base} → ${d.name}`}));
                if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
            } catch (err) {
                if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
            }
        },
    }));
}

// ── Forward/back navigation history ──
// Browser-style nav: each `loadDirectory` push appends to the history stack
// unless the load was itself triggered by back/forward (which sets the skip
// flag). Forward history is dropped on a new push (matches browser semantics).
var _fbHistory = [];
var _fbHistoryIdx = -1;
var _fbHistorySkipPush = false;

function _navHistoryRecord(path) {
    if (_fbHistorySkipPush) {
        _fbHistorySkipPush = false;
        return;
    }
    if (_fbHistory[_fbHistoryIdx] === path) return; // no-op nav to same dir
    // Drop forward history past the current index (new branch).
    _fbHistory = _fbHistory.slice(0, _fbHistoryIdx + 1);
    _fbHistory.push(path);
    _fbHistoryIdx = _fbHistory.length - 1;
    _updateNavButtons();
}

function _updateNavButtons() {
    if (typeof document === 'undefined') return;
    const back = document.getElementById('fbNavBack');
    const fwd = document.getElementById('fbNavFwd');
    if (back) back.disabled = _fbHistoryIdx <= 0;
    if (fwd) fwd.disabled = _fbHistoryIdx >= _fbHistory.length - 1;
}

function fileNavBack() {
    if (_fbHistoryIdx <= 0) return;
    _fbHistoryIdx--;
    _fbHistorySkipPush = true;
    loadDirectory(_fbHistory[_fbHistoryIdx]);
}

function fileNavForward() {
    if (_fbHistoryIdx >= _fbHistory.length - 1) return;
    _fbHistoryIdx++;
    _fbHistorySkipPush = true;
    loadDirectory(_fbHistory[_fbHistoryIdx]);
}

// ── Cmd+L editable path input ──
function showFilePathEditor() {
    if (typeof document === 'undefined') return;
    const input = document.getElementById('fileBrowserPathInput');
    const breadcrumb = document.getElementById('fileBreadcrumb');
    if (!input || !breadcrumb) return;
    input.value = _fileBrowserPath || '';
    input.classList.remove('fb-hidden');
    breadcrumb.classList.add('fb-hidden');
    input.focus();
    input.select();
}

function hideFilePathEditor() {
    if (typeof document === 'undefined') return;
    const input = document.getElementById('fileBrowserPathInput');
    const breadcrumb = document.getElementById('fileBreadcrumb');
    if (!input || !breadcrumb) return;
    input.classList.add('fb-hidden');
    breadcrumb.classList.remove('fb-hidden');
}

// ── Extension filter chips ──
// Categories map to existing global extension sets where available
// (`AUDIO_EXTS`, `DAW_EXTS`). Preset / MIDI / PDF / video / image use
// dedicated lists below. Folders are always visible regardless of chip so
// the user can navigate.
var _fbExtFilter = 'all';
var _fbExtCategories = null; // lazily built after globals are loaded

function _fbGetExtCategories() {
    if (_fbExtCategories) return _fbExtCategories;
    const audio = (typeof AUDIO_EXTS !== 'undefined') ? AUDIO_EXTS : [];
    const daw = (typeof DAW_EXTS !== 'undefined') ? DAW_EXTS : [];
    const preset = ['fxp', 'fxb', 'preset', 'aupreset', 'h2p', 'nksf', 'nksfx', 'adv', 'agr', 'vstpreset', 'cmb', 'patch', 'nrkt'];
    const midi = ['mid', 'midi'];
    const pdf = ['pdf'];
    const video = ['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v', 'wmv', 'flv', 'mpg', 'mpeg'];
    const image = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp', 'tiff', 'tif', 'ico'];
    _fbExtCategories = {audio, daw, preset, midi, pdf, video, image};
    return _fbExtCategories;
}

function _fbExtMatches(entry, category) {
    if (category === 'all') return true;
    if (entry.isDir) return true; // folders always visible — navigation can't break
    const cats = _fbGetExtCategories();
    if (category === 'other') {
        const inAny = [
            ...cats.audio, ...cats.daw, ...cats.preset, ...cats.midi,
            ...cats.pdf, ...cats.video, ...cats.image,
        ].includes(entry.ext);
        return !inAny;
    }
    const list = cats[category];
    return Array.isArray(list) && list.includes(entry.ext);
}

function setFileExtFilter(category) {
    _fbExtFilter = category || 'all';
    if (typeof document !== 'undefined') {
        document.querySelectorAll('.fb-ext-chips .ext-chip').forEach((c) => {
            c.classList.toggle('active', c.dataset.extFilter === _fbExtFilter);
        });
    }
    if (typeof renderFileList === 'function') renderFileList();
}

// ── Footer status line ──
function _fbFormatBytes(n) {
    if (!Number.isFinite(n) || n <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const idx = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
    return `${(n / Math.pow(1024, idx)).toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}

function updateFileListFooter(entries) {
    if (typeof document === 'undefined') return;
    const el = document.getElementById('fileListFooter');
    if (!el) return;
    const arr = Array.isArray(entries) ? entries : [];
    let audio = 0;
    let totalBytes = 0;
    const audioExts = (typeof AUDIO_EXTS !== 'undefined') ? AUDIO_EXTS : [];
    for (const e of arr) {
        if (!e.isDir) {
            if (audioExts.includes(e.ext)) audio++;
            totalBytes += Number(e.size) || 0;
        }
    }
    el.textContent = `${arr.length} items · ${audio} audio · ${_fbFormatBytes(totalBytes)}`;
}

// ── Folder scan-status badges ──
// After each directory render, query the backend for per-folder inventory
// counts (samples / presets / DAW / MIDI / PDF / video under the folder
// prefix) and inject small color-coded badges into the folder row's
// `.file-name` cell so the user can see at a glance which folders are
// already in inventory vs need scanning.
//
// Per-directory cache keyed on directory path — re-entering the same
// directory replays from cache (cheap) instead of re-querying the backend.
// Cache is cleared on full `loadDirectory` since the listing changed.
var _scanStatusCache = new Map(); // dirPath → Map<folderPath, status>

/** Token-bumped on each call so a slow IPC from a previous render can't paint
 *  badges into a newer render's DOM. */
var _scanStatusSeq = 0;

/** Build the badge HTML fragment for a single folder's scan-status. Returns
 *  empty string when nothing's been scanned (no badges = uninventoried). */
function _scanStatusBadgesHtml(status) {
    if (!status) return '';
    const parts = [];
    const add = (kind, n, label) => {
        if (n > 0) parts.push(`<span class="scan-badge sb-${kind}" title="${label}: ${n}">${kind.toUpperCase()[0]} ${n}</span>`);
    };
    add('samples', status.samples, 'Samples');
    add('presets', status.presets, 'Presets');
    add('daw', status.daw, 'DAW projects');
    add('midi', status.midi, 'MIDI files');
    add('pdf', status.pdf, 'PDFs');
    add('video', status.video, 'Videos');
    if (parts.length === 0) return '';
    return `<span class="scan-badges">${parts.join('')}</span>`;
}

/** Inject the badge fragment into the matching folder row's name cell. The
 *  size cell is owned exclusively by `refreshFolderFilesystemSizes` (the
 *  background recursive walk) — the two paint paths must not race for the
 *  same cell. Badges only touch `.file-name`; size only `_paintFolderSizeCell`
 *  touches `.file-size`. */
function _applyScanBadgesToRow(folderPath, status) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row.file-dir[data-file-path="${CSS.escape(folderPath)}"]`);
        if (!row) return;
        const nameCell = row.querySelector('.file-name');
        if (!nameCell) return;
        const prior = nameCell.querySelector('.scan-badges');
        if (prior) prior.remove();
        const html = _scanStatusBadgesHtml(status);
        if (html) nameCell.insertAdjacentHTML('beforeend', html);
    } catch (_) { /* CSS.escape unavailable */ }
}

// ── Background filesystem size for every visible folder ──
// On each directory render we kick off a recursive `fs_folder_size` walk for
// every folder row. Results stream back and replace the per-row size cell as
// they arrive (initial cells show "…" placeholder). Bounded concurrency so a
// directory with 100 folders doesn't spawn 100 simultaneous walks; a
// sequence guard prevents results from an abandoned directory from painting
// into the new one. Session cache makes revisits instant.
var _fsFolderSizeCache = new Map();
var _fsFolderSizeInFlight = new Map();
/** Bumped on each `refreshFolderFilesystemSizes` call. Painted results check
 *  this to ensure they belong to the current render (otherwise a slow walk
 *  from `/Music` could paint into `/Downloads` after the user navigated). */
var _fsFolderSizeSeq = 0;
/** Concurrent in-flight walks cap. JUCE/SMB I/O is the bottleneck — 4 keeps
 *  thrashing bounded while still finishing typical directories in seconds. */
const FS_FOLDER_SIZE_CONCURRENCY = 4;
var _fsFolderSizeQueue = [];
var _fsFolderSizeActive = 0;

async function getFsFolderSize(folderPath) {
    if (_fsFolderSizeCache.has(folderPath)) return _fsFolderSizeCache.get(folderPath);
    if (_fsFolderSizeInFlight.has(folderPath)) return _fsFolderSizeInFlight.get(folderPath);
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderSize !== 'function') return null;
    const p = window.vstUpdater.fsFolderSize(folderPath, 2000)
        .then((result) => {
            // Newer backend returns `{bytes, files}`; older builds returned a
            // bare u64. Accept both so a freshly-updated JS works against a
            // not-yet-rebuilt binary during development.
            const bytes = (result && typeof result === 'object')
                ? (Number(result.bytes) || 0)
                : (Number(result) || 0);
            const files = (result && typeof result === 'object')
                ? (Number(result.files) || 0)
                : 0;
            const r = {bytes, files};
            _fsFolderSizeCache.set(folderPath, r);
            return r;
        })
        .catch(() => null)
        .finally(() => { _fsFolderSizeInFlight.delete(folderPath); });
    _fsFolderSizeInFlight.set(folderPath, p);
    return p;
}

function _pumpFsFolderSizeQueue() {
    while (_fsFolderSizeActive < FS_FOLDER_SIZE_CONCURRENCY && _fsFolderSizeQueue.length > 0) {
        const job = _fsFolderSizeQueue.shift();
        if (job.seq !== _fsFolderSizeSeq) continue; // directory changed before we got to it
        _fsFolderSizeActive++;
        getFsFolderSize(job.folderPath)
            .then((result) => {
                if (job.seq !== _fsFolderSizeSeq) return;
                _paintFolderSizeCell(job.folderPath, result);
                // Stash the walked size/count on the entry so the sort helper
                // can see real folder sizes (the initial `e.size` is 0 because
                // `metadata.len()` returns inode size for directories). Files
                // already have their immediate size from fs_list_dir.
                if (result && Array.isArray(_fileBrowserEntries)) {
                    for (const e of _fileBrowserEntries) {
                        if (e.isDir && e.path === job.folderPath) {
                            e.size = Number(result.bytes) || 0;
                            e.itemsCount = Number(result.files) || 0;
                            break;
                        }
                    }
                }
            })
            .finally(() => {
                _fsFolderSizeActive--;
                _pumpFsFolderSizeQueue();
                // When the queue drains: if the user is sorting by Size (or
                // any sort that depends on folder data populated by the walk),
                // reorder the existing rows in place — no full re-render, so
                // the painted size cells stay intact (no flicker).
                if (_fsFolderSizeActive === 0
                    && _fsFolderSizeQueue.length === 0
                    && job.seq === _fsFolderSizeSeq
                    && (_fileSortKey === 'size' || _fileSortKey === 'items')
                    && typeof _reorderVisibleRowsInPlace === 'function') {
                    _reorderVisibleRowsInPlace();
                }
            });
    }
}

/** Reorders the existing `.file-row` elements in `#fileList` per the current
 *  sort key/direction WITHOUT touching cell contents. Used after the
 *  filesystem-size walks finish so the Size sort reflects the just-walked
 *  folder sizes (initial render saw `e.size = 0` for folders). Search-mode
 *  results are score-ordered; this is a no-op there. */
function _reorderVisibleRowsInPlace() {
    if (typeof document === 'undefined') return;
    const list = document.getElementById('fileList');
    if (!list) return;
    const searchInput = document.getElementById('fileSearchInput');
    const hasSearch = !!(searchInput && searchInput.value && searchInput.value.trim());
    if (hasSearch) return;
    if (!Array.isArray(_fileBrowserEntries)) return;
    const preFiltered = (_fbExtFilter && _fbExtFilter !== 'all')
        ? _fileBrowserEntries.filter((e) => _fbExtMatches(e, _fbExtFilter))
        : _fileBrowserEntries;
    const sorted = applyFileSort(preFiltered);
    const rowByPath = new Map();
    for (const row of list.querySelectorAll('.file-row')) {
        rowByPath.set(row.dataset.filePath, row);
    }
    // `appendChild` on an existing DOM node MOVES it — no clone, no innerHTML
    // reset. Doing this inside a DocumentFragment limits to one reflow.
    const fragment = document.createDocumentFragment();
    for (const e of sorted) {
        const row = rowByPath.get(e.path);
        if (row) fragment.appendChild(row);
    }
    list.appendChild(fragment);
}

function _fbFormatItemCount(n) {
    if (!Number.isFinite(n) || n < 0) return '';
    if (n < 1000) return `${n}`;
    if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}k`;
    return `${(n / 1_000_000).toFixed(1)}M`;
}

/** Paints both the size cell AND the items cell for a folder row from the
 *  bg-walk result `{bytes, files}`. `null` (walk failed) → dash in both. */
function _paintFolderSizeCell(folderPath, result) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row.file-dir[data-file-path="${CSS.escape(folderPath)}"]`);
        if (!row) return;
        const sizeCell = row.querySelector('.file-size');
        const itemsCell = row.querySelector('.file-items');
        if (result === null || result === undefined) {
            if (sizeCell) {
                sizeCell.textContent = '—';
                sizeCell.classList.remove('file-size-loading');
                sizeCell.classList.add('file-size-dash');
                sizeCell.title = 'Folder size unavailable (timeout, permission, or unreadable)';
            }
            if (itemsCell) {
                itemsCell.textContent = '—';
                itemsCell.classList.remove('file-items-loading');
                itemsCell.classList.add('file-items-dash');
            }
            return;
        }
        const bytes = Number(result.bytes) || 0;
        const files = Number(result.files) || 0;
        if (sizeCell) {
            sizeCell.textContent = _fbFormatBytes(bytes);
            sizeCell.classList.remove('file-size-loading', 'file-size-dash', 'file-size-inv');
            sizeCell.title = `Filesystem total: ${sizeCell.textContent} · ${files.toLocaleString()} files`;
        }
        if (itemsCell) {
            itemsCell.textContent = _fbFormatItemCount(files);
            itemsCell.classList.remove('file-items-loading', 'file-items-dash');
            itemsCell.title = `${files.toLocaleString()} files (recursive)`;
        }
    } catch (_) { /* CSS.escape unavailable */ }
}

/** Called after `renderFileList` finishes painting. Enumerates the visible
 *  folder rows and either replays from cache (instant) or queues a walk. */
function refreshFolderFilesystemSizes() {
    if (typeof document === 'undefined' || !_fileBrowserPath) return;
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderSize !== 'function') return;
    const folders = Array.from(document.querySelectorAll('.file-row.file-dir'))
        .map((row) => row.dataset.filePath)
        .filter(Boolean);
    if (folders.length === 0) return;
    // Bump the sequence so any queued jobs from a prior render are skipped
    // when the pump reaches them (see check in `_pumpFsFolderSizeQueue`).
    const seq = ++_fsFolderSizeSeq;
    for (const folderPath of folders) {
        if (_fsFolderSizeCache.has(folderPath)) {
            _paintFolderSizeCell(folderPath, _fsFolderSizeCache.get(folderPath));
            continue;
        }
        _fsFolderSizeQueue.push({folderPath, seq});
    }
    _pumpFsFolderSizeQueue();
}

/** Kick off (or replay from cache) the scan-status fetch for the currently
 *  visible folder rows. Called after `renderFileList` finishes its chunked
 *  paint. Failure is silent — badges are a hint, not a requirement. */
async function refreshFolderScanBadges() {
    if (typeof document === 'undefined' || !_fileBrowserPath) return;
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderScanStatus !== 'function') return;

    const folders = Array.isArray(_fileBrowserEntries)
        ? _fileBrowserEntries.filter((e) => e && e.isDir).map((e) => e.path)
        : [];
    if (folders.length === 0) return;

    const dirKey = _fileBrowserPath;
    const cached = _scanStatusCache.get(dirKey);
    if (cached) {
        // Replay from cache immediately — cheap, avoids backend hop on revisit.
        for (const f of folders) {
            const s = cached.get(f);
            if (s) _applyScanBadgesToRow(f, s);
        }
        return;
    }

    const seq = ++_scanStatusSeq;
    try {
        const result = await window.vstUpdater.fsFolderScanStatus(folders);
        if (seq !== _scanStatusSeq) return; // a newer render started; abandon this paint
        if (_fileBrowserPath !== dirKey) return; // directory changed mid-flight
        const map = new Map();
        for (const folder of folders) {
            const status = (result && result[folder]) || null;
            if (status) {
                map.set(folder, status);
                _applyScanBadgesToRow(folder, status);
            }
        }
        _scanStatusCache.set(dirKey, map);
    } catch (_) { /* badges are non-essential */ }
}

// ── File-list multi-select + bulk operations ──
// Selected entry paths for the CURRENT directory. Cleared on directory change
// (paths from the old dir would be invalid bulk-action targets). `var` for the
// same VM-sandbox visibility reason as the sort state below.
var _fileSelected = new Set();
// Index of the last interacted-with checkbox row (in the post-sort visible
// order). Used as the anchor for shift-click range select.
var _fileSelectLastIdx = -1;

function _fileEntryByPath(path) {
    if (!Array.isArray(_fileBrowserEntries)) return null;
    for (let i = 0; i < _fileBrowserEntries.length; i++) {
        if (_fileBrowserEntries[i].path === path) return _fileBrowserEntries[i];
    }
    return null;
}

function _setFileRowSelectedClass(path, selected) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row[data-file-path="${CSS.escape(path)}"]`);
        if (row) row.classList.toggle('file-selected', selected);
    } catch (_) { /* CSS.escape unavailable in old engines or test sandbox */ }
}

function updateFileBulkBar() {
    if (typeof document === 'undefined') return;
    const bar = document.getElementById('fileBulkBar');
    const count = document.getElementById('fileBulkCount');
    if (!bar) return;
    // `.fb-hidden` (CSS class with `display: none !important`) instead of
    // `style.display` — release WebKit handles class toggles more reliably
    // than inline display on dynamic flex containers.
    bar.classList.toggle('fb-hidden', _fileSelected.size === 0);
    if (count) count.textContent = String(_fileSelected.size);
    // Header `select-all` checkbox reflects current selection: checked when
    // all visible rows are selected, otherwise unchecked.
    const all = document.getElementById('fileRowCbAll');
    if (all) {
        const cbs = document.querySelectorAll('.file-row-cb');
        all.checked = cbs.length > 0 && _fileSelected.size >= cbs.length;
    }
}

function toggleFileSelect(path, selected) {
    if (selected) _fileSelected.add(path);
    else _fileSelected.delete(path);
    _setFileRowSelectedClass(path, selected);
    updateFileBulkBar();
}

function clearFileSelection() {
    const prev = [..._fileSelected];
    _fileSelected.clear();
    for (const p of prev) _setFileRowSelectedClass(p, false);
    if (typeof document !== 'undefined') {
        document.querySelectorAll('.file-row-cb').forEach((cb) => { cb.checked = false; });
    }
    _fileSelectLastIdx = -1;
    updateFileBulkBar();
}

function selectAllVisibleFiles() {
    if (typeof document === 'undefined') return;
    document.querySelectorAll('.file-row-cb').forEach((cb) => {
        cb.checked = true;
        const path = cb.dataset.fbCb;
        if (path) {
            _fileSelected.add(path);
            _setFileRowSelectedClass(path, true);
        }
    });
    updateFileBulkBar();
}

// Nautilus "Invert Selection" — every visible row that's currently
// selected becomes unselected, and vice versa.
function invertFileSelection() {
    if (typeof document === 'undefined') return;
    document.querySelectorAll('.file-row-cb').forEach((cb) => {
        const path = cb.dataset.fbCb;
        if (!path) return;
        const wasSelected = _fileSelected.has(path);
        if (wasSelected) {
            _fileSelected.delete(path);
            cb.checked = false;
            _setFileRowSelectedClass(path, false);
        } else {
            _fileSelected.add(path);
            cb.checked = true;
            _setFileRowSelectedClass(path, true);
        }
    });
    updateFileBulkBar();
}
if (typeof window !== 'undefined') window.invertFileSelection = invertFileSelection;

/**
 * Single source of truth for "what paths are bulk-action targets given the
 * current selection." Filters by predicate for actions that only make sense
 * for one kind of entry (folders for scan-as-*, files for open-in-default-app
 * et al.).
 */
function _fileBulkSelectionAsPaths(filterFn) {
    const out = [];
    for (const path of _fileSelected) {
        const entry = _fileEntryByPath(path);
        if (!entry) continue;
        if (filterFn && !filterFn(entry)) continue;
        out.push(path);
    }
    return out;
}

// ── File-list column resize ──
// Column widths live in CSS custom properties on `#tabFiles` so a single
// `style.setProperty('--fb-w-…')` write resizes header + every row together.
// Drag handlers are wired once at module load (delegated). Saved widths are
// keyed per column in `prefs.fileBrowserColWidths`.
var _fbColumnResizeWired = false;
const FB_RESIZABLE_COLS = ['ext', 'size', 'items', 'date', 'created'];

function saveFileColumnWidths() {
    if (typeof document === 'undefined') return;
    const tab = document.getElementById('tabFiles');
    if (!tab) return;
    const widths = {};
    for (const col of FB_RESIZABLE_COLS) {
        const v = tab.style.getPropertyValue(`--fb-w-${col}`);
        if (v) widths[col] = v.trim();
    }
    try { prefs.setItem('fileBrowserColWidths', widths); } catch (_) { /* ignore */ }
}

function loadFileColumnWidths() {
    if (typeof document === 'undefined') return;
    const tab = document.getElementById('tabFiles');
    if (!tab) return;
    try {
        const widths = prefs.getObject('fileBrowserColWidths', null);
        if (!widths || typeof widths !== 'object') return;
        for (const [col, w] of Object.entries(widths)) {
            if (!FB_RESIZABLE_COLS.includes(col)) continue;
            if (typeof w !== 'string' || !w) continue;
            tab.style.setProperty(`--fb-w-${col}`, w);
        }
    } catch (_) { /* ignore */ }
}

function initFileColumnResize() {
    if (_fbColumnResizeWired || typeof document === 'undefined') return;
    _fbColumnResizeWired = true;
    document.addEventListener('mousedown', (e) => {
        const handle = e.target && e.target.closest
            ? e.target.closest('.file-list-header .fb-col-resize')
            : null;
        if (!handle) return;
        const col = handle.dataset.fbResize;
        if (!FB_RESIZABLE_COLS.includes(col)) return;
        const tab = document.getElementById('tabFiles');
        if (!tab) return;
        e.preventDefault();
        e.stopPropagation();
        // Use the rendered px width (computed from the current CSS var) as the
        // starting point — handles the first-drag case where the var hasn't
        // been explicitly set yet.
        const headerCell = handle.closest('[data-fb-col]');
        const startWidth = headerCell ? headerCell.offsetWidth : 70;
        const startX = e.clientX;
        const minWidth = 40;
        handle.classList.add('resizing');
        document.body.classList.add('fb-col-resizing');
        function onMove(ev) {
            const delta = ev.clientX - startX;
            const next = Math.max(minWidth, startWidth + delta);
            tab.style.setProperty(`--fb-w-${col}`, `${next}px`);
        }
        function onUp() {
            handle.classList.remove('resizing');
            document.body.classList.remove('fb-col-resizing');
            document.removeEventListener('mousemove', onMove);
            document.removeEventListener('mouseup', onUp);
            saveFileColumnWidths();
        }
        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onUp);
    });
}

// ── File-list column sort ──
// Global (not per-directory) sort state. Stored in prefs as `{key, asc}`.
// Folders always sort first regardless of direction (standard file-browser
// convention — prevents folders from getting lost mid-list when sorting
// large directories by size/date).
// `var` (not `let`) so the VM-loaded test harness can read/mutate via
// `globalThis` to exercise the sort helpers without booting the full UI.
var _fileSortKey = 'name';
var _fileSortAsc = true;

function loadFileSortFromPrefs() {
    try {
        const raw = prefs.getObject('fileSort', null);
        if (raw && typeof raw === 'object') {
            if (['name', 'size', 'date', 'type', 'items', 'created'].includes(raw.key)) _fileSortKey = raw.key;
            if (typeof raw.asc === 'boolean') _fileSortAsc = raw.asc;
        }
    } catch (_) { /* ignore */ }
}

function saveFileSortToPrefs() {
    try {
        // Pass the raw object; `prefs.setItem` JSON-stringifies internally,
        // mirroring the `favDirs` save path above (and matching what
        // `prefs.getObject` expects to read back).
        prefs.setItem('fileSort', {key: _fileSortKey, asc: _fileSortAsc});
    } catch (_) { /* ignore */ }
}

/**
 * Sort a copy of the entries array by the current global sort key/direction.
 * Folders always come first within each direction. Secondary sort is always
 * name (asc) so equal-size or equal-date files stay alphabetical.
 */
function applyFileSort(entries) {
    const key = _fileSortKey;
    const dirMul = _fileSortAsc ? 1 : -1;
    const sorted = entries.slice();
    sorted.sort((a, b) => {
        if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
        let cmp = 0;
        switch (key) {
            case 'size':
                cmp = (Number(a.size) || 0) - (Number(b.size) || 0);
                break;
            case 'items':
                // `itemsCount` is set on folder entries by the bg walk in
                // `_pumpFsFolderSizeQueue`. Files don't have one — both sides
                // fall back to 0, which leaves files alphabetical via the
                // secondary name sort below.
                cmp = (Number(a.itemsCount) || 0) - (Number(b.itemsCount) || 0);
                break;
            case 'date':
                // `modified` is `YYYY-MM-DD HH:MM` (lexically sortable per `fs_list_dir`).
                cmp = String(a.modified || '').localeCompare(String(b.modified || ''));
                break;
            case 'created':
                // Same format as `modified`; empty when FS doesn't track
                // birthtime — empties sort to the start (asc) / end (desc)
                // since localeCompare treats them as smallest.
                cmp = String(a.created || '').localeCompare(String(b.created || ''));
                break;
            case 'type':
                cmp = String(a.ext || '').localeCompare(String(b.ext || ''));
                break;
            case 'name':
            default:
                cmp = String(a.name || '').toLowerCase().localeCompare(String(b.name || '').toLowerCase());
                break;
        }
        if (cmp === 0 && key !== 'name') {
            return String(a.name || '').toLowerCase().localeCompare(String(b.name || '').toLowerCase());
        }
        return cmp * dirMul;
    });
    return sorted;
}

function updateFileSortHeaderUI() {
    const header = document.getElementById('fileListHeader');
    if (!header) return;
    header.querySelectorAll('[data-sort]').forEach((el) => {
        el.classList.remove('sort-active', 'sort-asc', 'sort-desc');
    });
    const active = header.querySelector(`[data-sort="${_fileSortKey}"]`);
    if (active) {
        active.classList.add('sort-active', _fileSortAsc ? 'sort-asc' : 'sort-desc');
    }
}

function _onFileSortHeaderClick(e) {
    const target = e.target.closest('[data-sort]');
    if (!target) return;
    const key = target.dataset.sort;
    if (!['name', 'size', 'date', 'type', 'items', 'created'].includes(key)) return;
    if (_fileSortKey === key) {
        _fileSortAsc = !_fileSortAsc;
    } else {
        _fileSortKey = key;
        // Sensible default direction per column type — text columns ascend,
        // numeric/temporal columns descend (largest / newest first).
        _fileSortAsc = (key === 'name' || key === 'type');
        // size/items/date/created → desc by default (handled by the else above).
    }
    saveFileSortToPrefs();
    updateFileSortHeaderUI();
    if (typeof renderFileList === 'function') renderFileList();
}

if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const header = document.getElementById('fileListHeader');
        if (header) header.addEventListener('click', _onFileSortHeaderClick);
    });
}

async function initFileBrowser() {
    loadFileSortFromPrefs();
    updateFileSortHeaderUI();
    loadFileColumnWidths();
    initFileColumnResize();
    renderFavDirs();
    // Render tab bar even when revisiting the panel — handles the case
    // where prefs restored a tab list but the DOM wasn't drawn yet.
    renderFileBrowserTabs();
    // Apply persisted pane count to DOM (panes 2-4 unhide if user had >1).
    if (_fbPaneCount > 1) _fbSetPaneCount(_fbPaneCount);
    // Tab revisit: listing is still in the DOM (panel hidden, not destroyed) — avoid IPC + full re-render.
    if (_fileBrowserInited && _fileBrowserPath) {
        return;
    }
    // Prefer the persisted active tab's path over `_fileBrowserPath` —
    // tabs are the source of truth across launches. Falls through to
    // home / `/` only when no tabs were restored.
    const activeTab = _fbActiveTab();
    if (activeTab && activeTab.path) {
        await loadDirectory(activeTab.path);
        _fileBrowserInited = true;
        return;
    }
    if (_fileBrowserPath) {
        await loadDirectory(_fileBrowserPath);
        _fileBrowserInited = true;
        return;
    }
    // Start at home or first scan dir
    try {
        const home = await window.vstUpdater.getHomeDir();
        _fileBrowserPath = home;
        await loadDirectory(home);
    } catch {
        _fileBrowserPath = '/';
        await loadDirectory('/');
    }
    _fileBrowserInited = true;
}

async function loadDirectory(dirPath) {
    _fileListRenderSeq += 1;
    _fileBrowserPath = dirPath;
    // Mirror the new path into the active tab + persist + repaint the
    // tab bar so the label updates. If no tab exists yet (first ever
    // load), spin up the initial one — keeps the tab bar always
    // populated so users discover the +/× affordances.
    if (_fbTabs.length === 0) {
        const t = {id: _fbTabId(), path: dirPath};
        _fbTabs.push(t);
        _fbActiveTabId = t.id;
    } else {
        const active = _fbActiveTab();
        if (active) active.path = dirPath;
        else _fbActiveTabId = _fbTabs[0].id;
    }
    _fbTabsPersist();
    renderFileBrowserTabs();
    if (typeof _navHistoryRecord === 'function') _navHistoryRecord(dirPath);
    // Reset cursor cache when directory changes
    window._fbCursorPath = null;
    window._fbCursorEl = null;
    _wfQueue = [];
    // Selections are scoped to the current directory — paths from the old
    // listing would point at unrelated rows once the new listing renders.
    if (typeof clearFileSelection === 'function') clearFileSelection();
    // Drop the scan-status cache for this directory so badges re-fetch fresh
    // counts (catches the case where the user just ran a scan from another tab
    // and is revisiting the file browser).
    if (typeof _scanStatusCache !== 'undefined' && _scanStatusCache instanceof Map) {
        _scanStatusCache.delete(dirPath);
    }
    showGlobalProgress();
    try {
        const result = await window.vstUpdater.listDirectory(dirPath, _fbShowHidden);
        _fileBrowserEntries = result.entries;
        // Multi-pane: keep the active pane's snapshot in sync. Globals
        // are the source of truth for the active pane; this is a write-
        // through cache so a pane switch later reads correct data.
        const activePane = _fbActivePane();
        if (activePane) {
            activePane.path = dirPath;
            activePane.entries = _fileBrowserEntries;
            activePane.selection = _fileSelected;
            activePane.scroll = 0;
        }
        _fbPersistPanePaths();
        if (typeof fileBrowserRecordRecent === 'function') fileBrowserRecordRecent(dirPath, true);
        renderFileList();
        renderBreadcrumb(dirPath);
        updateBookmarkBtn();
        // Refresh the tree-view sidebar's active-row highlight + expose
        // the newly-entered folder if its parent chain happens to be
        // already-expanded. No-op when the sidebar is hidden.
        const sb = document.getElementById('fbTreeSidebar');
        if (sb && !sb.classList.contains('fb-hidden') && typeof renderFileBrowserTree === 'function') {
            renderFileBrowserTree();
        }
        // Git status — fire-and-forget IPC; on response we patch the
        // already-rendered rows in place instead of redrawing the whole
        // listing (avoids a flash + preserves scroll/selection). Outside
        // a git repo the response is an empty map and nothing changes.
        window.vstUpdater.fsGitStatus(dirPath).then((status) => {
            _fbGitStatus = status || {};
            if (Object.keys(_fbGitStatus).length === 0) return;
            // Patch in-place: for every row whose path is in the map,
            // inject a badge into its `.file-meta-tags-row` (creating
            // the container if needed). Keeps incremental — no full
            // renderFileList re-run.
            document.querySelectorAll('#fileList .file-row').forEach((row) => {
                const p = row.dataset.filePath;
                if (!p || !_fbGitStatus[p]) return;
                if (row.querySelector('.fb-git-badge')) return;
                const code = _fbGitStatus[p];
                const t = code.trim();
                let cls = 'fb-git-clean';
                if (t === '??') cls = 'fb-git-untracked';
                else if (t.startsWith('U') || t.endsWith('U') || t === 'AA' || t === 'DD') cls = 'fb-git-conflict';
                else if (t.startsWith('A') || t.startsWith('?')) cls = 'fb-git-added';
                else if (t.startsWith('M') || t.endsWith('M')) cls = 'fb-git-modified';
                else if (t.startsWith('D') || t.endsWith('D')) cls = 'fb-git-deleted';
                else if (t.startsWith('R') || t.startsWith('C')) cls = 'fb-git-renamed';
                const nameCell = row.querySelector('.file-name');
                if (!nameCell) return;
                let tagsRow = nameCell.querySelector('.file-meta-tags-row');
                if (!tagsRow) {
                    tagsRow = document.createElement('span');
                    tagsRow.className = 'file-meta-tags-row';
                    nameCell.appendChild(tagsRow);
                }
                const badge = document.createElement('span');
                badge.className = `file-meta-tag fb-git-badge ${cls}`;
                badge.title = `Git status: ${code}`;
                badge.textContent = t || code;
                tagsRow.appendChild(badge);
            });
        }).catch(() => { _fbGitStatus = {}; });
        // Swap the auto-reload watcher to this directory. Fire-and-forget —
        // the listing already painted; if Rust can't watch (path gone, EACCES)
        // the worst case is no auto-reload, not a failed navigation. Canonical
        // path is stored in `_fbWatchedCanonical` so the event listener can
        // compare strict-equal against the watcher's reply (the path Rust
        // canonicalizes may differ from `dirPath` by /System/Volumes/Data
        // prefix, symlinks, trailing slash).
        window.vstUpdater.fbWatcherSet(dirPath)
            .then((canonical) => { _fbWatchedCanonical = canonical || null; })
            .catch((err) => console.warn('fb_watcher_set failed:', err));
    } catch (err) {
        showToast(toastFmt('toast.failed_open_directory', {err: err.message || err}), 4000, 'error');
    } finally {
        hideGlobalProgress();
    }
}

// ── Auto-reload on disk change ──
// Canonical path Rust is watching (set by `loadDirectory` after the IPC
// returns). Compared strict-equal against the event payload so an in-flight
// event from a previously-watched dir (the user navigated mid-burst) can't
// cause a stale reload.
var _fbWatchedCanonical = null;
// Coalesce bursts that arrive after Rust's 300 ms debounce — e.g. on macOS
// FSEvents can deliver coordinated changes (create + chmod + rename) as
// two emits 50 ms apart. 150 ms here means at most one reload per ~half-
// second of activity, which feels instant but never thrashes.
var _fbReloadPending = false;
function _fbScheduleReload() {
    if (_fbReloadPending) return;
    _fbReloadPending = true;
    setTimeout(() => {
        _fbReloadPending = false;
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
    }, 150);
}
if (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.event) {
    window.__TAURI__.event.listen('file-browser-change', (e) => {
        const payloadDir = e && e.payload && e.payload.dir;
        if (!payloadDir) return;
        // Ignore stale events from a folder we navigated away from.
        if (payloadDir !== _fbWatchedCanonical) return;
        _fbScheduleReload();
    });
}

function renderBreadcrumb(dirPath) {
    const el = document.getElementById('fileBreadcrumb');
    if (!el) return;
    const norm = normalizePathSeparators(dirPath);

    if (/^[A-Za-z]:/.test(norm)) {
        const drive = norm.slice(0, 2);
        const rest = norm.slice(2).replace(/^\/+/, '');
        const segments = rest ? rest.split('/').filter(Boolean) : [];
        let html = `<span class="file-crumb" data-file-nav="${escapeHtml(drive + '/')}" title="${escapeHtml(drive)}">${escapeHtml(drive)}</span>`;
        let acc = drive;
        for (const part of segments) {
            acc = acc.endsWith(':') ? acc + '/' + part : acc + '/' + part;
            html += `<span class="file-crumb-sep">/</span><span class="file-crumb" data-file-nav="${escapeHtml(acc)}">${escapeHtml(part)}</span>`;
        }
        el.innerHTML = html;
        return;
    }

    const parts = norm.split('/').filter(Boolean);
    let html = `<span class="file-crumb" data-file-nav="/">/</span>`;
    let accumulated = '';
    for (const part of parts) {
        accumulated += '/' + part;
        html += `<span class="file-crumb-sep">/</span><span class="file-crumb" data-file-nav="${escapeHtml(accumulated)}">${escapeHtml(part)}</span>`;
    }
    el.innerHTML = html;
}

function buildFileListRowHtml(e, search, sampleByPath, mode) {
    const note = typeof noteIndicator === 'function' ? noteIndicator(e.path) : '';
    const cls = e.isDir ? ' file-dir' : '';
    const isAudio = !e.isDir && AUDIO_EXTS.includes(e.ext);

    const parts = [];
    if (typeof isFavorite === 'function' && isFavorite(e.path)) {
        parts.push('<span class="file-meta-tag file-meta-fav" title="Favorited">&#9733;</span>');
    }
    // Git status badge — `_fbGitStatus` is loaded asynchronously after
    // each loadDirectory. Path keys match what git porcelain hands back
    // (resolved via `git rev-parse --show-toplevel` in the Rust side).
    if (typeof _fbGitStatus !== 'undefined' && _fbGitStatus && _fbGitStatus[e.path]) {
        const code = _fbGitStatus[e.path];
        // Color-code by status family — Nautilus / VSCode convention:
        //   modified → orange, added/new → green, untracked → cyan,
        //   conflict → red, anything else → grey.
        const t = code.trim();
        let cls = 'fb-git-clean';
        if (t === '??') cls = 'fb-git-untracked';
        else if (t.startsWith('U') || t.endsWith('U') || t === 'AA' || t === 'DD') cls = 'fb-git-conflict';
        else if (t.startsWith('A') || t.startsWith('?')) cls = 'fb-git-added';
        else if (t.startsWith('M') || t.endsWith('M')) cls = 'fb-git-modified';
        else if (t.startsWith('D') || t.endsWith('D')) cls = 'fb-git-deleted';
        else if (t.startsWith('R') || t.startsWith('C')) cls = 'fb-git-renamed';
        parts.push(`<span class="file-meta-tag fb-git-badge ${cls}" title="Git status: ${escapeHtml(code)}">${escapeHtml(t || code)}</span>`);
    }
    if (typeof getNote === 'function') {
        const n = getNote(e.path);
        if (n && n.tags && n.tags.length > 0) {
            parts.push(`<span class="file-meta-tag file-meta-tags" title="Tags: ${escapeHtml(n.tags.join(', '))}">${escapeHtml(n.tags.slice(0, 2).join(', '))}${n.tags.length > 2 ? '…' : ''}</span>`);
        }
    }
    if (isAudio) {
        if (typeof _bpmCache !== 'undefined' && _bpmCache[e.path]) {
            parts.push(`<span class="file-meta-tag file-meta-bpm" title="BPM">${_bpmCache[e.path]}</span>`);
        }
        if (typeof _keyCache !== 'undefined' && _keyCache[e.path]) {
            parts.push(`<span class="file-meta-tag file-meta-key" title="Musical key">${escapeHtml(_keyCache[e.path])}</span>`);
        }
        if (sampleByPath) {
            const sample = sampleByPath.get(e.path);
            if (sample && sample.duration) {
                parts.push(`<span class="file-meta-tag file-meta-dur" title="Duration">${typeof formatTime === 'function' ? formatTime(sample.duration) : sample.duration.toFixed(1) + 's'}</span>`);
            }
        }
    }
    const extras = parts.length > 0 ? `<span class="file-meta-tags-row">${parts.join('')}</span>` : '';

    const wfBg = isAudio ? `<canvas class="file-waveform" data-wf-path="${escapeHtml(e.path)}" height="36" title="Waveform"></canvas><span class="file-wf-cursor"></span>` : '';
    // Inline image thumbnails for picture rows. The icon cell holds a
    // canvas instead of the doc-glyph; `_fbThumbObserver` lazy-loads the
    // bytes via `fs_image_thumbnail` (cached in SQLite). SVG is skipped
    // server-side (image crate doesn't decode SVG); .heic too.
    const isImageThumb = !e.isDir && ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'tiff', 'tif'].includes(e.ext);
    const isSelected = _fileSelected.has(e.path);
    // Folders show "…" placeholders for Size + Items until the background
    // walk completes (see `refreshFolderFilesystemSizes` →
    // `_paintFolderSizeCell`). Files have known sizes from `fs_list_dir` so
    // those cells fill on first paint; the Items cell is folder-only.
    const sizeContent = e.isDir
        ? '<span class="file-size-loading-text">…</span>'
        : e.sizeFormatted;
    const sizeCls = e.isDir ? ' file-size-loading' : '';
    const itemsContent = e.isDir
        ? '<span class="file-items-loading-text">…</span>'
        : '';
    const itemsCls = e.isDir ? ' file-items-loading' : '';
    // Color label ring — Finder convention. Inserted into the icon cell
    // as an absolutely-positioned span so it overlays the glyph corner.
    const labelIdx = (typeof fileBrowserGetLabel === 'function') ? fileBrowserGetLabel(e.path) : 0;
    const labelRing = labelIdx > 0
        ? `<span class="fb-label-ring" style="background:${FB_LABEL_COLORS[labelIdx]}" title="Label: ${FB_LABEL_NAMES[labelIdx]}"></span>`
        : '';
    const iconCell = isImageThumb
        ? `<span class="file-icon file-icon-thumb"><canvas class="file-image-thumb" data-fb-thumb-path="${escapeHtml(e.path)}" width="32" height="32"></canvas>${labelRing}</span>`
        : `<span class="file-icon">${fileIcon(e)}${labelRing}</span>`;
    return `<div class="file-row${cls}${isAudio ? ' file-audio' : ''}${isSelected ? ' file-selected' : ''}" data-file-path="${escapeHtml(e.path)}" data-file-dir="${e.isDir}" draggable="true" ${isAudio ? `data-wf-file="${escapeHtml(e.path)}"` : ''}>
      <span class="file-cb"><input type="checkbox" class="file-row-cb" data-fb-cb="${escapeHtml(e.path)}"${isSelected ? ' checked' : ''}></span>
      ${wfBg}
      ${iconCell}
      <span class="file-name">${search && typeof highlightMatch === 'function' ? highlightMatch(e.name, search, mode || 'fuzzy') : escapeHtml(e.name)}${extras}${note}</span>
      <span class="file-ext">${e.isDir ? 'DIR' : e.ext}</span>
      <span class="file-size${sizeCls}">${sizeContent}</span>
      <span class="file-items${itemsCls}">${itemsContent}</span>
      <span class="file-date">${e.modified}</span>
      <span class="file-created">${e.created || ''}</span>
    </div>`;
}

function renderFileList() {
    // Multi-pane: always render into the ACTIVE pane's list element.
    // For pane 0 this is still `#fileList` (kept for backwards compat
    // with the 9 callers that query that ID); for panes 1-3 it's the
    // matching `.fb-pane[data-pane-idx="N"] .file-list`.
    const list = (typeof _fbActiveListEl === 'function')
        ? (_fbActiveListEl() || document.getElementById('fileList'))
        : document.getElementById('fileList');
    if (!list) return;
    const search = (document.getElementById('fileSearchInput')?.value || '').trim();
    const mode = _lastFilesMode;
    // Extension chip filter applies BEFORE search/sort — chips narrow the
    // candidate pool, then search ranks within that pool. Folders are always
    // kept regardless of chip (see `_fbExtMatches`) so the user can navigate.
    const preFiltered = (_fbExtFilter && _fbExtFilter !== 'all')
        ? _fileBrowserEntries.filter((e) => _fbExtMatches(e, _fbExtFilter))
        : _fileBrowserEntries;
    let filtered;
    if (search) {
        const scored = preFiltered.map(e => {
            const score = typeof searchScore === 'function' ? searchScore(search, [e.name, e.ext], mode) : (e.name.toLowerCase().includes(search.toLowerCase()) ? 1 : 0);
            return {entry: e, score};
        }).filter(s => s.score > 0);
        // Search results stay in score order (best match first) — manual
        // sorting kicks in only for the unfiltered listing where there's no
        // semantic ranking to preserve.
        scored.sort((a, b) => b.score - a.score);
        filtered = scored.map(s => s.entry);
    } else {
        filtered = applyFileSort(preFiltered);
    }
    if (typeof updateFileListFooter === 'function') updateFileListFooter(filtered);

    if (filtered.length === 0) {
        _fileListRenderSeq += 1;
        list.innerHTML = '<div class="state-message"><div class="state-icon">&#128193;</div><h2>Empty Directory</h2></div>';
        return;
    }

    let sampleByPath = null;
    if (typeof allAudioSamples !== 'undefined' && Array.isArray(allAudioSamples) && allAudioSamples.length > 0) {
        sampleByPath = new Map();
        for (let i = 0; i < allAudioSamples.length; i++) {
            const s = allAudioSamples[i];
            if (s && s.path) sampleByPath.set(s.path, s);
        }
    }

    const seq = ++_fileListRenderSeq;
    list.innerHTML = '';
    let idx = 0;

    function appendChunk() {
        if (seq !== _fileListRenderSeq) return;
        const end = Math.min(idx + FILE_LIST_CHUNK, filtered.length);
        const slice = filtered.slice(idx, end);
        const html = slice.map((e) => buildFileListRowHtml(e, search, sampleByPath, mode)).join('');
        list.insertAdjacentHTML('beforeend', html);
        idx = end;
        if (idx < filtered.length) {
            const cont = appendChunk;
            if (typeof yieldToBrowser === 'function') {
                yieldToBrowser().then(cont);
            } else {
                setTimeout(cont, 0);
            }
        } else {
            requestAnimationFrame(() => initFileBrowserWaveforms());
            // Folder scan-status badges + filesystem-size walks — fire-and-forget
            // after the full list is painted so they can't delay the visible
            // render. Badges paint instantly from one batched SQL query;
            // filesystem-size walks are bounded-concurrency and stream in as
            // each per-folder walk completes.
            if (typeof refreshFolderScanBadges === 'function') {
                requestAnimationFrame(() => { refreshFolderScanBadges(); });
            }
            if (typeof refreshFolderFilesystemSizes === 'function') {
                requestAnimationFrame(() => { refreshFolderFilesystemSizes(); });
            }
        }
    }

    appendChunk();
}

// ── Lazy waveform rendering for file browser audio rows ──
let _wfQueue = [];
let _wfActive = 0;
// Reduced from 4 to 2 to avoid saturating IPC during heavy background jobs
const _wfMaxConcurrent = 2;
// Pause waveform loading during audio playback (SMB contention)
let _wfPausedForPlayback = false;

function setWaveformPausedForPlayback(paused) {
    _wfPausedForPlayback = paused;
    if (!paused) _processWfQueue(); // Resume queue processing
}
window.setWaveformPausedForPlayback = setWaveformPausedForPlayback;

function _processWfQueue() {
    if (_wfPausedForPlayback) return; // Pause during playback load
    while (_wfActive < _wfMaxConcurrent && _wfQueue.length > 0) {
        if (_wfPausedForPlayback) return; // Check again before each item
        const {canvas, path} = _wfQueue.shift();
        _wfActive++;
        drawMiniWaveform(canvas, path).finally(() => {
            _wfActive--;
            _processWfQueue();
        });
    }
}

let _fbWfObserver = null;

function initFileBrowserWaveforms() {
    const container = document.getElementById('fileList');
    if (!container) return;
    const canvases = container.querySelectorAll('canvas.file-waveform');
    if (canvases.length === 0) return;

    // Disconnect previous observer to prevent leak
    if (_fbWfObserver) {
        _fbWfObserver.disconnect();
        _fbWfObserver = null;
    }

    const observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            const canvas = entry.target;
            if (canvas._wfDrawn) continue;
            canvas._wfDrawn = true;
            observer.unobserve(canvas);
            _wfQueue.push({canvas, path: canvas.dataset.wfPath});
        }
        _processWfQueue();
    }, {root: container.closest('.tab-content'), threshold: 0.1});

    _fbWfObserver = observer;
    canvases.forEach(c => observer.observe(c));
}

async function drawMiniWaveform(canvas, filePath) {
    // Size canvas to parent row width
    const row = canvas.closest('.file-row');
    if (row) canvas.width = row.offsetWidth;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;

    // Check cache first
    if (typeof _waveformCache !== 'undefined' && _waveformCache[filePath]) {
        renderMiniWf(ctx, w, h, _waveformCache[filePath]);
        return;
    }

    const widthPx = Math.max(32, Math.min(Math.floor(w) || 800, 8192));

    try {
        let peaks = null;
        if (typeof window._fetchWaveformPeaksFromAudioEngine === 'function') {
            peaks = await window._fetchWaveformPeaksFromAudioEngine(filePath, widthPx);
        }
        if (!peaks) {
            const src = typeof convertFileSrc === 'function' ? convertFileSrc(filePath) : filePath;
            if (typeof window._decodePeaksViaWorker === 'function') {
                peaks = await window._decodePeaksViaWorker(src, widthPx);
            } else {
                if (!window._fbAudioCtx) window._fbAudioCtx = new AudioContext();
                const resp = await fetch(src);
                const buf = await resp.arrayBuffer();
                const audioBuf = await window._fbAudioCtx.decodeAudioData(buf);
                const raw = audioBuf.getChannelData(0);

                const bars = widthPx;
                const step = Math.floor(raw.length / bars);
                peaks = [];
                for (let i = 0; i < bars; i++) {
                    let max = 0, min = 0;
                    const start = i * step;
                    for (let j = start; j < start + step && j < raw.length; j++) {
                        if (raw[j] > max) max = raw[j];
                        if (raw[j] < min) min = raw[j];
                    }
                    peaks.push({max, min});
                }
            }
        }

        if (typeof window._storeWaveformPeaksInCache === 'function') {
            window._storeWaveformPeaksInCache(filePath, peaks);
        } else if (typeof _waveformCache !== 'undefined') {
            _waveformCache[filePath] = peaks;
        }
        renderMiniWf(ctx, w, h, peaks);
    } catch {
        // Draw flat line
        ctx.strokeStyle = 'rgba(5,217,232,0.2)';
        ctx.beginPath();
        ctx.moveTo(0, h / 2);
        ctx.lineTo(w, h / 2);
        ctx.stroke();
    }
}

function renderMiniWf(ctx, w, h, peaks) {
    const mid = h / 2;
    const isNew = peaks.length > 0 && typeof peaks[0] === 'object';
    ctx.clearRect(0, 0, w, h);

    if (isNew) {
        ctx.beginPath();
        ctx.moveTo(0, mid);
        for (let i = 0; i < peaks.length; i++) {
            ctx.lineTo(i, mid - peaks[i].max * mid * 0.9);
        }
        for (let i = peaks.length - 1; i >= 0; i--) {
            ctx.lineTo(i, mid - peaks[i].min * mid * 0.9);
        }
        ctx.closePath();
        const grad = ctx.createLinearGradient(0, 0, w, 0);
        grad.addColorStop(0, 'rgba(5,217,232,0.4)');
        grad.addColorStop(1, 'rgba(211,0,197,0.4)');
        ctx.fillStyle = grad;
        ctx.fill();
    } else {
        for (let i = 0; i < peaks.length; i++) {
            const barH = (typeof peaks[i] === 'number' ? peaks[i] : 0) * mid * 0.9;
            ctx.fillStyle = 'rgba(5,217,232,0.4)';
            ctx.fillRect(i, mid - barH, 1, barH * 2);
        }
    }
}

// ── Expandable metadata panel for audio files in file browser ──
let _fbExpandedPath = null;

async function toggleFileBrowserMeta(filePath) {
    const list = document.getElementById('fileList');
    if (!list) return;
    const existing = document.getElementById('fbMetaPanel');
    if (existing) {
        const wasPath = existing.dataset.metaPath;
        existing.remove();
        if (wasPath === filePath) {
            _fbExpandedPath = null;
            return;
        }
    }

    _fbExpandedPath = filePath;
    const row = list.querySelector(`.file-row[data-file-path="${CSS.escape(filePath)}"]`);
    if (!row) return;

    // Insert loading panel
    const panel = document.createElement('div');
    panel.id = 'fbMetaPanel';
    panel.dataset.metaPath = filePath;
    panel.className = 'fb-meta-panel';
    panel.innerHTML = '<div style="text-align:center;padding:12px;"><div class="spinner" style="width:14px;height:14px;margin:0 auto;"></div></div>';
    row.after(panel);

    try {
        const meta = await window.vstUpdater.getAudioMetadata(filePath);
        if (_fbExpandedPath !== filePath) return;
        const p = document.getElementById('fbMetaPanel');
        if (!p) return;

        const mi = (label, value) => value ? `<div class="fb-meta-item" title="${escapeHtml(label)}: ${escapeHtml(String(value))}"><span class="fb-meta-label">${label}</span><span class="fb-meta-val">${escapeHtml(String(value))}</span></div>` : '';

        let html = '<div class="fb-meta-grid">';
        html += mi('Format', meta.format);
        html += mi('Size', typeof formatAudioSize === 'function' ? formatAudioSize(meta.sizeBytes) : meta.sizeBytes);
        if (meta.sampleRate) html += mi('Sample Rate', meta.sampleRate.toLocaleString() + ' Hz');
        if (meta.bitsPerSample) html += mi('Bit Depth', meta.bitsPerSample + '-bit');
        if (meta.channels) html += mi('Channels', meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch');
        if (meta.duration) html += mi('Duration', typeof formatTime === 'function' ? formatTime(meta.duration) : meta.duration.toFixed(1) + 's');
        if (meta.byteRate) html += mi('Byte Rate', (typeof formatAudioSize === 'function' ? formatAudioSize(meta.byteRate) : meta.byteRate) + '/s');

        // BPM
        html += `<div class="fb-meta-item" title="BPM"><span class="fb-meta-label">BPM</span><span class="fb-meta-val" id="fbBpmVal"><span class="spinner" style="width:8px;height:8px;"></span></span></div>`;
        // Key
        html += `<div class="fb-meta-item" title="Musical Key"><span class="fb-meta-label">Key</span><span class="fb-meta-val" id="fbKeyVal"><span class="spinner" style="width:8px;height:8px;"></span></span></div>`;

        const fmtDate = (v) => {
            if (!v) return '—';
            const d = new Date(v);
            return isNaN(d) ? '—' : d.toLocaleString();
        };
        html += mi('Created', fmtDate(meta.created));
        html += mi('Modified', fmtDate(meta.modified));
        html += mi('Permissions', meta.permissions);
        html += mi('Path', meta.fullPath);
        html += '</div>';

        // Favorite, Notes, Tags as grid items
        const isFav = typeof isFavorite === 'function' && isFavorite(filePath);
        html += mi('Favorite', isFav ? '★ Yes' : '☆ No');
        const noteData = typeof getNote === 'function' ? getNote(filePath) : null;
        const tags = noteData?.tags?.length ? noteData.tags.join(', ') : '';
        html += mi('Tags', tags || '—');
        const noteText = noteData?.note || '';
        if (noteText) html += mi('Note', noteText);

        p.innerHTML = html;

        // Async BPM + Key
        const bpmFormats = ['wav', 'aiff', 'aif', 'mp3', 'flac', 'ogg', 'm4a', 'aac', 'opus'];
        if (bpmFormats.includes(meta.format?.toLowerCase() || '')) {
            // BPM
            (async () => {
                try {
                    if (typeof _bpmCache !== 'undefined' && _bpmCache[filePath] !== undefined) {
                        const el = document.getElementById('fbBpmVal');
                        if (el) el.textContent = _bpmCache[filePath] ? _bpmCache[filePath] + ' BPM' : '—';
                        return;
                    }
                    const bpm = await window.vstUpdater.estimateBpm(filePath);
                    if (typeof _bpmCache !== 'undefined') _bpmCache[filePath] = bpm;
                    const el = document.getElementById('fbBpmVal');
                    if (el && _fbExpandedPath === filePath) el.textContent = bpm ? bpm + ' BPM' : '—';
                } catch {
                    const el = document.getElementById('fbBpmVal');
                    if (el) el.textContent = '—';
                }
            })();
            // Key
            (async () => {
                try {
                    if (typeof _keyCache !== 'undefined' && _keyCache[filePath] !== undefined) {
                        const el = document.getElementById('fbKeyVal');
                        if (el) el.textContent = _keyCache[filePath] || '—';
                        return;
                    }
                    const key = await window.vstUpdater.detectAudioKey(filePath);
                    if (typeof _keyCache !== 'undefined') _keyCache[filePath] = key;
                    const el = document.getElementById('fbKeyVal');
                    if (el && _fbExpandedPath === filePath) el.textContent = key || '—';
                } catch {
                    const el = document.getElementById('fbKeyVal');
                    if (el) el.textContent = '—';
                }
            })();
        } else {
            const bEl = document.getElementById('fbBpmVal');
            const kEl = document.getElementById('fbKeyVal');
            if (bEl) bEl.textContent = '—';
            if (kEl) kEl.textContent = '—';
        }
    } catch (err) {
        const p = document.getElementById('fbMetaPanel');
        if (p) p.innerHTML = `<div style="padding:8px;color:var(--red);font-size:11px;">Failed: ${escapeHtml(err.message || String(err))}</div>`;
    }
}

// Click to navigate dirs or play/open files
document.addEventListener('click', (e) => {
    const crumb = e.target.closest('[data-file-nav]');
    if (crumb) {
        loadDirectory(crumb.dataset.fileNav);
        return;
    }

    const row = e.target.closest('.file-row');
    // Clicks inside the checkbox cell (the cell wrapper OR the input itself)
    // are exclusively for selection — they must not also trigger the row's
    // navigate / preview action. `stopPropagation` in the checkbox handler
    // (registered separately on document) doesn't help here because both
    // listeners are on the same target; only ordering + a guard does.
    if (row && !e.target.closest('.fb-meta-panel') && !e.target.closest('.file-cb')) {
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            loadDirectory(path);
        } else {
            const ext = path.split('.').pop().toLowerCase();
            if (AUDIO_EXTS.includes(ext)) {
                previewAudio(path);
                toggleFileBrowserMeta(path);
            }
            // Non-audio non-folder files (.pdf / .txt / .png / etc.): single
            // click is reserved for selection + preview-pane population (which
            // is wired in a separate handler below). Double-click opens in
            // default app — see the `dblclick` listener further down.
        }
        return;
    }
});

// Files tab — double-click on a non-audio file opens in default app
// (single-click is reserved for selection + preview pane).
document.addEventListener('dblclick', (e) => {
    const row = e.target instanceof Element ? e.target.closest('.file-row') : null;
    if (!row) return;
    if (e.target.closest('.fb-meta-panel') || e.target.closest('.file-cb')
        || e.target.closest('.fb-rename-input')) return;
    const path = row.dataset.filePath;
    const isDir = row.dataset.fileDir === 'true';
    if (isDir) return; // single-click already navigates dirs
    const ext = path.split('.').pop().toLowerCase();
    if (typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext)) return; // audio uses single-click play
    e.preventDefault();
    if (typeof opener_open === 'function') opener_open(path);
});

function opener_open(path) {
    // DAW project formats first (.als / .flp / .logicx etc. → Ableton, FL Studio,
    // Logic). When the path isn't a DAW project (or the DAW isn't installed), fall
    // back to the OS default-app opener instead of `openPresetFolder` (which
    // *revealed the parent folder* — surprising for files like .txt / .png where
    // the user expected the file itself to open). With this fallback a click on
    // foo.txt opens TextEdit / Notepad / xdg-open per platform; a click on foo.als
    // still opens Ableton via the DAW path.
    if (typeof fileBrowserRecordRecent === 'function') fileBrowserRecordRecent(path, false);
    window.vstUpdater.openDawProject(path).catch(() => {
        window.vstUpdater.openFileDefault(path).catch(() => {});
    });
}

// Action handlers
document.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action]');
    if (!action) return;
    if (action.dataset.action === 'fileUp') {
        if (_fileBrowserPath) {
            const n = normalizePathSeparators(_fileBrowserPath);
            if (n !== '/' && !/^[A-Za-z]:\/?$/.test(n.replace(/\/+$/, ''))) {
                const parent = parentDirectoryPath(_fileBrowserPath);
                if (normalizePathSeparators(parent) !== n.replace(/\/+$/, '')) {
                    loadDirectory(parent);
                }
            }
        }
    } else if (action.dataset.action === 'fileHome') {
        window.vstUpdater.getHomeDir().then(h => loadDirectory(h)).catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        });
    } else if (action.dataset.action === 'fileQuickNav') {
        const dir = action.dataset.dir;
        if (dir === '/') {
            loadDirectory('/');
        } else {
            window.vstUpdater.getHomeDir().then((h) => {
                const home = normalizePathSeparators(h).replace(/\/+$/, '');
                loadDirectory(home + '/' + dir);
            }).catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            });
        }
    } else if (action.dataset.action === 'fileAppDataDir') {
        const vu = window.vstUpdater;
        if (!vu || typeof vu.getPrefsPath !== 'function') {
            if (typeof showToast === 'function') showToast(String('getPrefsPath unavailable'), 4000, 'error');
            return;
        }
        vu.getPrefsPath()
            .then((p) => {
                const norm = normalizePathSeparators(String(p || ''));
                const dir = norm.replace(/\/[^/]+$/, '');
                if (!dir) {
                    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                        showToast(toastFmt('toast.failed', {err: 'invalid preferences path'}), 4000, 'error');
                    }
                    return;
                }
                loadDirectory(dir);
            })
            .catch((e) => {
                if (typeof showToast === 'function') showToast(String(e && e.message ? e.message : e), 4000, 'error');
            });
    } else if (action.dataset.action === 'fileFav') {
        if (_fileBrowserPath) {
            if (isFavDir(_fileBrowserPath)) removeFavDir(_fileBrowserPath);
            else addFavDir(_fileBrowserPath);
        }
    }
});

// Fav directory chip clicks
document.addEventListener('click', (e) => {
    const remove = e.target.closest('[data-remove-fav-dir]');
    if (remove) {
        e.stopPropagation();
        removeFavDir(remove.dataset.removeFavDir);
        return;
    }
    const chip = e.target.closest('[data-fav-dir]');
    if (chip) {
        loadDirectory(chip.dataset.favDir);
    }
});

// ── Multi-select interactions ──
// Per-row checkbox click: toggles single row, or with Shift, range-selects from
// the last interacted-with row to this one (inclusive). `stopPropagation` so the
// row's open/preview click handler doesn't fire on the checkbox click itself.
document.addEventListener('click', (e) => {
    const cb = e.target.closest('.file-row-cb');
    if (cb) {
        // `stopImmediatePropagation` (not just `stopPropagation`) — multiple
        // click listeners are attached to `document`; we have to prevent the
        // sibling row-click handler from also firing on this same event.
        e.stopImmediatePropagation();
        const path = cb.dataset.fbCb;
        if (!path) return;
        const allRows = Array.from(document.querySelectorAll('.file-row-cb'));
        const idx = allRows.indexOf(cb);
        if (e.shiftKey && _fileSelectLastIdx >= 0 && idx >= 0 && idx !== _fileSelectLastIdx) {
            const lo = Math.min(_fileSelectLastIdx, idx);
            const hi = Math.max(_fileSelectLastIdx, idx);
            const want = cb.checked;
            for (let i = lo; i <= hi; i++) {
                const c = allRows[i];
                if (!c) continue;
                c.checked = want;
                const p = c.dataset.fbCb;
                if (!p) continue;
                if (want) _fileSelected.add(p);
                else _fileSelected.delete(p);
                _setFileRowSelectedClass(p, want);
            }
            updateFileBulkBar();
        } else {
            toggleFileSelect(path, cb.checked);
        }
        _fileSelectLastIdx = idx;
        return;
    }
    const allCb = e.target.closest('.file-row-cb-all');
    if (allCb) {
        e.stopImmediatePropagation();
        if (allCb.checked) selectAllVisibleFiles();
        else clearFileSelection();
        return;
    }
});

// ── Bulk-action toolbar buttons ──
document.addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-action^="fileBulk"]');
    if (!btn) return;
    e.stopPropagation();
    const action = btn.dataset.action;
    if (action === 'fileBulkClear') {
        clearFileSelection();
        return;
    }
    const paths = [..._fileSelected];
    if (paths.length === 0) return;
    if (action === 'fileBulkFavorite') {
        let count = 0;
        for (const p of paths) {
            const entry = _fileEntryByPath(p);
            if (!entry) continue;
            if (entry.isDir) {
                if (typeof addFavDir === 'function') { addFavDir(p); count++; }
            } else if (typeof addFavorite === 'function') {
                const name = p.split('/').pop();
                const ext = (name.split('.').pop() || '').toLowerCase();
                const kind = (typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext)) ? 'sample' : 'file';
                addFavorite(kind, p, name, {format: ext.toUpperCase()});
                count++;
            }
        }
        if (typeof showToast === 'function' && count > 0) {
            showToast(toastFmt('toast.added_favorites_batch', {n: count}));
        }
        return;
    }
    if (action === 'fileBulkRename') {
        showFileBulkRenameModal();
        return;
    }
    if (action === 'fileBulkOpen') {
        for (const p of paths) {
            if (window.vstUpdater && typeof window.vstUpdater.openFileDefault === 'function') {
                window.vstUpdater.openFileDefault(p).catch(() => {});
            }
        }
        return;
    }
    if (action === 'fileBulkDelete') {
        // Bulk toolbar Delete moves to Trash (recoverable). Modal text is
        // trash-specific so the user knows the action isn't permanent.
        const target = paths.length === 1
            ? `"${paths[0].split('/').pop()}"`
            : `${paths.length} items`;
        const msg = `Move ${target} to Trash?`;
        // Prefer the in-app modal (`confirmAction`) over native `confirm()` —
        // native confirm is unreliable in Tauri WKWebView (silently dismissed
        // in some release builds).
        const ok = typeof confirmAction === 'function'
            ? await confirmAction(msg, 'Move to Trash')
            : confirm(msg);
        if (!ok) return;
        let failures = 0;
        for (const p of paths) {
            // moveToTrash (recoverable) — `deleteFile` is permanent and
            // reserved for inventory-cleanup paths.
            try { await window.vstUpdater.moveToTrash(p); }
            catch (_) { failures++; }
        }
        clearFileSelection();
        if (typeof showToast === 'function') {
            const survived = paths.length - failures;
            if (survived > 0) showToast(toastFmt('toast.deleted_name', {name: `moved ${survived} item${survived === 1 ? '' : 's'} to Trash`}));
            if (failures > 0) showToast(toastFmt('toast.failed', {err: `${failures} item${failures === 1 ? '' : 's'} failed`}), 4000, 'error');
        }
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
        return;
    }
    if (action === 'fileBulkScan') {
        const kind = btn.dataset.scan;
        const dirs = _fileBulkSelectionAsPaths((en) => en.isDir);
        if (dirs.length === 0) return; // silent no-op — user clicked scan with no folder selected

        const tabByKind = {samples: 'samples', presets: 'presets', daw: 'daw', midi: 'midi', pdf: 'pdf', videos: 'videos'};
        const tab = tabByKind[kind];
        if (tab && typeof switchTab === 'function') switchTab(tab);
        // scanMidi / scanVideos take (resume, overrideRoots); the others take
        // (resume, unifiedResult, overrideRoots).
        if (kind === 'samples' && typeof scanAudioSamples === 'function') scanAudioSamples(false, null, dirs);
        else if (kind === 'presets' && typeof scanPresets === 'function') scanPresets(false, null, dirs);
        else if (kind === 'daw' && typeof scanDawProjects === 'function') scanDawProjects(false, null, dirs);
        else if (kind === 'midi' && typeof scanMidi === 'function') scanMidi(false, dirs);
        else if (kind === 'pdf' && typeof scanPdfs === 'function') scanPdfs(false, null, dirs);
        else if (kind === 'videos' && typeof scanVideos === 'function') scanVideos(false, dirs);
        return;
    }
});

// Cmd/Ctrl+A while the Files tab is active and focus isn't in a text input →
// select all visible file rows.
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && (e.key === 'a' || e.key === 'A'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    selectAllVisibleFiles();
});

// Space → Quick-look overlay for the focused (or single-selected) file.
// Folders intentionally don't get a Quick-look — Space on a folder would be
// weird (and Enter/click already navigates into them).
document.addEventListener('keydown', (e) => {
    if (e.key !== ' ') return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    // If Quick-look is already up, Space dismisses it.
    if (isQuickLookVisible()) {
        e.preventDefault();
        hideQuickLook();
        return;
    }
    let row = (typeof getFileRows === 'function')
        ? getFileRows()[typeof _fileNavIdx !== 'undefined' ? _fileNavIdx : -1]
        : null;
    if (!row && _fileSelected && _fileSelected.size === 1) {
        const [p] = [..._fileSelected];
        if (typeof CSS !== 'undefined') {
            try { row = document.querySelector(`.file-row[data-file-path="${CSS.escape(p)}"]`); } catch (_) { /* ignore */ }
        }
    }
    if (!row) return;
    if (row.dataset.fileDir === 'true') return;
    const path = row.dataset.filePath;
    if (!path) return;
    e.preventDefault();
    showQuickLook(path);
});

// Esc → dismiss Quick-look when visible (also handled by overlay click).
document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (!isQuickLookVisible()) return;
    e.preventDefault();
    hideQuickLook();
});

// F2 → inline-rename the focused row (or single selection). Esc cancels.
// Active only while Files tab is active and focus isn't in a text input.
document.addEventListener('keydown', (e) => {
    if (e.key !== 'F2') return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    // Prefer the keyboard-nav cursor row; fall back to the single selection.
    let row = (typeof getFileRows === 'function')
        ? getFileRows()[typeof _fileNavIdx !== 'undefined' ? _fileNavIdx : -1]
        : null;
    if (!row && _fileSelected && _fileSelected.size === 1) {
        const [path] = [..._fileSelected];
        if (typeof CSS !== 'undefined') {
            try { row = document.querySelector(`.file-row[data-file-path="${CSS.escape(path)}"]`); } catch (_) { /* ignore */ }
        }
    }
    if (!row) return;
    e.preventDefault();
    startFileRename(row);
});

// Cmd+Shift+N → create new folder in current directory.
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'n' || e.key === 'N'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    fileBrowserNewFolder();
});

// Cmd+I → toggle preview pane visibility. Common file-browser convention
// (Get Info / inspector on macOS).
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && !e.shiftKey && (e.key === 'i' || e.key === 'I'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    toggleFileBrowserPreviewPane();
});

// ── Forward/back nav + New Folder button clicks ──
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="fileNavBack"]')) {
        e.stopPropagation();
        fileNavBack();
        return;
    }
    if (e.target.closest('[data-action="fileNavFwd"]')) {
        e.stopPropagation();
        fileNavForward();
        return;
    }
    if (e.target.closest('[data-action="fileNewFolder"]')) {
        e.stopPropagation();
        fileBrowserNewFolder();
        return;
    }
    if (e.target.closest('[data-action="fileTogglePreview"]')
        || e.target.closest('[data-action="fbPreviewClose"]')) {
        e.stopPropagation();
        if (e.target.closest('[data-action="fbPreviewClose"]')) setPreviewPaneVisible(false);
        else toggleFileBrowserPreviewPane();
        return;
    }
});

// Bulk rename modal: Cancel / Apply button delegation + live preview on input.
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="fbBulkRenameCancel"]')) {
        e.stopPropagation();
        hideFileBulkRenameModal();
        return;
    }
    if (e.target.closest('[data-action="fbBulkRenameApply"]')) {
        e.stopPropagation();
        applyFileBulkRename();
        return;
    }
    // Click on the dimmed overlay (outside the modal-content card) dismisses.
    const overlay = document.getElementById('fbBulkRenameModal');
    if (overlay && e.target === overlay) {
        hideFileBulkRenameModal();
    }
});

document.addEventListener('input', (e) => {
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    if (!modal.contains(e.target)) return;
    _fbBulkRenameRenderPreview();
});

document.addEventListener('change', (e) => {
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    if (!modal.contains(e.target)) return;
    _fbBulkRenameRenderPreview();
});

// Esc dismisses the bulk rename modal (consumed before any other Esc handler).
document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    e.preventDefault();
    e.stopPropagation();
    hideFileBulkRenameModal();
}, true);

// Restore preview-pane visibility from prefs on init (so the user's choice
// persists across app restarts). Wire row clicks to populate the pane.
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        try {
            const wanted = prefs.getItem('fileBrowserPreviewVisible');
            if (wanted === '1') setPreviewPaneVisible(true);
        } catch (_) { /* ignore */ }
    });
    // Update pane content whenever a file row is clicked.
    document.addEventListener('click', (e) => {
        const row = e.target instanceof Element ? e.target.closest('.file-row') : null;
        if (!row || row.dataset.fileDir === 'true') return;
        // Skip clicks that are routed to interactive children (checkbox, etc.)
        if (e.target.closest('.file-cb') || e.target.closest('.fb-rename-input')) return;
        const path = row.dataset.filePath;
        if (path) populatePreviewPane(path);
    });
}

// ── Extension chip clicks ──
document.addEventListener('click', (e) => {
    const chip = e.target.closest('.fb-ext-chips .ext-chip[data-ext-filter]');
    if (!chip) return;
    e.stopPropagation();
    setFileExtFilter(chip.dataset.extFilter);
});

// ── Cmd+L editable path input + Cmd+[ / Cmd+] history nav shortcuts ──
document.addEventListener('keydown', (e) => {
    if (!(e.metaKey || e.ctrlKey)) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    const inField = t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
    if (e.key === 'l' || e.key === 'L') {
        if (inField && t.id !== 'fileBrowserPathInput') return;
        e.preventDefault();
        showFilePathEditor();
        return;
    }
    if (e.key === '[') {
        if (inField) return;
        e.preventDefault();
        fileNavBack();
        return;
    }
    if (e.key === ']') {
        if (inField) return;
        e.preventDefault();
        fileNavForward();
        return;
    }
});

// Path input: Enter navigates, Esc cancels, blur hides.
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const input = document.getElementById('fileBrowserPathInput');
        if (!input) return;
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const path = input.value.trim();
                hideFilePathEditor();
                if (path) loadDirectory(path);
            } else if (e.key === 'Escape') {
                e.preventDefault();
                hideFilePathEditor();
            }
        });
        input.addEventListener('blur', () => {
            // Defer so Enter's loadDirectory has a chance to fire before the
            // breadcrumb takes its slot back.
            setTimeout(() => hideFilePathEditor(), 100);
        });
    });
}

// Filter — uses unified filter system
registerFilter('filterFiles', {
    inputId: 'fileSearchInput',
    regexToggleId: 'regexFiles',
    fetchFn() {
        _lastFilesMode = this.lastMode || 'fuzzy';
        renderFileList();
    },
    debounceMs: 150,
});

// Empty-space context menu — fires when the user right-clicks INSIDE the
// file list but NOT on a `.file-row`. Provides folder-level ops (New
// Folder, New File, Refresh, Open in Terminal, Reveal in Finder, Copy
// path) that the row-context-menu can't host because there's no
// path-of-interest. Registered BEFORE the row handler so it returns
// early when a row IS hit, leaving the row branch to fire.
document.addEventListener('contextmenu', (e) => {
    // Skip if click landed on a row — the row handler below will handle it.
    if (e.target.closest('.file-row')) return;
    // Only fire inside the file list container.
    if (!e.target.closest('#fileList')) return;
    if (!_fileBrowserPath) return;
    const dir = _fileBrowserPath;
    const dirName = dir.split('/').filter(Boolean).pop() || dir;
    const items = [
        {
            icon: '&#128193;',
            label: 'New Folder', ..._ctxMenuNoEcho,
            action: () => fileBrowserNewFolder(),
        },
        {
            icon: '&#128196;',
            label: 'New File', ..._ctxMenuNoEcho,
            action: () => fileBrowserNewFile(),
        },
    ];
    // Paste — only visible when there's something on the file clipboard
    // (Cmd+C / Cmd+X earlier). Cut clipboard empties itself on paste;
    // copy clipboard persists for further pastes (Finder behavior).
    const clip = window._fbClipboard;
    if (clip && clip.paths && clip.paths.length > 0) {
        const verb = clip.mode === 'cut' ? 'Move' : 'Paste';
        const count = clip.paths.length;
        items.push({
            icon: '&#128203;',
            label: `${verb} ${count} item${count === 1 ? '' : 's'} here`, ..._ctxMenuNoEcho,
            action: () => fileBrowserPasteClipboard(),
        });
    }
    // New Folder with Selection — only when there are selected rows.
    const selected = (typeof _fileSelected !== 'undefined' && _fileSelected instanceof Set)
        ? [..._fileSelected]
        : [];
    if (selected.length > 0) {
        items.push({
            icon: '&#128193;',
            label: `New Folder with ${selected.length} Item${selected.length === 1 ? '' : 's'}`,
            ..._ctxMenuNoEcho,
            action: () => fileBrowserNewFolderWithSelection(selected),
        });
    }
    // Invert Selection — only useful when something IS selected.
    if (selected.length > 0) {
        items.push({
            icon: '&#8646;',
            label: 'Invert Selection', ..._ctxMenuNoEcho,
            action: () => invertFileSelection(),
        });
        // Bulk Hash — fast SHA-256 for the whole selection (modal shows
        // per-row digests + Copy All).
        items.push({
            icon: '&#128273;',
            label: `Hash ${selected.length} Item${selected.length === 1 ? '' : 's'}`, ..._ctxMenuNoEcho,
            action: () => {
                // Folders aren't hashable; filter them out.
                const files = selected.filter((p) => {
                    const entry = _fileEntryByPath(p);
                    return entry && !entry.isDir;
                });
                if (files.length === 0) {
                    showToast(toastFmt('toast.failed', {err: 'No files in selection (folders skipped)'}), 4000, 'error');
                    return;
                }
                fileBrowserShowHashModal(files);
            },
        });
        // Bulk chmod — applies one octal mode to every selected path.
        items.push({
            icon: '&#128274;',
            label: `Permissions on ${selected.length} Item${selected.length === 1 ? '' : 's'}…`, ..._ctxMenuNoEcho,
            action: () => fileBrowserShowBulkChmodModal(selected),
        });
        // Bulk touch — set mtime to now on every selected path.
        items.push({
            icon: '&#9201;',
            label: `Touch ${selected.length} Item${selected.length === 1 ? '' : 's'}`, ..._ctxMenuNoEcho,
            action: () => fileBrowserTouchPaths(selected),
        });
        // Compress selection into one .zip.
        items.push({
            icon: '&#128230;',
            label: `Compress ${selected.length} Item${selected.length === 1 ? '' : 's'} into Archive…`, ..._ctxMenuNoEcho,
            action: () => fileBrowserBulkCompress(selected),
        });
        // Extract every selected archive (.zip / .tar / .tar.gz).
        const archives = selected.filter((p) => /\.(zip|tar|tar\.gz|tgz|7z)$/i.test(p));
        if (archives.length > 0) {
            items.push({
                icon: '&#128194;',
                label: `Extract ${archives.length} Archive${archives.length === 1 ? '' : 's'} Here`, ..._ctxMenuNoEcho,
                action: () => fileBrowserBulkExtract(archives),
            });
        }
    }
    items.push({
        icon: '&#128269;',
        label: 'Select by Pattern…', ..._ctxMenuNoEcho,
        action: () => fileBrowserPatternSelect(),
    });
    items.push({
        icon: '&#128270;',
        label: 'Find in Files… (grep)', ..._ctxMenuNoEcho,
        action: () => fileBrowserShowGrepModal(),
    });
    items.push({
        icon: '&#128269;',
        label: 'Find Duplicates… (by content)', ..._ctxMenuNoEcho,
        action: () => fileBrowserShowDuplicatesModal(),
    });
    items.push({
        icon: '&#9889;',
        label: 'Quick Open… (Cmd+P)', ..._ctxMenuNoEcho,
        action: () => fileBrowserShowQuickPalette(),
    });
    items.push({
        icon: '&#128269;',
        label: 'Spotlight — search all inventory (Cmd+K)', ..._ctxMenuNoEcho,
        action: () => fileBrowserShowSpotlight(),
    });
    items.push({
        icon: '&#9733;',
        label: 'Manage Bookmarks…', ..._ctxMenuNoEcho,
        action: () => fileBrowserShowBookmarksModal(),
    });
    if (_fbPaneCount >= 2) {
        items.push({
            icon: '&#8651;',
            label: 'Compare with Other Pane (folder tree diff)', ..._ctxMenuNoEcho,
            action: () => fileBrowserShowCompareModal(),
        });
    }
    if (selected.length === 2) {
        items.push({
            icon: '&#8651;',
            label: `Diff ${selected.map((p) => p.split('/').pop()).join(' ⇄ ')}`, ..._ctxMenuNoEcho,
            action: () => fileBrowserShowDiffModal(),
        });
    }
    items.push(...[
        '---',
        {
            icon: _fbShowHidden ? '&#128064;' : '&#128065;',
            label: _fbShowHidden ? 'Hide Hidden Files (Ctrl+H)' : 'Show Hidden Files (Ctrl+H)', ..._ctxMenuNoEcho,
            action: () => fileBrowserToggleHidden(),
        },
        {
            icon: '&#128218;',
            label: (document.getElementById('fbTreeSidebar') && !document.getElementById('fbTreeSidebar').classList.contains('fb-hidden'))
                ? 'Hide Tree Sidebar (Cmd+B)'
                : 'Show Tree Sidebar (Cmd+B)', ..._ctxMenuNoEcho,
            action: () => fileBrowserToggleTreeSidebar(),
        },
        {
            icon: '&#9783;',
            label: `Panes: ${_fbPaneCount}/4 (Cmd+\\\\ cycle, F5 copy, F6 move)`, ..._ctxMenuNoEcho,
            action: () => _fbCyclePaneCount(),
        },
        {
            icon: '&#128260;',
            label: 'Refresh', ..._ctxMenuNoEcho,
            action: () => loadDirectory(dir),
        },
        {
            icon: '&#9000;',
            label: 'Open in Terminal', ..._ctxMenuNoEcho,
            action: () => {
                if (window.vstUpdater && typeof window.vstUpdater.fsOpenTerminal === 'function') {
                    showToast(toastFmt('toast.opening_in_app', {app: 'Terminal'}));
                    window.vstUpdater.fsOpenTerminal(dir).catch((err) =>
                        showToast(toastFmt('toast.failed', {err: err && err.message ? err.message : err}), 4000, 'error')
                    );
                }
            },
        },
        {
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho,
            action: () => {
                showToast(toastFmt('toast.revealing_file'));
                window.vstUpdater.openPresetFolder(dir)
                    .then(() => showToast(toastFmt('toast.revealed_in_finder')))
                    .catch((err) => showToast(toastFmt('toast.failed', {err: err && err.message ? err.message : err}), 4000, 'error'));
            },
        },
        '---',
        {
            icon: '&#128203;',
            label: `${appFmt('menu.copy_path')}: ${dirName}`, ..._ctxMenuNoEcho,
            action: () => copyToClipboard(dir),
        },
    ]);
    showContextMenu(e, items);
    e.preventDefault();
    // file-browser.js loads BEFORE context-menu.js (per index.html script
    // order), so our listener fires first. Without stopImmediatePropagation,
    // context-menu.js's universal-fallback branch then runs and OVERWRITES
    // our menu with the "Copy visible text / Command Palette / Help /
    // Settings" generic menu. Stopping immediate propagation prevents
    // context-menu.js's listener from running for this event at all.
    e.stopImmediatePropagation();
});

// Right-click context menu
document.addEventListener('contextmenu', (e) => {
    const row = e.target.closest('.file-row');
    if (!row) return;
    const path = row.dataset.filePath;
    const isDir = row.dataset.fileDir === 'true';
    const name = row.querySelector('.file-name')?.textContent || '';
    const ext = path.split('.').pop().toLowerCase();

    const items = [];
    if (isDir) {
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.open_folder'), ..._ctxMenuNoEcho,
            action: () => {
                showToast(toastFmt('toast.opening_name', {name}));
                loadDirectory(path);
            }
        });
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('revealFile'),
            action: () => {
                showToast(toastFmt('toast.revealing_file'));
                window.vstUpdater.openPresetFolder(path)
                    .then(() => showToast(toastFmt('toast.revealed_in_finder')))
                    .catch((err) => showToast(toastFmt('toast.failed', {err: err && err.message ? err.message : err}), 4000, 'error'));
            }
        });
        const dirFav = isFavDir(path);
        items.push({
            icon: dirFav ? '&#9734;' : '&#9733;',
            label: dirFav ? appFmt('menu.remove_bookmark') : appFmt('menu.bookmark_directory'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('toggleFavorite'),
            action: () => dirFav ? removeFavDir(path) : addFavDir(path)
        });
    } else {
        if (AUDIO_EXTS.includes(ext)) {
            items.push({
                icon: '&#9654;',
                label: appFmt('menu.play'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('playPause'),
                action: () => previewAudio(path)
            });
        }
        items.push({
            icon: '&#128194;',
            label: appFmt('menu.open'), ..._ctxMenuNoEcho,
            action: () => {
                showToast(toastFmt('toast.opening_name', {name}));
                opener_open(path);
            }
        });
        // Explicit "Open in Default App" — bypasses the DAW-first routing in
        // `opener_open`. Useful when the user wants the OS default for a file
        // type that's also a DAW project (e.g. open .als raw in a text editor)
        // or for arbitrary files (.txt, .png, .docx) where there's no in-app
        // viewer. Optimistic toast on click; failure routes through the
        // explicit `toast.failed_open_file` error toast.
        items.push({
            icon: '&#128194;',
            label: appFmt('menu.open_default_app'), ..._ctxMenuNoEcho,
            action: () => {
                showToast(toastFmt('toast.opening_name', {name}));
                window.vstUpdater.openFileDefault(path).catch((err) =>
                    showToast(toastFmt('toast.failed_open_file', {err: err && err.message ? err.message : err}), 4000, 'error')
                );
            },
        });
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('revealFile'),
            action: () => {
                showToast(toastFmt('toast.revealing_file'));
                window.vstUpdater.openPresetFolder(path)
                    .then(() => showToast(toastFmt('toast.revealed_in_finder')))
                    .catch((err) => showToast(toastFmt('toast.failed', {err: err && err.message ? err.message : err}), 4000, 'error'));
            }
        });
    }
    items.push('---');
    items.push({
        icon: '&#128203;',
        label: appFmt('menu.copy_path'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('copyPath'),
        action: () => copyToClipboard(path)
    });
    items.push({
        icon: '&#128203;',
        label: appFmt('menu.copy_name'), ..._ctxMenuNoEcho,
        action: () => copyToClipboard(name.replace(/^[\u{1F4DD}]/u, '').trim())
    });
    items.push('---');

    // ALS XML viewer
    if (ext === 'als' && typeof showAlsViewer === 'function') {
        items.push({
            icon: '&#128196;',
            label: appFmt('menu.explore_xml_contents'),
            action: () => showAlsViewer(path, name)
        });
        items.push('---');
    }

    // Tags & notes
    const note = getNote(path);
    items.push({
        icon: '&#128221;',
        label: note ? appFmt('menu.edit_note') : appFmt('menu.add_note'),
        ..._ctxShortcutTip('addNote'),
        action: () => showNoteEditor(path, name)
    });

    const allTags = getAllTags();
    const currentTags = note?.tags || [];
    if (allTags.length > 0) {
        items.push('---');
        for (const tag of allTags.slice(0, 6)) {
            const has = currentTags.includes(tag);
            items.push({
                icon: has ? '&#10003;' : '&#9634;',
                label: has ? appFmt('menu.remove_tag_named', {tag}) : appFmt('menu.add_tag_named', {tag}), ..._ctxMenuNoEcho,
                action: () => {
                    if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag);
                    showToast(has ? toastFmt('toast.tag_removed', {tag}) : toastFmt('toast.tag_added', {tag}));
                    renderFileList();
                }
            });
        }
    }

    items.push('---');
    const fav = isFavorite(path);
    items.push({
        icon: fav ? '&#9734;' : '&#9733;',
        label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('toggleFavorite'),
        action: () => fav ? removeFavorite(path) : addFavorite(isDir ? 'folder' : (AUDIO_EXTS.includes(ext) ? 'sample' : 'file'), path, name, {format: ext.toUpperCase()})
    });

    items.push('---');
    items.push({
        icon: '&#128465;', label: appFmt('menu.delete'), ..._ctxShortcutTip('deleteItem'), action: async () => {
            const msg = appFmt('confirm.delete_file_browser', {name});
            // In-app modal (`confirmAction`) — native `confirm()` is
            // unreliable in Tauri WKWebView release builds.
            const ok = typeof confirmAction === 'function' ? await confirmAction(msg) : confirm(msg);
            if (!ok) return;
            try {
                // moveToTrash (recoverable) — never permanent unlink from
                // a user-facing menu.
                await window.vstUpdater.moveToTrash(path);
                showToast(toastFmt('toast.deleted_name_quotes', {name}));
                loadDirectory(_fileBrowserPath);
            } catch (err) {
                showToast(toastFmt('toast.delete_failed_msg', {err: err.message || err}), 4000, 'error');
            }
        }
    });

    showContextMenu(e, items);
    e.preventDefault();
});

// ── Ableton-style keyboard navigation ──
// `var` so module-load pane-init code (above) can sync this from prefs
// without hitting the `let` TDZ.
var _fileNavIdx = -1;

function getFileRows() {
    return [...document.querySelectorAll('#fileList .file-row')];
}

function fileNavSelect(idx) {
    const rows = getFileRows();
    if (rows.length === 0) return;
    // Clear previous
    rows.forEach(r => r.classList.remove('file-selected'));
    _fileNavIdx = Math.max(0, Math.min(idx, rows.length - 1));
    const row = rows[_fileNavIdx];
    row.classList.add('file-selected');
    row.scrollIntoView({block: 'nearest', behavior: 'smooth'});
}

document.addEventListener('click', (e) => {
    const row = e.target.closest('#fileList .file-row');
    if (row) {
        const rows = getFileRows();
        const idx = rows.indexOf(row);
        if (idx >= 0) {
            rows.forEach(r => r.classList.remove('file-selected'));
            _fileNavIdx = idx;
            row.classList.add('file-selected');
        }
    }
});

document.addEventListener('keydown', (e) => {
    // Only handle when Files tab is active and not typing in an input
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;

    // ── Nautilus-style shortcuts (must run BEFORE the rows.length guard
    //     below — they're folder-level and shouldn't require a selection) ──
    // Ctrl+H — toggle hidden files (`.dotfiles`).
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'h' || e.key === 'H')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        fileBrowserToggleHidden();
        return;
    }
    // Cmd+Shift+I — invert selection (Nautilus Ctrl+I, but Cmd+I is
    // Get Info here so we use Shift+I instead).
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && (e.key === 'i' || e.key === 'I')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (typeof invertFileSelection === 'function') invertFileSelection();
        return;
    }
    // Cmd+B — toggle tree-view sidebar (mirrors macOS Finder Cmd+Opt+S
    // for sidebar, but Opt+S is taken; B = "Browser tree").
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'b' || e.key === 'B')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (typeof fileBrowserToggleTreeSidebar === 'function') fileBrowserToggleTreeSidebar();
        return;
    }
    // Cmd+P — VSCode-style quick file palette (recent folders + files,
    // fuzzy filter, Enter to open).
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (typeof fileBrowserShowQuickPalette === 'function') fileBrowserShowQuickPalette();
        return;
    }
    // ── Multi-pane ──
    // Cmd+\\ — cycle pane count (1 → 2 → 3 → 4 → 1). Mirrors how
    // tmux uses Ctrl+B \ to split, but we cycle the count instead of
    // toggling so 3- and 4-pane layouts are reachable.
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key === '\\') {
        e.preventDefault();
        e.stopImmediatePropagation();
        _fbCyclePaneCount();
        return;
    }
    // F5 → copy active-pane selection into next pane's folder.
    // F6 → move active-pane selection into next pane's folder.
    // Norton Commander / Total Commander conventions — no Cmd modifier
    // (raw function keys), so they don't conflict with browser refresh
    // (which fires on Cmd+R / Ctrl+R, not F5, in the Tauri WebView).
    if (e.key === 'F5' && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
        e.preventDefault();
        e.stopImmediatePropagation();
        _fbCrossPaneOp('copy');
        return;
    }
    if (e.key === 'F6' && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
        e.preventDefault();
        e.stopImmediatePropagation();
        _fbCrossPaneOp('move');
        return;
    }
    // Cmd+Alt+1..4 — jump active to pane N (free of conflict with
    // Cmd+1..9 = tab switching).
    if ((e.ctrlKey || e.metaKey) && e.altKey && /^[1-4]$/.test(e.key)) {
        const idx = parseInt(e.key, 10) - 1;
        if (idx >= 0 && idx < _fbPaneCount) {
            e.preventDefault();
            e.stopImmediatePropagation();
            _fbSetActivePane(idx);
        }
        return;
    }
    // ── Tabs ──
    // Cmd+T — new tab (clones current path)
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 't' || e.key === 'T')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        fbNewTab();
        return;
    }
    // Cmd+Shift+W — close active tab. Cmd+W (without Shift) is bound by
    // the native menu to "close window" via PredefinedMenuItem and never
    // reaches the WebView, so we use Shift to disambiguate. The close
    // button on each tab is always available via mouse.
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && (e.key === 'w' || e.key === 'W')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (_fbActiveTabId) fbCloseTab(_fbActiveTabId);
        return;
    }
    // Cmd+Shift+] / Cmd+Shift+[ — next / previous tab (matches macOS
    // convention for tabbed apps like Safari).
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && (e.key === ']' || e.key === '}')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        fbCycleTab(1);
        return;
    }
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && (e.key === '[' || e.key === '{')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        fbCycleTab(-1);
        return;
    }
    // Cmd+1..9 — jump to Nth tab (Cmd+9 → last tab per Chrome/Safari
    // convention).
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key)) {
        const n = parseInt(e.key, 10);
        const idx = n === 9 ? _fbTabs.length - 1 : Math.min(n - 1, _fbTabs.length - 1);
        if (idx >= 0 && _fbTabs[idx]) {
            e.preventDefault();
            e.stopImmediatePropagation();
            fbSwitchTab(_fbTabs[idx].id);
        }
        return;
    }
    // Shift+Delete / Shift+Backspace — permanent delete (skip Trash).
    // Nautilus convention: holding Shift bypasses the Trash.
    if (e.shiftKey && (e.key === 'Delete' || e.key === 'Backspace')) {
        const sel = (typeof _fileSelected !== 'undefined' && _fileSelected instanceof Set)
            ? [..._fileSelected]
            : [];
        if (sel.length === 0) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        (async () => {
            const target = sel.length === 1 ? `"${sel[0].split('/').pop()}"` : `${sel.length} items`;
            const msg = `Permanently delete ${target}?\n\nThis cannot be undone — files will NOT be moved to Trash.`;
            const ok = typeof confirmAction === 'function'
                ? await confirmAction(msg, 'Delete Permanently')
                : confirm(msg);
            if (!ok) return;
            let ok_c = 0, fail = 0;
            for (const p of sel) {
                try { await window.vstUpdater.deleteFile(p); ok_c++; }
                catch (_) { fail++; }
            }
            if (typeof clearFileSelection === 'function') clearFileSelection();
            if (typeof showToast === 'function') {
                if (ok_c > 0) showToast(toastFmt('toast.deleted_name', {name: `permanently deleted ${ok_c} item${ok_c === 1 ? '' : 's'}`}));
                if (fail > 0) showToast(toastFmt('toast.failed', {err: `${fail} delete${fail === 1 ? '' : 's'} failed`}), 4000, 'error');
            }
            if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
        })();
        return;
    }

    const rows = getFileRows();
    if (rows.length === 0) return;

    if (e.key === 'ArrowDown' || (e.key === 'j' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        fileNavSelect(_fileNavIdx + 1);
    } else if (e.key === 'ArrowUp' || (e.key === 'k' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        fileNavSelect(_fileNavIdx - 1);
    } else if (e.key === 'Home') {
        e.preventDefault();
        fileNavSelect(0);
    } else if (e.key === 'End') {
        e.preventDefault();
        fileNavSelect(rows.length - 1);
    } else if (e.key === 'ArrowRight' || e.key === 'l') {
        // Right arrow: navigate into directory or play audio
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            loadDirectory(path).then(() => {
                _fileNavIdx = -1;
                fileNavSelect(0);
            });
        } else {
            const ext = path.split('.').pop().toLowerCase();
            if (AUDIO_EXTS.includes(ext)) {
                previewAudio(path);
            } else {
                opener_open(path);
            }
        }
    } else if (e.key === 'Enter') {
        // Enter: open in Finder (dir) or open with default app (file)
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            openFolder(path);
        } else {
            opener_open(path);
        }
    } else if (e.key === 'ArrowLeft' || e.key === 'h') {
        // Left arrow: go to parent directory
        e.preventDefault();
        if (_fileBrowserPath) {
            const n = normalizePathSeparators(_fileBrowserPath);
            if (n !== '/' && !/^[A-Za-z]:\/?$/.test(n.replace(/\/+$/, ''))) {
                const parent = parentDirectoryPath(_fileBrowserPath);
                if (normalizePathSeparators(parent) !== n.replace(/\/+$/, '')) {
                    loadDirectory(parent).then(() => {
                        _fileNavIdx = -1;
                        fileNavSelect(0);
                    });
                }
            }
        }
    } else if (e.key === ' ') {
        // Global shortcut handles Space for play/pause; do not restart preview on top of it.
        if (e.defaultPrevented) return;
        // Space: preview audio if selected
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const ext = path.split('.').pop().toLowerCase();
        if (AUDIO_EXTS.includes(ext)) {
            previewAudio(path);
        }
    }
});
