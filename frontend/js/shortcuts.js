// ── Keyboard Shortcut Customization ──

const DEFAULT_SHORTCUTS = {
  'tab1': { key: '1', mod: true, label: 'Plugins tab' },
  'tab2': { key: '2', mod: true, label: 'Samples tab' },
  'tab3': { key: '3', mod: true, label: 'DAW Projects tab' },
  'tab4': { key: '4', mod: true, label: 'Presets tab' },
  'tab5': { key: '5', mod: true, label: 'Favorites tab' },
  'tab6': { key: '6', mod: true, label: 'Notes tab' },
  'tab7': { key: '7', mod: true, label: 'Tags tab' },
  'tab8': { key: '8', mod: true, label: 'Files tab' },
  'tab9': { key: '9', mod: true, label: 'History tab' },
  'tab10': { key: '0', mod: true, label: 'Visualizer tab' },
  'tab11': { key: 'F3', mod: false, label: 'Walkers tab' },
  'tab12': { key: 'F4', mod: false, label: 'Settings tab' },
  'search': { key: 'f', mod: true, label: 'Focus search' },
  'help': { key: '?', mod: false, label: 'Help overlay' },
  'playPause': { key: ' ', mod: false, label: 'Play / Pause' },
  'nextTrack': { key: 'ArrowRight', mod: true, label: 'Next track' },
  'prevTrack': { key: 'ArrowLeft', mod: true, label: 'Previous track' },
  'scanAll': { key: 's', mod: true, label: 'Scan all' },
  'stopAll': { key: '.', mod: true, label: 'Stop all scans' },
  'commandPalette': { key: 'k', mod: true, label: 'Command palette' },
  'toggleLoop': { key: 'l', mod: false, label: 'Toggle loop' },
  'toggleMute': { key: 'm', mod: false, label: 'Mute / Unmute' },
  'volumeUp': { key: 'ArrowUp', mod: true, label: 'Volume up' },
  'volumeDown': { key: 'ArrowDown', mod: true, label: 'Volume down' },
  'revealFile': { key: 'r', mod: false, label: 'Reveal selected in Finder' },
  'copyPath': { key: 'c', mod: false, label: 'Copy selected path' },
  'toggleFavorite': { key: 'f', mod: false, label: 'Toggle favorite' },
  'addNote': { key: 'n', mod: false, label: 'Add note to selected' },
  'deleteItem': { key: 'Backspace', mod: false, label: 'Delete selected' },
  'selectAll': { key: 'a', mod: true, label: 'Select all visible' },
  'escape': { key: 'Escape', mod: false, label: 'Close / clear / stop' },
  'exportTab': { key: 'e', mod: true, label: 'Export current tab' },
  'importTab': { key: 'i', mod: true, label: 'Import to current tab' },
  'toggleShuffle': { key: 's', mod: false, label: 'Toggle shuffle' },
  'findDuplicates': { key: 'd', mod: true, label: 'Find duplicates' },
  'depGraph': { key: 'g', mod: true, label: 'Dependency graph' },
  'resetAllScans': { key: 'Backspace', mod: true, label: 'Reset all scans' },
  'toggleTheme': { key: 't', mod: true, label: 'Toggle light/dark theme' },
  'openPrefs': { key: ',', mod: true, label: 'Settings' },
  'nextTab': { key: 'Tab', mod: false, label: 'Next tab (Tab)' },
  'prevTab': { key: 'Tab', mod: false, label: 'Previous tab (Shift+Tab)' },
  'findSimilar': { key: 'w', mod: false, label: 'Find similar samples' },
  'togglePlayerExpand': { key: 'e', mod: false, label: 'Expand / collapse player' },
  'toggleEq': { key: 'q', mod: false, label: 'Toggle EQ panel' },
  'toggleMono': { key: 'u', mod: false, label: 'Toggle mono playback' },
  'newSmartPlaylist': { key: 'p', mod: true, label: 'New smart playlist' },
  'deselectAll': { key: 'Escape', mod: true, label: 'Deselect all' },
  'toggleABLoop': { key: 'b', mod: false, label: 'Set / clear A-B loop' },
  'heatmapDash': { key: 'd', mod: false, label: 'Heatmap dashboard' },
  'togglePlayer': { key: 'p', mod: false, label: 'Show / hide player' },
  'toggleCrt': { key: 'F1', mod: false, label: 'Toggle CRT effects' },
  'toggleNeonGlow': { key: 'F2', mod: false, label: 'Toggle neon glow' },
  'clearPlayHistory': { key: 'h', mod: true, label: 'Clear play history' },
};

const TAB_MAP = ['plugins', 'samples', 'daw', 'presets', 'favorites', 'notes', 'tags', 'files', 'history', 'midi', 'visualizer', 'walkers', 'settings'];

function getShortcuts() {
  const saved = prefs.getObject('customShortcuts', null);
  if (!saved) return { ...DEFAULT_SHORTCUTS };
  // Merge with defaults for new shortcuts
  const merged = { ...DEFAULT_SHORTCUTS };
  for (const [id, val] of Object.entries(saved)) {
    if (merged[id]) {
      merged[id] = { ...merged[id], key: val.key, mod: val.mod };
    }
  }
  return merged;
}

function saveShortcuts(shortcuts) {
  prefs.setItem('customShortcuts', shortcuts);
}

function resetShortcuts() {
  prefs.removeItem('customShortcuts');
  renderShortcutSettings();
  showToast('Shortcuts reset to defaults');
}

function formatKey(shortcut) {
  const isMac = navigator.platform.includes('Mac');
  let parts = [];
  if (shortcut.mod) parts.push(isMac ? '\u2318' : 'Ctrl');
  let k = shortcut.key;
  if (k === ' ') k = 'Space';
  else if (k === 'ArrowLeft') k = '\u2190';
  else if (k === 'ArrowRight') k = '\u2192';
  else if (k === 'ArrowUp') k = '\u2191';
  else if (k === 'ArrowDown') k = '\u2193';
  else if (k === 'Escape') k = 'Esc';
  else k = k.toUpperCase();
  parts.push(k);
  return parts.join('+');
}

function renderShortcutSettings(filter) {
  const list = document.getElementById('shortcutsList');
  if (!list) return;
  const shortcuts = getShortcuts();
  const q = (filter || '').trim();
  let entries;
  if (!q) {
    entries = Object.entries(shortcuts).map(([id, sc]) => [id, sc, 0]);
  } else {
    entries = [];
    for (const [id, sc] of Object.entries(shortcuts)) {
      const score = searchScore(q, [sc.label, formatKey(sc)], 'fuzzy');
      if (score > 0) entries.push([id, sc, score]);
    }
    entries.sort((a, b) => b[2] - a[2]);
  }
  const hl = typeof highlightMatch === 'function' && q
    ? (text) => highlightMatch(text, q, 'fuzzy')
    : (text) => (typeof escapeHtml === 'function' ? escapeHtml(text) : text);
  list.innerHTML = entries.map(([id, sc]) =>
    `<div class="shortcut-row" data-sc-id="${id}">
      <span class="shortcut-name">${hl(sc.label)}</span>
      <span class="shortcut-key" data-shortcut-id="${id}" title="Click to rebind">${q ? hl(formatKey(sc)) : formatKey(sc)}</span>
    </div>`
  ).join('');
  if (!q && typeof initDragReorder === 'function') {
    initDragReorder(list, '.shortcut-row', 'shortcutOrder', {
      getKey: (el) => el.dataset.scId || '',
      // Drag from anywhere on the row (skip list handles buttons)
    });
  }
}

// Filter input — uses unified filter system
registerFilter('filterShortcuts', {
  inputId: 'shortcutsFilter',
  fetchFn() { renderShortcutSettings(this.lastSearch || ''); },
});

// Recording state
let _recordingId = null;

document.addEventListener('click', (e) => {
  const keyEl = e.target.closest('.shortcut-key');
  if (keyEl && keyEl.dataset.shortcutId) {
    // Start recording
    if (_recordingId) {
      // Cancel previous
      document.querySelectorAll('.shortcut-key.recording').forEach(el => el.classList.remove('recording'));
    }
    _recordingId = keyEl.dataset.shortcutId;
    keyEl.classList.add('recording');
    keyEl.textContent = 'Press key...';
    e.stopPropagation();
    return;
  }
  const resetBtn = e.target.closest('[data-action="resetShortcuts"]');
  if (resetBtn) {
    resetShortcuts();
  }
});

document.addEventListener('keydown', (e) => {
  if (_recordingId) {
    e.preventDefault();
    e.stopPropagation();
    const isMac = navigator.platform.includes('Mac');
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (e.key === 'Escape') {
      // Cancel recording
      _recordingId = null;
      renderShortcutSettings();
      return;
    }
    // Don't record bare modifier keys
    if (['Meta', 'Control', 'Shift', 'Alt'].includes(e.key)) return;

    const shortcuts = getShortcuts();
    shortcuts[_recordingId] = { ...shortcuts[_recordingId], key: e.key, mod };
    saveShortcuts(shortcuts);
    _recordingId = null;
    renderShortcutSettings();
    showToast('Shortcut updated');
    return;
  }

  // Don't handle shortcuts when typing in inputs
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;
  if (e.target.isContentEditable || e.target.closest('[contenteditable]')) return;
  if (e.target.closest('.ctx-menu')) return;

  const isMac = navigator.platform.includes('Mac');
  const mod = isMac ? e.metaKey : e.ctrlKey;
  const shortcuts = getShortcuts();

  // Tab bar: Tab = next, Shift+Tab = previous. (Plain Tab used to hit prevTab in the loop because
  // prevTab was { Tab, mod:false } while nextTab required mod:true — so Tab alone went backward.)
  if (e.key === 'Tab') {
    e.preventDefault();
    _cycleTab(e.shiftKey ? -1 : 1);
    return;
  }

  for (const [id, sc] of Object.entries(shortcuts)) {
    if (sc.key === e.key && sc.mod === mod) {
      e.preventDefault();
      executeShortcut(id);
      return;
    }
  }
}, true); // capture phase to override other handlers

function executeShortcut(id) {
  if (id.startsWith('tab') && id.length >= 4 && id.length <= 5) {
    const num = parseInt(id.slice(3));
    const idx = num - 1;
    if (idx >= 0 && idx < TAB_MAP.length) switchTab(TAB_MAP[idx]);
  } else if (id === 'search') {
    const activeTab = document.querySelector('.tab-content.active');
    const input = activeTab?.querySelector('input[type="text"]');
    if (input) { input.focus(); input.select(); }
  } else if (id === 'help') {
    toggleHelpOverlay();
  } else if (id === 'playPause') {
    toggleAudioPlayback();
  } else if (id === 'nextTrack') {
    nextTrack();
  } else if (id === 'prevTrack') {
    prevTrack();
  } else if (id === 'scanAll') {
    if (typeof scanAll === 'function') scanAll();
  } else if (id === 'stopAll') {
    if (typeof stopAll === 'function') stopAll();
  } else if (id === 'commandPalette') {
    if (typeof toggleCommandPalette === 'function') toggleCommandPalette();
  } else if (id === 'toggleLoop') {
    if (typeof toggleAudioLoop === 'function') toggleAudioLoop();
  } else if (id === 'toggleMute') {
    if (typeof toggleMute === 'function') toggleMute();
  } else if (id === 'volumeUp') {
    _adjustVolume(5);
  } else if (id === 'volumeDown') {
    _adjustVolume(-5);
  } else if (id === 'revealFile') {
    _actionOnSelected('reveal');
  } else if (id === 'copyPath') {
    _actionOnSelected('copy');
  } else if (id === 'toggleFavorite') {
    _actionOnSelected('favorite');
  } else if (id === 'addNote') {
    _actionOnSelected('note');
  } else if (id === 'deleteItem') {
    _actionOnSelected('delete');
  } else if (id === 'selectAll') {
    if (typeof selectAllVisible === 'function') selectAllVisible();
  } else if (id === 'escape') {
    _handleEscape();
  } else if (id === 'exportTab') {
    _exportCurrentTab();
  } else if (id === 'importTab') {
    _importCurrentTab();
  } else if (id === 'toggleShuffle') {
    if (typeof toggleShuffle === 'function') toggleShuffle();
  } else if (id === 'findDuplicates') {
    if (typeof showDuplicateReport === 'function') showDuplicateReport();
  } else if (id === 'depGraph') {
    if (typeof showDepGraph === 'function') showDepGraph();
  } else if (id === 'resetAllScans') {
    if (typeof resetAllScans === 'function') resetAllScans();
  } else if (id === 'toggleTheme') {
    if (typeof settingToggleTheme === 'function') settingToggleTheme();
  } else if (id === 'openPrefs') {
    switchTab('settings');
  } else if (id === 'nextTab') {
    _cycleTab(1);
  } else if (id === 'prevTab') {
    _cycleTab(-1);
  } else if (id === 'findSimilar') {
    const path = _getSelectedPath();
    if (path && typeof findSimilarSamples === 'function') findSimilarSamples(path);
  } else if (id === 'togglePlayerExpand') {
    if (typeof togglePlayerExpanded === 'function') togglePlayerExpanded();
  } else if (id === 'toggleEq') {
    if (typeof toggleEqSection === 'function') toggleEqSection();
  } else if (id === 'toggleMono') {
    if (typeof toggleMono === 'function') toggleMono();
  } else if (id === 'newSmartPlaylist') {
    if (typeof showSmartPlaylistEditor === 'function') showSmartPlaylistEditor(null);
  } else if (id === 'deselectAll') {
    if (typeof deselectAll === 'function') deselectAll();
  } else if (id === 'toggleABLoop') {
    // Cycle: no loop → set A → set B → clear
    if (typeof _abLoop !== 'undefined' && _abLoop) {
      if (typeof clearAbLoop === 'function') clearAbLoop();
    } else {
      if (typeof setAbLoopStart === 'function') setAbLoopStart();
    }
  } else if (id === 'heatmapDash') {
    if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard();
  } else if (id === 'togglePlayer') {
    const np = document.getElementById('audioNowPlaying');
    if (np && np.classList.contains('active')) {
      if (typeof hidePlayer === 'function') hidePlayer();
    } else {
      if (typeof showPlayer === 'function') showPlayer();
    }
  } else if (id === 'toggleCrt') {
    if (typeof settingToggleCrt === 'function') settingToggleCrt();
  } else if (id === 'toggleNeonGlow') {
    if (typeof settingToggleNeonGlow === 'function') settingToggleNeonGlow();
  } else if (id === 'clearPlayHistory') {
    if (typeof clearRecentlyPlayed === 'function') clearRecentlyPlayed();
  }
}

function _exportCurrentTab() {
  const active = document.querySelector('.tab-content.active')?.id;
  if (active === 'tabPlugins' && typeof exportPlugins === 'function') exportPlugins();
  else if (active === 'tabSamples' && typeof exportAudio === 'function') exportAudio();
  else if (active === 'tabDaw' && typeof exportDaw === 'function') exportDaw();
  else if (active === 'tabPresets' && typeof exportPresets === 'function') exportPresets();
  else if (active === 'tabFavorites' && typeof exportFavorites === 'function') exportFavorites();
  else if (active === 'tabNotes' && typeof exportNotes === 'function') exportNotes();
  else if (active === 'tabMidi' && typeof exportMidi === 'function') exportMidi();
}

function _importCurrentTab() {
  const active = document.querySelector('.tab-content.active')?.id;
  if (active === 'tabPlugins' && typeof importPlugins === 'function') importPlugins();
  else if (active === 'tabSamples' && typeof importAudio === 'function') importAudio();
  else if (active === 'tabDaw' && typeof importDaw === 'function') importDaw();
  else if (active === 'tabPresets' && typeof importPresets === 'function') importPresets();
  else if (active === 'tabFavorites' && typeof importFavorites === 'function') importFavorites();
  else if (active === 'tabNotes' && typeof importNotes === 'function') importNotes();
}

function _cycleTab(dir) {
  const tabs = [...document.querySelectorAll('.tab-btn')];
  const activeIdx = tabs.findIndex(t => t.classList.contains('active'));
  const next = (activeIdx + dir + tabs.length) % tabs.length;
  const tab = tabs[next]?.dataset?.tab;
  if (tab) switchTab(tab);
}

function _adjustVolume(delta) {
  const slider = document.getElementById('npVolume');
  if (!slider) return;
  const val = Math.max(0, Math.min(100, parseInt(slider.value) + delta));
  slider.value = val;
  if (typeof setAudioVolume === 'function') setAudioVolume(val);
}

function _actionOnSelected(action) {
  const items = getNavigableItems();
  if (_navIndex < 0 || _navIndex >= items.length) return;
  const item = items[_navIndex];
  const path = item.getAttribute('data-audio-path') || item.dataset.dawPath || item.dataset.presetPath || item.dataset.path || '';
  const name = item.querySelector('.col-name,.plugin-name')?.textContent?.trim() || '';
  if (!path) return;

  if (action === 'reveal') {
    if (typeof openFolder === 'function') openFolder(path);
    else if (typeof openAudioFolder === 'function') openAudioFolder(path);
  } else if (action === 'copy') {
    if (typeof copyToClipboard === 'function') copyToClipboard(path);
  } else if (action === 'favorite') {
    if (typeof isFavorite === 'function' && typeof addFavorite === 'function' && typeof removeFavorite === 'function') {
      isFavorite(path) ? removeFavorite(path) : addFavorite('item', path, name);
    }
  } else if (action === 'note') {
    if (typeof showNoteEditor === 'function') showNoteEditor(path, name);
  } else if (action === 'delete') {
    if (typeof deleteFile === 'function') {
      if (confirm(`Delete "${name || path}"?`)) deleteFile(path);
    }
  }
}

function _handleEscape() {
  // Close modals first
  const modal = document.querySelector('.modal-overlay');
  if (modal) { modal.remove(); return; }
  // Close context menu
  const ctx = document.querySelector('.ctx-menu.visible');
  if (ctx) { ctx.classList.remove('visible'); return; }
  // Close command palette
  const palette = document.getElementById('commandPalette');
  if (palette) { palette.remove(); return; }
  // Clear search in active tab
  const activeTab = document.querySelector('.tab-content.active');
  const input = activeTab?.querySelector('input[type="text"]');
  if (input && input.value) { input.value = ''; input.dispatchEvent(new Event('input')); return; }
  // Stop current operation
  if (typeof stopAll === 'function') stopAll();
}
