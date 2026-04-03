// ── MIDI Tab ──
// Dedicated tab for MIDI files with sortable/draggable columns and MIDI-specific metadata.

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
    const midiFormats = new Set(['MID', 'MIDI']);
    if (typeof allPresets !== 'undefined' && allPresets.length > 0) {
      allMidiFiles = allPresets.filter(p => midiFormats.has(p.format));
    } else {
      const latest = await window.vstUpdater.getLatestPresetScan();
      if (latest && latest.presets) {
        allMidiFiles = latest.presets.filter(p => midiFormats.has(p.format));
      }
    }
    filteredMidi = allMidiFiles.slice();
    _midiLoaded = true;
    _midiTableInit = false;
    _midiRenderCount = 0;
    sortMidiArray();
    renderMidiTable();
    updateMidiCount();
    updateMidiHeaderCount();
  } catch (e) {
    console.warn('MIDI load error:', e);
  }
}

function getMidiCount() { return allMidiFiles.length; }

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

function filterMidi() {
  const input = document.getElementById('midiSearchInput');
  const q = input ? input.value.trim() : '';
  if (!q) {
    filteredMidi = allMidiFiles.slice();
  } else if (typeof fzfFilter === 'function') {
    filteredMidi = fzfFilter(allMidiFiles, q, ['name', 'directory'], 'fuzzy');
  } else {
    const ql = q.toLowerCase();
    filteredMidi = allMidiFiles.filter(s => s.name.toLowerCase().includes(ql) || s.directory.toLowerCase().includes(ql));
  }
  sortMidiArray();
  _midiTableInit = false;
  _midiRenderCount = 0;
  renderMidiTable();
  updateMidiCount();
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
    wrap.innerHTML = '<div style="text-align:center;padding:40px;color:var(--text-dim);">No MIDI files found. Run a preset scan to discover .mid files.</div>';
    return;
  }
  if (!_midiTableInit) {
    _midiTableInit = true;
    _midiRenderCount = 0;
    const arrow = (k) => `<span class="sort-arrow" id="midiSortArrow${k}">${midiSortKey === k.toLowerCase() ? (midiSortAsc ? '&#9650;' : '&#9660;') : ''}</span>`;
    wrap.innerHTML = `<table class="audio-table" id="midiTable">
      <thead>
        <tr>
          <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="Select all"></th>
          <th data-action="sortMidi" data-key="name" style="width:22%;" title="File name">Name ${arrow('Name')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="tracks" style="width:55px;" title="Track count">Tracks ${arrow('Tracks')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="bpm" style="width:65px;" title="Tempo (BPM)">BPM ${arrow('Bpm')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="time" style="width:55px;" title="Time signature">Time ${arrow('Time')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="key" style="width:80px;" title="Key signature">Key ${arrow('Key')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="notes" style="width:60px;" title="Note count">Notes ${arrow('Notes')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="ch" style="width:45px;" title="MIDI channels used">Ch ${arrow('Ch')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="duration" style="width:65px;" title="Duration">Dur ${arrow('Duration')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="size" style="width:60px;" title="File size">Size ${arrow('Size')}<span class="col-resize"></span></th>
          <th data-action="sortMidi" data-key="path" style="width:22%;" title="Directory path">Path ${arrow('Path')}<span class="col-resize"></span></th>
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
    tbody.insertAdjacentHTML('beforeend',
      `<tr id="midiLoadMore"><td colspan="12" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMoreMidi">
        Showing ${_midiRenderCount} of ${filteredMidi.length} — click to load more
      </td></tr>`);
  }
  if (!_midiMetadataRunning) loadMidiMetadata();
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
    tbody.insertAdjacentHTML('beforeend',
      `<tr id="midiLoadMore"><td colspan="12" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;" data-action="loadMoreMidi">
        Showing ${_midiRenderCount} of ${filteredMidi.length} — click to load more
      </td></tr>`);
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
  return `<tr data-midi-path="${hp}" title="${trackNames ? 'Tracks: ' + (typeof escapeHtml === 'function' ? escapeHtml(trackNames) : trackNames) : ''}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${hn}">${hn}${typeof rowBadges === 'function' ? rowBadges(s.path) : ''}</td>
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
      <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${hp}" title="Reveal in Finder">&#128193;</button>
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
    } catch(e) { if(typeof showToast==='function'&&e) showToast(String(e),4000,'error'); }
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

// Event handlers
document.addEventListener('input', (e) => {
  if (e.target.id === 'midiSearchInput') filterMidi();
});
document.addEventListener('click', (e) => {
  const sortBtn = e.target.closest('[data-action="sortMidi"]');
  if (sortBtn && sortBtn.dataset.key) {
    e.preventDefault();
    sortMidi(sortBtn.dataset.key);
  }
});
