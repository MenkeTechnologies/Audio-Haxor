// ── History ──
let historyScanList = [];
let historyAudioScanList = [];
let historyMergedList = []; // merged + sorted
let selectedScanId = null;
let selectedScanType = null; // 'plugin' or 'audio'

let historyDawScanList = [];
let historyPresetScanList = [];
let historyPdfScanList = [];
let historyMidiScanList = [];

function historyFmt(key, vars) {
  return catalogFmt(key, vars);
}

function historyCount(n, oneKey, otherKey) {
  const num = typeof n === 'number' ? n : Number(n);
  const c = Number.isFinite(num) ? num.toLocaleString() : String(n);
  return historyFmt(num === 1 ? oneKey : otherKey, { count: c });
}

/** Prefer camelCase from IPC; accept snake_case if a serializer ever emits it. */
function historyScanCountField(s, camelKey, snakeKey) {
  const raw = s[camelKey] ?? s[snakeKey];
  if (raw == null || raw === '') return 0;
  const n = typeof raw === 'number' ? raw : Number(raw);
  return Number.isFinite(n) ? n : 0;
}

function historyEmptyDetailHtml() {
  return `<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>${escapeHtml(historyFmt('ui.history.select_scan_hint'))}</p></div>`;
}

function historyCompareBlockHtml(id, action, optionsHtml) {
  return `
      <div class="compare-controls">
        <span>${escapeHtml(historyFmt('ui.history.compare_with'))}</span>
        <select id="compareSelect">
          <option value="">${escapeHtml(historyFmt('ui.history.select_scan_option'))}</option>
          ${optionsHtml}
        </select>
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="${action}" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.compare_btn_title'))}">${escapeHtml(historyFmt('ui.history.compare_btn'))}</button>
      </div>`;
}

function historyRootsHtml(roots) {
  if (!roots || roots.length === 0) return '';
  const label = escapeHtml(historyFmt('ui.history.scanned_label'));
  return `<div class="history-detail-roots"><span style="color: var(--text-dim); font-size: 11px;">${label}</span> ${roots.map(r => `<code class="root-path">${escapeHtml(r)}</code>`).join(' ')}</div>`;
}

function historyDiffMatchHtml() {
  return `<div class="state-message"><div class="state-icon">&#10003;</div><h2>${escapeHtml(historyFmt('ui.history.diff_no_diff_title'))}</h2><p>${escapeHtml(historyFmt('ui.history.diff_no_diff_sub'))}</p></div>`;
}

/** Sidebar tag: which scanner produced this history row (reuses main tab labels). */
function historyScanTypeLabel(scanType) {
  const key =
    scanType === 'preset' ? 'menu.tab_presets' :
    scanType === 'daw' ? 'menu.tab_daw' :
    scanType === 'audio' ? 'menu.tab_samples' :
    scanType === 'pdf' ? 'menu.tab_pdf' :
    scanType === 'midi' ? 'menu.tab_midi' :
    'menu.tab_plugins';
  return catalogFmt(key);
}

/** Monotonic id so a stale in-flight History refresh does not toast after a newer load. */
let _historyFetchSeq = 0;

async function fetchHistoryListsAndRender() {
  const [pluginScans, audioScans, dawScans, presetScans, pdfScans, midiScans] = await Promise.all([
    window.vstUpdater.getScans(),
    window.vstUpdater.getAudioScans(),
    window.vstUpdater.getDawScans(),
    window.vstUpdater.getPresetScans(),
    window.vstUpdater.getPdfScans(),
    window.vstUpdater.getMidiScans(),
  ]);
  historyScanList = pluginScans;
  historyAudioScanList = audioScans;
  historyDawScanList = dawScans;
  historyPresetScanList = presetScans;
  historyPdfScanList = pdfScans;
  historyMidiScanList = midiScans;
  historyMergedList = [
    ...pluginScans.map(s => ({ ...s, _type: 'plugin' })),
    ...audioScans.map(s => ({ ...s, _type: 'audio' })),
    ...dawScans.map(s => ({ ...s, _type: 'daw' })),
    ...presetScans.map(s => ({ ...s, _type: 'preset' })),
    ...pdfScans.map(s => ({ ...s, _type: 'pdf' })),
    ...midiScans.map(s => ({ ...s, _type: 'midi' })),
  ].sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
  renderHistoryList();
}

/**
 * @param {object} [opts]
 * @param {boolean} [opts.preferCache] — from tab switch: repaint last list immediately, refresh in background (no global progress).
 */
async function loadHistory(opts) {
  const preferCache = opts && opts.preferCache === true;
  if (preferCache && historyMergedList.length > 0) {
    renderHistoryList();
    const seq = ++_historyFetchSeq;
    try {
      await fetchHistoryListsAndRender();
    } catch (e) {
      if (seq === _historyFetchSeq) {
        showToast(toastFmt('toast.failed_load_history', { err: e.message || e }), 4000, 'error');
      }
    }
    return;
  }

  showGlobalProgress();
  try {
    _historyFetchSeq += 1;
    await fetchHistoryListsAndRender();
  } catch (e) {
    showToast(toastFmt('toast.failed_load_history', { err: e.message || e }), 4000, 'error');
  } finally {
    hideGlobalProgress();
  }
}

function renderHistoryList() {
  const container = document.getElementById('historyList');
  if (historyMergedList.length === 0) {
    const p1 = escapeHtml(historyFmt('ui.p.no_scan_history_yet'));
    const p2 = escapeHtml(historyFmt('ui.history.empty_run_hint'));
    container.innerHTML = `<div class="empty-history"><div class="empty-history-icon">&#128197;</div><p>${p1}<br>${p2}</p></div>`;
    return;
  }

  container.innerHTML = historyMergedList.map(s => {
    const d = new Date(s.timestamp);
    const dateStr = d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
    const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    const selected = s.id === selectedScanId ? ' selected' : '';
    const isAudio = s._type === 'audio';
    const isDaw = s._type === 'daw';
    const isPreset = s._type === 'preset';
    const isPdf = s._type === 'pdf';
    const isMidi = s._type === 'midi';
    const icon = isPreset ? '&#127924;' : isDaw ? '&#127911;' : isAudio ? '&#127925;' : isPdf ? '&#128196;' : isMidi ? '&#127932;' : '&#127911;';
    const label = isPreset
      ? historyCount(historyScanCountField(s, 'presetCount', 'preset_count'), 'ui.history.presets_one', 'ui.history.presets_other')
      : isDaw
      ? historyCount(historyScanCountField(s, 'projectCount', 'project_count'), 'ui.history.projects_one', 'ui.history.projects_other')
      : isAudio
      ? historyCount(historyScanCountField(s, 'sampleCount', 'sample_count'), 'ui.history.samples_one', 'ui.history.samples_other')
      : isPdf
      ? historyCount(historyScanCountField(s, 'pdfCount', 'pdf_count'), 'ui.history.pdfs_one', 'ui.history.pdfs_other')
      : isMidi
      ? historyCount(historyScanCountField(s, 'midiCount', 'midi_count'), 'ui.history.midi_one', 'ui.history.midi_other')
      : historyCount(historyScanCountField(s, 'pluginCount', 'plugin_count'), 'ui.history.plugins_one', 'ui.history.plugins_other');
    const typeTag = historyScanTypeLabel(s._type);
    const typeColor = isPreset ? 'var(--orange)' : isDaw ? 'var(--magenta)' : isAudio ? 'var(--yellow)' : isPdf ? 'var(--accent)' : isMidi ? 'var(--green)' : 'var(--cyan)';
    const rootsHint = s.roots && s.roots.length > 0
      ? `<div class="history-item-roots" title="${s.roots.map(r => escapeHtml(r)).join('\n')}">${s.roots.map(r => escapeHtml(r)).join(', ')}</div>`
      : '';
    return `
      <div class="history-item${selected}" data-action="selectScan" data-id="${s.id}" data-type="${s._type}">
        <div class="history-item-date">${icon} ${escapeHtml(historyFmt('ui.history.sidebar_datetime', { date: dateStr, time: timeStr }))}</div>
        <div class="history-item-meta">
          <span style="color: ${typeColor}; font-weight: 600;">${typeTag}</span>
          <span>${label}</span>
          <span>${timeAgo(d)}</span>
        </div>
        ${rootsHint}
      </div>`;
  }).join('');
}

async function selectScan(id, type) {
  selectedScanId = id;
  selectedScanType = type || 'plugin';
  renderHistoryList();

  if (selectedScanType === 'preset') {
    await selectPresetScan(id);
    return;
  }

  if (selectedScanType === 'pdf') {
    await selectPdfScan(id);
    return;
  }

  if (selectedScanType === 'midi') {
    await selectMidiScan(id);
    return;
  }

  if (selectedScanType === 'daw') {
    await selectDawScan(id);
    return;
  }

  if (selectedScanType === 'audio') {
    await selectAudioScan(id);
    return;
  }

  const detail = await window.vstUpdater.getScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  // Build compare dropdown (other plugin scans to diff against)
  const otherScans = historyScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.pluginCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runDiff', options);
  }

  // Type breakdown
  const types = {};
  detail.plugins.forEach(p => { types[p.type] = (types[p.type] || 0) + 1; });
  const typeBreakdown = Object.entries(types).map(([t, c]) => {
    const cls = t === 'VST2' ? 'type-vst2' : t === 'VST3' ? 'type-vst3' : t === 'CLAP' ? 'type-clap' : 'type-au';
    return `<span class="plugin-type ${cls}">${t}: ${c}</span>`;
  }).join(' ');

  const rootsHtml = historyRootsHtml(detail.roots);
  const pc = historyCount(detail.pluginCount, 'ui.history.plugins_one', 'ui.history.plugins_other');
  const metaPluginsHtml = historyFmt('ui.history.meta_plugins', { time: timeStr, count: pc, types: typeBreakdown });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaPluginsHtml}</div>
        ${rootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.plugins.length, 'ui.history.footer_plugins_one', 'ui.history.footer_plugins_other')}</div>
    <div id="pluginScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const plugListEl = document.getElementById('pluginScanDetailList');
  if (plugListEl) {
    let _r = 0;
    plugListEl._items = detail.plugins;
    function _renderPlugBatch() {
      const batch = plugListEl._items.slice(_r, _r + 200);
      plugListEl.insertAdjacentHTML('beforeend', batch.map(p => {
        const tc = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : p.type === 'CLAP' ? 'type-clap' : 'type-au';
        return `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span class="plugin-type ${tc}" style="font-size:9px;">${p.type}</span>
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(p.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${escapeHtml(p.manufacturer)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${p.size}</span>
          <button class="btn-small btn-folder" data-action="openFolder" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`;
      }).join(''));
      _r += batch.length;
    }
    _renderPlugBatch();
    plugListEl.addEventListener('scroll', throttle(() => { if (plugListEl.scrollTop + plugListEl.clientHeight >= plugListEl.scrollHeight - 50) _renderPlugBatch(); }, 100));
  }
}

async function selectAudioScan(id) {
  const detail = await window.vstUpdater.getAudioScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  // Format breakdown
  const fmtBreakdown = Object.entries(detail.formatCounts || {}).map(([fmt, count]) => {
    const cls = getFormatClass(fmt);
    return `<span class="format-badge ${cls}">${fmt}: ${count}</span>`;
  }).join(' ');

  // Compare dropdown (other audio scans)
  const otherScans = historyAudioScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.sampleCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runAudioDiff', options);
  }

  const audioRootsHtml = historyRootsHtml(detail.roots);
  const sc = historyCount(detail.sampleCount, 'ui.history.samples_one', 'ui.history.samples_other');
  const metaSamplesHtml = historyFmt('ui.history.meta_samples', {
    time: timeStr,
    count: sc,
    size: formatAudioSize(detail.totalBytes),
    formats: fmtBreakdown,
  });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127925; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaSamplesHtml}</div>
        ${audioRootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteAudioScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.samples.length, 'ui.history.footer_samples_one', 'ui.history.footer_samples_other')}</div>
    <div id="audioScanDetailList" style="margin-top: 8px;max-height:400px;overflow-y:auto;"></div>`;

  // Render first 200 samples only, load more on scroll
  const listEl = document.getElementById('audioScanDetailList');
  if (listEl) {
    let _audioDetailRendered = 0;
    const PAGE = 200;
    listEl._detailSamples = detail.samples;
    function _renderAudioBatch() {
      const samples = listEl._detailSamples;
      if (!samples || _audioDetailRendered >= samples.length) return;
      const batch = samples.slice(_audioDetailRendered, _audioDetailRendered + PAGE);
      listEl.insertAdjacentHTML('beforeend', batch.map(s => {
        const fmtClass = typeof getFormatClass === 'function' ? getFormatClass(s.format) : 'format-default';
        return `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span class="format-badge ${fmtClass}" style="font-size:9px;">${s.format}</span>
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(s.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${s.sizeFormatted || ''}</span>
          <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${escapeHtml(s.path)}" title="${escapeHtml(s.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`;
      }).join(''));
      _audioDetailRendered += batch.length;
    }
    _renderAudioBatch();
    // Load more on scroll to bottom
    listEl.addEventListener('scroll', throttle(() => {
      if (listEl.scrollTop + listEl.clientHeight >= listEl.scrollHeight - 50) {
        _renderAudioBatch();
      }
    }, 100));
  }
}

async function runAudioDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffAudioScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(s => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(s.name)}</div>
            <div class="diff-plugin-detail">${s.format} &middot; ${s.sizeFormatted || formatAudioSize(s.size)} &middot; ${escapeHtml(s.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(s => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(s.name)}</div>
            <div class="diff-plugin-detail">${s.format} &middot; ${s.sizeFormatted || formatAudioSize(s.size)} &middot; ${escapeHtml(s.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function selectDawScan(id) {
  const detail = await window.vstUpdater.getDawScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  const dawBreakdown = Object.entries(detail.dawCounts || {}).map(([daw, count]) => {
    return `<span class="format-badge format-default">${daw}: ${count}</span>`;
  }).join(' ');

  const otherScans = historyDawScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.projectCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runDawDiff', options);
  }

  const dawRootsHtml = historyRootsHtml(detail.roots);
  const dc = historyCount(detail.projectCount, 'ui.history.projects_one', 'ui.history.projects_other');
  const metaDawHtml = historyFmt('ui.history.meta_daw', {
    time: timeStr,
    count: dc,
    size: formatAudioSize(detail.totalBytes),
    daws: dawBreakdown,
  });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127911; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaDawHtml}</div>
        ${dawRootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteDawScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.projects.length, 'ui.history.footer_projects_one', 'ui.history.footer_projects_other')}</div>
    <div id="dawScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const dawListEl = document.getElementById('dawScanDetailList');
  if (dawListEl) {
    let _r = 0;
    dawListEl._items = detail.projects;
    function _renderDawBatch() {
      const batch = dawListEl._items.slice(_r, _r + 200);
      dawListEl.insertAdjacentHTML('beforeend', batch.map(p =>
        `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span class="format-badge format-default" style="font-size:9px;">${escapeHtml(p.daw)}</span>
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(p.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${p.sizeFormatted || ''}</span>
          <button class="btn-small btn-folder" data-action="openDawFolder" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`
      ).join(''));
      _r += batch.length;
    }
    _renderDawBatch();
    dawListEl.addEventListener('scroll', throttle(() => { if (dawListEl.scrollTop + dawListEl.clientHeight >= dawListEl.scrollHeight - 50) _renderDawBatch(); }, 100));
  }
}

async function runDawDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffDawScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${escapeHtml(p.daw)} &middot; ${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${escapeHtml(p.daw)} &middot; ${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function deleteDawScanEntry(id) {
  await window.vstUpdater.deleteDawScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function selectPresetScan(id) {
  const detail = await window.vstUpdater.getPresetScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  const fmtBreakdown = Object.entries(detail.formatCounts || {}).map(([fmt, count]) => {
    return `<span class="format-badge format-default">${fmt}: ${count}</span>`;
  }).join(' ');

  const presetRootsHtml = historyRootsHtml(detail.roots);

  const otherScans = historyPresetScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.presetCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runPresetDiff', options);
  }

  const prc = historyCount(detail.presetCount, 'ui.history.presets_one', 'ui.history.presets_other');
  const metaPresetHtml = historyFmt('ui.history.meta_preset', {
    time: timeStr,
    count: prc,
    size: formatAudioSize(detail.totalBytes),
    formats: fmtBreakdown,
  });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127924; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaPresetHtml}</div>
        ${presetRootsHtml}
      </div>
      <button class="btn-danger" data-action="deletePresetScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.presets.length, 'ui.history.footer_presets_one', 'ui.history.footer_presets_other')}</div>
    <div id="presetScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const presetListEl = document.getElementById('presetScanDetailList');
  if (presetListEl) {
    let _r = 0;
    presetListEl._items = detail.presets;
    function _renderPresetBatch() {
      const batch = presetListEl._items.slice(_r, _r + 200);
      presetListEl.insertAdjacentHTML('beforeend', batch.map(p =>
        `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span class="format-badge format-default" style="font-size:9px;">${p.format}</span>
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(p.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${p.sizeFormatted || ''}</span>
          <button class="btn-small btn-folder" data-action="openPresetFolder" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`
      ).join(''));
      _r += batch.length;
    }
    _renderPresetBatch();
    presetListEl.addEventListener('scroll', throttle(() => { if (presetListEl.scrollTop + presetListEl.clientHeight >= presetListEl.scrollHeight - 50) _renderPresetBatch(); }, 100));
  }
}

async function runPresetDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffPresetScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function selectPdfScan(id) {
  const detail = await window.vstUpdater.getPdfScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  const pdfRootsHtml = historyRootsHtml(detail.roots);

  const otherScans = historyPdfScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.pdfCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runPdfDiff', options);
  }

  const pfc = historyCount(detail.pdfCount, 'ui.history.pdfs_one', 'ui.history.pdfs_other');
  const metaPdfHtml = historyFmt('ui.history.meta_pdf', {
    time: timeStr,
    count: pfc,
    size: formatAudioSize(detail.totalBytes),
  });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#128196; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaPdfHtml}</div>
        ${pdfRootsHtml}
      </div>
      <button class="btn-danger" data-action="deletePdfScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.pdfs.length, 'ui.history.footer_pdfs_one', 'ui.history.footer_pdfs_other')}</div>
    <div id="pdfScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const pdfListEl = document.getElementById('pdfScanDetailList');
  if (pdfListEl) {
    let _r = 0;
    pdfListEl._items = detail.pdfs;
    function _renderPdfBatch() {
      const batch = pdfListEl._items.slice(_r, _r + 200);
      pdfListEl.insertAdjacentHTML('beforeend', batch.map(p =>
        `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(p.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${p.sizeFormatted || ''}</span>
          <button class="btn-small btn-folder" data-action="openPdfFile" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`
      ).join(''));
      _r += batch.length;
    }
    _renderPdfBatch();
    pdfListEl.addEventListener('scroll', throttle(() => { if (pdfListEl.scrollTop + pdfListEl.clientHeight >= pdfListEl.scrollHeight - 50) _renderPdfBatch(); }, 100));
  }
}

async function runPdfDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffPdfScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function deletePdfScanEntry(id) {
  await window.vstUpdater.deletePdfScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function selectMidiScan(id) {
  const detail = await window.vstUpdater.getMidiScanDetail(id);
  if (!detail) return;

  const d = new Date(detail.timestamp);
  const dateStr = d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
  const timeStr = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  const fmtBreakdown = Object.entries(detail.formatCounts || {}).map(([fmt, count]) => {
    return `<span class="format-badge format-default">${fmt}: ${count}</span>`;
  }).join(' ');

  const midiRootsHtml = historyRootsHtml(detail.roots);

  const otherScans = historyMidiScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.midiCount})</option>`;
    }).join('');
    compareHtml = historyCompareBlockHtml(id, 'runMidiDiff', options);
  }

  const mc = historyCount(detail.midiCount, 'ui.history.midi_one', 'ui.history.midi_other');
  const metaMidiHtml = historyFmt('ui.history.meta_midi', {
    time: timeStr,
    count: mc,
    size: formatAudioSize(detail.totalBytes),
    formats: fmtBreakdown,
  });

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127932; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${metaMidiHtml}</div>
        ${midiRootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteMidiScanEntry" data-id="${id}" title="${escapeHtml(historyFmt('ui.history.delete_entry_title'))}">${escapeHtml(historyFmt('ui.history.delete_btn'))}</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${historyCount(detail.midiFiles.length, 'ui.history.footer_midi_one', 'ui.history.footer_midi_other')}</div>
    <div id="midiScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const midiListEl = document.getElementById('midiScanDetailList');
  if (midiListEl) {
    let _r = 0;
    midiListEl._items = detail.midiFiles;
    function _renderMidiBatch() {
      const batch = midiListEl._items.slice(_r, _r + 200);
      midiListEl.insertAdjacentHTML('beforeend', batch.map(m =>
        `<div style="display:flex;align-items:center;gap:8px;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;">
          <span class="format-badge format-default" style="font-size:9px;">${escapeHtml(m.format)}</span>
          <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(m.name)}</span>
          <span style="color:var(--text-dim);font-size:10px;">${m.sizeFormatted || ''}</span>
          <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${escapeHtml(m.path)}" title="${escapeHtml(m.path)}" style="padding:2px 4px;">&#128193;</button>
        </div>`
      ).join(''));
      _r += batch.length;
    }
    _renderMidiBatch();
    midiListEl.addEventListener('scroll', throttle(() => { if (midiListEl.scrollTop + midiListEl.clientHeight >= midiListEl.scrollHeight - 50) _renderMidiBatch(); }, 100));
  }
}

async function runMidiDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffMidiScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(m => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(m.name)}</div>
            <div class="diff-plugin-detail">${escapeHtml(m.format)} &middot; ${m.sizeFormatted || formatAudioSize(m.size)} &middot; ${escapeHtml(m.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(m => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(m.name)}</div>
            <div class="diff-plugin-detail">${escapeHtml(m.format)} &middot; ${m.sizeFormatted || formatAudioSize(m.size)} &middot; ${escapeHtml(m.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function deleteMidiScanEntry(id) {
  await window.vstUpdater.deleteMidiScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function deletePresetScanEntry(id) {
  await window.vstUpdater.deletePresetScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function deleteAudioScanEntry(id) {
  await window.vstUpdater.deleteAudioScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function runDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0 && diff.versionChanged.length === 0) {
    html = historyDiffMatchHtml();
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>${escapeHtml(historyFmt('ui.history.diff_added'))} <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.type} &middot; ${escapeHtml(p.manufacturer)} &middot; v${p.version}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_removed'))} <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.type} &middot; ${escapeHtml(p.manufacturer)} &middot; v${p.version}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.versionChanged.length > 0) {
      html += `<div class="diff-section diff-changed">
        <h3>${escapeHtml(historyFmt('ui.history.diff_version_changed'))} <span class="diff-count">${diff.versionChanged.length}</span></h3>
        ${diff.versionChanged.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.type} &middot; v${p.previousVersion} &#8594; v${p.version}</div>
          </div>`).join('')}
      </div>`;
    }
  }

  container.innerHTML = html;
}

async function deleteScanEntry(id) {
  await window.vstUpdater.deleteScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
}

async function clearAllHistory() {
  if (!await confirmAction(
    catalogFmt('confirm.clear_all_history_tab'),
    catalogFmt('ui.history.confirm_clear_title'),
  )) return;
  await Promise.all([
    window.vstUpdater.clearHistory(),
    window.vstUpdater.clearAudioHistory(),
    window.vstUpdater.clearDawHistory(),
    window.vstUpdater.clearPresetHistory(),
    window.vstUpdater.clearPdfHistory(),
    window.vstUpdater.clearMidiHistory(),
  ]);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = historyEmptyDetailHtml();
  await loadHistory();
  showToast(toastFmt('toast.all_scan_history_cleared'));
}

function timeAgo(date) {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return historyFmt('ui.history.time_ago_just_now');
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return historyFmt('ui.history.time_ago_minutes', { n: String(minutes) });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return historyFmt('ui.history.time_ago_hours', { n: String(hours) });
  const days = Math.floor(hours / 24);
  if (days < 30) return historyFmt('ui.history.time_ago_days', { n: String(days) });
  const months = Math.floor(days / 30);
  return historyFmt('ui.history.time_ago_months', { n: String(months) });
}
