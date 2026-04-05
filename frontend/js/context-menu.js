// ── Context Menu ──
const ctxMenu = document.getElementById('ctxMenu');
/** Spread into menu items that already toast or should not echo the label (locale-safe; no English heuristics). */
const _noEcho = { skipEchoToast: true };

function showContextMenu(e, items) {
  e.preventDefault();
  // Store callbacks and render
  ctxMenu._actions = {};
  ctxMenu._labels = {};
  ctxMenu._skipEcho = {};
  ctxMenu.innerHTML = items.map((item, i) => {
    if (item === '---') return '<div class="ctx-menu-sep"></div>';
    if (item.action) {
      ctxMenu._actions[i] = item.action;
      ctxMenu._labels[i] = item.label;
      if (item.skipEchoToast) ctxMenu._skipEcho[i] = true;
    }
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
  ctxMenu._labels = {};
  ctxMenu._skipEcho = {};
}

// Click on menu item
ctxMenu.addEventListener('click', (e) => {
  const item = e.target.closest('.ctx-menu-item');
  if (!item || item.classList.contains('ctx-disabled')) return;
  const idx = item.dataset.ctxIdx;
  const action = ctxMenu._actions[idx];
  const label = ctxMenu._labels?.[idx];
  const skipEcho = ctxMenu._skipEcho?.[idx];
  hideContextMenu();
  if (action) action();
  if (label && !skipEcho) showToast(label);
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
    showToast(toastFmt('toast.opening_in_app', { app: appName }));
  }).catch(err => {
    showToast(toastFmt('toast.app_not_available', { app: appName, err }), 4000, 'error');
  });
}

// Copy helper
function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    showToast(toastFmt('toast.copied_clipboard'));
  }).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
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
      { icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, action: () => typeof previewAudio === 'function' && previewAudio(path) },
      { icon: '&#8634;', label: appFmt('menu.loop'), ..._noEcho, action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click')) },
      '---',
      { icon: '&#127926;', label: appFmt('menu.open_in_music'), ..._noEcho, action: () => typeof openWithApp === 'function' && openWithApp(path, 'Music') },
      { icon: '&#127911;', label: appFmt('menu.open_in_quicktime'), ..._noEcho, action: () => typeof openWithApp === 'function' && openWithApp(path, 'QuickTime Player') },
      { icon: '&#127908;', label: appFmt('menu.open_audacity'), ..._noEcho, action: () => typeof openWithApp === 'function' && openWithApp(path, 'Audacity') },
      '---',
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => { if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, '')); }, 200); } },
      { icon: '&#127925;', label: appFmt('menu.show_in_samples_tab'), ..._noEcho, action: () => {
        switchTab('samples');
        setTimeout(() => {
          const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
          if (row) {
            row.scrollIntoView({ behavior: 'smooth', block: 'center' });
            row.classList.add('row-playing');
            setTimeout(() => row.classList.remove('row-playing'), 2000);
          } else {
            // Sample not in table — add it on the spot from recently played data
            const recent = typeof recentlyPlayed !== 'undefined' ? recentlyPlayed.find(r => r.path === path) : null;
            if (recent) {
              const sample = { name: recent.name || name, path, directory: path.replace(/\/[^/]+$/, ''), format: recent.format || path.split('.').pop().toUpperCase(), size: 0, sizeFormatted: recent.size || '?', modified: '' };
              if (typeof allAudioSamples !== 'undefined') allAudioSamples.push(sample);
              if (typeof filteredAudioSamples !== 'undefined') filteredAudioSamples.push(sample);
              const tbody = document.getElementById('audioTableBody');
              if (tbody && typeof buildAudioRow === 'function') {
                tbody.insertAdjacentHTML('beforeend', buildAudioRow(sample));
                const newRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
                if (newRow) {
                  newRow.scrollIntoView({ behavior: 'smooth', block: 'center' });
                  newRow.classList.add('row-playing');
                  setTimeout(() => newRow.classList.remove('row-playing'), 2000);
                }
              }
              showToast(toastFmt('toast.added_name_to_samples', { name }));
            } else {
              const input = document.getElementById('audioSearchInput');
              if (input) { input.value = name; input.dispatchEvent(new Event('input')); }
            }
          }
        }, 200);
      }},
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
    ];
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name) });
    }
    if (typeof showNoteEditor === 'function') {
      items.push({ icon: '&#128221;', label: appFmt('menu.add_note_tags'), action: () => showNoteEditor(path, name) });
    }
    items.push(...quickTagItems(path, name));
    items.push('---');
    items.push({ icon: '&#128270;', label: appFmt('menu.find_similar_samples'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
    showContextMenu(e, items);
    return;
  }

  // ── Similarity result rows ──
  const simRow = e.target.closest('[data-similar-path]');
  if (simRow) {
    const path = simRow.dataset.similarPath || '';
    const name = path.split('/').pop().replace(/\.[^.]+$/, '');
    const items = [
      { icon: '&#9654;', label: appFmt('menu.play'), ..._noEcho, action: () => typeof previewAudio === 'function' && previewAudio(path) },
      { icon: '&#8634;', label: appFmt('menu.loop'), ..._noEcho, action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click')) },
      '---',
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => { if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, '')); }, 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      { icon: '&#128270;', label: appFmt('menu.find_similar_to_this'), action: () => { typeof closeSimilarPanel === 'function' && closeSimilarPanel(); typeof findSimilarSamples === 'function' && findSimilarSamples(path); } },
    ];
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
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
        items.push({ icon: has ? '&#10003;' : '&#9634;', label: has ? appFmt('menu.remove_tag_named', { tag }) : appFmt('menu.add_tag_named', { tag }), ..._noEcho,
          action: () => { if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag); showToast(has ? toastFmt('toast.tag_removed', { tag }) : toastFmt('toast.tag_added', { tag })); }
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
      { icon: '&#128269;', label: appFmt('menu.open_kvr'), ..._noEcho, action: () => kvrBtn && openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name) },
    ];
    if (mfgBtn && !mfgBtn.disabled) {
      items.push({ icon: '&#127760;', label: appFmt('menu.open_manufacturer_site'), ..._noEcho, action: () => openUpdate(mfgBtn.dataset.url) });
    }
    items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => folderBtn && openFolder(folderBtn.dataset.path) });
    items.push({ icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } });
    items.push('---');
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) });
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) });
    if (archBadges) {
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_architecture'), ..._noEcho, action: () => copyToClipboard(archBadges) });
    }
    items.push('---');
    if (typeof isFavorite === 'function') {
      const pluginFav = isFavorite(path);
      items.push({ icon: pluginFav ? '&#9734;' : '&#9733;', label: pluginFav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => pluginFav ? removeFavorite(path) : addFavorite('plugin', path, name, { format: pluginCard.querySelector('.plugin-type')?.textContent }) });
    }
    if (typeof showNoteEditor === 'function') items.push({ icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) });
    if (typeof findProjectsUsingPlugin === 'function') {
      items.push({ icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => {
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
      { icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, action: () => previewAudio(path) },
      { icon: '&#8634;', label: appFmt('menu.loop'), ..._noEcho, action: () => { toggleRowLoop(path, new MouseEvent('click')); } },
      '---',
      { icon: '&#127926;', label: appFmt('menu.open_in_music'), ..._noEcho, action: () => openWithApp(path, 'Music') },
      { icon: '&#127911;', label: appFmt('menu.open_in_quicktime'), ..._noEcho, action: () => openWithApp(path, 'QuickTime Player') },
      { icon: '&#127908;', label: appFmt('menu.open_audacity'), ..._noEcho, action: () => openWithApp(path, 'Audacity') },
      { icon: '&#9889;', label: appFmt('menu.open_default_app'), ..._noEcho, action: () => window.vstUpdater.openDawProject(path).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }) },
      '---',
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openAudioFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => f ? removeFavorite(path) : addFavorite('sample', path, name, { format: audioRow.querySelector('.format-badge')?.textContent }) }; })()],
      { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) },
      ...quickTagItems(path, name),
      '---',
      { icon: '&#128270;', label: appFmt('menu.find_similar_samples'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) },
      '---',
      ...[(() => { const on = prefs.getItem('expandOnClick') !== 'off'; return { icon: on ? '&#9660;' : '&#9654;', label: on ? appFmt('menu.disable_row_expand') : appFmt('menu.enable_row_expand'), ..._noEcho,
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
          showToast(on ? toastFmt('toast.row_expand_disabled') : toastFmt('toast.row_expand_enabled'));
        } }; })()],
      ...[(() => { const ap = prefs.getItem('autoplayNext') !== 'off'; return { icon: ap ? '&#9209;' : '&#9654;', label: ap ? appFmt('menu.disable_autoplay_next') : appFmt('menu.enable_autoplay_next'), ..._noEcho,
        action: () => { prefs.setItem('autoplayNext', ap ? 'off' : 'on'); if (typeof refreshSettingsUI === 'function') refreshSettingsUI(); showToast(ap ? toastFmt('toast.autoplay_next_disabled') : toastFmt('toast.autoplay_next_enabled')); } }; })()],
    ];
    showContextMenu(e, items);
    return;
  }

  // ── MIDI file rows ──
  const midiRow = e.target.closest('#midiTableBody tr[data-midi-path]');
  if (midiRow) {
    const path = midiRow.getAttribute('data-midi-path');
    const name = midiRow.querySelector('.col-name')?.textContent || '';
    const items = [
      { icon: '&#9654;', label: appFmt('menu.open_garageband'), ..._noEcho, action: () => window.vstUpdater.openWithApp(path, 'GarageBand').catch(() => showToast(toastFmt('toast.garageband_not_found'), 4000, 'error')) },
      { icon: '&#127911;', label: appFmt('menu.open_in_logic_pro'), ..._noEcho, action: () => window.vstUpdater.openWithApp(path, 'Logic Pro').catch(() => showToast(toastFmt('toast.logic_not_found'), 4000, 'error')) },
      { icon: '&#127925;', label: appFmt('menu.open_ableton_live'), ..._noEcho, action: () => window.vstUpdater.openWithApp(path, 'Ableton Live 12 Standard').catch(() => window.vstUpdater.openWithApp(path, 'Ableton Live 11 Suite').catch(() => showToast(toastFmt('toast.ableton_not_found'), 4000, 'error'))) },
      { icon: '&#9889;', label: appFmt('menu.open_with_default_app'), ..._noEcho, action: () => window.vstUpdater.openDawProject(path).catch(e => showToast(toastFmt('toast.no_midi_handler', { err: e }), 4000, 'error')) },
      '---',
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => { if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, '')); }, 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => typeof copyToClipboard === 'function' && copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => typeof copyToClipboard === 'function' && copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => f ? removeFavorite(path) : addFavorite('midi', path, name) }; })()],
      { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => typeof showNoteEditor === 'function' && showNoteEditor(path, name) },
      ...(typeof quickTagItems === 'function' ? quickTagItems(path, name) : []),
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
      { icon: '&#9654;', label: appFmt('menu.open_in_daw', { daw: dawName }), ..._noEcho, action: () => { showToast(toastFmt('toast.opening_in_daw', { name, daw: dawName })); window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', { daw: dawName, err }), 4000, 'error')); } },
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openDawFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      ...(typeof isXrefSupported === 'function' && isXrefSupported(dawRow.querySelector('.format-badge.format-default')?.textContent || '')
        ? [{ icon: '&#9889;', label: appFmt('menu.show_plugins_used'), action: () => showProjectPlugins(path, name) }]
        : []),
      ...(typeof showProjectViewer === 'function'
        ? [{ icon: '&#128196;', label: appFmt('menu.explore_project_contents'), action: () => showProjectViewer(path, name) }]
        : []),
      { icon: '&#128221;', label: appFmt('menu.open_in_text_editor'), ..._noEcho, action: () => {
        const ext = path.split('.').pop().toLowerCase();
        const xmlFormats = ['als', 'rpp', 'song', 'dawproject'];
        if (xmlFormats.includes(ext)) {
          // Decompress ALS first, others open directly
          if (ext === 'als') {
            window.vstUpdater.readAlsXml(path).then(xml => {
              const tmp = path.replace(/\.als$/i, '_decompressed.xml');
              window.vstUpdater.writeTextFile(tmp, xml).then(() => {
                window.vstUpdater.openWithApp(tmp, 'TextEdit').catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
                showToast(toastFmt('toast.decompressed_xml_textedit'));
              });
            }).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
          } else {
            window.vstUpdater.openWithApp(path, 'TextEdit').catch(() => window.vstUpdater.openDawProject(path).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }));
          }
        } else {
          // Binary — open with hex editor or default
          window.vstUpdater.openDawProject(path).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
        }
      }},
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => f ? removeFavorite(path) : addFavorite('daw', path, name, { format: dawRow.querySelector('.format-badge:last-of-type')?.textContent, daw: dawName }) }; })()],
      { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) },
      ...quickTagItems(path, name),
    ];
    showContextMenu(e, items);
    return;
  }

  // ── PDF rows ──
  const pdfRow = e.target.closest('#pdfTableBody tr[data-pdf-path]');
  if (pdfRow) {
    const path = pdfRow.dataset.pdfPath;
    const name = pdfRow.querySelector('td:nth-child(2)')?.textContent?.trim() || '';
    const items = [
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openPdfFile(path) },
      { icon: '&#9889;', label: appFmt('menu.open_with_default_app'), ..._noEcho, action: () => window.vstUpdater.openFileDefault(path).catch(err => showToast(toastFmt('toast.failed_open_file', { err: err.message || err }), 4000, 'error')) },
      { icon: '&#128195;', label: appFmt('menu.open_in_preview'), ..._noEcho, action: () => openWithApp(path, 'Preview') },
      { icon: '&#128196;', label: appFmt('menu.open_in_adobe_acrobat'), ..._noEcho, action: () => openWithApp(path, 'Adobe Acrobat') },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => f ? removeFavorite(path) : addFavorite('pdf', path, name) }; })()],
      { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) },
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
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openPresetFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      ...[(() => { const f = typeof isFavorite === 'function' && isFavorite(path); return { icon: f ? '&#9734;' : '&#9733;', label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => f ? removeFavorite(path) : addFavorite('preset', path, name, { format: presetRow.querySelector('.format-badge')?.textContent }) }; })()],
      { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) },
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
      { icon: '&#9650;', label: appFmt('menu.sort_ascending'), action: () => {
        if (action === 'sortAudio') { audioSortAsc = true; audioSortKey = key; sortAudio(key); }
        else if (action === 'sortDaw') { dawSortAsc = true; dawSortKey = key; sortDaw(key); }
        else if (action === 'sortPreset') { presetSortAsc = true; presetSortKey = key; sortPreset(key); }
        else if (action === 'sortPdf') { pdfSortAsc = true; pdfSortKey = key; sortPdf(key); }
      }},
      { icon: '&#9660;', label: appFmt('menu.sort_descending'), action: () => {
        if (action === 'sortAudio') { audioSortAsc = false; audioSortKey = key; sortAudio(key); }
        else if (action === 'sortDaw') { dawSortAsc = false; dawSortKey = key; sortDaw(key); }
        else if (action === 'sortPreset') { presetSortAsc = false; presetSortKey = key; sortPreset(key); }
        else if (action === 'sortPdf') { pdfSortAsc = false; pdfSortKey = key; sortPdf(key); }
      }},
      '---',
      { icon: '&#8596;', label: appFmt('menu.reset_columns'), action: () => settingResetColumns() },
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
        { icon: '&#10005;', label: appFmt('menu.clear_search'), action: () => { input.value = ''; input.dispatchEvent(new Event('input', { bubbles: true })); }, disabled: !hasText },
        { icon: '&#128203;', label: appFmt('menu.paste_and_search'), action: async () => {
          try {
            const text = await navigator.clipboard.readText();
            input.value = text;
            input.dispatchEvent(new Event('input', { bubbles: true }));
          } catch(e) { if(typeof showToast==='function'&&e) showToast(String(e),4000,'error'); }
        }},
        '---',
        { icon: '.*', label: isRegex ? appFmt('menu.switch_to_fuzzy') : appFmt('menu.switch_to_regex'), action: () => regexBtn && toggleRegex(regexBtn) },
      ];
      showContextMenu(e, items);
      return;
    }
  }

  // ── Filter dropdowns ──
  const filterSelect = e.target.closest('.filter-select');
  if (filterSelect) {
    const items = [
      { icon: '&#8635;', label: appFmt('menu.reset_to_all'), action: () => {
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
      items.push({ icon: '&#8635;', label: appFmt('menu.scan_plugins'), action: () => scanPlugins() });
      items.push({ icon: '&#9889;', label: appFmt('menu.check_updates'), action: () => checkUpdates(), disabled: allPlugins.length === 0 });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_plugins'), action: () => exportPlugins(), disabled: allPlugins.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_plugins'), action: () => importPlugins() });
    } else if (tabId === 'tabSamples') {
      items.push({ icon: '&#127925;', label: appFmt('menu.scan_samples'), action: () => scanAudioSamples() });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_samples'), action: () => exportAudio(), disabled: allAudioSamples.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_samples'), action: () => importAudio() });
    } else if (tabId === 'tabDaw') {
      items.push({ icon: '&#127911;', label: appFmt('menu.scan_daw'), action: () => scanDawProjects() });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_projects'), action: () => exportDaw(), disabled: allDawProjects.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_projects_short'), action: () => importDaw() });
    } else if (tabId === 'tabPresets') {
      items.push({ icon: '&#127924;', label: appFmt('menu.scan_presets'), action: () => scanPresets() });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_presets'), action: () => exportPresets(), disabled: allPresets.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_presets'), action: () => importPresets() });
    } else if (tabId === 'tabMidi') {
      items.push({ icon: '&#127924;', label: appFmt('menu.scan_midi'), action: () => { if (typeof scanMidi === 'function') scanMidi(); } });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_midi_files'), action: () => { if (typeof exportMidi === 'function') exportMidi(); }, disabled: typeof allMidiFiles === 'undefined' || allMidiFiles.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_midi_list'), action: () => { if (typeof importAudio === 'function') importAudio(); } });
    } else if (tabId === 'tabPdf') {
      items.push({ icon: '&#8635;', label: appFmt('menu.scan_pdf'), action: () => { if (typeof scanPdfs === 'function') scanPdfs(); } });
      items.push('---');
      items.push({ icon: '&#8615;', label: appFmt('menu.export_pdfs'), action: () => { if (typeof exportPdfs === 'function') exportPdfs(); }, disabled: typeof allPdfs === 'undefined' || allPdfs.length === 0 });
      items.push({ icon: '&#8613;', label: appFmt('menu.import_pdfs'), action: () => { if (typeof importPdfs === 'function') importPdfs(); } });
    }
    if (items.length) {
      items.push('---');
      items.push({ icon: '&#128270;', label: appFmt('menu.find_duplicates'), action: () => showDuplicateReport() });
      showContextMenu(e, items);
      return;
    }
  }

  // ── Stats bar ──
  const statsBar = e.target.closest('.stats-bar');
  if (statsBar) {
    const statsText = [...statsBar.querySelectorAll('.stat')].map(s => s.textContent.trim()).join(' | ');
    const items = [
      { icon: '&#128203;', label: appFmt('menu.copy_stats'), ..._noEcho, action: () => copyToClipboard(statsText) },
      '---',
      { icon: '&#9889;', label: appFmt('menu.scan_all'), action: () => scanAll() },
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
        { icon: '&#128203;', label: appFmt('menu.copy_process_stats'), ..._noEcho, action: () => copyToClipboard(statsText) },
      ];
      showContextMenu(e, items);
      return;
    }
    const items = [
      { icon: '&#128202;', label: appFmt('menu.heatmap_dashboard'), action: () => { if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard(); } },
      { icon: '&#128200;', label: appFmt('menu.dep_graph'), action: () => { if (typeof showDepGraph === 'function') showDepGraph(); } },
      '---',
      { icon: '&#127760;', label: appFmt('menu.open_github_repository'), action: () => openUpdate('https://github.com/MenkeTechnologies/Audio-Haxor') },
      { icon: '&#9881;', label: appFmt('menu.tab_settings'), action: () => switchTab('settings') },
      '---',
      { icon: '&#9889;', label: appFmt('menu.scan_all'), action: () => scanAll() },
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
        { icon: '&#128269;', label: appFmt('menu.view_details'), action: () => selectScan(id, type) },
        { icon: '&#128465;', label: appFmt('menu.delete_entry'), action: () => {
          if (type === 'audio') deleteAudioScanEntry(id);
          else if (type === 'daw') deleteDawScanEntry(id);
          else if (type === 'preset') deletePresetScanEntry(id);
          else if (type === 'pdf') deletePdfScanEntry(id);
          else if (type === 'midi') deleteMidiScanEntry(id);
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
      { icon: '&#128465;', label: appFmt('menu.clear_history'), action: () => settingClearAllHistory() },
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
      items.push({ icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, action: () => toggleAudioPlayback() });
      items.push({ icon: '&#8634;', label: audioLooping ? appFmt('menu.disable_loop') : appFmt('menu.enable_loop'), ..._noEcho, action: () => toggleAudioLoop() });
      items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openAudioFolder(audioPlayerPath) });
      items.push('---');
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(audioPlayerPath) });
      items.push('---');
    }
    items.push({ icon: isExpanded ? '&#9660;' : '&#9650;', label: isExpanded ? appFmt('menu.player_collapse') : appFmt('menu.player_expand'), ..._noEcho, action: () => togglePlayerExpanded() });
    items.push({ icon: '&#9868;', label: appFmt('menu.hide_player'), action: () => hidePlayer() });
    items.push({ icon: '&#10005;', label: appFmt('menu.stop_and_close'), ..._noEcho, action: () => stopAudioPlayback() });
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
      items.push({ icon: isPlaying ? '&#9646;&#9646;' : '&#9654;', label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, action: () => previewAudio(path) });
      items.push({ icon: '&#8634;', label: appFmt('menu.loop'), ..._noEcho, action: () => toggleRowLoop(path, new MouseEvent('click')) });
      items.push('---');
      items.push({ icon: '&#127926;', label: appFmt('menu.open_in_music'), ..._noEcho, action: () => openWithApp(path, 'Music') });
      items.push({ icon: '&#127911;', label: appFmt('menu.open_in_quicktime'), ..._noEcho, action: () => openWithApp(path, 'QuickTime Player') });
      items.push({ icon: '&#127908;', label: appFmt('menu.open_audacity'), ..._noEcho, action: () => openWithApp(path, 'Audacity') });
      items.push('---');
    } else if (type === 'daw') {
      const daw = favItem.querySelector('.format-badge')?.textContent || 'DAW';
      items.push({ icon: '&#9654;', label: appFmt('menu.open_in_daw', { daw }), ..._noEcho, action: () => { showToast(toastFmt('toast.opening_in_daw', { name, daw })); window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', { daw, err }), 4000, 'error')); } });
      items.push('---');
    } else if (type === 'plugin') {
      const plugin = typeof allPlugins !== 'undefined' && findByPath(allPlugins, path);
      const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
      items.push({ icon: '&#127760;', label: appFmt('menu.open_kvr'), ..._noEcho, action: () => window.vstUpdater.openUpdate(kvrUrl) });
      if (typeof findProjectsUsingPlugin === 'function') {
        items.push({ icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => { const projects = findProjectsUsingPlugin(name); showReverseXrefModal(name, projects); } });
      }
      items.push('---');
    } else if (type === 'pdf') {
      items.push({ icon: '&#9889;', label: appFmt('menu.open_with_default_app'), ..._noEcho, action: () => window.vstUpdater.openFileDefault(path).catch(err => showToast(toastFmt('toast.failed_open_file', { err: err.message || err }), 4000, 'error')) });
      items.push({ icon: '&#128195;', label: appFmt('menu.open_in_preview'), ..._noEcho, action: () => openWithApp(path, 'Preview') });
      items.push({ icon: '&#128196;', label: appFmt('menu.open_in_adobe_acrobat'), ..._noEcho, action: () => openWithApp(path, 'Adobe Acrobat') });
      items.push('---');
    }

    items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => {
      if (type === 'sample') openAudioFolder(path);
      else if (type === 'daw') openDawFolder(path);
      else if (type === 'preset') openPresetFolder(path);
      else if (type === 'pdf') openPdfFile(path);
      else openFolder(path);
    }});
    items.push({ icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } });
    items.push('---');
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) });
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) });
    items.push('---');
    items.push({ icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) });
    items.push(...quickTagItems(path, name));
    items.push('---');
    items.push({ icon: '&#9734;', label: appFmt('menu.remove_from_favorites'), ..._noEcho, action: () => { removeFavorite(path); if (typeof renderFavorites === 'function') renderFavorites(); } });

    showContextMenu(e, items);
    return;
  }

  // ── Note items ──
  const noteItem = e.target.closest('.note-item');
  if (noteItem) {
    const path = noteItem.dataset.path || '';
    const name = noteItem.querySelector('.note-item-name')?.textContent?.trim() || '';
    const items = [
      { icon: '&#128221;', label: appFmt('menu.edit_note'), action: () => showNoteEditor(path, name) },
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      { icon: '&#9733;', label: isFavorite(path) ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => isFavorite(path) ? removeFavorite(path) : addFavorite('item', path, name) },
      { icon: '&#128465;', label: appFmt('menu.delete_note'), action: () => { if (typeof deleteNote === 'function') { deleteNote(path); if (typeof renderNotesTab === 'function') renderNotesTab(); } } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Tag items ──
  const tagItem = e.target.closest('.tag-badge[data-tag]');
  if (tagItem) {
    const tag = tagItem.dataset.tag || '';
    const items = [
      { icon: '&#128269;', label: appFmt('menu.filter_by_this_tag'), action: () => { if (typeof setGlobalTag === 'function') setGlobalTag(tag); } },
      { icon: '&#128203;', label: appFmt('menu.copy_tag_name'), ..._noEcho, action: () => copyToClipboard(tag) },
      '---',
      { icon: '&#128465;', label: appFmt('menu.delete_tag_globally'), action: () => { if (typeof deleteTagGlobally === 'function' && confirm(appFmt('confirm.delete_tag_globally', { tag }))) { deleteTagGlobally(tag); } } },
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
      { icon: '&#128221;', label: appFmt('menu.edit_note'), action: () => { if (editBtn) editBtn.click(); else if (typeof showNoteEditor === 'function') showNoteEditor(path, name); } },
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openFolder(path) },
      { icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => { switchTab('files'); setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200); } },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      '---',
      { icon: '&#128465;', label: appFmt('menu.delete_note'), action: () => { if (typeof deleteNote === 'function') { deleteNote(path); if (typeof renderNotesTab === 'function') renderNotesTab(); } } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Xref plugin items (plugins found in DAW projects) ──
  const xrefItem = e.target.closest('.xref-item[data-xref-plugin]');
  if (xrefItem) {
    const pluginName = xrefItem.dataset.xrefPlugin;
    const items = [
      { icon: '&#128269;', label: appFmt('menu.find_in_plugins_tab'), action: () => {
        switchTab('plugins');
        const input = document.getElementById('searchInput');
        if (input) { input.value = pluginName; input.dispatchEvent(new Event('input', { bubbles: true })); }
        showToast(toastFmt('toast.searching_plugins_for', { pluginName }));
      }},
      { icon: '&#128203;', label: appFmt('menu.copy_plugin_name'), ..._noEcho, action: () => { navigator.clipboard.writeText(pluginName); showToast(toastFmt('toast.copied_plugin_name', { pluginName })); }},
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
      { icon: '&#128203;', label: appFmt('menu.copy_plugin_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_manufacturer'), ..._noEcho, action: () => copyToClipboard(mfg) },
    ];
    if (typeof findProjectsUsingPlugin === 'function') {
      items.push('---');
      items.push({ icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => { const projects = findProjectsUsingPlugin(name); showReverseXrefModal(name, projects); } });
    }
    const plugin = typeof allPlugins !== 'undefined' && allPlugins.find(p => p.name === name);
    if (plugin) {
      const kvrUrl = plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer);
      items.push({ icon: '&#127760;', label: appFmt('menu.open_kvr'), ..._noEcho, action: () => window.vstUpdater.openUpdate(kvrUrl) });
      items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => openFolder(plugin.path) });
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
      { icon: '&#9654;', label: appFmt('menu.open_in_daw', { daw: daw || 'DAW' }), ..._noEcho, action: () => { showToast(toastFmt('toast.opening_name', { name })); window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.failed_dash', { err }), 4000, 'error')); } },
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openDawFolder === 'function' && openDawFolder(path) },
      '---',
      { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
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
      { icon: '&#128203;', label: appFmt('menu.copy_plugin_name'), ..._noEcho, action: () => copyToClipboard(name) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) },
      { icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openFolder === 'function' && openFolder(path) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Tab buttons ──
  const tabBtn = e.target.closest('.tab-btn');
  if (tabBtn) {
    const tab = tabBtn.dataset.tab;
    const exportMap = { plugins: 'exportPlugins', samples: 'exportAudio', daw: 'exportDaw', presets: 'exportPresets', pdf: 'exportPdfs', midi: 'exportMidi' };
    const scanMap = { plugins: 'scanPlugins', samples: 'scanAudioSamples', daw: 'scanDawProjects', presets: 'scanPresets', pdf: 'scanPdfs', midi: 'scanMidi' };
    /** Tab bar rescan: same labels as Scan menu / toolbar (not generic "Rescan Tab Data"). */
    const scanLabelKeyByTab = {
      plugins: 'menu.scan_plugins',
      samples: 'menu.scan_samples',
      daw: 'menu.scan_daw',
      presets: 'menu.scan_presets',
      pdf: 'menu.scan_pdf',
      midi: 'menu.scan_midi',
    };
    const items = [
      { icon: '&#8635;', label: appFmt('menu.switch_to_tab'), action: () => switchTab(tab) },
      '---',
    ];
    const scanFn = scanMap[tab];
    if (scanFn && typeof window[scanFn] === 'function') {
      const scanKey = scanLabelKeyByTab[tab] || 'menu.rescan_tab_data';
      const scanLabel = typeof appFmt === 'function' ? appFmt(scanKey) : scanKey;
      items.push({ icon: '&#9889;', label: scanLabel, action: () => window[scanFn]() });
    }
    const exportFn = exportMap[tab];
    if (exportFn && typeof window[exportFn] === 'function') {
      items.push({ icon: '&#8615;', label: appFmt('menu.export_tab_data'), action: () => window[exportFn]() });
    }
    if (scanFn || exportFn) items.push('---');
    items.push({ icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder() });
    showContextMenu(e, items);
    return;
  }

  // ── Tab nav bar (empty area) ──
  const tabNav = e.target.closest('.tab-nav');
  if (tabNav) {
    const items = [
      { icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder() },
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
      items.push({ icon: isOn ? '&#9711;' : '&#9679;', label: isOn ? appFmt('menu.turn_off') : appFmt('menu.turn_on'), action: () => toggle.click() });
    }
    if (textarea) {
      items.push({ icon: '&#10005;', label: appFmt('menu.clear'), ..._noEcho, action: () => { textarea.value = ''; } });
      items.push({ icon: '&#128203;', label: appFmt('menu.copy'), ..._noEcho, action: () => copyToClipboard(textarea.value) });
    }
    if (items.length === 0) return; // no special actions
    showContextMenu(e, items);
    return;
  }

  // ── Settings container (empty area) ──
  const settingsContainer = e.target.closest('.settings-container');
  if (settingsContainer) {
    const items = [
      { icon: '&#8596;', label: appFmt('menu.reset_columns'), action: () => settingResetColumns() },
      { icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder() },
      { icon: '&#128465;', label: appFmt('menu.clear_history'), action: () => settingClearAllHistory() },
      '---',
      { icon: '&#128206;', label: appFmt('menu.open_prefs_file'), action: () => openPrefsFile() },
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
        { icon: '&#128193;', label: appFmt('menu.open_directory'), ..._noEcho, action: () => openFolder(dirPath) },
        { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(dirPath) },
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
      { icon: '&#128203;', label: appFmt('menu.copy_stats'), ..._noEcho, action: () => copyToClipboard(statsText) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── File browser breadcrumbs ──
  const crumb = e.target.closest('.file-crumb');
  if (crumb) {
    const crumbPath = crumb.dataset.fileNav || '';
    const items = [
      { icon: '&#128193;', label: appFmt('menu.open_in_finder'), ..._noEcho, action: () => typeof openFolder === 'function' && openFolder(crumbPath) },
      { icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(crumbPath) },
      { icon: '&#9733;', label: appFmt('menu.bookmark_this_directory'), action: () => typeof addFavDir === 'function' && addFavDir(crumbPath) },
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
      items.push({ icon: '&#9654;', label: appFmt('menu.play'), ..._noEcho, action: () => typeof previewAudio === 'function' && previewAudio(path) });
      items.push({ icon: '&#128269;', label: appFmt('menu.show_in_samples_tab'), ..._noEcho, action: async () => {
        // If not in allAudioSamples, add it
        if (typeof allAudioSamples !== 'undefined' && !findByPath(allAudioSamples, path)) {
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
          } catch(e) { if(typeof showToast==='function'&&e) showToast(String(e),4000,'error'); }
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
      items.push({ icon: '&#128270;', label: appFmt('menu.find_similar'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
      items.push('---');
    }
    if (isDir) {
      items.push({ icon: '&#128193;', label: appFmt('menu.open_directory'), ..._noEcho, action: () => typeof loadDirectory === 'function' && loadDirectory(path) });
    }
    items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => {
      const dir = isDir ? path : path.replace(/\/[^/]+$/, '');
      if (typeof openFolder === 'function') openFolder(dir);
      else if (typeof openAudioFolder === 'function') openAudioFolder(path);
    }});
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, action: () => copyToClipboard(path) });
    items.push({ icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) });
    items.push('---');
    if (typeof isFavorite === 'function') {
      const fav = isFavorite(path);
      const favType = isDir ? 'folder' : 'file';
      items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
        action: () => { fav ? removeFavorite(path) : addFavorite(favType, path, name); if (typeof renderFileList === 'function') renderFileList(); } });
    }
    items.push({ icon: '&#128221;', label: appFmt('menu.add_note_tags'), action: () => { if (typeof showNoteEditor === 'function') showNoteEditor(path, name); } });
    if (isAudio) {
      items.push('---');
      const ap = prefs.getItem('autoplayNext') !== 'off';
      items.push({ icon: ap ? '&#9209;' : '&#9654;', label: ap ? appFmt('menu.disable_autoplay_next') : appFmt('menu.enable_autoplay_next'), ..._noEcho,
        action: () => { prefs.setItem('autoplayNext', ap ? 'off' : 'on'); if (typeof refreshSettingsUI === 'function') refreshSettingsUI(); showToast(ap ? toastFmt('toast.autoplay_next_disabled') : toastFmt('toast.autoplay_next_enabled')); } });
    }
    showContextMenu(e, items);
    return;
  }

  // ── Disk usage segments ──
  const diskSeg = e.target.closest('.disk-segment, .disk-legend-item');
  if (diskSeg) {
    const label = diskSeg.getAttribute('title') || diskSeg.textContent.trim();
    const items = [
      { icon: '&#128203;', label: appFmt('menu.copy'), ..._noEcho, action: () => copyToClipboard(label) },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── EQ/Gain/Pan sliders ──
  const eqSlider = e.target.closest('.eq-slider, .volume-slider');
  if (eqSlider) {
    const items = [
      { icon: '&#8634;', label: appFmt('menu.reset_eq_default'), action: () => { if (typeof resetEq === 'function') resetEq(); } },
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
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_field_label', { label }), ..._noEcho, action: () => copyToClipboard(val) });
    }
    if (path) {
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_file_path'), ..._noEcho, action: () => copyToClipboard(path) });
      items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openAudioFolder === 'function' && openAudioFolder(path) });
      items.push('---');
      items.push({ icon: '&#9654;', label: appFmt('menu.play'), ..._noEcho, action: () => typeof previewAudio === 'function' && previewAudio(path) });
      if (typeof isFavorite === 'function') {
        const fav = isFavorite(path);
        const name = metaPanel.querySelector('.meta-value')?.textContent || '';
        items.push({ icon: fav ? '&#9734;' : '&#9733;', label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho,
          action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name) });
      }
      items.push({ icon: '&#128221;', label: appFmt('menu.add_note'), action: () => { const name = metaPanel.querySelector('.meta-value')?.textContent || ''; typeof showNoteEditor === 'function' && showNoteEditor(path, name); } });
      items.push({ icon: '&#128270;', label: appFmt('menu.find_similar'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path) });
      items.push('---');
      items.push({ icon: '&#10005;', label: appFmt('menu.close_panel'), action: () => { const mr = document.getElementById('audioMetaRow'); if (mr) mr.remove(); expandedMetaPath = null; } });
    }
    if (items.length > 0) { showContextMenu(e, items); return; }
  }

  // ── Waveform ──
  const waveform = e.target.closest('.now-playing-waveform, .meta-waveform');
  if (waveform) {
    const items = [];
    if (typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_file_path'), ..._noEcho, action: () => copyToClipboard(audioPlayerPath) });
      items.push({ icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, action: () => typeof openAudioFolder === 'function' && openAudioFolder(audioPlayerPath) });
    }
    if (items.length > 0) { showContextMenu(e, items); return; }
  }

  // ── Shortcut keys ──
  const shortcutKey = e.target.closest('.shortcut-key');
  if (shortcutKey) {
    const scId = shortcutKey.dataset.shortcutId;
    const items = [
      { icon: '&#9881;', label: appFmt('menu.rebind_shortcut'), action: () => shortcutKey.click() },
      { icon: '&#8634;', label: appFmt('menu.reset_all_shortcuts'), action: () => typeof resetShortcuts === 'function' && resetShortcuts() },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Color scheme buttons ──
  const schemeBtn = e.target.closest('.scheme-btn');
  if (schemeBtn) {
    const scheme = schemeBtn.dataset.scheme;
    const items = [
      { icon: '&#127912;', label: appFmt('menu.apply_scheme', { scheme: scheme || 'scheme' }), ..._noEcho, action: () => schemeBtn.click() },
      { icon: '&#128203;', label: appFmt('menu.copy_scheme_name'), ..._noEcho, action: () => copyToClipboard(scheme || '') },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Progress bars ──
  const progressBar = e.target.closest('.audio-progress-bar, .global-progress, .progress-bar');
  if (progressBar) {
    const items = [
      { icon: '&#9632;', label: appFmt('menu.stop_all_scans'), action: () => typeof stopAll === 'function' && stopAll() },
    ];
    showContextMenu(e, items);
    return;
  }

  // Smart playlists section
  const spSection = e.target.closest('.np-smart-playlists-section');
  if (spSection && !e.target.closest('.sp-item')) {
    const items = [
      { icon: '&#127926;', label: appFmt('menu.new_smart_playlist'), action: () => typeof showSmartPlaylistEditor === 'function' && showSmartPlaylistEditor(null) },
      '---',
    ];
    if (typeof getSmartPlaylistPresets === 'function') {
      for (const preset of getSmartPlaylistPresets()) {
        items.push({ icon: '&#127925;', label: appFmt('menu.add_smart_playlist_named', { name: preset.name }), action: () => {
          if (typeof createSmartPlaylist === 'function') {
            const pl = createSmartPlaylist(preset.name, preset.rules);
            pl.matchMode = preset.matchMode;
            if (typeof saveSmartPlaylists === 'function') saveSmartPlaylists();
            showToast(toastFmt('toast.created_preset', { name: preset.name }));
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
      { icon: '&#9866;', label: appFmt('menu.minimize'), action: () => typeof minimizeSimilarPanel === 'function' && minimizeSimilarPanel() },
      { icon: '&#10005;', label: appFmt('menu.close'), action: () => typeof closeSimilarPanel === 'function' && closeSimilarPanel() },
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
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_quoted_label_val', { label, val }), ..._noEcho, action: () => copyToClipboard(`${label}: ${val}`) });
    }
    if (card) {
      const title = card.querySelector('.hm-card-title')?.textContent || '';
      items.push({ icon: '&#128203;', label: appFmt('menu.copy_tabular_title', { title }), ..._noEcho, action: () => {
        const rows = [...card.querySelectorAll('.hm-bar-row')].map(r => {
          const l = r.querySelector('.hm-bar-label')?.textContent || '';
          const v = r.querySelector('.hm-bar-val')?.textContent || '';
          return `${l}\t${v}`;
        }).join('\n');
        copyToClipboard(rows || title);
      }});
    }
    items.push('---');
    items.push({ icon: '&#8634;', label: appFmt('menu.refresh_dashboard'), action: () => { if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard(); } });
    items.push({ icon: '&#10005;', label: appFmt('menu.close_dashboard'), action: () => { if (typeof closeHeatmapDash === 'function') closeHeatmapDash(); } });
    showContextMenu(e, items);
    return;
  }

  // ── Walker tiles ──
  const walkerTile = e.target.closest('.walker-tile');
  if (walkerTile) {
    const body = walkerTile.querySelector('.walker-tile-body');
    const dirs = body ? [...body.querySelectorAll('.walker-dir')].map(d => d.textContent).join('\n') : '';
    const title = walkerTile.querySelector('.walker-tile-title, h4, h3')?.textContent?.trim()
      || (typeof appFmt === 'function' ? appFmt('menu.context_walker_fallback') : 'Walker');
    const items = [
      { icon: '&#128203;', label: appFmt('menu.copy_all_paths'), ..._noEcho, action: () => copyToClipboard(dirs), disabled: !dirs },
      { icon: '&#128203;', label: appFmt('menu.copy_tile_title'), ..._noEcho, action: () => copyToClipboard(title) },
      '---',
      { icon: '&#10005;', label: appFmt('menu.clear_tile'), action: () => { if (body) body.innerHTML = ''; showToast(toastFmt('toast.tile_cleared', { title })); } },
    ];
    showContextMenu(e, items);
    return;
  }

  // ── Visualizer tiles ── (handled in visualizer.js — single menu with mode-specific items)

  // ── Settings sections ──
  const settingsSection = e.target.closest('.settings-section');
  if (settingsSection) {
    const heading = settingsSection.querySelector('.settings-heading')?.textContent?.trim()
      || (typeof appFmt === 'function' ? appFmt('menu.context_settings_section_fallback') : 'Section');
    const items = [
      { icon: '&#128203;', label: appFmt('menu.copy_section_name'), ..._noEcho, action: () => typeof copyToClipboard === 'function' && copyToClipboard(heading) },
      { icon: '&#9650;', label: appFmt('menu.move_up'), action: () => {
        const prev = settingsSection.previousElementSibling;
        if (prev && prev.classList.contains('settings-section')) {
          settingsSection.parentNode.insertBefore(settingsSection, prev);
          showToast(toastFmt('toast.moved_heading_up', { heading }));
        }
      }},
      { icon: '&#9660;', label: appFmt('menu.move_down'), action: () => {
        const next = settingsSection.nextElementSibling;
        if (next && next.classList.contains('settings-section')) {
          next.parentNode.insertBefore(next, settingsSection);
          showToast(toastFmt('toast.moved_heading_down', { heading }));
        }
      }},
      '---',
      { icon: '&#128065;', label: settingsSection.classList.contains('collapsed') ? appFmt('menu.section_expand') : appFmt('menu.section_collapse'), action: () => {
        settingsSection.classList.toggle('collapsed');
        const body = settingsSection.querySelectorAll('.settings-row');
        body.forEach(r => r.style.display = settingsSection.classList.contains('collapsed') ? 'none' : '');
      }},
    ];
    showContextMenu(e, items);
    return;
  }

  } catch (err) { console.error('Context menu error:', err, err.stack); showToast(toastFmt('toast.context_menu_error', { err: err.message || err }), 4000, 'error'); }
});
