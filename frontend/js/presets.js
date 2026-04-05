// ── Presets ──
let allPresets = [];
let filteredPresets = [];
let presetSortKey = 'name';
let presetSortAsc = true;
let presetScanProgressCleanup = null;
let PRESET_PAGE_SIZE = 500;
let presetRenderCount = 0;
let _presetOffset = 0;
let _presetTotalCount = 0;
let _presetTotalUnfiltered = 0;
// Incremental stats for presets — avoids O(N) rebuild on every scan flush.
let _presetStatsTotalBytes = 0;
let _presetStatsFormatCounts = {};

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
  const search = document.getElementById('presetSearchInput')?.value || '';
  const fmtSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('presetFormatFilter') : null;
  const formatFilter = fmtSet ? [...fmtSet].join(',') : null;
  // During an active scan, DOM-toggle filter existing rendered rows (O(visible))
  // instead of re-scanning the in-memory array. Scan streaming already excludes
  // MIDI and filters by active selection going forward.
  if (presetScanProgressCleanup) {
    const tbody = document.getElementById('presetTableBody');
    if (tbody) {
      const needle = search ? search.trim().toLowerCase() : '';
      const rows = tbody.rows;
      let visible = 0;
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i];
        const fmt = r.dataset.presetFormat;
        if (!fmt) continue;
        let match = true;
        if (fmtSet && !fmtSet.has(fmt)) match = false;
        if (match && needle && !r.dataset.presetName.includes(needle)) match = false;
        r.style.display = match ? '' : 'none';
        if (match) visible++;
      }
      _presetTotalCount = visible;
    }
    return;
  }
  try {
    const result = await window.vstUpdater.dbQueryPresets({
      search: search || null,
      format_filter: formatFilter,
      sort_key: presetSortKey,
      sort_asc: presetSortAsc,
      offset: _presetOffset,
      limit: PRESET_PAGE_SIZE,
    });
    let presets = result.presets || [];
    // Re-sort by fzf relevance score
    if (search && presets.length > 1) {
      const scored = presets.map(p => ({ p, score: searchScore(search, [p.name], _lastPresetMode) }));
      scored.sort((a, b) => b.score - a.score);
      presets = scored.map(x => x.p);
    }
    if (_presetOffset === 0) {
      filteredPresets = presets;
      allPresets = filteredPresets;
    } else {
      filteredPresets.push(...presets);
      allPresets.push(...presets);
    }
    _presetTotalCount = result.totalCount || 0;
    _presetTotalUnfiltered = result.totalUnfiltered || 0;
    renderPresetTable();
    rebuildPresetStats();
  } catch (e) {
    showToast(toastFmt('toast.preset_query_failed', { err: e }), 4000, 'error');
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
  const checked = batchSelected.has(p.path) ? ' checked' : '';
  return `<tr data-preset-path="${hp}" data-preset-format="${escapeHtml(p.format)}" data-preset-name="${escapeHtml((p.name || '').toLowerCase())}" style="cursor: pointer;" title="Double-click to reveal in Finder">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td>${_lastPresetSearch ? highlightMatch(p.name, _lastPresetSearch, _lastPresetMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? rowBadges(p.path) : ''}</td>
    <td class="col-format"><span class="format-badge format-default">${p.format}</span></td>
    <td title="${hp}">${escapeHtml(p.directory)}</td>
    <td class="col-size">${p.sizeFormatted || formatPresetSize(p.size)}</td>
    <td class="col-date">${p.modified}</td>
    <td class="col-actions">
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

function rebuildPresetStats() {
  const statsEl = document.getElementById('presetStats');
  if (!statsEl) return;
  // Use unfiltered scan count so the stats don't flash to 0 when a filter matches nothing.
  const displayCount = _presetTotalUnfiltered || _presetTotalCount || allPresets.length;
  statsEl.style.display = displayCount > 0 ? 'flex' : 'none';

  document.getElementById('presetCount').textContent = displayCount.toLocaleString();
  const headerCount = document.getElementById('presetCountHeader');
  if (headerCount) headerCount.textContent = displayCount.toLocaleString();

  // If accumulators are empty (e.g., after post-scan reload from DB), backfill from
  // allPresets ONCE. Subsequent flushes stay O(batch).
  if (_presetStatsTotalBytes === 0 && Object.keys(_presetStatsFormatCounts).length === 0 && allPresets.length > 0) {
    accumulatePresetStats(allPresets);
  }
  document.getElementById('presetTotalSize').textContent = formatPresetSize(_presetStatsTotalBytes);

  const entries = Object.entries(_presetStatsFormatCounts).sort((a, b) => b[1] - a[1]);
  const fmtHtml = entries
    .map(([fmt, count]) => `<span class="format-badge format-default">${fmt}: ${count}</span>`)
    .join(' ');
  document.getElementById('presetFormatBreakdown').innerHTML = fmtHtml;
}

function resetPresetStats() {
  document.getElementById('presetCount').textContent = '0';
  document.getElementById('presetTotalSize').textContent = '0 B';
  document.getElementById('presetFormatBreakdown').innerHTML = '';
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
  resetOffset() { _presetOffset = 0; },
  fetchFn() {
    _lastPresetSearch = this.lastSearch || '';
    _lastPresetMode = this.lastMode || 'fuzzy';
    fetchPresetPage();
    if (typeof loadMidiFiles === 'function') { _midiLoaded = false; loadMidiFiles(); }
  },
});
function filterPresets() { applyFilter('filterPresets'); }

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
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="7" style="text-align:center; padding: 12px;">
        <button class="btn btn-secondary" data-action="loadMorePresets" title="Load next batch of presets">Load more (${filteredPresets.length - PRESET_PAGE_SIZE} remaining)</button>
      </td></tr>`);
  }

  document.getElementById('presetFilteredCount').textContent =
    filteredPresets.length < allPresets.length ? `${filteredPresets.length} / ` : '';
}

function renderPresetTable() {
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
  tbody.innerHTML = filteredPresets.map(buildPresetRow).join('');
  presetRenderCount = filteredPresets.length;
  if (presetRenderCount < _presetTotalCount) {
    tbody.insertAdjacentHTML('beforeend',
      `<tr><td colspan="7" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMorePresets">
        Showing ${presetRenderCount} of ${_presetTotalCount} — click to load more
      </td></tr>`);
  }
  const fc = document.getElementById('presetFilteredCount');
  if (fc) fc.textContent = presetRenderCount < _presetTotalCount ? `${presetRenderCount} / ` : '';
}

function loadMorePresets() {
  _presetOffset = allPresets.length;
  fetchPresetPage();
}

function openPresetFolder(path) {
  window.vstUpdater.openPresetFolder(path).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', { err: e }), 4000, 'error'));
}

async function scanPresets(resume = false) {
  showGlobalProgress();
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
    resetPresetStatsAccumulators();
    document.getElementById('presetStats').style.display = 'none';
    tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for presets...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let firstBatch = true;
  let pendingPresets = [];
  if (typeof _midiScanCount !== 'undefined') _midiScanCount = 0;
  let pendingFound = 0;
  let flushScheduled = false;
  const presetEta = createETA();
  presetEta.start();
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);
  let lastFlush = 0;

  function flushPending() {
    flushScheduled = false;
    if (pendingPresets.length === 0) return;
    const batch = pendingPresets.splice(0);

    if (firstBatch) {
      firstBatch = false;
      tableWrap.innerHTML = `<table class="audio-table" id="presetTable">
        ${presetTableHeadHtml()}
        <tbody id="presetTableBody"></tbody>
      </table>`;
      document.getElementById('presetStats').style.display = 'flex';
      if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('presetTable'));
      if (typeof initTableColumnReorder === 'function') initTableColumnReorder('presetTable', 'presetColumnOrder');
    }

    // Split: MIDI files go to MIDI tab, presets stay here
    const midiFormats = new Set(['MID', 'MIDI']);
    const midiBatch = batch.filter(p => midiFormats.has(p.format));
    const presetBatch = batch.filter(p => !midiFormats.has(p.format));
    allPresets.push(...batch); // keep all in allPresets for export/history
    filteredPresets.push(...presetBatch);
    // Incrementally update stats — O(batch) not O(total).
    accumulatePresetStats(batch);
    // Stream MIDI files to MIDI tab incrementally
    if (midiBatch.length > 0 && typeof allMidiFiles !== 'undefined') {
      allMidiFiles.push(...midiBatch);
      if (typeof filteredMidi !== 'undefined') filteredMidi.push(...midiBatch);
      // Append rows instead of full rebuild
      const midiTbody = document.getElementById('midiTableBody');
      if (midiTbody && typeof buildMidiRow === 'function' && typeof _midiRenderCount !== 'undefined' && _midiRenderCount < 2000) {
        const toRender = midiBatch.slice(0, 2000 - _midiRenderCount);
        midiTbody.insertAdjacentHTML('beforeend', toRender.map(buildMidiRow).join(''));
        _midiRenderCount += toRender.length;
      } else if (!midiTbody && typeof renderMidiTable === 'function') {
        renderMidiTable(); // first batch — init table
      }
      if (typeof updateMidiCount === 'function') updateMidiCount();
      // Trigger metadata load for new MIDI rows
      if (typeof _midiMetadataRunning !== 'undefined' && !_midiMetadataRunning && typeof loadMidiMetadata === 'function') loadMidiMetadata();
    }
    const tbody = document.getElementById('presetTableBody');
    if (tbody && presetRenderCount < 2000) {
      const loadMore = document.getElementById('presetLoadMore');
      if (loadMore) loadMore.remove();
      // Apply active filter so newly-streamed rows respect user's checkbox/search.
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

    rebuildPresetStats();
    const presetElapsed = presetEta.elapsed();
    btn.innerHTML = `&#8635; ${pendingFound} found${presetElapsed ? ' — ' + presetElapsed : ''}`;
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
      // Split count: presets vs MIDI
      const midiFormats = new Set(['MID', 'MIDI']);
      const midiInBatch = data.presets ? data.presets.filter(p => midiFormats.has(p.format)).length : 0;
      if (typeof _midiScanCount !== 'undefined') _midiScanCount += midiInBatch;
      const presetOnly = pendingFound - (typeof _midiScanCount !== 'undefined' ? _midiScanCount : 0);
      document.getElementById('presetCountHeader').textContent = presetOnly;
      const midiEl = document.getElementById('midiScanCount');
      if (midiEl) midiEl.textContent = typeof _midiScanCount !== 'undefined' ? _midiScanCount : 0;
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
    // Refresh header count immediately — don't wait for next fetchPresetPage.
    // Exclude MIDI since they live in their own tab (matches backend `total_unfiltered` definition).
    const midiFormats = new Set(['MID', 'MIDI']);
    _presetTotalUnfiltered = allPresets.filter(p => !midiFormats.has(p.format)).length;
    // Reload MIDI tab from preset data
    if (typeof loadMidiFiles === 'function') { _midiLoaded = false; loadMidiFiles(); }
    if (!result.stopped) {
      try { await window.vstUpdater.savePresetScan(allPresets, result.roots); } catch (e) { showToast(toastFmt('toast.failed_save_preset_history', { err: e.message || e }), 4000, 'error'); }
    }
    if (result.stopped && allPresets.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (presetScanProgressCleanup) { presetScanProgressCleanup(); presetScanProgressCleanup = null; }
    flushPending();
    const errMsg = err.message || err || 'Unknown error';
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    showToast(toastFmt('toast.preset_scan_failed', { errMsg }), 4000, 'error');
  }

  hideGlobalProgress();
  btn.disabled = false;
  btn.innerHTML = '&#127924; Scan Presets';
  stopBtn.style.display = 'none';
  document.getElementById('btnExportPresets').style.display = allPresets.length > 0 ? '' : 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
}

async function stopPresetScan() {
  await window.vstUpdater.stopPresetScan();
}
