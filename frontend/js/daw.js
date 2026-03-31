// ── DAW Projects ──
let allDawProjects = [];
let filteredDawProjects = [];
let dawSortKey = 'name';
let dawSortAsc = true;
let dawScanProgressCleanup = null;

let dawStatCounts = {};
let dawStatBytes = 0;

function resetDawStats() {
  dawStatCounts = {};
  dawStatBytes = 0;
}

function accumulateDawStats(projects) {
  for (const p of projects) {
    dawStatCounts[p.daw] = (dawStatCounts[p.daw] || 0) + 1;
    dawStatBytes += p.size;
  }
}

function updateDawStats() {
  const stats = document.getElementById('dawStats');
  stats.style.display = 'flex';
  document.getElementById('dawTotalCount').textContent = allDawProjects.length;
  document.getElementById('dawAbletonCount').textContent = dawStatCounts['Ableton Live'] || 0;
  document.getElementById('dawLogicCount').textContent = dawStatCounts['Logic Pro'] || 0;
  document.getElementById('dawFlCount').textContent = dawStatCounts['FL Studio'] || 0;
  document.getElementById('dawReaperCount').textContent = dawStatCounts['REAPER'] || 0;
  const mainDaws = (dawStatCounts['Ableton Live'] || 0) + (dawStatCounts['Logic Pro'] || 0) + (dawStatCounts['FL Studio'] || 0) + (dawStatCounts['REAPER'] || 0);
  document.getElementById('dawOtherCount').textContent = allDawProjects.length - mainDaws;
  document.getElementById('dawTotalSize').textContent = formatAudioSize(dawStatBytes);
  document.getElementById('dawProjectCount').textContent = allDawProjects.length;
  document.getElementById('btnExportDaw').style.display = allDawProjects.length > 0 ? '' : 'none';
}

function rebuildDawStats() {
  resetDawStats();
  accumulateDawStats(allDawProjects);
  updateDawStats();
}

function initDawTable() {
  const tableWrap = document.getElementById('dawTableWrap');
  tableWrap.innerHTML = `<table class="audio-table" id="dawTable">
    <thead>
      <tr>
        <th data-action="sortDaw" data-key="name" style="width: 25%;">Name <span class="sort-arrow" id="dawSortArrowName">&#9660;</span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="daw" class="col-format" style="width: 12%;">DAW <span class="sort-arrow" id="dawSortArrowDaw"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="format" class="col-format" style="width: 80px;">Format <span class="sort-arrow" id="dawSortArrowFormat"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="size" class="col-size" style="width: 90px;">Size <span class="sort-arrow" id="dawSortArrowSize"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="modified" class="col-date" style="width: 100px;">Modified <span class="sort-arrow" id="dawSortArrowModified"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="directory" style="width: 28%;">Path <span class="sort-arrow" id="dawSortArrowDirectory"></span><span class="col-resize"></span></th>
        <th class="col-actions" style="width: 60px;"></th>
      </tr>
    </thead>
    <tbody id="dawTableBody"></tbody>
  </table>`;
  initColumnResize(document.getElementById('dawTable'));
}

function getDawBadgeClass(daw) {
  const d = daw.toLowerCase().replace(/\s+/g, '-');
  return 'daw-' + d;
}

function buildDawRow(p) {
  const ep = escapePath(p.path);
  const dawClass = getDawBadgeClass(p.daw);
  return `<tr data-daw-path="${ep}" title="Double-click to open in ${escapeHtml(p.daw)}" style="cursor: pointer;">
    <td class="col-name" title="${escapeHtml(p.name)}">${highlightMatch(p.name, _lastDawSearch, _lastDawMode)}</td>
    <td class="col-format"><span class="format-badge ${dawClass}">${escapeHtml(p.daw)}</span></td>
    <td class="col-format"><span class="format-badge format-default">${p.format}</span></td>
    <td class="col-size">${p.sizeFormatted}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-path" title="${escapeHtml(p.path)}">${escapeHtml(p.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-folder" data-action="openDawFolder" data-path="${ep}" title="Reveal in Finder">&#128193;</button>
    </td>
  </tr>`;
}

let _lastDawSearch = '';
let _lastDawMode = 'fuzzy';

function filterDawProjects() {
  const search = document.getElementById('dawSearchInput').value || '';
  const daw = document.getElementById('dawDawFilter').value;
  const mode = getSearchMode('regexDaw');
  _lastDawSearch = search;
  _lastDawMode = mode;

  filteredDawProjects = allDawProjects.filter(p => {
    if (daw !== 'all' && p.daw !== daw) return false;
    if (search && !searchMatch(search, [p.name, p.path, p.daw, p.format], mode)) return false;
    return true;
  });

  sortDawArray();
  renderDawTable();
}

function sortDaw(key) {
  if (dawSortKey === key) {
    dawSortAsc = !dawSortAsc;
  } else {
    dawSortKey = key;
    dawSortAsc = true;
  }
  ['Name', 'Daw', 'Format', 'Size', 'Modified', 'Directory'].forEach(k => {
    const el = document.getElementById('dawSortArrow' + k);
    if (el) {
      const isActive = k.toLowerCase() === dawSortKey;
      el.innerHTML = isActive ? (dawSortAsc ? '&#9650;' : '&#9660;') : '';
      el.closest('th').classList.toggle('sort-active', isActive);
    }
  });
  sortDawArray();
  renderDawTable();
}

function sortDawArray() {
  const key = dawSortKey;
  const dir = dawSortAsc ? 1 : -1;
  filteredDawProjects.sort((a, b) => {
    let va = a[key], vb = b[key];
    if (key === 'size') return (va - vb) * dir;
    if (typeof va === 'string') return va.localeCompare(vb) * dir;
    return 0;
  });
}

let dawRenderCount = 0;

function renderDawTable() {
  if (!document.getElementById('dawTable')) initDawTable();
  const tbody = document.getElementById('dawTableBody');
  if (!tbody) return;
  dawRenderCount = Math.min(DAW_PAGE_SIZE, filteredDawProjects.length);
  tbody.innerHTML = filteredDawProjects.slice(0, dawRenderCount).map(buildDawRow).join('');

  if (dawRenderCount < filteredDawProjects.length) {
    appendDawLoadMore(tbody);
  }
}

function appendDawLoadMore(tbody) {
  tbody.insertAdjacentHTML('beforeend',
    `<tr id="dawLoadMore"><td colspan="7" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreDaw">
      Showing ${dawRenderCount} of ${filteredDawProjects.length} &#8212; click to load more
    </td></tr>`);
}

function loadMoreDaw() {
  const tbody = document.getElementById('dawTableBody');
  const loadMore = document.getElementById('dawLoadMore');
  if (loadMore) loadMore.remove();
  const nextBatch = filteredDawProjects.slice(dawRenderCount, dawRenderCount + DAW_PAGE_SIZE);
  dawRenderCount += nextBatch.length;
  tbody.insertAdjacentHTML('beforeend', nextBatch.map(buildDawRow).join(''));
  if (dawRenderCount < filteredDawProjects.length) {
    appendDawLoadMore(tbody);
  }
}

async function scanDawProjects(resume = false) {
  const btn = document.getElementById('btnScanDaw');
  const resumeBtn = document.getElementById('btnResumeDaw');
  const stopBtn = document.getElementById('btnStopDaw');
  const progressBar = document.getElementById('dawProgressBar');
  const progressFill = document.getElementById('dawProgressFill');
  const tableWrap = document.getElementById('dawTableWrap');

  const excludePaths = resume ? allDawProjects.map(p => p.path) : null;

  btn.disabled = true;
  btn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...';
  resumeBtn.style.display = 'none';
  stopBtn.style.display = '';
  progressBar.classList.add('active');
  progressFill.style.width = '0%';

  if (!resume) {
    allDawProjects = [];
    filteredDawProjects = [];
    resetDawStats();
  }
  if (!resume) {
    document.getElementById('dawStats').style.display = 'none';
    tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for DAW projects...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let firstDawBatch = true;
  let pendingProjects = [];
  let pendingFound = 0;
  let flushScheduled = false;
  const FLUSH_INTERVAL = 300;
  let lastFlush = 0;

  function flushPendingProjects() {
    flushScheduled = false;
    if (pendingProjects.length === 0) return;

    if (firstDawBatch) {
      firstDawBatch = false;
      tableWrap.innerHTML = '';
      initDawTable();
    }

    const toAdd = pendingProjects;
    pendingProjects = [];

    allDawProjects.push(...toAdd);
    accumulateDawStats(toAdd);
    btn.innerHTML = `&#8635; ${pendingFound} found`;
    progressFill.style.width = '';
    progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

    const search = document.getElementById('dawSearchInput').value || '';
    const daw = document.getElementById('dawDawFilter').value;
    const scanMode = getSearchMode('regexDaw');
    const matching = toAdd.filter(p => {
      if (daw !== 'all' && p.daw !== daw) return false;
      if (search && !searchMatch(search, [p.name, p.path, p.daw], scanMode)) return false;
      return true;
    });
    if (matching.length > 0) {
      filteredDawProjects.push(...matching);
      const tbody = document.getElementById('dawTableBody');
      if (tbody && dawRenderCount < 2000) {
        const loadMore = document.getElementById('dawLoadMore');
        if (loadMore) loadMore.remove();
        const toRender = matching.slice(0, 2000 - dawRenderCount);
        tbody.insertAdjacentHTML('beforeend', toRender.map(buildDawRow).join(''));
        dawRenderCount += toRender.length;
      }
    }

    updateDawStats();
    lastFlush = performance.now();
  }

  function scheduleFlush() {
    if (flushScheduled) return;
    flushScheduled = true;
    const elapsed = performance.now() - lastFlush;
    const delay = Math.max(0, FLUSH_INTERVAL - elapsed);
    setTimeout(() => requestAnimationFrame(flushPendingProjects), delay);
  }

  if (dawScanProgressCleanup) dawScanProgressCleanup();
  dawScanProgressCleanup = window.vstUpdater.onDawScanProgress((data) => {
    if (data.phase === 'status') {
      // status message
    } else if (data.phase === 'scanning') {
      pendingProjects.push(...data.projects);
      pendingFound = data.found;
      scheduleFlush();
    }
  });

  try {
    const dawRoots = (prefs.getItem('dawScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanDawProjects(dawRoots.length ? dawRoots : undefined, excludePaths);
    if (dawScanProgressCleanup) { dawScanProgressCleanup(); dawScanProgressCleanup = null; }
    flushPendingProjects();
    if (resume) {
      allDawProjects = [...allDawProjects, ...result.projects];
    } else {
      allDawProjects = result.projects;
    }
    rebuildDawStats();
    filterDawProjects();
    try { await window.vstUpdater.saveDawScan(allDawProjects, result.roots); } catch (e) { showToast(`Failed to save DAW history — ${e.message || e}`, 4000, 'error'); }
    if (result.stopped && allDawProjects.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (dawScanProgressCleanup) { dawScanProgressCleanup(); dawScanProgressCleanup = null; }
    flushPendingProjects();
    const errMsg = err.message || err || 'Unknown error';
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    showToast(`DAW scan failed — ${errMsg}`, 4000, 'error');
  }

  btn.disabled = false;
  btn.innerHTML = '&#127911; Scan DAW Projects';
  stopBtn.style.display = 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
  progressFill.style.animation = '';
}

async function stopDawScan() {
  await window.vstUpdater.stopDawScan();
}

function openDawFolder(filePath) {
  window.vstUpdater.openDawFolder(filePath);
}
