// ── Batch Selection ──
// Checkboxes in table rows for multi-item operations

const batchSelected = new Set();

function getRowPath(tr) {
  if (!tr) return null;
  return tr.dataset.audioPath || tr.dataset.dawPath || tr.dataset.presetPath || null;
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
  document.querySelectorAll('.batch-cb').forEach(cb => { cb.checked = false; });
  updateBatchBar();
}

function updateBatchBar() {
  const bar = document.getElementById('batchActionBar');
  if (!bar) return;
  if (batchSelected.size === 0) {
    bar.style.display = 'none';
    return;
  }
  bar.style.display = 'flex';
  document.getElementById('batchCount').textContent = `${batchSelected.size} selected`;
}

function batchFavoriteAll() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return;
  let type = 'sample', items = allAudioSamples;
  if (activeTab.id === 'tabDaw') { type = 'daw'; items = allDawProjects; }
  else if (activeTab.id === 'tabPresets') { type = 'preset'; items = allPresets; }

  let added = 0;
  for (const path of batchSelected) {
    if (isFavorite(path)) continue;
    const item = items.find(i => i.path === path);
    if (item) {
      addFavorite(type, path, item.name, { format: item.format, daw: item.daw });
      added++;
    }
  }
  showToast(`Added ${added} items to favorites`);
  deselectAll();
}

function batchCopyPaths() {
  const paths = [...batchSelected].join('\n');
  copyToClipboard(paths);
  showToast(`Copied ${batchSelected.size} paths`);
}

function batchExportSelected() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab) return;

  let items = [];
  if (activeTab.id === 'tabSamples') {
    items = allAudioSamples.filter(s => batchSelected.has(s.path));
  } else if (activeTab.id === 'tabDaw') {
    items = allDawProjects.filter(p => batchSelected.has(p.path));
  } else if (activeTab.id === 'tabPresets') {
    items = allPresets.filter(p => batchSelected.has(p.path));
  }

  if (items.length === 0) return;
  copyToClipboard(JSON.stringify(items, null, 2));
  showToast(`Copied ${items.length} items as JSON`);
}

function batchRevealAll() {
  const activeTab = document.querySelector('.tab-content.active');
  if (!activeTab || batchSelected.size === 0) return;
  // Reveal first selected item
  const path = [...batchSelected][0];
  if (activeTab.id === 'tabSamples') openAudioFolder(path);
  else if (activeTab.id === 'tabDaw') openDawFolder(path);
  else if (activeTab.id === 'tabPresets') openPresetFolder(path);
  showToast(`Revealing first of ${batchSelected.size} items`);
}

// Wire up checkbox changes and batch action buttons
document.addEventListener('change', (e) => {
  if (e.target.classList.contains('batch-cb')) {
    const path = getRowPath(e.target.closest('tr'));
    if (path) toggleBatchSelect(path, e.target.checked);
  }
});

document.addEventListener('click', (e) => {
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
