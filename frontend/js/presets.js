// ── Presets ──
let allPresets = [];
let filteredPresets = [];
let presetSortKey = 'name';
let presetSortAsc = true;
let presetScanProgressCleanup = null;
let PRESET_PAGE_SIZE = 500;
let presetRenderCount = 0;

function formatPresetSize(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function buildPresetRow(p) {
  const ep = escapePath(p.path);
  return `<tr data-preset-path="${ep}" style="cursor: pointer;" title="Double-click to reveal in Finder">
    <td>${escapeHtml(p.name)}</td>
    <td class="col-format"><span class="format-badge format-default">${p.format}</span></td>
    <td title="${escapePath(p.path)}">${escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted || formatPresetSize(p.size)}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-actions">
      <button class="btn-small btn-folder" data-action="openPresetFolder" data-path="${escapePath(p.path)}" title="${escapePath(p.path)}">&#128193;</button>
    </td>
  </tr>`;
}

function rebuildPresetStats() {
  const statsEl = document.getElementById('presetStats');
  if (!statsEl) return;
  statsEl.style.display = allPresets.length > 0 ? 'flex' : 'none';

  document.getElementById('presetCount').textContent = allPresets.length;
  const headerCount = document.getElementById('presetCountHeader');
  if (headerCount) headerCount.textContent = allPresets.length;

  const totalBytes = allPresets.reduce((sum, p) => sum + p.size, 0);
  document.getElementById('presetTotalSize').textContent = formatPresetSize(totalBytes);

  const formats = {};
  allPresets.forEach(p => { formats[p.format] = (formats[p.format] || 0) + 1; });
  const fmtHtml = Object.entries(formats)
    .sort((a, b) => b[1] - a[1])
    .map(([fmt, count]) => `<span class="format-badge format-default">${fmt}: ${count}</span>`)
    .join(' ');
  document.getElementById('presetFormatBreakdown').innerHTML = fmtHtml;
}

function resetPresetStats() {
  document.getElementById('presetCount').textContent = '0';
  document.getElementById('presetTotalSize').textContent = '0 B';
  document.getElementById('presetFormatBreakdown').innerHTML = '';
}

function sortPreset(key) {
  if (presetSortKey === key) {
    presetSortAsc = !presetSortAsc;
  } else {
    presetSortKey = key;
    presetSortAsc = true;
  }
  ['Name', 'Format', 'Size', 'Modified', 'Directory'].forEach(k => {
    const el = document.getElementById('presetSortArrow' + k);
    if (el) {
      const isActive = k.toLowerCase() === presetSortKey;
      el.innerHTML = isActive ? (presetSortAsc ? '&#9650;' : '&#9660;') : '';
    }
  });
  filterPresets();
}

function filterPresets() {
  const search = (document.getElementById('presetSearchInput')?.value || '').toLowerCase();
  const formatFilter = document.getElementById('presetFormatFilter')?.value || 'all';

  filteredPresets = allPresets.filter(p => {
    if (formatFilter !== 'all' && p.format !== formatFilter) return false;
    if (search && !p.name.toLowerCase().includes(search) &&
        !p.path.toLowerCase().includes(search) &&
        !p.format.toLowerCase().includes(search)) return false;
    return true;
  });

  const key = presetSortKey;
  const dir = presetSortAsc ? 1 : -1;
  filteredPresets.sort((a, b) => {
    if (key === 'size') return (a.size - b.size) * dir;
    const av = (a[key] || '').toLowerCase();
    const bv = (b[key] || '').toLowerCase();
    return av < bv ? -dir : av > bv ? dir : 0;
  });

  presetRenderCount = 0;
  const tbody = document.getElementById('presetTableBody');
  if (!tbody) return;

  const page = filteredPresets.slice(0, PRESET_PAGE_SIZE);
  tbody.innerHTML = page.map(buildPresetRow).join('');
  presetRenderCount = page.length;

  if (filteredPresets.length > PRESET_PAGE_SIZE) {
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="6" style="text-align:center; padding: 12px;">
        <button class="btn btn-secondary" data-action="loadMorePresets">Load more (${filteredPresets.length - PRESET_PAGE_SIZE} remaining)</button>
      </td></tr>`);
  }

  document.getElementById('presetFilteredCount').textContent =
    filteredPresets.length < allPresets.length ? `${filteredPresets.length} / ` : '';
}

function loadMorePresets() {
  const tbody = document.getElementById('presetTableBody');
  const loadMoreRow = tbody.querySelector('[data-action="loadMorePresets"]')?.closest('tr');
  if (loadMoreRow) loadMoreRow.remove();

  const next = filteredPresets.slice(presetRenderCount, presetRenderCount + PRESET_PAGE_SIZE);
  tbody.insertAdjacentHTML('beforeend', next.map(buildPresetRow).join(''));
  presetRenderCount += next.length;

  if (presetRenderCount < filteredPresets.length) {
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="6" style="text-align:center; padding: 12px;">
        <button class="btn btn-secondary" data-action="loadMorePresets">Load more (${filteredPresets.length - presetRenderCount} remaining)</button>
      </td></tr>`);
  }
}

function openPresetFolder(path) {
  window.vstUpdater.openPresetFolder(path);
}

async function scanPresets(resume = false) {
  const btn = document.getElementById('btnScanPresets');
  const resumeBtn = document.getElementById('btnResumePresets');
  const stopBtn = document.getElementById('btnStopPresets');
  const progressBar = document.getElementById('presetProgressBar');
  const progressFill = document.getElementById('presetProgressFill');
  const tableWrap = document.getElementById('presetTableWrap');

  const excludePaths = resume ? allPresets.map(p => p.path) : null;

  btn.disabled = true;
  btn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...';
  resumeBtn.style.display = 'none';
  stopBtn.style.display = '';
  progressBar.classList.add('active');
  progressFill.style.width = '0%';

  if (!resume) {
    allPresets = [];
    filteredPresets = [];
    resetPresetStats();
    document.getElementById('presetStats').style.display = 'none';
    tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for presets...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let firstBatch = true;
  let pendingPresets = [];
  let pendingFound = 0;
  let flushScheduled = false;
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '300', 10);
  let lastFlush = 0;

  function flushPending() {
    flushScheduled = false;
    if (pendingPresets.length === 0) return;
    const batch = pendingPresets.splice(0);

    if (firstBatch) {
      firstBatch = false;
      tableWrap.innerHTML = `<table class="audio-table" id="presetTable">
        <thead><tr>
          <th data-action="sortPreset" data-key="name" style="width: 25%;">Name <span class="sort-arrow" id="presetSortArrowName">&#9660;</span></th>
          <th data-action="sortPreset" data-key="format" class="col-format" style="width: 100px;">Format <span class="sort-arrow" id="presetSortArrowFormat"></span></th>
          <th data-action="sortPreset" data-key="directory" style="width: 35%;">Path <span class="sort-arrow" id="presetSortArrowDirectory"></span></th>
          <th data-action="sortPreset" data-key="size" class="col-size" style="width: 90px;">Size <span class="sort-arrow" id="presetSortArrowSize"></span></th>
          <th data-action="sortPreset" data-key="modified" class="col-date" style="width: 100px;">Modified <span class="sort-arrow" id="presetSortArrowModified"></span></th>
          <th class="col-actions" style="width: 50px;"></th>
        </tr></thead>
        <tbody id="presetTableBody"></tbody>
      </table>`;
      document.getElementById('presetStats').style.display = 'flex';
    }

    allPresets.push(...batch);
    filteredPresets.push(...batch);
    const tbody = document.getElementById('presetTableBody');
    if (tbody && presetRenderCount < 2000) {
      const loadMore = document.getElementById('presetLoadMore');
      if (loadMore) loadMore.remove();
      const toRender = batch.slice(0, 2000 - presetRenderCount);
      tbody.insertAdjacentHTML('beforeend', toRender.map(buildPresetRow).join(''));
      presetRenderCount += toRender.length;
    }

    rebuildPresetStats();
    btn.innerHTML = `&#8635; ${pendingFound} found`;
    lastFlush = performance.now();
  }

  function scheduleFlush() {
    if (flushScheduled) return;
    flushScheduled = true;
    const elapsed = performance.now() - lastFlush;
    const delay = Math.max(0, FLUSH_INTERVAL - elapsed);
    setTimeout(() => requestAnimationFrame(flushPending), delay);
  }

  if (presetScanProgressCleanup) presetScanProgressCleanup();
  presetScanProgressCleanup = window.vstUpdater.onPresetScanProgress((data) => {
    if (data.phase === 'status') {
      // status message
    } else if (data.phase === 'scanning') {
      pendingPresets.push(...data.presets);
      pendingFound = data.found;
      scheduleFlush();
    }
  });

  try {
    const presetRoots = (prefs.getItem('presetScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanPresets(presetRoots.length ? presetRoots : undefined, excludePaths);
    if (presetScanProgressCleanup) { presetScanProgressCleanup(); presetScanProgressCleanup = null; }
    flushPending();
    if (resume) {
      allPresets = [...allPresets, ...result.presets];
    } else {
      allPresets = result.presets;
    }
    rebuildPresetStats();
    filterPresets();
    try { await window.vstUpdater.savePresetScan(allPresets, result.roots); } catch (e) { console.error('Failed to save preset scan history:', e); }
    if (result.stopped && allPresets.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (presetScanProgressCleanup) { presetScanProgressCleanup(); presetScanProgressCleanup = null; }
    flushPending();
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${err.message}</p></div>`;
  }

  btn.disabled = false;
  btn.innerHTML = '&#127924; Scan Presets';
  stopBtn.style.display = 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
}

async function stopPresetScan() {
  await window.vstUpdater.stopPresetScan();
}
