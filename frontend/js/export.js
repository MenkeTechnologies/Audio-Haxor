// ── Export / Import ──

function updateExportButton() {
  document.getElementById('btnExport').style.display = allPlugins.length > 0 ? '' : 'none';
}

async function showImportError(type, err) {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  const msg = `Import Error: ${err}`;
  if (dialogApi && dialogApi.message) {
    await dialogApi.message(msg, { title: 'Import Error', kind: 'error' });
  } else {
    showToast(msg, 4000, 'error');
  }
}

const ALL_IMPORT_FILTERS = [
  { name: 'All Supported', extensions: ['json', 'toml'] },
  { name: 'JSON', extensions: ['json'] },
  { name: 'TOML', extensions: ['toml'] },
];

// ── Export Modal ──

function exportFileName(label) {
  const now = new Date();
  const ts = now.toISOString().slice(0, 19).replace(/[T:]/g, '-');
  return `audiohaxor-${label}-${ts}`;
}

const EXPORT_FORMATS = [
  { id: 'json', label: 'JSON', ext: 'json', icon: '{ }', desc: 'Full data, re-importable' },
  { id: 'toml', label: 'TOML', ext: 'toml', icon: '[T]', desc: 'Human-readable config' },
  { id: 'csv',  label: 'CSV',  ext: 'csv',  icon: ',,,', desc: 'Spreadsheet compatible' },
  { id: 'tsv',  label: 'TSV',  ext: 'tsv',  icon: '\\t',  desc: 'Tab-separated values' },
  { id: 'pdf',  label: 'PDF',  ext: 'pdf',  icon: '&#128196;', desc: 'Printable A4 report' },
];

let _exportCtx = null; // { type, title, data, headers, rowsFn }

function showExportModal(type, title, itemCount) {
  let existing = document.getElementById('exportModal');
  if (existing) existing.remove();

  const html = `<div class="modal-overlay" id="exportModal" data-action-modal="closeExport">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Export ${escapeHtml(title)}</h2>
        <button class="modal-close" data-action-modal="closeExport" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="export-info">
          <span class="export-count">${itemCount.toLocaleString()} items</span>
          <span class="export-type">${escapeHtml(title)}</span>
        </div>
        <div class="export-formats" id="exportFormats">
          ${EXPORT_FORMATS.map(f => `
            <label class="export-format-option ${f.id === 'json' ? 'selected' : ''}">
              <input type="radio" name="exportFmt" value="${f.id}" ${f.id === 'json' ? 'checked' : ''}>
              <span class="export-fmt-icon">${f.icon}</span>
              <div class="export-fmt-info">
                <span class="export-fmt-label">${f.label}</span>
                <span class="export-fmt-desc">${f.desc}</span>
              </div>
              <span class="export-fmt-ext">.${f.ext}</span>
            </label>
          `).join('')}
        </div>
        <div class="export-progress" id="exportProgress" style="display:none;">
          <div class="export-progress-bar"><div class="export-progress-fill" id="exportProgressFill"></div></div>
          <span class="export-progress-text" id="exportProgressText">Exporting...</span>
        </div>
        <div class="export-actions" id="exportActions">
          <button class="btn btn-primary" data-action-modal="confirmExport" title="Export to selected format">&#8615; Export</button>
          <button class="btn btn-secondary" data-action-modal="closeExport" title="Cancel export">Cancel</button>
        </div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);

  // Radio change highlights
  document.getElementById('exportFormats').addEventListener('change', (e) => {
    document.querySelectorAll('.export-format-option').forEach(o => o.classList.remove('selected'));
    e.target.closest('.export-format-option')?.classList.add('selected');
  });
}

function closeExportModal() {
  const modal = document.getElementById('exportModal');
  if (modal) modal.remove();
  _exportCtx = null;
}

function getSelectedExportFormat() {
  const checked = document.querySelector('#exportFormats input[name="exportFmt"]:checked');
  return checked ? checked.value : 'json';
}

async function doExport() {
  if (!_exportCtx) return;
  const fmt = getSelectedExportFormat();
  const ext = EXPORT_FORMATS.find(f => f.id === fmt)?.ext || 'json';

  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;

  const filePath = await dialogApi.save({
    title: `Export ${_exportCtx.title}`,
    defaultPath: _exportCtx.defaultName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
  if (!filePath) return;

  // Show progress
  const progress = document.getElementById('exportProgress');
  const actions = document.getElementById('exportActions');
  const fill = document.getElementById('exportProgressFill');
  const text = document.getElementById('exportProgressText');
  if (progress) progress.style.display = '';
  if (actions) actions.style.display = 'none';
  if (fill) { fill.style.width = '0%'; fill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite'; }
  if (text) text.textContent = `Exporting as ${ext.toUpperCase()}...`;

  // Close modal immediately and run export in background
  const title = _exportCtx.title;
  const exportFn = _exportCtx.exportFn;
  closeExportModal();
  showToast(`Exporting ${title} as ${ext.toUpperCase()}...`);
  showGlobalProgress();
  exportFn(fmt, filePath).then(() => {
    showToast(`${title} exported as ${ext.toUpperCase()}`);
  }).catch(err => {
    showToast(`Export failed — ${err.message || err || 'Unknown error'}`, 4000, 'error');
  }).finally(() => {
    hideGlobalProgress();
  });
}

// Event delegation for export modal
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action-modal]');
  if (!action) return;
  const act = action.dataset.actionModal;
  if (act === 'closeExport') {
    if (e.target === action || action.classList.contains('modal-close') || action.classList.contains('btn-secondary')) {
      closeExportModal();
    }
  } else if (act === 'confirmExport') {
    doExport();
  }
});

// ── Per-tab export functions (open modal) ──

function exportPlugins() {
  if (allPlugins.length === 0) return;
  _exportCtx = {
    title: 'Plugin Inventory',
    defaultName: exportFileName('plugins', allPlugins.length),
    exportFn: async (fmt, filePath) => {
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
    }
  };
  showExportModal('plugins', 'Plugin Inventory', allPlugins.length);
}

function exportAudio() {
  if (allAudioSamples.length === 0) return;
  _exportCtx = {
    title: 'Audio Samples',
    defaultName: exportFileName('samples', allAudioSamples.length),
    exportFn: async (fmt, filePath) => {
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
    }
  };
  showExportModal('audio', 'Audio Samples', allAudioSamples.length);
}

function exportDaw() {
  if (allDawProjects.length === 0) return;
  _exportCtx = {
    title: 'DAW Projects',
    defaultName: exportFileName('daw-projects', allDawProjects.length),
    exportFn: async (fmt, filePath) => {
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
    }
  };
  showExportModal('daw', 'DAW Projects', allDawProjects.length);
}

function exportPresets() {
  if (allPresets.length === 0) return;
  _exportCtx = {
    title: 'Presets',
    defaultName: exportFileName('presets', allPresets.length),
    exportFn: async (fmt, filePath) => {
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
    }
  };
  showExportModal('presets', 'Presets', allPresets.length);
}

// ── Import functions (unchanged — use native file dialog) ──

async function importPlugins() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Plugin Inventory', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
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
  } finally { hideGlobalProgress(); }
}

async function importAudio() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Audio Sample List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  showGlobalProgress();
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
  } catch (err) { await showImportError('audio', err.message || String(err)); } finally { hideGlobalProgress(); }
}

async function importDaw() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import DAW Project List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  showGlobalProgress();
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
  } catch (err) { await showImportError('daw', err.message || String(err)); } finally { hideGlobalProgress(); }
}

async function importPresets() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Preset List', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  showGlobalProgress();
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
  } catch (err) { await showImportError('presets', err.message || String(err)); } finally { hideGlobalProgress(); }
}
