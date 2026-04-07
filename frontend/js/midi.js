// ── MIDI Tab ──
// Dedicated tab for MIDI files with sortable/draggable columns and MIDI-specific metadata.

function _midiFmt(key, vars) {
  if (typeof appFmt !== 'function') return key;
  return vars ? appFmt(key, vars) : appFmt(key);
}

let allMidiFiles = [];        // streaming buffer during scan (capped at 100K)
let filteredMidi = [];        // current DB page only (page-at-a-time model)
let _midiInfoCache = {};
let _midiLoaded = false;
let _midiTableInit = false;
let _midiRenderCount = 0;     // cumulative rows in DOM (offset + current page size)
let _midiMetadataRunning = false;
let midiSortKey = 'name';
let midiSortAsc = true;
let _midiOffset = 0;
let _midiTotalCount = 0;      // filtered count from DB
let _midiTotalUnfiltered = 0; // unfiltered count from DB
let _midiStatsSnapshot = null;
/** Monotonic id so stale `dbQueryMidi` results never overwrite a newer filter. (`MIDI_PAGE_SIZE` lives in ipc.js.) */
let _midiQuerySeq = 0;

function mountMidiTableShell() {
  const wrap = document.getElementById('midiTableWrap');
  if (!wrap) return;
  _midiRenderCount = 0;
  const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
  const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
  const selTitle = esc(tc('ui.audio.th_select_all'));
  const arrow = (k) => `<span class="sort-arrow" id="midiSortArrow${k}">${midiSortKey === k.toLowerCase() ? (midiSortAsc ? '&#9650;' : '&#9660;') : ''}</span>`;
  wrap.innerHTML = `<table class="audio-table" id="midiTable">
      <thead>
        <tr>
          <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="${selTitle}"></th>
          <th data-action="sortMidi" data-key="name" style="width:22%;" title="${esc(tc('ui.midi.tt_sort_name'))}">${tc('ui.export.col_name')} ${arrow('Name')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="tracks" style="width:55px;" title="${esc(tc('ui.midi.tt_sort_tracks'))}">${tc('ui.export.col_tracks')} ${arrow('Tracks')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="bpm" style="width:65px;" title="${esc(tc('ui.midi.tt_sort_bpm'))}">${tc('ui.export.col_bpm')} ${arrow('Bpm')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="time" style="width:55px;" title="${esc(tc('ui.midi.tt_sort_time_sig'))}">${tc('ui.midi.th_time')} ${arrow('Time')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="key" style="width:80px;" title="${esc(tc('ui.midi.tt_sort_key'))}">${tc('ui.export.col_key')} ${arrow('Key')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="notes" style="width:60px;" title="${esc(tc('ui.midi.tt_sort_notes'))}">${tc('ui.export.col_notes')} ${arrow('Notes')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="ch" style="width:45px;" title="${esc(tc('ui.midi.tt_sort_ch'))}">${tc('ui.export.col_ch')} ${arrow('Ch')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="duration" style="width:65px;" title="${esc(tc('ui.midi.tt_sort_duration'))}">${tc('ui.audio.th_dur')} ${arrow('Duration')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="size" style="width:60px;" title="${esc(tc('ui.midi.tt_sort_size'))}">${tc('ui.export.col_size')} ${arrow('Size')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="path" style="width:22%;" title="${esc(tc('ui.midi.tt_sort_path'))}">${tc('ui.export.col_path')} ${arrow('Path')}<span class="col-resize"></span></th>
          <th class="col-actions" style="width:50px;"></th>
        </tr>
      </thead>
      <tbody id="midiTableBody"></tbody>
    </table>`;
  if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('midiTable'));
  if (typeof initTableColumnReorder === 'function') initTableColumnReorder('midiTable', 'midiColumnOrder');
  _midiTableInit = true;
}

function ensureMidiTableForQuery() {
  if (document.getElementById('midiTable')) return;
  mountMidiTableShell();
}

function showMidiQueryLoading(isLoadMore) {
  ensureMidiTableForQuery();
  showTableQueryLoadingRow({
    tbodyId: 'midiTableBody',
    rowId: 'midiQueryLoadingRow',
    tableId: 'midiTable',
    colspan: 12,
    append: isLoadMore,
    label: typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…',
  });
}

async function loadMidiFiles() {
  // Initial paginated load from SQLite — mirrors audio.js pattern. Memory stays
  // bounded at one page regardless of scan size (6M+ safe).
  _midiLoaded = true;
  _midiTableInit = false;
  _midiRenderCount = 0;
  _midiOffset = 0;
  await fetchMidiPage();
  await refreshMidiStatsSnapshot(true);
  updateMidiHeaderCount();
}

async function fetchMidiPage() {
  const search = _midiSearch || '';
  const seq = ++_midiQuerySeq;
  const isLoadMore = _midiOffset > 0;
  showMidiQueryLoading(isLoadMore);
  if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('midiSearchInput', true);
  if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
  else await new Promise((r) => requestAnimationFrame(r));
  try {
    const result = await window.vstUpdater.dbQueryMidi({
      search: search || null,
      format_filter: null,
      sort_key: midiSortKey,
      sort_asc: midiSortAsc,
      offset: _midiOffset,
      limit: MIDI_PAGE_SIZE,
    });
    if (seq !== _midiQuerySeq) return;
    let files = result.midiFiles || [];
    // Re-sort by fzf relevance when searching
    if (search && files.length > 1 && typeof searchScore === 'function') {
      const mode = typeof getSearchMode === 'function' ? getSearchMode('regexMidi') : 'fuzzy';
      const scored = files.map(s => ({ s, score: searchScore(search, [s.name, s.directory || ''], mode) }));
      scored.sort((a, b) => b.score - a.score);
      files = scored.map(x => x.s);
    }
    filteredMidi = files;
    _midiTotalCount = result.totalCount || 0;
    _midiTotalUnfiltered = result.totalUnfiltered || 0;
    // Metadata-based sorts (tracks/bpm/time/key/notes/ch/duration) aren't stored in
    // the DB — SQL can only sort by name/size/modified/directory. For metadata sorts
    // we re-sort the current page client-side using _midiInfoCache (partial by design).
    const metadataKeys = new Set(['tracks', 'bpm', 'time', 'key', 'notes', 'ch', 'duration']);
    if (metadataKeys.has(midiSortKey)) {
      sortMidiArray();
    }
    if (typeof yieldToBrowser === 'function') await yieldToBrowser();
    if (seq !== _midiQuerySeq) return;
    renderMidiTable();
    if (_midiScanProgressCleanup) _midiScanDbView = true;
    updateMidiCount();
  } catch (e) {
    if (seq !== _midiQuerySeq) return;
    clearTableQueryLoadingRow('midiQueryLoadingRow', 'midiTable');
    if (typeof showToast === 'function') showToast(toastFmt('toast.midi_load_failed', { err: e.message || e }), 4000, 'error');
  } finally {
    if (seq === _midiQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('midiSearchInput', false);
  }
}

async function refreshMidiStatsSnapshot(force) {
  try {
    const search = _midiSearch || '';
    const agg = await window.vstUpdater.dbMidiFilterStats(search, null);
    if (typeof yieldToBrowser === 'function') await yieldToBrowser();
    _midiStatsSnapshot = {
      count: agg.count || 0,
      totalBytes: agg.totalBytes || 0,
      totalUnfiltered: agg.totalUnfiltered || 0,
    };
    _midiTotalCount = agg.count || 0;
    _midiTotalUnfiltered = agg.totalUnfiltered || 0;
    updateMidiCount();
  } catch {
    _midiStatsSnapshot = null;
    updateMidiCount();
  }
}

// ── MIDI scanner — fully independent from preset scanner ──
let _midiScanProgressCleanup = null;
let _midiScanDbView = false;

async function stopMidiScan() {
  try { await window.vstUpdater.stopMidiScan(); } catch (e) { /* ignore */ }
}

async function scanMidi(resume = false, overrideRoots = null) {
  if (typeof showGlobalProgress === 'function') showGlobalProgress();
  const btn = document.getElementById('btnScanMidi');
  const resumeBtn = document.getElementById('btnResumeMidi');
  const stopBtn = document.getElementById('btnStopMidi');
  const progressBar = document.getElementById('midiProgressBar');
  const progressFill = document.getElementById('midiProgressFill');
  const tableWrap = document.getElementById('midiTableWrap');
  const setBtn = (html, disabled) => { if (btn) { btn.innerHTML = html; btn.disabled = disabled; } };

  const excludePaths = resume ? allMidiFiles.map(m => m.path) : null;

  if (typeof btnLoading === 'function') btnLoading(btn, true);
  setBtn(
    '&#8635; ' + catalogFmt(resume ? 'ui.js.resuming_btn' : 'ui.js.scanning_btn'),
    true,
  );
  if (resumeBtn) resumeBtn.style.display = 'none';
  if (stopBtn) stopBtn.style.display = '';
  if (progressBar) progressBar.classList.add('active');
  if (progressFill) {
    progressFill.style.width = '';
    progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
  }

  if (!resume) {
    _midiScanDbView = false;
    allMidiFiles = [];
    filteredMidi = [];
    _midiInfoCache = {};
    _midiRenderCount = 0;
    _midiTableInit = false;
  }

  let scanMidiDomActive = false;
  let pendingMidi = [];
  let pendingFound = 0;
  let firstMidiBatch = true;
  const midiEta = typeof createETA === 'function' ? createETA() : null;
  if (midiEta) midiEta.start();
  const FLUSH_INTERVAL = parseInt((typeof prefs !== 'undefined' ? prefs.getItem('flushInterval') : null) || '100', 10);

  function flushPendingMidi() {
    if (pendingMidi.length === 0) return;
    const toAdd = pendingMidi;
    pendingMidi = [];

    const allowDom =
      scanMidiDomActive ||
      (typeof isMidiScanTableEmpty === 'function' && isMidiScanTableEmpty());
    if (!allowDom) {
      allMidiFiles.push(...toAdd);
      if (allMidiFiles.length > 100000) allMidiFiles.length = 100000;
      if (filteredMidi.length > 100000) filteredMidi.length = 100000;
      const q = (typeof _midiSearch === 'string' && _midiSearch) ? _midiSearch : '';
      const mode = typeof getSearchMode === 'function' ? getSearchMode('regexMidi') : 'fuzzy';
      const matching = q
        ? toAdd.filter(s => typeof searchMatch === 'function'
            ? searchMatch(q, [s.name, s.directory || ''], mode)
            : s.name.toLowerCase().includes(q.toLowerCase()))
        : toAdd;
      filteredMidi.push(...matching);
      updateMidiCount();
      updateMidiHeaderCount();
      if (progressFill) {
        progressFill.style.width = '';
        progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
      }
      return;
    }
    scanMidiDomActive = true;

    if (firstMidiBatch) { firstMidiBatch = false; _midiTableInit = false; _midiRenderCount = 0; }
    allMidiFiles.push(...toAdd);
    if (allMidiFiles.length > 100000) allMidiFiles.length = 100000;
    if (filteredMidi.length > 100000) filteredMidi.length = 100000;
    const q = (typeof _midiSearch === 'string' && _midiSearch) ? _midiSearch : '';
    const mode = typeof getSearchMode === 'function' ? getSearchMode('regexMidi') : 'fuzzy';
    const matching = q
      ? toAdd.filter(s => typeof searchMatch === 'function'
          ? searchMatch(q, [s.name, s.directory || ''], mode)
          : s.name.toLowerCase().includes(q.toLowerCase()))
      : toAdd;
    filteredMidi.push(...matching);
    if (!_midiScanDbView) {
      const tbody = document.getElementById('midiTableBody');
      if (!tbody) {
        renderMidiTable();
      } else if (_midiRenderCount < 2000) {
        const loadMore = document.getElementById('midiLoadMore');
        if (loadMore) loadMore.remove();
        const toRender = matching.slice(0, 2000 - _midiRenderCount);
        tbody.insertAdjacentHTML('beforeend', toRender.map(buildMidiRow).join(''));
        _midiRenderCount += toRender.length;
      }
    }
    updateMidiCount();
    updateMidiHeaderCount();
    if (progressFill) {
      progressFill.style.width = '';
      progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';
    }
  }

  const scheduleMidiFlush = createScanFlusher(flushPendingMidi, FLUSH_INTERVAL);

  if (_midiScanProgressCleanup) _midiScanProgressCleanup();
  _midiScanProgressCleanup = window.vstUpdater.onMidiScanProgress((data) => {
    if (data.phase === 'scanning') {
      if (data.midiFiles) pendingMidi.push(...data.midiFiles);
      pendingFound = data.found || 0;
      syncMidiStatsBarCount(pendingFound);
      const elapsed = midiEta ? midiEta.elapsed() : '';
      const timeSuffix = elapsed ? ' — ' + elapsed : '';
      setBtn(`&#8635; ${pendingFound.toLocaleString()} found${timeSuffix}`, true);
      scheduleMidiFlush();
    }
  });

  try {
    const midiRoots = (overrideRoots && overrideRoots.length > 0)
      ? overrideRoots
      : (typeof prefs !== 'undefined' ? (prefs.getItem('midiScanDirs') || '') : '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanMidiFiles(midiRoots.length ? midiRoots : undefined, excludePaths);
    // Drain any remaining buffered batch that didn't hit the flush timer.
    flushPendingMidi();
    scanMidiDomActive = false;
    if (result.streamed) {
      // Backend streamed results live — allMidiFiles was built from progress events.
    } else {
      const files = result.midiFiles || [];
      if (resume) {
        allMidiFiles = [...allMidiFiles, ...files];
      } else {
        allMidiFiles = files;
      }
    }
    filteredMidi = allMidiFiles.slice();
    // Backend already streamed-saved when result.streamed
    if (!result.streamed) {
      try { await window.vstUpdater.saveMidiScan(allMidiFiles, result.roots); }
      catch (e) { if (typeof showToast === 'function' && typeof toastFmt === 'function') showToast(toastFmt('toast.failed_save_midi_history', { err: e.message || e }), 4000, 'error'); }
    }
    if (_midiScanProgressCleanup) { _midiScanProgressCleanup(); _midiScanProgressCleanup = null; }
    _midiScanDbView = false;
    _midiTableInit = false;
    _midiRenderCount = 0;
    _midiOffset = 0;
    // After scan, drop the in-memory streaming buffer and reload first page from DB.
    // The DB has authoritative data — JS memory stays bounded regardless of scan size.
    allMidiFiles = [];
    await fetchMidiPage();
    await refreshMidiStatsSnapshot(true);
    updateMidiHeaderCount();
    syncMidiStatsBarCount(_midiTotalUnfiltered);
    if (result.stopped && _midiTotalUnfiltered > 0 && resumeBtn) {
      resumeBtn.style.display = '';
    }
    if (typeof postScanCompleteToast === 'function') {
      const n = _midiTotalUnfiltered || 0;
      postScanCompleteToast(
        !!result.stopped,
        'toast.post_scan_midi_complete',
        'toast.post_scan_midi_stopped',
        { n: n.toLocaleString() },
      );
    }
  } catch (err) {
    if (_midiScanProgressCleanup) { _midiScanProgressCleanup(); _midiScanProgressCleanup = null; }
    _midiScanDbView = false;
    scanMidiDomActive = false;
    const errMsg = err.message || err || 'Unknown error';
    if (tableWrap) tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    if (typeof showToast === 'function' && typeof toastFmt === 'function') showToast(toastFmt('toast.midi_scan_failed', { err: errMsg }), 4000, 'error');
  }

  if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
  if (typeof btnLoading === 'function') btnLoading(btn, false);
  setBtn(
    '&#127924; ' + catalogFmt('ui.btn.127924_scan_midi'),
    false,
  );
  if (stopBtn) stopBtn.style.display = 'none';
  if (progressBar) progressBar.classList.remove('active');
  if (progressFill) {
    progressFill.style.width = '0%';
    progressFill.style.animation = '';
  }
}

function getMidiCount() {
  return _midiTotalUnfiltered;
}

function syncMidiStatsBarCount(total) {
  const n = typeof total === 'number' ? total : _midiTotalUnfiltered;
  window.__midiScanPendingFound = n;
  if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({ midi: n });
  else {
    const el = document.getElementById('midiScanCount');
    if (el) el.textContent = n.toLocaleString();
  }
}

function updateMidiCount() {
  const filtered = _midiTotalCount;
  const total = _midiTotalUnfiltered;
  const isFiltered = total > 0 && filtered < total;
  const totalEl = document.getElementById('midiTotalCount');
  if (totalEl) {
    totalEl.textContent = isFiltered
      ? filtered.toLocaleString() + ' / ' + total.toLocaleString()
      : (total || filtered).toLocaleString();
  }
  const count = document.getElementById('midiCount');
  if (count) count.textContent = (total || filtered).toLocaleString();
  const sizeEl = document.getElementById('midiTotalSize');
  if (sizeEl) {
    const bytes = _midiStatsSnapshot ? _midiStatsSnapshot.totalBytes : 0;
    sizeEl.textContent = typeof formatAudioSize === 'function' ? formatAudioSize(bytes) : Math.round(bytes / 1024) + ' KB';
  }
  const statsBar = document.getElementById('midiStats');
  if (statsBar && total > 0) statsBar.style.display = '';
}

function updateMidiHeaderCount() {
  window.__midiScanPendingFound = _midiTotalUnfiltered;
  if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({ midi: _midiTotalUnfiltered });
  else {
    const el = document.getElementById('headerMidi');
    if (el) el.textContent = _midiTotalUnfiltered.toLocaleString();
  }
}

let _midiSearch = '';

registerFilter('filterMidi', {
  inputId: 'midiSearchInput',
  regexToggleId: 'regexMidi',
  resetOffset() { _midiOffset = 0; _midiRenderCount = 0; },
  fetchFn() {
    _midiSearch = this.lastSearch || '';
    fetchMidiPage();
    refreshMidiStatsSnapshot();
  },
});
function filterMidi() { applyFilter('filterMidi'); }

/** Full list for export: after scan `allMidiFiles` is cleared and only paginated `filteredMidi` is loaded from DB. */
const _MIDI_EXPORT_MAX = 100000;
async function fetchMidiFilesForExport() {
  const search = _midiSearch || '';
  const total = Math.max(_midiTotalCount || 0, _midiTotalUnfiltered || 0);
  const n = Math.min(total, _MIDI_EXPORT_MAX);
  if (n <= 0) return [];
  const result = await window.vstUpdater.dbQueryMidi({
    search: search || null,
    format_filter: null,
    sort_key: midiSortKey,
    sort_asc: midiSortAsc,
    offset: 0,
    limit: n,
  });
  let files = result.midiFiles || [];
  if (search && files.length > 1 && typeof searchScore === 'function') {
    const mode = typeof getSearchMode === 'function' ? getSearchMode('regexMidi') : 'fuzzy';
    const scored = files.map((s) => ({ s, score: searchScore(search, [s.name, s.directory || ''], mode) }));
    scored.sort((a, b) => b.score - a.score);
    files = scored.map((x) => x.s);
  }
  const metadataKeys = new Set(['tracks', 'bpm', 'time', 'key', 'notes', 'ch', 'duration']);
  if (metadataKeys.has(midiSortKey)) {
    const saved = filteredMidi;
    filteredMidi = files;
    sortMidiArray();
    files = filteredMidi.slice();
    filteredMidi = saved;
  }
  return files;
}

function sortMidi(key) {
  if (midiSortKey === key) { midiSortAsc = !midiSortAsc; } else { midiSortKey = key; midiSortAsc = true; }
  ['Name', 'Tracks', 'Bpm', 'Time', 'Key', 'Notes', 'Ch', 'Duration', 'Size', 'Path'].forEach(k => {
    const el = document.getElementById('midiSortArrow' + k);
    if (el) {
      const isActive = k.toLowerCase() === midiSortKey;
      el.innerHTML = isActive ? (midiSortAsc ? '&#9650;' : '&#9660;') : '';
      el.closest('th')?.classList.toggle('sort-active', isActive);
    }
  });
  _midiOffset = 0;
  _midiRenderCount = 0;
  fetchMidiPage();
  if (typeof saveSortState === 'function') saveSortState('midi', midiSortKey, midiSortAsc);
}

function sortMidiArray() {
  filteredMidi.sort((a, b) => {
    let va, vb;
    const ai = _midiInfoCache[a.path] || {};
    const bi = _midiInfoCache[b.path] || {};
    switch (midiSortKey) {
      case 'name': va = a.name.toLowerCase(); vb = b.name.toLowerCase(); break;
      case 'tracks': va = ai.trackCount || 0; vb = bi.trackCount || 0; break;
      case 'bpm': va = ai.tempo || 0; vb = bi.tempo || 0; break;
      case 'time': va = ai.timeSignature || ''; vb = bi.timeSignature || ''; break;
      case 'key': va = ai.keySignature || ''; vb = bi.keySignature || ''; break;
      case 'notes': va = ai.noteCount || 0; vb = bi.noteCount || 0; break;
      case 'ch': va = ai.channelsUsed || 0; vb = bi.channelsUsed || 0; break;
      case 'duration': va = ai.duration || 0; vb = bi.duration || 0; break;
      case 'size': va = a.size || 0; vb = b.size || 0; break;
      case 'path': va = a.directory.toLowerCase(); vb = b.directory.toLowerCase(); break;
      default: va = a.name.toLowerCase(); vb = b.name.toLowerCase();
    }
    if (va < vb) return midiSortAsc ? -1 : 1;
    if (va > vb) return midiSortAsc ? 1 : -1;
    return 0;
  });
}

function renderMidiTable() {
  clearTableQueryLoadingRow('midiQueryLoadingRow', 'midiTable');
  const wrap = document.getElementById('midiTableWrap');
  if (!wrap) return;
  const hasAny = filteredMidi.length > 0 || allMidiFiles.length > 0 || _midiTotalUnfiltered > 0;
  if (!hasAny) {
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
    const h2 = esc(_midiFmt('ui.h2.midi_index'));
    const p = esc(_midiFmt('ui.midi.empty_state'));
    wrap.innerHTML = `<div class="state-message" id="midiEmptyState">
      <div class="state-icon">&#127924;</div>
      <h2>${h2}</h2>
      <p>${p}</p>
    </div>`;
    _midiTableInit = false;
    return;
  }
  if (!document.getElementById('midiTable')) {
    mountMidiTableShell();
  }
  const tbody = document.getElementById('midiTableBody');
  if (!tbody) return;
  // Page-at-a-time: offset=0 replaces DOM, subsequent pages append. Matches audio.js.
  // During active scan, filteredMidi is the cumulative streaming buffer and _midiOffset
  // stays 0 — so we do a full replace every flush (existing scan behavior preserved).
  const streaming = !!_midiScanProgressCleanup && !_midiScanDbView;
  _midiRenderCount = streaming ? filteredMidi.length : (_midiOffset + filteredMidi.length);
  if (streaming || _midiOffset === 0) {
    tbody.innerHTML = filteredMidi.map(buildMidiRow).join('');
  } else {
    const more = document.getElementById('midiLoadMore');
    if (more) more.remove();
    tbody.insertAdjacentHTML('beforeend', filteredMidi.map(buildMidiRow).join(''));
  }
  const total = streaming ? filteredMidi.length : _midiTotalCount;
  if (_midiRenderCount < total) {
    appendMidiLoadMoreRow(tbody);
  }
  if (!_midiMetadataRunning) loadMidiMetadata();
}

function appendMidiLoadMoreRow(tbody) {
  const total = (_midiScanProgressCleanup && !_midiScanDbView) ? filteredMidi.length : _midiTotalCount;
  const line = catalogFmt('ui.js.load_more_hint', {
    shown: _midiRenderCount.toLocaleString(),
    total: total.toLocaleString(),
  });
  tbody.insertAdjacentHTML('beforeend',
    `<tr id="midiLoadMore"><td colspan="12" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMoreMidi">
      ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
    </td></tr>`);
}

function loadMoreMidi() {
  // During scan, still render from in-memory filteredMidi (scan stream). Post-scan,
  // advance DB offset and fetch next page.
  if (_midiScanProgressCleanup && !_midiScanDbView) {
    const MIDI_PAGE = 200;
    const tbody = document.getElementById('midiTableBody');
    const more = document.getElementById('midiLoadMore');
    if (more) more.remove();
    const next = filteredMidi.slice(_midiRenderCount, _midiRenderCount + MIDI_PAGE);
    tbody.insertAdjacentHTML('beforeend', next.map(buildMidiRow).join(''));
    _midiRenderCount += next.length;
    if (_midiRenderCount < filteredMidi.length) {
      appendMidiLoadMoreRow(tbody);
    }
    if (!_midiMetadataRunning) loadMidiMetadata();
    return;
  }
  _midiOffset = _midiRenderCount;
  fetchMidiPage();
}

function buildMidiRow(s) {
  const hp = typeof escapeHtml === 'function' ? escapeHtml(s.path) : s.path;
  const hn = typeof escapeHtml === 'function' ? escapeHtml(s.name) : s.name;
  const info = _midiInfoCache[s.path];
  const dur = info && info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : '';
  const trackNames = info && info.trackNames && info.trackNames.length > 0 ? info.trackNames.join(', ') : '';
  const checked = typeof batchSelected !== 'undefined' && batchSelected.has(s.path) ? ' checked' : '';
  const rowTitle = trackNames
    ? (typeof escapeHtml === 'function'
      ? escapeHtml(_midiFmt('ui.midi.tracks_tooltip', { names: trackNames }))
      : _midiFmt('ui.midi.tracks_tooltip', { names: trackNames }))
    : '';
  const revealT = typeof escapeHtml === 'function'
    ? escapeHtml(_midiFmt('menu.reveal_in_finder'))
    : _midiFmt('menu.reveal_in_finder');
  return `<tr data-midi-path="${hp}" title="${rowTitle}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${hn}">${_midiSearch && typeof highlightMatch === 'function' ? highlightMatch(s.name, _midiSearch, 'fuzzy') : hn}${typeof rowBadges === 'function' ? rowBadges(s.path) : ''}</td>
    <td style="text-align:center;">${info ? info.trackCount : ''}</td>
    <td style="text-align:center;color:var(--cyan);">${info ? info.tempo : ''}</td>
    <td style="text-align:center;">${info ? info.timeSignature : ''}</td>
    <td style="text-align:center;color:var(--accent);">${info ? (typeof escapeHtml === 'function' ? escapeHtml(info.keySignature) : info.keySignature) : ''}</td>
    <td style="text-align:right;">${info ? info.noteCount.toLocaleString() : ''}</td>
    <td style="text-align:center;">${info ? info.channelsUsed : ''}</td>
    <td style="text-align:center;">${dur}</td>
    <td class="col-size">${s.sizeFormatted}</td>
    <td class="col-path" title="${hp}">${_midiSearch && typeof highlightMatch === 'function' ? highlightMatch(s.directory, _midiSearch, 'fuzzy') : (typeof escapeHtml === 'function' ? escapeHtml(s.directory) : s.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${hp}" title="${revealT}">&#128193;</button>
    </td>
  </tr>`;
}

async function loadMidiMetadata() {
  if (_midiMetadataRunning) return;
  _midiMetadataRunning = true;
  for (const s of filteredMidi) {
    if (_midiInfoCache[s.path]) continue;
    try {
      const info = await window.vstUpdater.getMidiInfo(s.path);
      if (info) {
        _midiInfoCache[s.path] = info;
        const row = document.querySelector(`[data-midi-path="${CSS.escape(s.path)}"]`);
        if (row) {
          const c = row.cells;
          if (c.length >= 11) {
            c[2].textContent = info.trackCount;
            c[3].textContent = info.tempo;
            c[4].textContent = info.timeSignature;
            c[5].textContent = info.keySignature;
            c[6].textContent = info.noteCount.toLocaleString();
            c[7].textContent = info.channelsUsed;
            c[8].textContent = info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : '';
            if (info.trackNames && info.trackNames.length > 0) row.title = 'Tracks: ' + info.trackNames.join(', ');
          }
        }
      }
    } catch(e) { /* skip individual file errors silently */ }
    await new Promise(r => setTimeout(r, 5));
  }
  _midiMetadataRunning = false;
}

// Restore sort state on init
function restoreMidiSortState() {
  if (typeof restoreSortState === 'function') {
    const saved = restoreSortState('midi');
    if (saved) { midiSortKey = saved.key; midiSortAsc = saved.asc; }
  }
}
restoreMidiSortState();

// Event handlers — input handled by delegated handler in ipc.js via data-action="filterMidi"
document.addEventListener('click', (e) => {
  const sortBtn = e.target.closest('[data-action="sortMidi"]');
  if (sortBtn && sortBtn.dataset.key) {
    e.preventDefault();
    sortMidi(sortBtn.dataset.key);
  }
});
