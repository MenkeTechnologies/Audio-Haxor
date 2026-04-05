// ── File Browser ──
const _ctxMenuNoEcho = { skipEchoToast: true };

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
  showToast(toastFmt('toast.bookmarked_name', { name }));
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

const AUDIO_EXTS = ['wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'aac', 'opus', 'wma'];
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
  // Reset cursor cache when directory changes
  window._fbCursorPath = null;
  window._fbCursorEl = null;
  _wfQueue = [];
  showGlobalProgress();
  try {
    const result = await window.vstUpdater.listDirectory(dirPath);
    _fileBrowserEntries = result.entries;
    renderFileList();
    renderBreadcrumb(dirPath);
    updateBookmarkBtn();
  } catch (err) {
    showToast(toastFmt('toast.failed_open_directory', { err: err.message || err }), 4000, 'error');
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
  const search = (document.getElementById('fileSearchInput')?.value || '').trim();
  let filtered;
  if (search) {
    const scored = _fileBrowserEntries.map(e => {
      const score = typeof searchScore === 'function' ? searchScore(search, [e.name, e.ext], 'fuzzy') : (e.name.toLowerCase().includes(search.toLowerCase()) ? 1 : 0);
      return { entry: e, score };
    }).filter(s => s.score > 0);
    scored.sort((a, b) => b.score - a.score);
    filtered = scored.map(s => s.entry);
  } else {
    filtered = _fileBrowserEntries;
  }

  if (filtered.length === 0) {
    list.innerHTML = '<div class="state-message"><div class="state-icon">&#128193;</div><h2>Empty Directory</h2></div>';
    return;
  }

  list.innerHTML = filtered.map(e => {
    const note = typeof noteIndicator === 'function' ? noteIndicator(e.path) : '';
    const cls = e.isDir ? ' file-dir' : '';
    const isAudio = !e.isDir && AUDIO_EXTS.includes(e.ext);

    // Metadata badges — favorites/tags for ALL files and dirs, audio-specific for audio
    const parts = [];
    // Favorite (all files and dirs)
    if (typeof isFavorite === 'function' && isFavorite(e.path)) {
      parts.push('<span class="file-meta-tag file-meta-fav" title="Favorited">&#9733;</span>');
    }
    // Tags from notes (all files and dirs)
    if (typeof getNote === 'function') {
      const n = getNote(e.path);
      if (n && n.tags && n.tags.length > 0) {
        parts.push(`<span class="file-meta-tag file-meta-tags" title="Tags: ${escapeHtml(n.tags.join(', '))}">${escapeHtml(n.tags.slice(0, 2).join(', '))}${n.tags.length > 2 ? '…' : ''}</span>`);
      }
    }
    // Audio-specific metadata
    if (isAudio) {
      if (typeof _bpmCache !== 'undefined' && _bpmCache[e.path]) {
        parts.push(`<span class="file-meta-tag file-meta-bpm" title="BPM">${_bpmCache[e.path]}</span>`);
      }
      if (typeof _keyCache !== 'undefined' && _keyCache[e.path]) {
        parts.push(`<span class="file-meta-tag file-meta-key" title="Musical key">${escapeHtml(_keyCache[e.path])}</span>`);
      }
      if (typeof allAudioSamples !== 'undefined') {
        const sample = findByPath(allAudioSamples, e.path);
        if (sample && sample.duration) {
          parts.push(`<span class="file-meta-tag file-meta-dur" title="Duration">${typeof formatTime === 'function' ? formatTime(sample.duration) : sample.duration.toFixed(1) + 's'}</span>`);
        }
      }
    }
    const extras = parts.length > 0 ? `<span class="file-meta-tags-row">${parts.join('')}</span>` : '';

    const wfBg = isAudio ? `<canvas class="file-waveform" data-wf-path="${escapeHtml(e.path)}" height="36" title="Waveform"></canvas><span class="file-wf-cursor"></span>` : '';
    return `<div class="file-row${cls}${isAudio ? ' file-audio' : ''}" data-file-path="${escapeHtml(e.path)}" data-file-dir="${e.isDir}" ${isAudio ? `data-wf-file="${escapeHtml(e.path)}"` : ''}>
      ${wfBg}
      <span class="file-icon">${fileIcon(e)}</span>
      <span class="file-name">${search && typeof highlightMatch === 'function' ? highlightMatch(e.name, search, 'fuzzy') : escapeHtml(e.name)}${extras}${note}</span>
      <span class="file-ext">${e.isDir ? 'DIR' : e.ext}</span>
      <span class="file-size">${e.isDir ? '' : e.sizeFormatted}</span>
      <span class="file-date">${e.modified}</span>
    </div>`;
  }).join('');

  // Lazy-load waveforms for visible audio files
  requestAnimationFrame(() => initFileBrowserWaveforms());
}

// ── Lazy waveform rendering for file browser audio rows ──
let _wfQueue = [];
let _wfActive = 0;
const _wfMaxConcurrent = 4;

function _processWfQueue() {
  while (_wfActive < _wfMaxConcurrent && _wfQueue.length > 0) {
    const { canvas, path } = _wfQueue.shift();
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
  if (_fbWfObserver) { _fbWfObserver.disconnect(); _fbWfObserver = null; }

  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const canvas = entry.target;
      if (canvas._wfDrawn) continue;
      canvas._wfDrawn = true;
      observer.unobserve(canvas);
      _wfQueue.push({ canvas, path: canvas.dataset.wfPath });
    }
    _processWfQueue();
  }, { root: container.closest('.tab-content'), threshold: 0.1 });

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

  try {
    if (!window._fbAudioCtx) window._fbAudioCtx = new AudioContext();
    const src = typeof convertFileSrc === 'function' ? convertFileSrc(filePath) : filePath;
    const resp = await fetch(src);
    const buf = await resp.arrayBuffer();
    const audioBuf = await window._fbAudioCtx.decodeAudioData(buf);
    const raw = audioBuf.getChannelData(0);

    const bars = w;
    const step = Math.floor(raw.length / bars);
    const peaks = [];
    for (let i = 0; i < bars; i++) {
      let max = 0, min = 0;
      const start = i * step;
      for (let j = start; j < start + step && j < raw.length; j++) {
        if (raw[j] > max) max = raw[j];
        if (raw[j] < min) min = raw[j];
      }
      peaks.push({ max, min });
    }

    if (typeof _waveformCache !== 'undefined') _waveformCache[filePath] = peaks;
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
    if (wasPath === filePath) { _fbExpandedPath = null; return; }
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

    const fmtDate = (v) => { if (!v) return '—'; const d = new Date(v); return isNaN(d) ? '—' : d.toLocaleString(); };
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
        } catch { const el = document.getElementById('fbBpmVal'); if (el) el.textContent = '—'; }
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
        } catch { const el = document.getElementById('fbKeyVal'); if (el) el.textContent = '—'; }
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
  if (row && !e.target.closest('.fb-meta-panel')) {
    const path = row.dataset.filePath;
    const isDir = row.dataset.fileDir === 'true';
    if (isDir) {
      loadDirectory(path);
    } else {
      const ext = path.split('.').pop().toLowerCase();
      if (AUDIO_EXTS.includes(ext)) {
        previewAudio(path);
        toggleFileBrowserMeta(path);
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
    window.vstUpdater.getHomeDir().then(h => loadDirectory(h)).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
  } else if (action.dataset.action === 'fileQuickNav') {
    const dir = action.dataset.dir;
    if (dir === '/') {
      loadDirectory('/');
    } else {
      window.vstUpdater.getHomeDir().then(h => loadDirectory(h + '/' + dir)).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
    }
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

// Filter — uses unified filter system
registerFilter('filterFiles', {
  inputId: 'fileSearchInput',
  fetchFn() { renderFileList(); },
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
    items.push({ icon: '&#128193;', label: appFmt('menu.open_folder'), ..._ctxMenuNoEcho, action: () => loadDirectory(path) });
    items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, action: () => window.vstUpdater.openPresetFolder(path) });
    const dirFav = isFavDir(path);
    items.push({ icon: dirFav ? '&#9734;' : '&#9733;', label: dirFav ? appFmt('menu.remove_bookmark') : appFmt('menu.bookmark_directory'), ..._ctxMenuNoEcho,
      action: () => dirFav ? removeFavDir(path) : addFavDir(path) });
  } else {
    if (AUDIO_EXTS.includes(ext)) {
      items.push({ icon: '&#9654;', label: appFmt('menu.play'), ..._ctxMenuNoEcho, action: () => previewAudio(path) });
    }
    items.push({ icon: '&#128194;', label: appFmt('menu.open'), ..._ctxMenuNoEcho, action: () => opener_open(path) });
    items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, action: () => {
      const dir = path.replace(/\/[^/]+$/, '');
      window.vstUpdater.openPresetFolder(path);
    }});
  }
  items.push('---');
  items.push({ icon: '&#128203;', label: appFmt('menu.copy_path'), ..._ctxMenuNoEcho, action: () => copyToClipboard(path) });
  items.push({ icon: '&#128203;', label: appFmt('menu.copy_name'), ..._ctxMenuNoEcho, action: () => copyToClipboard(name.replace(/^[\u{1F4DD}]/u, '').trim()) });
  items.push('---');

  // ALS XML viewer
  if (ext === 'als' && typeof showAlsViewer === 'function') {
    items.push({ icon: '&#128196;', label: appFmt('menu.explore_xml_contents'), action: () => showAlsViewer(path, name) });
    items.push('---');
  }

  // Tags & notes
  const note = getNote(path);
  items.push({ icon: '&#128221;', label: note ? appFmt('menu.edit_note') : appFmt('menu.add_note'), action: () => showNoteEditor(path, name) });

  const allTags = getAllTags();
  const currentTags = note?.tags || [];
  if (allTags.length > 0) {
    items.push('---');
    for (const tag of allTags.slice(0, 6)) {
      const has = currentTags.includes(tag);
      items.push({ icon: has ? '&#10003;' : '&#9634;', label: has ? appFmt('menu.remove_tag_named', { tag }) : appFmt('menu.add_tag_named', { tag }), ..._ctxMenuNoEcho,
        action: () => { if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag); showToast(has ? toastFmt('toast.tag_removed', { tag }) : toastFmt('toast.tag_added', { tag })); renderFileList(); }
      });
    }
  }

  items.push('---');
  const fav = isFavorite(path);
  items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._ctxMenuNoEcho,
    action: () => fav ? removeFavorite(path) : addFavorite(isDir ? 'folder' : (AUDIO_EXTS.includes(ext) ? 'sample' : 'file'), path, name, { format: ext.toUpperCase() })
  });

  items.push('---');
  items.push({ icon: '&#128465;', label: appFmt('menu.delete'), action: async () => {
    if (!confirm(appFmt('confirm.delete_file_browser', { name }))) return;
    try {
      await window.vstUpdater.deleteFile(path);
      showToast(toastFmt('toast.deleted_name_quotes', { name }));
      loadDirectory(_fileBrowserPath);
    } catch (err) {
      showToast(toastFmt('toast.delete_failed_msg', { err: err.message || err }), 4000, 'error');
    }
  }});

  showContextMenu(e, items);
  e.preventDefault();
});

// ── Ableton-style keyboard navigation ──
let _fileNavIdx = -1;

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
  row.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
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

  const rows = getFileRows();
  if (rows.length === 0) return;

  if (e.key === 'ArrowDown' || e.key === 'j') {
    e.preventDefault();
    fileNavSelect(_fileNavIdx + 1);
  } else if (e.key === 'ArrowUp' || e.key === 'k') {
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
    if (_fileBrowserPath && _fileBrowserPath !== '/') {
      const parent = _fileBrowserPath.replace(/\/[^/]+\/?$/, '') || '/';
      loadDirectory(parent).then(() => {
        _fileNavIdx = -1;
        fileNavSelect(0);
      });
    }
  } else if (e.key === ' ') {
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
