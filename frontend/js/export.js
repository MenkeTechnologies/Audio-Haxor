// ── Export button visibility ──
function updateExportButton() {
  document.getElementById('btnExport').style.display = allPlugins.length > 0 ? '' : 'none';
}

// ── Export / Import ──

async function exportPlugins() {
  if (allPlugins.length === 0) return;

  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;

  const filePath = await dialogApi.save({
    title: 'Export Plugin Inventory',
    defaultPath: 'plugin-inventory',
    filters: [
      { name: 'JSON', extensions: ['json'] },
      { name: 'CSV', extensions: ['csv'] },
      { name: 'TSV', extensions: ['tsv'] },
    ],
  });
  if (!filePath) return;

  try {
    if (filePath.endsWith('.csv') || filePath.endsWith('.tsv')) {
      await window.vstUpdater.exportCsv(allPlugins, filePath);
    } else {
      const path = filePath.endsWith('.json') ? filePath : filePath + '.json';
      await window.vstUpdater.exportJson(allPlugins, path);
    }
  } catch (err) {
    console.error('Export failed:', err);
  }
}

async function importPlugins() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;

  const selected = await dialogApi.open({
    title: 'Import Plugin Inventory',
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!selected) return;

  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;

  try {
    const imported = await window.vstUpdater.importJson(filePath);
    if (imported && imported.length > 0) {
      allPlugins = imported;
      document.getElementById('totalCount').textContent = allPlugins.length;
      document.getElementById('btnCheckUpdates').disabled = false;
      document.getElementById('btnExport').style.display = '';
      renderPlugins(allPlugins);
    }
  } catch (err) {
    console.error('Import failed:', err);
  }
}

async function exportAudio() {
  if (allAudioSamples.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export Audio Sample List',
    defaultPath: 'audio-samples',
    filters: [
      { name: 'JSON', extensions: ['json'] },
      { name: 'CSV', extensions: ['csv'] },
      { name: 'TSV', extensions: ['tsv'] },
    ],
  });
  if (!filePath) return;
  try {
    if (filePath.endsWith('.csv') || filePath.endsWith('.tsv')) {
      await window.vstUpdater.exportAudioDsv(allAudioSamples, filePath);
    } else {
      const path = filePath.endsWith('.json') ? filePath : filePath + '.json';
      await window.vstUpdater.exportAudioJson(allAudioSamples, path);
    }
  } catch (err) { console.error('Audio export failed:', err); }
}

async function exportDaw() {
  if (allDawProjects.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export DAW Project List',
    defaultPath: 'daw-projects',
    filters: [
      { name: 'JSON', extensions: ['json'] },
      { name: 'CSV', extensions: ['csv'] },
      { name: 'TSV', extensions: ['tsv'] },
    ],
  });
  if (!filePath) return;
  try {
    if (filePath.endsWith('.csv') || filePath.endsWith('.tsv')) {
      await window.vstUpdater.exportDawDsv(allDawProjects, filePath);
    } else {
      const path = filePath.endsWith('.json') ? filePath : filePath + '.json';
      await window.vstUpdater.exportDawJson(allDawProjects, path);
    }
  } catch (err) { console.error('DAW export failed:', err); }
}

async function exportPresets() {
  if (allPresets.length === 0) return;
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const filePath = await dialogApi.save({
    title: 'Export Preset List',
    defaultPath: 'presets',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!filePath) return;
  try {
    const path = filePath.endsWith('.json') ? filePath : filePath + '.json';
    await window.vstUpdater.exportPresetsJson(allPresets, path);
  } catch (err) { console.error('Preset export failed:', err); }
}

async function importAudio() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({
    title: 'Import Audio Sample List',
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    const imported = await window.vstUpdater.importAudioJson(filePath);
    if (imported && imported.length > 0) {
      allAudioSamples = imported;
      rebuildAudioStats();
      filterAudioSamples();
      document.getElementById('btnExportAudio').style.display = '';
    }
  } catch (err) { console.error('Audio import failed:', err); }
}

async function importDaw() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({
    title: 'Import DAW Project List',
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    const imported = await window.vstUpdater.importDawJson(filePath);
    if (imported && imported.length > 0) {
      allDawProjects = imported;
      rebuildDawStats();
      filterDawProjects();
      document.getElementById('btnExportDaw').style.display = '';
    }
  } catch (err) { console.error('DAW import failed:', err); }
}

async function importPresets() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({
    title: 'Import Preset List',
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    const imported = await window.vstUpdater.importPresetsJson(filePath);
    if (imported && imported.length > 0) {
      allPresets = imported;
      rebuildPresetStats();
      filterPresets();
      document.getElementById('btnExportPresets').style.display = '';
    }
  } catch (err) { console.error('Preset import failed:', err); }
}
