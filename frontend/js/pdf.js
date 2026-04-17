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
let _pdfScanDbView = false;
let pdfRenderCount = 0;
let _pdfOffset = 0;
let _pdfTotalCount = 0;
let _pdfTotalCountCapped = false;
let _pdfTotalUnfiltered = 0;
/** Monotonic id so stale `dbQueryPdfs` results never overwrite a newer filter. */
let _pdfQuerySeq = 0;

function ensurePdfTableForQuery() {
    if (document.getElementById('pdfTable')) return;
    const tableWrap = document.getElementById('pdfTableWrap');
    if (!tableWrap) return;
    tableWrap.innerHTML = buildPdfTableHtml();
    if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('pdfTable'));
    if (typeof initTableColumnReorder === 'function') initTableColumnReorder('pdfTable', 'pdfColumnOrder');
}

function showPdfQueryLoading(isLoadMore) {
    ensurePdfTableForQuery();
    showTableQueryLoadingRow({
        tbodyId: 'pdfTableBody',
        rowId: 'pdfQueryLoadingRow',
        tableId: 'pdfTable',
        colspan: 9,
        append: isLoadMore,
        label: typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…',
    });
}

// Incremental stats for PDFs — avoids O(N) rebuild on every scan flush.
let _pdfStatsTotalBytes = 0;
// PDF metadata cache: path -> { pages?, pdfCreationDate?, pdfModDate? }.
// `pages`: number | null | undefined (undefined = not loaded; null = parse failed).
// Dates: string | null | undefined (undefined = not loaded; null/'' = absent in PDF).
const _pdfMetaCache = {};
let _pdfMetaRunning = false;
let _pdfMetaProgressCleanup = null;
/** Last `pdf-metadata-progress` payload for header badge repaint after `ui-idle-heavy-cpu`. */
let _pdfMetaLastProgress = null;
/** Debounced + single-flight `pdfMetadataGet` from progress events (avoids SQL churn every 100 files). */
let _pdfMetaProgDebounceTimer = null;
let _pdfMetaProgGetInFlight = false;
let _pdfMetaProgGetPending = false;
/** True when the user explicitly stops page-count extraction (toolbar / palette); skips refresh restart in `finally`. */
let _pdfMetaUserCancelled = false;

let _lastPdfSearch = '';
let _lastPdfMode = 'fuzzy';

async function fetchPdfPage() {
    const search = _lastPdfSearch || '';
    const seq = ++_pdfQuerySeq;
    const isLoadMore = _pdfOffset > 0;
    showPdfQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('pdfSearchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        // Backend only knows filesystem sort keys. When user picks 'pages' (client-side),
        // fetch by name and re-sort in renderPdfTable using `_pdfMetaCache` page counts.
        const backendSortKey = pdfSortKey === 'pages' ? 'name' : pdfSortKey;
        const result = await window.vstUpdater.dbQueryPdfs({
            search: search || null,
            sort_key: backendSortKey,
            sort_asc: pdfSortAsc,
            search_regex: _lastPdfMode === 'regex',
            offset: _pdfOffset,
            limit: PDF_PAGE_SIZE,
        });
        if (seq !== _pdfQuerySeq) return;
        // Page-at-a-time: filteredPdfs only holds the LATEST page, DOM accumulates.
        filteredPdfs = result.pdfs || [];
        _pdfTotalCount = result.totalCount || 0;
        _pdfTotalCountCapped = result.totalCountCapped === true;
        _pdfTotalUnfiltered = result.totalUnfiltered || 0;
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _pdfQuerySeq) return;
        renderPdfTable();
        if (pdfScanProgressCleanup) _pdfScanDbView = true;
        if (typeof requestIdleCallback === 'function') {
            requestIdleCallback(() => {
                void rebuildPdfStats();
            });
        } else {
            setTimeout(() => {
                void rebuildPdfStats();
            }, 0);
        }
        // Hydrate the pages cache for visible rows, then kick off background extract.
        loadPdfPagesForVisible();
    } catch (e) {
        if (seq !== _pdfQuerySeq) return;
        clearTableQueryLoadingRow('pdfQueryLoadingRow', 'pdfTable');
        showToast(toastFmt('toast.pdf_query_failed', {err: e && e.message ? e.message : e}), 4000, 'error');
    } finally {
        if (seq === _pdfQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('pdfSearchInput', false);
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
    const regexOn = typeof getSearchMode === 'function' && getSearchMode('regexPdf') === 'regex';
    const key = search.trim() + '|' + (regexOn ? 'r' : 'f');
    let displayCount = 0, displayBytes = 0, unfiltered = 0;
    {
        const cacheHit = !force && key === _lastPdfAggKey && _pdfAggCache;
        try {
            let agg;
            if (cacheHit) {
                agg = _pdfAggCache;
            } else {
                agg = await window.vstUpdater.dbPdfFilterStats(search.trim(), regexOn);
                if (typeof yieldToBrowser === 'function') await yieldToBrowser();
                _lastPdfAggKey = key;
                _pdfAggCache = agg;
            }
            displayCount = agg.count || 0;
            displayBytes = agg.totalBytes || 0;
            unfiltered = agg.totalUnfiltered || 0;
            _pdfTotalCount = displayCount;
            _pdfTotalCountCapped = agg.countCapped === true;
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
    const dcPart = _pdfTotalCountCapped ? displayCount.toLocaleString() + '+' : displayCount.toLocaleString();
    const countStr = isFiltered
        ? dcPart + ' / ' + unfiltered.toLocaleString()
        : (_pdfTotalCountCapped ? displayCount.toLocaleString() + '+' : (unfiltered || displayCount).toLocaleString());
    document.getElementById('pdfCount').textContent = countStr;
    document.getElementById('pdfTotalSize').textContent = formatAudioSize(displayBytes);
    const btn = document.getElementById('btnExportPdf');
    if (btn) btn.style.display = (unfiltered > 0 || displayCount > 0) ? '' : 'none';
    const u = unfiltered || displayCount || 0;
    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({pdf: u});
    else {
        const headerEl = document.getElementById('pdfCountHeader');
        if (headerEl) headerEl.textContent = u.toLocaleString();
    }
}

function pdfPagesUnknownHtml() {
    const t = typeof escapeHtml === 'function'
        ? escapeHtml(_pdfFmt('ui.tt.pdf_could_not_parse_pages'))
        : _pdfFmt('ui.tt.pdf_could_not_parse_pages');
    return `<span style="color:var(--text-dim);" title="${t}">?</span>`;
}

function pdfMetaPagesDisplay(m) {
    if (!m || m.pages === undefined) return '<span style="color:var(--text-dim);">—</span>';
    if (m.pages === null) return pdfPagesUnknownHtml();
    return Number(m.pages).toLocaleString();
}

function pdfMetaInfoDateDisplay(v) {
    if (v === undefined) return '<span style="color:var(--text-dim);">—</span>';
    if (v == null || v === '') return '<span style="color:var(--text-dim);">—</span>';
    const s = String(v);
    const e = typeof escapeHtml === 'function' ? escapeHtml(s) : s;
    return `<span title="${e}">${e}</span>`;
}

function mergePdfMetaFromApi(path, raw) {
    const cur = (_pdfMetaCache[path] && typeof _pdfMetaCache[path] === 'object')
        ? { ..._pdfMetaCache[path] }
        : {};
    if (raw != null && typeof raw === 'object' && !Array.isArray(raw)) {
        if ('pages' in raw) cur.pages = raw.pages;
        if ('pdfCreationDate' in raw) cur.pdfCreationDate = raw.pdfCreationDate;
        if ('pdfModDate' in raw) cur.pdfModDate = raw.pdfModDate;
    } else if (typeof raw === 'number' || raw === null) {
        cur.pages = raw;
    }
    _pdfMetaCache[path] = cur;
    patchPdfMetaRow(path);
}

function buildPdfRow(p) {
    const hp = escapeHtml(p.path);
    const checked =
        typeof batchSetForTabId === 'function' && batchSetForTabId('tabPdf').has(p.path) ? ' checked' : '';
    const rowTt = typeof escapeHtml === 'function'
        ? escapeHtml(_pdfFmt('ui.tt.pdf_row_double_click_open_default'))
        : _pdfFmt('ui.tt.pdf_row_double_click_open_default');
    const m = _pdfMetaCache[p.path];
    const pagesCell = pdfMetaPagesDisplay(m);
    const pdfCreCell = pdfMetaInfoDateDisplay(m && m.pdfCreationDate);
    const pdfModCell = pdfMetaInfoDateDisplay(m && m.pdfModDate);
    return `<tr data-pdf-path="${hp}" data-pdf-name="${escapeHtml((p.name || '').toLowerCase())}" style="cursor: pointer;" title="${rowTt}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(p.name)}">${_lastPdfSearch ? highlightMatch(p.name, _lastPdfSearch, _lastPdfMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td class="col-path" title="${hp}">${_lastPdfSearch ? highlightMatch(p.directory, _lastPdfSearch, _lastPdfMode) : escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted}</td>
    <td class="col-pages" data-pdf-pages-cell="${hp}" style="text-align:right;">${pagesCell}</td>
    <td class="col-pdf-cre" data-pdf-creation-cell="${hp}" style="font-size:11px;max-width:140px;overflow:hidden;text-overflow:ellipsis;">${pdfCreCell}</td>
    <td class="col-pdf-mod" data-pdf-mod-cell="${hp}" style="font-size:11px;max-width:140px;overflow:hidden;text-overflow:ellipsis;">${pdfModCell}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-folder" data-action="openPdfFile" data-path="${hp}" title="${hp}">&#128193;</button>
    </td>
  </tr>`;
}

function renderPdfTable() {
    clearTableQueryLoadingRow('pdfQueryLoadingRow', 'pdfTable');
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
            const av = _pdfMetaCache[a.path]?.pages ?? -1;
            const bv = _pdfMetaCache[b.path]?.pages ?? -1;
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
    const pdfHasMore = _pdfTotalCountCapped
        ? (filteredPdfs.length === PDF_PAGE_SIZE)
        : (pdfRenderCount < _pdfTotalCount);
    if (pdfHasMore) {
        const totalShown = _pdfTotalCountCapped ? _pdfTotalCount.toLocaleString() + '+' : _pdfTotalCount.toLocaleString();
        const line = catalogFmt('ui.js.load_more_hint', {
            shown: pdfRenderCount.toLocaleString(),
            total: totalShown,
        });
        tbody.insertAdjacentHTML('beforeend',
            `<tr><td colspan="9" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMorePdfs">
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
      <th data-key="col-pdf-creation" class="col-pdf-cre" style="width: 120px;">${tc('ui.export.col_pdf_creation_date')}<span class="col-resize"></span></th>
      <th data-key="col-pdf-mod" class="col-pdf-mod" style="width: 120px;">${tc('ui.export.col_pdf_mod_date')}<span class="col-resize"></span></th>
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

function sortPdf(key, forceAsc) {
    if (typeof forceAsc === 'boolean') {
        pdfSortKey = key;
        pdfSortAsc = forceAsc;
    } else if (pdfSortKey === key) {
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
    resetOffset() {
        _pdfOffset = 0;
    },
    fetchFn() {
        _lastPdfSearch = this.lastSearch || '';
        _lastPdfMode = this.lastMode || 'fuzzy';
        fetchPdfPage();
    },
});

function filterPdfs() {
    applyFilter('filterPdfs');
}

/** Full list for export when cold-loaded from SQLite left `allPdfs` empty (paginated DB model). */
const _PDF_EXPORT_MAX = 100000;

async function fetchPdfsForExport() {
    const search = _lastPdfSearch || '';
    let total = _pdfTotalCount || 0;
    if (total <= 0) {
        try {
            const backendSortKey = pdfSortKey === 'pages' ? 'name' : pdfSortKey;
            const probe = await window.vstUpdater.dbQueryPdfs({
                search: search || null,
                sort_key: backendSortKey,
                sort_asc: pdfSortAsc,
                search_regex: _lastPdfMode === 'regex',
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch {
            return [];
        }
    }
    const n = Math.min(total, _PDF_EXPORT_MAX);
    if (n <= 0) return [];
    const backendSortKey = pdfSortKey === 'pages' ? 'name' : pdfSortKey;
    const result = await window.vstUpdater.dbQueryPdfs({
        search: search || null,
        sort_key: backendSortKey,
        sort_asc: pdfSortAsc,
        search_regex: _lastPdfMode === 'regex',
        offset: 0,
        limit: n,
    });
    const pdfs = result.pdfs || [];
    if (pdfSortKey === 'pages') {
        const dir = pdfSortAsc ? 1 : -1;
        pdfs.sort((a, b) => {
            const av = _pdfMetaCache[a.path]?.pages ?? -1;
            const bv = _pdfMetaCache[b.path]?.pages ?? -1;
            return (av - bv) * dir;
        });
    }
    return pdfs;
}

function openPdfFile(path) {
    window.vstUpdater.openPdfFile(path)
        .then(() => showToast(toastFmt('toast.pdf_opened')))
        .catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}

// When `unifiedResult` is passed (by scanAll), skip this function's Tauri
// invoke and consume the shared result from a single scan_unified call.
function _shouldUpdatePdfScanBadgeUi() {
    return !(typeof isUiIdleHeavyCpu === 'function' && isUiIdleHeavyCpu());
}

function applyPdfScanStatusBarProgress(found) {
    const badge = document.getElementById('bgPdfScanBadge');
    if (!badge) return;
    if (!_shouldUpdatePdfScanBadgeUi()) {
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
        return;
    }
    const n = found == null ? 0 : found;
    badge.textContent = formatBgJobBadgeLine('pdfScan', 'ui.stats.pdf_scan_bg_progress', {n: n.toLocaleString()});
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

async function scanPdfs(resume = false, unifiedResult = null, overrideRoots = null) {
    if (typeof window !== 'undefined') window.__statusBarPdfScanJob = true;
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    applyPdfScanStatusBarProgress(0);
    const scanBtn = document.querySelector('[data-action="scanPdfs"]');
    const resumeBtn = document.getElementById('btnResumePdf');
    const stopBtn = document.getElementById('btnStopPdf');
    const progressBar = document.getElementById('pdfProgressBar');
    const progressFill = document.getElementById('pdfProgressFill');
    const tableWrap = document.getElementById('pdfTableWrap');

    if (scanBtn) {
        if (typeof btnLoading === 'function') btnLoading(scanBtn, true);
        scanBtn.disabled = true;
        scanBtn.innerHTML = '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn');
    }
    if (resumeBtn) resumeBtn.style.display = 'none';
    if (stopBtn) stopBtn.style.display = '';
    progressBar.classList.add('active');
    progressFill.style.width = '0%';

    const excludePaths = resume ? allPdfs.map(p => p.path) : null;

    if (!resume) {
        _pdfScanDbView = false;
        allPdfs = [];
        filteredPdfs = [];
        resetPdfStatsAccumulators();
        _pdfTotalUnfiltered = 0;
    }

    let scanPdfDomActive = false;
    let firstBatch = true;
    let pendingPdfs = [];
    let pendingFound = 0;
    const pdfEta = createETA();
    pdfEta.start();
    const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);

    function flushPending() {
        if (pendingPdfs.length === 0) return;
        const batch = pendingPdfs.splice(0);

        const elapsed = pdfEta.elapsed();
        if (scanBtn) {
            scanBtn.innerHTML = catalogFmt('ui.audio.scan_progress_line', {
                n: pendingFound.toLocaleString(),
                elapsed: elapsed ? ' — ' + elapsed : '',
            });
        }

        const allowDom =
            scanPdfDomActive ||
            (typeof isPdfScanTableEmpty === 'function' && isPdfScanTableEmpty());
        if (!allowDom) {
            allPdfs.push(...batch);
            if (allPdfs.length > 100000) allPdfs.length = 100000;
            if (filteredPdfs.length > 100000) filteredPdfs.length = 100000;
            filteredPdfs.push(...batch);
            accumulatePdfStats(batch);
            _pdfTotalUnfiltered = allPdfs.length;
            rebuildPdfStats();
            return;
        }
        scanPdfDomActive = true;

        if (firstBatch) {
            firstBatch = false;
            tableWrap.innerHTML = buildPdfTableHtml();
            document.getElementById('pdfStats').style.display = 'flex';
            if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('pdfTable'));
            if (typeof initTableColumnReorder === 'function') initTableColumnReorder('pdfTable', 'pdfColumnOrder');
        }

        allPdfs.push(...batch);
        if (allPdfs.length > 100000) allPdfs.length = 100000;
        if (filteredPdfs.length > 100000) filteredPdfs.length = 100000;
        filteredPdfs.push(...batch);
        accumulatePdfStats(batch);

        const tbody = document.getElementById('pdfTableBody');
        if (!_pdfScanDbView && tbody && pdfRenderCount < 2000) {
            const scanSearch = (document.getElementById('pdfSearchInput')?.value || '').trim();
            const scanMode = typeof getSearchMode === 'function' ? getSearchMode('regexPdf') : (_lastPdfMode || 'fuzzy');
            const visibleBatch = scanSearch
                ? batch.filter(p => typeof searchMatch === 'function'
                    ? searchMatch(scanSearch, [p.name || '', p.directory || ''], scanMode)
                    : (p.name || '').toLowerCase().includes(scanSearch.toLowerCase()))
                : batch;
            const toRender = visibleBatch.slice(0, 2000 - pdfRenderCount);
            tbody.insertAdjacentHTML('beforeend', toRender.map(buildPdfRow).join(''));
            pdfRenderCount += toRender.length;
        }

        _pdfTotalUnfiltered = allPdfs.length;
        rebuildPdfStats();
    }

    const scheduleFlush = createScanFlusher(flushPending, FLUSH_INTERVAL);

    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

    if (pdfScanProgressCleanup) pdfScanProgressCleanup();
    pdfScanProgressCleanup = await window.vstUpdater.onPdfScanProgress((data) => {
        if (data.phase === 'status') {
            // status message
        } else if (data.phase === 'scanning') {
            pendingPdfs.push(...data.pdfs);
            pendingFound = data.found;
            window.__pdfScanPendingFound = pendingFound;
            applyPdfScanStatusBarProgress(pendingFound);
            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({pdf: pendingFound});
            else {
                const headerEl = document.getElementById('pdfCountHeader');
                if (headerEl) headerEl.textContent = pendingFound.toLocaleString();
            }
            scheduleFlush();
        }
    });

    try {
        const pdfRoots = (overrideRoots && overrideRoots.length > 0)
            ? overrideRoots
            : (prefs.getItem('pdfScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
        const result = unifiedResult
            ? await unifiedResult
            : await window.vstUpdater.scanPdfs(pdfRoots.length ? pdfRoots : undefined, excludePaths);
        flushPending();
        scanPdfDomActive = false;
        if (result.streamed) {
            // Backend streamed results live — allPdfs was built from progress events.
        } else if (resume) {
            allPdfs = [...allPdfs, ...result.pdfs];
        } else {
            allPdfs = result.pdfs;
        }
        _pdfTotalUnfiltered = allPdfs.length;
        // Invalidate aggregate cache — fresh rows after save will be aggregated below.
        _lastPdfAggKey = null;
        _pdfAggCache = null;
        // Save BEFORE rebuildPdfStats/filterPdfs so the DB has the new rows; otherwise
        // the filter-stats query hits stale/empty data and the top counter flickers.
        if (!result.streamed) {
            try {
                await window.vstUpdater.savePdfScan(allPdfs, result.roots);
                // Scan saved — hydrate pages cache + kick background extraction
                loadPdfPagesForVisible();
            } catch (e) {
                showToast(toastFmt('toast.failed_save_pdf_history', {err: e && e.message ? e.message : e}), 4000, 'error');
            }
        } else {
            // Backend already saved — still hydrate pages cache.
            loadPdfPagesForVisible();
        }
        if (pdfScanProgressCleanup) {
            pdfScanProgressCleanup();
            pdfScanProgressCleanup = null;
        }
        _pdfScanDbView = false;
        rebuildPdfStats(true);
        filterPdfs();
        if (result.stopped && allPdfs.length > 0 && resumeBtn) {
            resumeBtn.style.display = '';
        }
        if (typeof postScanCompleteToast === 'function') {
            const n = _pdfTotalUnfiltered || allPdfs.length || 0;
            postScanCompleteToast(
                !!result.stopped,
                'toast.post_scan_pdf_complete',
                'toast.post_scan_pdf_stopped',
                {n: n.toLocaleString()},
            );
        }
    } catch (err) {
        if (pdfScanProgressCleanup) {
            pdfScanProgressCleanup();
            pdfScanProgressCleanup = null;
        }
        _pdfScanDbView = false;
        flushPending();
        scanPdfDomActive = false;
        const errMsg = err.message || err || catalogFmt('toast.unknown_error');
        const errTitle = typeof escapeHtml === 'function' ? escapeHtml(_pdfFmt('ui.audio.scan_error_title')) : _pdfFmt('ui.audio.scan_error_title');
        tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>${errTitle}</h2><p>${escapeHtml(errMsg)}</p></div>`;
        showToast(toastFmt('toast.pdf_scan_failed', {err: errMsg}), 4000, 'error');
    }

    window.__pdfScanPendingFound = 0;
    if (typeof window !== 'undefined') window.__statusBarPdfScanJob = false;
    const pdfScanBadge = document.getElementById('bgPdfScanBadge');
    if (pdfScanBadge) pdfScanBadge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    if (scanBtn) {
        scanBtn.disabled = false;
        if (typeof btnLoading === 'function') btnLoading(scanBtn, false);
        scanBtn.innerHTML = '&#8635; ' + catalogFmt('ui.btn.scan_pdfs');
    }
    if (stopBtn) stopBtn.style.display = 'none';
    progressBar.classList.remove('active');
    progressFill.style.width = '0%';
}

function clearPdfScanButtonSpinnerImmediate() {
    const scanBtn = document.querySelector('[data-action="scanPdfs"]');
    const stopBtn = document.getElementById('btnStopPdf');
    const progressBar = document.getElementById('pdfProgressBar');
    const progressFill = document.getElementById('pdfProgressFill');
    if (typeof window !== 'undefined') window.__pdfScanPendingFound = 0;
    if (typeof window !== 'undefined') window.__statusBarPdfScanJob = false;
    const pdfScanBadge = document.getElementById('bgPdfScanBadge');
    if (pdfScanBadge) pdfScanBadge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    if (scanBtn) {
        scanBtn.disabled = false;
        if (typeof btnLoading === 'function') btnLoading(scanBtn, false);
        scanBtn.innerHTML = '&#8635; ' + catalogFmt('ui.btn.scan_pdfs');
    }
    if (stopBtn) stopBtn.style.display = 'none';
    if (progressBar) progressBar.classList.remove('active');
    if (progressFill) progressFill.style.width = '0%';
}

async function stopPdfScan() {
    clearPdfScanButtonSpinnerImmediate();
    try {
        await window.vstUpdater.stopPdfScan();
    } catch (e) {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    }
}

// ── PDF metadata (page counts) ──

function patchPdfMetaRow(path) {
    const tbody = document.getElementById('pdfTableBody');
    if (!tbody) return;
    const m = _pdfMetaCache[path];
    for (const td of tbody.querySelectorAll('td[data-pdf-pages-cell]')) {
        if (td.getAttribute('data-pdf-pages-cell') !== path) continue;
        td.innerHTML = pdfMetaPagesDisplay(m);
        break;
    }
    for (const td of tbody.querySelectorAll('td[data-pdf-creation-cell]')) {
        if (td.getAttribute('data-pdf-creation-cell') !== path) continue;
        td.innerHTML = pdfMetaInfoDateDisplay(m && m.pdfCreationDate);
        break;
    }
    for (const td of tbody.querySelectorAll('td[data-pdf-mod-cell]')) {
        if (td.getAttribute('data-pdf-mod-cell') !== path) continue;
        td.innerHTML = pdfMetaInfoDateDisplay(m && m.pdfModDate);
        break;
    }
}

function isPdfMetadataAutoExtractOn() {
    return typeof prefs !== 'undefined' && prefs.getItem('pdfMetadataAutoExtract') !== 'off';
}

function shouldStartPdfMetadataExtraction(forceNoIdle) {
    if (forceNoIdle) return true;
    // Opt-in background mode: keep indexing when the window is hidden/minimized (isUiIdleHeavyCpu).
    if (isPdfMetadataAutoExtractOn()) return true;
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) return false;
    return true;
}

/** When Settings → Background PDF metadata (auto) is on, kick the batch extractor (no PDF tab required). */
function maybeStartPdfBackgroundMetadataExtraction() {
    if (!isPdfMetadataAutoExtractOn()) return;
    void startPdfMetadataExtraction();
}

function _shouldUpdatePdfMetaBadgeUi() {
    return !(typeof isUiIdleHeavyCpu === 'function' && isUiIdleHeavyCpu());
}

function clearPdfMetaBadge() {
    _pdfMetaLastProgress = null;
    const badge = document.getElementById('bgPdfMetaBadge');
    if (!badge) return;
    badge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

/**
 * Paint PDF metadata progress on the status bar before `pdfMetadataExtractBatch` runs.
 * Progress events can be delivered after the `invoke` promise settles; without this, `finally`
 * may clear the badge while the row never showed a label.
 */
function paintPdfMetaBadgeQueuedTotal(total) {
    const badge = document.getElementById('bgPdfMetaBadge');
    if (!badge) return;
    if (total != null && Number.isFinite(total) && total > 0) {
        badge.textContent = formatBgJobBadgeLine('pdfMeta', 'ui.stats.pdf_meta_bg_progress', {done: 0, total});
    } else {
        badge.textContent = formatBgJobBadgeLine('pdfMeta', 'ui.stats.pdf_meta_bg_working');
    }
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function applyPdfMetaBadgePayload(pl) {
    if (!pl || typeof pl !== 'object') return;
    const badge = document.getElementById('bgPdfMetaBadge');
    const phase = pl.phase;
    if (phase === 'done' || phase === 'aborted') {
        clearPdfMetaBadge();
        return;
    }
    if (phase === 'start') {
        _pdfMetaLastProgress = pl;
        if (!badge || !_shouldUpdatePdfMetaBadgeUi()) {
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            return;
        }
        const t = pl.total;
        if (t != null && Number.isFinite(t) && t > 0) {
            badge.textContent = formatBgJobBadgeLine('pdfMeta', 'ui.stats.pdf_meta_bg_progress', {done: 0, total: t});
        } else {
            badge.textContent = formatBgJobBadgeLine('pdfMeta', 'ui.stats.pdf_meta_bg_working');
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
        return;
    }
    if (phase === 'progress') {
        _pdfMetaLastProgress = pl;
        if (!badge || !_shouldUpdatePdfMetaBadgeUi()) {
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            return;
        }
        if (pl.done != null && pl.total != null && Number.isFinite(pl.total)) {
            badge.textContent = formatBgJobBadgeLine('pdfMeta', 'ui.stats.pdf_meta_bg_progress', {done: pl.done, total: pl.total});
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    }
}

function clearPdfMetaProgressDebounceState() {
    if (_pdfMetaProgDebounceTimer) {
        clearTimeout(_pdfMetaProgDebounceTimer);
        _pdfMetaProgDebounceTimer = null;
    }
    _pdfMetaProgGetPending = false;
}

function schedulePdfMetaProgressPageFetch() {
    if (_pdfMetaProgDebounceTimer) clearTimeout(_pdfMetaProgDebounceTimer);
    _pdfMetaProgDebounceTimer = setTimeout(() => {
        _pdfMetaProgDebounceTimer = null;
        void runPdfMetaProgressPageFetch();
    }, 300);
}

async function runPdfMetaProgressPageFetch() {
    const missing = filteredPdfs.slice(0, 2000).map(r => r.path).filter(p => _pdfMetaCache[p] === undefined);
    if (missing.length === 0) return;
    if (_pdfMetaProgGetInFlight) {
        _pdfMetaProgGetPending = true;
        return;
    }
    _pdfMetaProgGetInFlight = true;
    try {
        const map = await window.vstUpdater.pdfMetadataGet(missing);
        for (const [path, raw] of Object.entries(map || {})) {
            mergePdfMetaFromApi(path, raw);
        }
    } catch {
    } finally {
        _pdfMetaProgGetInFlight = false;
        if (_pdfMetaProgGetPending) {
            _pdfMetaProgGetPending = false;
            void runPdfMetaProgressPageFetch();
        }
    }
}

async function abortPdfMetadataExtraction() {
    try {
        if (window.vstUpdater && typeof window.vstUpdater.pdfMetadataExtractAbort === 'function') {
            await window.vstUpdater.pdfMetadataExtractAbort();
        }
    } catch {
    }
}

function syncPdfMetaExtractStopButton() {
    const el = document.getElementById('btnStopPdfMeta');
    if (!el) return;
    el.style.display = _pdfMetaRunning ? '' : 'none';
}

async function stopPdfMetadataExtractionUser() {
    const wasRunning = _pdfMetaRunning;
    if (wasRunning) _pdfMetaUserCancelled = true;
    await abortPdfMetadataExtraction();
    if (wasRunning && typeof showToast === 'function') {
        showToast(toastFmt('toast.pdf_metadata_extract_stopped'), 2500);
    }
}

// Load cached page counts from DB for currently-visible rows (when the PDF table exists), then
// optionally start the background extractor — gated only by Settings → pdfMetadataAutoExtract.
async function loadPdfPagesForVisible() {
    const rows = filteredPdfs.slice(0, 2000);
    const paths = rows.map(r => r.path);
    const canPatchUi = typeof document !== 'undefined' && document.getElementById('pdfTableBody');
    if (paths.length > 0 && canPatchUi) {
        try {
            const map = await window.vstUpdater.pdfMetadataGet(paths);
            for (const [path, raw] of Object.entries(map || {})) {
                mergePdfMetaFromApi(path, raw);
            }
        } catch { /* ignore — rows stay at "—" */
        }
    }
    maybeStartPdfBackgroundMetadataExtraction();
}

async function startPdfMetadataExtraction(opts) {
    const forceNoIdle = opts && opts.forceNoIdle === true;
    if (_pdfMetaRunning) return;
    if (!shouldStartPdfMetadataExtraction(forceNoIdle)) return;
    _pdfMetaUserCancelled = false;
    _pdfMetaRunning = true;
    if (typeof window !== 'undefined') window.__statusBarPdfMetaJob = true;
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    syncPdfMetaExtractStopButton();
    let hadUncachedWork = false;
    let extractionFailed = false;
    try {
        const uncached = await window.vstUpdater.pdfMetadataUnindexed(100000);
        if (_pdfMetaUserCancelled) {
            _pdfMetaUserCancelled = false;
            return;
        }
        if (!Array.isArray(uncached) || uncached.length === 0) return;
        hadUncachedWork = true;
        // Listen to progress events to patch cells as they resolve
        if (_pdfMetaProgressCleanup) {
            _pdfMetaProgressCleanup();
            _pdfMetaProgressCleanup = null;
        }
        clearPdfMetaProgressDebounceState();
        _pdfMetaProgressCleanup = await window.vstUpdater.onPdfMetadataProgress((payload) => {
            applyPdfMetaBadgePayload(payload);
            schedulePdfMetaProgressPageFetch();
        });
        paintPdfMetaBadgeQueuedTotal(uncached.length);
        await window.vstUpdater.pdfMetadataExtractBatch(uncached);
        await new Promise((r) => setTimeout(r, 0));
    } catch (e) {
        extractionFailed = true;
        if (window.vstUpdater?.appendLog) {
            window.vstUpdater.appendLog(
                `PDF META BG — session error: ${e && e.message ? e.message : e}`
            );
        }
        if (typeof showToast === 'function') {
            showToast(toastFmt('toast.pdf_metadata_extract_failed', {err: e && e.message ? e.message : e}), 4000, 'error');
        }
    } finally {
        clearPdfMetaProgressDebounceState();
        if (_pdfMetaProgressCleanup) {
            _pdfMetaProgressCleanup();
            _pdfMetaProgressCleanup = null;
        }
        clearPdfMetaBadge();
        const skipRefreshAfterUserStop = _pdfMetaUserCancelled;
        _pdfMetaUserCancelled = false;
        _pdfMetaRunning = false;
        if (typeof window !== 'undefined') window.__statusBarPdfMetaJob = false;
        syncPdfMetaExtractStopButton();
        if (
            typeof showToast === 'function' &&
            hadUncachedWork &&
            !skipRefreshAfterUserStop &&
            !extractionFailed
        ) {
            showToast(toastFmt('toast.pdf_metadata_extract_complete'), 2500);
        }
        // Final reload for any rows we missed via progress events (not after explicit user stop — avoids immediate restart)
        if (hadUncachedWork && !skipRefreshAfterUserStop) loadPdfPagesForVisible();
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    }
}

// Triggered from command palette / context menu to force re-extraction.
async function buildPdfPagesCache() {
    // Force re-scan of all paths currently loaded in the PDF table
    for (const p of allPdfs) delete _pdfMetaCache[p.path];
    await abortPdfMetadataExtraction();
    const deadline = Date.now() + 120000;
    while (_pdfMetaRunning && Date.now() < deadline) {
        await new Promise((r) => setTimeout(r, 50));
    }
    if (typeof showToast === 'function') showToast(toastFmt('toast.pdf_extracting_metadata'), 3000);
    await startPdfMetadataExtraction({ forceNoIdle: true });
}

(function initPdfMetadataExtractionLifecycle() {
    if (typeof document === 'undefined') return;
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        const idle = e && e.detail && e.detail.idle;
        if (idle) {
            if (!isPdfMetadataAutoExtractOn()) void abortPdfMetadataExtraction();
        } else {
            if (_pdfMetaRunning && _pdfMetaLastProgress) {
                applyPdfMetaBadgePayload(_pdfMetaLastProgress);
            }
            if (window.__statusBarPdfScanJob) {
                const n = typeof window.__pdfScanPendingFound === 'number' ? window.__pdfScanPendingFound : 0;
                applyPdfScanStatusBarProgress(n);
            }
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            if (typeof loadPdfPagesForVisible === 'function') {
                void loadPdfPagesForVisible();
            }
        }
    });
})();

/** Settings → auto PDF metadata: run page-count extraction once after startup (full library pass). */
function maybeAutoStartPdfMetadataOnStartup() {
    try {
        if (typeof prefs === 'undefined' || prefs.getItem('autoPdfMetadataOnStartup') !== 'on') return;
        if (typeof startPdfMetadataExtraction !== 'function') return;
        void startPdfMetadataExtraction({ forceNoIdle: true });
    } catch {
        /* ignore */
    }
}

/** Settings → auto PDF scan: run once at launch when the indexed count is still zero. */
function maybeAutoStartPdfScanOnStartup() {
    try {
        if (typeof prefs === 'undefined' || prefs.getItem('autoPdfScanOnStartup') !== 'on') return;
        const roots = (prefs.getItem('pdfScanDirs') || '').split('\n').map((s) => s.trim()).filter(Boolean);
        if (roots.length === 0) return;
        const n = typeof _pdfTotalCount === 'number' ? _pdfTotalCount : 0;
        if (n > 0) return;
        if (typeof scanPdfs === 'function') void scanPdfs();
    } catch {
        /* ignore */
    }
}

// Export for menu-action handler in ipc.js
window.buildPdfPagesCache = buildPdfPagesCache;
window.stopPdfMetadataExtractionUser = stopPdfMetadataExtractionUser;
