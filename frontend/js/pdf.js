// ── PDFs ──
function _pdfFmt(key, vars) {
  if (typeof appFmt !== 'function') return key;
  return vars ? appFmt(key, vars) : appFmt(key);
}

let allPdfs = [];
let filteredPdfs = [];
let pdfSortKey = 'name';
let pdfSortAsc = true;
let pdfScanProgressCleanup = null;
let PDF_PAGE_SIZE = 200;
let pdfRenderCount = 0;
let _pdfOffset = 0;
let _pdfTotalCount = 0;
let _pdfTotalUnfiltered = 0;
// Incremental stats for PDFs — avoids O(N) rebuild on every scan flush.
let _pdfStatsTotalBytes = 0;
// Page-count cache: path -> number (or null if extraction failed).
// Populated lazily via background extractor + DB on-demand lookups.
const _pdfPagesCache = {};
let _pdfMetaRunning = false;
let _pdfMetaProgressCleanup = null;

let _lastPdfSearch = '';
let _lastPdfMode = 'fuzzy';

async function fetchPdfPage() {
  const search = _lastPdfSearch || '';
  if (typeof showGlobalProgress === 'function') showGlobalProgress();
  // During an active scan, DOM-toggle filter existing rendered rows instead of
  // hitting the DB (scan isn't saved yet, query would wipe live results).
  if (pdfScanProgressCleanup) {
    const tbody = document.getElementById('pdfTableBody');
    if (tbody) {
      const needle = search ? search.trim().toLowerCase() : '';
      const mode = _lastPdfMode;
      const rows = tbody.rows;
      let visible = 0;
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i];
        const name = r.dataset.pdfName;
        if (name === undefined) continue;
        const match = !needle || name.includes(needle);
        r.style.display = match ? '' : 'none';
        if (match) {
          visible++;
          const nameCell = r.querySelector('.col-name');
          if (nameCell) applyScanCellHighlight(nameCell, nameCell.title, search, mode, highlightMatch);
          const pathCell = r.querySelector('.col-path');
          if (pathCell) applyScanCellHighlight(pathCell, pathCell.title.replace(/[/\\][^/\\]*$/, ''), search, mode, highlightMatch);
        }
      }
      _pdfTotalUnfiltered = allPdfs.length;
      _pdfTotalCount = needle ? visible : _pdfTotalUnfiltered;
      rebuildPdfStats();
    }
    if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
    return;
  }
  try {
    // Backend only knows filesystem sort keys. When user picks 'pages' (client-side),
    // fetch by name and re-sort in renderPdfTable using the _pdfPagesCache.
    const backendSortKey = pdfSortKey === 'pages' ? 'name' : pdfSortKey;
    const result = await window.vstUpdater.dbQueryPdfs({
      search: search || null,
      sort_key: backendSortKey,
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
    // Page-at-a-time: filteredPdfs only holds the LATEST page, DOM accumulates.
    filteredPdfs = pdfs;
    _pdfTotalCount = result.totalCount || 0;
    _pdfTotalUnfiltered = result.totalUnfiltered || 0;
    renderPdfTable();
    rebuildPdfStats();
    // Hydrate the pages cache for visible rows, then kick off background extract.
    loadPdfPagesForVisible();
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

let _lastPdfAggKey = null;
let _pdfAggCache = null;
async function rebuildPdfStats(force) {
  const statsEl = document.getElementById('pdfStats');
  if (!statsEl) return;
  const search = document.getElementById('pdfSearchInput')?.value || '';
  const key = search.trim();
  let displayCount = 0, displayBytes = 0, unfiltered = 0;
  // During an active scan, the DB doesn't have the new data yet (save happens at
  // scan-end). Use the in-memory accumulators so the counter reflects live progress.
  if (pdfScanProgressCleanup) {
    const needle = key.toLowerCase();
    if (needle) {
      let c = 0, b = 0;
      for (const p of allPdfs) {
        if ((p.name || '').toLowerCase().includes(needle) || (p.path || '').toLowerCase().includes(needle)) {
          c++; b += (p.size || 0);
        }
      }
      displayCount = c; displayBytes = b;
    } else {
      displayCount = allPdfs.length;
      if (_pdfStatsTotalBytes === 0 && allPdfs.length > 0) accumulatePdfStats(allPdfs);
      displayBytes = _pdfStatsTotalBytes;
    }
    unfiltered = allPdfs.length;
    _pdfTotalCount = displayCount;
    _pdfTotalUnfiltered = unfiltered;
  } else {
    const cacheHit = !force && key === _lastPdfAggKey && _pdfAggCache;
    try {
      const agg = cacheHit ? _pdfAggCache : await window.vstUpdater.dbPdfFilterStats(search.trim());
      if (!cacheHit) { _lastPdfAggKey = key; _pdfAggCache = agg; }
      displayCount = agg.count || 0;
      displayBytes = agg.totalBytes || 0;
      unfiltered = agg.totalUnfiltered || 0;
      _pdfTotalCount = displayCount;
      _pdfTotalUnfiltered = unfiltered;
    } catch {
      displayCount = allPdfs.length;
      if (_pdfStatsTotalBytes === 0 && allPdfs.length > 0) accumulatePdfStats(allPdfs);
      displayBytes = _pdfStatsTotalBytes;
      unfiltered = allPdfs.length;
    }
  }
  const isFiltered = search.trim() && displayCount < unfiltered;
  statsEl.style.display = (displayCount > 0 || unfiltered > 0) ? 'flex' : 'none';
  const countStr = isFiltered
    ? displayCount.toLocaleString() + ' / ' + unfiltered.toLocaleString()
    : (unfiltered || displayCount).toLocaleString();
  document.getElementById('pdfCount').textContent = countStr;
  document.getElementById('pdfTotalSize').textContent = formatAudioSize(displayBytes);
  const btn = document.getElementById('btnExportPdf');
  if (btn) btn.style.display = (unfiltered > 0 || displayCount > 0) ? '' : 'none';
  // Mirror into the global stats-bar counter (top of app). Always unfiltered —
  // the top counter must not react to the active search/filter.
  const headerEl = document.getElementById('pdfCountHeader');
  if (headerEl) headerEl.textContent = (unfiltered || displayCount || 0).toLocaleString();
}

function pdfPagesUnknownHtml() {
  const t = typeof escapeHtml === 'function'
    ? escapeHtml(_pdfFmt('ui.tt.pdf_could_not_parse_pages'))
    : _pdfFmt('ui.tt.pdf_could_not_parse_pages');
  return `<span style="color:var(--text-dim);" title="${t}">?</span>`;
}

function buildPdfRow(p) {
  const hp = escapeHtml(p.path);
  const checked = batchSelected.has(p.path) ? ' checked' : '';
  const rowTt = typeof escapeHtml === 'function'
    ? escapeHtml(_pdfFmt('ui.tt.pdf_row_double_click_open_default'))
    : _pdfFmt('ui.tt.pdf_row_double_click_open_default');
  const cached = _pdfPagesCache[p.path];
  const pagesCell = cached === undefined ? '<span style="color:var(--text-dim);">—</span>'
    : cached === null ? pdfPagesUnknownHtml()
    : cached.toLocaleString();
  return `<tr data-pdf-path="${hp}" data-pdf-name="${escapeHtml((p.name || '').toLowerCase())}" style="cursor: pointer;" title="${rowTt}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(p.name)}">${_lastPdfSearch ? highlightMatch(p.name, _lastPdfSearch, _lastPdfMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td class="col-path" title="${hp}">${_lastPdfSearch ? highlightMatch(p.directory, _lastPdfSearch, _lastPdfMode) : escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted}</td>
    <td class="col-pages" data-pdf-pages-cell="${hp}" style="text-align:right;">${pagesCell}</td>
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
  // Pages sort is client-side only (the backend doesn't JOIN pdf_metadata).
  // We sort the already-fetched page, values the user hasn't extracted show as -1.
  if (pdfSortKey === 'pages') {
    const dir = pdfSortAsc ? 1 : -1;
    filteredPdfs.sort((a, b) => {
      const av = _pdfPagesCache[a.path] ?? -1;
      const bv = _pdfPagesCache[b.path] ?? -1;
      return (av - bv) * dir;
    });
  }
  const tbody = document.getElementById('pdfTableBody');
  if (!tbody) return;
  // Page-at-a-time: offset=0 replaces DOM, subsequent pages append. Matches audio.js.
  pdfRenderCount = _pdfOffset + filteredPdfs.length;
  if (_pdfOffset === 0) {
    tbody.innerHTML = filteredPdfs.map(buildPdfRow).join('');
  } else {
    const loadMoreRow = tbody.querySelector('tr [data-action="loadMorePdfs"]')?.closest('tr');
    if (loadMoreRow) loadMoreRow.remove();
    tbody.insertAdjacentHTML('beforeend', filteredPdfs.map(buildPdfRow).join(''));
  }
  if (pdfRenderCount < _pdfTotalCount) {
    const line = catalogFmt('ui.js.load_more_hint', {
      shown: pdfRenderCount.toLocaleString(),
      total: _pdfTotalCount.toLocaleString(),
    });
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="7" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMorePdfs">
        ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
      </td></tr>`);
  }
}

function buildPdfTableHtml() {
  const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
  const sel = typeof escapeHtml === 'function' ? escapeHtml(tc('ui.audio.th_select_all')) : tc('ui.audio.th_select_all');
  return `<table class="audio-table" id="pdfTable">
    <thead><tr>
      <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${sel}"></th>
      <th data-action="sortPdf" data-key="name" style="width: 30%;">${tc('ui.export.col_name')} <span class="sort-arrow" id="pdfSortArrowName">&#9660;</span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="directory" style="width: 40%;">${tc('ui.export.col_path')} <span class="sort-arrow" id="pdfSortArrowDirectory"></span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="size" class="col-size" style="width: 90px;">${tc('ui.export.col_size')} <span class="sort-arrow" id="pdfSortArrowSize"></span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="pages" class="col-pages" style="width: 70px;text-align:right;">${tc('ui.export.col_pages')} <span class="sort-arrow" id="pdfSortArrowPages"></span><span class="col-resize"></span></th>
      <th data-action="sortPdf" data-key="modified" class="col-date" style="width: 100px;">${tc('ui.export.col_modified')} <span class="sort-arrow" id="pdfSortArrowModified"></span><span class="col-resize"></span></th>
      <th class="col-actions" style="width: 50px;"></th>
    </tr></thead>
    <tbody id="pdfTableBody"></tbody>
  </table>`;
}

function loadMorePdfs() {
  _pdfOffset = pdfRenderCount;
  fetchPdfPage();
}

function sortPdf(key) {
  if (pdfSortKey === key) {
    pdfSortAsc = !pdfSortAsc;
  } else {
    pdfSortKey = key;
    pdfSortAsc = true;
  }
  ['Name', 'Size', 'Pages', 'Modified', 'Directory'].forEach(k => {
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

// When `unifiedResult` is passed (by scanAll), skip this function's Tauri
// invoke and consume the shared result from a single scan_unified call.
async function scanPdfs(resume = false, unifiedResult = null) {
  showGlobalProgress();
  const scanBtn = document.querySelector('[data-action="scanPdfs"]');
  const resumeBtn = document.getElementById('btnResumePdf');
  const stopBtn = document.getElementById('btnStopPdf');
  const progressBar = document.getElementById('pdfProgressBar');
  const progressFill = document.getElementById('pdfProgressFill');
  const tableWrap = document.getElementById('pdfTableWrap');

  const excludePaths = resume ? allPdfs.map(p => p.path) : null;

  if (scanBtn) {
    if (typeof btnLoading === 'function') btnLoading(scanBtn, true);
    scanBtn.disabled = true;
    scanBtn.innerHTML = '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn');
  }
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
    {
      const h2 = typeof escapeHtml === 'function' ? escapeHtml(_pdfFmt('ui.pdf.scanning_title')) : _pdfFmt('ui.pdf.scanning_title');
      const sub = typeof escapeHtml === 'function' ? escapeHtml(_pdfFmt('ui.audio.scanning_sub')) : _pdfFmt('ui.audio.scanning_sub');
      tableWrap.innerHTML = `<div class="state-message"><div class="spinner"></div><h2>${h2}</h2><p>${sub}</p></div>`;
    }
  }

  let firstBatch = true;
  let pendingPdfs = [];
  let pendingFound = 0;
  const pdfEta = createETA();
  pdfEta.start();
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);

  function flushPending() {
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
    // Cap in-memory array to prevent OOM on 1M+ scans — DB has authoritative data.
    if (allPdfs.length > 100000) allPdfs.length = 100000;
    if (filteredPdfs.length > 100000) filteredPdfs.length = 100000;
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
    if (scanBtn) {
      scanBtn.innerHTML = catalogFmt('ui.audio.scan_progress_line', {
        n: pendingFound.toLocaleString(),
        elapsed: elapsed ? ' — ' + elapsed : '',
      });
    }
  }

  const scheduleFlush = createScanFlusher(flushPending, FLUSH_INTERVAL);

  if (pdfScanProgressCleanup) pdfScanProgressCleanup();
  pdfScanProgressCleanup = window.vstUpdater.onPdfScanProgress((data) => {
    if (data.phase === 'status') {
      // status message
    } else if (data.phase === 'scanning') {
      pendingPdfs.push(...data.pdfs);
      pendingFound = data.found;
      const headerEl = document.getElementById('pdfCountHeader');
      if (headerEl) headerEl.textContent = pendingFound.toLocaleString();
      scheduleFlush();
    }
  });

  try {
    const pdfRoots = (prefs.getItem('pdfScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = unifiedResult
      ? await unifiedResult
      : await window.vstUpdater.scanPdfs(pdfRoots.length ? pdfRoots : undefined, excludePaths);
    flushPending();
    if (result.streamed) {
      // Backend streamed results live — allPdfs was built from progress events.
    } else if (resume) {
      allPdfs = [...allPdfs, ...result.pdfs];
    } else {
      allPdfs = result.pdfs;
    }
    _pdfTotalUnfiltered = allPdfs.length;
    // Invalidate aggregate cache — fresh rows after save will be aggregated below.
    _lastPdfAggKey = null; _pdfAggCache = null;
    // Save BEFORE rebuildPdfStats/filterPdfs so the DB has the new rows; otherwise
    // the filter-stats query hits stale/empty data and the top counter flickers.
    if (!result.streamed) {
      try {
        await window.vstUpdater.savePdfScan(allPdfs, result.roots);
        // Scan saved — hydrate pages cache + kick background extraction
        loadPdfPagesForVisible();
      } catch (e) { showToast(toastFmt('toast.failed_save_pdf_history', { err: e && e.message ? e.message : e }), 4000, 'error'); }
    } else {
      // Backend already saved — still hydrate pages cache.
      loadPdfPagesForVisible();
    }
    if (pdfScanProgressCleanup) { pdfScanProgressCleanup(); pdfScanProgressCleanup = null; }
    rebuildPdfStats(true);
    filterPdfs();
    if (result.stopped && allPdfs.length > 0 && resumeBtn) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (pdfScanProgressCleanup) { pdfScanProgressCleanup(); pdfScanProgressCleanup = null; }
    flushPending();
    const errMsg = err.message || err || catalogFmt('toast.unknown_error');
    const errTitle = typeof escapeHtml === 'function' ? escapeHtml(_pdfFmt('ui.audio.scan_error_title')) : _pdfFmt('ui.audio.scan_error_title');
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>${errTitle}</h2><p>${escapeHtml(errMsg)}</p></div>`;
    showToast(toastFmt('toast.pdf_scan_failed', { err: errMsg }), 4000, 'error');
  }

  hideGlobalProgress();
  if (scanBtn) {
    scanBtn.disabled = false;
    if (typeof btnLoading === 'function') btnLoading(scanBtn, false);
    scanBtn.innerHTML = '&#8635; ' + catalogFmt('ui.btn.scan_pdfs');
  }
  if (stopBtn) stopBtn.style.display = 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
}

async function stopPdfScan() {
  await window.vstUpdater.stopPdfScan();
}

// ── PDF metadata (page counts) ──

// Patch a single row's Pages cell without re-rendering the whole table.
function patchPdfPagesCell(path, pages) {
  const tbody = document.getElementById('pdfTableBody');
  if (!tbody) return;
  // Escape matches the attribute format set in buildPdfRow
  const escaped = typeof escapeHtml === 'function' ? escapeHtml(path) : path;
  const cell = tbody.querySelector('[data-pdf-pages-cell="' + CSS.escape(escaped) + '"]');
  if (!cell) return;
  if (pages == null) {
    cell.innerHTML = pdfPagesUnknownHtml();
  } else {
    cell.textContent = Number(pages).toLocaleString();
  }
}

// Load cached page counts from DB for currently-visible rows, then trigger a
// background extraction pass for any paths still uncached.
async function loadPdfPagesForVisible() {
  const rows = filteredPdfs.slice(0, 2000);
  const paths = rows.map(r => r.path);
  if (paths.length === 0) return;
  try {
    const map = await window.vstUpdater.pdfMetadataGet(paths);
    for (const [path, pages] of Object.entries(map || {})) {
      _pdfPagesCache[path] = pages; // null if extraction previously failed
      patchPdfPagesCell(path, pages);
    }
  } catch { /* ignore — rows stay at "—" */ }
  // Fire-and-forget: kick the background extractor for paths still missing.
  startPdfMetadataExtraction();
}

async function startPdfMetadataExtraction() {
  if (_pdfMetaRunning) return;
  _pdfMetaRunning = true;
  try {
    const uncached = await window.vstUpdater.pdfMetadataUnindexed(100000);
    if (!Array.isArray(uncached) || uncached.length === 0) return;
    // Listen to progress events to patch cells as they resolve
    if (_pdfMetaProgressCleanup) { _pdfMetaProgressCleanup(); _pdfMetaProgressCleanup = null; }
    _pdfMetaProgressCleanup = window.vstUpdater.onPdfMetadataProgress(() => {
      // After each progress event, re-fetch the visible paths that are still missing
      const missing = filteredPdfs.slice(0, 2000).map(r => r.path).filter(p => _pdfPagesCache[p] === undefined);
      if (missing.length === 0) return;
      window.vstUpdater.pdfMetadataGet(missing).then(map => {
        for (const [path, pages] of Object.entries(map || {})) {
          _pdfPagesCache[path] = pages;
          patchPdfPagesCell(path, pages);
        }
      }).catch(() => {});
    });
    await window.vstUpdater.pdfMetadataExtractBatch(uncached);
  } catch (e) {
    if (typeof showToast === 'function') {
      showToast(toastFmt('toast.pdf_page_extract_failed', { err: e && e.message ? e.message : e }), 4000, 'error');
    }
  } finally {
    if (_pdfMetaProgressCleanup) { _pdfMetaProgressCleanup(); _pdfMetaProgressCleanup = null; }
    _pdfMetaRunning = false;
    // Final reload for any rows we missed via progress events
    loadPdfPagesForVisible();
  }
}

// Triggered from command palette / context menu to force re-extraction.
async function buildPdfPagesCache() {
  // Force re-scan of ALL latest-scan paths
  for (const p of allPdfs) delete _pdfPagesCache[p.path];
  _pdfMetaRunning = false;
  if (typeof showToast === 'function') showToast(toastFmt('toast.pdf_extracting_page_counts'), 3000);
  await startPdfMetadataExtraction();
  if (typeof showToast === 'function') showToast(toastFmt('toast.pdf_page_extract_complete'), 2500);
}
