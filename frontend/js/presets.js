// ── Presets ──
let allPresets = [];
let filteredPresets = [];
let presetSortKey = 'name';
let presetSortAsc = true;
let presetScanProgressCleanup = null;
let _presetScanDbView = false;
let PRESET_PAGE_SIZE = 200;
let presetRenderCount = 0;
let _presetOffset = 0;
let _presetTotalCount = 0;
let _presetTotalCountCapped = false;
let _presetTotalUnfiltered = 0;
/** Monotonic id so stale `dbQueryPresets` results never overwrite a newer filter. */
let _presetQuerySeq = 0;

function ensurePresetTableShellForQuery() {
    if (document.getElementById('presetTable')) return;
    const tableWrap = document.getElementById('presetTableWrap');
    if (!tableWrap) return;
    tableWrap.innerHTML = `<table class="audio-table" id="presetTable">
    ${presetTableHeadHtml()}
    <tbody id="presetTableBody"></tbody>
  </table>`;
    if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('presetTable'));
    if (typeof initTableColumnReorder === 'function') initTableColumnReorder('presetTable', 'presetColumnOrder');
}

function showPresetQueryLoading(isLoadMore) {
    ensurePresetTableShellForQuery();
    showTableQueryLoadingRow({
        tbodyId: 'presetTableBody',
        rowId: 'presetQueryLoadingRow',
        tableId: 'presetTable',
        colspan: 7,
        append: isLoadMore,
        label: typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…',
    });
}

// Incremental stats for presets — avoids O(N) rebuild on every scan flush.
let _presetStatsTotalBytes = 0;
let _presetStatsFormatCounts = {};

function _presetFmt(key, vars) {
    if (typeof appFmt !== 'function') return key;
    return vars ? appFmt(key, vars) : appFmt(key);
}

function presetTableHeadHtml() {
    const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
    const sel = typeof escapeHtml === 'function' ? escapeHtml(tc('ui.audio.th_select_all')) : tc('ui.audio.th_select_all');
    return `<thead><tr>
          <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${sel}"></th>
          <th data-action="sortPreset" data-key="name" style="width: 25%;">${tc('ui.export.col_name')} <span class="sort-arrow" id="presetSortArrowName">&#9660;</span><span class="col-resize"></span></th>
          <th data-action="sortPreset" data-key="format" class="col-format" style="width: 100px;">${tc('ui.export.col_format')} <span class="sort-arrow" id="presetSortArrowFormat"></span><span class="col-resize"></span></th>
          <th data-action="sortPreset" data-key="directory" style="width: 35%;">${tc('ui.export.col_path')} <span class="sort-arrow" id="presetSortArrowDirectory"></span><span class="col-resize"></span></th>
          <th data-action="sortPreset" data-key="size" class="col-size" style="width: 90px;">${tc('ui.export.col_size')} <span class="sort-arrow" id="presetSortArrowSize"></span><span class="col-resize"></span></th>
          <th data-action="sortPreset" data-key="modified" class="col-date" style="width: 100px;">${tc('ui.export.col_modified')} <span class="sort-arrow" id="presetSortArrowModified"></span><span class="col-resize"></span></th>
          <th class="col-actions" style="width: 50px;"></th>
        </tr></thead>`;
}

async function fetchPresetPage() {
    const search = _lastPresetSearch || '';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('presetFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    const seq = ++_presetQuerySeq;
    const isLoadMore = _presetOffset > 0;
    showPresetQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('presetSearchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        const result = await window.vstUpdater.dbQueryPresets({
            search: search || null,
            format_filter: formatFilter,
            sort_key: presetSortKey,
            sort_asc: presetSortAsc,
            search_regex: _lastPresetMode === 'regex',
            offset: _presetOffset,
            limit: PRESET_PAGE_SIZE,
        });
        if (seq !== _presetQuerySeq) return;
        let presets = result.presets || [];
        // Re-sort by fzf relevance score
        if (search && presets.length > 1) {
            const scored = presets.map(p => ({p, score: searchScore(search, [p.name], _lastPresetMode)}));
            scored.sort((a, b) => b.score - a.score);
            presets = scored.map(x => x.p);
        }
        // Page-at-a-time: filteredPresets only holds the LATEST page, DOM accumulates.
        filteredPresets = presets;
        _presetTotalCount = result.totalCount || 0;
        _presetTotalCountCapped = result.totalCountCapped === true;
        _presetTotalUnfiltered = result.totalUnfiltered || 0;
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _presetQuerySeq) return;
        renderPresetTable();
        if (presetScanProgressCleanup) _presetScanDbView = true;
        if (typeof requestIdleCallback === 'function') {
            requestIdleCallback(() => {
                void rebuildPresetStats();
            });
        } else {
            setTimeout(() => {
                void rebuildPresetStats();
            }, 0);
        }
    } catch (e) {
        if (seq !== _presetQuerySeq) return;
        clearTableQueryLoadingRow('presetQueryLoadingRow', 'presetTable');
        showToast(toastFmt('toast.preset_query_failed', {err: e}), 4000, 'error');
    } finally {
        if (seq === _presetQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('presetSearchInput', false);
    }
}

function formatPresetSize(bytes) {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function buildPresetRow(p) {
    const hp = escapeHtml(p.path);
    const checked =
        typeof batchSetForTabId === 'function' && batchSetForTabId('tabPresets').has(p.path) ? ' checked' : '';
    const rowTt = typeof escapeHtml === 'function'
        ? escapeHtml(_presetFmt('ui.tt.row_double_click_reveal_finder'))
        : _presetFmt('ui.tt.row_double_click_reveal_finder');
    return `<tr data-preset-path="${hp}" data-preset-format="${escapeHtml(p.format)}" data-preset-name="${escapeHtml((p.name || '').toLowerCase())}" style="cursor: pointer;" title="${rowTt}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(p.name)}">${_lastPresetSearch ? highlightMatch(p.name, _lastPresetSearch, _lastPresetMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td class="col-format"><span class="format-badge format-default">${p.format}</span></td>
    <td class="col-path" title="${hp}">${_lastPresetSearch ? highlightMatch(p.directory, _lastPresetSearch, _lastPresetMode) : escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted || formatPresetSize(p.size)}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-folder" data-action="openPresetFolder" data-path="${hp}" title="${hp}">&#128193;</button>
    </td>
  </tr>`;
}

// Maintain incremental stats so flushPending is O(batch) not O(total).
// Accepts a batch of new items; excludes MIDI (own tab).
function accumulatePresetStats(batch) {
    const midiFormats = new Set(['MID', 'MIDI']);
    for (let i = 0; i < batch.length; i++) {
        const p = batch[i];
        if (midiFormats.has(p.format)) continue;
        _presetStatsTotalBytes += p.size || 0;
        _presetStatsFormatCounts[p.format] = (_presetStatsFormatCounts[p.format] || 0) + 1;
    }
}

function resetPresetStatsAccumulators() {
    _presetStatsTotalBytes = 0;
    _presetStatsFormatCounts = {};
}

/** Dynamic format rows are direct children of `#presetStats` (draggable with count/size). */
function clearPresetFormatBreakdownPairs() {
    const ps = document.getElementById('presetStats');
    if (!ps) return;
    ps.querySelectorAll('.audio-stats-pair[data-stat-key^="fmt-"]').forEach((el) => el.remove());
}

function renderPresetFormatBreakdown(entries) {
    clearPresetFormatBreakdownPairs();
    const ps = document.getElementById('presetStats');
    if (!ps) return;
    for (const [fmt, c] of entries) {
        const span = document.createElement('span');
        span.className = 'audio-stats-pair format-badge format-default';
        span.dataset.statKey = 'fmt-' + fmt;
        span.textContent = fmt + ': ' + c.toLocaleString();
        ps.appendChild(span);
    }
    applyPresetStatsOrderFromPrefs();
}

/** Re-apply prefs after format rows are recreated (DOMContentLoaded restore runs before breakdown exists). */
function applyPresetStatsOrderFromPrefs() {
    if (typeof prefs === 'undefined') return;
    const saved = prefs.getObject('presetStatsOrder', null);
    if (!saved || !Array.isArray(saved)) return;
    const ps = document.getElementById('presetStats');
    if (!ps) return;
    const getKey = (el) => el.dataset.statKey || '';
    const children = [...ps.children].filter((c) => c.classList.contains('audio-stats-pair'));
    const map = {};
    children.forEach((c) => {
        const k = getKey(c);
        if (k) map[k] = c;
    });
    for (const key of saved) {
        if (map[key]) ps.appendChild(map[key]);
    }
    children.forEach((c) => {
        const k = getKey(c);
        if (k && !saved.includes(k)) ps.appendChild(c);
    });
}

let _lastPresetAggKey = null;
let _presetAggCache = null;

async function rebuildPresetStats(force) {
    const statsEl = document.getElementById('presetStats');
    if (!statsEl) {
        updatePresetExportButton();
        return;
    }
    const search = document.getElementById('presetSearchInput')?.value || '';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('presetFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    const regexOn = typeof getSearchMode === 'function' && getSearchMode('regexPresets') === 'regex';
    const key = search.trim() + '|' + (formatFilter || '') + '|' + (regexOn ? 'r' : 'f');
    let count = 0, bytes = 0, unfiltered = 0, byType = {};
    {
        const cacheHit = !force && key === _lastPresetAggKey && _presetAggCache;
        try {
            let agg;
            if (cacheHit) {
                agg = _presetAggCache;
            } else {
                agg = await window.vstUpdater.dbPresetFilterStats(search.trim(), formatFilter, regexOn);
                if (typeof yieldToBrowser === 'function') await yieldToBrowser();
                _lastPresetAggKey = key;
                _presetAggCache = agg;
            }
            count = agg.count || 0;
            bytes = agg.totalBytes || 0;
            unfiltered = agg.totalUnfiltered || 0;
            byType = agg.byType || {};
            _presetTotalCount = count;
            _presetTotalCountCapped = agg.countCapped === true;
            _presetTotalUnfiltered = unfiltered;
        } catch {
            // Fallback to incremental accumulator
            if (_presetStatsTotalBytes === 0 && Object.keys(_presetStatsFormatCounts).length === 0 && allPresets.length > 0) {
                accumulatePresetStats(allPresets);
            }
            bytes = _presetStatsTotalBytes;
            byType = _presetStatsFormatCounts;
            count = Object.values(byType).reduce((a, b) => a + b, 0);
            unfiltered = count;
        }
    }
    const isFiltered = unfiltered > 0 && count > 0 && count < unfiltered;
    const displayCount = count || unfiltered;
    statsEl.style.display = (displayCount > 0 || unfiltered > 0) ? 'flex' : 'none';
    const countPart = _presetTotalCountCapped ? count.toLocaleString() + '+' : count.toLocaleString();
    const countStr = isFiltered
        ? countPart + ' / ' + unfiltered.toLocaleString()
        : (_presetTotalCountCapped ? count.toLocaleString() + '+' : (unfiltered || count).toLocaleString());
    document.getElementById('presetCount').textContent = countStr;
    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({presets: unfiltered || count});
    else {
        const headerCount = document.getElementById('presetCountHeader');
        if (headerCount) headerCount.textContent = (unfiltered || count).toLocaleString();
    }
    document.getElementById('presetTotalSize').textContent = formatPresetSize(bytes);
    const entries = Object.entries(byType).sort((a, b) => b[1] - a[1]);
    renderPresetFormatBreakdown(entries);
    updatePresetExportButton();
}

function resetPresetStats() {
    document.getElementById('presetCount').textContent = '0';
    document.getElementById('presetTotalSize').textContent = '0 B';
    clearPresetFormatBreakdownPairs();
    resetPresetStatsAccumulators();
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
    if (typeof saveSortState === 'function') saveSortState('preset', presetSortKey, presetSortAsc);
}

let _lastPresetSearch = '';
let _lastPresetMode = 'fuzzy';

registerFilter('filterPresets', {
    inputId: 'presetSearchInput',
    regexToggleId: 'regexPresets',
    resetOffset() {
        _presetOffset = 0;
    },
    fetchFn() {
        _lastPresetSearch = this.lastSearch || '';
        _lastPresetMode = this.lastMode || 'fuzzy';
        fetchPresetPage();
    },
});

function filterPresets() {
    applyFilter('filterPresets');
}

/** Full list for export when SQLite-backed UI has only paginated rows (or scan-in-progress buffer). */
const _PRESET_EXPORT_MAX = 100000;

async function fetchPresetsForExport() {
    const search = _lastPresetSearch || '';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('presetFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    let total = _presetTotalCount || 0;
    if (total <= 0) {
        try {
            const probe = await window.vstUpdater.dbQueryPresets({
                search: search || null,
                format_filter: formatFilter,
                sort_key: presetSortKey,
                sort_asc: presetSortAsc,
                search_regex: _lastPresetMode === 'regex',
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch {
            return [];
        }
    }
    const n = Math.min(total, _PRESET_EXPORT_MAX);
    if (n <= 0) return [];
    const result = await window.vstUpdater.dbQueryPresets({
        search: search || null,
        format_filter: formatFilter,
        sort_key: presetSortKey,
        sort_asc: presetSortAsc,
        search_regex: _lastPresetMode === 'regex',
        offset: 0,
        limit: n,
    });
    let presets = result.presets || [];
    if (search && presets.length > 1) {
        const scored = presets.map((p) => ({p, score: searchScore(search, [p.name], _lastPresetMode)}));
        scored.sort((a, b) => b.score - a.score);
        presets = scored.map((x) => x.p);
    }
    return presets;
}

function updatePresetExportButton() {
    const btn = document.getElementById('btnExportPresets');
    if (!btn) return;
    const n = Math.max(_presetTotalCount || 0, typeof allPresets !== 'undefined' ? allPresets.length : 0);
    btn.style.display = n > 0 ? '' : 'none';
}

function _legacyFilterPresets() {
    // Kept for scan streaming — not used for user-initiated filter
    if (!allPresets.length) return;
    filteredPresets = allPresets;

    if (false) {
        const key = presetSortKey;
        const dir = presetSortAsc ? 1 : -1;
        filteredPresets.sort((a, b) => {
            if (key === 'size') return (a.size - b.size) * dir;
            const av = (a[key] || '').toLowerCase();
            const bv = (b[key] || '').toLowerCase();
            return av < bv ? -dir : av > bv ? dir : 0;
        });
    }

    presetRenderCount = 0;
    const tbody = document.getElementById('presetTableBody');
    if (!tbody) return;

    const page = filteredPresets.slice(0, PRESET_PAGE_SIZE);
    tbody.innerHTML = page.map(buildPresetRow).join('');
    presetRenderCount = page.length;

    if (filteredPresets.length > PRESET_PAGE_SIZE) {
        const rem = filteredPresets.length - PRESET_PAGE_SIZE;
        const btnLabel = catalogFmt('ui.preset.load_more_btn', {n: rem.toLocaleString()});
        const btnTitle = typeof escapeHtml === 'function'
            ? escapeHtml(_presetFmt('ui.tt.load_next_preset_batch'))
            : _presetFmt('ui.tt.load_next_preset_batch');
        const safeLabel = typeof escapeHtml === 'function' ? escapeHtml(btnLabel) : btnLabel;
        tbody.insertAdjacentHTML('beforeend',
            `<tr><td colspan="7" style="text-align:center; padding: 12px;">
        <button class="btn btn-secondary" data-action="loadMorePresets" title="${btnTitle}">${safeLabel}</button>
      </td></tr>`);
    }

    // (pagination render-count display removed)
    document.getElementById('presetFilteredCount').textContent = '';
}

function renderPresetTable() {
    clearTableQueryLoadingRow('presetQueryLoadingRow', 'presetTable');
    if (!document.getElementById('presetTable')) {
        // Table not initialized yet — will be created by scan flush
        const tableWrap = document.getElementById('presetTableWrap');
        if (tableWrap && filteredPresets.length > 0) {
            tableWrap.innerHTML = `<table class="audio-table" id="presetTable">
        ${presetTableHeadHtml()}
        <tbody id="presetTableBody"></tbody>
      </table>`;
            document.getElementById('presetStats').style.display = 'flex';
            if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('presetTable'));
            if (typeof initTableColumnReorder === 'function') initTableColumnReorder('presetTable', 'presetColumnOrder');
        }
    }
    const tbody = document.getElementById('presetTableBody');
    if (!tbody) return;
    // Page-at-a-time: offset=0 replaces DOM, subsequent pages append. Matches audio.js.
    presetRenderCount = _presetOffset + filteredPresets.length;
    if (_presetOffset === 0) {
        tbody.innerHTML = filteredPresets.map(buildPresetRow).join('');
    } else {
        const loadMore = tbody.querySelector('tr [data-action="loadMorePresets"]')?.closest('tr');
        if (loadMore) loadMore.remove();
        tbody.insertAdjacentHTML('beforeend', filteredPresets.map(buildPresetRow).join(''));
    }
    const presetHasMore = _presetTotalCountCapped
        ? (filteredPresets.length === PRESET_PAGE_SIZE)
        : (presetRenderCount < _presetTotalCount);
    if (presetHasMore) {
        const totalShown = _presetTotalCountCapped ? _presetTotalCount.toLocaleString() + '+' : _presetTotalCount.toLocaleString();
        const line = catalogFmt('ui.js.load_more_hint', {
            shown: presetRenderCount.toLocaleString(),
            total: totalShown,
        });
        tbody.insertAdjacentHTML('beforeend',
            `<tr><td colspan="7" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMorePresets">
        ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
      </td></tr>`);
    }
    // (pagination render-count display removed — presetCount already shows "filtered / total")
    const fc = document.getElementById('presetFilteredCount');
    if (fc) fc.textContent = '';
}

function loadMorePresets() {
    _presetOffset = presetRenderCount;
    fetchPresetPage();
}

function openPresetFolder(path) {
    window.vstUpdater.openPresetFolder(path).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}

// When `unifiedResult` is passed (by scanAll), skip this function's Tauri
// invoke and consume the shared result from a single scan_unified call.
async function scanPresets(resume = false, unifiedResult = null, overrideRoots = null) {
    showGlobalProgress();
    const btn = document.getElementById('btnScanPresets');
    const setBtn = (html, disabled) => {
        btn.innerHTML = html;
        btn.disabled = disabled;
    };
    const resumeBtn = document.getElementById('btnResumePresets');
    const stopBtn = document.getElementById('btnStopPresets');
    const setBtnDisplay = (el, display) => {
        if (el) el.style.display = display;
    };
    const progressBar = document.getElementById('presetProgressBar');
    const progressFill = document.getElementById('presetProgressFill');
    const tableWrap = document.getElementById('presetTableWrap');

    const excludePaths = resume ? allPresets.map(p => p.path) : null;

    if (typeof btnLoading === 'function') btnLoading(btn, true);
    setBtn(resume ? '&#8635; Resuming...' : '&#8635; Scanning...', true);
    setBtnDisplay(resumeBtn, 'none');
    setBtnDisplay(stopBtn, '');
    progressBar.classList.add('active');
    progressFill.style.width = '0%';

    if (!resume) {
        _presetScanDbView = false;
        allPresets = [];
        filteredPresets = [];
        resetPresetStats();
        resetPresetStatsAccumulators();
    }

    let scanPresetDomActive = false;
    let scanMidiFromPresetDomActive = false;
    let firstBatch = true;
    let pendingPresets = [];
    let pendingFound = 0;
    const presetEta = createETA();
    presetEta.start();
    const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);

    function flushPending() {
        if (pendingPresets.length === 0) return;
        const batch = pendingPresets.splice(0);

        const presetElapsed = presetEta.elapsed();
        const timeSuffix = presetElapsed ? ' — ' + presetElapsed : '';
        setBtn(`&#8635; ${pendingFound.toLocaleString()} found${timeSuffix}`, true);

        const allowPresetDom =
            scanPresetDomActive ||
            (typeof isPresetScanTableEmpty === 'function' && isPresetScanTableEmpty());
        const allowMidiDom =
            scanMidiFromPresetDomActive ||
            (typeof isMidiScanTableEmpty === 'function' && isMidiScanTableEmpty());

        if (allowPresetDom) scanPresetDomActive = true;
        if (allowMidiDom) scanMidiFromPresetDomActive = true;

        if (allowPresetDom && firstBatch) {
            firstBatch = false;
            tableWrap.innerHTML = `<table class="audio-table" id="presetTable">
        ${presetTableHeadHtml()}
        <tbody id="presetTableBody"></tbody>
      </table>`;
            document.getElementById('presetStats').style.display = 'flex';
            if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('presetTable'));
            if (typeof initTableColumnReorder === 'function') initTableColumnReorder('presetTable', 'presetColumnOrder');
        }

        const midiFormats = new Set(['MID', 'MIDI']);
        const midiBatch = batch.filter(p => midiFormats.has(p.format));
        const presetBatch = batch.filter(p => !midiFormats.has(p.format));
        allPresets.push(...batch);
        if (allPresets.length > 100000) allPresets.length = 100000;
        if (filteredPresets.length > 100000) filteredPresets.length = 100000;
        filteredPresets.push(...presetBatch);
        accumulatePresetStats(batch);

        if (midiBatch.length > 0 && typeof allMidiFiles !== 'undefined') {
            allMidiFiles.push(...midiBatch);
            if (allMidiFiles.length > 100000) allMidiFiles.length = 100000;
            if (typeof filteredMidi !== 'undefined') filteredMidi.push(...midiBatch);
            if (allowMidiDom) {
                const skipMidiDom = typeof _midiScanDbView !== 'undefined' && _midiScanDbView;
                const midiTbody = document.getElementById('midiTableBody');
                if (!skipMidiDom && midiTbody && typeof buildMidiRow === 'function' && typeof _midiRenderCount !== 'undefined' && _midiRenderCount < 2000) {
                    const toRender = midiBatch.slice(0, 2000 - _midiRenderCount);
                    midiTbody.insertAdjacentHTML('beforeend', toRender.map(buildMidiRow).join(''));
                    _midiRenderCount += toRender.length;
                } else if (!skipMidiDom && !midiTbody && typeof renderMidiTable === 'function') {
                    renderMidiTable();
                }
                if (typeof updateMidiCount === 'function') updateMidiCount();
                if (typeof _midiMetadataRunning !== 'undefined' && !_midiMetadataRunning && typeof loadMidiMetadata === 'function') loadMidiMetadata();
            }
        }

        if (allowPresetDom) {
            const tbody = document.getElementById('presetTableBody');
            if (!_presetScanDbView && tbody && presetRenderCount < 2000) {
                const loadMore = document.getElementById('presetLoadMore');
                if (loadMore) loadMore.remove();
                const scanFmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('presetFormatFilter') : null;
                const scanSearch = (document.getElementById('presetSearchInput')?.value || '').trim().toLowerCase();
                const visibleBatch = (scanFmtSet || scanSearch)
                    ? presetBatch.filter(p => {
                        if (scanFmtSet && !scanFmtSet.has(p.format)) return false;
                        if (scanSearch && !((p.name || '').toLowerCase().includes(scanSearch))) return false;
                        return true;
                    })
                    : presetBatch;
                const toRender = visibleBatch.slice(0, 2000 - presetRenderCount);
                tbody.insertAdjacentHTML('beforeend', toRender.map(buildPresetRow).join(''));
                presetRenderCount += toRender.length;
            }
        }

        rebuildPresetStats();
    }

    const scheduleFlush = createScanFlusher(flushPending, FLUSH_INTERVAL);

    if (presetScanProgressCleanup) presetScanProgressCleanup();
    presetScanProgressCleanup = window.vstUpdater.onPresetScanProgress((data) => {
        if (data.phase === 'status') {
            // status message
        } else if (data.phase === 'scanning') {
            pendingPresets.push(...data.presets);
            pendingFound = data.found;
            window.__presetScanPendingFound = pendingFound;
            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({presets: pendingFound});
            else document.getElementById('presetCountHeader').textContent = pendingFound.toLocaleString();
            scheduleFlush();
        }
    });

    try {
        const presetRoots = (overrideRoots && overrideRoots.length > 0)
            ? overrideRoots
            : (prefs.getItem('presetScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
        const result = unifiedResult
            ? await unifiedResult
            : await window.vstUpdater.scanPresets(presetRoots.length ? presetRoots : undefined, excludePaths);
        // Drain final streamed batch with the scan-active guard still set so the
        // rebuild inside flushPending uses incremental accumulators.
        flushPending();
        if (result.streamed) {
            // Backend streamed results live — allPresets was built from progress events.
        } else if (resume) {
            allPresets = [...allPresets, ...result.presets];
        } else {
            allPresets = result.presets;
        }
        // Refresh header count immediately — don't wait for next fetchPresetPage.
        // Exclude MIDI since they live in their own tab (matches backend `total_unfiltered` definition).
        const midiFormats = new Set(['MID', 'MIDI']);
        _presetTotalUnfiltered = allPresets.filter(p => !midiFormats.has(p.format)).length;
        // Save to the DB BEFORE rebuildPresetStats — otherwise the filter-stats
        // query hits stale/empty rows and the top counter flickers between the
        // previous scan's totals and zero/filtered values.
        if (!result.streamed) {
            try {
                await window.vstUpdater.savePresetScan(allPresets, result.roots);
            } catch (e) {
                showToast(toastFmt('toast.failed_save_preset_history', {err: e.message || e}), 4000, 'error');
            }
        }
        if (presetScanProgressCleanup) {
            presetScanProgressCleanup();
            presetScanProgressCleanup = null;
        }
        _presetScanDbView = false;
        rebuildPresetStats(true);
        filterPresets();
        // MIDI tab has its own independent scanner/DB — don't reload from preset scan.
        if (result.stopped && allPresets.length > 0) {
            setBtnDisplay(resumeBtn, '');
        }
        if (typeof postScanCompleteToast === 'function') {
            const n = _presetTotalUnfiltered || 0;
            postScanCompleteToast(
                !!result.stopped,
                'toast.post_scan_presets_complete',
                'toast.post_scan_presets_stopped',
                {n: n.toLocaleString()},
            );
        }
    } catch (err) {
        if (presetScanProgressCleanup) {
            presetScanProgressCleanup();
            presetScanProgressCleanup = null;
        }
        _presetScanDbView = false;
        flushPending();
        const errMsg = err.message || err || 'Unknown error';
        tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
        showToast(toastFmt('toast.preset_scan_failed', {errMsg}), 4000, 'error');
    }

    window.__presetScanPendingFound = 0;
    scanPresetDomActive = false;
    scanMidiFromPresetDomActive = false;
    hideGlobalProgress();
    btn.disabled = false;
    if (typeof btnLoading === 'function') btnLoading(btn, false);
    btn.innerHTML = '&#127924; Scan Presets';
    setBtnDisplay(stopBtn, 'none');
    updatePresetExportButton();
    progressBar.classList.remove('active');
    progressFill.style.width = '0%';
}

async function stopPresetScan() {
    await window.vstUpdater.stopPresetScan();
}
