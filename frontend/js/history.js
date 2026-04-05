// ── History ──
let historyScanList = [];
let historyAudioScanList = [];
let historyMergedList = []; // merged + sorted
let selectedScanId = null;
let selectedScanType = null; // 'plugin' or 'audio'

let historyDawScanList = [];
let historyPresetScanList = [];

async function loadHistory() {
  showGlobalProgress();
  try {
    const [pluginScans, audioScans, dawScans, presetScans] = await Promise.all([
      window.vstUpdater.getScans(),
      window.vstUpdater.getAudioScans(),
      window.vstUpdater.getDawScans(),
      window.vstUpdater.getPresetScans(),
    ]);
    historyScanList = pluginScans;
    historyAudioScanList = audioScans;
    historyDawScanList = dawScans;
    historyPresetScanList = presetScans;
    historyMergedList = [
      ...pluginScans.map(s => ({ ...s, _type: 'plugin' })),
      ...audioScans.map(s => ({ ...s, _type: 'audio' })),
      ...dawScans.map(s => ({ ...s, _type: 'daw' })),
      ...presetScans.map(s => ({ ...s, _type: 'preset' })),
    ].sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
    renderHistoryList();
  } catch (e) {
    showToast(toastFmt('toast.failed_load_history', { err: e.message || e }), 4000, 'error');
  } finally { hideGlobalProgress(); }
}

function renderHistoryList() {
  const container = document.getElementById('historyList');
  if (historyMergedList.length === 0) {
    container.innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#128197;</div><p>No scan history yet.<br>Run a scan to start tracking.</p></div>';
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
    const icon = isPreset ? '&#127924;' : isDaw ? '&#127911;' : isAudio ? '&#127925;' : '&#127911;';
    const label = isPreset
      ? `${s.presetCount} preset${s.presetCount !== 1 ? 's' : ''}`
      : isDaw
      ? `${s.projectCount} project${s.projectCount !== 1 ? 's' : ''}`
      : isAudio
      ? `${s.sampleCount} sample${s.sampleCount !== 1 ? 's' : ''}`
      : `${s.pluginCount} plugin${s.pluginCount !== 1 ? 's' : ''}`;
    const typeTag = isPreset ? 'Presets' : isDaw ? 'DAW Projects' : isAudio ? 'Samples' : 'Plugins';
    const typeColor = isPreset ? 'var(--orange)' : isDaw ? 'var(--magenta)' : isAudio ? 'var(--yellow)' : 'var(--cyan)';
    const rootsHint = s.roots && s.roots.length > 0
      ? `<div class="history-item-roots" title="${s.roots.map(r => escapeHtml(r)).join('\n')}">${s.roots.map(r => escapeHtml(r)).join(', ')}</div>`
      : '';
    return `
      <div class="history-item${selected}" data-action="selectScan" data-id="${s.id}" data-type="${s._type}">
        <div class="history-item-date">${icon} ${dateStr} at ${timeStr}</div>
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
    compareHtml = `
      <div class="compare-controls">
        <span>Compare with:</span>
        <select id="compareSelect">
          <option value="">Select a scan...</option>
          ${options}
        </select>
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runDiff" data-id="${id}" title="Compare with selected scan">Compare</button>
      </div>`;
  }

  // Type breakdown
  const types = {};
  detail.plugins.forEach(p => { types[p.type] = (types[p.type] || 0) + 1; });
  const typeBreakdown = Object.entries(types).map(([t, c]) => {
    const cls = t === 'VST2' ? 'type-vst2' : t === 'VST3' ? 'type-vst3' : 'type-au';
    return `<span class="plugin-type ${cls}">${t}: ${c}</span>`;
  }).join(' ');

  const rootsHtml = detail.roots && detail.roots.length > 0
    ? `<div class="history-detail-roots"><span style="color: var(--text-dim); font-size: 11px;">Scanned:</span> ${detail.roots.map(r => `<code class="root-path">${escapeHtml(r)}</code>`).join(' ')}</div>`
    : '';

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${timeStr} &middot; ${detail.pluginCount} plugins &middot; ${typeBreakdown}</div>
        ${rootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteScanEntry" data-id="${id}" title="Delete this scan entry">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${detail.plugins.length.toLocaleString()} plugins in this scan</div>
    <div id="pluginScanDetailList" style="margin-top:8px;max-height:400px;overflow-y:auto;"></div>`;
  const plugListEl = document.getElementById('pluginScanDetailList');
  if (plugListEl) {
    let _r = 0;
    plugListEl._items = detail.plugins;
    function _renderPlugBatch() {
      const batch = plugListEl._items.slice(_r, _r + 200);
      plugListEl.insertAdjacentHTML('beforeend', batch.map(p => {
        const tc = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : 'type-au';
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
    plugListEl.addEventListener('scroll', () => { if (plugListEl.scrollTop + plugListEl.clientHeight >= plugListEl.scrollHeight - 50) _renderPlugBatch(); });
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
    compareHtml = `
      <div class="compare-controls">
        <span>Compare with:</span>
        <select id="compareSelect">
          <option value="">Select a scan...</option>
          ${options}
        </select>
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runAudioDiff" data-id="${id}" title="Compare with selected scan">Compare</button>
      </div>`;
  }

  const audioRootsHtml = detail.roots && detail.roots.length > 0
    ? `<div class="history-detail-roots"><span style="color: var(--text-dim); font-size: 11px;">Scanned:</span> ${detail.roots.map(r => `<code class="root-path">${escapeHtml(r)}</code>`).join(' ')}</div>`
    : '';

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127925; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${timeStr} &middot; ${detail.sampleCount} samples &middot; ${formatAudioSize(detail.totalBytes)} &middot; ${fmtBreakdown}</div>
        ${audioRootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteAudioScanEntry" data-id="${id}" title="Delete this scan entry">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;color:var(--text-muted);font-size:11px;">${detail.samples.length.toLocaleString()} samples in this scan</div>
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
    listEl.addEventListener('scroll', () => {
      if (listEl.scrollTop + listEl.clientHeight >= listEl.scrollHeight - 50) {
        _renderAudioBatch();
      }
    });
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
    html = '<div class="state-message"><div class="state-icon">&#10003;</div><h2>No differences found</h2><p>These two scans match.</p></div>';
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>Added <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(s => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(s.name)}</div>
            <div class="diff-plugin-detail">${s.format} &middot; ${s.sizeFormatted || formatAudioSize(s.size)} &middot; ${escapeHtml(s.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>Removed <span class="diff-count">${diff.removed.length}</span></h3>
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
    compareHtml = `
      <div class="compare-controls">
        <span>Compare with:</span>
        <select id="compareSelect">
          <option value="">Select a scan...</option>
          ${options}
        </select>
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runDawDiff" data-id="${id}" title="Compare with selected scan">Compare</button>
      </div>`;
  }

  const dawRootsHtml = detail.roots && detail.roots.length > 0
    ? `<div class="history-detail-roots"><span style="color: var(--text-dim); font-size: 11px;">Scanned:</span> ${detail.roots.map(r => `<code class="root-path">${escapeHtml(r)}</code>`).join(' ')}</div>`
    : '';

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127911; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${timeStr} &middot; ${detail.projectCount} projects &middot; ${formatAudioSize(detail.totalBytes)} &middot; ${dawBreakdown}</div>
        ${dawRootsHtml}
      </div>
      <button class="btn-danger" data-action="deleteDawScanEntry" data-id="${id}" title="Delete this scan entry">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${detail.projects.length.toLocaleString()} projects in this scan</div>
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
    dawListEl.addEventListener('scroll', () => { if (dawListEl.scrollTop + dawListEl.clientHeight >= dawListEl.scrollHeight - 50) _renderDawBatch(); });
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
    html = '<div class="state-message"><div class="state-icon">&#10003;</div><h2>No differences found</h2><p>These two scans match.</p></div>';
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>Added <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${escapeHtml(p.daw)} &middot; ${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>Removed <span class="diff-count">${diff.removed.length}</span></h3>
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
  document.getElementById('historyDetail').innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>Select a scan from the sidebar to view details</p></div>';
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

  const presetRootsHtml = detail.roots && detail.roots.length > 0
    ? `<div class="history-detail-roots"><span style="color: var(--text-dim); font-size: 11px;">Scanned:</span> ${detail.roots.map(r => `<code class="root-path">${escapeHtml(r)}</code>`).join(' ')}</div>`
    : '';

  const otherScans = historyPresetScanList.filter(s => s.id !== id);
  let compareHtml = '';
  if (otherScans.length > 0) {
    const options = otherScans.map(s => {
      const od = new Date(s.timestamp);
      return `<option value="${s.id}">${od.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })} ${od.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} (${s.presetCount})</option>`;
    }).join('');
    compareHtml = `
      <div class="compare-controls">
        <span>Compare with:</span>
        <select id="compareSelect">
          <option value="">Select a scan...</option>
          ${options}
        </select>
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runPresetDiff" data-id="${id}" title="Compare with selected scan">Compare</button>
      </div>`;
  }

  const container = document.getElementById('historyDetail');
  container.innerHTML = `
    <div class="history-detail-header">
      <div>
        <h2>&#127924; ${dateStr}</h2>
        <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">${timeStr} &middot; ${detail.presetCount} presets &middot; ${formatAudioSize(detail.totalBytes)} &middot; ${fmtBreakdown}</div>
        ${presetRootsHtml}
      </div>
      <button class="btn-danger" data-action="deletePresetScanEntry" data-id="${id}" title="Delete this scan entry">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top:8px;color:var(--text-muted);font-size:11px;">${detail.presets.length.toLocaleString()} presets in this scan</div>
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
    presetListEl.addEventListener('scroll', () => { if (presetListEl.scrollTop + presetListEl.clientHeight >= presetListEl.scrollHeight - 50) _renderPresetBatch(); });
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
    html = '<div class="state-message"><div class="state-icon">&#10003;</div><h2>No differences found</h2><p>These two scans match.</p></div>';
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>Added <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.format} &middot; ${p.sizeFormatted || formatAudioSize(p.size)} &middot; ${escapeHtml(p.directory || '')}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>Removed <span class="diff-count">${diff.removed.length}</span></h3>
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

async function deletePresetScanEntry(id) {
  await window.vstUpdater.deletePresetScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>Select a scan from the sidebar to view details</p></div>';
  await loadHistory();
}

async function deleteAudioScanEntry(id) {
  await window.vstUpdater.deleteAudioScan(id);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>Select a scan from the sidebar to view details</p></div>';
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
    html = '<div class="state-message"><div class="state-icon">&#10003;</div><h2>No differences found</h2><p>These two scans match.</p></div>';
  } else {
    if (diff.added.length > 0) {
      html += `<div class="diff-section diff-added">
        <h3>Added <span class="diff-count">${diff.added.length}</span></h3>
        ${diff.added.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.type} &middot; ${escapeHtml(p.manufacturer)} &middot; v${p.version}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.removed.length > 0) {
      html += `<div class="diff-section diff-removed">
        <h3>Removed <span class="diff-count">${diff.removed.length}</span></h3>
        ${diff.removed.map(p => `
          <div class="diff-plugin">
            <div class="diff-plugin-name">${escapeHtml(p.name)}</div>
            <div class="diff-plugin-detail">${p.type} &middot; ${escapeHtml(p.manufacturer)} &middot; v${p.version}</div>
          </div>`).join('')}
      </div>`;
    }
    if (diff.versionChanged.length > 0) {
      html += `<div class="diff-section diff-changed">
        <h3>Version Changed <span class="diff-count">${diff.versionChanged.length}</span></h3>
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
  document.getElementById('historyDetail').innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>Select a scan from the sidebar to view details</p></div>';
  await loadHistory();
}

async function clearAllHistory() {
  if (!await confirmAction('Clear all scan history? This cannot be undone.', 'Clear History')) return;
  await Promise.all([
    window.vstUpdater.clearHistory(),
    window.vstUpdater.clearAudioHistory(),
    window.vstUpdater.clearDawHistory(),
    window.vstUpdater.clearPresetHistory(),
  ]);
  selectedScanId = null;
  selectedScanType = null;
  document.getElementById('historyDetail').innerHTML = '<div class="empty-history"><div class="empty-history-icon">&#8592;</div><p>Select a scan from the sidebar to view details</p></div>';
  await loadHistory();
  showToast(toastFmt('toast.all_scan_history_cleared'));
}

function timeAgo(date) {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  return `${months}mo ago`;
}
