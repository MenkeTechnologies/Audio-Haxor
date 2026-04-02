// ── Command Palette (Cmd+K) ──

let _paletteOpen = false;
let _paletteQuery = '';
let _paletteResults = [];
let _paletteSelected = 0;

const PALETTE_MAX = 50;

function collectPaletteItems() {
  const items = [];

  // Tabs — always available
  const tabs = [
    { type: 'tab', name: 'Plugins', icon: '&#9889;', action: () => switchTab('plugins') },
    { type: 'tab', name: 'Samples', icon: '&#127925;', action: () => switchTab('samples') },
    { type: 'tab', name: 'DAW Projects', icon: '&#127911;', action: () => switchTab('daw') },
    { type: 'tab', name: 'Presets', icon: '&#127924;', action: () => switchTab('presets') },
    { type: 'tab', name: 'Favorites', icon: '&#9733;', action: () => switchTab('favorites') },
    { type: 'tab', name: 'Notes', icon: '&#128221;', action: () => switchTab('notes') },
    { type: 'tab', name: 'Tags', icon: '&#127991;', action: () => switchTab('tags') },
    { type: 'tab', name: 'History', icon: '&#128197;', action: () => switchTab('history') },
    { type: 'tab', name: 'Files', icon: '&#128193;', action: () => switchTab('files') },
    { type: 'tab', name: 'Settings', icon: '&#9881;', action: () => switchTab('settings') },
  ];
  items.push(...tabs);

  // Actions
  items.push({ type: 'action', name: 'Scan Plugins', icon: '&#8635;', action: () => scanPlugins() });
  items.push({ type: 'action', name: 'Scan Samples', icon: '&#8635;', action: () => scanAudioSamples() });
  items.push({ type: 'action', name: 'Scan DAW Projects', icon: '&#8635;', action: () => scanDawProjects() });
  items.push({ type: 'action', name: 'Scan Presets', icon: '&#8635;', action: () => scanPresets() });
  items.push({ type: 'action', name: 'Check Updates', icon: '&#9889;', action: () => checkUpdates() });
  items.push({ type: 'action', name: 'Find Duplicates', icon: '&#128270;', action: () => showDuplicateReport() });
  items.push({ type: 'action', name: 'Reset All Scans', icon: '&#128465;', action: () => resetAllScans() });
  if (typeof buildXrefIndex === 'function') {
    items.push({ type: 'action', name: 'Build Plugin Index', icon: '&#9889;', action: () => buildXrefIndex() });
  }
  if (typeof showDepGraph === 'function') {
    items.push({ type: 'action', name: 'Plugin Dependency Graph', icon: '&#128200;', action: () => showDepGraph() });
  }
  if (typeof findSimilarSamples === 'function' && typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
    items.push({ type: 'action', name: 'Find Similar to Current Track', icon: '&#128270;', action: () => findSimilarSamples(audioPlayerPath) });
  }

  // Plugins
  if (typeof allPlugins !== 'undefined') {
    for (const p of allPlugins) {
      items.push({
        type: 'plugin', name: p.name, detail: p.type + (p.manufacturer ? ' · ' + p.manufacturer : ''),
        icon: '&#9889;', fields: [p.name, p.type, p.manufacturer || '', p.path],
        action: () => { switchTab('plugins'); setTimeout(() => { document.getElementById('pluginSearchInput').value = p.name; filterPlugins(); }, 100); }
      });
    }
  }

  // Samples
  if (typeof allAudioSamples !== 'undefined') {
    for (const s of allAudioSamples) {
      items.push({
        type: 'sample', name: s.name, detail: s.format + ' · ' + s.sizeFormatted,
        icon: '&#127925;', fields: [s.name, s.format, s.path],
        action: () => { switchTab('samples'); previewAudio(s.path); }
      });
    }
  }

  // DAW Projects
  if (typeof allDawProjects !== 'undefined') {
    for (const d of allDawProjects) {
      items.push({
        type: 'daw', name: d.name, detail: d.daw + ' · ' + d.sizeFormatted,
        icon: '&#127911;', fields: [d.name, d.daw, d.format, d.path],
        action: () => { switchTab('daw'); setTimeout(() => { document.getElementById('dawSearchInput').value = d.name; filterDawProjects(); }, 100); }
      });
    }
  }

  // Presets
  if (typeof allPresets !== 'undefined') {
    for (const p of allPresets) {
      items.push({
        type: 'preset', name: p.name, detail: p.format,
        icon: '&#127924;', fields: [p.name, p.format, p.path],
        action: () => { switchTab('presets'); setTimeout(() => { document.getElementById('presetSearchInput').value = p.name; filterPresets(); }, 100); }
      });
    }
  }

  // Bookmarked dirs
  if (typeof getFavDirs === 'function') {
    for (const d of getFavDirs()) {
      items.push({
        type: 'bookmark', name: d.name, detail: d.path,
        icon: '&#128278;', fields: [d.name, d.path],
        action: () => { switchTab('files'); loadDirectory(d.path); }
      });
    }
  }

  // Tags
  if (typeof getAllTags === 'function') {
    for (const t of getAllTags()) {
      items.push({
        type: 'tag', name: t, detail: 'Tag',
        icon: '&#127991;', fields: [t],
        action: () => { if (typeof setGlobalTag === 'function') setGlobalTag(t); switchTab('plugins'); }
      });
    }
  }

  return items;
}

function filterPaletteItems(query, items) {
  if (!query) {
    // Show tabs and actions when empty
    return items.filter(i => i.type === 'tab' || i.type === 'action');
  }
  const scored = [];
  for (const item of items) {
    const fields = item.fields || [item.name];
    const score = searchScore(query, fields, 'fuzzy');
    if (score > 0) scored.push({ item, score });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, PALETTE_MAX).map(s => s.item);
}

function openPalette() {
  if (_paletteOpen) return;
  _paletteOpen = true;
  _paletteQuery = '';
  _paletteSelected = 0;

  const html = `<div class="palette-overlay" id="paletteOverlay">
    <div class="palette-box">
      <input type="text" class="palette-input" id="paletteInput" placeholder="Search everything... (plugins, samples, projects, actions)" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">
      <div class="palette-results" id="paletteResults"></div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);

  const input = document.getElementById('paletteInput');
  input.focus();
  renderPaletteResults();

  input.addEventListener('input', () => {
    _paletteQuery = input.value;
    _paletteSelected = 0;
    renderPaletteResults();
  });
}

function closePalette() {
  if (!_paletteOpen) return;
  _paletteOpen = false;
  const overlay = document.getElementById('paletteOverlay');
  if (overlay) overlay.remove();
}

function renderPaletteResults() {
  const container = document.getElementById('paletteResults');
  if (!container) return;

  const allItems = collectPaletteItems();
  _paletteResults = filterPaletteItems(_paletteQuery, allItems);

  if (_paletteResults.length === 0) {
    container.innerHTML = '<div class="palette-empty">No results</div>';
    return;
  }

  container.innerHTML = _paletteResults.map((item, i) => {
    const typeCls = 'palette-type-' + item.type;
    const sel = i === _paletteSelected ? ' palette-selected' : '';
    const typeLabel = { tab: 'Tab', action: 'Action', plugin: 'Plugin', sample: 'Sample', daw: 'DAW', preset: 'Preset', bookmark: 'Dir', tag: 'Tag' }[item.type] || item.type;
    const detail = item.detail ? `<span class="palette-detail">${escapeHtml(item.detail)}</span>` : '';
    return `<div class="palette-row${sel}" data-palette-idx="${i}">
      <span class="palette-icon">${item.icon}</span>
      <span class="palette-name">${_paletteQuery ? highlightMatch(item.name, _paletteQuery, 'fuzzy') : escapeHtml(item.name)}</span>
      ${detail}
      <span class="palette-badge ${typeCls}">${typeLabel}</span>
    </div>`;
  }).join('');
}

function executePaletteItem(idx) {
  const item = _paletteResults[idx];
  if (!item) return;
  closePalette();
  item.action();
}

// Keyboard navigation
document.addEventListener('keydown', (e) => {
  // Open palette: Cmd+K or Ctrl+K
  const isMac = navigator.platform.includes('Mac');
  const mod = isMac ? e.metaKey : e.ctrlKey;
  if (mod && e.key === 'k') {
    e.preventDefault();
    if (_paletteOpen) closePalette();
    else openPalette();
    return;
  }

  if (!_paletteOpen) return;

  if (e.key === 'Escape') {
    e.preventDefault();
    closePalette();
    return;
  }

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    _paletteSelected = Math.min(_paletteSelected + 1, _paletteResults.length - 1);
    renderPaletteResults();
    scrollPaletteSelection();
    return;
  }

  if (e.key === 'ArrowUp') {
    e.preventDefault();
    _paletteSelected = Math.max(_paletteSelected - 1, 0);
    renderPaletteResults();
    scrollPaletteSelection();
    return;
  }

  if (e.key === 'Enter') {
    e.preventDefault();
    executePaletteItem(_paletteSelected);
    return;
  }
}, true);

function scrollPaletteSelection() {
  const sel = document.querySelector('.palette-selected');
  if (sel) sel.scrollIntoView({ block: 'nearest' });
}

// Click handling
document.addEventListener('click', (e) => {
  if (!_paletteOpen) return;

  const row = e.target.closest('[data-palette-idx]');
  if (row) {
    executePaletteItem(parseInt(row.dataset.paletteIdx, 10));
    return;
  }

  // Click outside the palette box closes it
  if (e.target.id === 'paletteOverlay') {
    closePalette();
  }
});

// Hover to highlight
document.addEventListener('mousemove', (e) => {
  if (!_paletteOpen) return;
  const row = e.target.closest('[data-palette-idx]');
  if (row) {
    const idx = parseInt(row.dataset.paletteIdx, 10);
    if (idx !== _paletteSelected) {
      _paletteSelected = idx;
      document.querySelectorAll('.palette-row').forEach((r, i) => {
        r.classList.toggle('palette-selected', i === idx);
      });
    }
  }
});
