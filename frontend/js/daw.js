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
let _dawScanDbView = false;
let _dawOffset = 0;
let _dawTotalCount = 0;
let _dawTotalUnfiltered = 0;
/** Monotonic id so stale `dbQueryDaw` results never overwrite a newer filter. */
let _dawQuerySeq = 0;

function ensureDawTableForQuery() {
    if (document.getElementById('dawTable')) return;
    initDawTable();
}

function showDawQueryLoading(isLoadMore) {
    ensureDawTableForQuery();
    showTableQueryLoadingRow({
        tbodyId: 'dawTableBody',
        rowId: 'dawQueryLoadingRow',
        tableId: 'dawTable',
        colspan: 8,
        append: isLoadMore,
        label: typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…',
    });
}

let dawStatCounts = {};
let dawStatBytes = 0;
// Snapshot of unfiltered per-DAW counts + total bytes from the library (deduped by path).
// Set once on mount / post-scan via dbDawFilterStats so filter changes DON'T wipe it.
let _dawStatsSnapshot = null;

async function fetchDawPage() {
    const search = _lastDawSearch || '';
    const dawSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('dawDawFilter') : null;
    const dawFilter = dawSet ? [...dawSet].join(',') : null;
    const seq = ++_dawQuerySeq;
    const isLoadMore = _dawOffset > 0;
    showDawQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('dawSearchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        const result = await window.vstUpdater.dbQueryDaw({
            search: search || null,
            daw_filter: dawFilter,
            search_regex: _lastDawMode === 'regex',
            sort_key: dawSortKey,
            sort_asc: dawSortAsc,
            offset: _dawOffset,
            limit: typeof DAW_PAGE_SIZE !== 'undefined' ? DAW_PAGE_SIZE : 200,
        });
        if (seq !== _dawQuerySeq) return;
        let projects = result.projects || [];
        // Re-sort by fzf relevance score
        if (search && projects.length > 1) {
            const scored = projects.map(p => ({p, score: searchScore(search, [p.name], _lastDawMode)}));
            scored.sort((a, b) => b.score - a.score);
            projects = scored.map(x => x.p);
        }
        // Page-at-a-time: filteredDawProjects only holds the LATEST page, DOM accumulates.
        // This keeps JS memory bounded at one page regardless of scan size (6M+ safe).
        filteredDawProjects = projects;
        _dawTotalCount = result.totalCount || 0;
        _dawTotalUnfiltered = result.totalUnfiltered || 0;
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _dawQuerySeq) return;
        renderDawTable();
        if (dawScanProgressCleanup) _dawScanDbView = true;
        // Header totals from paginated query; per-DAW toolbar breakdown debounced (matches Samples tab).
        updateDawStats();
        rebuildDawFilterStats();
    } catch (e) {
        if (seq !== _dawQuerySeq) return;
        clearTableQueryLoadingRow('dawQueryLoadingRow', 'dawTable');
        showToast(toastFmt('toast.daw_query_failed', {err: e}), 4000, 'error');
    } finally {
        if (seq === _dawQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('dawSearchInput', false);
    }
}

function resetDawStats() {
    dawStatCounts = {};
    dawStatBytes = 0;
    _dawTotalCount = 0;
    _dawTotalUnfiltered = 0;
    // Drop any stale DB snapshot from a prior scan — otherwise updateDawStats
    // would read from the old snapshot while the new scan's accumulator fills
    // dawStatCounts, causing the stats row to lag the scan button counter.
    _dawStatsSnapshot = null;
    _lastDawAggKey = null;
    _pendingDawAggKey = '';
    if (_dawStatsDebounceTimer !== null) {
        clearTimeout(_dawStatsDebounceTimer);
        _dawStatsDebounceTimer = null;
    }
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
    // Use filter-aware snapshot — counts + size reflect current search/filter.
    const src = _dawStatsSnapshot ? _dawStatsSnapshot.counts : dawStatCounts;
    const bytes = _dawStatsSnapshot ? _dawStatsSnapshot.totalBytes : dawStatBytes;
    const ableton = src['Ableton Live'] || 0;
    const logic = src['Logic Pro'] || 0;
    const fl = src['FL Studio'] || 0;
    const reaper = src['REAPER'] || 0;
    const mainDaws = ableton + logic + fl + reaper;
    const dawDisplayCount = _dawTotalCount || _dawTotalUnfiltered || 0;
    const unfiltered = _dawTotalUnfiltered || 0;
    const isFiltered = unfiltered > 0 && dawDisplayCount > 0 && dawDisplayCount < unfiltered;
    const countStr = isFiltered
        ? dawDisplayCount.toLocaleString() + ' / ' + unfiltered.toLocaleString()
        : dawDisplayCount.toLocaleString();
    document.getElementById('dawTotalCount').textContent = countStr;
    document.getElementById('dawAbletonCount').textContent = ableton;
    document.getElementById('dawLogicCount').textContent = logic;
    document.getElementById('dawFlCount').textContent = fl;
    document.getElementById('dawReaperCount').textContent = reaper;
    document.getElementById('dawOtherCount').textContent = Math.max(0, dawDisplayCount - mainDaws);
    document.getElementById('dawTotalSize').textContent = formatAudioSize(bytes);
    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({daw: unfiltered || dawDisplayCount});
    else document.getElementById('dawProjectCount').textContent = (unfiltered || dawDisplayCount).toLocaleString();
    document.getElementById('btnExportDaw').style.display = (unfiltered > 0 || dawDisplayCount > 0) ? '' : 'none';
    if (typeof updateDawDiskUsage === 'function') updateDawDiskUsage();
}

let _lastDawAggKey = null;
let _pendingDawAggKey = '';
/** Debounce heavy `GROUP BY daw` stats IPC so typing a filter does not stack a second round-trip every keystroke. */
let _dawStatsDebounceTimer = null;
const DAW_STATS_DEBOUNCE_MS = 450;

async function rebuildDawFilterStats(force) {
    try {
        const search = (_lastDawSearch || '').trim();
        const dawSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('dawDawFilter') : null;
        const dawFilter = dawSet ? [...dawSet].join(',') : null;
        const key = search + '|' + (dawFilter || '') + '|' + (_lastDawMode === 'regex' ? 'r' : 'f');
        if (!force && key === _lastDawAggKey) {
            updateDawStats();
            return;
        }
        _pendingDawAggKey = key;
        updateDawStats();

        if (force) {
            if (_dawStatsDebounceTimer !== null) {
                clearTimeout(_dawStatsDebounceTimer);
                _dawStatsDebounceTimer = null;
            }
            await _runDawFilterStatsAgg();
            return;
        }
        if (_dawStatsDebounceTimer !== null) {
            clearTimeout(_dawStatsDebounceTimer);
        }
        _dawStatsDebounceTimer = setTimeout(() => {
            _dawStatsDebounceTimer = null;
            void _runDawFilterStatsAgg();
        }, DAW_STATS_DEBOUNCE_MS);
    } catch {
        resetDawStats();
        updateDawStats();
    }
}

async function _runDawFilterStatsAgg() {
    try {
        const search = (_lastDawSearch || '').trim();
        const dawSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('dawDawFilter') : null;
        const dawFilter = dawSet ? [...dawSet].join(',') : null;
        const key = search + '|' + (dawFilter || '') + '|' + (_lastDawMode === 'regex' ? 'r' : 'f');
        if (key !== _pendingDawAggKey) {
            return;
        }
        const agg = await window.vstUpdater.dbDawFilterStats(
            search || null,
            dawFilter,
            _lastDawMode === 'regex',
        );
        if (key !== _pendingDawAggKey) {
            return;
        }
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (key !== _pendingDawAggKey) {
            return;
        }
        _lastDawAggKey = key;
        _dawStatsSnapshot = {
            counts: agg.byType || {},
            bytesByType: agg.bytesByType || {},
            totalBytes: agg.totalBytes || 0,
            projectCount: agg.count || 0,
        };
        _dawTotalCount = agg.count || 0;
        _dawTotalUnfiltered = agg.totalUnfiltered || 0;
        updateDawStats();
    } catch {
        resetDawStats();
        updateDawStats();
    }
}

function refreshDawStatsFromMemory() {
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
        ? escapeHtml(_dawFmt('ui.tt.daw_open_in_project', {daw: p.daw}))
        : _dawFmt('ui.tt.daw_open_in_project', {daw: p.daw});
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
    <td class="col-path" title="${escapeHtml(p.path)}">${_lastDawSearch ? highlightMatch(p.directory, _lastDawSearch, _lastDawMode) : escapeHtml(p.directory)}</td>
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
    // Match Samples tab: at 3+ chars backend uses FTS5 MATCH (heavier than LIKE).
    debounceMs: 400,
    resetOffset() {
        _dawOffset = 0;
    },
    fetchFn() {
        _lastDawSearch = this.lastSearch || '';
        _lastDawMode = this.lastMode || 'fuzzy';
        fetchDawPage();
    },
});

function filterDawProjects() {
    applyFilter('filterDawProjects');
}

/** Full list for export when SQLite-backed UI has left `allDawProjects` empty (paginated DB model). */
const _DAW_EXPORT_MAX = 100000;

/**
 * All DAW rows from SQLite for plugin xref index (ignores the search box and DAW filter).
 * `allDawProjects` is only one paginated page in DB mode — xref must walk the full library.
 */
async function fetchAllDawProjectsForXref() {
    if (!window.vstUpdater || typeof window.vstUpdater.dbQueryDaw !== 'function') return [];
    let total = _dawTotalUnfiltered || _dawTotalCount || 0;
    if (total <= 0) {
        try {
            const agg = await window.vstUpdater.dbDawFilterStats('', null, false);
            total = agg.totalUnfiltered || agg.count || 0;
        } catch {
            return [];
        }
    }
    if (total <= 0) return [];
    const n = Math.min(total, _DAW_EXPORT_MAX);
    const result = await window.vstUpdater.dbQueryDaw({
        search: null,
        daw_filter: null,
        search_regex: false,
        sort_key: dawSortKey,
        sort_asc: dawSortAsc,
        offset: 0,
        limit: n,
    });
    return result.projects || [];
}

async function fetchDawProjectsForExport() {
    const search = _lastDawSearch || '';
    const dawSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('dawDawFilter') : null;
    const dawFilter = dawSet ? [...dawSet].join(',') : null;
    let total = Number(_dawTotalCount) || 0;
    if (total <= 0 && window.vstUpdater && typeof window.vstUpdater.dbDawFilterStats === 'function') {
        try {
            const agg = await window.vstUpdater.dbDawFilterStats(search.trim(), dawFilter, _lastDawMode === 'regex');
            total = agg.count || 0;
        } catch { /* stale in-memory totals */ }
    }
    if (total <= 0 && window.vstUpdater && typeof window.vstUpdater.dbQueryDaw === 'function') {
        try {
            const probe = await window.vstUpdater.dbQueryDaw({
                search: search || null,
                daw_filter: dawFilter,
                search_regex: _lastDawMode === 'regex',
                sort_key: dawSortKey,
                sort_asc: dawSortAsc,
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch { /* empty library */ }
    }
    const n = Math.min(total, _DAW_EXPORT_MAX);
    if (n <= 0) return [];
    const result = await window.vstUpdater.dbQueryDaw({
        search: search || null,
        daw_filter: dawFilter,
        search_regex: _lastDawMode === 'regex',
        sort_key: dawSortKey,
        sort_asc: dawSortAsc,
        offset: 0,
        limit: n,
    });
    let projects = result.projects || [];
    if (search && projects.length > 1 && typeof searchScore === 'function') {
        const scored = projects.map((p) => ({p, score: searchScore(search, [p.name], _lastDawMode)}));
        scored.sort((a, b) => b.score - a.score);
        projects = scored.map((x) => x.p);
    }
    return projects;
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
    clearTableQueryLoadingRow('dawQueryLoadingRow', 'dawTable');
    const wrap = document.getElementById('dawTableWrap');
    // No data at all — restore the initial state-message (unfiltered empty scan).
    if (filteredDawProjects.length === 0 && _dawTotalCount === 0 && _dawTotalUnfiltered === 0) {
        if (wrap) {
            const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
            const h2 = esc(catalogFmt('ui.h2.daw_project_index'));
            const p = esc(catalogFmt('ui.p.daw_empty'));
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
    // Page-at-a-time: offset=0 replaces DOM, subsequent pages append. Matches audio.js.
    dawRenderCount = _dawOffset + filteredDawProjects.length;
    if (_dawOffset === 0) {
        tbody.innerHTML = filteredDawProjects.map(buildDawRow).join('');
    } else {
        const loadMore = document.getElementById('dawLoadMore');
        if (loadMore) loadMore.remove();
        tbody.insertAdjacentHTML('beforeend', filteredDawProjects.map(buildDawRow).join(''));
    }

    // Filter active + no matches: inline row with a clear message instead of a blank table body.
    if (_dawOffset === 0 && filteredDawProjects.length === 0 && _dawTotalUnfiltered > 0) {
        const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
        const msg = esc(catalogFmt('ui.daw.no_filter_matches'));
        tbody.insertAdjacentHTML('beforeend',
            `<tr><td colspan="7" style="text-align:center;padding:24px;color:var(--text-dim);"><span style="font-size:24px;display:block;margin-bottom:8px;">&#128269;</span>${msg}</td></tr>`);
        return;
    }
    if (dawRenderCount < _dawTotalCount) {
        appendDawLoadMore(tbody);
    }
}

function appendDawLoadMore(tbody) {
    const line = catalogFmt('ui.js.load_more_hint', {
        shown: dawRenderCount.toLocaleString(),
        total: _dawTotalCount.toLocaleString(),
    });
    tbody.insertAdjacentHTML('beforeend',
        `<tr id="dawLoadMore"><td colspan="7" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreDaw">
      ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
    </td></tr>`);
}

function loadMoreDaw() {
    _dawOffset = dawRenderCount;
    fetchDawPage();
}

// When `unifiedResult` is passed (by scanAll), skip this function's Tauri
// invoke and consume the shared result from a single scan_unified call.
async function scanDawProjects(resume = false, unifiedResult = null, overrideRoots = null) {
    showGlobalProgress();
    const btn = document.getElementById('btnScanDaw');
    const resumeBtn = document.getElementById('btnResumeDaw');
    const stopBtn = document.getElementById('btnStopDaw');
    const progressBar = document.getElementById('dawProgressBar');
    const progressFill = document.getElementById('dawProgressFill');
    const tableWrap = document.getElementById('dawTableWrap');

    const excludePaths = resume ? allDawProjects.map(p => p.path) : null;

    if (typeof btnLoading === 'function') btnLoading(btn, true);
    btn.disabled = true;
    btn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...';
    resumeBtn.style.display = 'none';
    stopBtn.style.display = '';
    progressBar.classList.add('active');
    progressFill.style.width = '0%';

    if (!resume) {
        _dawScanDbView = false;
        allDawProjects = [];
        filteredDawProjects = [];
        resetDawStats();
    }

    let scanStreamDomActive = false;
    let pendingDawClear = !resume;
    let firstDawBatch = true;
    let pendingProjects = [];
    let pendingFound = 0;
    const dawEta = createETA();
    dawEta.start();
    const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);

    function flushPendingProjects() {
        if (pendingProjects.length === 0) return;

        const dawElapsed = dawEta.elapsed();
        btn.innerHTML = `&#8635; ${pendingFound.toLocaleString()} found${dawElapsed ? ' — ' + dawElapsed : ''}`;
        progressFill.style.width = '';
        progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

        const allowDom =
            scanStreamDomActive ||
            (typeof isDawScanTableEmpty === 'function' && isDawScanTableEmpty());
        if (!allowDom) {
            pendingProjects = [];
            return;
        }
        scanStreamDomActive = true;

        if (pendingDawClear && pendingProjects.length > 0) {
            pendingDawClear = false;
            allDawProjects = [];
            filteredDawProjects = [];
            resetDawStats();
            const dawStatsEl = document.getElementById('dawStats');
            if (dawStatsEl) dawStatsEl.style.display = 'none';
        }

        if (firstDawBatch) {
            firstDawBatch = false;
            tableWrap.innerHTML = '';
            initDawTable();
        }

        const toAdd = pendingProjects;
        pendingProjects = [];

        allDawProjects.push(...toAdd);
        if (allDawProjects.length > 100000) allDawProjects.length = 100000;
        accumulateDawStats(toAdd);

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
            if (filteredDawProjects.length > 100000) filteredDawProjects.length = 100000;
            if (!_dawScanDbView) {
                const tbody = document.getElementById('dawTableBody');
                if (tbody && dawRenderCount < 2000) {
                    const loadMore = document.getElementById('dawLoadMore');
                    if (loadMore) loadMore.remove();
                    const toRender = matching.slice(0, 2000 - dawRenderCount);
                    tbody.insertAdjacentHTML('beforeend', toRender.map(buildDawRow).join(''));
                    dawRenderCount += toRender.length;
                }
            }
        }

        updateDawStats();
    }

    const scheduleFlush = createScanFlusher(flushPendingProjects, FLUSH_INTERVAL);

    if (dawScanProgressCleanup) dawScanProgressCleanup();
    dawScanProgressCleanup = window.vstUpdater.onDawScanProgress((data) => {
        if (data.phase === 'status') {
            // status message
        } else if (data.phase === 'scanning') {
            pendingProjects.push(...data.projects);
            pendingFound = data.found;
            window.__dawScanPendingFound = pendingFound;
            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({daw: pendingFound});
            else document.getElementById('dawProjectCount').textContent = pendingFound.toLocaleString();
            scheduleFlush();
        }
    });

    try {
        const dawRoots = (overrideRoots && overrideRoots.length > 0)
            ? overrideRoots
            : (prefs.getItem('dawScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
        const result = unifiedResult
            ? await unifiedResult
            : await window.vstUpdater.scanDawProjects(dawRoots.length ? dawRoots : undefined, excludePaths);
        if (dawScanProgressCleanup) {
            dawScanProgressCleanup();
            dawScanProgressCleanup = null;
        }
        _dawScanDbView = false;
        flushPendingProjects();
        if (pendingDawClear) {
            pendingDawClear = false;
            allDawProjects = [];
            filteredDawProjects = [];
            resetDawStats();
        }
        scanStreamDomActive = false;
        if (result.streamed) {
            // Backend streamed rows into SQLite — JS may not mirror them if DOM stream was skipped.
        } else if (resume) {
            allDawProjects = [...allDawProjects, ...result.projects];
        } else {
            allDawProjects = result.projects;
        }
        if (!result.streamed) {
            try {
                await window.vstUpdater.saveDawScan(allDawProjects, result.roots);
            } catch (e) {
                showToast(toastFmt('toast.failed_save_daw_history', {err: e.message || e}), 4000, 'error');
            }
        }
        _dawOffset = 0;
        await fetchDawPage();
        await rebuildDawFilterStats(true);
        if (result.stopped && (_dawTotalUnfiltered || 0) > 0) {
            resumeBtn.style.display = '';
        }
        if (typeof postScanCompleteToast === 'function') {
            const n = _dawTotalUnfiltered || 0;
            postScanCompleteToast(
                !!result.stopped,
                'toast.post_scan_daw_complete',
                'toast.post_scan_daw_stopped',
                {n: n.toLocaleString()},
            );
        }
    } catch (err) {
        if (dawScanProgressCleanup) {
            dawScanProgressCleanup();
            dawScanProgressCleanup = null;
        }
        _dawScanDbView = false;
        flushPendingProjects();
        if (pendingDawClear) {
            pendingDawClear = false;
            allDawProjects = [];
            filteredDawProjects = [];
            resetDawStats();
        }
        scanStreamDomActive = false;
        const errMsg = err.message || err || 'Unknown error';
        tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
        showToast(toastFmt('toast.daw_scan_failed', {errMsg}), 4000, 'error');
    }

    window.__dawScanPendingFound = 0;
    scanStreamDomActive = false;
    hideGlobalProgress();
    btn.disabled = false;
    if (typeof btnLoading === 'function') btnLoading(btn, false);
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
    window.vstUpdater.openDawFolder(filePath).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}
