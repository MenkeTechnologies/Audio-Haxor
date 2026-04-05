// ── MIDI Tab ──
// Dedicated tab for MIDI files with sortable/draggable columns and MIDI-specific metadata.

function _midiFmt(key, vars) {
  if (typeof appFmt !== 'function') return key;
  return vars ? appFmt(key, vars) : appFmt(key);
}

let allMidiFiles = [];
let filteredMidi = [];
let _midiInfoCache = {};
let _midiLoaded = false;
let _midiTableInit = false;
let _midiRenderCount = 0;
let _midiMetadataRunning = false;
let midiSortKey = 'name';
let midiSortAsc = true;

async function loadMidiFiles() {
  try {
    // Load from the MIDI scanner's own DB — completely independent of presets.
    const latest = await window.vstUpdater.getLatestMidiScan();
    if (latest && latest.midiFiles) {
      allMidiFiles = latest.midiFiles;
    } else {
      allMidiFiles = [];
    }
    filteredMidi = allMidiFiles.slice();
    _midiLoaded = true;
    _midiTableInit = false;
    _midiRenderCount = 0;
    sortMidiArray();
    renderMidiTable();
    updateMidiCount();
    updateMidiHeaderCount();
    syncMidiStatsBarCount();
  } catch (e) {
    if (typeof showToast === 'function') showToast(toastFmt('toast.midi_load_failed', { err: e.message || e }), 4000, 'error');
  }
}

// ── MIDI scanner — fully independent from preset scanner ──
let _midiScanProgressCleanup = null;

async function stopMidiScan() {
  try { await window.vstUpdater.stopMidiScan(); } catch (e) { /* ignore */ }
}

async function scanMidi(resume = false) {
  if (typeof showGlobalProgress === 'function') showGlobalProgress();
  const btn = document.getElementById('btnScanMidi');
  const resumeBtn = document.getElementById('btnResumeMidi');
  const stopBtn = document.getElementById('btnStopMidi');
  const progressBar = document.getElementById('midiProgressBar');
  const progressFill = document.getElementById('midiProgressFill');
  const tableWrap = document.getElementById('midiTableWrap');
  const setBtn = (html, disabled) => { if (btn) { btn.innerHTML = html; btn.disabled = disabled; } };

  const excludePaths = resume ? allMidiFiles.map(m => m.path) : null;

  setBtn(resume ? '&#8635; Resuming...' : '&#8635; Scanning...', true);
  if (resumeBtn) resumeBtn.style.display = 'none';
  if (stopBtn) stopBtn.style.display = '';
  if (progressBar) progressBar.classList.add('active');
  if (progressFill) progressFill.style.width = '0%';

  if (!resume) {
    allMidiFiles = [];
    filteredMidi = [];
    _midiInfoCache = {};
    _midiRenderCount = 0;
    _midiTableInit = false;
    if (tableWrap) tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for MIDI files...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let pendingMidi = [];
  let pendingFound = 0;
  const midiEta = typeof createETA === 'function' ? createETA() : null;
  if (midiEta) midiEta.start();

  if (_midiScanProgressCleanup) _midiScanProgressCleanup();
  _midiScanProgressCleanup = window.vstUpdater.onMidiScanProgress((data) => {
    if (data.phase === 'scanning') {
      if (data.midiFiles) pendingMidi.push(...data.midiFiles);
      pendingFound = data.found || 0;
      syncMidiStatsBarCount(pendingFound);
      const elapsed = midiEta ? midiEta.elapsed() : '';
      const timeSuffix = elapsed ? ' — ' + elapsed : '';
      setBtn(`&#8635; ${pendingFound.toLocaleString()} found${timeSuffix}`, true);
    }
  });

  try {
    const midiRoots = (typeof prefs !== 'undefined' ? (prefs.getItem('midiScanDirs') || '') : '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanMidiFiles(midiRoots.length ? midiRoots : undefined, excludePaths);
    const files = result.midiFiles || [];
    if (resume) {
      allMidiFiles = [...allMidiFiles, ...files];
    } else {
      allMidiFiles = files;
    }
    filteredMidi = allMidiFiles.slice();
    // Save to DB — the scan isn't persistent until this runs.
    if (!result.stopped) {
      try { await window.vstUpdater.saveMidiScan(allMidiFiles, result.roots); }
      catch (e) { if (typeof showToast === 'function') showToast('Failed to save MIDI history: ' + (e.message || e), 4000, 'error'); }
    }
    if (_midiScanProgressCleanup) { _midiScanProgressCleanup(); _midiScanProgressCleanup = null; }
    _midiTableInit = false;
    _midiRenderCount = 0;
    sortMidiArray();
    renderMidiTable();
    updateMidiCount();
    updateMidiHeaderCount();
    syncMidiStatsBarCount();
    if (result.stopped && allMidiFiles.length > 0 && resumeBtn) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (_midiScanProgressCleanup) { _midiScanProgressCleanup(); _midiScanProgressCleanup = null; }
    const errMsg = err.message || err || 'Unknown error';
    if (tableWrap) tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    if (typeof showToast === 'function') showToast('MIDI scan failed: ' + errMsg, 4000, 'error');
  }

  if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
  setBtn('&#127924; Scan MIDI', false);
  if (stopBtn) stopBtn.style.display = 'none';
  if (progressBar) progressBar.classList.remove('active');
  if (progressFill) progressFill.style.width = '0%';
}

function getMidiCount() { return allMidiFiles.length; }

/** Stats bar "MIDI Found" — must not rely on preset scan (MIDI has its own walker/DB). */
function syncMidiStatsBarCount(total) {
  const el = document.getElementById('midiScanCount');
  if (!el) return;
  const n = typeof total === 'number' ? total : allMidiFiles.length;
  el.textContent = n.toLocaleString();
}

function updateMidiCount() {
  const count = document.getElementById('midiCount');
  if (count) count.textContent = filteredMidi.length;
  const total = document.getElementById('midiTotalCount');
  if (total) total.textContent = allMidiFiles.length;
  const sizeEl = document.getElementById('midiTotalSize');
  if (sizeEl) {
    const bytes = allMidiFiles.reduce((s, f) => s + (f.size || 0), 0);
    sizeEl.textContent = typeof formatAudioSize === 'function' ? formatAudioSize(bytes) : Math.round(bytes / 1024) + ' KB';
  }
  const statsBar = document.getElementById('midiStats');
  if (statsBar && allMidiFiles.length > 0) statsBar.style.display = '';
}

function updateMidiHeaderCount() {
  const el = document.getElementById('headerMidi');
  if (el) el.textContent = allMidiFiles.length;
}

let _midiSearch = '';

registerFilter('filterMidi', {
  inputId: 'midiSearchInput',
  regexToggleId: 'regexMidi',
  resetOffset() { _midiRenderCount = 0; },
  fetchFn() {
    const q = this.lastSearch || '';
    const mode = this.lastMode || 'fuzzy';
    _midiSearch = q;
    if (!q) {
      filteredMidi = allMidiFiles.slice();
    } else {
      const scored = allMidiFiles
        .map(s => ({ s, score: searchScore(q, [s.name, s.directory || ''], mode) }))
        .filter(x => x.score > 0);
      scored.sort((a, b) => b.score - a.score);
      filteredMidi = scored.map(x => x.s);
    }
    sortMidiArray();
    _midiTableInit = false;
    renderMidiTable();
    updateMidiCount();
  },
});
function filterMidi() { applyFilter('filterMidi'); }

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
  sortMidiArray();
  _midiTableInit = false;
  _midiRenderCount = 0;
  renderMidiTable();
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
  const wrap = document.getElementById('midiTableWrap');
  if (!wrap) return;
  if (filteredMidi.length === 0 && allMidiFiles.length === 0) {
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
    const h2 = esc(_midiFmt('ui.h2.midi_index'));
    const p = esc(_midiFmt('ui.midi.empty_state'));
    wrap.innerHTML = `<div class="state-message" id="midiEmptyState">
      <div class="state-icon">&#127924;</div>
      <h2>${h2}</h2>
      <p>${p}</p>
    </div>`;
    return;
  }
  if (!_midiTableInit) {
    _midiTableInit = true;
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
  }
  const MIDI_PAGE = 200;
  const tbody = document.getElementById('midiTableBody');
  if (!tbody) return;
  tbody.innerHTML = filteredMidi.slice(0, MIDI_PAGE).map(buildMidiRow).join('');
  _midiRenderCount = Math.min(filteredMidi.length, MIDI_PAGE);
  if (_midiRenderCount < filteredMidi.length) {
    appendMidiLoadMoreRow(tbody);
  }
  if (!_midiMetadataRunning) loadMidiMetadata();
}

function appendMidiLoadMoreRow(tbody) {
  const line = typeof appFmt === 'function'
    ? appFmt('ui.js.load_more_hint', {
        shown: _midiRenderCount.toLocaleString(),
        total: filteredMidi.length.toLocaleString(),
      })
    : `Showing ${_midiRenderCount} of ${filteredMidi.length} — click to load more`;
  tbody.insertAdjacentHTML('beforeend',
    `<tr id="midiLoadMore"><td colspan="12" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMoreMidi">
      ${typeof escapeHtml === 'function' ? escapeHtml(line) : line}
    </td></tr>`);
}

function loadMoreMidi() {
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
    <td class="col-path" title="${hp}">${typeof escapeHtml === 'function' ? escapeHtml(s.directory) : s.directory}</td>
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
