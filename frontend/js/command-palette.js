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
    { type: 'tab', name: 'Visualizer', icon: '&#127911;', action: () => switchTab('visualizer') },
    { type: 'tab', name: 'Walkers', icon: '&#128270;', action: () => switchTab('walkers') },
    { type: 'tab', name: 'MIDI', icon: '&#127924;', action: () => switchTab('midi') },
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
  if (typeof showPlayer === 'function') {
    const np = document.getElementById('audioNowPlaying');
    const visible = np && np.classList.contains('active');
    items.push({ type: 'action', name: visible ? 'Hide Audio Player' : 'Show Audio Player', icon: '&#9835;', action: () => { visible ? hidePlayer() : showPlayer(); } });
  }
  if (typeof showHeatmapDashboard === 'function') {
    items.push({ type: 'action', name: 'Heatmap Dashboard', icon: '&#128202;', action: () => showHeatmapDashboard() });
  }
  if (typeof showSmartPlaylistEditor === 'function') {
    items.push({ type: 'action', name: 'New Smart Playlist', icon: '&#127926;', action: () => showSmartPlaylistEditor(null) });
  }
  if (typeof exportSettingsPdf === 'function') {
    items.push({ type: 'action', name: 'Export Settings & Keybindings', icon: '&#128196;', action: () => exportSettingsPdf() });
  }
  if (typeof exportLogPdf === 'function') {
    items.push({ type: 'action', name: 'Export App Log', icon: '&#128196;', action: () => exportLogPdf() });
  }
  if (typeof exportMidi === 'function') {
    items.push({ type: 'action', name: 'Export MIDI Files', icon: '&#127924;', action: () => exportMidi() });
  }
  if (typeof exportXref === 'function') {
    items.push({ type: 'action', name: 'Export Plugin Cross-Reference', icon: '&#9889;', action: () => exportXref() });
  }
  if (typeof exportSmartPlaylists === 'function') {
    items.push({ type: 'action', name: 'Export Smart Playlists', icon: '&#127926;', action: () => exportSmartPlaylists() });
  }
  items.push({ type: 'action', name: 'Clear All Caches', icon: '&#128465;', action: () => {
    window.vstUpdater.dbClearCaches().then(() => {
      if (typeof _bpmCache !== 'undefined') { _bpmCache = {}; _keyCache = {}; _lufsCache = {}; }
      if (typeof _waveformCache !== 'undefined') { _waveformCache = {}; _spectrogramCache = {}; }
      showToast('All caches cleared');
    }).catch(e => showToast('Failed: ' + e, 4000, 'error'));
  }});
  if (typeof settingToggleTheme === 'function') {
    items.push({ type: 'action', name: 'Toggle Dark/Light Theme', icon: '&#127912;', action: () => settingToggleTheme() });
  }
  items.push({ type: 'action', name: 'Scan All', icon: '&#9889;', action: () => typeof scanAll === 'function' && scanAll() });
  items.push({ type: 'action', name: 'Stop All Scans', icon: '&#9632;', action: () => typeof stopAll === 'function' && stopAll() });
  items.push({ type: 'action', name: 'Export Current Tab', icon: '&#8615;', action: () => typeof _exportCurrentTab === 'function' && _exportCurrentTab() });
  items.push({ type: 'action', name: 'Import to Current Tab', icon: '&#8613;', action: () => typeof _importCurrentTab === 'function' && _importCurrentTab() });
  items.push({ type: 'action', name: 'Help / Keyboard Shortcuts', icon: '&#10068;', action: () => typeof toggleHelpOverlay === 'function' && toggleHelpOverlay() });
  items.push({ type: 'action', name: 'Open Log File', icon: '&#128196;', action: () => window.vstUpdater.getPrefsPath().then(p => { const lp = p.replace(/preferences\.toml$/, 'app.log'); window.vstUpdater.openWithApp(lp, 'TextEdit').catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }); }) });
  items.push({ type: 'action', name: 'Open Preferences File', icon: '&#128196;', action: () => typeof window.vstUpdater.openPrefsFile === 'function' && window.vstUpdater.openPrefsFile() });
  items.push({ type: 'action', name: 'Focus Search', icon: '&#128269;', action: () => { const tab = document.querySelector('.tab-content.active'); const input = tab?.querySelector('input[type="text"]'); if (input) { input.focus(); input.select(); } } });

  // Player controls
  if (typeof toggleAudioPlayback === 'function') {
    items.push({ type: 'action', name: 'Play / Pause', icon: '&#9654;', action: () => toggleAudioPlayback() });
  }
  if (typeof nextTrack === 'function') {
    items.push({ type: 'action', name: 'Next Track', icon: '&#9193;', action: () => nextTrack() });
  }
  if (typeof prevTrack === 'function') {
    items.push({ type: 'action', name: 'Previous Track', icon: '&#9194;', action: () => prevTrack() });
  }
  if (typeof toggleAudioLoop === 'function') {
    items.push({ type: 'action', name: 'Toggle Loop', icon: '&#128257;', action: () => toggleAudioLoop() });
  }
  if (typeof toggleShuffle === 'function') {
    items.push({ type: 'action', name: 'Toggle Shuffle', icon: '&#128256;', action: () => toggleShuffle() });
  }
  if (typeof toggleMute === 'function') {
    items.push({ type: 'action', name: 'Toggle Mute', icon: '&#128263;', action: () => toggleMute() });
  }
  if (typeof toggleMono === 'function') {
    items.push({ type: 'action', name: 'Toggle Mono', icon: '&#127897;', action: () => toggleMono() });
  }
  if (typeof toggleEqSection === 'function') {
    items.push({ type: 'action', name: 'Toggle EQ Panel', icon: '&#127900;', action: () => toggleEqSection() });
  }
  if (typeof togglePlayerExpanded === 'function') {
    items.push({ type: 'action', name: 'Expand / Collapse Player', icon: '&#9744;', action: () => togglePlayerExpanded() });
  }
  if (typeof setAbLoopStart === 'function') {
    items.push({ type: 'action', name: 'Toggle A-B Loop', icon: '&#128260;', action: () => {
      if (typeof _abLoop !== 'undefined' && _abLoop) { if (typeof clearAbLoop === 'function') clearAbLoop(); }
      else { setAbLoopStart(); }
    }});
  }
  if (typeof clearRecentlyPlayed === 'function') {
    items.push({ type: 'action', name: 'Clear Play History', icon: '&#128465;', action: () => clearRecentlyPlayed() });
  }

  // Selection
  if (typeof selectAllVisible === 'function') {
    items.push({ type: 'action', name: 'Select All Visible', icon: '&#9745;', action: () => selectAllVisible() });
  }
  if (typeof deselectAll === 'function') {
    items.push({ type: 'action', name: 'Deselect All', icon: '&#9744;', action: () => deselectAll() });
  }

  // Effects toggles
  if (typeof settingToggleCrt === 'function') {
    items.push({ type: 'action', name: 'Toggle CRT Effects', icon: '&#128250;', action: () => settingToggleCrt() });
  }
  if (typeof settingToggleNeonGlow === 'function') {
    items.push({ type: 'action', name: 'Toggle Neon Glow', icon: '&#10024;', action: () => settingToggleNeonGlow() });
  }

  // Data items (plugins, samples, DAW, presets) are searched lazily
  // in filterPaletteResults to avoid blocking UI on palette open.
  // See _searchDataItems() below.

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
    return items.filter(i => i.type === 'tab' || i.type === 'action');
  }
  const scored = [];
  for (const item of items) {
    const fields = item.fields || [item.name];
    const score = searchScore(query, fields, 'fuzzy');
    if (score > 0) scored.push({ item, score });
  }
  // Lazy search data items only when query is 2+ chars (avoids blocking on single char)
  if (query.length >= 2) {
    const dataSearch = (arr, type, icon, mkItem) => {
      if (!arr) return;
      const limit = 20; let count = 0;
      for (const item of arr) {
        if (count >= limit) break;
        const built = mkItem(item);
        const score = searchScore(query, built.fields, 'fuzzy');
        if (score > 0) { scored.push({ item: { ...built, type, icon }, score }); count++; }
      }
    };
    if (typeof allPlugins !== 'undefined') dataSearch(allPlugins, 'plugin', '&#9889;', p => ({ name: p.name, detail: p.type + (p.manufacturer ? ' · ' + p.manufacturer : ''), fields: [p.name, p.type, p.manufacturer || ''], action: () => { switchTab('plugins'); setTimeout(() => { const el = document.getElementById('pluginSearchInput'); if (el) { el.value = p.name; filterPlugins(); } }, 100); } }));
    if (typeof allDawProjects !== 'undefined') dataSearch(allDawProjects, 'daw', '&#127911;', d => ({ name: d.name, detail: d.daw + ' · ' + d.sizeFormatted, fields: [d.name, d.daw, d.format], action: () => { switchTab('daw'); setTimeout(() => { const el = document.getElementById('dawSearchInput'); if (el) { el.value = d.name; filterDawProjects(); } }, 100); } }));
    if (typeof allPresets !== 'undefined') dataSearch(allPresets.slice(0, 5000), 'preset', '&#127924;', p => ({ name: p.name, detail: p.format, fields: [p.name, p.format], action: () => { switchTab('presets'); setTimeout(() => { const el = document.getElementById('presetSearchInput'); if (el) { el.value = p.name; filterPresets(); } }, 100); } }));
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

  let _palTimer;
  input.addEventListener('input', () => {
    _paletteQuery = input.value;
    _paletteSelected = 0;
    clearTimeout(_palTimer);
    _palTimer = setTimeout(renderPaletteResults, 150);
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
