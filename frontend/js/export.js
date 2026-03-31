// ── Export button visibility ──
function updateExportButton() {
  document.getElementById('btnExport').style.display = allPlugins.length > 0 ? '' : 'none';
}

async function showImportError(type, err) {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  const examples = {
    plugins: `[{ "name": "Serum", "path": "/Library/.../Serum.vst3", "type": "VST3", ... }]`,
    audio: `[{ "name": "kick", "path": "/Samples/kick.wav", "format": "WAV", ... }]`,
    daw: `[{ "name": "MySong", "path": "/Music/MySong.als", "daw": "Ableton Live", ... }]`,
    presets: `[{ "name": "Lead", "path": "/Presets/Lead.fxp", "format": "FXP", ... }]`,
  };
  const msg = `Import Error: ${err}\n\nExpected JSON/TOML with data like:\n${examples[type] || examples.plugins}`;
  if (dialogApi && dialogApi.message) {
    await dialogApi.message(msg, { title: 'Import Error', kind: 'error' });
  } else {
    alert(msg);
  }
}

const ALL_EXPORT_FILTERS = [
  { name: 'JSON', extensions: ['json'] },
  { name: 'TOML', extensions: ['toml'] },
  { name: 'CSV', extensions: ['csv'] },
  { name: 'TSV', extensions: ['tsv'] },
  { name: 'PDF', extensions: ['pdf'] },
];

const ALL_IMPORT_FILTERS = [
  { name: 'All Supported', extensions: ['json', 'toml'] },
  { name: 'JSON', extensions: ['json'] },
  { name: 'TOML', extensions: ['toml'] },
];

function getFileFormat(filePath) {
  if (filePath.endsWith('.toml')) return 'toml';
  if (filePath.endsWith('.csv')) return 'csv';
  if (filePath.endsWith('.tsv')) return 'tsv';
  if (filePath.endsWith('.pdf')) return 'pdf';
  return 'json';
}

// ── Plugins ──

async function exportPlugins() {
  if (allPlugins.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export Plugin Inventory',
    defaultPath: 'plugin-inventory',
    filters: ALL_EXPORT_FILTERS,
  });
  if (!filePath) return;
  const btn = document.getElementById('btnExport');
  btnLoading(btn, true);
  showGlobalProgress();
  try {
    const fmt = getFileFormat(filePath);
    if (fmt === 'pdf') {
      const headers = ['Name', 'Type', 'Version', 'Manufacturer', 'Architecture', 'Size', 'Modified'];
      const rows = allPlugins.map(p => [p.name, p.type, p.version, p.manufacturer || '', (p.architectures || []).join(', '), p.size, p.modified]);
      await window.vstUpdater.exportPdf('Plugin Inventory', headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
      await window.vstUpdater.exportCsv(allPlugins, filePath);
    } else if (fmt === 'toml') {
      await window.vstUpdater.exportToml({ plugins: allPlugins }, filePath);
    } else {
      await window.vstUpdater.exportJson(allPlugins, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
    showToast('Plugins exported');
  } catch (err) {
    showToast(`Export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error');
  } finally {
    btnLoading(btn, false);
    hideGlobalProgress();
  }
}

async function importPlugins() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Plugin Inventory', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  const ibtn = document.getElementById('btnImport');
  btnLoading(ibtn, true);
  showGlobalProgress();
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.plugins || data;
    } else {
      imported = await window.vstUpdater.importJson(filePath);
    }
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('plugins', 'File contains no plugins or is empty.');
      return;
    }
    allPlugins = imported;
    document.getElementById('totalCount').textContent = allPlugins.length;
    document.getElementById('btnCheckUpdates').disabled = false;
    document.getElementById('btnExport').style.display = '';
    renderPlugins(allPlugins);
    resolveKvrDownloads();
    showToast(`Imported ${imported.length} plugins`);
  } catch (err) {
    await showImportError('plugins', err.message || String(err));
  } finally {
    btnLoading(ibtn, false);
    hideGlobalProgress();
  }
}

// ── Audio ──

async function exportAudio() {
  if (allAudioSamples.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export Audio Sample List',
    defaultPath: 'audio-samples',
    filters: ALL_EXPORT_FILTERS,
  });
  if (!filePath) return;
  showGlobalProgress();
  try {
    const fmt = getFileFormat(filePath);
    if (fmt === 'pdf') {
      const headers = ['Name', 'Format', 'Size', 'Modified', 'Path'];
      const rows = allAudioSamples.map(s => [s.name, s.format, s.sizeFormatted, s.modified, s.directory]);
      await window.vstUpdater.exportPdf('Audio Sample Library', headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
      await window.vstUpdater.exportAudioDsv(allAudioSamples, filePath);
    } else if (fmt === 'toml') {
      await window.vstUpdater.exportToml({ samples: allAudioSamples }, filePath);
    } else {
      await window.vstUpdater.exportAudioJson(allAudioSamples, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
    showToast('Samples exported');
  } catch (err) { showToast(`Audio export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
}

async function importAudio() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Audio Sample List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.samples || data;
    } else {
      imported = await window.vstUpdater.importAudioJson(filePath);
    }
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('audio', 'File contains no audio samples or is empty.');
      return;
    }
    allAudioSamples = imported;
    rebuildAudioStats();
    filterAudioSamples();
    document.getElementById('btnExportAudio').style.display = '';
    showToast(`Imported ${imported.length} samples`);
  } catch (err) { await showImportError('audio', err.message || String(err)); }
}

// ── DAW ──

async function exportDaw() {
  if (allDawProjects.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export DAW Project List',
    defaultPath: 'daw-projects',
    filters: ALL_EXPORT_FILTERS,
  });
  if (!filePath) return;
  try {
    const fmt = getFileFormat(filePath);
    if (fmt === 'pdf') {
      const headers = ['Name', 'DAW', 'Format', 'Size', 'Modified', 'Path'];
      const rows = allDawProjects.map(p => [p.name, p.daw, p.format, p.sizeFormatted, p.modified, p.directory]);
      await window.vstUpdater.exportPdf('DAW Projects', headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
      await window.vstUpdater.exportDawDsv(allDawProjects, filePath);
    } else if (fmt === 'toml') {
      await window.vstUpdater.exportToml({ projects: allDawProjects }, filePath);
    } else {
      await window.vstUpdater.exportDawJson(allDawProjects, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
    showToast('DAW projects exported');
  } catch (err) { showToast(`DAW export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
}

async function importDaw() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import DAW Project List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.projects || data;
    } else {
      imported = await window.vstUpdater.importDawJson(filePath);
    }
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('daw', 'File contains no DAW projects or is empty.');
      return;
    }
    allDawProjects = imported;
    rebuildDawStats();
    filterDawProjects();
    document.getElementById('btnExportDaw').style.display = '';
    showToast(`Imported ${imported.length} DAW projects`);
  } catch (err) { await showImportError('daw', err.message || String(err)); }
}

// ── Presets ──

async function exportPresets() {
  if (allPresets.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export Preset List',
    defaultPath: 'presets',
    filters: ALL_EXPORT_FILTERS,
  });
  if (!filePath) return;
  try {
    const fmt = getFileFormat(filePath);
    if (fmt === 'pdf') {
      const headers = ['Name', 'Format', 'Size', 'Modified', 'Path'];
      const rows = allPresets.map(p => [p.name, p.format, p.sizeFormatted || '', p.modified, p.directory]);
      await window.vstUpdater.exportPdf('Presets', headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
      await window.vstUpdater.exportPresetsDsv(allPresets, filePath);
    } else if (fmt === 'toml') {
      await window.vstUpdater.exportToml({ presets: allPresets }, filePath);
    } else {
      await window.vstUpdater.exportPresetsJson(allPresets, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
    showToast('Presets exported');
  } catch (err) { showToast(`Preset export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
}

async function importPresets() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Preset List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.presets || data;
    } else {
      imported = await window.vstUpdater.importPresetsJson(filePath);
    }
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('presets', 'File contains no presets or is empty.');
      return;
    }
    allPresets = imported;
    rebuildPresetStats();
    filterPresets();
    document.getElementById('btnExportPresets').style.display = '';
    showToast(`Imported ${imported.length} presets`);
  } catch (err) { await showImportError('presets', err.message || String(err)); }
}
