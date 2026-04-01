// ── Favorites ──
// Stored in prefs as an array of { type, path, name, ... }

function getFavorites() {
  return prefs.getObject('favorites', []);
}

function saveFavorites(favs) {
  prefs.setItem('favorites', favs);
}

function isFavorite(path) {
  return getFavorites().some(f => f.path === path);
}

function addFavorite(type, path, name, extra) {
  const favs = getFavorites();
  if (favs.some(f => f.path === path)) {
    showToast(`"${name}" is already in favorites`);
    return;
  }
  favs.unshift({ type, path, name, ...extra, addedAt: new Date().toISOString() });
  saveFavorites(favs);
  showToast(`Added "${name}" to favorites`);
}

function removeFavorite(path) {
  const favs = getFavorites().filter(f => f.path !== path);
  saveFavorites(favs);
  showToast('Removed from favorites');
  renderFavorites();
}

function exportFavorites() {
  const favs = getFavorites();
  if (favs.length === 0) { showToast('No favorites to export'); return; }
  _exportCtx = {
    title: 'Favorites',
    defaultName: exportFileName('favorites', favs.length),
    exportFn: async (fmt, filePath) => {
      if (fmt === 'pdf') {
        const headers = ['Name', 'Type', 'Format', 'Path'];
        const rows = favs.map(f => [f.name, f.type, f.format || f.daw || '', f.path]);
        await window.vstUpdater.exportPdf('Favorites', headers, rows, filePath);
      } else if (fmt === 'csv' || fmt === 'tsv') {
        const sep = fmt === 'tsv' ? '\t' : ',';
        const esc = (v) => { const s = String(v || ''); return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s; };
        const lines = ['Name' + sep + 'Type' + sep + 'Format' + sep + 'Path'];
        for (const f of favs) lines.push([f.name, f.type, f.format || f.daw || '', f.path].map(esc).join(sep));
        await window.__TAURI__.core.invoke('write_text_file', { filePath, contents: lines.join('\n') });
      } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({ favorites: favs }, filePath);
      } else {
        await window.__TAURI__.core.invoke('write_text_file', { filePath, contents: JSON.stringify(favs, null, 2) });
      }
    }
  };
  showExportModal('favorites', 'Favorites', favs.length);
}

async function importFavorites() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Favorites', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.favorites || data;
    } else {
      const text = await window.__TAURI__.core.invoke('read_text_file', { filePath }));
      imported = JSON.parse(text);
    }
    if (!Array.isArray(imported)) throw new Error('Expected an array');
    const favs = getFavorites();
    const existing = new Set(favs.map(f => f.path));
    let added = 0;
    for (const item of imported) {
      if (item.path && !existing.has(item.path)) {
        favs.push(item);
        existing.add(item.path);
        added++;
      }
    }
    saveFavorites(favs);
    renderFavorites();
    showToast(`Imported ${added} favorites (${imported.length - added} duplicates skipped)`);
  } catch (e) {
    showToast(`Import failed: ${e.message || e}`, 4000, 'error');
  }
}

function clearFavorites() {
  if (!confirm('Remove all favorites?')) return;
  saveFavorites([]);
  showToast('All favorites cleared');
  renderFavorites();
}

function renderFavorites() {
  const list = document.getElementById('favList');
  const empty = document.getElementById('favEmptyState');
  if (!list) return;

  const favs = getFavorites();
  const search = (document.getElementById('favSearchInput')?.value || '').toLowerCase();
  const typeFilter = document.getElementById('favTypeFilter')?.value || 'all';

  let filtered = favs.filter(f => {
    if (typeFilter !== 'all' && f.type !== typeFilter) return false;
    return true;
  });
  if (search) {
    const scored = filtered.map(f => ({ f, score: searchScore(search, [f.name, f.path], 'fuzzy') })).filter(s => s.score > 0);
    scored.sort((a, b) => b.score - a.score);
    filtered = scored.map(s => s.f);
  }

  if (filtered.length === 0) {
    list.innerHTML = '';
    if (empty) empty.style.display = '';
    if (favs.length > 0 && filtered.length === 0) {
      list.innerHTML = '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching favorites</h2></div>';
      if (empty) empty.style.display = 'none';
    }
    return;
  }
  if (empty) empty.style.display = 'none';

  list.innerHTML = filtered.map(f => {
    const typeLabel = { plugin: 'Plugin', sample: 'Sample', daw: 'DAW Project', preset: 'Preset', folder: 'Folder', file: 'File' }[f.type] || f.type;
    const typeClass = { plugin: 'type-vst3', sample: 'format-wav', daw: 'daw-ableton-live', preset: 'format-default', folder: 'format-default', file: 'format-default' }[f.type] || 'format-default';
    const extra = f.format ? `<span class="format-badge format-default">${escapeHtml(f.format)}</span>` : '';
    const daw = f.daw ? `<span class="format-badge ${getDawBadgeClass ? getDawBadgeClass(f.daw) : 'format-default'}">${escapeHtml(f.daw)}</span>` : '';
    const hp = escapeHtml(f.path);
    const isPlaying = f.type === 'sample' && typeof audioPlayerPath !== 'undefined' && audioPlayerPath === f.path && typeof audioPlayer !== 'undefined' && !audioPlayer.paused;
    const playBtn = f.type === 'sample'
      ? `<button class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${hp}" title="Play">${isPlaying ? '&#9646;&#9646;' : '&#9654;'}</button>`
      : '';
    const loopBtn = f.type === 'sample'
      ? `<button class="btn-small btn-loop" data-action="toggleRowLoop" data-path="${hp}" title="Loop">&#8634;</button>`
      : '';
    const cursor = (f.type === 'sample' || f.type === 'daw') ? ' style="cursor:pointer;"' : '';
    return `<div class="fav-item" data-path="${hp}" data-type="${f.type}" data-name="${escapeHtml(f.name)}"${cursor}>
      <span class="fav-star">&#9733;</span>
      <span class="fav-type"><span class="format-badge ${typeClass}">${typeLabel}</span></span>
      <span class="fav-name" title="${hp}">${escapeHtml(f.name)}</span>
      ${extra}${daw}
      <span class="fav-actions">
        ${playBtn}${loopBtn}
        <button class="btn-small btn-folder" data-action="openFavFolder" data-path="${hp}" data-type="${f.type}" title="Reveal in Finder">&#128193;</button>
        <button class="btn-small btn-stop" data-action="removeFav" data-path="${hp}" title="Remove from favorites">&#10005;</button>
      </span>
    </div>`;
  }).join('');
}

// Wire up fav actions via delegation
document.addEventListener('click', (e) => {
  const el = e.target.closest('[data-action="removeFav"]');
  if (el) {
    removeFavorite(el.dataset.path);
    return;
  }
  const folder = e.target.closest('[data-action="openFavFolder"]');
  if (folder) {
    const type = folder.dataset.type;
    const path = folder.dataset.path;
    if (type === 'plugin') openFolder(path);
    else if (type === 'sample') openAudioFolder(path);
    else if (type === 'daw') openDawFolder(path);
    else if (type === 'preset') openPresetFolder(path);
    return;
  }
  // Single click on sample favorite → play
  const favItem = e.target.closest('.fav-item[data-type="sample"]');
  if (favItem && !e.target.closest('.fav-actions') && !e.target.closest('button')) {
    const path = favItem.dataset.path;
    if (path && typeof previewAudio === 'function') previewAudio(path);
  }
});

// Double-click on DAW favorite → open in DAW, plugin → open KVR
document.addEventListener('dblclick', (e) => {
  const favItem = e.target.closest('.fav-item');
  if (!favItem || e.target.closest('.fav-actions') || e.target.closest('button')) return;
  const type = favItem.dataset.type;
  const path = favItem.dataset.path;
  const name = favItem.dataset.name || '';

  if (type === 'daw') {
    const daw = favItem.querySelector('.format-badge')?.textContent || 'DAW';
    showToast(`Opening "${name}" in ${daw}...`);
    window.vstUpdater.openDawProject(path).catch(err => showToast(`${daw} not installed — ${err}`, 4000, 'error'));
  } else if (type === 'plugin') {
    const plugin = typeof allPlugins !== 'undefined' && allPlugins.find(p => p.path === path);
    const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
    window.vstUpdater.openUpdate(kvrUrl);
  } else if (type === 'preset') {
    if (typeof openPresetFolder === 'function') openPresetFolder(path);
  }
});
