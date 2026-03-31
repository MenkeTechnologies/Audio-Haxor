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
    showToast(`Failed to load history — ${e.message || e}`, 4000, 'error');
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
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runDiff" data-id="${id}">Compare</button>
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
      <button class="btn-danger" data-action="deleteScanEntry" data-id="${id}">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;">
      ${detail.plugins.map(p => {
        const typeClass = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : 'type-au';
        return `
          <div class="plugin-card">
            <div class="plugin-info">
              <h3>${escapeHtml(p.name)}</h3>
              <div class="plugin-meta">
                <span class="plugin-type ${typeClass}">${p.type}</span>
                <span>${escapeHtml(p.manufacturer)}</span>
                <span>v${p.version}</span>
                <span>${p.size}</span>
              </div>
            </div>
            <div></div><div></div>
            <div class="plugin-actions">
              <button class="btn-small btn-folder" data-action="openFolder" data-path="${escapePath(p.path)}" title="${escapePath(p.path)}">&#128193;</button>
            </div>
          </div>`;
      }).join('')}
    </div>`;
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
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runAudioDiff" data-id="${id}">Compare</button>
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
      <button class="btn-danger" data-action="deleteAudioScanEntry" data-id="${id}">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;">
      ${detail.samples.map(s => {
        const fmtClass = getFormatClass(s.format);
        return `
          <div class="plugin-card">
            <div class="plugin-info">
              <h3>${escapeHtml(s.name)}</h3>
              <div class="plugin-meta">
                <span class="format-badge ${fmtClass}">${s.format}</span>
                <span>${s.sizeFormatted || formatAudioSize(s.size)}</span>
                <span>${escapeHtml(s.directory || '')}</span>
              </div>
            </div>
            <div></div><div></div>
            <div class="plugin-actions">
              <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${escapePath(s.path)}" title="${escapePath(s.path)}">&#128193;</button>
            </div>
          </div>`;
      }).join('')}
    </div>`;
}

async function runAudioDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffAudioScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = '<div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">No differences found between these scans.</div>';
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
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runDawDiff" data-id="${id}">Compare</button>
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
      <button class="btn-danger" data-action="deleteDawScanEntry" data-id="${id}">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;">
      ${detail.projects.map(p => {
        return `
          <div class="plugin-card">
            <div class="plugin-info">
              <h3>${escapeHtml(p.name)}</h3>
              <div class="plugin-meta">
                <span class="format-badge format-default">${escapeHtml(p.daw)}</span>
                <span>${p.format}</span>
                <span>${p.sizeFormatted || formatAudioSize(p.size)}</span>
                <span>${escapeHtml(p.directory || '')}</span>
              </div>
            </div>
            <div></div><div></div>
            <div class="plugin-actions">
              <button class="btn-small btn-folder" data-action="openDawFolder" data-path="${escapePath(p.path)}" title="${escapePath(p.path)}">&#128193;</button>
            </div>
          </div>`;
      }).join('')}
    </div>`;
}

async function runDawDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffDawScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = '<div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">No differences found between these scans.</div>';
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
        <button class="btn btn-secondary" style="padding: 6px 14px; font-size: 12px;" data-action="runPresetDiff" data-id="${id}">Compare</button>
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
      <button class="btn-danger" data-action="deletePresetScanEntry" data-id="${id}">Delete</button>
    </div>
    ${compareHtml}
    <div id="diffResults"></div>
    <div style="margin-top: 8px;">
      ${detail.presets.map(p => {
        return `
          <div class="plugin-card">
            <div class="plugin-info">
              <h3>${escapeHtml(p.name)}</h3>
              <div class="plugin-meta">
                <span class="format-badge format-default">${p.format}</span>
                <span>${p.sizeFormatted || formatAudioSize(p.size)}</span>
                <span>${escapeHtml(p.directory || '')}</span>
              </div>
            </div>
            <div></div><div></div>
            <div class="plugin-actions">
              <button class="btn-small btn-folder" data-action="openPresetFolder" data-path="${escapePath(p.path)}" title="${escapePath(p.path)}">&#128193;</button>
            </div>
          </div>`;
      }).join('')}
    </div>`;
}

async function runPresetDiff(currentId) {
  const compareId = document.getElementById('compareSelect').value;
  if (!compareId) return;

  const diff = await window.vstUpdater.diffPresetScans(compareId, currentId);
  if (!diff) return;

  const container = document.getElementById('diffResults');
  let html = '';

  if (diff.added.length === 0 && diff.removed.length === 0) {
    html = '<div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">No differences found between these scans.</div>';
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
    html = '<div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">No differences found between these scans.</div>';
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
