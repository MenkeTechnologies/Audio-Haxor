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

// Open file with specific app
function openWithApp(filePath, appName) {
  window.vstUpdater.openWithApp(filePath, appName).then(() => {
    showToast(`Opening in ${appName}...`);
  }).catch(err => {
    showToast(`${appName} not available — ${err}`, 4000, 'error');
  });
}

// Copy helper
function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    showToast('Copied to clipboard');
  }).catch(() => {});
}

// ── Right-click handlers ──
document.addEventListener('contextmenu', (e) => {
  // Always suppress default browser menu on app content
  if (e.target.closest('.app, .audio-now-playing, .header, .stats-bar, .tab-nav')) {
    e.preventDefault();
  }

  try {

  // ── Audio player song rows (recently played / search results) ──
  const npItem = e.target.closest('.np-history-item');
  if (npItem) {
    const path = npItem.dataset.path || '';
    const name = npItem.querySelector('.np-h-name')?.textContent?.trim() || '';
    const isPlaying = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === path && typeof audioPlayer !== 'undefined' && !audioPlayer.paused;
    const items = [
      { icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? 'Pause' : 'Play', action: () => typeof previewAudio === 'function' && previewAudio(path) },
      { icon: '&#8634;', label: 'Loop', action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click')) },
      '---',
      { icon: '&#127926;', label: 'Open in Music', action: () => typeof openWithApp === 'function' && openWithApp(path, 'Music') },
      { icon: '&#127911;', label: 'Open in QuickTime', action: () => typeof openWithApp === 'function' && openWithApp(path, 'QuickTime Player') },
      { icon: '&#127908;', label: 'Open in Audacity', action: () => typeof openWithApp === 'function' && openWithApp(path, 'Audacity') },
      '---',
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) },
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); typeof loadDirectory === 'function' && loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      { icon: '&#127925;', label: 'Show in Samples Tab', action: () => {
        switchTab('samples');
        setTimeout(() => {
          const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
          if (row) {
            row.scrollIntoView({ behavior: 'smooth', block: 'center' });
            row.classList.add('row-playing');
            setTimeout(() => row.classList.remove('row-playing'), 2000);
          } else {
            // Try searching for it
            const input = document.getElementById('audioSearchInput');
            if (input) { input.value = name; input.dispatchEvent(new Event('input')); }
          }
        }, 200);
      }},
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
    ];
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name) });
    }
    if (typeof showNoteEditor === 'function') {
      items.push({ icon: '&#128221;', label: 'Add Note / Tags', action: () => showNoteEditor(path, name) });
    }
    items.push(...quickTagItems(path, name));
    items.push('---');
    items.push({ icon: '&#128270;', label: 'Find Similar Samples', action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
    showContextMenu(e, items);
    return;
  }

  // ── Similarity result rows ──
  const simRow = e.target.closest('[data-similar-path]');
  if (simRow) {
    const path = simRow.dataset.similarPath || '';
    const name = path.split('/').pop().replace(/\.[^.]+$/, '');
    const items = [
      { icon: '&#9654;', label: 'Play', action: () => typeof previewAudio === 'function' && previewAudio(path) },
      { icon: '&#8634;', label: 'Loop', action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click')) },
      '---',
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) },
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); typeof loadDirectory === 'function' && loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      '---',
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      { icon: '&#128270;', label: 'Find Similar to This', action: () => { typeof closeSimilarPanel === 'function' && closeSimilarPanel(); typeof findSimilarSamples === 'function' && findSimilarSamples(path); } },
    ];
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name) });
    }
    showContextMenu(e, items);
    return;
  }

  // ── Plugin cards ──
  const pluginCard = e.target.closest('#pluginList .plugin-card');
  // Helper: build quick-tag menu items for a path
  function quickTagItems(path, name) {
    const items = [];
    if (typeof getNote !== 'function' || typeof getAllTags !== 'function') return items;
    const note = getNote(path);
    const currentTags = note?.tags || [];
    const allTags = getAllTags();
    if (allTags.length > 0) {
      items.push('---');
      for (const tag of allTags.slice(0, 8)) {
        const has = currentTags.includes(tag);
        items.push({ icon: has ? '&#10003;' : '&#9634;', label: `${has ? 'Remove' : 'Add'} tag: ${tag}`,
          action: () => { if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag); showToast(`Tag "${tag}" ${has ? 'removed' : 'added'}`); }
        });
      }
    }
    return items;
  }

  if (pluginCard) {
    e.preventDefault();
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
    items.push({ icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } });
    items.push('---');
    items.push({ icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) });
    items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) });
    if (archBadges) {
      items.push({ icon: '&#128203;', label: 'Copy Architecture', action: () => copyToClipboard(archBadges) });
    }
    items.push('---');
    if (typeof isFavorite === 'function') {
      const pluginFav = isFavorite(path);
      items.push({ icon: pluginFav ? '&#9734;' : '&#9733;', label: pluginFav ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => pluginFav ? removeFavorite(path) : addFavorite('plugin', path, name, { format: pluginCard.querySelector('.plugin-type')?.textContent }) });
    }
    if (typeof showNoteEditor === 'function') items.push({ icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) });
    if (typeof findProjectsUsingPlugin === 'function') {
      items.push({ icon: '&#9889;', label: 'Find Projects Using This', action: () => {
        const projects = findProjectsUsingPlugin(name);
        showReverseXrefModal(name, projects);
      }});
    }
    items.push(...quickTagItems(path, name));
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
      '---',
      { icon: '&#127926;', label: 'Open in Music', action: () => openWithApp(path, 'Music') },
      { icon: '&#127911;', label: 'Open in QuickTime', action: () => openWithApp(path, 'QuickTime Player') },
      { icon: '&#127908;', label: 'Open in Audacity', action: () => openWithApp(path, 'Audacity') },
      { icon: '&#9889;', label: 'Open in Default App', action: () => window.vstUpdater.openDawProject(path).catch(() => {}) },
      '---',
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openAudioFolder(path) },
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('sample', path, name, { format: audioRow.querySelector('.format-badge')?.textContent }) }; })()],
      { icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) },
      ...quickTagItems(path, name),
      '---',
      { icon: '&#128270;', label: 'Find Similar Samples', action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) },
      '---',
      ...[(() => { const on = prefs.getItem('expandOnClick') !== 'off'; return { icon: on ? '&#9660;' : '&#9654;', label: on ? 'Disable Row Expand' : 'Enable Row Expand',
        action: () => {
          if (on) {
            // Disable: close any expanded row
            prefs.setItem('expandOnClick', 'off');
            const meta = document.getElementById('audioMetaRow');
            if (meta) { meta.remove(); expandedMetaPath = null; }
            const exp = document.querySelector('#audioTableBody tr.row-expanded');
            if (exp) exp.classList.remove('row-expanded');
          } else {
            // Enable, play, and expand the right-clicked row
            prefs.setItem('expandOnClick', 'on');
            if (typeof previewAudio === 'function') previewAudio(path);
            const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
            if (row) row.click();
          }
          if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
          showToast(on ? 'Row expand disabled' : 'Row expand enabled');
        } }; })()],
      ...[(() => { const ap = prefs.getItem('autoplayNext') !== 'off'; return { icon: ap ? '&#9209;' : '&#9654;', label: ap ? 'Disable Autoplay Next' : 'Enable Autoplay Next',
        action: () => { prefs.setItem('autoplayNext', ap ? 'off' : 'on'); if (typeof refreshSettingsUI === 'function') refreshSettingsUI(); showToast(ap ? 'Autoplay next disabled' : 'Autoplay next enabled'); } }; })()],
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
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      ...(typeof isXrefSupported === 'function' && isXrefSupported(dawRow.querySelector('.format-badge.format-default')?.textContent || '')
        ? [{ icon: '&#9889;', label: 'Show Plugins Used', action: () => showProjectPlugins(path, name) }]
        : []),
      ...(typeof showProjectViewer === 'function'
        ? [{ icon: '&#128196;', label: 'Explore Project Contents', action: () => showProjectViewer(path, name) }]
        : []),
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('daw', path, name, { format: dawRow.querySelector('.format-badge:last-of-type')?.textContent, daw: dawName }) }; })()],
      { icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) },
      ...quickTagItems(path, name),
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
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => f ? removeFavorite(path) : addFavorite('preset', path, name, { format: presetRow.querySelector('.format-badge')?.textContent }) }; })()],
      { icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) },
      ...quickTagItems(path, name),
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
      items.push('---');
      items.push({ icon: '&#128270;', label: 'Find Duplicates', action: () => showDuplicateReport() });
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
      { icon: '&#128202;', label: 'Heatmap Dashboard', action: () => { if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard(); } },
      { icon: '&#128200;', label: 'Dependency Graph', action: () => { if (typeof showDepGraph === 'function') showDepGraph(); } },
      '---',
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
    items.push({ icon: '&#9868;', label: 'Hide Player', action: () => hidePlayer() });
    items.push({ icon: '&#10005;', label: 'Stop &amp; Close', action: () => stopAudioPlayback() });
    showContextMenu(e, items);
    return;
  }

  // ── Favorite items ──
  const favItem = e.target.closest('.fav-item');
  if (favItem) {
    const path = favItem.dataset.path || '';
    const name = favItem.querySelector('.fav-name')?.textContent?.trim() || '';
    const type = favItem.dataset.type || '';
    const items = [];

    if (type === 'sample') {
      const isPlaying = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === path && !audioPlayer.paused;
      items.push({ icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? 'Pause' : 'Play', action: () => previewAudio(path) });
      items.push({ icon: '&#8634;', label: 'Loop', action: () => toggleRowLoop(path, new MouseEvent('click')) });
      items.push('---');
      items.push({ icon: '&#127926;', label: 'Open in Music', action: () => openWithApp(path, 'Music') });
      items.push({ icon: '&#127911;', label: 'Open in QuickTime', action: () => openWithApp(path, 'QuickTime Player') });
      items.push({ icon: '&#127908;', label: 'Open in Audacity', action: () => openWithApp(path, 'Audacity') });
      items.push('---');
    } else if (type === 'daw') {
      const daw = favItem.querySelector('.format-badge')?.textContent || 'DAW';
      items.push({ icon: '&#9654;', label: `Open in ${daw}`, action: () => { showToast(`Opening "${name}" in ${daw}...`); window.vstUpdater.openDawProject(path).catch(err => showToast(`${daw} not installed — ${err}`, 4000, 'error')); } });
      items.push('---');
    } else if (type === 'plugin') {
      const plugin = typeof allPlugins !== 'undefined' && allPlugins.find(p => p.path === path);
      const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
      items.push({ icon: '&#127760;', label: 'Open on KVR', action: () => window.vstUpdater.openUpdate(kvrUrl) });
      if (typeof findProjectsUsingPlugin === 'function') {
        items.push({ icon: '&#9889;', label: 'Find Projects Using This', action: () => { const projects = findProjectsUsingPlugin(name); showReverseXrefModal(name, projects); } });
      }
      items.push('---');
    }

    items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => {
      if (type === 'sample') openAudioFolder(path);
      else if (type === 'daw') openDawFolder(path);
      else if (type === 'preset') openPresetFolder(path);
      else openFolder(path);
    }});
    items.push({ icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } });
    items.push('---');
    items.push({ icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) });
    items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) });
    items.push('---');
    items.push({ icon: '&#128221;', label: 'Add Note', action: () => showNoteEditor(path, name) });
    items.push(...quickTagItems(path, name));
    items.push('---');
    items.push({ icon: '&#9734;', label: 'Remove from Favorites', action: () => { removeFavorite(path); if (typeof renderFavorites === 'function') renderFavorites(); } });

    showContextMenu(e, items);
    return;
  }

  // ── Note items ──
  const noteItem = e.target.closest('.note-item');
  if (noteItem) {
    const path = noteItem.dataset.path || '';
    const name = noteItem.querySelector('.note-item-name')?.textContent?.trim() || '';
    const items = [
      { icon: '&#128221;', label: 'Edit Note', action: () => showNoteEditor(path, name) },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openFolder(path) },
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      { icon: '&#9733;', label: isFavorite(path) ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => isFavorite(path) ? removeFavorite(path) : addFavorite('item', path, name) },
      { icon: '&#128465;', label: 'Delete Note', action: () => { if (typeof deleteNote === 'function') { deleteNote(path); if (typeof renderNotesTab === 'function') renderNotesTab(); } } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Tag items ──
  const tagItem = e.target.closest('.tag-badge[data-tag]');
  if (tagItem) {
    const tag = tagItem.dataset.tag || '';
    const items = [
      { icon: '&#128269;', label: 'Filter by This Tag', action: () => { if (typeof setGlobalTag === 'function') setGlobalTag(tag); } },
      { icon: '&#128203;', label: 'Copy Tag Name', action: () => copyToClipboard(tag) },
      '---',
      { icon: '&#128465;', label: 'Delete Tag from All Items', action: () => { if (typeof deleteTagGlobally === 'function' && confirm(`Delete tag "${tag}" from all items?`)) { deleteTagGlobally(tag); } } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Note cards ──
  const noteCard = e.target.closest('.note-card');
  if (noteCard) {
    const pathEl = noteCard.querySelector('.note-card-path');
    const nameEl = noteCard.querySelector('.note-card-name');
    const path = pathEl?.textContent?.trim() || '';
    const name = nameEl?.textContent?.trim() || '';
    const editBtn = noteCard.querySelector('[data-action-note="edit"]');
    const items = [
      { icon: '&#128221;', label: 'Edit Note', action: () => { if (editBtn) editBtn.click(); else if (typeof showNoteEditor === 'function') showNoteEditor(path, name); } },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openFolder(path) },
      { icon: '&#128194;', label: 'Show in File Browser', action: () => { switchTab('files'); loadDirectory(path.replace(/\/[^/]+$/, '')); } },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      '---',
      { icon: '&#128465;', label: 'Delete Note', action: () => { if (typeof deleteNote === 'function') { deleteNote(path); if (typeof renderNotesTab === 'function') renderNotesTab(); } } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Dep graph plugin rows ──
  const depRow = e.target.closest('.dep-plugin-row');
  if (depRow) {
    const name = depRow.querySelector('.dep-plugin-name')?.textContent?.trim() || '';
    const mfg = depRow.querySelector('.dep-plugin-mfg')?.textContent?.trim() || '';
    const items = [
      { icon: '&#128203;', label: 'Copy Plugin Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Manufacturer', action: () => copyToClipboard(mfg) },
    ];
    if (typeof findProjectsUsingPlugin === 'function') {
      items.push('---');
      items.push({ icon: '&#9889;', label: 'Find Projects Using This', action: () => { const projects = findProjectsUsingPlugin(name); showReverseXrefModal(name, projects); } });
    }
    const plugin = typeof allPlugins !== 'undefined' && allPlugins.find(p => p.name === name);
    if (plugin) {
      const kvrUrl = plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer);
      items.push({ icon: '&#127760;', label: 'Open on KVR', action: () => window.vstUpdater.openUpdate(kvrUrl) });
      items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => openFolder(plugin.path) });
    }
    showContextMenu(e, items);
    return;
  }

  // ── Dep graph project rows ──
  const depProject = e.target.closest('.dep-project-row');
  if (depProject) {
    const path = depProject.dataset.depProject || '';
    const name = depProject.querySelector('.dep-project-name')?.textContent?.trim() || '';
    const daw = depProject.querySelector('.format-badge')?.textContent?.trim() || '';
    const items = [
      { icon: '&#9654;', label: `Open in ${daw || 'DAW'}`, action: () => { showToast(`Opening "${name}"...`); window.vstUpdater.openDawProject(path).catch(err => showToast(`Failed — ${err}`, 4000, 'error')); } },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openDawFolder === 'function' && openDawFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Dep graph orphaned plugin rows ──
  const depOrphan = e.target.closest('.dep-orphan');
  if (depOrphan) {
    const name = depOrphan.querySelector('.dep-plugin-name')?.textContent?.trim() || '';
    const path = depOrphan.getAttribute('title') || '';
    const items = [
      { icon: '&#128203;', label: 'Copy Plugin Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openFolder === 'function' && openFolder(path) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Tab buttons ──
  const tabBtn = e.target.closest('.tab-btn');
  if (tabBtn) {
    const tab = tabBtn.dataset.tab;
    const exportMap = { plugins: 'exportPlugins', samples: 'exportAudio', daw: 'exportDaw', presets: 'exportPresets' };
    const scanMap = { plugins: 'scanPlugins', samples: 'scanAudioSamples', daw: 'scanDawProjects', presets: 'scanPresets' };
    const items = [
      { icon: '&#8635;', label: 'Switch to Tab', action: () => switchTab(tab) },
      '---',
    ];
    const scanFn = scanMap[tab];
    if (scanFn && typeof window[scanFn] === 'function') {
      items.push({ icon: '&#9889;', label: 'Rescan Tab Data', action: () => window[scanFn]() });
    }
    const exportFn = exportMap[tab];
    if (exportFn && typeof window[exportFn] === 'function') {
      items.push({ icon: '&#8615;', label: 'Export Tab Data', action: () => window[exportFn]() });
    }
    if (scanFn || exportFn) items.push('---');
    items.push({ icon: '&#8644;', label: 'Reset Tab Order', action: () => settingResetTabOrder() });
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

  // ── File browser breadcrumbs ──
  const crumb = e.target.closest('.file-crumb');
  if (crumb) {
    const crumbPath = crumb.dataset.fileNav || '';
    const items = [
      { icon: '&#128193;', label: 'Open in Finder', action: () => typeof openFolder === 'function' && openFolder(crumbPath) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(crumbPath) },
      { icon: '&#9733;', label: 'Bookmark This Directory', action: () => typeof addFavDir === 'function' && addFavDir(crumbPath) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── File browser rows ──
  const fileRow = e.target.closest('.file-row');
  if (fileRow && !e.target.closest('.fb-meta-panel')) {
    const path = fileRow.dataset.filePath;
    const isDir = fileRow.dataset.fileDir === 'true';
    const name = path.split('/').pop();
    const ext = name.split('.').pop().toLowerCase();
    const isAudio = !isDir && typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
    const items = [];
    if (isAudio) {
      items.push({ icon: '&#9654;', label: 'Play', action: () => typeof previewAudio === 'function' && previewAudio(path) });
      items.push({ icon: '&#128269;', label: 'Show in Samples Tab', action: async () => {
        // If not in allAudioSamples, add it
        if (typeof allAudioSamples !== 'undefined' && !allAudioSamples.some(s => s.path === path)) {
          try {
            const meta = await window.vstUpdater.getAudioMetadata(path);
            allAudioSamples.push({
              name: meta.fileName.replace(/\.[^.]+$/, ''),
              path: meta.fullPath,
              directory: meta.directory || path.replace(/\/[^/]+$/, ''),
              format: meta.format,
              size: meta.sizeBytes,
              sizeBytes: meta.sizeBytes,
              sizeFormatted: typeof formatAudioSize === 'function' ? formatAudioSize(meta.sizeBytes) : '',
              modified: meta.modified || '',
              duration: meta.duration || null,
              sampleRate: meta.sampleRate || null,
              channels: meta.channels || null,
              bitsPerSample: meta.bitsPerSample || null,
            });
          } catch {}
        }
        switchTab('samples');
        // Clear any existing search filter so the row is visible
        const searchInput = document.getElementById('audioSearchInput');
        if (searchInput && searchInput.value) { searchInput.value = ''; }
        if (typeof filterAudioSamples === 'function') filterAudioSamples();
        // Scroll to and highlight the row
        setTimeout(() => {
          const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
          if (row) {
            row.scrollIntoView({ block: 'center', behavior: 'smooth' });
            row.classList.add('row-playing');
            setTimeout(() => row.classList.remove('row-playing'), 2000);
          }
        }, 100);
      }});
      items.push({ icon: '&#128270;', label: 'Find Similar', action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
      items.push('---');
    }
    if (isDir) {
      items.push({ icon: '&#128193;', label: 'Open Directory', action: () => typeof loadDirectory === 'function' && loadDirectory(path) });
    }
    items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => {
      const dir = isDir ? path : path.replace(/\/[^/]+$/, '');
      if (typeof openFolder === 'function') openFolder(dir);
      else if (typeof openAudioFolder === 'function') openAudioFolder(path);
    }});
    items.push({ icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) });
    items.push({ icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) });
    items.push('---');
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      const favType = isDir ? 'folder' : 'file';
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? 'Remove from Favorites' : 'Add to Favorites',
        action: () => { fav ? removeFavorite(path) : addFavorite(favType, path, name); if (typeof renderFileList === 'function') renderFileList(); } });
    }
    items.push({ icon: '&#128221;', label: 'Add Note / Tags', action: () => { if (typeof showNoteEditor === 'function') showNoteEditor(path, name); } });
    if (isAudio) {
      items.push('---');
      const ap = prefs.getItem('autoplayNext') !== 'off';
      items.push({ icon: ap ? '&#9209;' : '&#9654;', label: ap ? 'Disable Autoplay Next' : 'Enable Autoplay Next',
        action: () => { prefs.setItem('autoplayNext', ap ? 'off' : 'on'); if (typeof refreshSettingsUI === 'function') refreshSettingsUI(); showToast(ap ? 'Autoplay next disabled' : 'Autoplay next enabled'); } });
    }
    showContextMenu(e, items);
    return;
  }

  // ── Disk usage segments ──
  const diskSeg = e.target.closest('.disk-segment, .disk-legend-item');
  if (diskSeg) {
    const label = diskSeg.getAttribute('title') || diskSeg.textContent.trim();
    const items = [
      { icon: '&#128203;', label: 'Copy', action: () => copyToClipboard(label) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── EQ/Gain/Pan sliders ──
  const eqSlider = e.target.closest('.eq-slider, .volume-slider');
  if (eqSlider) {
    const items = [
      { icon: '&#8634;', label: 'Reset to Default', action: () => { if (typeof resetEq === 'function') resetEq(); } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Expanded metadata panel ──
  const metaPanel = e.target.closest('.audio-meta-panel');
  if (metaPanel && !e.target.closest('.meta-waveform')) {
    const metaRow = metaPanel.closest('#audioMetaRow');
    const path = metaRow?.getAttribute('data-meta-path') || '';
    const metaValue = e.target.closest('.meta-item');
    const items = [];
    // Copy the specific value if clicking a meta-item
    if (metaValue) {
      const label = metaValue.querySelector('.meta-label')?.textContent || '';
      const val = metaValue.querySelector('.meta-value')?.textContent || '';
      items.push({ icon: '&#128203;', label: `Copy ${label}`, action: () => copyToClipboard(val) });
    }
    if (path) {
      items.push({ icon: '&#128203;', label: 'Copy File Path', action: () => copyToClipboard(path) });
      items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) });
      items.push('---');
      items.push({ icon: '&#9654;', label: 'Play', action: () => typeof previewAudio === 'function' && previewAudio(path) });
      if (typeof isFavorite === 'function') {
        const fav = isFavorite(path);
        const name = metaPanel.querySelector('.meta-value')?.textContent || '';
        items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? 'Remove from Favorites' : 'Add to Favorites',
          action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name) });
      }
      items.push({ icon: '&#128221;', label: 'Add Note', action: () => { const name = metaPanel.querySelector('.meta-value')?.textContent || ''; typeof showNoteEditor === 'function' && showNoteEditor(path, name); } });
      items.push({ icon: '&#128270;', label: 'Find Similar', action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
      items.push('---');
      items.push({ icon: '&#10005;', label: 'Close Panel', action: () => { const mr = document.getElementById('audioMetaRow'); if (mr) mr.remove(); expandedMetaPath = null; } });
    }
    if (items.length > 0) { showContextMenu(e, items); return; }
  }

  // ── Waveform ──
  const waveform = e.target.closest('.now-playing-waveform, .meta-waveform');
  if (waveform) {
    const items = [];
    if (typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
      items.push({ icon: '&#128203;', label: 'Copy File Path', action: () => copyToClipboard(audioPlayerPath) });
      items.push({ icon: '&#128193;', label: 'Reveal in Finder', action: () => typeof openAudioFolder === 'function' && openAudioFolder(audioPlayerPath) });
    }
    if (items.length > 0) { showContextMenu(e, items); return; }
  }

  // ── Shortcut keys ──
  const shortcutKey = e.target.closest('.shortcut-key');
  if (shortcutKey) {
    const scId = shortcutKey.dataset.shortcutId;
    const items = [
      { icon: '&#9881;', label: 'Rebind This Shortcut', action: () => shortcutKey.click() },
      { icon: '&#8634;', label: 'Reset All Shortcuts', action: () => typeof resetShortcuts === 'function' && resetShortcuts() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Color scheme buttons ──
  const schemeBtn = e.target.closest('.scheme-btn');
  if (schemeBtn) {
    const scheme = schemeBtn.dataset.scheme;
    const items = [
      { icon: '&#127912;', label: `Apply ${scheme || 'scheme'}`, action: () => schemeBtn.click() },
      { icon: '&#128203;', label: 'Copy Scheme Name', action: () => copyToClipboard(scheme || '') },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Progress bars ──
  const progressBar = e.target.closest('.audio-progress-bar, .global-progress, .progress-bar');
  if (progressBar) {
    const items = [
      { icon: '&#9632;', label: 'Stop All Scans', action: () => typeof stopAll === 'function' && stopAll() },
    ];
    showContextMenu(e, items);
    return;
  }

  // Smart playlists section
  const spSection = e.target.closest('.np-smart-playlists-section');
  if (spSection && !e.target.closest('.sp-item')) {
    const items = [
      { icon: '&#127926;', label: 'New Smart Playlist', action: () => typeof showSmartPlaylistEditor === 'function' && showSmartPlaylistEditor(null) },
      '---',
    ];
    if (typeof getSmartPlaylistPresets === 'function') {
      for (const preset of getSmartPlaylistPresets()) {
        items.push({ icon: '&#127925;', label: `Add: ${preset.name}`, action: () => {
          if (typeof createSmartPlaylist === 'function') {
            const pl = createSmartPlaylist(preset.name, preset.rules);
            pl.matchMode = preset.matchMode;
            if (typeof saveSmartPlaylists === 'function') saveSmartPlaylists();
            showToast(`Created "${preset.name}"`);
          }
        }});
      }
    }
    showContextMenu(e, items);
    return;
  }

  // ── Similar panel ──
  const simPanel = e.target.closest('.similar-panel');
  if (simPanel && !e.target.closest('[data-similar-path]')) {
    const items = [
      { icon: '&#9866;', label: 'Minimize', action: () => typeof minimizeSimilarPanel === 'function' && minimizeSimilarPanel() },
      { icon: '&#10005;', label: 'Close', action: () => typeof closeSimilarPanel === 'function' && closeSimilarPanel() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Heatmap dashboard ──
  const hmDash = e.target.closest('#heatmapDashModal');
  if (hmDash) {
    const card = e.target.closest('.hm-card');
    const barRow = e.target.closest('.hm-bar-row');
    const items = [];
    if (barRow) {
      const label = barRow.querySelector('.hm-bar-label')?.textContent || '';
      const val = barRow.querySelector('.hm-bar-val')?.textContent || '';
      items.push({ icon: '&#128203;', label: `Copy "${label}: ${val}"`, action: () => copyToClipboard(`${label}: ${val}`) });
    }
    if (card) {
      const title = card.querySelector('.hm-card-title')?.textContent || '';
      items.push({ icon: '&#128203;', label: `Copy ${title} Data`, action: () => {
        const rows = [...card.querySelectorAll('.hm-bar-row')].map(r => {
          const l = r.querySelector('.hm-bar-label')?.textContent || '';
          const v = r.querySelector('.hm-bar-val')?.textContent || '';
          return `${l}\t${v}`;
        }).join('\n');
        copyToClipboard(rows || title);
      }});
    }
    items.push('---');
    items.push({ icon: '&#8634;', label: 'Refresh Dashboard', action: () => { if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard(); } });
    items.push({ icon: '&#10005;', label: 'Close Dashboard', action: () => { if (typeof closeHeatmapDash === 'function') closeHeatmapDash(); } });
    showContextMenu(e, items);
    return;
  }

  // ── Walker tiles ──
  const walkerTile = e.target.closest('.walker-tile');
  if (walkerTile) {
    const body = walkerTile.querySelector('.walker-tile-body');
    const dirs = body ? [...body.querySelectorAll('.walker-dir')].map(d => d.textContent).join('\n') : '';
    const title = walkerTile.querySelector('.walker-tile-title, h4, h3')?.textContent?.trim() || 'Walker';
    const items = [
      { icon: '&#128203;', label: 'Copy All Paths', action: () => copyToClipboard(dirs), disabled: !dirs },
      { icon: '&#128203;', label: 'Copy Tile Title', action: () => copyToClipboard(title) },
      '---',
      { icon: '&#10005;', label: 'Clear Tile', action: () => { if (body) body.innerHTML = ''; showToast(`${title} cleared`); } },
    ];
    showContextMenu(e, items);
    return;
  }

  } catch (err) { console.error('Context menu error:', err, err.stack); showToast('Context menu error: ' + (err.message || err), 4000, 'error'); }
});
