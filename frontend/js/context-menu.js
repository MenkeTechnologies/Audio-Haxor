// ── Context Menu ──
const ctxMenu = document.getElementById('ctxMenu');

function showContextMenu(e, items) {
  e.preventDefault();
  ctxMenu.innerHTML = items.map(item => {
    if (item === '---') return '<div class="ctx-menu-sep"></div>';
    return `<div class="ctx-menu-item" data-ctx-idx="${item._idx}">
      <span class="ctx-icon">${item.icon || ''}</span>${item.label}
    </div>`;
  }).join('');

  // Store callbacks
  ctxMenu._actions = {};
  items.forEach((item, i) => {
    if (item !== '---' && item.action) {
      item._idx = i;
      ctxMenu._actions[i] = item.action;
    }
  });
  // Re-render with indices
  ctxMenu.innerHTML = items.map((item, i) => {
    if (item === '---') return '<div class="ctx-menu-sep"></div>';
    return `<div class="ctx-menu-item" data-ctx-idx="${i}">
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
  if (!item) return;
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
  // Plugin cards
  const pluginCard = e.target.closest('#pluginList .plugin-card');
  if (pluginCard) {
    const name = pluginCard.querySelector('h3')?.textContent || '';
    const path = pluginCard.dataset.path || '';
    const kvrBtn = pluginCard.querySelector('[data-action="openKvr"]');
    const mfgBtn = pluginCard.querySelector('[data-action="openUpdate"][title]');
    const folderBtn = pluginCard.querySelector('[data-action="openFolder"]');
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
    showContextMenu(e, items);
    return;
  }

  // Audio sample rows
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
    ];
    showContextMenu(e, items);
    return;
  }

  // DAW project rows
  const dawRow = e.target.closest('#dawTableBody tr[data-daw-path]');
  if (dawRow) {
    const path = dawRow.dataset.dawPath;
    const name = dawRow.querySelector('.col-name')?.textContent || '';
    const dawName = dawRow.querySelector('.format-badge')?.textContent || 'DAW';
    const items = [
      { icon: '&#9654;', label: `Open in ${dawName}`, action: () => { showToast(`Opening "${name}" in ${dawName}...`); window.vstUpdater.openDawProject(path); } },
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openDawFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
    ];
    showContextMenu(e, items);
    return;
  }

  // Preset rows
  const presetRow = e.target.closest('#presetTableBody tr[data-preset-path]');
  if (presetRow) {
    const path = presetRow.dataset.presetPath;
    const name = presetRow.querySelector('td')?.textContent || '';
    const items = [
      { icon: '&#128193;', label: 'Reveal in Finder', action: () => openPresetFolder(path) },
      '---',
      { icon: '&#128203;', label: 'Copy Name', action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: 'Copy Path', action: () => copyToClipboard(path) },
    ];
    showContextMenu(e, items);
    return;
  }

  // History entries
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

  // Tab buttons
  const tabBtn = e.target.closest('.tab-btn');
  if (tabBtn) {
    const tab = tabBtn.dataset.tab;
    const items = [
      { icon: '&#8635;', label: 'Switch to Tab', action: () => switchTab(tab) },
    ];
    showContextMenu(e, items);
    return;
  }
});
