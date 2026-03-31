// ── File Browser ──

let _fileBrowserPath = null;
let _fileBrowserEntries = [];
let _fileBrowserInited = false;

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
  const name = dirPath.split('/').filter(Boolean).pop() || dirPath;
  dirs.push({ path: dirPath, name });
  saveFavDirs(dirs);
  renderFavDirs();
  updateBookmarkBtn();
  showToast(`Bookmarked "${name}"`);
}

function removeFavDir(dirPath) {
  saveFavDirs(getFavDirs().filter(d => d.path !== dirPath));
  renderFavDirs();
  updateBookmarkBtn();
  showToast('Bookmark removed');
}

function renderFavDirs() {
  const container = document.getElementById('fileFavs');
  const grid = document.getElementById('fileFavsGrid');
  if (!container || !grid) return;
  const dirs = getFavDirs();
  if (dirs.length === 0) {
    container.style.display = 'none';
    return;
  }
  container.style.display = '';
  grid.innerHTML = dirs.map(d =>
    `<div class="file-fav-chip" data-fav-dir="${escapeHtml(d.path)}" title="${escapeHtml(d.path)}">
      <span class="fav-chip-icon">&#128193;</span>
      <span class="fav-chip-name">${escapeHtml(d.name)}</span>
      <span class="fav-chip-remove" data-remove-fav-dir="${escapeHtml(d.path)}" title="Remove bookmark">&#10005;</span>
    </div>`
  ).join('');
}

function updateBookmarkBtn() {
  const btn = document.getElementById('btnFileFav');
  if (!btn || !_fileBrowserPath) return;
  const fav = isFavDir(_fileBrowserPath);
  btn.innerHTML = fav ? '&#9733; Unbookmark' : '&#9733; Bookmark';
  btn.title = fav ? 'Remove current directory from bookmarks' : 'Bookmark current directory';
}

const AUDIO_EXTS = ['wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'aac'];
const DAW_EXTS = ['als', 'logicx', 'flp', 'rpp', 'cpr', 'npr', 'ptx', 'ptf', 'song', 'reason', 'aup', 'aup3', 'band', 'ardour', 'dawproject', 'bwproject'];
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

async function initFileBrowser() {
  renderFavDirs();
  if (_fileBrowserPath) {
    await loadDirectory(_fileBrowserPath);
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
  _fileBrowserPath = dirPath;
  showGlobalProgress();
  try {
    const result = await window.vstUpdater.listDirectory(dirPath);
    _fileBrowserEntries = result.entries;
    renderFileList();
    renderBreadcrumb(dirPath);
    updateBookmarkBtn();
  } catch (err) {
    showToast(`Failed to open directory — ${err.message || err}`, 4000, 'error');
  } finally {
    hideGlobalProgress();
  }
}

function renderBreadcrumb(dirPath) {
  const el = document.getElementById('fileBreadcrumb');
  if (!el) return;
  const parts = dirPath.split('/').filter(Boolean);
  let html = `<span class="file-crumb" data-file-nav="/">/</span>`;
  let accumulated = '';
  for (const part of parts) {
    accumulated += '/' + part;
    html += `<span class="file-crumb-sep">/</span><span class="file-crumb" data-file-nav="${escapeHtml(accumulated)}">${escapeHtml(part)}</span>`;
  }
  el.innerHTML = html;
}

function renderFileList() {
  const list = document.getElementById('fileList');
  if (!list) return;
  const search = (document.getElementById('fileSearchInput')?.value || '').toLowerCase();
  const filtered = search
    ? _fileBrowserEntries.filter(e => e.name.toLowerCase().includes(search))
    : _fileBrowserEntries;

  if (filtered.length === 0) {
    list.innerHTML = '<div class="state-message"><div class="state-icon">&#128193;</div><h2>Empty Directory</h2></div>';
    return;
  }

  list.innerHTML = filtered.map(e => {
    const note = noteIndicator(e.path);
    const cls = e.isDir ? ' file-dir' : '';
    return `<div class="file-row${cls}" data-file-path="${escapeHtml(e.path)}" data-file-dir="${e.isDir}">
      <span class="file-icon">${fileIcon(e)}</span>
      <span class="file-name">${note}${escapeHtml(e.name)}</span>
      <span class="file-ext">${e.isDir ? 'DIR' : e.ext}</span>
      <span class="file-size">${e.isDir ? '' : e.sizeFormatted}</span>
      <span class="file-date">${e.modified}</span>
    </div>`;
  }).join('');
}

// Click to navigate dirs or play/open files
document.addEventListener('click', (e) => {
  const crumb = e.target.closest('[data-file-nav]');
  if (crumb) {
    loadDirectory(crumb.dataset.fileNav);
    return;
  }

  const row = e.target.closest('.file-row');
  if (row) {
    const path = row.dataset.filePath;
    const isDir = row.dataset.fileDir === 'true';
    if (isDir) {
      loadDirectory(path);
    } else {
      const ext = path.split('.').pop().toLowerCase();
      if (AUDIO_EXTS.includes(ext)) {
        previewAudio(path);
      } else {
        opener_open(path);
      }
    }
    return;
  }
});

function opener_open(path) {
  window.vstUpdater.openDawProject(path).catch(() => {
    window.vstUpdater.openPresetFolder(path);
  });
}

// Action handlers
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action]');
  if (!action) return;
  if (action.dataset.action === 'fileUp') {
    if (_fileBrowserPath && _fileBrowserPath !== '/') {
      const parent = _fileBrowserPath.replace(/\/[^/]+\/?$/, '') || '/';
      loadDirectory(parent);
    }
  } else if (action.dataset.action === 'fileHome') {
    window.vstUpdater.getHomeDir().then(h => loadDirectory(h)).catch(() => {});
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

// Filter
document.addEventListener('input', (e) => {
  if (e.target.dataset.action === 'filterFiles') renderFileList();
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
    items.push({ icon: '&#128193;', label: 'Open Folder', action: () => loadDirectory(path) });
    items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => window.vstUpdater.openPresetFolder(path) });
    const dirFav = isFavDir(path);
    items.push({ icon: dirFav ? '&#9734;' : '&#9733;', label: dirFav ? 'Remove Bookmark' : 'Bookmark Directory',
      action: () => dirFav ? removeFavDir(path) : addFavDir(path) });
  } else {
    if (AUDIO_EXTS.includes(ext)) {
      items.push({ icon: '&#9654;', label: 'Play', action: () => previewAudio(path) });
    }
    items.push({ icon: '&#128194;', label: 'Open', action: () => opener_open(path) });
    items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => {
      const dir = path.replace(/\/[^/]+$/, '');
      window.vstUpdater.openPresetFolder(path);
    }});
  }
  items.push('---');
  items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) });
  items.push({ icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name.replace(/^[\u{1F4DD}]/u, '').trim()) });
  items.push('---');

  // Tags & notes
  const note = getNote(path);
  items.push({ icon: '&#128221;', label: note ? 'Edit Note' : 'Add Note', action: () => showNoteEditor(path, name) });

  const allTags = getAllTags();
  const currentTags = note?.tags || [];
  if (allTags.length > 0) {
    items.push('---');
    for (const tag of allTags.slice(0, 6)) {
      const has = currentTags.includes(tag);
      items.push({ icon: has ? '&#10003;' : '&#9634;', label: `${has ? 'Remove' : 'Add'} tag: ${tag}`,
        action: () => { if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag); showToast(`Tag "${tag}" ${has ? 'removed' : 'added'}`); renderFileList(); }
      });
    }
  }

  items.push('---');
  const fav = isFavorite(path);
  items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? 'Remove from Favorites' : 'Add to Favorites',
    action: () => fav ? removeFavorite(path) : addFavorite(isDir ? 'daw' : 'sample', path, name, { format: ext.toUpperCase() })
  });

  items.push('---');
  items.push({ icon: '&#128465;', label: 'Delete', action: async () => {
    if (!confirm(`Delete "${name}"? This cannot be undone.`)) return;
    try {
      await window.vstUpdater.deleteFile(path);
      showToast(`Deleted "${name}"`);
      loadDirectory(_fileBrowserPath);
    } catch (err) {
      showToast(`Delete failed — ${err.message || err}`, 4000, 'error');
    }
  }});

  showContextMenu(e, items);
  e.preventDefault();
});
