// ── Batch Selection ──
// Checkboxes in table rows for multi-item operations.
// Selections are scoped per inventory tab (tabSamples, tabDaw, …) — not one global Set.

/** Tab panel ids that use `.batch-cb` in a table body. */
const TABS_WITH_BATCH = new Set(['tabSamples', 'tabDaw', 'tabPresets', 'tabMidi', 'tabPdf']);

/** @type {Map<string, Set<string>>} */
const batchByTab = new Map();

function tabIdForBatchContext() {
    const tab = document.querySelector('.tab-content.active');
    return tab && TABS_WITH_BATCH.has(tab.id) ? tab.id : null;
}

/**
 * Paths selected on the given inventory tab (for row HTML `checked` state).
 * @param {string} tabId — e.g. `tabSamples`
 */
function batchSetForTabId(tabId) {
    if (!TABS_WITH_BATCH.has(tabId)) return new Set();
    if (!batchByTab.has(tabId)) batchByTab.set(tabId, new Set());
    return batchByTab.get(tabId);
}

/** Mutable Set for the active inventory tab, or null if the active tab has no batch UI. */
function getActiveBatchSet() {
    const id = tabIdForBatchContext();
    if (!id) return null;
    return batchSetForTabId(id);
}

function activeBatchCount() {
    const s = getActiveBatchSet();
    return s ? s.size : 0;
}

function getPathFromBatchRow(el) {
    if (!el) return null;
    return (
        el.dataset.audioPath ||
        el.dataset.dawPath ||
        el.dataset.presetPath ||
        el.dataset.midiPath ||
        el.dataset.pdfPath ||
        el.dataset.path ||
        null
    );
}

/** @param {string} trId */
function _isLoadMoreRow(tr, trId) {
    return !!(trId && (trId === 'audioLoadMore' || trId === 'dawLoadMore' || trId.endsWith('LoadMore')));
}

/**
 * Resolve batch paths to row objects: try filtered + full in-memory arrays (paginated SQLite tabs),
 * then visible table rows for paths from earlier "load more" pages not in the current filtered slice.
 * @param {Set<string>} set
 * @param {string} tabId — `tabSamples`, `tabDaw`, …
 * @returns {unknown[]}
 */
function resolveBatchInventoryItems(set, tabId) {
    const pools = [];
    let tbodyId = null;
    /** @type {((tr: HTMLElement, path: string) => Record<string, unknown>) | null} */
    let shallow = null;

    if (tabId === 'tabSamples') {
        if (typeof filteredAudioSamples !== 'undefined') pools.push(filteredAudioSamples);
        if (typeof allAudioSamples !== 'undefined') pools.push(allAudioSamples);
        tbodyId = 'audioTableBody';
        shallow = (tr, path) => {
            const name =
                tr.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                tr.querySelector('.col-name')?.textContent?.trim() ||
                path.split('/').pop();
            const format = tr.querySelector('.format-badge')?.textContent?.trim() || '';
            return { path, name, format };
        };
    } else if (tabId === 'tabDaw') {
        if (typeof filteredDawProjects !== 'undefined') pools.push(filteredDawProjects);
        if (typeof allDawProjects !== 'undefined') pools.push(allDawProjects);
        tbodyId = 'dawTableBody';
        shallow = (tr, path) => {
            const name =
                tr.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                tr.querySelector('.col-name')?.textContent?.trim() ||
                path.split('/').pop();
            const badges = tr.querySelectorAll('.col-format .format-badge');
            const daw = (badges[0]?.textContent || tr.dataset.dawName || '').trim();
            const format = (badges[1]?.textContent || '').trim();
            return { path, name, format, daw };
        };
    } else if (tabId === 'tabPresets') {
        if (typeof filteredPresets !== 'undefined') pools.push(filteredPresets);
        if (typeof allPresets !== 'undefined') pools.push(allPresets);
        tbodyId = 'presetTableBody';
        shallow = (tr, path) => {
            const name =
                tr.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                tr.querySelector('.col-name')?.textContent?.trim() ||
                path.split('/').pop();
            const format = (tr.dataset.presetFormat || tr.querySelector('.format-badge')?.textContent || '').trim();
            return { path, name, format };
        };
    } else if (tabId === 'tabMidi') {
        if (typeof filteredMidi !== 'undefined') pools.push(filteredMidi);
        if (typeof allMidiFiles !== 'undefined') pools.push(allMidiFiles);
        tbodyId = 'midiTableBody';
        shallow = (tr, path) => {
            const name =
                tr.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                tr.querySelector('.col-name')?.textContent?.trim() ||
                path.split('/').pop();
            return { path, name, format: 'MIDI', sizeFormatted: tr.querySelector('.col-size')?.textContent?.trim() || '' };
        };
    } else if (tabId === 'tabPdf') {
        if (typeof filteredPdfs !== 'undefined') pools.push(filteredPdfs);
        if (typeof allPdfs !== 'undefined') pools.push(allPdfs);
        tbodyId = 'pdfTableBody';
        shallow = (tr, path) => {
            const name =
                tr.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                tr.querySelector('.col-name')?.textContent?.trim() ||
                path.split('/').pop();
            return { path, name, format: 'PDF', sizeFormatted: tr.querySelector('.col-size')?.textContent?.trim() || '' };
        };
    } else {
        return [];
    }

    const out = [];
    const findFn = typeof findByPath === 'function' ? findByPath : null;

    for (const path of set) {
        if (path == null || path === '') continue;
        let item = null;
        for (const arr of pools) {
            if (!arr || !arr.length) continue;
            item = findFn ? findFn(arr, path) : arr.find((i) => i && i.path === path);
            if (item) break;
        }
        if (!item && tbodyId && typeof shallow === 'function') {
            const tb = document.getElementById(tbodyId);
            if (tb) {
                for (const tr of tb.querySelectorAll('tr')) {
                    const tid = tr.id || '';
                    if (_isLoadMoreRow(tr, tid)) continue;
                    if (getPathFromBatchRow(tr) === path) {
                        item = shallow(tr, path);
                        break;
                    }
                }
            }
        }
        if (item) out.push(item);
    }
    return out;
}

function rowElementFromBatchCheckbox(cb) {
    return cb.closest('tr') || cb.closest('.plugin-card') || cb.closest('.fav-item');
}

function toggleBatchSelect(path, checked) {
    if (path == null || path === '') return;
    const set = getActiveBatchSet();
    if (!set) return;
    if (checked) {
        set.add(path);
    } else {
        set.delete(path);
    }
    updateBatchBar();
}

function selectAllVisible() {
    const id = tabIdForBatchContext();
    if (!id) return;
    const tbody = document.querySelector('.tab-content.active tbody');
    if (!tbody) return;
    const set = batchSetForTabId(id);
    tbody.querySelectorAll('.batch-cb').forEach(cb => {
        cb.checked = true;
        const path = getPathFromBatchRow(rowElementFromBatchCheckbox(cb));
        if (path) set.add(path);
    });
    updateBatchBar();
}

function deselectAll() {
    batchByTab.clear();
    document.querySelectorAll('.batch-cb').forEach(cb => {
        cb.checked = false;
    });
    document.querySelectorAll('.batch-cb-all').forEach(cb => {
        cb.checked = false;
    });
    updateBatchBar();
}

function updateBatchBar() {
    const bar = document.getElementById('batchActionBar');
    if (!bar) return;
    const n = activeBatchCount();
    if (n === 0) {
        bar.style.display = 'none';
        document.querySelectorAll('.batch-cb-all').forEach(cb => {
            cb.checked = false;
        });
        return;
    }
    bar.style.display = 'flex';
    const bc = document.getElementById('batchSelectionCount');
    if (bc) {
        bc.textContent = catalogFmt('menu.batch_selected', {n});
    }

    const tbody = document.querySelector('.tab-content.active tbody');
    if (tbody) {
        const allCbs = tbody.querySelectorAll('.batch-cb');
        const allChecked = allCbs.length > 0 && [...allCbs].every(cb => cb.checked);
        const headerCb = tbody.closest('table')?.querySelector('.batch-cb-all');
        if (headerCb) headerCb.checked = allChecked;
    }
}

function batchFavoriteAll() {
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return;
    const set = getActiveBatchSet();
    if (!set || set.size === 0) return;

    const tid = activeTab.id;
    if (tid === 'tabPlugins') {
        const plugins = typeof allPlugins !== 'undefined' ? allPlugins : [];
        let added = 0;
        for (const path of set) {
            if (isFavorite(path)) continue;
            const item = typeof findByPath === 'function' ? findByPath(plugins, path) : plugins.find((i) => i.path === path);
            if (item) {
                addFavorite('plugin', path, item.name, {format: item.type || item.format || ''});
                added++;
            }
        }
        showToast(toastFmt('toast.added_favorites_batch', {n: added}));
        deselectAll();
        return;
    }

    const typeMap = {
        tabSamples: 'sample',
        tabDaw: 'daw',
        tabPresets: 'preset',
        tabMidi: 'midi',
        tabPdf: 'pdf',
    };
    const type = typeMap[tid];
    if (!type) return;

    const items = resolveBatchInventoryItems(set, tid);
    let added = 0;
    for (const item of items) {
        if (!item || !item.path) continue;
        if (isFavorite(item.path)) continue;
        addFavorite(type, item.path, item.name, {format: item.format, daw: item.daw});
        added++;
    }
    if (set.size > 0 && items.length === 0) {
        showToast(toastFmt('toast.no_matching_samples'), 3500, 'warning');
        return;
    }
    showToast(toastFmt('toast.added_favorites_batch', {n: added}));
    deselectAll();
}

function batchCopyPaths() {
    const set = getActiveBatchSet();
    if (!set || set.size === 0) return;
    const paths = [...set].join('\n');
    if (typeof copyToClipboard !== 'function') return;
    copyToClipboard(paths);
    showToast(toastFmt('toast.copied_n_paths', {n: set.size}));
}

function batchExportSelected() {
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return;
    const set = getActiveBatchSet();
    if (!set || set.size === 0) return;

    const tid = activeTab.id;
    let items = [];
    if (tid === 'tabPlugins') {
        const plugins = typeof allPlugins !== 'undefined' ? allPlugins : [];
        for (const path of set) {
            const item = typeof findByPath === 'function' ? findByPath(plugins, path) : plugins.find((i) => i.path === path);
            if (item) items.push(item);
        }
    } else if (TABS_WITH_BATCH.has(tid)) {
        items = resolveBatchInventoryItems(set, tid);
    }

    if (items.length === 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.no_list_export'), 3500, 'warning');
        }
        return;
    }
    if (typeof copyToClipboard !== 'function') return;
    copyToClipboard(JSON.stringify(items, null, 2));
    showToast(toastFmt('toast.copied_n_json', {n: items.length}));
}

function batchRevealAll() {
    const activeTab = document.querySelector('.tab-content.active');
    const set = getActiveBatchSet();
    if (!activeTab || !set || set.size === 0) return;
    const path = [...set][0];
    if (activeTab.id === 'tabSamples') {
        if (typeof openAudioFolder === 'function') openAudioFolder(path);
    } else if (activeTab.id === 'tabDaw') {
        if (typeof openDawFolder === 'function') openDawFolder(path);
    } else if (activeTab.id === 'tabPresets') {
        if (typeof openPresetFolder === 'function') openPresetFolder(path);
    } else if (activeTab.id === 'tabPlugins') {
        if (typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.openPluginFolder === 'function') {
            window.vstUpdater.openPluginFolder(path).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
        }
    } else if (activeTab.id === 'tabMidi' || activeTab.id === 'tabPdf') {
        if (typeof openAudioFolder === 'function') openAudioFolder(path);
    }
    showToast(toastFmt('toast.revealing_first_batch', {n: set.size}));
}

// Wire up checkbox changes and batch action buttons
document.addEventListener('change', (e) => {
    // Header "select all" uses both `batch-cb-all` and `batch-cb` — ignore here (click handler sets rows).
    if (e.target.classList.contains('batch-cb-all')) return;
    if (e.target.classList.contains('batch-cb')) {
        const path = getPathFromBatchRow(rowElementFromBatchCheckbox(e.target));
        if (path) toggleBatchSelect(path, e.target.checked);
    }
});

document.addEventListener('click', (e) => {
    // Header "select all" checkbox — must check before batch-cb
    if (e.target.classList.contains('batch-cb-all')) {
        e.stopPropagation();
        if (e.target.checked) selectAllVisible();
        else deselectAll();
        return;
    }

    // Prevent row click-through on checkbox cell
    if (e.target.classList.contains('batch-cb')) {
        e.stopPropagation();
        return;
    }

    const action = e.target.closest('[data-batch-action]');
    if (action) {
        const act = action.dataset.batchAction;
        if (act === 'selectAll') selectAllVisible();
        else if (act === 'deselectAll') deselectAll();
        else if (act === 'favorite') batchFavoriteAll();
        else if (act === 'copyPaths') batchCopyPaths();
        else if (act === 'exportJson') batchExportSelected();
        else if (act === 'reveal') batchRevealAll();
    }
});

if (typeof window !== 'undefined') {
    window.batchSetForTabId = batchSetForTabId;
    window.getActiveBatchSet = getActiveBatchSet;
    window.activeBatchCount = activeBatchCount;
    window.getRowPath = getPathFromBatchRow;
    window.toggleBatchSelect = toggleBatchSelect;
    window.deselectAll = deselectAll;
    window.selectAllVisible = selectAllVisible;
}
