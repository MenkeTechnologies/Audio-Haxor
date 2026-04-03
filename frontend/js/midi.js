// ── MIDI Tab ──
// Shows MIDI files from the audio scan with MIDI-specific metadata columns.

let allMidiFiles = [];
let filteredMidi = [];
let _midiInfoCache = {};

function extractMidiFiles() {
  if (typeof allAudioSamples === 'undefined') return;
  allMidiFiles = allAudioSamples.filter(s => s.format === 'MID' || s.format === 'MIDI');
  filteredMidi = allMidiFiles;
  renderMidiTable();
  const count = document.getElementById('midiCount');
  if (count) count.textContent = `${allMidiFiles.length} MIDI files`;
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
  const count = document.getElementById('midiCount');
  if (count) count.textContent = `${filteredMidi.length} of ${allMidiFiles.length} MIDI files`;
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
        <th style="width:25%;">Name</th>
        <th style="width:60px;">Tracks</th>
        <th style="width:70px;">BPM</th>
        <th style="width:60px;">Time</th>
        <th style="width:80px;">Key</th>
        <th style="width:60px;">Notes</th>
        <th style="width:55px;">Ch</th>
        <th style="width:70px;">Duration</th>
        <th style="width:65px;">Size</th>
        <th style="width:25%;">Path</th>
      </tr>
    </thead>
    <tbody id="midiTableBody"></tbody>
  </table>`;
  const tbody = document.getElementById('midiTableBody');
  tbody.innerHTML = filteredMidi.map(buildMidiRow).join('');
  // Lazy-load MIDI metadata for visible rows
  loadMidiMetadata();
}

function buildMidiRow(s) {
  const hp = typeof escapeHtml === 'function' ? escapeHtml(s.path) : s.path;
  const info = _midiInfoCache[s.path];
  const tracks = info ? info.trackCount : '';
  const bpm = info ? info.tempo : '';
  const timeSig = info ? info.timeSignature : '';
  const key = info ? info.keySignature : '';
  const notes = info ? info.noteCount.toLocaleString() : '';
  const ch = info ? info.channelsUsed : '';
  const dur = info && info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : '';
  const trackNames = info && info.trackNames && info.trackNames.length > 0 ? info.trackNames.join(', ') : '';
  return `<tr data-midi-path="${hp}" title="${trackNames ? 'Tracks: ' + (typeof escapeHtml === 'function' ? escapeHtml(trackNames) : trackNames) : ''}">
    <td class="col-name" title="${typeof escapeHtml === 'function' ? escapeHtml(s.name) : s.name}">${typeof highlightMatch === 'function' ? highlightMatch(s.name, document.getElementById('midiSearchInput')?.value || '', 'fuzzy') : (typeof escapeHtml === 'function' ? escapeHtml(s.name) : s.name)}</td>
    <td style="text-align:center;">${tracks}</td>
    <td style="text-align:center;color:var(--cyan);">${bpm}</td>
    <td style="text-align:center;">${timeSig}</td>
    <td style="text-align:center;color:var(--accent);">${typeof escapeHtml === 'function' ? escapeHtml(key) : key}</td>
    <td style="text-align:right;">${notes}</td>
    <td style="text-align:center;">${ch}</td>
    <td style="text-align:center;">${dur}</td>
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
        // Update the row in-place
        const row = document.querySelector(`[data-midi-path="${CSS.escape(s.path)}"]`);
        if (row) {
          const cells = row.cells;
          cells[1].textContent = info.trackCount;
          cells[2].textContent = info.tempo;
          cells[3].textContent = info.timeSignature;
          cells[4].textContent = info.keySignature;
          cells[5].textContent = info.noteCount.toLocaleString();
          cells[6].textContent = info.channelsUsed;
          cells[7].textContent = info.duration ? (typeof formatTime === 'function' ? formatTime(info.duration) : info.duration.toFixed(1) + 's') : '';
          if (info.trackNames && info.trackNames.length > 0) {
            row.title = 'Tracks: ' + info.trackNames.join(', ');
          }
        }
      }
    } catch {}
    // Yield to UI
    await new Promise(r => setTimeout(r, 5));
  }
}

// Re-extract when audio scan completes
if (typeof document !== 'undefined') {
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action="filterMidi"]');
    if (btn || e.target.id === 'midiSearchInput') return;
  });
  document.addEventListener('input', (e) => {
    if (e.target.id === 'midiSearchInput') filterMidi();
  });
}
