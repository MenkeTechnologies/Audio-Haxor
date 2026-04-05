// ── DAW Projects ──
function _dawFmt(key, vars) {
  if (typeof appFmt !== 'function') return key;
  return vars ? appFmt(key, vars) : appFmt(key);
}

let allDawProjects = [];
let filteredDawProjects = [];
let dawSortKey = 'name';
let dawSortAsc = true;
let dawScanProgressCleanup = null;
let _dawOffset = 0;
let _dawTotalCount = 0;
let _dawTotalUnfiltered = 0;

let dawStatCounts = {};
let dawStatBytes = 0;
// Snapshot of unfiltered per-DAW counts + total bytes from the latest scan.
// Set once on mount / post-scan via dbDawStats so filter changes DON'T wipe it.
let _dawStatsSnapshot = null;

async function fetchDawPage() {
  const search = document.getElementById('dawSearchInput')?.value || '';
  const dawSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('dawDawFilter') : null;
  const dawFilter = dawSet ? [...dawSet].join(',') : null;
  // During an active scan, DOM-toggle filter existing rendered rows (O(visible))
  // instead of re-scanning the in-memory array. Scan streaming already filters
  // incoming batches, so going forward stays consistent.
  if (dawScanProgressCleanup) {
    const tbody = document.getElementById('dawTableBody');
    if (tbody) {
      const needle = search ? search.trim().toLowerCase() : '';
      const rows = tbody.rows;
      let visible = 0;
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i];
        const daw = r.dataset.dawName;
        if (!daw) continue;
        let match = true;
        if (dawSet && !dawSet.has(daw)) match = false;
        if (match && needle && !r.dataset.dawSearch.includes(needle)) match = false;
        r.style.display = match ? '' : 'none';
        if (match) visible++;
      }
      _dawTotalCount = visible;
    }
    return;
  }
  try {
    const result = await window.vstUpdater.dbQueryDaw({
      search: search || null,
      daw_filter: dawFilter,
      sort_key: dawSortKey,
      sort_asc: dawSortAsc,
      offset: _dawOffset,
      limit: typeof DAW_PAGE_SIZE !== 'undefined' ? DAW_PAGE_SIZE : 200,
    });
    let projects = result.projects || [];
    // Re-sort by fzf relevance score
    if (search && projects.length > 1) {
      const scored = projects.map(p => ({ p, score: searchScore(search, [p.name], _lastDawMode) }));
      scored.sort((a, b) => b.score - a.score);
      projects = scored.map(x => x.p);
    }
    if (_dawOffset === 0) {
      filteredDawProjects = projects;
      allDawProjects = filteredDawProjects;
    } else {
      filteredDawProjects.push(...projects);
      allDawProjects.push(...projects);
    }
    _dawTotalCount = result.totalCount || 0;
    _dawTotalUnfiltered = result.totalUnfiltered || 0;
    renderDawTable();
    // NOTE: do NOT rebuild the stats breakdown here — it would shrink to match
    // the filtered page. updateDawStats() reads the unfiltered _dawStatsSnapshot.
    updateDawStats();
  } catch (e) {
    showToast(toastFmt('toast.daw_query_failed', { err: e }), 4000, 'error');
  }
}

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
  // Prefer the unfiltered snapshot so filter/search changes NEVER shrink the breakdown.
  // Fall back to incremental (scan-in-progress) or paged values only if snapshot absent.
  const src = _dawStatsSnapshot ? _dawStatsSnapshot.counts : dawStatCounts;
  const bytes = _dawStatsSnapshot ? _dawStatsSnapshot.totalBytes : dawStatBytes;
  const ableton = src['Ableton Live'] || 0;
  const logic = src['Logic Pro'] || 0;
  const fl = src['FL Studio'] || 0;
  const reaper = src['REAPER'] || 0;
  const mainDaws = ableton + logic + fl + reaper;
  let accumulatedTotal = 0;
  for (const k in src) accumulatedTotal += src[k];
  const dawDisplayCount = Math.max(_dawTotalUnfiltered || 0, accumulatedTotal, _dawTotalCount || 0, allDawProjects.length);
  document.getElementById('dawTotalCount').textContent = dawDisplayCount;
  document.getElementById('dawAbletonCount').textContent = ableton;
  document.getElementById('dawLogicCount').textContent = logic;
  document.getElementById('dawFlCount').textContent = fl;
  document.getElementById('dawReaperCount').textContent = reaper;
  document.getElementById('dawOtherCount').textContent = Math.max(0, dawDisplayCount - mainDaws);
  document.getElementById('dawTotalSize').textContent = formatAudioSize(bytes);
  document.getElementById('dawProjectCount').textContent = dawDisplayCount;
  document.getElementById('btnExportDaw').style.display = dawDisplayCount > 0 ? '' : 'none';
  if (typeof updateDawDiskUsage === 'function') updateDawDiskUsage();
}

// Load unfiltered stats snapshot from DB (for post-scan / app-mount paths).
// This drives the stats breakdown and is immune to table-filter changes.
async function refreshDawStatsSnapshot() {
  try {
    const s = await window.vstUpdater.dbDawStats();
    _dawStatsSnapshot = {
      counts: s.dawCounts || {},
      totalBytes: s.totalBytes || 0,
      projectCount: s.projectCount || 0,
    };
    if (_dawStatsSnapshot.projectCount > 0) {
      _dawTotalUnfiltered = _dawStatsSnapshot.projectCount;
    }
    updateDawStats();
  } catch { /* fall through — updateDawStats() still works with incremental state */ }
}

function rebuildDawStats() {
  resetDawStats();
  accumulateDawStats(allDawProjects);
  updateDawStats();
}

function initDawTable() {
  const tableWrap = document.getElementById('dawTableWrap');
  const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
  const sel = typeof escapeHtml === 'function' ? escapeHtml(tc('ui.audio.th_select_all')) : tc('ui.audio.th_select_all');
  tableWrap.innerHTML = `<table class="audio-table" id="dawTable">
    <thead>
      <tr>
        <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${sel}"></th>
        <th data-action="sortDaw" data-key="name" style="width: 23%;">${tc('ui.export.col_name')} <span class="sort-arrow" id="dawSortArrowName">&#9660;</span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="daw" class="col-format" style="width: 12%;">${tc('ui.export.col_daw')} <span class="sort-arrow" id="dawSortArrowDaw"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="format" class="col-format" style="width: 80px;">${tc('ui.export.col_format')} <span class="sort-arrow" id="dawSortArrowFormat"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="size" class="col-size" style="width: 90px;">${tc('ui.export.col_size')} <span class="sort-arrow" id="dawSortArrowSize"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="modified" class="col-date" style="width: 100px;">${tc('ui.export.col_modified')} <span class="sort-arrow" id="dawSortArrowModified"></span><span class="col-resize"></span></th>
        <th data-action="sortDaw" data-key="directory" style="width: 28%;">${tc('ui.export.col_path')} <span class="sort-arrow" id="dawSortArrowDirectory"></span><span class="col-resize"></span></th>
        <th class="col-actions" style="width: 60px;"></th>
      </tr>
    </thead>
    <tbody id="dawTableBody"></tbody>
  </table>`;
  initColumnResize(document.getElementById('dawTable'));
  if (typeof initTableColumnReorder === 'function') initTableColumnReorder('dawTable', 'dawColumnOrder');
}

function getDawBadgeClass(daw) {
  const d = daw.toLowerCase().replace(/\s+/g, '-');
  return 'daw-' + d;
}

function buildDawRow(p) {
  const hp = escapeHtml(p.path);
  const dawClass = getDawBadgeClass(p.daw);
  const checked = batchSelected.has(p.path) ? ' checked' : '';
  const xrefSupported = typeof isXrefSupported === 'function' && isXrefSupported(p.format);
  const cached = typeof _xrefCache !== 'undefined' && _xrefCache[p.path];
  const xrefTitle = typeof escapeHtml === 'function'
    ? escapeHtml(_dawFmt('ui.tt.daw_xref_show_plugins'))
    : _dawFmt('ui.tt.daw_xref_show_plugins');
  const xrefBtn = xrefSupported
    ? `<button class="xref-badge${cached && cached.length > 0 ? ' has-plugins' : ''}" data-action="showXref" data-path="${hp}" data-name="${escapeHtml(p.name)}" title="${xrefTitle}">&#9889;${cached ? ' ' + cached.length : ''}</button>`
    : '';
  const rowTt = typeof escapeHtml === 'function'
    ? escapeHtml(_dawFmt('ui.tt.daw_open_in_project', { daw: p.daw }))
    : _dawFmt('ui.tt.daw_open_in_project', { daw: p.daw });
  const revealT = typeof escapeHtml === 'function'
    ? escapeHtml(_dawFmt('menu.reveal_in_finder'))
    : _dawFmt('menu.reveal_in_finder');
  return `<tr data-daw-path="${hp}" data-daw-name="${escapeHtml(p.daw)}" data-daw-search="${escapeHtml((p.name || '').toLowerCase())}" title="${rowTt}" style="cursor: pointer;">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(p.name)}">${_lastDawSearch ? highlightMatch(p.name, _lastDawSearch, _lastDawMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td class="col-format"><span class="format-badge ${dawClass}">${escapeHtml(p.daw)}</span></td>
    <td class="col-format"><span class="format-badge format-default">${p.format}</span>${xrefBtn}</td>
    <td class="col-size">${p.sizeFormatted}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-path" title="${escapeHtml(p.path)}">${escapeHtml(p.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-folder" data-action="openDawFolder" data-path="${hp}" title="${revealT}">&#128193;</button>
    </td>
  </tr>`;
}

let _lastDawSearch = '';
let _lastDawMode = 'fuzzy';

registerFilter('filterDawProjects', {
  inputId: 'dawSearchInput',
  regexToggleId: 'regexDaw',
  formatDropdownId: 'dawDawFilter',
  resetOffset() { _dawOffset = 0; },
  fetchFn() {
    _lastDawSearch = this.lastSearch || '';
    _lastDawMode = this.lastMode || 'fuzzy';
    fetchDawPage();
  },
});
function filterDawProjects() { applyFilter('filterDawProjects'); }

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
  _dawOffset = 0;
  fetchDawPage();
  if (typeof saveSortState === 'function') saveSortState('daw', dawSortKey, dawSortAsc);
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
  const wrap = document.getElementById('dawTableWrap');
  // No data at all — restore the initial state-message (unfiltered empty scan).
  if (filteredDawProjects.length === 0 && _dawTotalCount === 0 && _dawTotalUnfiltered === 0) {
    if (wrap) {
      const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
      const h2 = esc(typeof appFmt === 'function' ? appFmt('ui.h2.daw_project_index') : 'DAW Project Index');
      const p = esc(typeof appFmt === 'function' ? appFmt('ui.p.daw_empty') : 'Click "Scan DAW Projects" to find DAW project files on your system.');
      wrap.innerHTML = `<div class="state-message" id="dawEmptyState">
        <div class="state-icon">&#127911;</div>
        <h2>${h2}</h2>
        <p>${p}</p>
      </div>`;
    }
    return;
  }
  if (!document.getElementById('dawTable')) initDawTable();
  const tbody = document.getElementById('dawTableBody');
  if (!tbody) return;
  dawRenderCount = filteredDawProjects.length;
  tbody.innerHTML = filteredDawProjects.map(buildDawRow).join('');

  // Filter active + no matches: inline row with a clear message instead of a blank table body.
  if (filteredDawProjects.length === 0 && _dawTotalUnfiltered > 0) {
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
    const msg = esc(typeof appFmt === 'function' ? appFmt('ui.daw.no_filter_matches') : 'No DAW projects match the current filter.');
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="7" style="text-align:center;padding:24px;color:var(--text-dim);"><span style="font-size:24px;display:block;margin-bottom:8px;">&#128269;</span>${msg}</td></tr>`);
    return;
  }
  if (dawRenderCount < _dawTotalCount) {
    appendDawLoadMore(tbody);
  }
}

function appendDawLoadMore(tbody) {
  const line = typeof appFmt === 'function'
    ? appFmt('ui.js.load_more_hint', {
        shown: dawRenderCount.toLocaleString(),
        total: _dawTotalCount.toLocaleString(),
      })
    : `Showing ${dawRenderCount} of ${_dawTotalCount} — click to load more`;
  tbody.insertAdjacentHTML('beforeend',
    `<tr id="dawLoadMore"><td colspan="7" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreDaw">
      ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
    </td></tr>`);
}

function loadMoreDaw() {
  _dawOffset = allDawProjects.length;
  fetchDawPage();
}

async function scanDawProjects(resume = false) {
  showGlobalProgress();
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
  const dawEta = createETA();
  dawEta.start();
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);
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
    const dawElapsed = dawEta.elapsed();
    btn.innerHTML = `&#8635; ${pendingFound} found${dawElapsed ? ' — ' + dawElapsed : ''}`;
    progressFill.style.width = '';
    progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

    const search = document.getElementById('dawSearchInput').value || '';
    const scanDawSet = getMultiFilterValues('dawDawFilter');
    const scanMode = getSearchMode('regexDaw');
    const matching = toAdd.filter(p => {
      if (scanDawSet && !scanDawSet.has(p.daw)) return false;
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
      // Immediately update header counter
      document.getElementById('dawProjectCount').textContent = pendingFound;
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
    _dawTotalUnfiltered = allDawProjects.length;
    filterDawProjects();
    // Only save if scan completed fully (not stopped/aborted with partial results)
    if (!result.stopped) {
      try { await window.vstUpdater.saveDawScan(allDawProjects, result.roots); } catch (e) { showToast(toastFmt('toast.failed_save_daw_history', { err: e.message || e }), 4000, 'error'); }
    }
    // Pull authoritative unfiltered breakdown from DB so filter changes don't reset it.
    await refreshDawStatsSnapshot();
    if (result.stopped && allDawProjects.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (dawScanProgressCleanup) { dawScanProgressCleanup(); dawScanProgressCleanup = null; }
    flushPendingProjects();
    const errMsg = err.message || err || 'Unknown error';
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    showToast(toastFmt('toast.daw_scan_failed', { errMsg }), 4000, 'error');
  }

  hideGlobalProgress();
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
  window.vstUpdater.openDawFolder(filePath).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error'));
}
