// ── Video files tab ── Paginated SQLite queries (same UX model as PDF tab).

function _videoFmt(key, vars) {
    if (typeof appFmt !== 'function') return key;
    return vars ? appFmt(key, vars) : appFmt(key);
}

let allVideos = [];
let filteredVideos = [];
let videoSortKey = 'name';
let videoSortAsc = true;
let videoScanProgressCleanup = null;
/** When true, video scan streaming flush must not touch the DOM (late IPC after invoke, or Stop cleared the UI). */
let _videoScanProgressFlushDisabled = false;
let _videoScanDbView = false;
let videoRenderCount = 0;
let _videoOffset = 0;
let _videoTotalCount = 0;
let _videoTotalCountCapped = false;
let _videoTotalUnfiltered = 0;
let _videoQuerySeq = 0;
let _lastVideoSearch = '';
let _lastVideoMode = 'fuzzy';
let _videoLoaded = false;

function ensureVideoTableForQuery() {
    if (document.getElementById('videoTable')) return;
    const tableWrap = document.getElementById('videoTableWrap');
    if (!tableWrap) return;
    tableWrap.innerHTML = buildVideoTableHtml();
    if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('videoTable'));
    if (typeof initTableColumnReorder === 'function') initTableColumnReorder('videoTable', 'videoColumnOrder');
}

function showVideoQueryLoading(isLoadMore) {
    ensureVideoTableForQuery();
    showTableQueryLoadingRow({
        tbodyId: 'videoTableBody',
        rowId: 'videoQueryLoadingRow',
        tableId: 'videoTable',
        colspan: 7,
        append: isLoadMore,
        label: typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…',
    });
}

async function fetchVideoPage() {
    const search = _lastVideoSearch || '';
    const seq = ++_videoQuerySeq;
    const isLoadMore = _videoOffset > 0;
    showVideoQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('videoSearchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('videoFormatFilter') : null;
        const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
        const result = await window.vstUpdater.dbQueryVideo({
            search: search || null,
            format_filter: formatFilter,
            sort_key: videoSortKey,
            sort_asc: videoSortAsc,
            search_regex: _lastVideoMode === 'regex',
            offset: _videoOffset,
            limit: VIDEO_PAGE_SIZE,
        });
        if (seq !== _videoQuerySeq) return;
        filteredVideos = result.videoFiles || [];
        _videoTotalCount = result.totalCount || 0;
        _videoTotalCountCapped = result.totalCountCapped === true;
        _videoTotalUnfiltered = result.totalUnfiltered || 0;
        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _videoQuerySeq) return;
        renderVideoTable();
        if (videoScanProgressCleanup) _videoScanDbView = true;
        if (typeof requestIdleCallback === 'function') {
            requestIdleCallback(() => {
                void rebuildVideoStats();
            });
        } else {
            setTimeout(() => {
                void rebuildVideoStats();
            }, 0);
        }
    } catch (e) {
        if (seq !== _videoQuerySeq) return;
        clearTableQueryLoadingRow('videoQueryLoadingRow', 'videoTable');
        if (typeof showToast === 'function') {
            showToast(toastFmt('toast.video_query_failed', {err: e && e.message ? e.message : e}), 4000, 'error');
        }
    } finally {
        if (seq === _videoQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('videoSearchInput', false);
    }
}

let _lastVideoAggKey = null;
let _videoAggCache = null;

async function rebuildVideoStats(force) {
    const statsEl = document.getElementById('videoStats');
    if (!statsEl) return;
    const search = document.getElementById('videoSearchInput')?.value || '';
    const regexOn = typeof getSearchMode === 'function' && getSearchMode('regexVideo') === 'regex';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('videoFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    const key = search.trim() + '|' + (regexOn ? 'r' : 'f') + '|' + (formatFilter || '');
    let displayCount = 0;
    let displayBytes = 0;
    let unfiltered = 0;
    let aggResolved = null;
    {
        const cacheHit = !force && key === _lastVideoAggKey && _videoAggCache;
        try {
            let agg;
            if (cacheHit) {
                agg = _videoAggCache;
            } else {
                agg = await window.vstUpdater.dbVideoFilterStats(search.trim(), formatFilter, regexOn);
                if (typeof yieldToBrowser === 'function') await yieldToBrowser();
                _lastVideoAggKey = key;
                _videoAggCache = agg;
            }
            aggResolved = agg;
            displayCount = agg.count || 0;
            displayBytes = agg.totalBytes || 0;
            unfiltered = agg.totalUnfiltered || 0;
            _videoTotalCount = displayCount;
            _videoTotalCountCapped = agg.countCapped === true;
            _videoTotalUnfiltered = unfiltered;
        } catch {
            aggResolved = null;
            displayCount = allVideos.length;
            displayBytes = 0;
            unfiltered = allVideos.length;
        }
    }
    const isFiltered = search.trim() && displayCount < unfiltered;
    statsEl.style.display = (displayCount > 0 || unfiltered > 0) ? 'flex' : 'none';
    const dcPart = _videoTotalCountCapped ? displayCount.toLocaleString() + '+' : displayCount.toLocaleString();
    const countStr = isFiltered
        ? dcPart + ' / ' + unfiltered.toLocaleString()
        : (_videoTotalCountCapped ? displayCount.toLocaleString() + '+' : (unfiltered || displayCount).toLocaleString());
    const countEl = document.getElementById('videoCount');
    if (countEl) countEl.textContent = countStr;
    const sizeEl = document.getElementById('videoTotalSize');
    if (sizeEl) sizeEl.textContent = formatAudioSize(displayBytes);
    const u = unfiltered || displayCount || 0;
    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({video: u});
    else {
        const headerEl = document.getElementById('videoCountHeader');
        if (headerEl) headerEl.textContent = u.toLocaleString();
    }
    const exportBtn = document.getElementById('btnExportVideo');
    if (exportBtn) exportBtn.style.display = (u > 0) ? '' : 'none';
    if (typeof updateVideoDiskUsage === 'function') {
        if (
            aggResolved
            && displayCount > 0
            && aggResolved.countCapped !== true
            && aggResolved.bytesByType
            && Object.keys(aggResolved.bytesByType).length > 0
        ) {
            updateVideoDiskUsage(aggResolved.bytesByType, displayBytes);
        } else {
            updateVideoDiskUsage(null, 0);
        }
    }
}

function buildVideoRow(v) {
    const hp = escapeHtml(v.path);
    const checked =
        typeof batchSetForTabId === 'function' && batchSetForTabId('tabVideos').has(v.path) ? ' checked' : '';
    const rowTt = typeof escapeHtml === 'function'
        ? escapeHtml(_videoFmt('ui.tt.video_row_double_click_open'))
        : _videoFmt('ui.tt.video_row_double_click_open');
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => s;
    const previewBtnT = esc(_videoFmt('ui.audio.row_btn_preview'));
    const loopBtnT = esc(_videoFmt('ui.audio.row_btn_loop'));
    const revealBtnT = esc(_videoFmt('menu.reveal_in_finder'));
    const isPlaying = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === v.path;
    const rowClass = isPlaying ? ' class="row-playing"' : '';
    return `<tr${rowClass} data-video-path="${hp}" data-video-name="${escapeHtml((v.name || '').toLowerCase())}" data-action="toggleVideoMeta" data-path="${hp}" data-video-size="${Number(v.size) || 0}" style="cursor: pointer;" title="${rowTt}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(v.name)}">${_lastVideoSearch ? highlightMatch(v.name, _lastVideoSearch, _lastVideoMode) : escapeHtml(v.name)}${typeof rowBadges === 'function' ? rowBadges(v.path) : ''}</td>
    <td class="col-path" title="${hp}">${_lastVideoSearch ? highlightMatch(v.directory, _lastVideoSearch, _lastVideoMode) : escapeHtml(v.directory)}</td>
    <td class="col-format">${escapeHtml(v.format || '')}</td>
    <td class="col-size">${v.sizeFormatted}</td>
    <td class="col-date">${v.modified}</td>
    <td class="col-actions" data-action-stop>
      <span class="table-row-actions">
      <button type="button" class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewVideo" data-path="${hp}" title="${previewBtnT}">
        ${isPlaying && typeof isAudioPlaying === 'function' && isAudioPlaying() ? '&#9646;&#9646;' : '&#9654;'}
      </button>
      <button type="button" class="btn-small btn-loop${isPlaying && typeof audioLooping !== 'undefined' && audioLooping ? ' active' : ''}" data-action="toggleVideoRowLoop" data-path="${hp}" title="${loopBtnT}">&#8634;</button>
      <button type="button" class="btn-small btn-folder" data-action="openVideoFile" data-path="${hp}" title="${revealBtnT}">&#128193;</button>
      </span>
    </td>
  </tr>`;
}

function renderVideoTable() {
    clearTableQueryLoadingRow('videoQueryLoadingRow', 'videoTable');
    if (!document.getElementById('videoTable')) {
        const tableWrap = document.getElementById('videoTableWrap');
        if (tableWrap && filteredVideos.length > 0) {
            tableWrap.innerHTML = buildVideoTableHtml();
            const st = document.getElementById('videoStats');
            if (st) st.style.display = 'flex';
            if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('videoTable'));
            if (typeof initTableColumnReorder === 'function') initTableColumnReorder('videoTable', 'videoColumnOrder');
        }
    }
    const tbody = document.getElementById('videoTableBody');
    if (!tbody) return;
    videoRenderCount = _videoOffset + filteredVideos.length;
    if (_videoOffset === 0) {
        closeVideoMetaRow();
        tbody.innerHTML = filteredVideos.map(buildVideoRow).join('');
    } else {
        const loadMoreRow = tbody.querySelector('tr [data-action="loadMoreVideos"]')?.closest('tr');
        if (loadMoreRow) loadMoreRow.remove();
        tbody.insertAdjacentHTML('beforeend', filteredVideos.map(buildVideoRow).join(''));
    }
    const videoHasMore = _videoTotalCountCapped
        ? (filteredVideos.length === VIDEO_PAGE_SIZE)
        : (videoRenderCount < _videoTotalCount);
    if (videoHasMore) {
        const totalShown = _videoTotalCountCapped ? _videoTotalCount.toLocaleString() + '+' : _videoTotalCount.toLocaleString();
        const line = catalogFmt('ui.js.load_more_hint', {
            shown: videoRenderCount.toLocaleString(),
            total: totalShown,
        });
        tbody.insertAdjacentHTML('beforeend',
            `<tr><td colspan="7" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMoreVideos">
        ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
      </td></tr>`);
    }
}

function isVideoScanTableEmpty() {
    const tbody = document.getElementById('videoTableBody');
    if (!tbody) return true;
    return tbody.querySelector('tr[data-video-path]') == null;
}

function buildVideoTableHtml() {
    const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
    const sel = typeof escapeHtml === 'function' ? escapeHtml(tc('ui.audio.th_select_all')) : tc('ui.audio.th_select_all');
    return `<table class="audio-table" id="videoTable">
    <thead><tr>
      <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${sel}"></th>
      <th data-action="sortVideo" data-key="name" style="width: 24%;">${tc('ui.export.col_name')} <span class="sort-arrow" id="videoSortArrowName">&#9660;</span><span class="col-resize"></span></th>
      <th data-action="sortVideo" data-key="directory" style="width: 32%;">${tc('ui.export.col_path')} <span class="sort-arrow" id="videoSortArrowDirectory"></span><span class="col-resize"></span></th>
      <th data-action="sortVideo" data-key="format" style="width: 70px;">${tc('ui.export.col_format')} <span class="sort-arrow" id="videoSortArrowFormat"></span><span class="col-resize"></span></th>
      <th data-action="sortVideo" data-key="size" class="col-size" style="width: 90px;">${tc('ui.export.col_size')} <span class="sort-arrow" id="videoSortArrowSize"></span><span class="col-resize"></span></th>
      <th data-action="sortVideo" data-key="modified" class="col-date" style="width: 100px;">${tc('ui.export.col_modified')} <span class="sort-arrow" id="videoSortArrowModified"></span><span class="col-resize"></span></th>
      <th class="col-actions" style="width: 152px;"></th>
    </tr></thead>
    <tbody id="videoTableBody"></tbody>
  </table>`;
}

function loadMoreVideos() {
    _videoOffset = videoRenderCount;
    fetchVideoPage();
}

function sortVideo(key, forceAsc) {
    if (typeof forceAsc === 'boolean') {
        videoSortKey = key;
        videoSortAsc = forceAsc;
    } else if (videoSortKey === key) {
        videoSortAsc = !videoSortAsc;
    } else {
        videoSortKey = key;
        videoSortAsc = true;
    }
    ['Name', 'Directory', 'Format', 'Size', 'Modified'].forEach((k) => {
        const el = document.getElementById('videoSortArrow' + k);
        if (el) {
            const isActive = k.toLowerCase() === videoSortKey;
            el.innerHTML = isActive ? (videoSortAsc ? '&#9650;' : '&#9660;') : '';
        }
    });
    filterVideos();
    if (typeof saveSortState === 'function') saveSortState('video', videoSortKey, videoSortAsc);
}

registerFilter('filterVideos', {
    inputId: 'videoSearchInput',
    regexToggleId: 'regexVideo',
    resetOffset() {
        _videoOffset = 0;
    },
    fetchFn() {
        _lastVideoSearch = this.lastSearch || '';
        _lastVideoMode = this.lastMode || 'fuzzy';
        fetchVideoPage();
    },
});

function filterVideos() {
    applyFilter('filterVideos');
}

async function loadVideoFiles() {
    _videoLoaded = true;
    _videoOffset = 0;
    videoRenderCount = 0;
    await fetchVideoPage();
    if (typeof requestIdleCallback === 'function') {
        requestIdleCallback(() => {
            void rebuildVideoStats(true);
        });
    } else {
        setTimeout(() => {
            void rebuildVideoStats(true);
        }, 0);
    }
}

function clearVideoScanButtonSpinnerImmediate() {
    _videoScanProgressFlushDisabled = true;
    const btn = document.getElementById('btnScanVideos');
    const stopBtn = document.getElementById('btnStopVideo');
    const progressBar = document.getElementById('videoProgressBar');
    const progressFill = document.getElementById('videoProgressFill');
    if (typeof btnLoading === 'function') btnLoading(btn, false);
    if (btn) {
        btn.disabled = false;
        btn.innerHTML = '&#127909; <span data-i18n="ui.btn.scan_videos">' + catalogFmt('ui.btn.scan_videos') + '</span>';
    }
    if (stopBtn) stopBtn.style.display = 'none';
    if (progressBar) progressBar.classList.remove('active');
    if (progressFill) {
        progressFill.style.width = '0%';
        progressFill.style.animation = '';
    }
    if (typeof updateHeaderInfo === 'function') void updateHeaderInfo();
}

async function stopVideoScan() {
    clearVideoScanButtonSpinnerImmediate();
    try {
        await window.vstUpdater.stopVideoScan();
    } catch {
        /* ignore */
    }
}

async function scanVideos(resume = false, overrideRoots = null) {
    if (typeof showGlobalProgress === 'function') showGlobalProgress();
    _videoScanProgressFlushDisabled = false;
    const btn = document.getElementById('btnScanVideos');
    const resumeBtn = document.getElementById('btnResumeVideo');
    const stopBtn = document.getElementById('btnStopVideo');
    const progressBar = document.getElementById('videoProgressBar');
    const progressFill = document.getElementById('videoProgressFill');
    const tableWrap = document.getElementById('videoTableWrap');

    if (typeof btnLoading === 'function') btnLoading(btn, true);
    if (btn) {
        btn.innerHTML = '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn');
        btn.disabled = true;
    }
    if (resumeBtn) resumeBtn.style.display = 'none';
    if (stopBtn) stopBtn.style.display = '';
    if (progressBar) progressBar.classList.add('active');
    if (progressFill) {
        progressFill.style.width = '';
        progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
    }

    const excludePaths = resume ? allVideos.map((v) => v.path) : null;

    if (!resume) {
        _videoScanDbView = false;
        allVideos = [];
        filteredVideos = [];
        _videoOffset = 0;
        videoRenderCount = 0;
        closeVideoMetaRow();
        if (tableWrap) {
            tableWrap.innerHTML = `<div class="state-message" id="videoEmptyState">
            <div class="state-icon">&#127909;</div>
            <h2 data-i18n="ui.h2.video_index">${catalogFmt('ui.h2.video_index')}</h2>
            <p data-i18n="ui.p.videos_scanning">${catalogFmt('ui.p.videos_scanning')}</p>
        </div>`;
        }
    }

    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

    let pendingVideos = [];
    let pendingFound = 0;
    let scanVideoDomActive = false;
    let firstVideoBatch = true;
    const videoEta = typeof createETA === 'function' ? createETA() : null;
    if (videoEta) videoEta.start();
    const FLUSH_INTERVAL = parseInt((typeof prefs !== 'undefined' ? prefs.getItem('flushInterval') : null) || '100', 10);

    function filterVideoScanBatch(toAdd) {
        const search = (document.getElementById('videoSearchInput')?.value || '').trim();
        const scanFmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('videoFormatFilter') : null;
        const scanMode = typeof getSearchMode === 'function' ? getSearchMode('regexVideo') : 'fuzzy';
        return toAdd.filter((v) => {
            if (scanFmtSet && scanFmtSet.size > 0 && !scanFmtSet.has(v.format)) return false;
            if (search && typeof searchMatch === 'function' && !searchMatch(search, [v.name, v.directory || ''], scanMode)) return false;
            return true;
        });
    }

    function flushPendingVideo() {
        if (_videoScanProgressFlushDisabled) {
            pendingVideos.length = 0;
            return;
        }
        if (pendingVideos.length === 0) return;
        const toAdd = pendingVideos;
        pendingVideos = [];

        const videoElapsed = videoEta ? videoEta.elapsed() : '';
        if (btn) {
            btn.innerHTML = catalogFmt('ui.audio.scan_progress_line', {
                n: pendingFound.toLocaleString(),
                elapsed: videoElapsed ? ' — ' + videoElapsed : '',
            });
        }
        if (progressFill) {
            progressFill.style.width = '';
            progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
        }

        const allowDom =
            scanVideoDomActive ||
            (typeof isVideoScanTableEmpty === 'function' && isVideoScanTableEmpty());
        if (!allowDom) {
            allVideos.push(...toAdd);
            if (allVideos.length > 100000) allVideos.length = 100000;
            const matching = filterVideoScanBatch(toAdd);
            filteredVideos.push(...matching);
            if (filteredVideos.length > 100000) filteredVideos.length = 100000;
            return;
        }
        scanVideoDomActive = true;

        if (firstVideoBatch) {
            firstVideoBatch = false;
            if (resume) videoRenderCount = 0;
        }

        if (!document.getElementById('videoTableBody') && tableWrap) {
            tableWrap.innerHTML = buildVideoTableHtml();
            const st = document.getElementById('videoStats');
            if (st) st.style.display = 'flex';
            if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('videoTable'));
            if (typeof initTableColumnReorder === 'function') initTableColumnReorder('videoTable', 'videoColumnOrder');
        }

        allVideos.push(...toAdd);
        if (allVideos.length > 100000) allVideos.length = 100000;
        const matching = filterVideoScanBatch(toAdd);
        filteredVideos.push(...matching);
        if (filteredVideos.length > 100000) filteredVideos.length = 100000;

        if (!_videoScanDbView) {
            const tbody = document.getElementById('videoTableBody');
            if (tbody && videoRenderCount < 2000) {
                const loadMoreHint = tbody.querySelector('tr [data-action="loadMoreVideos"]')?.closest('tr');
                if (loadMoreHint) loadMoreHint.remove();
                const toRender = matching.slice(0, 2000 - videoRenderCount);
                tbody.insertAdjacentHTML('beforeend', toRender.map(buildVideoRow).join(''));
                videoRenderCount += toRender.length;
                if (typeof reorderNewTableRows === 'function') reorderNewTableRows('videoTable');
            }
        }
    }

    const scheduleVideoFlush = typeof createScanFlusher === 'function'
        ? createScanFlusher(flushPendingVideo, FLUSH_INTERVAL)
        : () => flushPendingVideo();

    if (videoScanProgressCleanup) {
        if (typeof videoScanProgressCleanup === 'function') videoScanProgressCleanup();
        videoScanProgressCleanup = null;
    }
    videoScanProgressCleanup = await window.vstUpdater.onVideoScanProgress((payload) => {
        if (!payload) return;
        if (payload.phase === 'scanning' && Array.isArray(payload.videoFiles)) {
            pendingVideos.push(...payload.videoFiles);
            pendingFound = payload.found != null ? payload.found : pendingFound;
            if (typeof window !== 'undefined') window.__videoScanPendingFound = pendingFound;
            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({video: pendingFound});
            scheduleVideoFlush();
        }
    });

    let videoScanStopped = false;
    try {
        const roots = (overrideRoots && overrideRoots.length > 0)
            ? overrideRoots
            : (typeof prefs !== 'undefined' ? (prefs.getItem('videoScanDirs') || '') : '').split('\n').map(s => s.trim()).filter(Boolean);
        const scanResult = await window.vstUpdater.scanVideoFiles(roots.length ? roots : undefined, excludePaths);
        videoScanStopped = !!(scanResult && scanResult.stopped);
        if (typeof videoScanProgressCleanup === 'function') {
            videoScanProgressCleanup();
            videoScanProgressCleanup = null;
        }
        flushPendingVideo();
        _videoScanProgressFlushDisabled = true;
        scanVideoDomActive = false;
    } catch (e) {
        if (typeof videoScanProgressCleanup === 'function') {
            videoScanProgressCleanup();
            videoScanProgressCleanup = null;
        }
        scanVideoDomActive = false;
        flushPendingVideo();
        _videoScanProgressFlushDisabled = true;
        if (typeof showToast === 'function') showToast(String(e && e.message ? e.message : e), 5000, 'error');
    }

    clearVideoScanButtonSpinnerImmediate();
    if (resumeBtn) {
        resumeBtn.style.display = videoScanStopped && allVideos.length > 0 ? '' : 'none';
    }
    if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
    if (typeof window !== 'undefined') delete window.__videoScanPendingFound;
    if (typeof scheduleRefreshInventoryFromDb === 'function') scheduleRefreshInventoryFromDb();
    _videoScanDbView = true;
    _videoOffset = 0;
    videoRenderCount = 0;
    await fetchVideoPage();
    await rebuildVideoStats(true);
    const empty = document.getElementById('videoEmptyState');
    if (empty) empty.remove();
}

function openVideoFile(path) {
    /** Same as PDF row folder control: reveal in Finder / Explorer (`open_plugin_folder`), not default-app open. */
    const inv =
        typeof window !== 'undefined' && window.vstUpdater && typeof window.vstUpdater.openPluginFolder === 'function'
            ? window.vstUpdater.openPluginFolder(path)
            : Promise.reject(new Error('openPluginFolder unavailable'));
    inv
        .then(() => {
            if (typeof showToast === 'function') showToast(toastFmt('toast.revealed_in_finder'));
        })
        .catch((e) => {
            if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: e}), 4000, 'error');
        });
}

// ── Video player expanded row ──

let expandedVideoPath = null;
let videoPlayerPath = null;
let _videoRafId = null;
/** DOM cache for `_videoRafLoop` (~60 Hz) — invalidate when `videoPlayerPath` changes. */
let _videoRafDomPath = null;
let _videoRafCachedVid = null;
let _videoRafCachedWfBox = null;
let _videoRafCachedTimeDisp = null;
let _videoRafCachedFsSeek = null;
let _videoEngineActive = false;
let _videoFallbackAudio = false;
let _videoWfDrawSeq = 0;
/** True while user drags `#videoFsSeek` — RAF must not overwrite the thumb. */
let _videoFsSeekUserDrag = false;
/** Last value written to `#videoFsSeek` (0…1000) — skip redundant DOM writes each rAF tick. */
let _videoFsSeekLastWritten = null;
/** Bumps on each new `previewVideo` load and on `stopVideoPlayback` — stale `loadeddata` handlers must not hide the spinner for the wrong file. */
let _videoPlayerLoadUiSeq = 0;
/** Skip `<video>` frame-sampled waveform fallback above this — many seeks stress demux on huge files. Host `waveform_preview` still runs (ffmpeg/Symphonia extract ≤300s, then JUCE). */
const VIDEO_VISUAL_WAVEFORM_MAX_FILE_BYTES = 96 * 1024 * 1024;
let _lastVideoVisualSeekMs = 0;

/**
 * Pref `videoAudioRoute`: `engine` — AudioEngine decode + inserts (default); `html5` — WebView `<video>`
 * audio (lower RAM on huge containers, no VST/AU chain).
 */
function videoPlaybackUsesEngineAudio() {
    if (typeof prefs === 'undefined' || typeof prefs.getItem !== 'function') return true;
    return prefs.getItem('videoAudioRoute') !== 'html5';
}

function _videoFileSrc(path) {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    if (tauri?.core?.convertFileSrc) return tauri.core.convertFileSrc(path);
    if (typeof window !== 'undefined' && typeof window.convertFileSrc === 'function') return window.convertFileSrc(path);
    return path;
}

function _showVideoPlayerLoading() {
    const el = document.getElementById('videoPlayerLoading');
    if (el) {
        el.classList.remove('video-player-loading--hidden');
        el.setAttribute('aria-busy', 'true');
    }
}

function _hideVideoPlayerLoading() {
    const el = document.getElementById('videoPlayerLoading');
    if (el) {
        el.classList.add('video-player-loading--hidden');
        el.setAttribute('aria-busy', 'false');
    }
}

function _wireVideoPlayerLoadingListeners(vid, filePath, loadUiSeq) {
    if (!vid || !filePath || typeof loadUiSeq !== 'number') return;
    _showVideoPlayerLoading();
    const hide = () => {
        if (loadUiSeq !== _videoPlayerLoadUiSeq) return;
        if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath !== filePath) return;
        _hideVideoPlayerLoading();
    };
    vid.addEventListener('loadeddata', hide, { once: true });
    vid.addEventListener('canplay', hide, { once: true });
    vid.addEventListener('error', hide, { once: true });
    const trySync = () => {
        if (loadUiSeq !== _videoPlayerLoadUiSeq) return;
        if (vid.readyState >= 2) hide();
    };
    if (typeof requestAnimationFrame === 'function') requestAnimationFrame(trySync);
    else setTimeout(trySync, 0);
}

function _videoFullscreenHostEl() {
    return document.getElementById('videoFullscreenRoot');
}

function _videoExitFullscreenIfActive() {
    const host = _videoFullscreenHostEl();
    const vid = document.getElementById('videoPlayerEl');
    const doc = document;
    const el = doc.fullscreenElement ?? doc.webkitFullscreenElement;
    if (host && el === host) {
        if (typeof doc.exitFullscreen === 'function') void doc.exitFullscreen();
        else if (typeof doc.webkitExitFullscreen === 'function') void doc.webkitExitFullscreen();
        return;
    }
    /* Legacy: older builds fullscreened `<video>` only. */
    if (vid && el === vid) {
        if (typeof doc.exitFullscreen === 'function') void doc.exitFullscreen();
        else if (typeof doc.webkitExitFullscreen === 'function') void doc.webkitExitFullscreen();
    }
}

function _syncVideoFsControlsFromNp() {
    const np = document.getElementById('npVolume');
    const fs = document.getElementById('videoFsVolume');
    const fsp = document.getElementById('videoFsVolumePct');
    if (np && fs) fs.value = np.value;
    if (np && fsp) fsp.textContent = (np.value || '100') + '%';
}

function _syncVideoMaximizeBtnState() {
    const btn = document.getElementById('btnVideoMaximize');
    const host = _videoFullscreenHostEl();
    const vid = document.getElementById('videoPlayerEl');
    if (!btn || (!host && !vid)) return;
    const doc = document;
    const fsEl = doc.fullscreenElement ?? doc.webkitFullscreenElement;
    const fs = (host && fsEl === host) || (!!vid && fsEl === vid);
    const maxT = catalogFmt('ui.tt.video_maximize');
    const restT = catalogFmt('ui.tt.video_restore');
    btn.title = fs ? restT : maxT;
    if (fs && host && fsEl === host) _syncVideoFsControlsFromNp();
}

function toggleVideoMaximize() {
    const host = _videoFullscreenHostEl();
    const vid = document.getElementById('videoPlayerEl');
    if (!host || !vid) return;
    const doc = document;
    const fsEl = doc.fullscreenElement ?? doc.webkitFullscreenElement;
    if (fsEl === host || fsEl === vid) {
        if (typeof doc.exitFullscreen === 'function') void doc.exitFullscreen();
        else if (typeof doc.webkitExitFullscreen === 'function') void doc.webkitExitFullscreen();
        return;
    }
    const fallbackVidFs = () => {
        if (typeof vid.requestFullscreen === 'function') {
            return vid.requestFullscreen();
        }
        if (typeof vid.webkitRequestFullscreen === 'function') {
            vid.webkitRequestFullscreen();
            return Promise.resolve();
        }
        if (typeof vid.webkitEnterFullscreen === 'function') {
            vid.webkitEnterFullscreen();
        }
        return Promise.resolve();
    };
    if (typeof host.requestFullscreen === 'function') {
        void host.requestFullscreen().catch(() => fallbackVidFs());
        return;
    }
    if (typeof host.webkitRequestFullscreen === 'function') {
        try {
            host.webkitRequestFullscreen();
        } catch {
            void fallbackVidFs();
        }
        return;
    }
    void fallbackVidFs();
}

/** Fullscreen scrubber (`#videoFsSeek` 0…1000). */
function onVideoFsSeekInput(raw) {
    const v = parseInt(String(raw), 10);
    if (!Number.isFinite(v)) return;
    seekVideoToPercent(Math.max(0, Math.min(1, v / 1000)));
}

function closeVideoMetaRow() {
    _videoExitFullscreenIfActive();
    stopVideoPlayback();
    const meta = document.getElementById('videoMetaRow');
    if (meta) meta.remove();
    const expanded = document.querySelector('#videoTableBody tr.row-expanded');
    if (expanded) expanded.classList.remove('row-expanded');
    expandedVideoPath = null;
}

/**
 * Pause video, stop engine / RAF, and clear `videoPlayerPath`. By default clears `<video src>` (black frame).
 * @param {{ keepVideoFrame?: boolean }} [opts] — When true, leave `src` intact so the last decoded frame
 *   stays visible while another track (e.g. library MP3 via AudioEngine) takes the output device.
 */
function stopVideoPlayback(opts) {
    const keepVideoFrame = opts && opts.keepVideoFrame === true;
    _videoPlayerLoadUiSeq++;
    _videoExitFullscreenIfActive();
    if (_videoRafId) {
        cancelAnimationFrame(_videoRafId);
        _videoRafId = null;
    }
    _videoFsSeekLastWritten = null;
    const vid = document.getElementById('videoPlayerEl');
    if (vid) {
        try {
            vid.pause();
        } catch (_) {}
        if (!keepVideoFrame) {
            vid.removeAttribute('src');
            try {
                vid.load();
            } catch (_) {}
        }
    }
    if (typeof connectMediaToEq === 'function') connectMediaToEq();
    _hideVideoPlayerLoading();
    if (_videoEngineActive) {
        _videoEngineActive = false;
        if (typeof window.enginePlaybackStop === 'function') {
            void window.enginePlaybackStop();
        }
        if (typeof window.setEnginePlaybackActive === 'function') {
            window.setEnginePlaybackActive(false);
        }
    }
    _videoFallbackAudio = false;
    videoPlayerPath = null;
    // Clear now-playing bar if it was showing this video
    if (typeof audioPlayerPath !== 'undefined') {
        // Only clear if audioPlayerPath was set by us (video)
        const npName = document.getElementById('npName');
        if (npName && npName.dataset.videoSource === 'true') {
            audioPlayerPath = null;
            npName.dataset.videoSource = '';
            delete npName.dataset.videoSizeBytes;
            if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
            if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
        }
    }
}

/** Show video in the floating now-playing bar + kick the audio RAF loop for tray updates. */
function _showVideoInNowPlaying(filePath, opts) {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || filePath;
    const o = opts && typeof opts === 'object' ? opts : {};

    const np = document.getElementById('audioNowPlaying');
    // Respect "player hidden" state: if the bar is not active but a track was loaded,
    // the user explicitly hid it — don't force it open.
    const wasHiddenByUser = !!(np && !np.classList.contains('active') && typeof audioPlayerPath !== 'undefined' && audioPlayerPath != null);
    const playerPaneHiddenByPref =
        typeof prefs !== 'undefined' &&
        typeof prefs.getItem === 'function' &&
        prefs.getItem('playerPaneHidden') === 'on';

    // Set audioPlayerPath so tray + now-playing bar recognize active playback
    if (typeof audioPlayerPath !== 'undefined') {
        audioPlayerPath = filePath;
    }
    if (typeof window !== 'undefined') {
        window._enginePlaybackResumePath = filePath;
    }

    if (np) {
        const stayHidden =
            (o.minimizeFloatingPlayer === true && !np.classList.contains('active')) ||
            playerPaneHiddenByPref ||
            wasHiddenByUser;
        if (stayHidden) {
            np.classList.remove('active');
            const pill = document.getElementById('audioRestorePill');
            if (pill) pill.classList.add('active');
        } else {
            const pill = document.getElementById('audioRestorePill');
            if (pill) pill.classList.remove('active');
            np.classList.add('active');
            if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function' && prefs.getItem('playerExpanded') === 'on') {
                np.classList.add('expanded');
                if (typeof renderRecentlyPlayed === 'function') renderRecentlyPlayed();
            }
        }
    }

    const npName = document.getElementById('npName');
    if (npName) {
        npName.textContent = '🎬 ' + fileName;
        npName.dataset.videoSource = 'true';
        const vtbody = document.getElementById('videoTableBody');
        let szAttr = '';
        if (vtbody) {
            const tr = vtbody.querySelector(`tr[data-video-path="${CSS.escape(filePath)}"]`);
            if (tr && tr.dataset.videoSize != null && tr.dataset.videoSize !== '') {
                szAttr = String(tr.dataset.videoSize);
            }
        }
        if (szAttr !== '') npName.dataset.videoSizeBytes = szAttr;
        else delete npName.dataset.videoSizeBytes;
    }

    if (typeof updatePlayBtnStates === 'function') updatePlayBtnStates();
    if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();

    // Draw waveform in the now-playing bar
    if (typeof scheduleNowPlayingWaveform === 'function') scheduleNowPlayingWaveform(filePath);

    // Kick the audio RAF loop so it runs updatePlaybackTime + syncTrayNowPlayingFromPlayback
    if (typeof window.kickPlaybackRafLoop === 'function') window.kickPlaybackRafLoop();

    // Apply loop region braces
    if (typeof applyMetaLoopRegionUI === 'function') applyMetaLoopRegionUI(filePath);
    if (typeof syncAbLoopFromSampleRegion === 'function') syncAbLoopFromSampleRegion(filePath);
    if (typeof refreshNpLoopRegionUI === 'function') refreshNpLoopRegionUI();
    /* Must run after `audioPlayerPath` is set above — `syncVideoPlayerElPlaybackRate` requires
     * `audioPlayerPath === videoPlayerPath` (HTML5 container audio uses `<video>`, not `#audioPlayer`). */
    if (typeof window.syncVideoPlayerElPlaybackRate === 'function') window.syncVideoPlayerElPlaybackRate();
}

function toggleVideoLoopRegionFn() {
    const box = document.getElementById('videoWaveformBox');
    if (!box) return;
    const filePath = box.dataset.path || '';
    if (!filePath) return;
    if (typeof getSampleLoopRegion !== 'function' || typeof setSampleLoopRegion !== 'function') return;
    const region = getSampleLoopRegion(filePath);
    region.enabled = !region.enabled;
    setSampleLoopRegion(filePath, region);
    if (typeof applyMetaLoopRegionUI === 'function') applyMetaLoopRegionUI(filePath);
    if (typeof syncAbLoopFromSampleRegion === 'function') syncAbLoopFromSampleRegion(filePath);
}

function toggleVideoMeta(filePath, event) {
    if (event && event.target.closest('.col-actions')) return;

    if (expandedVideoPath === filePath) {
        closeVideoMetaRow();
        return;
    }

    void expandVideoMetaForPath(filePath);
}

function expandVideoMetaForPath(filePath) {
    const tbody = document.getElementById('videoTableBody');
    if (!tbody) return;

    // Close any existing video meta row
    const existing = document.getElementById('videoMetaRow');
    if (existing) {
        stopVideoPlayback();
        existing.remove();
        const prev = tbody.querySelector('tr.row-expanded');
        if (prev) prev.classList.remove('row-expanded');
    }

    // Also close audio expanded row if open to avoid two engine streams
    if (typeof closeMetaRow === 'function') closeMetaRow();

    expandedVideoPath = filePath;

    const row = tbody.querySelector(`tr[data-video-path="${CSS.escape(filePath)}"]`);
    if (!row) return;
    row.classList.add('row-expanded');

    const metaRow = document.createElement('tr');
    metaRow.id = 'videoMetaRow';
    metaRow.className = 'video-meta-row';
    metaRow.setAttribute('data-meta-path', filePath);
    metaRow.innerHTML = `<td colspan="7"><div class="video-meta-panel" style="justify-items:center;"><div class="spinner" style="width:18px;height:18px;"></div></div></td>`;
    row.after(metaRow);
    row.scrollIntoView({ behavior: 'smooth', block: 'nearest' });

    // Build the panel HTML
    const hp = typeof escapeHtml === 'function' ? escapeHtml(filePath) : filePath;
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || filePath;

    const maxBtnLabel = catalogFmt('ui.video.btn_maximize');
    const maxBtnTt = catalogFmt('ui.tt.video_maximize');
    metaRow.innerHTML = `<td colspan="7"><div class="video-meta-panel">
      <div class="video-meta-panel-actions">
        <button type="button" class="video-maximize-btn" id="btnVideoMaximize" data-action="toggleVideoMaximize" data-i18n="ui.video.btn_maximize" data-i18n-title="ui.tt.video_maximize" title="${typeof escapeHtml === 'function' ? escapeHtml(maxBtnTt) : maxBtnTt}">${typeof escapeHtml === 'function' ? escapeHtml(maxBtnLabel) : maxBtnLabel}</button>
        <span class="meta-close-btn" data-action="closeVideoMetaRow" title="Close">&#10005;</span>
      </div>
      <div class="video-player-wrap">
        <div id="videoPlayerLoading" class="video-player-loading" aria-busy="true" role="status">
          <div class="video-player-loading-inner">
            <div class="spinner" style="width:28px;height:28px;"></div>
            <span class="video-player-loading-text" data-i18n="ui.js.query_loading">Loading…</span>
          </div>
        </div>
        <div id="videoFullscreenRoot" class="video-fullscreen-root">
          <div class="video-fs-stage">
            <video id="videoPlayerEl" playsinline></video>
          </div>
          <div class="video-fs-overlay" aria-label="Fullscreen video controls">
            <input type="range" id="videoFsSeek" class="video-fs-seek" data-action="videoFsSeek" min="0" max="1000" value="0" step="1"
              data-i18n-title="ui.audio.meta_waveform_seek_title" title="Seek playback position" />
            <span class="video-fs-vol-wrap">
              <input type="range" id="videoFsVolume" class="video-fs-vol-slider" data-action="setVolume" min="0" max="100" value="100" step="1"
                data-i18n-title="ui.tt.volume_cmd_up_down" title="Volume" />
              <span class="volume-pct" id="videoFsVolumePct">100%</span>
            </span>
          </div>
        </div>
      </div>
      <div class="meta-waveform" id="videoWaveformBox" data-path="${hp}" title="Click to seek">
        <canvas id="videoWaveformCanvas"></canvas>
        <div class="waveform-progress-fill"></div>
        <div class="waveform-loop-region" style="display:none;"></div>
        <div class="waveform-loop-brace waveform-loop-brace-start" data-loop-brace="start" style="display:none;left:25%;" title="Drag to set loop start"></div>
        <div class="waveform-loop-brace waveform-loop-brace-end" data-loop-brace="end" style="display:none;left:75%;" title="Drag to set loop end"></div>
        <button type="button" class="waveform-loop-toggle" data-action="toggleVideoLoopRegion" title="Toggle loop region">L</button>
        <div class="waveform-cursor" style="left:0;"></div>
        <div class="waveform-time-label"></div>
      </div>
      <div class="video-transport">
        <button type="button" class="btn-video-play" id="btnVideoPlayPause" data-action="videoPlayPause" title="Play / Pause">&#9654;</button>
        <span style="font-size:11px;font-family:'Orbitron',sans-serif;color:var(--text-muted);" id="videoTimeDisplay">0:00 / 0:00</span>
      </div>
      <div class="video-meta-info">
        <div class="meta-item"><span class="meta-label">FILE</span><span class="meta-value video-meta-path-line">${typeof escapeHtml === 'function' ? escapeHtml(fileName) : fileName}</span></div>
        <div class="meta-item"><span class="meta-label">PATH</span><span class="meta-value video-meta-path-line video-meta-path">${hp}</span></div>
      </div>
    </div></td>`;

    // Start playback immediately — do not await engine or waveform work on this stack.
    void previewVideo(filePath, { minimizeFloatingPlayer: true });

    // Waveform: one animation frame yields layout for `#videoWaveformCanvas` (double rAF cost ~32ms+ before IPC).
    _videoWfDrawSeq++;
    const seq = _videoWfDrawSeq;
    const runWaveform = () => {
        if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;
        void drawVideoWaveform(filePath, seq);
    };
    if (typeof requestAnimationFrame === 'function') {
        requestAnimationFrame(runWaveform);
    } else {
        setTimeout(runWaveform, 0);
    }

    if (typeof applyUiI18n === 'function') applyUiI18n();
    if (typeof applyMetaLoopRegionUI === 'function') applyMetaLoopRegionUI(filePath);
    _syncVideoMaximizeBtnState();
}

async function previewVideo(filePath, opts) {
    const o = opts && typeof opts === 'object' ? opts : {};
    if (videoPlayerPath === filePath && (_videoEngineActive || _videoFallbackAudio)) {
        const vid = document.getElementById('videoPlayerEl');
        if (_videoEngineActive) {
            const paused = window._enginePlaybackPaused === true;
            if (typeof window.vstUpdater?.audioEngineInvoke === 'function') {
                await window.vstUpdater.audioEngineInvoke({ cmd: 'playback_pause', paused: !paused });
            }
            window._enginePlaybackPaused = !paused;
            if (vid) {
                if (!paused) vid.pause();
                else void vid.play().catch(() => {});
            }
        } else if (vid) {
            if (vid.paused) void vid.play().catch(() => {});
            else vid.pause();
        }
        _updateVideoPlayBtn();
        if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
        if (typeof updatePlayBtnStates === 'function') updatePlayBtnStates();
        const playingNow = _videoEngineActive
            ? window._enginePlaybackPaused !== true
            : !!(vid && !vid.paused && !vid.ended);
        if (!playingNow) {
            if (_videoRafId) {
                cancelAnimationFrame(_videoRafId);
                _videoRafId = null;
            }
        } else if (!_videoRafId) {
            _videoRafId = requestAnimationFrame(_videoRafLoop);
        }
        return;
    }

    _lastVideoVisualSeekMs = 0;
    const vid = document.getElementById('videoPlayerEl');
    if (!vid) return;

    // `videoPlayerPath` only after we know `<video>` exists — otherwise we strand state and may have
    // already called `enginePlaybackStop` while nothing can play video.
    videoPlayerPath = filePath;

    // Fire-and-forget stop: `enginePlaybackStart → start_output_stream` always calls
    // `stopOutputLocked()` internally, and the C++ generation counter protects `sessionPath`
    // from stale `playback_stop` clears.  Awaiting the stop here would serialize a full
    // CoreAudio device close (~200-500 ms) that `start_output_stream` repeats anyway.
    if (typeof window._enginePlaybackActive !== 'undefined' && window._enginePlaybackActive) {
        if (typeof window.enginePlaybackStop === 'function') void window.enginePlaybackStop();
        if (typeof window.setEnginePlaybackActive === 'function') window.setEnginePlaybackActive(false);
    }
    // Pause HTML5 audio player if active
    if (typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused) {
        audioPlayer.pause();
    }

    _videoPlayerLoadUiSeq++;
    const videoLoadUiSeq = _videoPlayerLoadUiSeq;
    _wireVideoPlayerLoadingListeners(vid, filePath, videoLoadUiSeq);

    const canEngine =
        typeof window !== 'undefined' &&
        window.vstUpdater &&
        typeof window.vstUpdater.audioEngineInvoke === 'function' &&
        typeof window.enginePlaybackStart === 'function';

    const useEngineForVideoAudio = canEngine && videoPlaybackUsesEngineAudio();

    _videoFallbackAudio = false;
    _videoEngineActive = false;

    // Engine path: picture-only on `<video>`. HTML5 path: `<video>` carries sound (no inserts).
    vid.src = _videoFileSrc(filePath);
    vid.muted = useEngineForVideoAudio;
    vid.preload = 'auto';
    vid.loop =
        typeof prefs !== 'undefined' &&
        typeof prefs.getItem === 'function' &&
        prefs.getItem('audioLoop') === 'on';
    const loopPathForMeta = filePath;
    vid.addEventListener(
        'loadedmetadata',
        () => {
            if (videoPlayerPath !== loopPathForMeta) return;
            if (typeof syncAbLoopFromSampleRegion === 'function') syncAbLoopFromSampleRegion(loopPathForMeta);
            if (typeof window.syncVideoPlayerElPlaybackRate === 'function') window.syncVideoPlayerElPlaybackRate();
            /* `loadedmetadata` can run before `enginePlaybackStart` resolves — do not gate on `_videoEngineActive`. */
            if (
                typeof window._enginePlaybackDurSec === 'number' &&
                window._enginePlaybackDurSec <= 0 &&
                Number.isFinite(vid.duration) &&
                vid.duration > 0
            ) {
                window._enginePlaybackDurSec = vid.duration;
            }
            if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
        },
        { once: true },
    );

    if (useEngineForVideoAudio) {
        /* Do not `play()` until the engine clock exists. Starting decode ahead of JUCE output forces
         * `_videoRafLoop` to seek `<video>` backward every ~380ms — visible stutter for the first seconds. */
        try {
            vid.pause();
        } catch (_) {}
        /* Invalidate the <video> loadeddata/canplay hide — the muted <video> element finishes
         * loading well before `enginePlaybackStart` resolves, so the spinner would vanish while
         * the picture stays frozen.  Hide manually after `vid.play()` below instead. */
        _videoPlayerLoadUiSeq++;
        const engineLoadUiSeq = _videoPlayerLoadUiSeq;
        _startVideoRaf();
        _updateVideoPlayBtn();
        _showVideoInNowPlaying(filePath, o);

        void (async () => {
            try {
                await window.enginePlaybackStart(filePath, { streamFromDisk: true });
                if (videoPlayerPath !== filePath) return;
                _videoEngineActive = true;
                _videoFallbackAudio = false;
                vid.muted = true;
                if (
                    (typeof window._enginePlaybackDurSec !== 'number' || window._enginePlaybackDurSec <= 0) &&
                    Number.isFinite(vid.duration) &&
                    vid.duration > 0
                ) {
                    window._enginePlaybackDurSec = vid.duration;
                }
                if (typeof window.kickPlaybackRafLoop === 'function') window.kickPlaybackRafLoop();
                try {
                    const t =
                        typeof window._enginePlaybackPosSec === 'number' && Number.isFinite(window._enginePlaybackPosSec)
                            ? Math.max(0, window._enginePlaybackPosSec)
                            : 0;
                    vid.currentTime = t;
                    _lastVideoVisualSeekMs = performance.now();
                } catch (_) {}
                void vid.play().catch(() => {});
                if (engineLoadUiSeq === _videoPlayerLoadUiSeq) _hideVideoPlayerLoading();
                if (typeof window.syncVideoPlayerElPlaybackRate === 'function') window.syncVideoPlayerElPlaybackRate();
                _updateVideoPlayBtn();
                if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
                if (typeof updatePlayBtnStates === 'function') updatePlayBtnStates();
                if (typeof syncAbLoopFromSampleRegion === 'function') syncAbLoopFromSampleRegion(filePath);
            } catch {
                if (videoPlayerPath !== filePath) return;
                if (typeof window.setEnginePlaybackActive === 'function') window.setEnginePlaybackActive(false);
                if (typeof window.stopEnginePlaybackPoll === 'function') window.stopEnginePlaybackPoll();
                _videoEngineActive = false;
                _videoFallbackAudio = true;
                vid.muted = false;
                if (typeof window.kickPlaybackRafLoop === 'function') window.kickPlaybackRafLoop();
                try {
                    await vid.play();
                } catch (e) {
                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                }
                if (engineLoadUiSeq === _videoPlayerLoadUiSeq) _hideVideoPlayerLoading();
                if (
                    !vid.paused &&
                    typeof window.connectVideoHtml5AudioThroughWebAudio === 'function'
                ) {
                    window.connectVideoHtml5AudioThroughWebAudio(vid);
                }
                if (typeof window.syncVideoPlayerElPlaybackRate === 'function') window.syncVideoPlayerElPlaybackRate();
                _updateVideoPlayBtn();
                if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
                if (typeof updatePlayBtnStates === 'function') updatePlayBtnStates();
            }
        })();
        return;
    }

    // Pref `html5` or no AudioEngine IPC: `<video>` carries audio (inserts unavailable — Web Audio tap only).
    _videoFallbackAudio = true;
    _videoEngineActive = false;
    vid.muted = false;
    try {
        await vid.play();
    } catch (e) {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    }
    if (!vid.paused && typeof window.connectVideoHtml5AudioThroughWebAudio === 'function') {
        window.connectVideoHtml5AudioThroughWebAudio(vid);
    }
    _startVideoRaf();
    _updateVideoPlayBtn();
    _showVideoInNowPlaying(filePath, o);
}

function _startVideoRaf() {
    if (_videoRafId) cancelAnimationFrame(_videoRafId);
    _videoRafId = requestAnimationFrame(_videoRafLoop);
}

function _videoRafLoop() {
    _videoRafId = null;
    if (!videoPlayerPath) {
        _videoRafDomPath = null;
        _videoRafCachedVid = null;
        _videoRafCachedWfBox = null;
        _videoRafCachedTimeDisp = null;
        _videoRafCachedFsSeek = null;
        return;
    }

    if (_videoRafDomPath !== videoPlayerPath) {
        _videoRafDomPath = videoPlayerPath;
        _videoRafCachedVid = document.getElementById('videoPlayerEl');
        _videoRafCachedWfBox = document.getElementById('videoWaveformBox');
        _videoRafCachedTimeDisp = document.getElementById('videoTimeDisplay');
        _videoRafCachedFsSeek = document.getElementById('videoFsSeek');
        _videoFsSeekLastWritten = null;
    }
    let vid = _videoRafCachedVid;
    if (vid && !vid.isConnected) {
        _videoRafCachedVid = document.getElementById('videoPlayerEl');
        _videoRafCachedWfBox = document.getElementById('videoWaveformBox');
        _videoRafCachedTimeDisp = document.getElementById('videoTimeDisplay');
        _videoRafCachedFsSeek = document.getElementById('videoFsSeek');
        _videoFsSeekLastWritten = null;
        vid = _videoRafCachedVid;
    }
    let cur = 0;
    let dur = 0;

    if (_videoEngineActive && typeof window !== 'undefined') {
        // Interpolate engine position at rAF rate (honor playback speed)
        const basePos =
            typeof window._enginePlaybackPosSec === 'number' && Number.isFinite(window._enginePlaybackPosSec)
                ? window._enginePlaybackPosSec
                : 0;
        const anchor = typeof window._enginePlaybackPosAnchorMs === 'number'
            ? window._enginePlaybackPosAnchorMs : performance.now();
        const paused = window._enginePlaybackPaused === true;
        let speed = 1;
        if (typeof prefs !== 'undefined' && typeof prefs.getItem === 'function') {
            const raw = parseFloat(prefs.getItem('audioSpeed') || '1');
            if (Number.isFinite(raw)) speed = Math.max(0.25, Math.min(4, raw));
        }
        const elapsed = paused ? 0 : (performance.now() - anchor) / 1000;
        cur = basePos + elapsed * speed;
        dur = typeof window._enginePlaybackDurSec === 'number' ? window._enginePlaybackDurSec : 0;
        if (dur <= 0 && vid && Number.isFinite(vid.duration) && vid.duration > 0) dur = vid.duration;
        if (dur > 0 && cur > dur) cur = dur;
        if (cur < 0) cur = 0;

        // A-B loop enforcement — seek engine and snap `<video>` so the picture cannot sit past the end brace
        // while audio loops (skew throttle would otherwise delay the visual jump).
        if (typeof _abLoop !== 'undefined' && _abLoop && dur > 0 && cur >= _abLoop.end) {
            if (typeof window.vstUpdater?.audioEngineInvoke === 'function') {
                void window.vstUpdater.audioEngineInvoke({ cmd: 'playback_seek', position_sec: _abLoop.start });
            }
            cur = _abLoop.start;
            if (vid) {
                vid.currentTime = _abLoop.start;
                _lastVideoVisualSeekMs = performance.now();
            }
        }

        // Sync <video> to engine clock — throttle seeks: constant currentTime writes on big files freeze WebKit.
        // Above 1×, the same wall-clock thresholds fire far too often (decoder vs poll interpolation drift),
        // which reads as stutter; scale by √speed for speeds > 1 only (leave slow-mo paths unchanged).
        if (vid) {
            const skew = Math.abs(vid.currentTime - cur);
            const now = performance.now();
            const spdRel = speed > 1 ? Math.sqrt(speed) : 1;
            const hardSkew = 0.48 * spdRel;
            const softSkew = 0.2 * spdRel;
            const minSeekMs = Math.round(380 * spdRel);
            if (skew > hardSkew || (skew > softSkew && now - _lastVideoVisualSeekMs >= minSeekMs)) {
                vid.currentTime = cur;
                _lastVideoVisualSeekMs = now;
            }
        }
    } else if (vid) {
        cur = vid.currentTime || 0;
        dur = vid.duration || 0;
        if (typeof _abLoop !== 'undefined' && _abLoop && dur > 0 && cur >= _abLoop.end) {
            vid.currentTime = _abLoop.start;
            cur = _abLoop.start;
            _lastVideoVisualSeekMs = performance.now();
        }
    }

    /* Expanded-row waveform cursor + fill: `updatePlaybackTime` (audio.js) — keeps playhead correct when paused
     * and matches NP / engine / `<video>` transport in one place. */
    const timeDisp = _videoRafCachedTimeDisp;
    if (timeDisp) timeDisp.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;

    const fsSeek = _videoRafCachedFsSeek;
    if (fsSeek && dur > 0 && !_videoFsSeekUserDrag) {
        const v = Math.min(1000, Math.max(0, Math.round((cur / dur) * 1000)));
        if (_videoFsSeekLastWritten !== v) {
            _videoFsSeekLastWritten = v;
            fsSeek.value = String(v);
        }
    }

    // Check if playback ended
    if (_videoEngineActive && dur > 0 && cur >= dur - 0.05) {
        _updateVideoPlayBtn();
        if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
        return;
    }
    if (_videoFallbackAudio && vid && vid.ended) {
        _updateVideoPlayBtn();
        if (typeof updateNowPlayingBtn === 'function') updateNowPlayingBtn();
        return;
    }

    // Continue while playing, attaching engine-backed video, or **paused** with a loaded path (time / fs seek / tray helpers).
    const playing = _videoEngineActive
        ? (window._enginePlaybackPaused !== true)
        : _videoFallbackAudio
            ? !!(vid && !vid.paused && !vid.ended)
            : !!videoPlayerPath;
    if (playing || videoPlayerPath) {
        _videoRafId = requestAnimationFrame(_videoRafLoop);
    }
}

function _updateVideoPlayBtn() {
    const btn = document.getElementById('btnVideoPlayPause');
    if (!btn) return;
    const playing = _videoEngineActive
        ? (window._enginePlaybackPaused !== true)
        : (() => { const v = document.getElementById('videoPlayerEl'); return v && !v.paused && !v.ended; })();
    btn.innerHTML = playing ? '&#9646;&#9646;' : '&#9654;';
}

function seekVideoToPercent(pct) {
    const p = Math.max(0, Math.min(1, pct));
    const vid = document.getElementById('videoPlayerEl');
    if (_videoEngineActive && window.vstUpdater && typeof window.vstUpdater.audioEngineInvoke === 'function') {
        let dur = typeof window._enginePlaybackDurSec === 'number' ? window._enginePlaybackDurSec : 0;
        if (dur <= 0 && vid && Number.isFinite(vid.duration) && vid.duration > 0) dur = vid.duration;
        if (dur <= 0) return;
        const pos = p * dur;
        void window.vstUpdater.audioEngineInvoke({ cmd: 'playback_seek', position_sec: pos });
        if (vid) {
            vid.currentTime = pos;
            _lastVideoVisualSeekMs = performance.now();
        }
        window._enginePlaybackPosSec = pos;
        window._enginePlaybackPosAnchorMs = performance.now();
    } else if (vid && Number.isFinite(vid.duration) && vid.duration > 0) {
        vid.currentTime = p * vid.duration;
    }
    const fsSeek = document.getElementById('videoFsSeek');
    if (fsSeek) {
        const v = Math.min(1000, Math.max(0, Math.round(p * 1000)));
        _videoFsSeekLastWritten = v;
        fsSeek.value = String(v);
    }
    // Restart RAF if paused so UI updates
    if (!_videoRafId) _videoRafId = requestAnimationFrame(_videoRafLoop);
}

function seekVideoWaveform(event) {
    const box = document.getElementById('videoWaveformBox');
    if (!box || !videoPlayerPath) return;
    const rect = box.getBoundingClientRect();
    if (rect.width <= 0) return;
    const pct = (event.clientX - rect.left) / rect.width;
    seekVideoToPercent(pct);
}

function _videoRowSizeBytes(filePath) {
    const tbody = document.getElementById('videoTableBody');
    if (!tbody) return NaN;
    const row = tbody.querySelector(`tr[data-video-path="${CSS.escape(filePath)}"]`);
    if (!row || row.dataset.videoSize == null || row.dataset.videoSize === '') return NaN;
    const n = parseInt(row.dataset.videoSize, 10);
    return Number.isFinite(n) && n >= 0 ? n : NaN;
}

/** Host transcodes video containers (ffmpeg→MP3 or Symphonia→WAV) before JUCE `waveform_preview`. */
async function _ensureVideoElDuration(vid) {
    if (!vid) return false;
    if (vid.readyState >= 1 && Number.isFinite(vid.duration) && vid.duration > 0) return true;
    await new Promise((resolve) => {
        const done = () => resolve();
        vid.addEventListener('loadedmetadata', done, { once: true });
        vid.addEventListener('error', done, { once: true });
        setTimeout(done, 12000);
    });
    return Number.isFinite(vid.duration) && vid.duration > 0;
}

/**
 * Coarse timeline envelope from decoded video frames (spatial variance per time slice).
 * Not a true audio waveform; fills the bar when the engine cannot demux the container.
 */
async function _buildVideoVisualPeaksFromElement(vid, numCols, isStillValid) {
    if (!vid || numCols < 1 || typeof isStillValid !== 'function') return null;
    const okDur = await _ensureVideoElDuration(vid);
    if (!okDur || !isStillValid()) return null;
    const duration = vid.duration;
    const n = Math.min(120, Math.max(48, Math.min(numCols, 200)));
    const metrics = new Array(n);
    const w = 48;
    const h = 48;
    const oc = document.createElement('canvas');
    oc.width = w;
    oc.height = h;
    const octx = oc.getContext('2d', { willReadFrequently: true });
    if (!octx) return null;

    const wasPaused = vid.paused;
    if (!wasPaused) vid.pause();

    const t0 = vid.currentTime;
    try {
        for (let i = 0; i < n; i++) {
            if (!isStillValid()) return null;
            const target = ((i + 0.5) / n) * duration;
            if (Math.abs(vid.currentTime - target) > 0.05) {
                vid.currentTime = target;
                await new Promise((resolve) => {
                    const onSeeked = () => {
                        vid.removeEventListener('seeked', onSeeked);
                        resolve();
                    };
                    vid.addEventListener('seeked', onSeeked);
                    setTimeout(() => {
                        vid.removeEventListener('seeked', onSeeked);
                        resolve();
                    }, 2500);
                });
            }
            if (!isStillValid()) return null;
            octx.fillStyle = '#000';
            octx.fillRect(0, 0, w, h);
            try {
                octx.drawImage(vid, 0, 0, w, h);
            } catch {
                return null;
            }
            let img;
            try {
                img = octx.getImageData(0, 0, w, h);
            } catch {
                return null;
            }
            const d = img.data;
            let sum = 0;
            let sum2 = 0;
            let count = 0;
            for (let p = 0; p < d.length; p += 64) {
                const lum = 0.299 * d[p] + 0.587 * d[p + 1] + 0.114 * d[p + 2];
                sum += lum;
                sum2 += lum * lum;
                count++;
            }
            if (count < 1) {
                metrics[i] = 0;
            } else {
                const mean = sum / count;
                const variance = Math.max(0, sum2 / count - mean * mean);
                metrics[i] = Math.min(1, Math.sqrt(variance) / 96);
            }
            if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        }
    } finally {
        try {
            vid.currentTime = t0;
        } catch { /* ignore */ }
        if (!wasPaused && isStillValid()) void vid.play().catch(() => {});
    }

    const peaks = [];
    for (let i = 0; i < numCols; i++) {
        const u = (i + 0.5) / numCols;
        const ix = u * (n - 1);
        const i0 = Math.floor(ix);
        const i1 = Math.min(n - 1, i0 + 1);
        const f = ix - i0;
        const m = metrics[i0] * (1 - f) + metrics[i1] * f;
        const amp = Math.min(1, m * 2.8);
        peaks.push({ min: -amp * 0.42, max: amp * 0.95 });
    }
    return peaks;
}

/** Sync width for `waveform_preview` width_px — starts IPC before `resolveWaveformBoxSize` (can wait many rAFs). */
function _videoWaveformBoxWidthHint(container) {
    if (!container) return 560;
    try {
        const r = container.getBoundingClientRect();
        if (r.width >= 2) return Math.min(2000, Math.round(r.width));
    } catch {
        /* ignore */
    }
    const cw = container.clientWidth;
    if (cw >= 2) return Math.min(2000, cw);
    return 560;
}

async function drawVideoWaveform(filePath, seq) {
    const canvas = document.getElementById('videoWaveformCanvas');
    if (!canvas) return;
    if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;

    const container = canvas.parentElement;
    const cwHint = _videoWaveformBoxWidthHint(container);

    if (typeof window.hydrateWaveformPeaksFromSqlite === 'function') {
        await window.hydrateWaveformPeaksFromSqlite(filePath);
        if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;
    }

    const cachedPeaks = typeof _waveformCache !== 'undefined' ? _waveformCache[filePath] : null;
    if (cachedPeaks && Array.isArray(cachedPeaks) && cachedPeaks.length > 0 && typeof renderWaveformData === 'function') {
        let cw = cwHint;
        let ch = 56;
        if (typeof resolveWaveformBoxSize === 'function') {
            const dim = await resolveWaveformBoxSize(container, 560, 56);
            if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;
            cw = dim.w;
            ch = dim.h;
        } else if (container) {
            cw = container.clientWidth || cwHint;
            ch = container.clientHeight || 56;
        }
        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.max(1, Math.round(cw * dpr));
        canvas.height = Math.max(1, Math.round(ch * dpr));
        const ctx = canvas.getContext('2d');
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        renderWaveformData(ctx, canvas, cachedPeaks);
        return;
    }

    const barsEarly = Math.max(1, Math.min(Math.floor(cwHint), 800));
    const enginePromise =
        typeof fetchWaveformPreviewFromEngine === 'function'
            ? fetchWaveformPreviewFromEngine(filePath, barsEarly)
            : Promise.resolve(null);

    let cw = cwHint;
    let ch = 56;
    if (typeof resolveWaveformBoxSize === 'function') {
        const dim = await resolveWaveformBoxSize(container, 560, 56);
        if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;
        cw = dim.w;
        ch = dim.h;
    } else if (container) {
        cw = container.clientWidth || cwHint;
        ch = container.clientHeight || 56;
    }

    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(cw * dpr));
    canvas.height = Math.max(1, Math.round(ch * dpr));
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const rowBytes = _videoRowSizeBytes(filePath);
    const skipVisualSeekHeavy = Number.isFinite(rowBytes) && rowBytes > VIDEO_VISUAL_WAVEFORM_MAX_FILE_BYTES;

    let peaks = null;
    try {
        peaks = await enginePromise;
    } catch {
        /* ignore */
    }

    if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;

    // `expandedVideoPath` is set before `previewVideo` — unlike `videoPlayerPath`, it is not delayed
    // by `await enginePlaybackStop()`, so frame-sampled peaks still run when engine path fails.
    if (
        (!peaks || !peaks.length)
        && !skipVisualSeekHeavy
        && typeof renderWaveformData === 'function'
        && expandedVideoPath === filePath
    ) {
        const vid = document.getElementById('videoPlayerEl');
        const barsVisual = Math.max(1, Math.min(Math.floor(cw), 800));
        const visual = await _buildVideoVisualPeaksFromElement(vid, barsVisual, () => seq === _videoWfDrawSeq && expandedVideoPath === filePath);
        if (visual && visual.length) peaks = visual;
    }

    if (seq !== _videoWfDrawSeq || expandedVideoPath !== filePath) return;

    if (peaks && Array.isArray(peaks) && peaks.length > 0) {
        if (typeof storeWaveformPeaksInCache === 'function') {
            storeWaveformPeaksInCache(filePath, peaks);
        } else if (typeof _waveformCache !== 'undefined') {
            _waveformCache[filePath] = peaks;
            if (typeof _evictCache === 'function') _evictCache(_waveformCache);
            if (typeof notifyWaveformCacheUpdatedForTray === 'function') notifyWaveformCacheUpdatedForTray(filePath);
        }
    }

    if (peaks && typeof renderWaveformData === 'function') {
        renderWaveformData(ctx, canvas, peaks);
    } else {
        ctx.strokeStyle = 'rgba(5,217,232,0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, canvas.height / 2);
        ctx.lineTo(canvas.width, canvas.height / 2);
        ctx.stroke();
    }
}

/** Called from keyboard-nav when navigating video rows with expand open. */
function syncExpandedVideoMetaWithKeyboardSelection(newPath) {
    if (expandedVideoPath === null) return;
    if (expandedVideoPath === newPath) return;
    void expandVideoMetaForPath(newPath);
}

const _VIDEO_EXPORT_MAX = 100000;

async function fetchVideosForExport() {
    const search = _lastVideoSearch || '';
    const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('videoFormatFilter') : null;
    const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
    let total = _videoTotalCount || 0;
    if (total <= 0) {
        try {
            const probe = await window.vstUpdater.dbQueryVideo({
                search: search || null,
                format_filter: formatFilter,
                sort_key: videoSortKey,
                sort_asc: videoSortAsc,
                search_regex: _lastVideoMode === 'regex',
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch {
            return [];
        }
    }
    const n = Math.min(total, _VIDEO_EXPORT_MAX);
    if (n <= 0) return [];
    const result = await window.vstUpdater.dbQueryVideo({
        search: search || null,
        format_filter: formatFilter,
        sort_key: videoSortKey,
        sort_asc: videoSortAsc,
        search_regex: _lastVideoMode === 'regex',
        offset: 0,
        limit: n,
    });
    return result.videoFiles || [];
}

/**
 * Path for video play/pause / shortcuts: `videoPlayerPath` when loaded; else expanded row only while
 * the Videos tab is active (hidden `#tabVideos` still holds `expandedVideoPath` / `#videoMetaRow` in DOM).
 * `previewAudio` → `stopVideoPlayback({ keepVideoFrame: true })` clears `videoPlayerPath` but keeps the
 * last video frame; full `stopVideoPlayback()` clears `src` (e.g. meta row close).
 */
function getVideoTransportTargetPath() {
    if (typeof videoPlayerPath !== 'undefined' && videoPlayerPath) return videoPlayerPath;
    const onVideosTab =
        typeof document !== 'undefined' && document.querySelector('.tab-content.active')?.id === 'tabVideos';
    if (!onVideosTab) return '';
    if (expandedVideoPath) return expandedVideoPath;
    const row = document.getElementById('videoMetaRow');
    const p = row && row.getAttribute('data-meta-path');
    return p ? String(p) : '';
}

if (typeof window !== 'undefined') {
    window.getVideoTransportTargetPath = getVideoTransportTargetPath;
}

if (typeof document !== 'undefined') {
    document.addEventListener('fullscreenchange', _syncVideoMaximizeBtnState);
    document.addEventListener('webkitfullscreenchange', _syncVideoMaximizeBtnState);
    document.addEventListener(
        'pointerdown',
        (e) => {
            const t = e.target;
            if (t && typeof t.id === 'string' && t.id === 'videoFsSeek') _videoFsSeekUserDrag = true;
        },
        true
    );
    document.addEventListener('pointerup', () => {
        _videoFsSeekUserDrag = false;
    });
    document.addEventListener('pointercancel', () => {
        _videoFsSeekUserDrag = false;
    });
    /** Fullscreen: click picture to seek when the host is `#videoFullscreenRoot` or legacy video-only fullscreen. */
    document.addEventListener(
        'pointerdown',
        (e) => {
            if (!videoPlayerPath) return;
            const vid = e.target instanceof Element ? e.target.closest('#videoPlayerEl') : null;
            if (!vid) return;
            if (e.target instanceof Element && e.target.closest('.video-fs-overlay')) return;
            const doc = document;
            const fsEl = doc.fullscreenElement ?? doc.webkitFullscreenElement;
            const host = _videoFullscreenHostEl();
            const fsVideo = fsEl === vid;
            const fsHost = !!(host && fsEl === host);
            if (!fsVideo && !fsHost) return;
            if (e.button !== 0) return;
            const r = vid.getBoundingClientRect();
            if (r.width <= 0) return;
            e.preventDefault();
            const pct = (e.clientX - r.left) / r.width;
            seekVideoToPercent(pct);
        },
        true
    );
}
