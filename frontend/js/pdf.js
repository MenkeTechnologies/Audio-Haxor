// ── PDFs ──
let allPdfs = [];
let filteredPdfs = [];
let pdfSortKey = 'name';
let pdfSortAsc = true;
let pdfScanProgressCleanup = null;
let PDF_PAGE_SIZE = 500;
let pdfRenderCount = 0;
let _pdfOffset = 0;
let _pdfTotalCount = 0;
let _pdfTotalUnfiltered = 0;
// Incremental stats for PDFs — avoids O(N) rebuild on every scan flush.
let _pdfStatsTotalBytes = 0;

let _lastPdfSearch = '';
let _lastPdfMode = 'fuzzy';

async function fetchPdfPage() {
  const search = document.getElementById('pdfSearchInput')?.value || '';
  if (typeof showGlobalProgress === 'function') showGlobalProgress();
  // During an active scan, DOM-toggle filter existing rendered rows instead of
  // hitting the DB (scan isn't saved yet, query would wipe live results).
  if (pdfScanProgressCleanup) {
    const tbody = document.getElementById('pdfTableBody');
    if (tbody) {
      const needle = search ? search.trim().toLowerCase() : '';
      const rows = tbody.rows;
      let visible = 0;
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i];
        const name = r.dataset.pdfName;
        if (name === undefined) continue;
        const match = !needle || name.includes(needle);
        r.style.display = match ? '' : 'none';
        if (match) visible++;
      }
      _pdfTotalCount = visible;
    }
    if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
    return;
  }
  try {
    const result = await window.vstUpdater.dbQueryPdfs({
      search: search || null,
      sort_key: pdfSortKey,
      sort_asc: pdfSortAsc,
      offset: _pdfOffset,
      limit: PDF_PAGE_SIZE,
    });
    let pdfs = result.pdfs || [];
    if (search && pdfs.length > 1) {
      const scored = pdfs.map(p => ({ p, score: searchScore(search, [p.name], _lastPdfMode) }));
      scored.sort((a, b) => b.score - a.score);
      pdfs = scored.map(x => x.p);
    }
    if (_pdfOffset === 0) {
      filteredPdfs = pdfs;
      allPdfs = filteredPdfs;
    } else {
      filteredPdfs.push(...pdfs);
      allPdfs.push(...pdfs);
    }
    _pdfTotalCount = result.totalCount || 0;
    _pdfTotalUnfiltered = result.totalUnfiltered || 0;
    renderPdfTable();
  } catch (e) {
    showToast(toastFmt('toast.pdf_query_failed', { err: e && e.message ? e.message : e }), 4000, 'error');
  } finally {
    if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
  }
}

function accumulatePdfStats(batch) {
  for (let i = 0; i < batch.length; i++) {
    _pdfStatsTotalBytes += batch[i].size || 0;
  }
}

function resetPdfStatsAccumulators() {
  _pdfStatsTotalBytes = 0;
}

function rebuildPdfStats() {
  const statsEl = document.getElementById('pdfStats');
  if (!statsEl) return;
  const displayCount = _pdfTotalUnfiltered || _pdfTotalCount || allPdfs.length;
  statsEl.style.display = displayCount > 0 ? 'flex' : 'none';
  document.getElementById('pdfCount').textContent = displayCount.toLocaleString();
  if (_pdfStatsTotalBytes === 0 && allPdfs.length > 0) {
    accumulatePdfStats(allPdfs);
  }
  document.getElementById('pdfTotalSize').textContent = formatAudioSize(_pdfStatsTotalBytes);
}

function buildPdfRow(p) {
  const hp = escapeHtml(p.path);
  const checked = batchSelected.has(p.path) ? ' checked' : '';
  return `<tr data-pdf-path="${hp}" data-pdf-name="${escapeHtml((p.name || '').toLowerCase())}" style="cursor: pointer;" title="Double-click to reveal in Finder">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td>${_lastPdfSearch ? highlightMatch(p.name, _lastPdfSearch, _lastPdfMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td title="${hp}">${escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-actions">
      <button class="btn-small btn-folder" data-action="openPdfFile" data-path="${hp}" title="${hp}">&#128193;</button>
    </td>
  </tr>`;
}

function renderPdfTable() {
  if (!document.getElementById('pdfTable')) {
    const tableWrap = document.getElementById('pdfTableWrap');
    if (tableWrap && filteredPdfs.length > 0) {
      tableWrap.innerHTML = buildPdfTableHtml();
      document.getElementById('pdfStats').style.display = 'flex';
      if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('pdfTable'));
      if (typeof initTableColumnReorder === 'function') initTableColumnReorder('pdfTable', 'pdfColumnOrder');
    }
  }
  const tbody = document.getElementById('pdfTableBody');
  if (!tbody) return;
  tbody.innerHTML = filteredPdfs.map(buildPdfRow).join('');
  pdfRenderCount = filteredPdfs.length;
  if (pdfRenderCount < _pdfTotalCount) {
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="6" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMorePdfs">
        Showing ${pdfRenderCount} of ${_pdfTotalCount} — click to load more
      </td></tr>`);
  }
}

function buildPdfTableHtml() {
  return `<table class="audio-table" id="pdfTable">
    <thead><tr>
      <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="Select all"></th>
      <th data-action="sortPdf" data-key="name" style="width: 30%;">Name <span class="sort-arrow" id="pdfSortArrowName">&#9660;</span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="directory" style="width: 40%;">Path <span class="sort-arrow" id="pdfSortArrowDirectory"></span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="size" class="col-size" style="width: 90px;">Size <span class="sort-arrow" id="pdfSortArrowSize"></span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="modified" class="col-date" style="width: 100px;">Modified <span class="sort-arrow" id="pdfSortArrowModified"></span><span class="col-resize"></span></th>
      <th class="col-actions" style="width: 50px;"></th>
    </tr></thead>
    <tbody id="pdfTableBody"></tbody>
  </table>`;
}

function loadMorePdfs() {
  _pdfOffset = allPdfs.length;
  fetchPdfPage();
}

function sortPdf(key) {
  if (pdfSortKey === key) {
    pdfSortAsc = !pdfSortAsc;
  } else {
    pdfSortKey = key;
    pdfSortAsc = true;
  }
  ['Name', 'Size', 'Modified', 'Directory'].forEach(k => {
    const el = document.getElementById('pdfSortArrow' + k);
    if (el) {
      const isActive = k.toLowerCase() === pdfSortKey;
      el.innerHTML = isActive ? (pdfSortAsc ? '&#9650;' : '&#9660;') : '';
    }
  });
  filterPdfs();
  if (typeof saveSortState === 'function') saveSortState('pdf', pdfSortKey, pdfSortAsc);
}

registerFilter('filterPdfs', {
  inputId: 'pdfSearchInput',
  regexToggleId: 'regexPdf',
  resetOffset() { _pdfOffset = 0; },
  fetchFn() {
    _lastPdfSearch = this.lastSearch || '';
    _lastPdfMode = this.lastMode || 'fuzzy';
    fetchPdfPage();
  },
});
function filterPdfs() { applyFilter('filterPdfs'); }

function openPdfFile(path) {
  window.vstUpdater.openPdfFile(path)
    .then(() => showToast(toastFmt('toast.revealed_in_finder')))
    .catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error'));
}

async function scanPdfs(resume = false) {
  showGlobalProgress();
  const scanBtn = document.querySelector('[data-action="scanPdfs"]');
  const resumeBtn = document.getElementById('btnResumePdf');
  const stopBtn = document.getElementById('btnStopPdf');
  const progressBar = document.getElementById('pdfProgressBar');
  const progressFill = document.getElementById('pdfProgressFill');
  const tableWrap = document.getElementById('pdfTableWrap');

  const excludePaths = resume ? allPdfs.map(p => p.path) : null;

  if (scanBtn) { scanBtn.disabled = true; scanBtn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...'; }
  if (resumeBtn) resumeBtn.style.display = 'none';
  if (stopBtn) stopBtn.style.display = '';
  progressBar.classList.add('active');
  progressFill.style.width = '0%';

  if (!resume) {
    allPdfs = [];
    filteredPdfs = [];
    resetPdfStatsAccumulators();
    _pdfTotalUnfiltered = 0;
    document.getElementById('pdfStats').style.display = 'none';
    tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for PDFs...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let firstBatch = true;
  let pendingPdfs = [];
  let pendingFound = 0;
  let flushScheduled = false;
  const pdfEta = createETA();
  pdfEta.start();
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);
  let lastFlush = 0;

  function flushPending() {
    flushScheduled = false;
    if (pendingPdfs.length === 0) return;
    const batch = pendingPdfs.splice(0);

    if (firstBatch) {
      firstBatch = false;
      tableWrap.innerHTML = buildPdfTableHtml();
      document.getElementById('pdfStats').style.display = 'flex';
      if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('pdfTable'));
      if (typeof initTableColumnReorder === 'function') initTableColumnReorder('pdfTable', 'pdfColumnOrder');
    }

    allPdfs.push(...batch);
    filteredPdfs.push(...batch);
    accumulatePdfStats(batch);

    const tbody = document.getElementById('pdfTableBody');
    if (tbody && pdfRenderCount < 2000) {
      const scanSearch = (document.getElementById('pdfSearchInput')?.value || '').trim().toLowerCase();
      const visibleBatch = scanSearch
        ? batch.filter(p => (p.name || '').toLowerCase().includes(scanSearch))
        : batch;
      const toRender = visibleBatch.slice(0, 2000 - pdfRenderCount);
      tbody.insertAdjacentHTML('beforeend', toRender.map(buildPdfRow).join(''));
      pdfRenderCount += toRender.length;
    }

    _pdfTotalUnfiltered = allPdfs.length;
    rebuildPdfStats();
    const elapsed = pdfEta.elapsed();
    if (scanBtn) scanBtn.innerHTML = `&#8635; ${pendingFound} found${elapsed ? ' — ' + elapsed : ''}`;
    lastFlush = performance.now();
  }

  function scheduleFlush() {
    if (flushScheduled) return;
    flushScheduled = true;
    const elapsed = performance.now() - lastFlush;
    const delay = Math.max(0, FLUSH_INTERVAL - elapsed);
    setTimeout(() => requestAnimationFrame(flushPending), delay);
  }

  if (pdfScanProgressCleanup) pdfScanProgressCleanup();
  pdfScanProgressCleanup = window.vstUpdater.onPdfScanProgress((data) => {
    if (data.phase === 'status') {
      // status message
    } else if (data.phase === 'scanning') {
      pendingPdfs.push(...data.pdfs);
      pendingFound = data.found;
      scheduleFlush();
    }
  });

  try {
    const pdfRoots = (prefs.getItem('pdfScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanPdfs(pdfRoots.length ? pdfRoots : undefined, excludePaths);
    if (pdfScanProgressCleanup) { pdfScanProgressCleanup(); pdfScanProgressCleanup = null; }
    flushPending();
    if (resume) {
      allPdfs = [...allPdfs, ...result.pdfs];
    } else {
      allPdfs = result.pdfs;
    }
    _pdfTotalUnfiltered = allPdfs.length;
    rebuildPdfStats();
    filterPdfs();
    if (!result.stopped) {
      try {
        await window.vstUpdater.savePdfScan(allPdfs, result.roots);
      } catch (e) { showToast(toastFmt('toast.failed_save_pdf_history', { err: e && e.message ? e.message : e }), 4000, 'error'); }
    }
    if (result.stopped && allPdfs.length > 0 && resumeBtn) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (pdfScanProgressCleanup) { pdfScanProgressCleanup(); pdfScanProgressCleanup = null; }
    flushPending();
    const errMsg = err.message || err || 'Unknown error';
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${escapeHtml(errMsg)}</p></div>`;
    showToast(toastFmt('toast.pdf_scan_failed', { err: errMsg }), 4000, 'error');
  }

  hideGlobalProgress();
  if (scanBtn) { scanBtn.disabled = false; scanBtn.innerHTML = '&#8635; Scan PDFs'; }
  if (stopBtn) stopBtn.style.display = 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
}

async function stopPdfScan() {
  await window.vstUpdater.stopPdfScan();
}
