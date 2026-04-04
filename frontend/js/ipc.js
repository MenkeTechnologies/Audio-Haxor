// Tauri IPC bridge — replaces Electron's preload.js window.vstUpdater API
const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// App i18n — strings loaded from SQLite via get_app_strings (seeded from i18n/app_i18n_en.json at build).
window.__appStr = {};
window.__toastStr = window.__appStr;
function appFmt(key, vars) {
  const map = window.__appStr;
  let s = map && map[key];
  if (s == null || s === '') return key;
  if (vars && typeof vars === 'object') {
    s = s.replace(/\{(\w+)\}/g, (_, name) => (vars[name] != null && vars[name] !== '') ? String(vars[name]) : '');
  }
  return s;
}
window.appFmt = appFmt;
window.toastFmt = appFmt;
window.__appReady = invoke('get_app_strings', { locale: null }).then((m) => {
  window.__appStr = m || {};
  window.__toastStr = window.__appStr;
  if (typeof applyUiI18n === 'function') applyUiI18n();
}).catch(() => {});
window.__toastReady = window.__appReady;

async function reloadAppStrings(locale) {
  const loc =
    locale === 'de' ||
    locale === 'es' ||
    locale === 'sv' ||
    locale === 'fr' ||
    locale === 'pt' ||
    locale === 'nl' ||
    locale === 'en'
      ? locale
      : null;
  try {
    const m = await invoke('get_app_strings', { locale: loc });
    window.__appStr = m || {};
    window.__toastStr = window.__appStr;
    if (typeof applyUiI18n === 'function') applyUiI18n();
    if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
    if (typeof renderShortcutSettings === 'function') {
      const sf = document.getElementById('shortcutsFilter');
      renderShortcutSettings(sf && sf.value ? sf.value : '');
    }
    try {
      await invoke('refresh_native_menu');
    } catch (_) {}
  } catch (_) {}
}
window.reloadAppStrings = reloadAppStrings;

// ── Menu bar event handler ──
listen('menu-action', (event) => {
  const id = event.payload;
  switch (id) {
    // File
    case 'scan_all': scanAll(); break;
    case 'stop_all': stopAll(); break;
    case 'export_plugins': exportPlugins(); break;
    case 'import_plugins': importPlugins(); break;
    case 'export_audio': exportAudio(); break;
    case 'import_audio': importAudio(); break;
    case 'export_daw': exportDaw(); break;
    case 'import_daw': importDaw(); break;
    case 'export_presets': exportPresets(); break;
    case 'import_presets': importPresets(); break;
    case 'open_prefs': openPrefsFile(); break;
    case 'open_prefs_app': switchTab('settings'); break;
    // Scan
    case 'scan_plugins': scanPlugins(); break;
    case 'scan_audio': scanAudioSamples(); break;
    case 'scan_daw': scanDawProjects(); break;
    case 'scan_presets': scanPresets(); break;
    case 'check_updates': checkUpdates(); break;
    // View — tabs
    case 'tab_plugins': switchTab('plugins'); break;
    case 'tab_samples': switchTab('samples'); break;
    case 'tab_daw': switchTab('daw'); break;
    case 'tab_presets': switchTab('presets'); break;
    case 'tab_favorites': switchTab('favorites'); break;
    case 'tab_notes': switchTab('notes'); break;
    case 'tab_history': switchTab('history'); break;
    case 'tab_settings': switchTab('settings'); break;
    case 'tab_files': switchTab('files'); break;
    // View — appearance
    case 'toggle_theme': settingToggleTheme(); break;
    case 'toggle_crt': settingToggleCrt(); break;
    case 'reset_columns': settingResetColumns(); break;
    case 'reset_tabs': settingResetTabOrder(); break;
    // Data
    case 'clear_history': settingClearAllHistory(); break;
    case 'clear_kvr': settingClearKvrCache(); break;
    case 'clear_favorites': clearFavorites(); break;
    case 'reset_all': resetAllScans(); break;
    // Playback
    case 'play_pause': toggleAudioPlayback(); break;
    case 'toggle_loop': toggleAudioLoop(); break;
    case 'stop_playback': stopAudioPlayback(); break;
    case 'expand_player': togglePlayerExpanded(); break;
    case 'next_track': nextTrack(); break;
    case 'prev_track': prevTrack(); break;
    case 'toggle_shuffle': toggleShuffle(); break;
    case 'toggle_mute': toggleMute(); break;
    // Tools
    case 'find_duplicates': showDuplicateReport(); break;
    case 'dep_graph': showDepGraph(); break;
    case 'cmd_palette': openPalette(); break;
    case 'help_overlay': toggleHelpOverlay(); break;
    // Help
    case 'github': showToast(toastFmt('toast.opening_github')); openUpdate('https://github.com/MenkeTechnologies/universal-plugin-update-manager'); break;
    case 'docs': showToast(toastFmt('toast.opening_docs')); openUpdate('https://menketechnologies.github.io/universal-plugin-update-manager/'); break;
    // Find (handled by existing Cmd+F)
    case 'find': {
      const activeTab = document.querySelector('.tab-content.active');
      const input = activeTab?.querySelector('input[type="text"]');
      if (input) { input.focus(); input.select(); }
      break;
    }
  }
});

// Event delegation — replaces inline onclick/oninput/onchange for Tauri v2 CSP compatibility
document.addEventListener('click', (e) => {
  if (e.target.closest('.col-resize')) return;
  const el = e.target.closest('[data-action]');
  if (!el) return;
  // If there's a data-action-stop container between the target and the matched action element, skip parent actions
  if (el.dataset.action === 'toggleMetadata') {
    const stop = e.target.closest('[data-action-stop]');
    if (stop && el.contains(stop)) return;
  }
  const action = el.dataset.action;
  try { switch (action) {
    case 'stopCurrentOperation': stopCurrentOperation(); break;
    case 'scanAll': scanAll(); break;
    case 'stopAll': stopAll(); break;
    case 'resumeAll': resumeAll(); break;
    case 'scanPlugins': scanPlugins(); break;
    case 'resumePluginScan': scanPlugins(true); break;
    case 'stopPluginScan': window.vstUpdater.stopScan(); break;
    case 'checkUpdates': checkUpdates(); break;
    case 'switchTab': switchTab(el.dataset.tab); break;
    case 'skipUpdate': skipUpdate(); break;
    case 'openNextUpdate': openNextUpdate(); break;
    case 'toggleDirs': toggleDirs(); break;
    case 'clearAllHistory': clearAllHistory(); break;
    case 'scanAudioSamples': scanAudioSamples(); break;
    case 'resumeAudioScan': scanAudioSamples(true); break;
    case 'stopAudioScan': stopAudioScan(); break;
    case 'toggleAudioPlayback': toggleAudioPlayback(); break;
    case 'toggleAudioLoop': toggleAudioLoop(); break;
    case 'seekAudio': seekAudio(e); break;
    case 'seekMetaWaveform': seekMetaWaveform(e); break;
    case 'stopAudioPlayback': stopAudioPlayback(); break;
    case 'openUpdate': showToast(toastFmt('toast.opening_link')); openUpdate(el.dataset.url); break;
    case 'openKvr': openKvr(el, el.dataset.url, el.dataset.name); break;
    case 'openFolder': openFolder(el.dataset.path); break;
    case 'openAudioFolder': openAudioFolder(el.dataset.path); break;
    case 'selectScan': selectScan(el.dataset.id, el.dataset.type); break;
    case 'runDiff': runDiff(el.dataset.id); break;
    case 'runAudioDiff': runAudioDiff(el.dataset.id); break;
    case 'deleteScanEntry': deleteScanEntry(el.dataset.id); break;
    case 'deleteAudioScanEntry': deleteAudioScanEntry(el.dataset.id); break;
    case 'sortAudio': sortAudio(el.dataset.key); break;
    case 'loadMoreAudio': loadMoreAudio(); break;
    case 'loadMorePlugins': if (typeof loadMorePlugins === 'function') loadMorePlugins(); break;
    case 'loadMoreMidi': if (typeof loadMoreMidi === 'function') loadMoreMidi(); break;
    case 'loadMoreFavs': if (typeof loadMoreFavs === 'function') loadMoreFavs(); break;
    case 'toggleMetadata': toggleMetadata(el.dataset.path, e); break;
    case 'previewAudio': previewAudio(el.dataset.path); break;
    case 'toggleRowLoop': toggleRowLoop(el.dataset.path, e); break;
    case 'scanDawProjects': scanDawProjects(); break;
    case 'resumeDawScan': scanDawProjects(true); break;
    case 'stopDawScan': stopDawScan(); break;
    case 'buildXrefIndex': buildXrefIndex().then(() => filterDawProjects()); break;
    case 'showDepGraph': showDepGraph(); break;
    case 'showHeatmapDash': if (typeof showHeatmapDashboard === 'function') showHeatmapDashboard(); break;
    case 'scanPresets': scanPresets(); break;
    case 'resumePresetScan': scanPresets(true); break;
    case 'stopPresetScan': stopPresetScan(); break;
    case 'openPresetFolder': openPresetFolder(el.dataset.path); break;
    case 'sortPreset': sortPreset(el.dataset.key); break;
    case 'loadMorePresets': loadMorePresets(); break;
    case 'openDawFolder': openDawFolder(el.dataset.path); break;
    case 'sortDaw': sortDaw(el.dataset.key); break;
    case 'loadMoreDaw': loadMoreDaw(); break;
    case 'runDawDiff': runDawDiff(el.dataset.id); break;
    case 'deleteDawScanEntry': deleteDawScanEntry(el.dataset.id); break;
    case 'runPresetDiff': runPresetDiff(el.dataset.id); break;
    case 'deletePresetScanEntry': deletePresetScanEntry(el.dataset.id); break;
    case 'exportPlugins': exportPlugins(); break;
    case 'importPlugins': importPlugins(); break;
    case 'exportAudio': exportAudio(); break;
    case 'importAudio': importAudio(); break;
    case 'exportDaw': exportDaw(); break;
    case 'exportXrefPlugins': if (typeof exportXrefPlugins === 'function') exportXrefPlugins(); break;
    case 'importDaw': importDaw(); break;
    case 'exportPresets': exportPresets(); break;
    case 'importPresets': importPresets(); break;
    case 'exportMidi': if (typeof exportMidi === 'function') exportMidi(); break;
    case 'exportXref': if (typeof exportXref === 'function') exportXref(); break;
    case 'exportSmartPlaylists': if (typeof exportSmartPlaylists === 'function') exportSmartPlaylists(); break;
    case 'settingToggleTheme': settingToggleTheme(); break;
    case 'settingToggleCrt': settingToggleCrt(); break;
    case 'settingToggleNeonGlow': settingToggleNeonGlow(); break;
    case 'settingToggleTagBar': {
      const current = prefs.getItem('tagBarVisible') !== 'off';
      prefs.setItem('tagBarVisible', current ? 'off' : 'on');
      const bar = document.getElementById('globalTagBar');
      if (bar && current) bar.style.display = 'none';
      showToast(current ? toastFmt('toast.tag_bar_hidden') : toastFmt('toast.tag_bar_show_when_active'));
      if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
    } break;
    case 'settingTagBarPosition': {
      const pos = document.getElementById('settingTagBarPosition')?.value || 'top';
      prefs.setItem('tagBarPosition', pos);
      const bar = document.getElementById('globalTagBar');
      const tabNav = document.querySelector('.tab-nav');
      if (bar && tabNav) {
        if (pos === 'bottom') {
          const lastTab = [...document.querySelectorAll('.tab-content')].pop();
          if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
        } else {
          tabNav.parentNode.insertBefore(bar, tabNav);
        }
      }
      showToast(toastFmt('toast.tag_bar_moved', { pos }));
    } break;
    case 'clearFavorites': clearFavorites(); break;
    case 'exportFavorites': exportFavorites(); break;
    case 'importFavorites': importFavorites(); break;
    case 'exportNotes': exportNotes(); break;
    case 'importNotes': importNotes(); break;
    case 'clearAllNotes': clearAllNotes(); break;
    case 'clearGlobalTag': clearGlobalTag(); break;
    case 'hideTagBar': {
      const bar = document.getElementById('globalTagBar');
      if (bar) bar.style.display = 'none';
      prefs.setItem('tagBarVisible', 'off');
      showToast(toastFmt('toast.tag_bar_hidden_filter'));
    } break;
    case 'moveTagBar': {
      const bar = document.getElementById('globalTagBar');
      if (!bar) break;
      const main = bar.parentNode;
      const tabNav = document.querySelector('.tab-nav');
      const isTop = bar.compareDocumentPosition(tabNav) & Node.DOCUMENT_POSITION_FOLLOWING;
      if (isTop) {
        // Move to bottom (after tab content area)
        const lastTab = [...document.querySelectorAll('.tab-content')].pop();
        if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
        prefs.setItem('tagBarPosition', 'bottom');
        showToast(toastFmt('toast.tag_bar_bottom'));
      } else {
        // Move to top (before tab nav)
        if (tabNav) tabNav.parentNode.insertBefore(bar, tabNav);
        prefs.setItem('tagBarPosition', 'top');
        showToast(toastFmt('toast.tag_bar_top'));
      }
    } break;
    case 'settingResetAllUI': settingResetAllUI(); break;
    case 'settingResetColumns': settingResetColumns(); break;
    case 'settingResetSectionOrder': resetSettingsSectionOrder(); break;
    case 'settingResetTabOrder': settingResetTabOrder(); break;
    case 'settingClearAllHistory': settingClearAllHistory(); break;
    case 'settingClearKvrCache': settingClearKvrCache(); break;
    case 'settingClearAnalysisCache': window.vstUpdater.dbClearCaches().then(() => { if (typeof _bpmCache !== 'undefined') { _bpmCache = {}; _keyCache = {}; _lufsCache = {}; } if (typeof _waveformCache !== 'undefined') { _waveformCache = {}; _spectrogramCache = {}; } showToast(toastFmt('toast.all_caches_cleared')); if (typeof renderCacheStats === 'function') renderCacheStats(); }).catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error')); break;
    case 'clearCacheTable': { const c = el.dataset.cache; if (c) window.vstUpdater.dbClearCacheTable(c).then(() => { if (c === 'bpm' && typeof _bpmCache !== 'undefined') _bpmCache = {}; if (c === 'key' && typeof _keyCache !== 'undefined') _keyCache = {}; if (c === 'lufs' && typeof _lufsCache !== 'undefined') _lufsCache = {}; if (c === 'waveform' && typeof _waveformCache !== 'undefined') _waveformCache = {}; if (c === 'spectrogram' && typeof _spectrogramCache !== 'undefined') _spectrogramCache = {}; showToast(toastFmt('toast.cache_type_cleared', { cache: c.toUpperCase() })); if (typeof renderCacheStats === 'function') renderCacheStats(); }).catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error')); } break;
    case 'exportSettingsPdf': if (typeof exportSettingsPdf === 'function') exportSettingsPdf(); break;
    case 'exportLogPdf': if (typeof exportLogPdf === 'function') exportLogPdf(); break;
    case 'clearAppLog': window.vstUpdater.clearLog().then(() => showToast(toastFmt('toast.log_cleared'))).catch(() => showToast(toastFmt('toast.failed_clear_log'), 4000, 'error')); break;
    case 'openLogFile': showToast(toastFmt('toast.opening_log_file')); window.vstUpdater.getPrefsPath().then(p => { const logPath = p.replace(/preferences\.toml$/, 'app.log'); window.vstUpdater.openWithApp(logPath, 'TextEdit').catch(() => window.vstUpdater.openDawProject(logPath).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); })); }); break;
    case 'openDataDir': showToast(toastFmt('toast.opening_data_dir')); window.vstUpdater.getPrefsPath().then(p => { const dir = p.replace(/[/\\][^/\\]+$/, ''); window.vstUpdater.openPluginFolder(dir).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }); }); break;
    case 'refreshCacheList': if (typeof renderCacheFilesList === 'function') { renderCacheFilesList(); showToast(toastFmt('toast.cache_list_refreshed')); } break;
    case 'refreshCacheStats': if (typeof renderCacheStats === 'function') { renderCacheStats(); showToast(toastFmt('toast.cache_stats_refreshed')); } break;
    case 'revealDataFile': if (el.dataset.path) { showToast(toastFmt('toast.revealing_file')); window.vstUpdater.openAudioFolder(el.dataset.path).catch(() => showToast(toastFmt('toast.failed_reveal_file'), 4000, 'error')); } break;
    case 'deleteDataFile': if (el.dataset.name && confirm(appFmt('confirm.delete_data_file', { name: el.dataset.name }))) { window.vstUpdater.deleteDataFile(el.dataset.name).then(() => { showToast(toastFmt('toast.deleted_name', { name: el.dataset.name })); if (typeof renderCacheFilesList === 'function') renderCacheFilesList(); }).catch(e => showToast(toastFmt('toast.delete_failed', { err: e }), 4000, 'error')); } break;
    case 'resetAllScans': resetAllScans(); break;
    case 'settingColorScheme': settingColorScheme(el.dataset.scheme); break;
    case 'settingToggleAutoScan': settingToggleAutoScan(); break;
    case 'settingToggleFolderWatch': settingToggleFolderWatch(); break;
    case 'settingToggleAutoUpdate': settingToggleAutoUpdate(); break;
    case 'settingToggleSingleClickPlay': settingToggleSingleClickPlay(); break;
    case 'settingToggleExpandOnClick': settingToggleExpandOnClick(); break;
    case 'settingToggleShowPlayerOnStartup': settingToggleShowPlayerOnStartup(); break;
    case 'settingToggleAutoplayNext': settingToggleAutoplayNext(); break;
    case 'resetFzfParams': resetFzfParams(); break;
    case 'settingToggleIncludeBackups': settingToggleIncludeBackups(); break;
    case 'saveBlacklist': { const el = document.getElementById('settingBlacklist'); if (el) { prefs.setItem('blacklistDirs', el.value); showSavedMsg('savedMsgBlacklist'); showToast(toastFmt('toast.directory_blacklist_saved')); } } break;
    case 'applyCustomScheme': applyCustomScheme(); break;
    case 'showSavePreset': showSavePreset(); break;
    case 'confirmSavePreset': confirmSavePreset(); break;
    case 'cancelSavePreset': cancelSavePreset(); break;
    case 'deleteCustomSchemes': deleteCustomSchemes(); break;
    case 'loadCustomPreset': loadCustomPreset(el.dataset.idx); break;
    case 'browseDir': browseDir(el.dataset.target); break;
    case 'saveCustomDirs': saveCustomDirs(); break;
    case 'saveAudioScanDirs': saveAudioScanDirs(); break;
    case 'saveDawScanDirs': saveDawScanDirs(); break;
    case 'savePresetScanDirs': savePresetScanDirs(); break;
    case 'openPrefsFile': showToast(toastFmt('toast.opening_preferences')); openPrefsFile(); break;
    case 'toggleRegex': toggleRegex(el); break;
    case 'collapsePlayer': collapsePlayer(); break;
    case 'hidePlayer': hidePlayer(); break;
    case 'showPlayer': showPlayer(); break;
    case 'prevTrack': prevTrack(); break;
    case 'nextTrack': nextTrack(); break;
    case 'toggleShuffle': toggleShuffle(); break;
    case 'favCurrentTrack': favCurrentTrack(); break;
    case 'tagCurrentTrack': tagCurrentTrack(); break;
    case 'toggleMute': toggleMute(); break;
    case 'resetEq': resetEq(); break;
    case 'clearRecentlyPlayed': clearRecentlyPlayed(); break;
    case 'exportRecentlyPlayed': exportRecentlyPlayed(); break;
    case 'importRecentlyPlayed': importRecentlyPlayed(); break;
    case 'toggleMono': toggleMono(); break;
    case 'toggleEqSection': toggleEqSection(); break;
    case 'setAbA': setAbLoopStart(); break;
    case 'setAbB': setAbLoopEnd(); break;
    case 'clearAbLoop': clearAbLoop(); break;
    case 'createTag': createNewTag(); break;
    case 'closeMetaRow': closeMetaRow(); break;
  } } catch (err) { console.error('Action error:', action, err); showToast(toastFmt('toast.action_error', { err: err.message || err }), 4000, 'error'); }
});
document.addEventListener('dblclick', (e) => {
  // DAW projects — open in DAW
  const dawRow = e.target.closest('#dawTableBody tr[data-daw-path]');
  if (dawRow) {
    e.preventDefault();
    const filePath = dawRow.dataset.dawPath;
    const name = dawRow.querySelector('.col-name')?.textContent || filePath.split('/').pop();
    const dawName = dawRow.querySelector('.format-badge')?.textContent || 'DAW';
    dawRow.classList.remove('row-opening');
    void dawRow.offsetWidth;
    dawRow.classList.add('row-opening');
    showToast(toastFmt('toast.opening_in_daw', { name, daw: dawName }));
    window.vstUpdater.openDawProject(filePath).catch(err => {
      showToast(toastFmt('toast.daw_not_installed', { daw: dawName, err }), 4000, 'error');
    });
    return;
  }

  // Audio samples — start playback
  const audioRow = e.target.closest('#audioTableBody tr[data-audio-path]');
  if (audioRow && !e.target.closest('.col-actions')) {
    e.preventDefault();
    previewAudio(audioRow.getAttribute('data-audio-path'));
    return;
  }

  // Plugins — open on KVR
  const pluginCard = e.target.closest('#pluginList .plugin-card');
  if (pluginCard && !e.target.closest('.plugin-actions')) {
    e.preventDefault();
    const kvrBtn = pluginCard.querySelector('[data-action="openKvr"]');
    if (kvrBtn) {
      openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name);
    }
    return;
  }

  // Presets — reveal in Finder
  const presetRow = e.target.closest('#presetTableBody tr[data-preset-path]');
  if (presetRow && !e.target.closest('.col-actions')) {
    e.preventDefault();
    const presetName = presetRow.querySelector('td')?.textContent || 'preset';
    openPresetFolder(presetRow.dataset.presetPath);
    showToast(toastFmt('toast.revealing_preset_finder', { presetName }));
    return;
  }
});
document.addEventListener('input', (e) => {
  const action = e.target.dataset.action;
  if (_filterRegistry[action]) { applyFilterDebounced(action); return; }
  if (action === 'setVolume') setAudioVolume(e.target.value);
  else if (action === 'setEqLow') setEqBand('low', e.target.value);
  else if (action === 'setEqMid') setEqBand('mid', e.target.value);
  else if (action === 'setEqHigh') setEqBand('high', e.target.value);
  else if (action === 'setGain') setPreampGain(e.target.value);
  else if (action === 'setPan') setPan(e.target.value);
  else if (action === 'settingPageSize') settingUpdatePageSize(e.target.value);
  else if (action === 'settingFlushInterval') settingUpdateFlushInterval(e.target.value);
  else if (action === 'settingThreadMultiplier') settingUpdateThreadMultiplier(e.target.value);
  else if (action === 'settingChannelBuffer') settingUpdateChannelBuffer(e.target.value);
  else if (action === 'settingBatchSize') settingUpdateBatchSize(e.target.value);
  else if (action === 'settingFdLimit') settingUpdateFdLimit(e.target.value);
  else if (action === 'settingVizFps') settingUpdateVizFps(e.target.value);
  else if (action === 'settingWfCacheMax') settingUpdateWfCacheMax(e.target.value);
  else if (action === 'settingAnalysisPause') settingUpdateAnalysisPause(e.target.value);
  else if (action === 'settingMaxRecent') settingUpdateMaxRecent(e.target.value);
});
document.addEventListener('change', (e) => {
  const action = e.target.dataset.action;
  if (_filterRegistry[action]) { applyFilter(action); return; }
  if (action === 'setPlaybackSpeed') { setPlaybackSpeed(e.target.value); showToast(toastFmt('toast.speed_value', { value: e.target.value })); }
  else if (action === 'settingDefaultTypeFilter') { settingSaveSelect('defaultTypeFilter', e.target.value); showToast(toastFmt('toast.default_type_filter', { value: e.target.value })); }
  else if (action === 'settingPluginSort') { settingSaveSelect('pluginSort', e.target.value); showToast(toastFmt('toast.plugin_sort', { value: e.target.value })); }
  else if (action === 'settingAudioSort') { settingSaveSelect('audioSort', e.target.value); showToast(toastFmt('toast.audio_sort', { value: e.target.value })); }
  else if (action === 'settingDawSort') { settingSaveSelect('dawSort', e.target.value); showToast(toastFmt('toast.daw_sort', { value: e.target.value })); }
  else if (action === 'settingTagBarPosition') {
    const pos = e.target.value || 'top';
    prefs.setItem('tagBarPosition', pos);
    const bar = document.getElementById('globalTagBar');
    const tabNav = document.querySelector('.tab-nav');
    if (bar && tabNav) {
      if (pos === 'bottom') {
        const lastTab = [...document.querySelectorAll('.tab-content')].pop();
        if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
      } else {
        tabNav.parentNode.insertBefore(bar, tabNav);
      }
    }
    showToast(toastFmt('toast.tag_bar_moved', { pos }));
  } else if (action === 'settingUiLocale') {
    const v = e.target.value || 'en';
    prefs.setItem('uiLocale', v);
    reloadAppStrings(v);
    showToast(toastFmt('toast.locale_changed'), 4000, '');
  }
});
document.addEventListener('blur', (e) => {}, true);

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
  const isMac = navigator.platform.includes('Mac');
  const mod = isMac ? e.metaKey : e.ctrlKey;

  // Escape — clear search or stop operation
  if (e.key === 'Escape') {
    const focused = document.activeElement;
    if (focused?.tagName === 'INPUT' && focused.value) {
      focused.value = '';
      focused.dispatchEvent(new Event('input', { bubbles: true }));
    } else if (currentOperation) {
      stopCurrentOperation();
    }
  }

  // Cmd/Ctrl+1-7 — handled by native menu accelerators
  // Cmd+F — handled by native menu accelerator (find)
});

function showToast(message, duration = 2500, type = '') {
  const container = document.getElementById('toastContainer');
  const el = document.createElement('div');
  el.className = 'toast' + (type ? ` toast-${type}` : '');
  el.textContent = message;
  const fadeStart = (duration - 300) / 1000;
  el.style.animation = `toast-in 0.3s ease-out, toast-out 0.3s ease-in ${fadeStart}s forwards`;
  container.appendChild(el);
  setTimeout(() => el.remove(), duration);
  // Log error toasts to app.log
  if (type === 'error' && window.vstUpdater?.appendLog) {
    window.vstUpdater.appendLog('TOAST_ERROR: ' + message);
  }
}

window.vstUpdater = {
  appendLog: (msg) => invoke('append_log', { msg }),
  getVersion: () => invoke('get_version'),
  getAppStrings: (locale) => invoke('get_app_strings', { locale: locale ?? null }),
  getToastStrings: (locale) => invoke('get_toast_strings', { locale: locale ?? null }),
  scanPlugins: (customRoots, excludePaths) => invoke('scan_plugins', { customRoots: customRoots || null, excludePaths: excludePaths || null }),
  stopScan: () => invoke('stop_scan'),
  onScanProgress: (callback) => {
    const p = listen('scan-progress', (event) => callback(event.payload));
    return () => { p.then(fn => fn()); };
  },
  checkUpdates: (plugins) => invoke('check_updates', { plugins }),
  stopUpdates: () => invoke('stop_updates'),
  onUpdateProgress: (callback) => {
    const p = listen('update-progress', (event) => callback(event.payload));
    return () => { p.then(fn => fn()); };
  },
  resolveKvr: (directUrl, pluginName) => invoke('resolve_kvr', { directUrl, pluginName }),
  openUpdateUrl: (url) => invoke('open_update_url', { url }),
  openPluginFolder: (path) => invoke('open_plugin_folder', { pluginPath: path }),
  // History
  getScans: () => invoke('history_get_scans'),
  getScanDetail: (id) => invoke('history_get_detail', { id }),
  deleteScan: (id) => invoke('history_delete', { id }),
  clearHistory: () => invoke('history_clear'),
  diffScans: (oldId, newId) => invoke('history_diff', { oldId, newId }),
  getLatestScan: () => invoke('history_latest'),
  // Audio samples
  scanAudioSamples: (customRoots, excludePaths) => invoke('scan_audio_samples', { customRoots: customRoots || null, excludePaths: excludePaths || null }),
  stopAudioScan: () => invoke('stop_audio_scan'),
  onAudioScanProgress: (callback) => {
    const p = listen('audio-scan-progress', (event) => callback(event.payload));
    return () => { p.then(fn => fn()); };
  },
  openAudioFolder: (path) => invoke('open_audio_folder', { filePath: path }),
  getAudioMetadata: (filePath) => invoke('get_audio_metadata', { filePath }),
  // Audio history
  saveAudioScan: (samples, roots) => invoke('audio_history_save', { samples, roots: roots || null }),
  getAudioScans: () => invoke('audio_history_get_scans'),
  getAudioScanDetail: (id) => invoke('audio_history_get_detail', { id }),
  deleteAudioScan: (id) => invoke('audio_history_delete', { id }),
  clearAudioHistory: () => invoke('audio_history_clear'),
  getLatestAudioScan: () => invoke('audio_history_latest'),
  diffAudioScans: (oldId, newId) => invoke('audio_history_diff', { oldId, newId }),
  // DAW projects
  scanDawProjects: (customRoots, excludePaths) => invoke('scan_daw_projects', { customRoots: customRoots || null, excludePaths: excludePaths || null }),
  // Presets
  scanPresets: (customRoots, excludePaths) => invoke('scan_presets', { customRoots: customRoots || null, excludePaths: excludePaths || null }),
  stopPresetScan: () => invoke('stop_preset_scan'),
  onPresetScanProgress: (callback) => {
    const p = listen('preset-scan-progress', (event) => callback(event.payload));
    return () => { p.then(fn => fn()); };
  },
  openPresetFolder: (path) => invoke('open_preset_folder', { filePath: path }),
  savePresetScan: (presets, roots) => invoke('preset_history_save', { presets, roots: roots || null }),
  getPresetScans: () => invoke('preset_history_get_scans'),
  getPresetScanDetail: (id) => invoke('preset_history_get_detail', { id }),
  deletePresetScan: (id) => invoke('preset_history_delete', { id }),
  clearPresetHistory: () => invoke('preset_history_clear'),
  getLatestPresetScan: () => invoke('preset_history_latest'),
  diffPresetScans: (oldId, newId) => invoke('preset_history_diff', { oldId, newId }),
  exportPresetsJson: (presets, filePath) => invoke('export_presets_json', { presets, filePath }),
  importPresetsJson: (filePath) => invoke('import_presets_json', { filePath }),
  importAudioJson: (filePath) => invoke('import_audio_json', { filePath }),
  importDawJson: (filePath) => invoke('import_daw_json', { filePath }),
  stopDawScan: () => invoke('stop_daw_scan'),
  onDawScanProgress: (callback) => {
    const p = listen('daw-scan-progress', (event) => callback(event.payload));
    return () => { p.then(fn => fn()); };
  },
  openDawFolder: (path) => invoke('open_daw_folder', { filePath: path }),
  openDawProject: (path) => invoke('open_daw_project', { filePath: path }),
  extractProjectPlugins: (path) => invoke('extract_project_plugins', { filePath: path }),
  estimateBpm: (path) => invoke('estimate_bpm', { filePath: path }),
  detectAudioKey: (path) => invoke('detect_audio_key', { filePath: path }),
  measureLufs: (path) => invoke('measure_lufs', { filePath: path }),
  readCacheFile: (name) => invoke('read_cache_file', { name }),
  writeCacheFile: (name, data) => invoke('write_cache_file', { name, data }),
  getWalkerStatus: () => invoke('get_walker_status'),
  appendLog: (msg) => invoke('append_log', { msg }).catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }),
  readLog: () => invoke('read_log'),
  clearLog: () => invoke('clear_log'),
  listDataFiles: () => invoke('list_data_files'),
  readBwproject: (path) => invoke('read_bwproject', { filePath: path }),
  deleteDataFile: (name) => invoke('delete_data_file', { name }),
  // DAW history
  saveDawScan: (projects, roots) => invoke('daw_history_save', { projects, roots: roots || null }),
  getDawScans: () => invoke('daw_history_get_scans'),
  getDawScanDetail: (id) => invoke('daw_history_get_detail', { id }),
  deleteDawScan: (id) => invoke('daw_history_delete', { id }),
  clearDawHistory: () => invoke('daw_history_clear'),
  getLatestDawScan: () => invoke('daw_history_latest'),
  diffDawScans: (oldId, newId) => invoke('daw_history_diff', { oldId, newId }),
  // KVR cache
  getKvrCache: () => invoke('kvr_cache_get'),
  updateKvrCache: (entries) => invoke('kvr_cache_update', { entries }),
  // Export / Import
  exportJson: (plugins, filePath) => invoke('export_plugins_json', { plugins, filePath }),
  exportCsv: (plugins, filePath) => invoke('export_plugins_csv', { plugins, filePath }),
  importJson: (filePath) => invoke('import_plugins_json', { filePath }),
  exportAudioJson: (samples, filePath) => invoke('export_audio_json', { samples, filePath }),
  exportAudioDsv: (samples, filePath) => invoke('export_audio_dsv', { samples, filePath }),
  exportDawJson: (projects, filePath) => invoke('export_daw_json', { projects, filePath }),
  exportDawDsv: (projects, filePath) => invoke('export_daw_dsv', { projects, filePath }),
  exportPresetsDsv: (presets, filePath) => invoke('export_presets_dsv', { presets, filePath }),
  exportToml: (data, filePath) => invoke('export_toml', { data, filePath }),
  importToml: (filePath) => invoke('import_toml', { filePath }),
  exportPdf: (title, headers, rows, filePath) => invoke('export_pdf', { title, headers, rows, filePath }),
  writeTextFile: (filePath, contents) => invoke('write_text_file', { filePath, contents }),
  openWithApp: (filePath, appName) => invoke('open_with_app', { filePath, appName }),
  // File browser
  listDirectory: (dirPath) => invoke('fs_list_dir', { dirPath }),
  deleteFile: (filePath) => invoke('delete_file', { filePath }),
  renameFile: (oldPath, newPath) => invoke('rename_file', { oldPath, newPath }),
  getHomeDir: () => invoke('get_home_dir'),
  // Similarity
  findSimilarSamples: (filePath, candidatePaths, maxResults) => invoke('find_similar_samples', { filePath, candidatePaths, maxResults: maxResults || 20 }),
  readAlsXml: (filePath) => invoke('read_als_xml', { filePath }),
  readProjectFile: (filePath) => invoke('read_project_file', { filePath }),
  // Preferences (file-backed)
  getProcessStats: () => invoke('get_process_stats'),
  openPrefsFile: () => invoke('open_prefs_file'),
  getPrefsPath: () => invoke('get_prefs_path'),
  prefsGetAll: () => invoke('prefs_get_all'),
  prefsSet: (key, value) => invoke('prefs_set', { key, value }),
  prefsRemove: (key) => invoke('prefs_remove', { key }),
  prefsSaveAll: (prefs) => invoke('prefs_save_all', { prefs }),
  // Database-backed queries (SQLite)
  dbQueryAudio: (params) => invoke('db_query_audio', { params }),
  dbAudioStats: (scanId) => invoke('db_audio_stats', { scanId: scanId || null }),
  dbListScans: () => invoke('db_list_scans'),
  dbUpdateBpm: (path, bpm) => invoke('db_update_bpm', { path, bpm }),
  dbUpdateKey: (path, key) => invoke('db_update_key', { path, key }),
  dbUpdateLufs: (path, lufs) => invoke('db_update_lufs', { path, lufs }),
  dbGetAnalysis: (path) => invoke('db_get_analysis', { path }),
  dbUnanalyzedPaths: (limit) => invoke('db_unanalyzed_paths', { limit: limit || 100 }),
  dbMigrateJson: () => invoke('db_migrate_json'),
  dbCacheStats: () => invoke('db_cache_stats'),
  dbClearCaches: () => invoke('db_clear_caches'),
  dbClearCacheTable: (table) => invoke('db_clear_cache_table', { table }),
  // File watcher
  startFileWatcher: (dirs) => invoke('start_file_watcher', { dirs }),
  stopFileWatcher: () => invoke('stop_file_watcher'),
  getFileWatcherStatus: () => invoke('get_file_watcher_status'),
  // MIDI
  getMidiInfo: (filePath) => invoke('get_midi_info', { filePath }),
  batchAnalyze: (paths) => invoke('batch_analyze', { paths }),
  dbQueryPlugins: (params) => invoke('db_query_plugins', params || {}),
  dbQueryDaw: (params) => invoke('db_query_daw', params || {}),
  dbQueryPresets: (params) => invoke('db_query_presets', params || {}),
};

// ── Preferences layer (file-backed, survives reboots) ──
// In-memory cache loaded from Rust on startup; writes go to both cache and disk.
const prefs = {
  _cache: {},
  _loaded: false,
  async load() {
    this._cache = await window.vstUpdater.prefsGetAll();
    this._loaded = true;
  },
  getItem(key) {
    const val = this._cache[key];
    if (val === undefined || val === null) return null;
    return typeof val === 'string' ? val : JSON.stringify(val);
  },
  getObject(key, fallback) {
    const val = this._cache[key];
    if (val === undefined || val === null) return fallback;
    if (typeof val === 'string') {
      try { return JSON.parse(val); } catch { return fallback; }
    }
    return val;
  },
  setItem(key, value) {
    this._cache[key] = value;
    window.vstUpdater.prefsSet(key, value).catch(() => showToast(toastFmt('toast.failed_save_preference'), 4000, 'error'));
  },
  removeItem(key) {
    delete this._cache[key];
    window.vstUpdater.prefsRemove(key).catch(() => showToast(toastFmt('toast.failed_save_preference'), 4000, 'error'));
  },
};

let allPlugins = [];
let pluginsWithUpdates = [];
let currentOperation = null; // 'scan' or 'updates'
let AUDIO_PAGE_SIZE = 500;
let DAW_PAGE_SIZE = 500;

// Common audio plugin manufacturers where bundle ID doesn't match KVR slug
const KVR_MANUFACTURER_MAP = {
  'madronalabs': 'madrona-labs',
  'audiothing': 'audio-thing',
  'audiodamage': 'audio-damage',
  'soundtoys': 'soundtoys',
  'native-instruments': 'native-instruments',
  'plugin-alliance': 'plugin-alliance',
  'softube': 'softube',
  'izotope': 'izotope',
  'eventide': 'eventide',
  'arturia': 'arturia',
  'u-he': 'u-he',
};

// Display app version in header
window.vstUpdater.getVersion().then(v => {
  const vStr = 'v' + v;
  const el = document.getElementById('appVersion');
  if (el) el.textContent = vStr;
  const sv = document.getElementById('settingsVersion');
  if (sv) sv.textContent = vStr;
}).catch(() => {});

function showStopButton() {
  const btn = document.getElementById('btnStop') || document.getElementById('btnStopAll');
  if (btn) btn.style.display = '';
}

function hideStopButton() {
  const btn = document.getElementById('btnStop') || document.getElementById('btnStopAll');
  if (btn) btn.style.display = 'none';
  currentOperation = null;
}

async function stopCurrentOperation() {
  if (currentOperation === 'scan') {
    await window.vstUpdater.stopScan();
  } else if (currentOperation === 'updates') {
    await window.vstUpdater.stopUpdates();
  } else if (currentOperation === 'kvr-resolve') {
    stopKvrResolve();
  } else if (currentOperation === 'audio-scan') {
    await window.vstUpdater.stopAudioScan();
  } else if (currentOperation === 'daw-scan') {
    await window.vstUpdater.stopDawScan();
  } else if (currentOperation === 'preset-scan') {
    await window.vstUpdater.stopPresetScan();
  }
}

// ── Global error logging ──
window.addEventListener('error', (e) => {
  const msg = `ERROR: ${e.message} at ${e.filename}:${e.lineno}:${e.colno}`;
  window.vstUpdater?.appendLog(msg);
});
window.addEventListener('unhandledrejection', (e) => {
  const msg = `UNHANDLED_REJECTION: ${e.reason?.message || e.reason || 'unknown'}`;
  window.vstUpdater?.appendLog(msg);
});
