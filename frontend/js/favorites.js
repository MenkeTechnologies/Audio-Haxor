// ── Favorites ──
// Stored in prefs as an array of { type, path, name, ... }

// Cache favorites array + Set for O(1) isFavorite lookups.
let _favsCache = null;
let _favsPathSet = null;
function getFavorites() {
  if (!_favsCache) {
    _favsCache = prefs.getObject('favorites', []);
    _favsPathSet = new Set(_favsCache.map(f => f.path));
  }
  return _favsCache;
}

function saveFavorites(favs) {
  _favsCache = null;
  _favsPathSet = null;
  prefs.setItem('favorites', favs);
}

function isFavorite(path) {
  if (!_favsPathSet) getFavorites();
  return _favsPathSet.has(path);
}

function addFavorite(type, path, name, extra) {
  const favs = getFavorites();
  if (favs.some(f => f.path === path)) {
    showToast(toastFmt('toast.already_in_favorites', { name }));
    return;
  }
  favs.unshift({ type, path, name, ...extra, addedAt: new Date().toISOString() });
  saveFavorites(favs);
  showToast(toastFmt('toast.added_to_favorites', { name }));
  if (typeof refreshRowBadges === 'function') refreshRowBadges(path);
}

function removeFavorite(path) {
  const favs = getFavorites().filter(f => f.path !== path);
  saveFavorites(favs);
  showToast(toastFmt('toast.removed_from_favorites'));
  if (typeof refreshRowBadges === 'function') refreshRowBadges(path);
  renderFavorites();
}

function exportFavorites() {
  const favs = getFavorites();
  if (favs.length === 0) { showToast(toastFmt('toast.no_favorites_export')); return; }
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
      const text = await window.__TAURI__.core.invoke('read_text_file', { filePath });
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
    showToast(toastFmt('toast.imported_favorites', { added, dup: imported.length - added }));
  } catch (e) {
    showToast(toastFmt('toast.import_failed_favs', { err: e.message || e }), 4000, 'error');
  }
}

function clearFavorites() {
  if (!confirm(appFmt('confirm.remove_all_favorites'))) return;
  saveFavorites([]);
  showToast(toastFmt('toast.all_favorites_cleared'));
  renderFavorites();
}

let _favSearch = '';

registerFilter('filterFavorites', {
  inputId: 'favSearchInput',
  resetOffset() { _favRenderCount = 0; },
  fetchFn() { _favSearch = this.lastSearch || ''; renderFavorites(); },
});

function renderFavorites() {
  if (typeof saveAllFilterStates === 'function') saveAllFilterStates();
  const list = document.getElementById('favList');
  const empty = document.getElementById('favEmptyState');
  if (!list) return;

  const favs = getFavorites();
  const search = _favSearch || (document.getElementById('favSearchInput')?.value || '').trim();
  const typeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('favTypeFilter') : null;

  let filtered = favs.filter(f => {
    if (typeSet && !typeSet.has(f.type)) return false;
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

  const FAV_PAGE = 200;
  _favFiltered = filtered;
  _favRenderCount = 0;
  const page = filtered.slice(0, FAV_PAGE);
  list.innerHTML = page.map(f => {
    const typeLabel = { plugin: 'Plugin', sample: 'Sample', daw: 'DAW Project', preset: 'Preset', folder: 'Folder', file: 'File' }[f.type] || f.type;
    const typeClass = { plugin: 'type-vst3', sample: 'format-wav', daw: 'daw-ableton-live', preset: 'format-default', folder: 'format-default', file: 'format-default' }[f.type] || 'format-default';
    const extra = f.format ? `<span class="format-badge format-default">${escapeHtml(f.format)}</span>` : '';
    const daw = f.daw ? `<span class="format-badge ${getDawBadgeClass ? getDawBadgeClass(f.daw) : 'format-default'}">${escapeHtml(f.daw)}</span>` : '';
    const hp = escapeHtml(f.path);
    const isPlaying = f.type === 'sample' && typeof audioPlayerPath !== 'undefined' && audioPlayerPath === f.path && (typeof isAudioPlaying === 'function' ? isAudioPlaying() : typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused);
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
      <span class="fav-name" title="${hp}">${_favSearch && typeof highlightMatch === 'function' ? highlightMatch(f.name, _favSearch, 'fuzzy') : escapeHtml(f.name)}</span>
      ${extra}${daw}
      <span class="fav-actions">
        ${playBtn}${loopBtn}
        <button class="btn-small btn-folder" data-action="openFavFolder" data-path="${hp}" data-type="${f.type}" title="Reveal in Finder">&#128193;</button>
        <button class="btn-small btn-stop" data-action="removeFav" data-path="${hp}" title="Remove from favorites">&#10005;</button>
      </span>
    </div>`;
  }).join('');
  _favRenderCount = page.length;
  if (_favRenderCount < filtered.length) {
    list.insertAdjacentHTML('beforeend',
      `<div id="favLoadMore" data-action="loadMoreFavs" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        Showing ${_favRenderCount} of ${filtered.length} — click to load more
      </div>`);
  }
  if (typeof initFavDragReorder === 'function') requestAnimationFrame(initFavDragReorder);
}
let _favFiltered = [];
let _favRenderCount = 0;
function loadMoreFavs() {
  const FAV_PAGE = 200;
  const list = document.getElementById('favList');
  const more = document.getElementById('favLoadMore');
  if (more) more.remove();
  const next = _favFiltered.slice(_favRenderCount, _favRenderCount + FAV_PAGE);
  // Reuse the same rendering from renderFavorites — inline here
  list.insertAdjacentHTML('beforeend', next.map(f => {
    const typeLabel = { plugin: 'Plugin', sample: 'Sample', daw: 'DAW Project', preset: 'Preset', folder: 'Folder', file: 'File' }[f.type] || f.type;
    const typeClass = { plugin: 'type-vst3', sample: 'format-wav', daw: 'daw-ableton-live', preset: 'format-default', folder: 'format-default', file: 'format-default' }[f.type] || 'format-default';
    const extra = f.format ? `<span class="format-badge format-default">${escapeHtml(f.format)}</span>` : '';
    const hp = escapeHtml(f.path);
    return `<div class="fav-item" data-path="${hp}" data-type="${f.type}" data-name="${escapeHtml(f.name)}">
      <span class="fav-star">&#9733;</span>
      <span class="fav-type"><span class="format-badge ${typeClass}">${typeLabel}</span></span>
      <span class="fav-name" title="${hp}">${_favSearch && typeof highlightMatch === 'function' ? highlightMatch(f.name, _favSearch, 'fuzzy') : escapeHtml(f.name)}</span>${extra}
      <span class="fav-actions">
        <button class="btn-small btn-folder" data-action="openFavFolder" data-path="${hp}" data-type="${f.type}" title="Reveal in Finder">&#128193;</button>
        <button class="btn-small btn-stop" data-action="removeFav" data-path="${hp}" title="Remove">&#10005;</button>
      </span>
    </div>`;
  }).join(''));
  _favRenderCount += next.length;
  if (_favRenderCount < _favFiltered.length) {
    list.insertAdjacentHTML('beforeend',
      `<div id="favLoadMore" data-action="loadMoreFavs" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        Showing ${_favRenderCount} of ${_favFiltered.length} — click to load more
      </div>`);
  }
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
    showToast(toastFmt('toast.opening_in_daw', { name, daw }));
    window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', { daw, err }), 4000, 'error'));
  } else if (type === 'plugin') {
    const plugin = typeof allPlugins !== 'undefined' && findByPath(allPlugins, path);
    const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
    window.vstUpdater.openUpdate(kvrUrl);
  } else if (type === 'preset') {
    if (typeof openPresetFolder === 'function') openPresetFolder(path);
  }
});
