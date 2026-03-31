// ── Export button visibility ──
function updateExportButton() {
  document.getElementById('btnExport').style.display = allPlugins.length > 0 ? '' : 'none';
}

async function showImportError(type, err) {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  const examples = {
    plugins: `[
  {
    "name": "Serum",
    "path": "/Library/Audio/Plug-Ins/VST3/Serum.vst3",
    "type": "VST3",
    "version": "1.3.5",
    "manufacturer": "Xfer Records",
    "size": "12.5 MB",
    "modified": "2024-01-15"
  }
]`,
    audio: `[
  {
    "name": "kick",
    "path": "/Users/you/Samples/kick.wav",
    "directory": "/Users/you/Samples",
    "format": "WAV",
    "size": 102400,
    "sizeFormatted": "100.0 KB",
    "modified": "2024-01-15"
  }
]`,
    daw: `[
  {
    "name": "MySong",
    "path": "/Users/you/Music/MySong.als",
    "directory": "/Users/you/Music",
    "format": "ALS",
    "daw": "Ableton Live",
    "size": 524288,
    "sizeFormatted": "512.0 KB",
    "modified": "2024-01-15"
  }
]`,
    presets: `[
  {
    "name": "Lead Synth",
    "path": "/Users/you/Presets/Lead.fxp",
    "directory": "/Users/you/Presets",
    "format": "FXP",
    "size": 4096,
    "sizeFormatted": "4.0 KB",
    "modified": "2024-01-15"
  }
]`,
  };

  const msg = `Invalid file format.\n\nError: ${err}\n\nExpected a JSON array like:\n\n${examples[type] || examples.plugins}`;

  if (dialogApi && dialogApi.message) {
    await dialogApi.message(msg, { title: 'Import Error', kind: 'error' });
  } else {
    alert(msg);
  }
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
    showToast(`Export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error');
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
  } catch (err) {
    await showImportError('plugins', err.message || String(err));
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
  } catch (err) { showToast(`Audio export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
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
  } catch (err) { showToast(`DAW export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
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
  } catch (err) { showToast(`Preset export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error'); }
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
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('audio', 'File contains no audio samples or is empty.');
      return;
    }
    allAudioSamples = imported;
    rebuildAudioStats();
    filterAudioSamples();
    document.getElementById('btnExportAudio').style.display = '';
  } catch (err) { await showImportError('audio', err.message || String(err)); }
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
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('daw', 'File contains no DAW projects or is empty.');
      return;
    }
    allDawProjects = imported;
    rebuildDawStats();
    filterDawProjects();
    document.getElementById('btnExportDaw').style.display = '';
  } catch (err) { await showImportError('daw', err.message || String(err)); }
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
    if (!imported || !Array.isArray(imported) || imported.length === 0) {
      await showImportError('presets', 'File contains no presets or is empty.');
      return;
    }
    allPresets = imported;
    rebuildPresetStats();
    filterPresets();
    document.getElementById('btnExportPresets').style.display = '';
  } catch (err) { await showImportError('presets', err.message || String(err)); }
}
