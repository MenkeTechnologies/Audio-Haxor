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
    { type: 'tab', name: appFmt('menu.tab_plugins'), icon: '&#9889;', action: () => switchTab('plugins') },
    { type: 'tab', name: appFmt('menu.tab_samples'), icon: '&#127925;', action: () => switchTab('samples') },
    { type: 'tab', name: appFmt('menu.tab_daw'), icon: '&#127911;', action: () => switchTab('daw') },
    { type: 'tab', name: appFmt('menu.tab_presets'), icon: '&#127924;', action: () => switchTab('presets') },
    { type: 'tab', name: appFmt('menu.tab_favorites'), icon: '&#9733;', action: () => switchTab('favorites') },
    { type: 'tab', name: appFmt('menu.tab_notes'), icon: '&#128221;', action: () => switchTab('notes') },
    { type: 'tab', name: appFmt('menu.tab_tags'), icon: '&#127991;', action: () => switchTab('tags') },
    { type: 'tab', name: appFmt('menu.tab_history'), icon: '&#128197;', action: () => switchTab('history') },
    { type: 'tab', name: appFmt('menu.tab_files'), icon: '&#128193;', action: () => switchTab('files') },
    { type: 'tab', name: appFmt('menu.tab_visualizer'), icon: '&#127911;', action: () => switchTab('visualizer') },
    { type: 'tab', name: appFmt('menu.tab_walkers'), icon: '&#128270;', action: () => switchTab('walkers') },
    { type: 'tab', name: appFmt('menu.tab_midi'), icon: '&#127924;', action: () => switchTab('midi') },
    { type: 'tab', name: appFmt('menu.tab_pdf'), icon: '&#128196;', action: () => switchTab('pdf') },
    { type: 'tab', name: appFmt('menu.tab_settings'), icon: '&#9881;', action: () => switchTab('settings') },
  ];
  items.push(...tabs);

  // Actions — all trigger toast confirmation
  items.push({ type: 'action', name: appFmt('menu.scan_plugins'), icon: '&#8635;', action: () => { showToast(toastFmt('toast.scanning_plugins')); scanPlugins(); } });
  items.push({ type: 'action', name: appFmt('menu.scan_samples'), icon: '&#8635;', action: () => { showToast(toastFmt('toast.scanning_samples')); scanAudioSamples(); } });
  items.push({ type: 'action', name: appFmt('menu.scan_daw'), icon: '&#8635;', action: () => { showToast(toastFmt('toast.scanning_daw_projects')); scanDawProjects(); } });
  items.push({ type: 'action', name: appFmt('menu.scan_presets'), icon: '&#8635;', action: () => { showToast(toastFmt('toast.scanning_presets')); scanPresets(); } });
  items.push({ type: 'action', name: appFmt('ui.btn.scan_pdfs'), icon: '&#8635;', action: () => { showToast(toastFmt('toast.scanning_pdfs_progress')); scanPdfs(); } });
  items.push({ type: 'action', name: appFmt('menu.stop_pdf_scan'), icon: '&#9632;', action: () => { if (typeof stopPdfScan === 'function') stopPdfScan(); } });
  items.push({ type: 'action', name: appFmt('menu.export_pdfs'), icon: '&#8615;', action: () => { if (typeof exportPdfs === 'function' && typeof runExport === 'function') runExport(exportPdfs); else if (typeof exportPdfs === 'function') exportPdfs(); } });
  items.push({ type: 'action', name: appFmt('menu.import_pdfs'), icon: '&#8613;', action: () => { if (typeof importPdfs === 'function') importPdfs(); } });
  items.push({ type: 'action', name: appFmt('menu.extract_pdf_page_counts'), icon: '&#128196;', action: () => { if (typeof buildPdfPagesCache === 'function') buildPdfPagesCache(); } });
  items.push({ type: 'action', name: appFmt('menu.build_fingerprint_cache'), icon: '&#127925;', action: () => {
    const paths = (typeof allAudioSamples !== 'undefined' ? allAudioSamples : []).map(s => s.path);
    if (paths.length === 0) { showToast(toastFmt('toast.no_audio_samples_scan_first'), 4000, 'error'); return; }
    showToast(toastFmt('toast.fingerprint_building_n', { n: paths.length.toLocaleString() }), 4000);
    window.vstUpdater.buildFingerprintCache(paths)
      .then(res => showToast(toastFmt('toast.fingerprint_build_complete_n', { n: res.built.toLocaleString() })))
      .catch(e => showToast(toastFmt('toast.fingerprint_build_failed', { err: e.message || e }), 4000, 'error'));
  } });
  items.push({ type: 'action', name: appFmt('menu.check_updates'), icon: '&#9889;', action: () => { showToast(toastFmt('toast.checking_updates')); checkUpdates(); } });
  items.push({ type: 'action', name: appFmt('menu.find_duplicates'), icon: '&#128270;', action: () => { showToast(toastFmt('toast.scanning_duplicates')); showDuplicateReport(); } });
  items.push({ type: 'action', name: appFmt('menu.reset_all_scans'), icon: '&#128465;', action: () => { showToast(toastFmt('toast.resetting_scans')); resetAllScans(); } });
  if (typeof buildXrefIndex === 'function') {
    items.push({ type: 'action', name: appFmt('menu.build_plugin_index'), icon: '&#9889;', action: () => { showToast(toastFmt('toast.building_plugin_index')); buildXrefIndex(); } });
  }
  if (typeof showDepGraph === 'function') {
    items.push({ type: 'action', name: appFmt('menu.dep_graph'), icon: '&#128200;', action: () => { showToast(toastFmt('toast.opening_dep_graph')); showDepGraph(); } });
  }
  if (typeof findSimilarSamples === 'function' && typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
    items.push({ type: 'action', name: appFmt('menu.find_similar_current'), icon: '&#128270;', action: () => { showToast(toastFmt('toast.finding_similar')); findSimilarSamples(audioPlayerPath); } });
  }
  if (typeof showPlayer === 'function') {
    const np = document.getElementById('audioNowPlaying');
    const visible = np && np.classList.contains('active');
    items.push({ type: 'action', name: visible ? appFmt('menu.hide_audio_player') : appFmt('menu.show_audio_player'), icon: '&#9835;', action: () => { visible ? hidePlayer() : showPlayer(); showToast(visible ? toastFmt('toast.player_hidden') : toastFmt('toast.player_shown')); } });
  }
  if (typeof showHeatmapDashboard === 'function') {
    items.push({ type: 'action', name: appFmt('menu.heatmap_dashboard'), icon: '&#128202;', action: () => { showToast(toastFmt('toast.opening_dashboard')); showHeatmapDashboard(); } });
  }
  if (typeof showSmartPlaylistEditor === 'function') {
    items.push({ type: 'action', name: appFmt('menu.new_smart_playlist'), icon: '&#127926;', action: () => { showToast(toastFmt('toast.creating_smart_playlist')); showSmartPlaylistEditor(null); } });
  }
  if (typeof exportSettingsPdf === 'function') {
    items.push({ type: 'action', name: appFmt('menu.export_settings_keybindings'), icon: '&#128196;', action: () => { showToast(toastFmt('toast.exporting_settings_pdf')); exportSettingsPdf(); } });
  }
  if (typeof exportLogPdf === 'function') {
    items.push({ type: 'action', name: appFmt('menu.export_app_log'), icon: '&#128196;', action: () => { showToast(toastFmt('toast.exporting_log')); exportLogPdf(); } });
  }
  if (typeof exportMidi === 'function') {
    items.push({ type: 'action', name: appFmt('menu.export_midi_files'), icon: '&#127924;', action: () => { showToast(toastFmt('toast.exporting_midi')); if (typeof runExport === 'function') runExport(exportMidi); else exportMidi(); } });
  }
  if (typeof exportXref === 'function') {
    items.push({ type: 'action', name: appFmt('menu.export_plugin_xref'), icon: '&#9889;', action: () => { showToast(toastFmt('toast.exporting_xref')); exportXref(); } });
  }
  if (typeof exportSmartPlaylists === 'function') {
    items.push({ type: 'action', name: appFmt('menu.export_smart_playlists'), icon: '&#127926;', action: () => { showToast(toastFmt('toast.exporting_playlists')); exportSmartPlaylists(); } });
  }
  items.push({ type: 'action', name: appFmt('menu.clear_all_caches'), icon: '&#128465;', action: () => {
    showToast(toastFmt('toast.clearing_caches'));
    window.vstUpdater.dbClearCaches().then(() => {
      if (typeof _bpmCache !== 'undefined') { _bpmCache = {}; _keyCache = {}; _lufsCache = {}; }
      if (typeof _waveformCache !== 'undefined') { _waveformCache = {}; _spectrogramCache = {}; }
      showToast(toastFmt('toast.all_caches_cleared'));
    }).catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error'));
  }});
  if (typeof settingToggleTheme === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_dark_light_theme'), icon: '&#127912;', action: () => settingToggleTheme() });
  }
  items.push({ type: 'action', name: appFmt('menu.scan_all'), icon: '&#9889;', action: () => { showToast(toastFmt('toast.scanning_all')); typeof scanAll === 'function' && scanAll(); } });
  items.push({ type: 'action', name: appFmt('menu.stop_all_scans'), icon: '&#9632;', action: () => { showToast(toastFmt('toast.stopping_scans')); typeof stopAll === 'function' && stopAll(); } });
  items.push({ type: 'action', name: appFmt('menu.export_current_tab'), icon: '&#8615;', action: () => { typeof _exportCurrentTab === 'function' && _exportCurrentTab(); } });
  items.push({ type: 'action', name: appFmt('menu.import_to_current_tab'), icon: '&#8613;', action: () => { typeof _importCurrentTab === 'function' && _importCurrentTab(); } });
  items.push({ type: 'action', name: appFmt('menu.help_keyboard_shortcuts'), icon: '&#10068;', action: () => { typeof toggleHelpOverlay === 'function' && toggleHelpOverlay(); } });
  items.push({ type: 'action', name: appFmt('menu.open_log_file'), icon: '&#128196;', action: () => { showToast(toastFmt('toast.opening_log')); window.vstUpdater.getPrefsPath().then(p => { const lp = p.replace(/preferences\.toml$/, 'app.log'); window.vstUpdater.openWithApp(lp, 'TextEdit').catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }); }); } });

  // Toggles
  if (typeof settingToggleCrt === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_crt'), icon: '&#128187;', action: () => settingToggleCrt() });
  if (typeof settingToggleNeonGlow === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_neon_glow'), icon: '&#10024;', action: () => settingToggleNeonGlow() });
  if (typeof settingToggleAutoAnalysis === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_auto_analyze_startup'), icon: '&#127925;', action: () => settingToggleAutoAnalysis() });
  if (typeof settingToggleAutoScan === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_auto_scan_launch'), icon: '&#8635;', action: () => settingToggleAutoScan() });
  if (typeof settingToggleAutoUpdate === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_auto_check_updates'), icon: '&#9889;', action: () => settingToggleAutoUpdate() });
  if (typeof settingToggleFolderWatch === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_folder_watch'), icon: '&#128065;', action: () => settingToggleFolderWatch() });
  if (typeof settingToggleSingleClickPlay === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_single_click_play'), icon: '&#9654;', action: () => settingToggleSingleClickPlay() });
  if (typeof settingToggleAutoPlaySampleOnSelect === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_play_sample_on_keyboard_select'), icon: '&#9835;', action: () => settingToggleAutoPlaySampleOnSelect() });
  if (typeof settingToggleAutoplayNext === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_autoplay_next'), icon: '&#9197;', action: () => settingToggleAutoplayNext() });
  if (typeof settingToggleShowPlayerOnStartup === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_show_player_startup'), icon: '&#9835;', action: () => settingToggleShowPlayerOnStartup() });
  if (typeof settingToggleExpandOnClick === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_expand_on_click'), icon: '&#8597;', action: () => settingToggleExpandOnClick() });
  if (typeof settingToggleIncludeBackups === 'function') items.push({ type: 'action', name: appFmt('menu.toggle_include_ableton_backups'), icon: '&#128190;', action: () => settingToggleIncludeBackups() });

  // Resets & Clears
  if (typeof resetTabOrder === 'function') items.push({ type: 'action', name: appFmt('menu.reset_tabs'), icon: '&#8634;', action: () => { resetTabOrder(); showToast(toastFmt('toast.tab_order_reset')); } });
  if (typeof resetSettingsSectionOrder === 'function') items.push({ type: 'action', name: appFmt('menu.reset_settings_layout'), icon: '&#8634;', action: () => { resetSettingsSectionOrder(); showToast(toastFmt('toast.settings_layout_reset')); } });
  if (typeof resetFzfParams === 'function') items.push({ type: 'action', name: appFmt('menu.reset_search_weights'), icon: '&#8634;', action: () => { resetFzfParams(); showToast(toastFmt('toast.search_weights_reset')); } });
  if (typeof settingResetAllUI === 'function') items.push({ type: 'action', name: appFmt('menu.reset_all_ui_layout'), icon: '&#9888;', action: () => { settingResetAllUI(); showToast(toastFmt('toast.all_ui_layout_reset')); } });
  if (typeof settingResetColumns === 'function') items.push({ type: 'action', name: appFmt('menu.reset_columns'), icon: '&#8634;', action: () => { settingResetColumns(); showToast(toastFmt('toast.column_widths_reset')); } });
  if (typeof settingClearAllHistory === 'function') items.push({ type: 'action', name: appFmt('menu.clear_all_scan_history'), icon: '&#128465;', action: () => { settingClearAllHistory(); showToast(toastFmt('toast.all_history_cleared')); } });
  if (typeof settingClearAllDatabases === 'function') items.push({ type: 'action', name: appFmt('menu.clear_all_databases'), icon: '&#128465;', action: () => settingClearAllDatabases() });
  if (typeof settingClearKvrCache === 'function') items.push({ type: 'action', name: appFmt('menu.clear_kvr'), icon: '&#128465;', action: () => { settingClearKvrCache(); showToast(toastFmt('toast.kvr_cache_cleared_palette')); } });
  items.push({ type: 'action', name: appFmt('menu.clear_app_log'), icon: '&#128465;', action: () => { window.vstUpdater.clearLog().then(() => showToast(toastFmt('toast.log_cleared'))).catch(() => showToast(toastFmt('toast.failed_clear_log'), 4000, 'error')); } });
  items.push({ type: 'action', name: appFmt('menu.preferences'), icon: '&#128196;', action: () => { showToast(toastFmt('toast.opening_preferences')); typeof openPrefsFile === 'function' && openPrefsFile(); } });
  items.push({ type: 'action', name: appFmt('menu.open_data_directory'), icon: '&#128193;', action: () => { showToast(toastFmt('toast.opening_data_dir')); window.vstUpdater.getPrefsPath().then(p => { const dir = p.replace(/[/\\][^/\\]+$/, ''); window.vstUpdater.openPluginFolder(dir); }); } });
  if (typeof clearRecentlyPlayed === 'function') items.push({ type: 'action', name: appFmt('menu.clear_play_history'), icon: '&#128465;', action: () => clearRecentlyPlayed() });
  if (typeof clearFavorites === 'function') items.push({ type: 'action', name: appFmt('menu.clear_favorites'), icon: '&#128465;', action: () => clearFavorites() });
  if (typeof clearAllNotes === 'function') items.push({ type: 'action', name: appFmt('menu.clear_all_notes_tags'), icon: '&#128465;', action: () => clearAllNotes() });
  items.push({ type: 'action', name: appFmt('menu.focus_search'), icon: '&#128269;', action: () => { const tab = document.querySelector('.tab-content.active'); const input = tab?.querySelector('input[type="text"]'); if (input) { input.focus(); input.select(); } } });

  // Player controls
  if (typeof toggleAudioPlayback === 'function') {
    items.push({ type: 'action', name: appFmt('menu.play_pause'), icon: '&#9654;', action: () => toggleAudioPlayback() });
  }
  if (typeof nextTrack === 'function') {
    items.push({ type: 'action', name: appFmt('menu.next_track'), icon: '&#9193;', action: () => nextTrack() });
  }
  if (typeof prevTrack === 'function') {
    items.push({ type: 'action', name: appFmt('menu.prev_track'), icon: '&#9194;', action: () => prevTrack() });
  }
  if (typeof toggleAudioLoop === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_loop'), icon: '&#128257;', action: () => toggleAudioLoop() });
  }
  if (typeof toggleShuffle === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_shuffle'), icon: '&#128256;', action: () => toggleShuffle() });
  }
  if (typeof toggleMute === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_mute'), icon: '&#128263;', action: () => toggleMute() });
  }
  if (typeof toggleMono === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_mono'), icon: '&#127897;', action: () => toggleMono() });
  }
  if (typeof toggleEqSection === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_eq_panel'), icon: '&#127900;', action: () => toggleEqSection() });
  }
  if (typeof togglePlayerExpanded === 'function') {
    items.push({ type: 'action', name: appFmt('menu.expand_player'), icon: '&#9744;', action: () => togglePlayerExpanded() });
  }
  if (typeof setAbLoopStart === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_ab_loop'), icon: '&#128260;', action: () => {
      if (typeof _abLoop !== 'undefined' && _abLoop) { if (typeof clearAbLoop === 'function') clearAbLoop(); }
      else { setAbLoopStart(); }
    }});
  }

  // Selection
  if (typeof selectAllVisible === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_select_all_visible'), icon: '&#9745;', action: () => selectAllVisible() });
  }
  if (typeof deselectAll === 'function') {
    items.push({ type: 'action', name: appFmt('menu.toggle_deselect_all'), icon: '&#9744;', action: () => deselectAll() });
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
        type: 'tag', name: t, detail: catalogFmt('ui.palette.type_tag'),
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
    if (typeof allPdfs !== 'undefined') dataSearch(allPdfs.slice(0, 5000), 'pdf', '&#128196;', p => ({ name: p.name, detail: p.sizeFormatted || '', fields: [p.name, p.directory || ''], action: () => { switchTab('pdf'); setTimeout(() => { const el = document.getElementById('pdfSearchInput'); if (el) { el.value = p.name; filterPdfs(); } }, 100); } }));
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, PALETTE_MAX).map(s => s.item);
}

function openPalette() {
  if (_paletteOpen) return;
  _paletteOpen = true;
  _paletteQuery = '';
  _paletteSelected = 0;

  const ph = catalogFmt('ui.palette.placeholder');
  const html = `<div class="palette-overlay" id="paletteOverlay">
    <div class="palette-box">
      <input type="text" class="palette-input" id="paletteInput" placeholder="" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">
      <div class="palette-results" id="paletteResults"></div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);

  const input = document.getElementById('paletteInput');
  if (input) input.placeholder = ph;
  if (input) input.focus();
  renderPaletteResults();

  let _palTimer;
  if (input) {
    input.addEventListener('input', () => {
      _paletteQuery = input.value;
      _paletteSelected = 0;
      clearTimeout(_palTimer);
      _palTimer = setTimeout(renderPaletteResults, 150);
    });
  }
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
    const empty = catalogFmt('ui.palette.empty');
    container.innerHTML = '<div class="palette-empty">' + escapeHtml(empty) + '</div>';
    return;
  }

  container.innerHTML = _paletteResults.map((item, i) => {
    const typeCls = 'palette-type-' + item.type;
    const sel = i === _paletteSelected ? ' palette-selected' : '';
    const typeLabel = ({
      tab: catalogFmt('ui.palette.type_tab'),
      action: catalogFmt('ui.palette.type_action'),
      plugin: catalogFmt('ui.palette.type_plugin'),
      sample: catalogFmt('ui.palette.type_sample'),
      daw: catalogFmt('ui.palette.type_daw'),
      pdf: catalogFmt('ui.palette.type_pdf'),
      preset: catalogFmt('ui.palette.type_preset'),
      bookmark: catalogFmt('ui.palette.type_bookmark'),
      tag: catalogFmt('ui.palette.type_tag'),
    })[item.type] || item.type;
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

// Hover to highlight — swap class on old/new row only (no querySelectorAll)
document.addEventListener('mousemove', (e) => {
  if (!_paletteOpen) return;
  const row = e.target.closest('[data-palette-idx]');
  if (row) {
    const idx = parseInt(row.dataset.paletteIdx, 10);
    if (idx !== _paletteSelected) {
      const prev = document.querySelector('.palette-row.palette-selected');
      if (prev) prev.classList.remove('palette-selected');
      row.classList.add('palette-selected');
      _paletteSelected = idx;
    }
  }
});
