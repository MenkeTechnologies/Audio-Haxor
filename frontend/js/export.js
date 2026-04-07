// ── Export / Import ──

function _exportFmt(key, vars) {
    return catalogFmt(key, vars);
}

function resolveExportTitle(ctx) {
    if (!ctx) return '';
    if (ctx.titleKey) return _exportFmt(ctx.titleKey, ctx.titleVars || {});
    return ctx.title || '';
}

function pdfHeaders(...keys) {
    return keys.map((k) => _exportFmt(k));
}

function getAllImportFilters() {
    return [
        {name: _exportFmt('ui.export.filter_all'), extensions: ['json', 'toml']},
        {name: _exportFmt('ui.export.fmt_json'), extensions: ['json']},
        {name: _exportFmt('ui.export.fmt_toml'), extensions: ['toml']},
    ];
}

if (typeof window !== 'undefined') {
    Object.defineProperty(window, 'ALL_IMPORT_FILTERS', {
        configurable: true,
        enumerable: true,
        get() {
            return getAllImportFilters();
        },
    });
}

function updateExportButton() {
    const btn = document.getElementById('btnExport');
    if (!btn) return;
    const n = typeof getPluginExportableCount === 'function'
        ? getPluginExportableCount()
        : (typeof allPlugins !== 'undefined' ? allPlugins.length : 0);
    btn.style.display = n > 0 ? '' : 'none';
}

async function showImportError(kind, err) {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    const title = _exportFmt('ui.export.import_error_title');
    const msg = `${title}: ${err}`;
    if (dialogApi && dialogApi.message) {
        await dialogApi.message(msg, {title, kind: 'error'});
    } else {
        showToast(msg, 4000, 'error');
    }
}

// ── Export Modal ──

function exportFileName(label) {
    const now = new Date();
    const ts = now.toISOString().slice(0, 19).replace(/[T:]/g, '-');
    return `audiohaxor-${label}-${ts}`;
}

const EXPORT_FORMAT_DEFS = [
    {id: 'json', labelKey: 'ui.export.fmt_json', ext: 'json', icon: '{ }', descKey: 'ui.export.fmt_json_desc'},
    {id: 'toml', labelKey: 'ui.export.fmt_toml', ext: 'toml', icon: '[T]', descKey: 'ui.export.fmt_toml_desc'},
    {id: 'csv', labelKey: 'ui.export.fmt_csv', ext: 'csv', icon: ',,,', descKey: 'ui.export.fmt_csv_desc'},
    {id: 'tsv', labelKey: 'ui.export.fmt_tsv', ext: 'tsv', icon: '\\t', descKey: 'ui.export.fmt_tsv_desc'},
    {id: 'pdf', labelKey: 'ui.export.fmt_pdf', ext: 'pdf', icon: '&#128196;', descKey: 'ui.export.fmt_pdf_desc'},
];

function getExportFormatOptions() {
    return EXPORT_FORMAT_DEFS.map((d) => ({
        id: d.id,
        ext: d.ext,
        icon: d.icon,
        label: _exportFmt(d.labelKey),
        desc: _exportFmt(d.descKey),
    }));
}

let _exportCtx = null; // { titleKey, titleVars, defaultName, exportFn }

function showExportModal(type, titleKey, itemCount, titleVars) {
    let existing = document.getElementById('exportModal');
    if (existing) existing.remove();

    const title = _exportFmt(titleKey, titleVars || {});
    const modalHeading = _exportFmt('ui.export.modal_title', {title});
    const formats = getExportFormatOptions();
    const closeT = _exportFmt('ui.export.close');
    const itemsLine = _exportFmt('ui.export.items_count', {n: itemCount.toLocaleString()});
    const exporting = _exportFmt('ui.export.exporting');
    const btnExport = _exportFmt('ui.export.btn_export');
    const btnCancel = _exportFmt('ui.export.btn_cancel');

    const html = `<div class="modal-overlay" id="exportModal" data-action-modal="closeExport">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>${escapeHtml(modalHeading)}</h2>
        <button class="modal-close" data-action-modal="closeExport" title="${escapeHtml(closeT)}">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="export-info">
          <span class="export-count">${escapeHtml(itemsLine)}</span>
          <span class="export-type">${escapeHtml(title)}</span>
        </div>
        <div class="export-formats" id="exportFormats">
          ${formats.map(f => `
            <label class="export-format-option ${f.id === 'json' ? 'selected' : ''}">
              <input type="radio" name="exportFmt" value="${f.id}" ${f.id === 'json' ? 'checked' : ''}>
              <span class="export-fmt-icon">${f.icon}</span>
              <div class="export-fmt-info">
                <span class="export-fmt-label">${escapeHtml(f.label)}</span>
                <span class="export-fmt-desc">${escapeHtml(f.desc)}</span>
              </div>
              <span class="export-fmt-ext">.${f.ext}</span>
            </label>
          `).join('')}
        </div>
        <div class="export-progress" id="exportProgress" style="display:none;">
          <div class="export-progress-bar"><div class="export-progress-fill" id="exportProgressFill"></div></div>
          <span class="export-progress-text" id="exportProgressText">${escapeHtml(exporting)}</span>
        </div>
        <div class="export-actions" id="exportActions">
          <button class="btn btn-primary" data-action-modal="confirmExport" title="${escapeHtml(btnExport)}">&#8615; ${escapeHtml(btnExport)}</button>
          <button class="btn btn-secondary" data-action-modal="closeExport" title="${escapeHtml(btnCancel)}">${escapeHtml(btnCancel)}</button>
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
    const ext = EXPORT_FORMAT_DEFS.find(f => f.id === fmt)?.ext || 'json';

    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;

    const exportTitle = resolveExportTitle(_exportCtx);
    const filePath = await dialogApi.save({
        title: _exportFmt('ui.export.modal_title', {title: exportTitle}),
        defaultPath: _exportCtx.defaultName,
        filters: [{name: ext.toUpperCase(), extensions: [ext]}],
    });
    if (!filePath) return;

    // Show progress
    const progress = document.getElementById('exportProgress');
    const actions = document.getElementById('exportActions');
    const fill = document.getElementById('exportProgressFill');
    const text = document.getElementById('exportProgressText');
    if (progress) progress.style.display = '';
    if (actions) actions.style.display = 'none';
    if (fill) {
        fill.style.width = '0%';
        fill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
    }
    if (text) text.textContent = _exportFmt('ui.export.exporting_as', {ext: ext.toUpperCase()});

    // Close modal immediately and run export in background
    const title = exportTitle;
    const exportFn = _exportCtx.exportFn;
    closeExportModal();
    showToast(toastFmt('toast.exporting_title_as', {title, ext: ext.toUpperCase()}));
    showGlobalProgress();
    exportFn(fmt, filePath).then(() => {
        showToast(toastFmt('toast.exported_title_as', {title, ext: ext.toUpperCase()}));
    }).catch(err => {
        showToast(toastFmt('toast.export_failed', {err: err.message || err || 'Unknown error'}), 4000, 'error');
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

// ── Shared writers (full-library export + batch selection use the same formats / dialog) ──

function enrichSamplesForExport(items) {
    if (!items || !items.length) return items;
    const find = typeof findByPath === 'function' ? findByPath : null;
    if (!find) return items;
    const pools = [];
    if (typeof filteredAudioSamples !== 'undefined' && filteredAudioSamples) pools.push(filteredAudioSamples);
    if (typeof allAudioSamples !== 'undefined' && allAudioSamples) pools.push(allAudioSamples);
    return items.map((it) => {
        if (!it || !it.path) return it;
        for (const arr of pools) {
            const f = find(arr, it.path);
            if (f) return f;
        }
        return it;
    });
}

function enrichDawForExport(items) {
    if (!items || !items.length) return items;
    const find = typeof findByPath === 'function' ? findByPath : null;
    if (!find) return items;
    const pools = [];
    if (typeof filteredDawProjects !== 'undefined' && filteredDawProjects) pools.push(filteredDawProjects);
    if (typeof allDawProjects !== 'undefined' && allDawProjects) pools.push(allDawProjects);
    return items.map((it) => {
        if (!it || !it.path) return it;
        for (const arr of pools) {
            const f = find(arr, it.path);
            if (f) return f;
        }
        return it;
    });
}

function enrichPresetsForExport(items) {
    if (!items || !items.length) return items;
    const find = typeof findByPath === 'function' ? findByPath : null;
    if (!find) return items;
    const pools = [];
    if (typeof filteredPresets !== 'undefined' && filteredPresets) pools.push(filteredPresets);
    if (typeof allPresets !== 'undefined' && allPresets) pools.push(allPresets);
    return items.map((it) => {
        if (!it || !it.path) return it;
        for (const arr of pools) {
            const f = find(arr, it.path);
            if (f) return f;
        }
        return it;
    });
}

function enrichMidiForExport(items) {
    if (!items || !items.length) return items;
    const find = typeof findByPath === 'function' ? findByPath : null;
    if (!find) return items;
    const pools = [];
    if (typeof filteredMidi !== 'undefined' && filteredMidi) pools.push(filteredMidi);
    if (typeof allMidiFiles !== 'undefined' && allMidiFiles) pools.push(allMidiFiles);
    return items.map((it) => {
        if (!it || !it.path) return it;
        for (const arr of pools) {
            const f = find(arr, it.path);
            if (f) return f;
        }
        return it;
    });
}

function enrichPdfsForExport(items) {
    if (!items || !items.length) return items;
    const find = typeof findByPath === 'function' ? findByPath : null;
    if (!find) return items;
    const pools = [];
    if (typeof filteredPdfs !== 'undefined' && filteredPdfs) pools.push(filteredPdfs);
    if (typeof allPdfs !== 'undefined' && allPdfs) pools.push(allPdfs);
    return items.map((it) => {
        if (!it || !it.path) return it;
        for (const arr of pools) {
            const f = find(arr, it.path);
            if (f) return f;
        }
        return it;
    });
}

async function writeAudioExport(s, fmt, filePath) {
    if (fmt === 'pdf') {
        const headers = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_format',
            'ui.export.col_size',
            'ui.export.col_modified',
            'ui.export.col_path',
        );
        const rows = s.map(row => [row.name, row.format, row.sizeFormatted, row.modified, row.directory]);
        await window.vstUpdater.exportPdf(_exportFmt('ui.export.pdf_audio_library'), headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
        await window.vstUpdater.exportAudioDsv(s, filePath);
    } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({samples: s}, filePath);
    } else {
        await window.vstUpdater.exportAudioJson(s, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
}

async function writeDawExport(pr, fmt, filePath) {
    if (fmt === 'pdf') {
        const headers = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_daw',
            'ui.export.col_format',
            'ui.export.col_size',
            'ui.export.col_modified',
            'ui.export.col_path',
        );
        const rows = pr.map(p => [p.name, p.daw, p.format, p.sizeFormatted, p.modified, p.directory]);
        await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_daw_projects'), headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
        await window.vstUpdater.exportDawDsv(pr, filePath);
    } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({projects: pr}, filePath);
    } else {
        await window.vstUpdater.exportDawJson(pr, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
}

async function writePresetsExport(pr, fmt, filePath) {
    if (fmt === 'pdf') {
        const headers = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_format',
            'ui.export.col_size',
            'ui.export.col_modified',
            'ui.export.col_path',
        );
        const rows = pr.map(p => [p.name, p.format, p.sizeFormatted || '', p.modified, p.directory]);
        await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_presets'), headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
        await window.vstUpdater.exportPresetsDsv(pr, filePath);
    } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({presets: pr}, filePath);
    } else {
        await window.vstUpdater.exportPresetsJson(pr, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
}

async function writePdfsExport(pdf, fmt, filePath) {
    if (fmt === 'pdf') {
        const headers = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_size',
            'ui.export.col_modified',
            'ui.export.col_path',
        );
        const rows = pdf.map(p => [p.name, p.sizeFormatted || '', p.modified, p.directory]);
        await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_pdfs'), headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
        await window.vstUpdater.exportPdfsDsv(pdf, filePath);
    } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({pdfs: pdf}, filePath);
    } else {
        await window.vstUpdater.exportPdfsJson(pdf, filePath.endsWith('.json') ? filePath : filePath + '.json');
    }
}

async function writeMidiExport(list, fmt, filePath) {
    const data = list.map(s => {
        const info = typeof _midiInfoCache !== 'undefined' ? _midiInfoCache[s.path] || {} : {};
        return {
            name: s.name,
            path: s.path,
            directory: s.directory,
            format: s.format,
            size: s.size,
            sizeFormatted: s.sizeFormatted,
            modified: s.modified,
            tracks: info.trackCount,
            tempo: info.tempo,
            timeSignature: info.timeSignature,
            keySignature: info.keySignature,
            noteCount: info.noteCount,
            channelsUsed: info.channelsUsed,
            duration: info.duration,
            trackNames: info.trackNames
        };
    });
    if (fmt === 'pdf') {
        const headers = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_tracks',
            'ui.export.col_bpm',
            'ui.export.col_time_sig',
            'ui.export.col_key',
            'ui.export.col_notes',
            'ui.export.col_ch',
            'ui.export.col_duration',
            'ui.export.col_size',
            'ui.export.col_path',
        );
        const rows = data.map(m => [m.name, String(m.tracks ?? ''), String(m.tempo ?? ''), m.timeSignature || '', m.keySignature || '', String(m.noteCount ?? ''), String(m.channelsUsed ?? ''), m.duration ? (typeof formatTime === 'function' ? formatTime(m.duration) : m.duration + 's') : '', m.sizeFormatted, m.directory].map(v => String(v)));
        await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_midi'), headers, rows, filePath);
    } else if (fmt === 'csv' || fmt === 'tsv') {
        const sep = fmt === 'tsv' ? '\t' : ',';
        const esc = (v) => {
            const s = String(v ?? '');
            return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s;
        };
        const hdr = pdfHeaders(
            'ui.export.col_name',
            'ui.export.col_tracks',
            'ui.export.col_bpm',
            'ui.export.col_time_sig',
            'ui.export.col_key',
            'ui.export.col_notes',
            'ui.export.col_channels',
            'ui.export.col_duration',
            'ui.export.col_size',
            'ui.export.col_path',
        );
        const lines = [hdr.map(esc).join(sep)];
        for (const m of data) lines.push([m.name, m.tracks, m.tempo, m.timeSignature, m.keySignature, m.noteCount, m.channelsUsed, m.duration, m.sizeFormatted, m.directory].map(esc).join(sep));
        await window.vstUpdater.writeTextFile(filePath, lines.join('\n'));
    } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({midi: data}, filePath);
    } else {
        const json = JSON.stringify(data, null, 2);
        await window.vstUpdater.writeTextFile(filePath, json);
    }
}

/** Batch bar: same export modal as the toolbar, only selected rows (Samples / DAW / Presets / MIDI / PDF). */
async function exportAudioSubset(itemsRaw) {
    const samples = enrichSamplesForExport(itemsRaw);
    if (!samples || samples.length === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_audio_samples',
        defaultName: exportFileName('samples-selection'),
        exportFn: async (fmt, filePath) => {
            await writeAudioExport(samples, fmt, filePath);
        }
    };
    showExportModal('audio', 'ui.export.title_audio_samples', samples.length);
}

async function exportDawSubset(itemsRaw) {
    const projects = enrichDawForExport(itemsRaw);
    if (!projects || projects.length === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_daw_projects',
        defaultName: exportFileName('daw-projects-selection'),
        exportFn: async (fmt, filePath) => {
            await writeDawExport(projects, fmt, filePath);
        }
    };
    showExportModal('daw', 'ui.export.title_daw_projects', projects.length);
}

async function exportPresetsSubset(itemsRaw) {
    const presets = enrichPresetsForExport(itemsRaw);
    if (!presets || presets.length === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_presets',
        defaultName: exportFileName('presets-selection'),
        exportFn: async (fmt, filePath) => {
            await writePresetsExport(presets, fmt, filePath);
        }
    };
    showExportModal('presets', 'ui.export.title_presets', presets.length);
}

async function exportMidiSubset(itemsRaw) {
    const list = enrichMidiForExport(itemsRaw);
    if (!list || list.length === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_midi',
        defaultName: exportFileName('midi-selection'),
        exportFn: async (fmt, filePath) => {
            await writeMidiExport(list, fmt, filePath);
        }
    };
    showExportModal('midi', 'ui.export.title_midi', list.length);
}

async function exportPdfsSubset(itemsRaw) {
    const pdfs = enrichPdfsForExport(itemsRaw);
    if (!pdfs || pdfs.length === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_pdfs',
        defaultName: exportFileName('pdfs-selection'),
        exportFn: async (fmt, filePath) => {
            await writePdfsExport(pdfs, fmt, filePath);
        }
    };
    showExportModal('pdfs', 'ui.export.title_pdfs', pdfs.length);
}

// ── Per-tab export functions (open modal) ──

async function exportPlugins() {
    let plugins = null;
    if (typeof scanProgressCleanup === 'undefined' || !scanProgressCleanup) {
        plugins = typeof allPlugins !== 'undefined' && allPlugins.length > 0 ? allPlugins.slice() : null;
    }
    const countForModal = plugins && plugins.length > 0
        ? plugins.length
        : (typeof getPluginExportableCount === 'function' ? getPluginExportableCount() : 0);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_plugin_inventory',
        defaultName: exportFileName('plugins'),
        exportFn: async (fmt, filePath) => {
            let pl = plugins;
            if (!pl || pl.length === 0) {
                if (typeof fetchPluginsForExport !== 'function') {
                    throw new Error('fetchPluginsForExport unavailable');
                }
                pl = await fetchPluginsForExport();
            }
            if (!pl || pl.length === 0) {
                throw new Error('No plugins to export');
            }
            if (fmt === 'pdf') {
                const headers = pdfHeaders(
                    'ui.export.col_name',
                    'ui.export.col_type',
                    'ui.export.col_version',
                    'ui.export.col_manufacturer',
                    'ui.export.col_architecture',
                    'ui.export.col_size',
                    'ui.export.col_modified',
                );
                const rows = pl.map(p => [p.name, p.type, p.version, p.manufacturer || '', (p.architectures || []).join(', '), p.size, p.modified]);
                await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_plugin_inventory'), headers, rows, filePath);
            } else if (fmt === 'csv' || fmt === 'tsv') {
                await window.vstUpdater.exportCsv(pl, filePath);
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({plugins: pl}, filePath);
            } else {
                await window.vstUpdater.exportJson(pl, filePath.endsWith('.json') ? filePath : filePath + '.json');
            }
        }
    };
    showExportModal('plugins', 'ui.export.title_plugin_inventory', countForModal);
}

async function exportAudio() {
    let samples = null;
    if (typeof audioScanProgressCleanup === 'undefined' || !audioScanProgressCleanup) {
        samples = typeof allAudioSamples !== 'undefined' && allAudioSamples.length > 0 ? allAudioSamples : null;
    }
    const pageHint = typeof filteredAudioSamples !== 'undefined' && filteredAudioSamples ? filteredAudioSamples.length : 0;
    const countForModal = samples && samples.length > 0
        ? samples.length
        : Math.max(Number(typeof audioTotalCount !== 'undefined' ? audioTotalCount : 0) || 0, pageHint);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_audio_samples',
        defaultName: exportFileName('samples'),
        exportFn: async (fmt, filePath) => {
            let s = samples;
            if (!s || s.length === 0) {
                if (typeof fetchAudioSamplesForExport !== 'function') {
                    throw new Error('fetchAudioSamplesForExport unavailable');
                }
                s = await fetchAudioSamplesForExport();
            }
            if (!s || s.length === 0) {
                throw new Error('No samples to export');
            }
            await writeAudioExport(s, fmt, filePath);
        }
    };
    showExportModal('audio', 'ui.export.title_audio_samples', countForModal);
}

async function exportDaw() {
    let projects = null;
    if (typeof dawScanProgressCleanup === 'undefined' || !dawScanProgressCleanup) {
        projects = typeof allDawProjects !== 'undefined' && allDawProjects.length > 0 ? allDawProjects : null;
    }
    const pageHint = typeof filteredDawProjects !== 'undefined' && filteredDawProjects ? filteredDawProjects.length : 0;
    const countForModal = projects && projects.length > 0
        ? projects.length
        : Math.max(Number(_dawTotalCount) || 0, pageHint);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_daw_projects',
        defaultName: exportFileName('daw-projects'),
        exportFn: async (fmt, filePath) => {
            let pr = projects;
            if (!pr || pr.length === 0) {
                if (typeof fetchDawProjectsForExport !== 'function') {
                    throw new Error('fetchDawProjectsForExport unavailable');
                }
                pr = await fetchDawProjectsForExport();
            }
            if (!pr || pr.length === 0) {
                throw new Error('No DAW projects to export');
            }
            await writeDawExport(pr, fmt, filePath);
        }
    };
    showExportModal('daw', 'ui.export.title_daw_projects', countForModal);
}

async function exportPdfs() {
    let pdfs = null;
    if (typeof pdfScanProgressCleanup === 'undefined' || !pdfScanProgressCleanup) {
        pdfs = typeof allPdfs !== 'undefined' && allPdfs.length > 0 ? allPdfs : null;
    }
    const pageHint = typeof filteredPdfs !== 'undefined' && filteredPdfs ? filteredPdfs.length : 0;
    const countForModal = pdfs && pdfs.length > 0
        ? pdfs.length
        : Math.max(Number(typeof _pdfTotalCount !== 'undefined' ? _pdfTotalCount : 0) || 0, pageHint);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_pdfs',
        defaultName: exportFileName('pdfs'),
        exportFn: async (fmt, filePath) => {
            let pdf = pdfs;
            if (!pdf || pdf.length === 0) {
                if (typeof fetchPdfsForExport !== 'function') {
                    throw new Error('fetchPdfsForExport unavailable');
                }
                pdf = await fetchPdfsForExport();
            }
            if (!pdf || pdf.length === 0) {
                throw new Error('No PDFs to export');
            }
            await writePdfsExport(pdf, fmt, filePath);
        }
    };
    showExportModal('pdfs', 'ui.export.title_pdfs', countForModal);
}

async function importPdfs() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: _exportFmt('ui.export.dialog_import_pdfs'),
        multiple: false,
        filters: getAllImportFilters()
    });
    if (!selected) return;
    const filePath = typeof selected === 'string' ? selected : selected.path;
    if (!filePath) return;
    showGlobalProgress();
    try {
        let imported;
        if (filePath.endsWith('.toml')) {
            const data = await window.vstUpdater.importToml(filePath);
            imported = data.pdfs || data;
        } else {
            imported = await window.vstUpdater.importPdfsJson(filePath);
        }
        if (!imported || !Array.isArray(imported) || imported.length === 0) {
            await showImportError('pdfs', _exportFmt('ui.export.import_empty_pdfs'));
            return;
        }
        allPdfs = imported;
        if (typeof rebuildPdfStats === 'function') rebuildPdfStats();
        if (typeof filterPdfs === 'function') filterPdfs();
        const btn = document.getElementById('btnExportPdf');
        if (btn) btn.style.display = '';
        showToast(toastFmt('toast.imported_n_pdfs', {n: imported.length}));
    } catch (err) {
        await showImportError('pdfs', err.message || String(err));
    } finally {
        hideGlobalProgress();
    }
}

async function exportPresets() {
    let presets = null;
    if (typeof presetScanProgressCleanup === 'undefined' || !presetScanProgressCleanup) {
        presets = typeof allPresets !== 'undefined' && allPresets.length > 0 ? allPresets.slice() : null;
    }
    const pageHint = typeof filteredPresets !== 'undefined' && filteredPresets ? filteredPresets.length : 0;
    const countForModal = presets && presets.length > 0
        ? presets.length
        : Math.max(Number(typeof _presetTotalCount !== 'undefined' ? _presetTotalCount : 0) || 0, pageHint);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_presets',
        defaultName: exportFileName('presets'),
        exportFn: async (fmt, filePath) => {
            let pr = presets;
            if (!pr || pr.length === 0) {
                if (typeof fetchPresetsForExport !== 'function') {
                    throw new Error('fetchPresetsForExport unavailable');
                }
                pr = await fetchPresetsForExport();
            }
            if (!pr || pr.length === 0) {
                throw new Error('No presets to export');
            }
            await writePresetsExport(pr, fmt, filePath);
        }
    };
    showExportModal('presets', 'ui.export.title_presets', countForModal);
}

// ── MIDI export ──

async function exportMidi() {
    let midiList = null;
    if (typeof _midiScanProgressCleanup === 'undefined' || !_midiScanProgressCleanup) {
        midiList = typeof allMidiFiles !== 'undefined' && allMidiFiles.length > 0 ? allMidiFiles : null;
    }
    const pageHint = typeof filteredMidi !== 'undefined' && filteredMidi ? filteredMidi.length : 0;
    const countForModal = midiList && midiList.length > 0
        ? midiList.length
        : Math.max(Number(typeof _midiTotalCount !== 'undefined' ? _midiTotalCount : 0) || 0, pageHint);
    if (countForModal === 0) {
        showToast(toastFmt('toast.no_list_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_midi',
        defaultName: exportFileName('midi'),
        exportFn: async (fmt, filePath) => {
            let list = midiList;
            if (!list || list.length === 0) {
                if (typeof fetchMidiFilesForExport !== 'function') {
                    throw new Error('fetchMidiFilesForExport unavailable');
                }
                list = await fetchMidiFilesForExport();
            }
            if (!list || list.length === 0) {
                throw new Error('No MIDI files to export');
            }
            await writeMidiExport(list, fmt, filePath);
        }
    };
    showExportModal('midi', 'ui.export.title_midi', countForModal);
}

// ── Xref/Dependency export ──

function exportXref() {
    const cache = typeof _xrefCache !== 'undefined' ? _xrefCache : {};
    const entries = Object.entries(cache).filter(([, v]) => v && v.length > 0);
    if (entries.length === 0) {
        showToast(toastFmt('toast.no_xref_build_index'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_xref',
        defaultName: exportFileName('xref', entries.length),
        exportFn: async (fmt, filePath) => {
            if (fmt === 'pdf') {
                const headers = pdfHeaders(
                    'ui.export.col_project',
                    'ui.export.col_plugin',
                    'ui.export.col_type',
                    'ui.export.col_manufacturer',
                );
                const rows = [];
                for (const [project, plugins] of entries) {
                    const pName = project.split('/').pop() || project;
                    for (const p of plugins) rows.push([pName, p.name, p.pluginType || '', p.manufacturer || '']);
                }
                await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_xref'), headers, rows, filePath);
            } else if (fmt === 'csv' || fmt === 'tsv') {
                const sep = fmt === 'tsv' ? '\t' : ',';
                const esc = (v) => {
                    const s = String(v ?? '');
                    return s.includes(sep) || s.includes('"') ? '"' + s.replace(/"/g, '""') + '"' : s;
                };
                const hdr = pdfHeaders(
                    'ui.export.col_project',
                    'ui.export.col_plugin',
                    'ui.export.col_type',
                    'ui.export.col_manufacturer',
                );
                const lines = [hdr.map(esc).join(sep)];
                for (const [project, plugins] of entries) for (const p of plugins) lines.push([project, p.name, p.pluginType || '', p.manufacturer || ''].map(esc).join(sep));
                await window.vstUpdater.writeTextFile(filePath, lines.join('\n'));
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({xref: Object.fromEntries(entries)}, filePath);
            } else {
                await window.vstUpdater.writeTextFile(filePath, JSON.stringify(Object.fromEntries(entries), null, 2));
            }
        }
    };
    showExportModal('xref', 'ui.export.title_xref', entries.length);
}

// ── Smart Playlists export/import ──

function exportSmartPlaylists() {
    const playlists = typeof prefs !== 'undefined' ? prefs.getObject('smartPlaylists', []) : [];
    if (playlists.length === 0) {
        showToast(toastFmt('toast.no_smart_playlists_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.title_smart_playlists',
        defaultName: exportFileName('smart-playlists', playlists.length),
        exportFn: async (fmt, filePath) => {
            if (fmt === 'pdf') {
                const headers = pdfHeaders(
                    'ui.export.col_name',
                    'ui.export.col_rules',
                    'ui.export.col_match_mode',
                );
                const rows = playlists.map(p => [
                    p.name || _exportFmt('ui.sp_untitled'),
                    `${(p.rules || []).length} ${_exportFmt('ui.export.col_rules')}`,
                    p.matchMode || 'AND',
                ]);
                await window.vstUpdater.exportPdf(_exportFmt('ui.export.title_smart_playlists'), headers, rows, filePath);
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({playlists}, filePath);
            } else {
                await window.vstUpdater.writeTextFile(filePath, JSON.stringify(playlists, null, 2));
            }
        }
    };
    showExportModal('playlists', 'ui.export.title_smart_playlists', playlists.length);
}

// ── Import functions (unchanged — use native file dialog) ──

async function importPlugins() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: _exportFmt('ui.export.dialog_import_plugins'),
        multiple: false,
        filters: getAllImportFilters()
    });
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
            await showImportError('plugins', _exportFmt('ui.export.import_empty_plugins'));
            return;
        }
        allPlugins = imported;
        if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: imported.length});
        else document.getElementById('totalCount').textContent = allPlugins.length.toLocaleString();
        document.getElementById('btnCheckUpdates').disabled = false;
        document.getElementById('btnExport').style.display = '';
        renderPlugins(allPlugins);
        resolveKvrDownloads();
        showToast(toastFmt('toast.imported_n_plugins', {n: imported.length}));
    } catch (err) {
        await showImportError('plugins', err.message || String(err));
    } finally {
        hideGlobalProgress();
    }
}

async function importAudio() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: _exportFmt('ui.export.dialog_import_audio'),
        multiple: false,
        filters: getAllImportFilters()
    });
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
            await showImportError('audio', _exportFmt('ui.export.import_empty_audio'));
            return;
        }
        allAudioSamples = imported;
        await rebuildAudioStats(true);
        filterAudioSamples();
        document.getElementById('btnExportAudio').style.display = '';
        showToast(toastFmt('toast.imported_n_samples', {n: imported.length}));
    } catch (err) {
        await showImportError('audio', err.message || String(err));
    } finally {
        hideGlobalProgress();
    }
}

async function importDaw() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: _exportFmt('ui.export.dialog_import_daw'),
        multiple: false,
        filters: getAllImportFilters()
    });
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
            await showImportError('daw', _exportFmt('ui.export.import_empty_daw'));
            return;
        }
        allDawProjects = imported;
        refreshDawStatsFromMemory();
        filterDawProjects();
        document.getElementById('btnExportDaw').style.display = '';
        showToast(toastFmt('toast.imported_n_daw', {n: imported.length}));
    } catch (err) {
        await showImportError('daw', err.message || String(err));
    } finally {
        hideGlobalProgress();
    }
}

async function importPresets() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({
        title: _exportFmt('ui.export.dialog_import_presets'),
        multiple: false,
        filters: getAllImportFilters()
    });
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
            await showImportError('presets', _exportFmt('ui.export.import_empty_presets'));
            return;
        }
        allPresets = imported;
        rebuildPresetStats();
        filterPresets();
        if (typeof updatePresetExportButton === 'function') updatePresetExportButton();
        else document.getElementById('btnExportPresets').style.display = '';
        showToast(toastFmt('toast.imported_n_presets', {n: imported.length}));
    } catch (err) {
        await showImportError('presets', err.message || String(err));
    } finally {
        hideGlobalProgress();
    }
}

function exportXrefPlugins() {
    const plugins = window._xrefExportPlugins || [];
    const projectName = window._xrefExportProjectName || 'Project';
    if (plugins.length === 0) {
        showToast(toastFmt('toast.no_plugins_export'));
        return;
    }
    _exportCtx = {
        titleKey: 'ui.export.plugins_in_project',
        titleVars: {name: projectName},
        defaultName: exportFileName('project-plugins'),
        exportFn: async (fmt, filePath) => {
            const headers = pdfHeaders('ui.export.col_name', 'ui.export.col_type', 'ui.export.col_manufacturer');
            const rows = plugins.map(p => [p.name, p.pluginType, p.manufacturer || '']);
            if (fmt === 'pdf') {
                const projectPath = window._xrefExportProjectPath || '';
                const line1 = _exportFmt('ui.export.plugins_in_project', {name: projectName});
                const pdfTitle = projectPath ? `${line1}\n${projectPath}` : line1;
                await window.vstUpdater.exportPdf(pdfTitle, headers, rows, filePath);
            } else if (fmt === 'csv' || fmt === 'tsv') {
                const sep = fmt === 'tsv' ? '\t' : ',';
                const esc = (s) => s.includes(sep) || s.includes('"') ? '"' + s.replace(/"/g, '""') + '"' : s;
                const lines = [headers.map(esc).join(sep), ...rows.map(r => r.map(esc).join(sep))].join('\n');
                await window.vstUpdater.writeTextFile(filePath, lines);
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({project: projectName, plugins}, filePath);
            } else {
                await window.vstUpdater.writeTextFile(filePath, JSON.stringify({
                    project: projectName,
                    plugins
                }, null, 2));
            }
        }
    };
    showExportModal('xref', 'ui.export.plugins_in_project', plugins.length, {name: projectName});
}
