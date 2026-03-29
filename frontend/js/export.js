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
      document.getElementById('toolbar').style.display = 'flex';
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
