// ── MIDI Tab ──
// Shows MIDI files with MIDI-specific metadata (tempo, key, notes, tracks).
// Queries SQLite directly with format filter for MID/MIDI.

let allMidiFiles = [];
let filteredMidi = [];
let _midiInfoCache = {};
let _midiLoaded = false;

async function loadMidiFiles() {
  try {
    // Query all MIDI files from the database
    const result = await window.vstUpdater.dbQueryAudio({
      format_filter: 'MID',
      sort_key: 'name',
      sort_asc: true,
      offset: 0,
      limit: 50000,
    });
    allMidiFiles = result.samples || [];
    // Also try MIDI extension
    const result2 = await window.vstUpdater.dbQueryAudio({
      format_filter: 'MIDI',
      sort_key: 'name',
      sort_asc: true,
      offset: 0,
      limit: 50000,
    });
    if (result2.samples && result2.samples.length > 0) {
      const paths = new Set(allMidiFiles.map(s => s.path));
      for (const s of result2.samples) {
        if (!paths.has(s.path)) allMidiFiles.push(s);
      }
    }
    filteredMidi = allMidiFiles;
    _midiLoaded = true;
    renderMidiTable();
    updateMidiCount();
  } catch (e) {
    console.warn('MIDI load error:', e);
  }
}

function updateMidiCount() {
  const count = document.getElementById('midiCount');
  if (count) count.textContent = `${filteredMidi.length}${filteredMidi.length !== allMidiFiles.length ? ' of ' + allMidiFiles.length : ''} MIDI files`;
}

function filterMidi() {
  const input = document.getElementById('midiSearchInput');
  const q = input ? input.value.trim() : '';
  if (!q) {
    filteredMidi = allMidiFiles;
  } else if (typeof fzfFilter === 'function') {
    filteredMidi = fzfFilter(allMidiFiles, q, ['name', 'directory'], 'fuzzy');
  } else {
    const ql = q.toLowerCase();
    filteredMidi = allMidiFiles.filter(s => s.name.toLowerCase().includes(ql) || s.directory.toLowerCase().includes(ql));
  }
  renderMidiTable();
  updateMidiCount();
}

function renderMidiTable() {
  const wrap = document.getElementById('midiTableWrap');
  if (!wrap) return;
  if (filteredMidi.length === 0) {
    wrap.innerHTML = '<div style="text-align:center;padding:40px;color:var(--text-dim);">No MIDI files found. Run an audio scan to discover .mid files.</div>';
    return;
  }
  wrap.innerHTML = `<table class="audio-table" id="midiTable">
    <thead>
      <tr>
        <th style="width:25%;">Name<span class="col-resize"></span></th>
        <th style="width:55px;">Tracks<span class="col-resize"></span></th>
        <th style="width:65px;">BPM<span class="col-resize"></span></th>
        <th style="width:55px;">Time<span class="col-resize"></span></th>
        <th style="width:80px;">Key<span class="col-resize"></span></th>
        <th style="width:60px;">Notes<span class="col-resize"></span></th>
        <th style="width:45px;">Ch<span class="col-resize"></span></th>
        <th style="width:65px;">Duration<span class="col-resize"></span></th>
        <th style="width:60px;">Size<span class="col-resize"></span></th>
        <th style="width:25%;">Path<span class="col-resize"></span></th>
      </tr>
    </thead>
    <tbody id="midiTableBody"></tbody>
  </table>`;
  const tbody = document.getElementById('midiTableBody');
  tbody.innerHTML = filteredMidi.map(buildMidiRow).join('');
  if (typeof initColumnResize === 'function') initColumnResize(document.getElementById('midiTable'));
  loadMidiMetadata();
}

function buildMidiRow(s) {
  const hp = typeof escapeHtml === 'function' ? escapeHtml(s.path) : s.path;
  const hn = typeof escapeHtml === 'function' ? escapeHtml(s.name) : s.name;
  const info = _midiInfoCache[s.path];
  return `<tr data-midi-path="${hp}" data-action="openAudioFolder" data-path="${hp}">
    <td class="col-name" title="${hn}">${hn}${typeof rowBadges === 'function' ? rowBadges(s.path) : ''}</td>
    <td style="text-align:center;">${info ? info.trackCount : '<span class="spinner" style="width:10px;height:10px;"></span>'}</td>
    <td style="text-align:center;color:var(--cyan);">${info ? info.tempo : ''}</td>
    <td style="text-align:center;">${info ? info.timeSignature : ''}</td>
    <td style="text-align:center;color:var(--accent);">${info ? (typeof escapeHtml === 'function' ? escapeHtml(info.keySignature) : info.keySignature) : ''}</td>
    <td style="text-align:right;">${info ? info.noteCount.toLocaleString() : ''}</td>
    <td style="text-align:center;">${info ? info.channelsUsed : ''}</td>
    <td style="text-align:center;">${info && info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : ''}</td>
    <td class="col-size">${s.sizeFormatted}</td>
    <td class="col-path" title="${hp}">${typeof escapeHtml === 'function' ? escapeHtml(s.directory) : s.directory}</td>
  </tr>`;
}

async function loadMidiMetadata() {
  for (const s of filteredMidi) {
    if (_midiInfoCache[s.path]) continue;
    try {
      const info = await window.vstUpdater.getMidiInfo(s.path);
      if (info) {
        _midiInfoCache[s.path] = info;
        const row = document.querySelector(`[data-midi-path="${CSS.escape(s.path)}"]`);
        if (row) {
          const c = row.cells;
          c[1].textContent = info.trackCount;
          c[2].textContent = info.tempo;
          c[3].textContent = info.timeSignature;
          c[4].textContent = info.keySignature;
          c[5].textContent = info.noteCount.toLocaleString();
          c[6].textContent = info.channelsUsed;
          c[7].textContent = info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : '';
          if (info.trackNames && info.trackNames.length > 0) {
            row.title = 'Tracks: ' + info.trackNames.join(', ');
          }
        }
      }
    } catch {}
    await new Promise(r => setTimeout(r, 5));
  }
}

// Filter input handler
document.addEventListener('input', (e) => {
  if (e.target.id === 'midiSearchInput') filterMidi();
});
