// ── Batch Selection ──
// Checkboxes in table rows for multi-item operations

const batchSelected = new Set();

function getRowPath(tr) {
    if (!tr) return null;
    return tr.dataset.audioPath || tr.dataset.dawPath || tr.dataset.presetPath || tr.dataset.midiPath || tr.dataset.pdfPath || null;
}

function toggleBatchSelect(path, checked) {
    if (checked) {
        batchSelected.add(path);
    } else {
        batchSelected.delete(path);
    }
    updateBatchBar();
}

function selectAllVisible() {
    const tbody = document.querySelector('.tab-content.active tbody');
    if (!tbody) return;
    tbody.querySelectorAll('.batch-cb').forEach(cb => {
        cb.checked = true;
        const path = getRowPath(cb.closest('tr'));
        if (path) batchSelected.add(path);
    });
    updateBatchBar();
}

function deselectAll() {
    batchSelected.clear();
    document.querySelectorAll('.batch-cb').forEach(cb => {
        cb.checked = false;
    });
    updateBatchBar();
}

function updateBatchBar() {
    const bar = document.getElementById('batchActionBar');
    if (!bar) return;
    if (batchSelected.size === 0) {
        bar.style.display = 'none';
        // Uncheck header "select all" checkboxes
        document.querySelectorAll('.batch-cb-all').forEach(cb => {
            cb.checked = false;
        });
        return;
    }
    bar.style.display = 'flex';
    const bc = document.getElementById('batchSelectionCount');
    if (bc) {
        bc.textContent = catalogFmt('menu.batch_selected', {n: batchSelected.size});
    }

    // Update header checkbox state (checked if all visible are selected)
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
    let type = 'sample', items = typeof allAudioSamples !== 'undefined' ? allAudioSamples : [];
    if (activeTab.id === 'tabDaw') {
        type = 'daw';
        items = typeof allDawProjects !== 'undefined' ? allDawProjects : [];
    } else if (activeTab.id === 'tabPresets') {
        type = 'preset';
        items = typeof allPresets !== 'undefined' ? allPresets : [];
    } else if (activeTab.id === 'tabMidi') {
        type = 'midi';
        items = typeof allMidiFiles !== 'undefined' ? allMidiFiles : [];
    }

    let added = 0;
    for (const path of batchSelected) {
        if (isFavorite(path)) continue;
        const item = items.find(i => i.path === path);
        if (item) {
            addFavorite(type, path, item.name, {format: item.format, daw: item.daw});
            added++;
        }
    }
    showToast(toastFmt('toast.added_favorites_batch', {n: added}));
    deselectAll();
}

function batchCopyPaths() {
    const paths = [...batchSelected].join('\n');
    copyToClipboard(paths);
    showToast(toastFmt('toast.copied_n_paths', {n: batchSelected.size}));
}

function batchExportSelected() {
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return;

    // O(selected) via path index instead of O(total) linear filter — matters when
    // total is millions and selection is small.
    const pickByPaths = (arr) => {
        const out = [];
        for (const path of batchSelected) {
            const item = findByPath(arr, path);
            if (item) out.push(item);
        }
        return out;
    };
    let items = [];
    if (activeTab.id === 'tabSamples') {
        items = pickByPaths(allAudioSamples);
    } else if (activeTab.id === 'tabDaw') {
        items = pickByPaths(allDawProjects);
    } else if (activeTab.id === 'tabPresets') {
        items = pickByPaths(allPresets);
    }

    if (items.length === 0) return;
    copyToClipboard(JSON.stringify(items, null, 2));
    showToast(toastFmt('toast.copied_n_json', {n: items.length}));
}

function batchRevealAll() {
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || batchSelected.size === 0) return;
    // Reveal first selected item
    const path = [...batchSelected][0];
    if (activeTab.id === 'tabSamples') openAudioFolder(path);
    else if (activeTab.id === 'tabDaw') openDawFolder(path);
    else if (activeTab.id === 'tabPresets') openPresetFolder(path);
    showToast(toastFmt('toast.revealing_first_batch', {n: batchSelected.size}));
}

// Wire up checkbox changes and batch action buttons
document.addEventListener('change', (e) => {
    if (e.target.classList.contains('batch-cb')) {
        const path = getRowPath(e.target.closest('tr'));
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
