// ── Context Menu ──
const ctxMenu = document.getElementById('ctxMenu');

function showContextMenu(e, items) {
  e.preventDefault();
  // Store callbacks and render
  ctxMenu._actions = {};
  ctxMenu.innerHTML = items.map((item, i) => {
    if (item === '---') return '<div class="ctx-menu-sep"></div>';
    if (item.action) ctxMenu._actions[i] = item.action;
    const cls = item.disabled ? ' ctx-disabled' : '';
    return `<div class="ctx-menu-item${cls}" data-ctx-idx="${i}">
      <span class="ctx-icon">${item.icon || ''}</span>${item.label}
    </div>`;
  }).join('');

  ctxMenu.classList.add('visible');

  // Position — keep within viewport
  const rect = ctxMenu.getBoundingClientRect();
  let x = e.clientX, y = e.clientY;
  if (x + rect.width > window.innerWidth) x = window.innerWidth - rect.width - 4;
  if (y + rect.height > window.innerHeight) y = window.innerHeight - rect.height - 4;
  ctxMenu.style.left = x + 'px';
  ctxMenu.style.top = y + 'px';
}

function hideContextMenu() {
  ctxMenu.classList.remove('visible');
  ctxMenu._actions = {};
}

// Click on menu item
ctxMenu.addEventListener('click', (e) => {
  const item = e.target.closest('.ctx-menu-item');
  if (!item || item.classList.contains('ctx-disabled')) return;
  const idx = item.dataset.ctxIdx;
  const action = ctxMenu._actions[idx];
  hideContextMenu();
  if (action) action();
});

// Dismiss on click outside or Escape
document.addEventListener('click', (e) => {
  if (!ctxMenu.contains(e.target)) hideContextMenu();
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') hideContextMenu();
});

// Copy helper
function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    showToast('Copied to clipboard');
  }).catch(() => {});
}

// ── Right-click handlers ──
document.addEventListener('contextmenu', (e) => {

  // ── Plugin cards ──
  const pluginCard = e.target.closest('#pluginList .plugin-card');
  if (pluginCard) {
    const name = pluginCard.querySelector('h3')?.textContent || '';
    const path = pluginCard.dataset.path || '';
    const kvrBtn = pluginCard.querySelector('[data-action="openKvr"]');
    const mfgBtn = pluginCard.querySelector('[data-action="openUpdate"][title]');
    const folderBtn = pluginCard.querySelector('[data-action="openFolder"]');
    const archBadges = [...pluginCard.querySelectorAll('.arch-badge')].map(b => b.textContent).join(', ');
    const items = [
      { icon: '&#128269;', label: 'Open on KVR', action: () => kvrBtn && openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name) },
    ];
    if (mfgBtn && !mfgBtn.disabled) {
      items.push({ icon: '&#127760;', label: 'Open Manufacturer Site', action: () => openUpdate(mfgBtn.dataset.url) });
    }
    items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => folderBtn && openFolder(folderBtn.dataset.path) });
    items.push('---');
    items.push({ icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) });
    items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) });
    if (archBadges) {
      items.push({ icon: '&#128203;', label: 'Copy Architecture', action: () => copyToClipboard(archBadges) });
    }
    items.push('---');
    const pluginFav = isFavorite(path);
    items.push({ icon: pluginFav ? '&#9734;' : '&#9733;', label: pluginFav ? 'Remove from Favorites' : 'Add to Favorites',
      action: () => pluginFav ? removeFavorite(path) : addFavorite('plugin', path, name, { format: pluginCard.querySelector('.plugin-type')?.textContent }) });
    items.push({ icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) });
    showContextMenu(e, items);
    return;
  }

  // ── Audio sample rows ──
  const audioRow = e.target.closest('#audioTableBody tr[data-audio-path]');
  if (audioRow) {
    const path = audioRow.getAttribute('data-audio-path');
    const name = audioRow.querySelector('.col-name')?.textContent || '';
    const isPlaying = audioPlayerPath === path && !audioPlayer.paused;
    const items = [
      { icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? 'Pause' : 'Play', action: () => previewAudio(path) },
      { icon: '&#8634;', label: 'Loop', action: () => { toggleRowLoop(path, new MouseEvent('click')); } },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openAudioFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('sample', path, name, { format: audioRow.querySelector('.format-badge')?.textContent }) }; })()],
      { icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── DAW project rows ──
  const dawRow = e.target.closest('#dawTableBody tr[data-daw-path]');
  if (dawRow) {
    const path = dawRow.dataset.dawPath;
    const name = dawRow.querySelector('.col-name')?.textContent || '';
    const dawName = dawRow.querySelector('.format-badge')?.textContent || 'DAW';
    const items = [
      { icon: '&#9654;', label: `Open in ${dawName}`, action: () => { showToast(`Opening "${name}" in ${dawName}...`); window.vstUpdater.openDawProject(path).catch(err => showToast(`${dawName} not installed — ${err}`, 4000, 'error')); } },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openDawFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('daw', path, name, { format: dawRow.querySelector('.format-badge:last-of-type')?.textContent, daw: dawName }) }; })()],
      { icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Preset rows ──
  const presetRow = e.target.closest('#presetTableBody tr[data-preset-path]');
  if (presetRow) {
    const path = presetRow.dataset.presetPath;
    const name = presetRow.querySelector('td')?.textContent || '';
    const items = [
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openPresetFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('preset', path, name, { format: presetRow.querySelector('.format-badge')?.textContent }) }; })()],
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Table column headers ──
  const th = e.target.closest('th[data-action]');
  if (th) {
    const action = th.dataset.action;
    const key = th.dataset.key;
    const items = [
      { icon: '&#9650;', label: 'Sort Ascending', action: () => {
        if (action === 'sortAudio') { audioSortAsc = true; audioSortKey = key; sortAudio(key); }
        else if (action === 'sortDaw') { dawSortAsc = true; dawSortKey = key; sortDaw(key); }
        else if (action === 'sortPreset') { presetSortAsc = true; presetSortKey = key; sortPreset(key); }
      }},
      { icon: '&#9660;', label: 'Sort Descending', action: () => {
        if (action === 'sortAudio') { audioSortAsc = false; audioSortKey = key; sortAudio(key); }
        else if (action === 'sortDaw') { dawSortAsc = false; dawSortKey = key; sortDaw(key); }
        else if (action === 'sortPreset') { presetSortAsc = false; presetSortKey = key; sortPreset(key); }
      }},
      '---',
      { icon: '&#8596;', label: 'Reset Column Widths', action: () => settingResetColumns() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Search boxes ──
  const searchBox = e.target.closest('.search-box');
  if (searchBox) {
    const input = searchBox.querySelector('input');
    const regexBtn = searchBox.querySelector('.btn-regex');
    if (input) {
      const hasText = input.value.length > 0;
      const isRegex = regexBtn?.classList.contains('active');
      const items = [
        { icon: '&#10005;', label: 'Clear Search', action: () => { input.value = ''; input.dispatchEvent(new Event('input', { bubbles: true })); }, disabled: !hasText },
        { icon: '&#128203;', label: 'Paste & Search', action: async () => {
          try {
            const text = await navigator.clipboard.readText();
            input.value = text;
            input.dispatchEvent(new Event('input', { bubbles: true }));
          } catch {}
        }},
        '---',
        { icon: '.*', label: isRegex ? 'Switch to Fuzzy' : 'Switch to Regex', action: () => regexBtn && toggleRegex(regexBtn) },
      ];
      showContextMenu(e, items);
      return;
    }
  }

  // ── Filter dropdowns ──
  const filterSelect = e.target.closest('.filter-select');
  if (filterSelect) {
    const items = [
      { icon: '&#8635;', label: 'Reset to All', action: () => {
        filterSelect.value = 'all';
        filterSelect.dispatchEvent(new Event('change', { bubbles: true }));
      }},
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Toolbar areas ──
  const toolbar = e.target.closest('.audio-toolbar');
  if (toolbar) {
    const tab = toolbar.closest('.tab-content');
    const tabId = tab?.id;
    const items = [];
    if (tabId === 'tabPlugins') {
      items.push({ icon: '&#8635;', label: 'Scan Plugins', action: () => scanPlugins() });
      items.push({ icon: '&#9889;', label: 'Check Updates', action: () => checkUpdates(), disabled: allPlugins.length === 0 });
      items.push('---');
      items.push({ icon: '&#8615;', label: 'Export Plugins', action: () => exportPlugins(), disabled: allPlugins.length === 0 });
      items.push({ icon: '&#8613;', label: 'Import Plugins', action: () => importPlugins() });
    } else if (tabId === 'tabSamples') {
      items.push({ icon: '&#127925;', label: 'Scan Samples', action: () => scanAudioSamples() });
      items.push('---');
      items.push({ icon: '&#8615;', label: 'Export Samples', action: () => exportAudio(), disabled: allAudioSamples.length === 0 });
      items.push({ icon: '&#8613;', label: 'Import Samples', action: () => importAudio() });
    } else if (tabId === 'tabDaw') {
      items.push({ icon: '&#127911;', label: 'Scan DAW Projects', action: () => scanDawProjects() });
      items.push('---');
      items.push({ icon: '&#8615;', label: 'Export Projects', action: () => exportDaw(), disabled: allDawProjects.length === 0 });
      items.push({ icon: '&#8613;', label: 'Import Projects', action: () => importDaw() });
    } else if (tabId === 'tabPresets') {
      items.push({ icon: '&#127924;', label: 'Scan Presets', action: () => scanPresets() });
      items.push('---');
      items.push({ icon: '&#8615;', label: 'Export Presets', action: () => exportPresets(), disabled: allPresets.length === 0 });
      items.push({ icon: '&#8613;', label: 'Import Presets', action: () => importPresets() });
    }
    if (items.length) {
      showContextMenu(e, items);
      return;
    }
  }

  // ── Stats bar ──
  const statsBar = e.target.closest('.stats-bar');
  if (statsBar) {
    const statsText = [...statsBar.querySelectorAll('.stat')].map(s => s.textContent.trim()).join(' | ');
    const items = [
      { icon: '&#128203;', label: 'Copy Stats', action: () => copyToClipboard(statsText) },
      '---',
      { icon: '&#9889;', label: 'Scan All', action: () => scanAll() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Header area / logo ──
  const header = e.target.closest('.header');
  if (header) {
    const headerInfo = e.target.closest('.header-info');
    if (headerInfo) {
      const statsText = [...headerInfo.querySelectorAll('.header-info-item')].map(s => s.textContent.trim()).join(' | ');
      const items = [
        { icon: '&#128203;', label: 'Copy Process Stats', action: () => copyToClipboard(statsText) },
      ];
      showContextMenu(e, items);
      return;
    }
    const items = [
      { icon: '&#127760;', label: 'Open GitHub Repository', action: () => openUpdate('https://github.com/MenkeTechnologies/universal-plugin-update-manager') },
      { icon: '&#9881;', label: 'Settings', action: () => switchTab('settings') },
      '---',
      { icon: '&#9889;', label: 'Scan All', action: () => scanAll() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── History entries ──
  const historyRow = e.target.closest('.history-item');
  if (historyRow) {
    const id = historyRow.dataset.id;
    const type = historyRow.dataset.type;
    if (id) {
      const items = [
        { icon: '&#128269;', label: 'View Details', action: () => selectScan(id, type) },
        { icon: '&#128465;', label: 'Delete Entry', action: () => {
          if (type === 'audio') deleteAudioScanEntry(id);
          else if (type === 'daw') deleteDawScanEntry(id);
          else if (type === 'preset') deletePresetScanEntry(id);
          else deleteScanEntry(id);
        }},
      ];
      showContextMenu(e, items);
      return;
    }
  }

  // ── History tab (empty area) ──
  const historyTab = e.target.closest('#tabHistory');
  if (historyTab) {
    const items = [
      { icon: '&#128465;', label: 'Clear All History', action: () => settingClearAllHistory() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Floating player ──
  const player = e.target.closest('#audioNowPlaying');
  if (player && player.classList.contains('active')) {
    const isPlaying = audioPlayerPath && !audioPlayer.paused;
    const isExpanded = player.classList.contains('expanded');
    const items = [];
    if (audioPlayerPath) {
      items.push({ icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? 'Pause' : 'Play', action: () => toggleAudioPlayback() });
      items.push({ icon: '&#8634;', label: audioLooping ? 'Disable Loop' : 'Enable Loop', action: () => toggleAudioLoop() });
      items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => openAudioFolder(audioPlayerPath) });
      items.push('---');
      items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(audioPlayerPath) });
      items.push('---');
    }
    items.push({ icon: isExpanded ? '&#9660;' : '&#9650;', label: isExpanded ? 'Collapse Player' : 'Expand Player', action: () => togglePlayerExpanded() });
    items.push({ icon: '&#10005;', label: 'Close Player', action: () => stopAudioPlayback() });
    showContextMenu(e, items);
    return;
  }

  // ── Tab buttons ──
  const tabBtn = e.target.closest('.tab-btn');
  if (tabBtn) {
    const tab = tabBtn.dataset.tab;
    const items = [
      { icon: '&#8635;', label: 'Switch to Tab', action: () => switchTab(tab) },
      '---',
      { icon: '&#8644;', label: 'Reset Tab Order', action: () => settingResetTabOrder() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Tab nav bar (empty area) ──
  const tabNav = e.target.closest('.tab-nav');
  if (tabNav) {
    const items = [
      { icon: '&#8644;', label: 'Reset Tab Order', action: () => settingResetTabOrder() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Settings rows ──
  const settingsRow = e.target.closest('.settings-row');
  if (settingsRow) {
    const toggle = settingsRow.querySelector('.settings-toggle');
    const textarea = settingsRow.querySelector('.settings-textarea');
    const items = [];
    if (toggle) {
      const isOn = toggle.classList.contains('active');
      items.push({ icon: isOn ? '&#9711;' : '&#9679;', label: isOn ? 'Turn Off' : 'Turn On', action: () => toggle.click() });
    }
    if (textarea) {
      items.push({ icon: '&#10005;', label: 'Clear', action: () => { textarea.value = ''; } });
      items.push({ icon: '&#128203;', label: 'Copy', action: () => copyToClipboard(textarea.value) });
    }
    if (items.length === 0) return; // no special actions
    showContextMenu(e, items);
    return;
  }

  // ── Settings container (empty area) ──
  const settingsContainer = e.target.closest('.settings-container');
  if (settingsContainer) {
    const items = [
      { icon: '&#8596;', label: 'Reset Column Widths', action: () => settingResetColumns() },
      { icon: '&#8644;', label: 'Reset Tab Order', action: () => settingResetTabOrder() },
      { icon: '&#128465;', label: 'Clear All History', action: () => settingClearAllHistory() },
      '---',
      { icon: '&#128206;', label: 'Open Prefs File', action: () => openPrefsFile() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Directory breakdown rows ──
  const dirsRow = e.target.closest('#dirsList tr');
  if (dirsRow) {
    const dirPath = dirsRow.querySelector('td')?.textContent?.trim() || '';
    if (dirPath) {
      const items = [
        { icon: '&#128193;', label: 'Open Directory', action: () => openFolder(dirPath) },
        { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(dirPath) },
      ];
      showContextMenu(e, items);
      return;
    }
  }

  // ── Audio/DAW/Preset stats bars ──
  const audioStats = e.target.closest('.audio-stats');
  if (audioStats) {
    const statsText = audioStats.textContent.trim().replace(/\s+/g, ' ');
    const items = [
      { icon: '&#128203;', label: 'Copy Stats', action: () => copyToClipboard(statsText) },
    ];
    showContextMenu(e, items);
    return;
  }
});
