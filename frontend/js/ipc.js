// Tauri IPC bridge — replaces Electron's preload.js window.vstUpdater API
const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

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
    case 'github': openUpdate('https://github.com/MenkeTechnologies/universal-plugin-update-manager'); break;
    case 'docs': openUpdate('https://menketechnologies.github.io/universal-plugin-update-manager/'); break;
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
    case 'openUpdate': openUpdate(el.dataset.url); break;
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
    case 'importDaw': importDaw(); break;
    case 'exportPresets': exportPresets(); break;
    case 'importPresets': importPresets(); break;
    case 'settingToggleTheme': settingToggleTheme(); break;
    case 'settingToggleCrt': settingToggleCrt(); break;
    case 'clearFavorites': clearFavorites(); break;
    case 'exportFavorites': exportFavorites(); break;
    case 'importFavorites': importFavorites(); break;
    case 'exportNotes': exportNotes(); break;
    case 'importNotes': importNotes(); break;
    case 'clearAllNotes': clearAllNotes(); break;
    case 'clearGlobalTag': clearGlobalTag(); break;
    case 'settingResetAllUI': settingResetAllUI(); break;
    case 'settingResetColumns': settingResetColumns(); break;
    case 'settingResetSectionOrder': resetSettingsSectionOrder(); break;
    case 'settingResetTabOrder': settingResetTabOrder(); break;
    case 'settingClearAllHistory': settingClearAllHistory(); break;
    case 'settingClearKvrCache': settingClearKvrCache(); break;
    case 'resetAllScans': resetAllScans(); break;
    case 'settingColorScheme': settingColorScheme(el.dataset.scheme); break;
    case 'settingToggleAutoScan': settingToggleAutoScan(); break;
    case 'settingToggleAutoUpdate': settingToggleAutoUpdate(); break;
    case 'settingToggleSingleClickPlay': settingToggleSingleClickPlay(); break;
    case 'settingToggleExpandOnClick': settingToggleExpandOnClick(); break;
    case 'settingToggleShowPlayerOnStartup': settingToggleShowPlayerOnStartup(); break;
    case 'settingToggleAutoplayNext': settingToggleAutoplayNext(); break;
    case 'resetFzfParams': resetFzfParams(); break;
    case 'settingToggleIncludeBackups': settingToggleIncludeBackups(); break;
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
    case 'openPrefsFile': openPrefsFile(); break;
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
  } } catch (err) { console.error('Action error:', action, err); showToast('Action error: ' + (err.message || err), 4000, 'error'); }
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
    showToast(`Opening "${name}" in ${dawName}...`);
    window.vstUpdater.openDawProject(filePath).catch(err => {
      showToast(`${dawName} not installed — ${err}`, 4000, 'error');
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
    showToast(`Revealing "${presetName}" in Finder...`);
    return;
  }
});
document.addEventListener('input', (e) => {
  const action = e.target.dataset.action;
  if (action === 'filterPlugins') filterPlugins();
  else if (action === 'filterAudioSamples') filterAudioSamples();
  else if (action === 'filterDawProjects') filterDawProjects();
  else if (action === 'filterPresets') filterPresets();
  else if (action === 'filterFavorites') renderFavorites();
  else if (action === 'filterNotes') renderNotesTab();
  else if (action === 'filterTags') renderTagsManager();
  else if (action === 'setVolume') setAudioVolume(e.target.value);
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
});
document.addEventListener('change', (e) => {
  const action = e.target.dataset.action;
  if (action === 'filterPlugins') filterPlugins();
  else if (action === 'filterAudioSamples') filterAudioSamples();
  else if (action === 'filterDawProjects') filterDawProjects();
  else if (action === 'filterPresets') filterPresets();
  else if (action === 'setPlaybackSpeed') setPlaybackSpeed(e.target.value);
  else if (action === 'settingDefaultTypeFilter') settingSaveSelect('defaultTypeFilter', e.target.value);
  else if (action === 'settingPluginSort') settingSaveSelect('pluginSort', e.target.value);
  else if (action === 'settingAudioSort') settingSaveSelect('audioSort', e.target.value);
  else if (action === 'settingDawSort') settingSaveSelect('dawSort', e.target.value);
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
}

window.vstUpdater = {
  getVersion: () => invoke('get_version'),
  scanPlugins: (customRoots, excludePaths) => invoke('scan_plugins', { customRoots: customRoots || null, excludePaths: excludePaths || null }),
  stopScan: () => invoke('stop_scan'),
  onScanProgress: (callback) => {
    let unlisten = null;
    listen('scan-progress', (event) => callback(event.payload)).then(fn => unlisten = fn);
    return () => { if (unlisten) unlisten(); };
  },
  checkUpdates: (plugins) => invoke('check_updates', { plugins }),
  stopUpdates: () => invoke('stop_updates'),
  onUpdateProgress: (callback) => {
    let unlisten = null;
    listen('update-progress', (event) => callback(event.payload)).then(fn => unlisten = fn);
    return () => { if (unlisten) unlisten(); };
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
    let unlisten = null;
    listen('audio-scan-progress', (event) => callback(event.payload)).then(fn => unlisten = fn);
    return () => { if (unlisten) unlisten(); };
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
    let unlisten = null;
    listen('preset-scan-progress', (event) => callback(event.payload)).then(fn => unlisten = fn);
    return () => { if (unlisten) unlisten(); };
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
    let unlisten = null;
    listen('daw-scan-progress', (event) => callback(event.payload)).then(fn => unlisten = fn);
    return () => { if (unlisten) unlisten(); };
  },
  openDawFolder: (path) => invoke('open_daw_folder', { filePath: path }),
  openDawProject: (path) => invoke('open_daw_project', { filePath: path }),
  extractProjectPlugins: (path) => invoke('extract_project_plugins', { filePath: path }),
  estimateBpm: (path) => invoke('estimate_bpm', { filePath: path }),
  detectAudioKey: (path) => invoke('detect_audio_key', { filePath: path }),
  measureLufs: (path) => invoke('measure_lufs', { filePath: path }),
  getWalkerStatus: () => invoke('get_walker_status'),
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
  openWithApp: (filePath, appName) => invoke('open_with_app', { filePath, appName }),
  // File browser
  listDirectory: (dirPath) => invoke('fs_list_dir', { dirPath }),
  deleteFile: (filePath) => invoke('delete_file', { filePath }),
  renameFile: (oldPath, newPath) => invoke('rename_file', { oldPath, newPath }),
  getHomeDir: () => invoke('get_home_dir'),
  // Similarity
  findSimilarSamples: (filePath, candidatePaths, maxResults) => invoke('find_similar_samples', { filePath, candidatePaths, maxResults: maxResults || 20 }),
  readAlsXml: (filePath) => invoke('read_als_xml', { filePath }),
  // Preferences (file-backed)
  getProcessStats: () => invoke('get_process_stats'),
  openPrefsFile: () => invoke('open_prefs_file'),
  getPrefsPath: () => invoke('get_prefs_path'),
  prefsGetAll: () => invoke('prefs_get_all'),
  prefsSet: (key, value) => invoke('prefs_set', { key, value }),
  prefsRemove: (key) => invoke('prefs_remove', { key }),
  prefsSaveAll: (prefs) => invoke('prefs_save_all', { prefs }),
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
    window.vstUpdater.prefsSet(key, value).catch(() => {});
  },
  removeItem(key) {
    delete this._cache[key];
    window.vstUpdater.prefsRemove(key).catch(() => {});
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
  document.getElementById('appVersion').textContent = vStr;
  const sv = document.getElementById('settingsVersion');
  if (sv) sv.textContent = vStr;
});

function showStopButton() {
  document.getElementById('btnStop').style.display = '';
}

function hideStopButton() {
  document.getElementById('btnStop').style.display = 'none';
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
