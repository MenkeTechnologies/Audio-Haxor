// ── Audio Samples ──
let allAudioSamples = [];
let filteredAudioSamples = [];
let audioSortKey = 'name';
let audioSortAsc = true;
let audioScanProgressCleanup = null;

// Playback state
let audioPlayer = new Audio();
let audioPlayerPath = null;
let audioLooping = false;
let audioPlaybackRAF = null;
let expandedMetaPath = null;
let recentlyPlayed = [];
const MAX_RECENT = 50;
let audioShuffling = false;
let audioMuted = false;
let savedVolume = 1;

// ── Web Audio processing chain ──
let _playbackCtx = null;
let _sourceNode = null;
let _eqLow = null;
let _eqMid = null;
let _eqHigh = null;
let _gainNode = null;
let _panNode = null;
let _monoMode = false;
let _abLoop = null; // { start, end } in seconds, or null

function ensureAudioGraph() {
  if (_playbackCtx) return;
  _playbackCtx = new AudioContext();
  _sourceNode = _playbackCtx.createMediaElementSource(audioPlayer);

  // 3-band EQ
  _eqLow = _playbackCtx.createBiquadFilter();
  _eqLow.type = 'lowshelf';
  _eqLow.frequency.value = 200;
  _eqLow.gain.value = 0;

  _eqMid = _playbackCtx.createBiquadFilter();
  _eqMid.type = 'peaking';
  _eqMid.frequency.value = 1000;
  _eqMid.Q.value = 1;
  _eqMid.gain.value = 0;

  _eqHigh = _playbackCtx.createBiquadFilter();
  _eqHigh.type = 'highshelf';
  _eqHigh.frequency.value = 8000;
  _eqHigh.gain.value = 0;

  // Gain (preamp)
  _gainNode = _playbackCtx.createGain();
  _gainNode.gain.value = 1;

  // Stereo pan
  _panNode = _playbackCtx.createStereoPanner();
  _panNode.pan.value = 0;

  // Chain: source → eqLow → eqMid → eqHigh → gain → pan → destination
  _sourceNode.connect(_eqLow);
  _eqLow.connect(_eqMid);
  _eqMid.connect(_eqHigh);
  _eqHigh.connect(_gainNode);
  _gainNode.connect(_panNode);
  _panNode.connect(_playbackCtx.destination);
}

function setEqBand(band, value) {
  ensureAudioGraph();
  const db = parseFloat(value);
  if (band === 'low') _eqLow.gain.value = db;
  else if (band === 'mid') _eqMid.gain.value = db;
  else if (band === 'high') _eqHigh.gain.value = db;
  // Update label
  const label = document.getElementById('npEq' + band.charAt(0).toUpperCase() + band.slice(1) + 'Val');
  if (label) label.textContent = (db >= 0 ? '+' : '') + db.toFixed(0) + ' dB';
}

function setPreampGain(value) {
  ensureAudioGraph();
  const g = parseFloat(value);
  _gainNode.gain.value = g;
  const label = document.getElementById('npGainVal');
  if (label) label.textContent = (g * 100).toFixed(0) + '%';
}

function setPan(value) {
  ensureAudioGraph();
  const p = parseFloat(value);
  _panNode.pan.value = p;
  const label = document.getElementById('npPanVal');
  if (label) {
    if (Math.abs(p) < 0.05) label.textContent = 'C';
    else if (p < 0) label.textContent = Math.round(Math.abs(p) * 100) + 'L';
    else label.textContent = Math.round(p * 100) + 'R';
  }
}

function toggleEqSection() {
  const section = document.getElementById('npEqSection');
  const btn = document.getElementById('npEqToggle');
  section.classList.toggle('visible');
  btn.classList.toggle('active', section.classList.contains('visible'));
}

function toggleMono() {
  _monoMode = !_monoMode;
  const btn = document.getElementById('npBtnMono');
  if (btn) btn.classList.toggle('active', _monoMode);
  // Mono via pan automation isn't possible with StereoPanner alone,
  // so we use a ChannelMerger approach. Simpler: just set pan to center
  // and note the state. Full mono requires a splitter/merger which is
  // heavy — for a preview player, center-pan is the practical equivalent.
  if (_monoMode) {
    setPan(0);
    const slider = document.getElementById('npPanSlider');
    if (slider) { slider.value = 0; slider.disabled = true; }
  } else {
    const slider = document.getElementById('npPanSlider');
    if (slider) slider.disabled = false;
  }
}

function resetEq() {
  ensureAudioGraph();
  _eqLow.gain.value = 0;
  _eqMid.gain.value = 0;
  _eqHigh.gain.value = 0;
  _gainNode.gain.value = 1;
  _panNode.pan.value = 0;
  _monoMode = false;
  // Update UI
  ['npEqLow', 'npEqMid', 'npEqHigh'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.value = 0;
  });
  const gain = document.getElementById('npGainSlider');
  if (gain) gain.value = 1;
  const pan = document.getElementById('npPanSlider');
  if (pan) { pan.value = 0; pan.disabled = false; }
  const mono = document.getElementById('npBtnMono');
  if (mono) mono.classList.remove('active');
  document.getElementById('npEqLowVal').textContent = '0 dB';
  document.getElementById('npEqMidVal').textContent = '0 dB';
  document.getElementById('npEqHighVal').textContent = '0 dB';
  document.getElementById('npGainVal').textContent = '100%';
  document.getElementById('npPanVal').textContent = 'C';
  showToast('EQ reset');
}

// A-B loop
function setAbLoopStart() {
  if (!audioPlayerPath || !audioPlayer.duration) return;
  if (!_abLoop) _abLoop = { start: 0, end: audioPlayer.duration };
  _abLoop.start = audioPlayer.currentTime;
  updateAbLoopUI();
  showToast(`A point: ${formatTime(_abLoop.start)}`);
}

function setAbLoopEnd() {
  if (!audioPlayerPath || !audioPlayer.duration) return;
  if (!_abLoop) _abLoop = { start: 0, end: audioPlayer.duration };
  _abLoop.end = audioPlayer.currentTime;
  updateAbLoopUI();
  showToast(`B point: ${formatTime(_abLoop.end)}`);
}

function clearAbLoop() {
  _abLoop = null;
  updateAbLoopUI();
}

function updateAbLoopUI() {
  const aBtn = document.getElementById('npAbA');
  const bBtn = document.getElementById('npAbB');
  const clearBtn = document.getElementById('npAbClear');
  if (aBtn) aBtn.classList.toggle('active', !!_abLoop);
  if (bBtn) bBtn.classList.toggle('active', !!_abLoop);
  if (clearBtn) clearBtn.style.display = _abLoop ? '' : 'none';
  // Show markers on waveform
  const wf = document.getElementById('npWaveform');
  let markerA = document.getElementById('npAbMarkerA');
  let markerB = document.getElementById('npAbMarkerB');
  if (!_abLoop) {
    if (markerA) markerA.style.display = 'none';
    if (markerB) markerB.style.display = 'none';
    return;
  }
  const dur = audioPlayer.duration || 1;
  if (!markerA) {
    markerA = document.createElement('div');
    markerA.id = 'npAbMarkerA';
    markerA.className = 'ab-marker ab-marker-a';
    wf.appendChild(markerA);
  }
  if (!markerB) {
    markerB = document.createElement('div');
    markerB.id = 'npAbMarkerB';
    markerB.className = 'ab-marker ab-marker-b';
    wf.appendChild(markerB);
  }
  markerA.style.display = '';
  markerB.style.display = '';
  markerA.style.left = ((_abLoop.start / dur) * 100) + '%';
  markerB.style.left = ((_abLoop.end / dur) * 100) + '%';
}

function loadRecentlyPlayed() {
  recentlyPlayed = prefs.getObject('recentlyPlayed', []);
}
function saveRecentlyPlayed() {
  prefs.setItem('recentlyPlayed', recentlyPlayed);
}

function clearRecentlyPlayed() {
  recentlyPlayed = [];
  saveRecentlyPlayed();
  renderRecentlyPlayed();
  showToast('Play history cleared');
}

function exportRecentlyPlayed() {
  if (recentlyPlayed.length === 0) { showToast('No play history to export'); return; }
  _exportCtx = {
    title: 'Play History',
    defaultName: exportFileName('play-history', recentlyPlayed.length),
    exportFn: async (fmt, filePath) => {
      if (fmt === 'pdf') {
        const headers = ['Name', 'Format', 'Size', 'Path'];
        const rows = recentlyPlayed.map(r => [r.name, r.format, r.size || '', r.path]);
        await window.vstUpdater.exportPdf('Play History', headers, rows, filePath);
      } else if (fmt === 'csv' || fmt === 'tsv') {
        const sep = fmt === 'tsv' ? '\t' : ',';
        const esc = (v) => { const s = String(v || ''); return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s; };
        const lines = ['Name' + sep + 'Format' + sep + 'Size' + sep + 'Path'];
        for (const r of recentlyPlayed) lines.push([r.name, r.format, r.size || '', r.path].map(esc).join(sep));
        await window.__TAURI__.core.invoke('plugin:fs|write_text_file', { path: filePath, contents: lines.join('\n') });
      } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml({ history: recentlyPlayed }, filePath);
      } else {
        const json = JSON.stringify(recentlyPlayed, null, 2);
        await window.__TAURI__.core.invoke('plugin:fs|write_text_file', { path: filePath, contents: json });
      }
    }
  };
  showExportModal('history', 'Play History', recentlyPlayed.length);
}

async function importRecentlyPlayed() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({
    title: 'Import Play History',
    multiple: false,
    filters: ALL_IMPORT_FILTERS,
  });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      const data = await window.vstUpdater.importToml(filePath);
      imported = data.history || data;
    } else {
      const text = await window.__TAURI__.core.invoke('plugin:fs|read_text_file', { path: filePath });
      imported = JSON.parse(text);
    }
    if (!Array.isArray(imported)) throw new Error('Expected an array');
    const existing = new Set(recentlyPlayed.map(r => r.path));
    let added = 0;
    for (const item of imported) {
      if (item.path && !existing.has(item.path)) {
        recentlyPlayed.push(item);
        existing.add(item.path);
        added++;
      }
    }
    if (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.length = MAX_RECENT;
    saveRecentlyPlayed();
    renderRecentlyPlayed();
    showToast(`Imported ${added} tracks (${imported.length - added} duplicates skipped)`);
  } catch (e) {
    showToast(`Import failed: ${e.message || e}`, 4000, 'error');
  }
}

audioPlayer.addEventListener('ended', () => {
  if (!audioLooping) {
    if (filteredAudioSamples.length > 1) {
      nextTrack(); // auto-advance
    } else {
      updatePlayBtnStates();
      updateNowPlayingBtn();
    }
  }
});
audioPlayer.addEventListener('timeupdate', updatePlaybackTime);

function formatAudioSize(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function formatTime(sec) {
  if (!sec || !isFinite(sec)) return '0:00';
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return m + ':' + String(s).padStart(2, '0');
}

function getFormatClass(format) {
  const f = format.toLowerCase();
  if (['wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'aac'].includes(f)) return 'format-' + f;
  return 'format-default';
}

async function scanAudioSamples(resume = false) {
  showGlobalProgress();
  const btn = document.getElementById('btnScanAudio');
  const resumeBtn = document.getElementById('btnResumeAudio');
  const stopBtn = document.getElementById('btnStopAudio');
  const progressBar = document.getElementById('audioProgressBar');
  const progressFill = document.getElementById('audioProgressFill');
  const tableWrap = document.getElementById('audioTableWrap');

  const excludePaths = resume ? allAudioSamples.map(s => s.path) : null;

  btn.disabled = true;
  btn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...';
  resumeBtn.style.display = 'none';
  stopBtn.style.display = '';
  progressBar.classList.add('active');
  progressFill.style.width = '0%';

  if (!resume) {
    allAudioSamples = [];
    filteredAudioSamples = [];
    expandedMetaPath = null;
    resetAudioStats();
    document.getElementById('audioStats').style.display = 'none';
    tableWrap.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for audio files...</h2><p>Walking filesystem directories parallelized...</p></div>';
  }

  let firstAudioBatch = true;
  let pendingSamples = [];
  let pendingFound = 0;
  let flushScheduled = false;
  const audioEta = createETA();
  audioEta.start();
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '100', 10);
  let lastFlush = 0;

  function flushPendingSamples() {
    flushScheduled = false;
    if (pendingSamples.length === 0) return;

    if (firstAudioBatch) {
      firstAudioBatch = false;
      tableWrap.innerHTML = '';
      initAudioTable();
    }

    const toAdd = pendingSamples;
    pendingSamples = [];

    allAudioSamples.push(...toAdd);
    accumulateAudioStats(toAdd);
    const audioElapsed = audioEta.elapsed();
    btn.innerHTML = `&#8635; ${pendingFound} found${audioElapsed ? ' — ' + audioElapsed : ''}`;
    progressFill.style.width = '';
    progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

    // Incrementally append matching rows (cap DOM at 2000 during scan)
    const search = document.getElementById('audioSearchInput').value || '';
    const scanFmtSet = getMultiFilterValues('audioFormatFilter');
    const scanMode = getSearchMode('regexAudio');
    const matching = toAdd.filter(s => {
      if (scanFmtSet && !scanFmtSet.has(s.format)) return false;
      if (search && !searchMatch(search, [s.name, s.path, s.format], scanMode)) return false;
      return true;
    });
    if (matching.length > 0) {
      filteredAudioSamples.push(...matching);
      const tbody = document.getElementById('audioTableBody');
      if (tbody && audioRenderCount < 2000) {
        const loadMore = document.getElementById('audioLoadMore');
        if (loadMore) loadMore.remove();
        const toRender = matching.slice(0, 2000 - audioRenderCount);
        tbody.insertAdjacentHTML('beforeend', toRender.map(buildAudioRow).join(''));
        audioRenderCount += toRender.length;
      }
    }

    updateAudioStats();
    lastFlush = performance.now();
  }

  function scheduleFlush() {
    if (flushScheduled) return;
    flushScheduled = true;
    const elapsed = performance.now() - lastFlush;
    const delay = Math.max(0, FLUSH_INTERVAL - elapsed);
    setTimeout(() => requestAnimationFrame(flushPendingSamples), delay);
  }

  if (audioScanProgressCleanup) audioScanProgressCleanup();
  audioScanProgressCleanup = window.vstUpdater.onAudioScanProgress((data) => {
    if (data.phase === 'status') {
      // status message
    } else if (data.phase === 'scanning') {
      pendingSamples.push(...data.samples);
      pendingFound = data.found;
      // Immediately update header counter
      document.getElementById('sampleCount').textContent = pendingFound;
      scheduleFlush();
    }
  });

  try {
    const audioRoots = (prefs.getItem('audioScanDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanAudioSamples(audioRoots.length ? audioRoots : undefined, excludePaths);
    if (audioScanProgressCleanup) { audioScanProgressCleanup(); audioScanProgressCleanup = null; }
    flushPendingSamples();
    if (resume) {
      allAudioSamples = [...allAudioSamples, ...result.samples];
    } else {
      allAudioSamples = result.samples;
    }
    rebuildAudioStats();
    filterAudioSamples();
    try { await window.vstUpdater.saveAudioScan(allAudioSamples, result.roots); } catch (e) { showToast(`Failed to save audio history — ${e.message || e}`, 4000, 'error'); }
    if (result.stopped && allAudioSamples.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (audioScanProgressCleanup) { audioScanProgressCleanup(); audioScanProgressCleanup = null; }
    flushPendingSamples();
    const errMsg = err.message || err || 'Unknown error';
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    showToast(`Audio scan failed — ${errMsg}`, 4000, 'error');
  }

  hideGlobalProgress();
  btn.disabled = false;
  btn.innerHTML = '&#127925; Scan Samples';
  stopBtn.style.display = 'none';
  progressBar.classList.remove('active');
  progressFill.style.width = '0%';
  progressFill.style.animation = '';
}

async function stopAudioScan() {
  await window.vstUpdater.stopAudioScan();
}

// Running stat counters — avoid re-scanning the full array every flush
let audioStatCounts = {};
let audioStatBytes = 0;

function resetAudioStats() {
  audioStatCounts = {};
  audioStatBytes = 0;
}

function accumulateAudioStats(samples) {
  for (const s of samples) {
    audioStatCounts[s.format] = (audioStatCounts[s.format] || 0) + 1;
    audioStatBytes += s.size;
  }
}

function updateAudioStats() {
  const stats = document.getElementById('audioStats');
  stats.style.display = 'flex';
  document.getElementById('audioTotalCount').textContent = allAudioSamples.length;
  document.getElementById('audioWavCount').textContent = audioStatCounts['WAV'] || 0;
  document.getElementById('audioMp3Count').textContent = audioStatCounts['MP3'] || 0;
  document.getElementById('audioAiffCount').textContent = (audioStatCounts['AIFF'] || 0) + (audioStatCounts['AIF'] || 0);
  document.getElementById('audioFlacCount').textContent = audioStatCounts['FLAC'] || 0;
  const mainFormats = (audioStatCounts['WAV'] || 0) + (audioStatCounts['MP3'] || 0) + (audioStatCounts['AIFF'] || 0) + (audioStatCounts['AIF'] || 0) + (audioStatCounts['FLAC'] || 0);
  document.getElementById('audioOtherCount').textContent = allAudioSamples.length - mainFormats;
  document.getElementById('audioTotalSize').textContent = formatAudioSize(audioStatBytes);
  // Update top stats bar sample count
  document.getElementById('sampleCount').textContent = allAudioSamples.length;
  document.getElementById('btnExportAudio').style.display = allAudioSamples.length > 0 ? '' : 'none';
  if (typeof updateAudioDiskUsage === 'function') updateAudioDiskUsage();
}

function rebuildAudioStats() {
  resetAudioStats();
  accumulateAudioStats(allAudioSamples);
  updateAudioStats();
}

function initAudioTable() {
  const tableWrap = document.getElementById('audioTableWrap');
  tableWrap.innerHTML = `<table class="audio-table" id="audioTable">
    <thead>
      <tr>
        <th class="col-cb"><input type="checkbox" class="batch-cb batch-cb-all" data-batch-action="toggleAll" title="Select all"></th>
        <th data-action="sortAudio" data-key="name" style="width: 26%;">Name <span class="sort-arrow" id="sortArrowName">&#9660;</span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="format" class="col-format" style="width: 70px;">Format <span class="sort-arrow" id="sortArrowFormat"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="size" class="col-size" style="width: 90px;">Size <span class="sort-arrow" id="sortArrowSize"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="modified" class="col-date" style="width: 100px;">Modified <span class="sort-arrow" id="sortArrowModified"></span><span class="col-resize"></span></th>
        <th data-action="sortAudio" data-key="directory" style="width: 30%;">Path <span class="sort-arrow" id="sortArrowDirectory"></span><span class="col-resize"></span></th>
        <th class="col-actions" style="width: 130px;"></th>
      </tr>
    </thead>
    <tbody id="audioTableBody"></tbody>
  </table>`;
  initColumnResize(document.getElementById('audioTable'));
}

let _lastAudioSearch = '';
let _lastAudioMode = 'fuzzy';

function filterAudioSamples() {
  const search = document.getElementById('audioSearchInput').value || '';
  const formatEl = document.getElementById('audioFormatFilter');
  autoSelectDropdown(formatEl, search);
  const fmtSet = getMultiFilterValues('audioFormatFilter');
  const mode = getSearchMode('regexAudio');
  _lastAudioSearch = search;
  _lastAudioMode = mode;

  if (search) {
    const scored = [];
    for (const s of allAudioSamples) {
      if (typeof passesGlobalTagFilter === 'function' && !passesGlobalTagFilter(s.path)) continue;
      if (fmtSet && !fmtSet.has(s.format)) continue;
      const score = searchScore(search, [s.name, s.path, s.format], mode);
      if (score > 0) scored.push({ item: s, score });
    }
    scored.sort((a, b) => b.score - a.score);
    filteredAudioSamples = scored.map(s => s.item);
  } else {
    filteredAudioSamples = allAudioSamples.filter(s => {
      if (typeof passesGlobalTagFilter === 'function' && !passesGlobalTagFilter(s.path)) return false;
      if (fmtSet && !fmtSet.has(s.format)) return false;
      return true;
    });
    sortAudioArray();
  }
  renderAudioTable();
}

function sortAudio(key) {
  if (audioSortKey === key) {
    audioSortAsc = !audioSortAsc;
  } else {
    audioSortKey = key;
    audioSortAsc = true;
  }
  ['Name', 'Format', 'Size', 'Modified', 'Directory'].forEach(k => {
    const el = document.getElementById('sortArrow' + k);
    if (el) {
      const isActive = k.toLowerCase() === audioSortKey;
      el.innerHTML = isActive ? (audioSortAsc ? '&#9650;' : '&#9660;') : '';
      el.closest('th').classList.toggle('sort-active', isActive);
    }
  });
  sortAudioArray();
  renderAudioTable();
}

function sortAudioArray() {
  const key = audioSortKey;
  const dir = audioSortAsc ? 1 : -1;
  filteredAudioSamples.sort((a, b) => {
    let va = a[key], vb = b[key];
    if (key === 'size') return (va - vb) * dir;
    if (typeof va === 'string') return va.localeCompare(vb) * dir;
    return 0;
  });
}

let audioRenderCount = 0;

function renderAudioTable() {
  if (!document.getElementById('audioTable')) initAudioTable();
  const tbody = document.getElementById('audioTableBody');
  if (!tbody) return;
  audioRenderCount = Math.min(AUDIO_PAGE_SIZE, filteredAudioSamples.length);
  tbody.innerHTML = filteredAudioSamples.slice(0, audioRenderCount).map(buildAudioRow).join('');

  if (audioRenderCount < filteredAudioSamples.length) {
    appendLoadMore(tbody);
  }
}

function appendLoadMore(tbody) {
  tbody.insertAdjacentHTML('beforeend',
    `<tr id="audioLoadMore"><td colspan="7" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreAudio">
      Showing ${audioRenderCount} of ${filteredAudioSamples.length} &#8212; click to load more
    </td></tr>`);
}

function loadMoreAudio() {
  const tbody = document.getElementById('audioTableBody');
  const loadMore = document.getElementById('audioLoadMore');
  if (loadMore) loadMore.remove();
  const nextBatch = filteredAudioSamples.slice(audioRenderCount, audioRenderCount + AUDIO_PAGE_SIZE);
  audioRenderCount += nextBatch.length;
  tbody.insertAdjacentHTML('beforeend', nextBatch.map(buildAudioRow).join(''));
  if (audioRenderCount < filteredAudioSamples.length) {
    appendLoadMore(tbody);
  }
}

function buildAudioRow(s) {
  const fmtClass = getFormatClass(s.format);
  const hp = escapeHtml(s.path);
  const isPlaying = audioPlayerPath === s.path;
  const rowClass = isPlaying ? ' class="row-playing"' : '';
  const checked = batchSelected.has(s.path) ? ' checked' : '';
  return `<tr${rowClass} data-audio-path="${hp}" data-action="toggleMetadata" data-path="${hp}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(s.name)}">${noteIndicator(s.path)}${highlightMatch(s.name, _lastAudioSearch, _lastAudioMode)}</td>
    <td class="col-format"><span class="format-badge ${fmtClass}">${s.format}</span></td>
    <td class="col-size">${s.sizeFormatted}</td>
    <td class="col-date">${s.modified}</td>
    <td class="col-path" title="${hp}">${escapeHtml(s.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${hp}" title="Preview">
        ${isPlaying && !audioPlayer.paused ? '&#9646;&#9646;' : '&#9654;'}
      </button>
      <button class="btn-small btn-loop${isPlaying && audioLooping ? ' active' : ''}" data-action="toggleRowLoop" data-path="${hp}" title="Loop">&#8634;</button>
      <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${hp}" title="Reveal in Finder">&#128193;</button>
    </td>
  </tr>`;
}

// ── Audio Preview / Playback ──
async function previewAudio(filePath) {
  if (audioPlayerPath === filePath && !audioPlayer.paused) {
    // Pause current
    audioPlayer.pause();
    updatePlayBtnStates();
    updateNowPlayingBtn();
    return;
  }

  if (audioPlayerPath === filePath && audioPlayer.paused) {
    // Resume current
    audioPlayer.play();
    updatePlayBtnStates();
    updateNowPlayingBtn();
    return;
  }

  // New file
  try {
    ensureAudioGraph();
    if (_playbackCtx.state === 'suspended') _playbackCtx.resume();
    audioPlayer.src = convertFileSrc(filePath);
    audioPlayer.loop = audioLooping;
    audioPlayerPath = filePath;
    audioPlayer.play();

    // Show now-playing bar, restore expanded state from prefs
    const np = document.getElementById('audioNowPlaying');
    np.classList.add('active');
    if (prefs.getItem('playerExpanded') === 'on') {
      np.classList.add('expanded');
      renderRecentlyPlayed();
    }
    const sample = allAudioSamples.find(s => s.path === filePath);
    const displayName = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
    document.getElementById('npName').textContent = displayName;

    // Track recently played
    addToRecentlyPlayed(filePath, sample);

    updatePlayBtnStates();
    updateNowPlayingBtn();
    updateMetaLine();
    drawWaveform(filePath);
  } catch (err) {
    showToast(`Playback failed — ${err.message || err || 'Unknown error'}`, 4000, 'error');
  }
}

function toggleAudioPlayback() {
  if (!audioPlayerPath) return;
  if (audioPlayer.paused) {
    audioPlayer.play();
  } else {
    audioPlayer.pause();
  }
  updatePlayBtnStates();
  updateNowPlayingBtn();
}

function toggleAudioLoop() {
  audioLooping = !audioLooping;
  audioPlayer.loop = audioLooping;
  document.getElementById('npBtnLoop').classList.toggle('active', audioLooping);
  updateLoopBtnStates();
}

function toggleRowLoop(filePath, event) {
  event.stopPropagation();
  // If this sample isn't playing yet, start it with loop on
  if (audioPlayerPath !== filePath) {
    audioLooping = true;
    audioPlayer.loop = true;
    document.getElementById('npBtnLoop').classList.add('active');
    previewAudio(filePath);
    updateLoopBtnStates();
    return;
  }
  // Toggle loop for the currently playing sample
  toggleAudioLoop();
}

function updateLoopBtnStates() {
  document.querySelectorAll('.audio-table .btn-loop').forEach(btn => {
    const row = btn.closest('tr');
    if (!row) return;
    const rowPath = row.getAttribute('data-audio-path');
    const isThis = rowPath === audioPlayerPath;
    btn.classList.toggle('active', isThis && audioLooping);
  });
}

function stopAudioPlayback() {
  audioPlayer.pause();
  audioPlayer.currentTime = 0;
  audioPlayer.src = '';
  audioPlayerPath = null;
  clearAudioPlaybackUI();
}

function clearAudioPlaybackUI() {
  const np = document.getElementById('audioNowPlaying');
  np.classList.remove('active');
  np.classList.remove('expanded');
  np.classList.remove('np-playing');
  document.getElementById('npProgress').style.width = '0%';
  document.getElementById('npTime').textContent = '0:00 / 0:00';
  const npCursor = document.getElementById('npCursor');
  if (npCursor) npCursor.style.display = 'none';
  const pill = document.getElementById('audioRestorePill');
  if (pill) pill.classList.remove('active');
  updatePlayBtnStates();
}

function updatePlayBtnStates() {
  document.querySelectorAll('.audio-table .btn-play').forEach(btn => {
    const row = btn.closest('tr');
    if (!row) return;
    const rowPath = row.getAttribute('data-audio-path');
    const isThis = rowPath === audioPlayerPath;
    btn.classList.toggle('playing', isThis && !audioPlayer.paused);
    btn.innerHTML = (isThis && !audioPlayer.paused) ? '&#9646;&#9646;' : '&#9654;';
    row.classList.toggle('row-playing', isThis && !audioPlayer.paused);
  });
  updateLoopBtnStates();
  renderRecentlyPlayed();
}

function updateNowPlayingBtn() {
  const btn = document.getElementById('npBtnPlay');
  const np = document.getElementById('audioNowPlaying');
  if (audioPlayer.paused) {
    btn.innerHTML = '&#9654;';
    btn.classList.remove('playing');
    np.classList.remove('np-playing');
  } else {
    btn.innerHTML = '&#9646;&#9646;';
    btn.classList.add('playing');
    np.classList.add('np-playing');
  }
}

function updatePlaybackTime() {
  const cur = audioPlayer.currentTime;
  const dur = audioPlayer.duration;
  // A-B loop enforcement
  if (_abLoop && dur > 0 && cur >= _abLoop.end) {
    audioPlayer.currentTime = _abLoop.start;
  }
  document.getElementById('npTime').textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
  if (dur > 0) {
    const pct = (cur / dur) * 100;
    document.getElementById('npProgress').style.width = pct + '%';
    // Playback cursor — floating player
    const npCursor = document.getElementById('npCursor');
    if (npCursor) {
      npCursor.style.display = '';
      npCursor.style.left = pct + '%';
    }
    // Playback cursor — metadata panel
    const metaWaveform = document.getElementById('metaWaveformBox');
    if (metaWaveform && metaWaveform.dataset.path === audioPlayerPath) {
      const fill = metaWaveform.querySelector('.waveform-progress-fill');
      const cursor = metaWaveform.querySelector('.waveform-cursor');
      const timeLabel = metaWaveform.querySelector('.waveform-time-label');
      if (fill) fill.style.width = pct + '%';
      if (cursor) cursor.style.left = pct + '%';
      if (timeLabel) timeLabel.textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
    }
  }
}

function seekAudio(event) {
  if (!audioPlayerPath || !audioPlayer.duration) return;
  const bar = document.getElementById('npWaveform');
  const rect = bar.getBoundingClientRect();
  const pct = (event.clientX - rect.left) / rect.width;
  audioPlayer.currentTime = pct * audioPlayer.duration;
}

function setAudioVolume(value) {
  const vol = parseInt(value, 10) / 100;
  audioPlayer.volume = Math.max(0, Math.min(1, vol));
  // Also set via gain node for Web Audio API path
  if (_gainNode) {
    _gainNode.gain.value = vol * parseFloat(document.getElementById('npGainSlider')?.value || '1');
  }
  document.getElementById('npVolumePct').textContent = value + '%';
}

function setPlaybackSpeed(value) {
  audioPlayer.playbackRate = parseFloat(value);
}

// ── Metadata Panel ──
async function toggleMetadata(filePath, event) {
  // Don't toggle if clicking buttons
  if (event.target.closest('.col-actions')) return;

  // Single-click starts playback
  previewAudio(filePath);

  const tbody = document.getElementById('audioTableBody');
  if (!tbody) return;

  const existingMeta = document.getElementById('audioMetaRow');

  // Close existing
  if (existingMeta) {
    const wasPath = existingMeta.getAttribute('data-meta-path');
    existingMeta.remove();
    // Un-mark the expanded row
    const prevRow = tbody.querySelector(`tr.row-expanded`);
    if (prevRow) prevRow.classList.remove('row-expanded');

    if (wasPath === filePath) {
      expandedMetaPath = null;
      return; // toggle off
    }
  }

  expandedMetaPath = filePath;

  // Find the clicked row
  const row = tbody.querySelector(`tr[data-audio-path="${CSS.escape(filePath)}"]`);
  if (!row) return;
  row.classList.add('row-expanded');

  // Insert loading row
  const metaRow = document.createElement('tr');
  metaRow.id = 'audioMetaRow';
  metaRow.className = 'audio-meta-row';
  metaRow.setAttribute('data-meta-path', filePath);
  metaRow.innerHTML = `<td colspan="7"><div class="audio-meta-panel" style="justify-items: center;"><div class="spinner" style="width: 18px; height: 18px;"></div></div></td>`;
  row.after(metaRow);

  // Fetch metadata
  try {
    const meta = await window.vstUpdater.getAudioMetadata(filePath);
    if (expandedMetaPath !== filePath) return; // user closed it

    let items = '';
    items += metaItem('File Name', meta.fileName, true);
    items += metaItem('Format', meta.format);
    items += metaItem('Size', formatAudioSize(meta.sizeBytes));
    items += metaItem('Full Path', meta.fullPath, true);

    if (meta.sampleRate) items += metaItem('Sample Rate', meta.sampleRate.toLocaleString() + ' Hz');
    if (meta.bitsPerSample) items += metaItem('Bit Depth', meta.bitsPerSample + '-bit');
    if (meta.channels) items += metaItem('Channels', meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch');
    if (meta.duration) items += metaItem('Duration', formatTime(meta.duration));
    if (meta.byteRate) items += metaItem('Byte Rate', formatAudioSize(meta.byteRate) + '/s');

    // BPM placeholder — filled async
    items += `<div class="meta-item" id="metaBpmItem"><span class="meta-label">BPM</span><span class="meta-value" id="metaBpmValue" style="display:flex;align-items:center;gap:6px;"><span class="spinner" style="width:10px;height:10px;"></span></span></div>`;

    const fmtDate = (v) => { if (!v) return '—'; const d = new Date(v); return isNaN(d) ? '—' : d.toLocaleString(); };
    items += metaItem('Created', fmtDate(meta.created));
    items += metaItem('Modified', fmtDate(meta.modified));
    items += metaItem('Accessed', fmtDate(meta.accessed));
    items += metaItem('Permissions', meta.permissions);

    // Waveform preview with seek support
    const waveformHtml = `<div class="meta-waveform" id="metaWaveformBox" data-path="${escapeHtml(filePath)}" data-action="seekMetaWaveform">
      <canvas id="metaWaveformCanvas"></canvas>
      <div class="waveform-progress-fill"></div>
      <div class="waveform-cursor" style="left:0;"></div>
      <div class="waveform-time-label">${meta.duration ? formatTime(meta.duration) : ''}</div>
    </div>`;

    metaRow.innerHTML = `<td colspan="7"><div class="audio-meta-panel">${waveformHtml}${items}</div></td>`;

    // Draw waveform on the meta canvas
    drawMetaWaveform(filePath);

    // Sync cursor if already playing this track
    if (audioPlayerPath === filePath && audioPlayer.duration > 0) {
      const pct = (audioPlayer.currentTime / audioPlayer.duration) * 100;
      const box = document.getElementById('metaWaveformBox');
      if (box) {
        const fill = box.querySelector('.waveform-progress-fill');
        const cursor = box.querySelector('.waveform-cursor');
        const timeLabel = box.querySelector('.waveform-time-label');
        if (fill) fill.style.width = pct + '%';
        if (cursor) cursor.style.left = pct + '%';
        if (timeLabel) timeLabel.textContent = `${formatTime(audioPlayer.currentTime)} / ${formatTime(audioPlayer.duration)}`;
      }
    }

    // Estimate BPM async (WAV/AIFF only)
    const bpmFormats = ['WAV', 'AIFF', 'AIF'];
    if (bpmFormats.includes(meta.format)) {
      estimateBpmForMeta(filePath);
    } else {
      const bpmEl = document.getElementById('metaBpmValue');
      if (bpmEl) bpmEl.textContent = 'N/A (format not supported)';
    }
  } catch (err) {
    metaRow.innerHTML = `<td colspan="7"><div class="audio-meta-panel"><span style="color: var(--red);">Failed to load metadata</span></div></td>`;
  }
}

// BPM cache
const _bpmCache = {};

async function estimateBpmForMeta(filePath) {
  const bpmEl = document.getElementById('metaBpmValue');
  if (!bpmEl) return;

  if (_bpmCache[filePath] !== undefined) {
    bpmEl.textContent = _bpmCache[filePath] ? _bpmCache[filePath] + ' BPM' : '—';
    return;
  }

  try {
    const bpm = await window.vstUpdater.estimateBpm(filePath);
    _bpmCache[filePath] = bpm;
    // Check the panel is still showing this file
    const currentBpmEl = document.getElementById('metaBpmValue');
    const metaRow = document.getElementById('audioMetaRow');
    if (currentBpmEl && metaRow && metaRow.getAttribute('data-meta-path') === filePath) {
      currentBpmEl.textContent = bpm ? bpm + ' BPM' : '—';
    }
  } catch {
    if (bpmEl) bpmEl.textContent = '—';
  }
}

function metaItem(label, value, wide) {
  const cls = wide ? 'meta-item meta-item-wide' : 'meta-item';
  return `<div class="${cls}"><span class="meta-label">${label}</span><span class="meta-value">${escapeHtml(String(value || '—'))}</span></div>`;
}

function openAudioFolder(filePath) {
  window.vstUpdater.openAudioFolder(filePath);
}

// ── Recently Played / Expanded Player ──
function addToRecentlyPlayed(filePath, sample) {
  // Remove duplicate if already in list
  recentlyPlayed = recentlyPlayed.filter(r => r.path !== filePath);
  // Add to front
  recentlyPlayed.unshift({
    path: filePath,
    name: sample ? sample.name : filePath.split('/').pop().replace(/\.[^.]+$/, ''),
    format: sample ? sample.format : filePath.split('.').pop().toUpperCase(),
    size: sample ? sample.sizeFormatted : '',
  });
  // Cap
  if (recentlyPlayed.length > MAX_RECENT) recentlyPlayed.length = MAX_RECENT;
  saveRecentlyPlayed();
  renderRecentlyPlayed();
}

function renderRecentlyPlayed() {
  const list = document.getElementById('npHistoryList');
  if (!list) return;
  const searchInput = document.getElementById('npSearchInput');
  const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

  let items;
  if (query) {
    // Search all audio samples + recently played, deduplicated, scored by fzf
    const seen = new Set();
    const scored = [];
    for (const r of recentlyPlayed) {
      const score = searchScore(query, [r.name, r.path], 'fuzzy');
      if (score > 0 && !seen.has(r.path)) { seen.add(r.path); scored.push({ item: r, score: score + 1000 }); }
    }
    if (typeof allAudioSamples !== 'undefined') {
      for (const s of allAudioSamples) {
        const score = searchScore(query, [s.name, s.path], 'fuzzy');
        if (score > 0 && !seen.has(s.path)) {
          seen.add(s.path);
          scored.push({ item: { path: s.path, name: s.name, format: s.format, size: s.sizeFormatted }, score });
        }
      }
    }
    scored.sort((a, b) => b.score - a.score);
    items = scored.slice(0, 100).map(s => s.item);
  } else {
    items = recentlyPlayed;
  }

  if (items.length === 0 && query) {
    list.innerHTML = '<div style="text-align:center;color:var(--text-dim);font-size:11px;padding:12px;">No matches</div>';
    return;
  }

  list.innerHTML = items.map(r => {
    const isActive = r.path === audioPlayerPath;
    const isPlaying = isActive && !audioPlayer.paused;
    return `<div class="np-history-item${isActive ? ' active' : ''}" data-action="playRecent" data-path="${escapeHtml(r.path)}">
      <span class="np-h-icon">${isPlaying ? '&#9654;' : '&#9835;'}</span>
      <span class="np-h-name" title="${escapeHtml(r.path)}">${query ? highlightMatch(r.name, query, 'fuzzy') : escapeHtml(r.name)}</span>
      <span class="np-h-format">${r.format}</span>
      ${r.size ? `<span class="np-h-dur">${r.size}</span>` : ''}
    </div>`;
  }).join('');
}

// Search input in player — renders to mini search results or expanded history list
document.getElementById('npSearchInput')?.addEventListener('input', () => {
  const np = document.getElementById('audioNowPlaying');
  if (np && np.classList.contains('expanded')) {
    renderRecentlyPlayed();
  } else {
    renderMiniSearchResults();
  }
});

function renderMiniSearchResults() {
  const container = document.getElementById('npSearchResults');
  if (!container) return;
  const searchInput = document.getElementById('npSearchInput');
  const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

  if (!query) { container.innerHTML = ''; return; }

  const seen = new Set();
  const scored = [];
  for (const r of recentlyPlayed) {
    const score = searchScore(query, [r.name, r.path], 'fuzzy');
    if (score > 0 && !seen.has(r.path)) { seen.add(r.path); scored.push({ item: r, score: score + 1000 }); }
  }
  if (typeof allAudioSamples !== 'undefined') {
    for (const s of allAudioSamples) {
      const score = searchScore(query, [s.name, s.path], 'fuzzy');
      if (score > 0 && !seen.has(s.path)) {
        seen.add(s.path);
        scored.push({ item: { path: s.path, name: s.name, format: s.format, size: s.sizeFormatted }, score });
      }
    }
  }
  scored.sort((a, b) => b.score - a.score);
  const items = scored.slice(0, 50).map(s => s.item);

  if (items.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:var(--text-dim);font-size:11px;padding:8px;">No matches</div>';
    return;
  }

  container.innerHTML = items.map(r => {
    const isActive = r.path === audioPlayerPath;
    return `<div class="np-history-item${isActive ? ' active' : ''}" data-action="playRecent" data-path="${escapeHtml(r.path)}">
      <span class="np-h-icon">&#9835;</span>
      <span class="np-h-name" title="${escapeHtml(r.path)}">${highlightMatch(r.name, query, 'fuzzy')}</span>
      <span class="np-h-format">${r.format}</span>
    </div>`;
  }).join('');
}

function togglePlayerExpanded() {
  const np = document.getElementById('audioNowPlaying');
  np.classList.toggle('expanded');
  prefs.setItem('playerExpanded', np.classList.contains('expanded') ? 'on' : 'off');
  if (np.classList.contains('expanded')) {
    renderRecentlyPlayed();
  }
}

function collapsePlayer() {
  document.getElementById('audioNowPlaying').classList.remove('expanded');
  prefs.setItem('playerExpanded', 'off');
}

function hidePlayer() {
  const np = document.getElementById('audioNowPlaying');
  prefs.setItem('playerExpanded', np.classList.contains('expanded') ? 'on' : 'off');
  np.classList.remove('active');
  const pill = document.getElementById('audioRestorePill');
  if (pill && audioPlayerPath && !audioPlayer.paused) {
    pill.classList.add('active');
  }
}

function showPlayer() {
  const pill = document.getElementById('audioRestorePill');
  if (pill) pill.classList.remove('active');
  if (audioPlayerPath) {
    const np = document.getElementById('audioNowPlaying');
    np.classList.add('active');
    if (prefs.getItem('playerExpanded') === 'on') np.classList.add('expanded');
  }
}

// Double-click to expand/collapse player
document.getElementById('audioNowPlaying').addEventListener('dblclick', (e) => {
  // Don't toggle if clicking controls
  if (e.target.closest('button, input, select, .now-playing-waveform, .np-history-item')) return;
  togglePlayerExpanded();
});

// Play from recently played list
document.getElementById('npHistoryList')?.addEventListener('click', (e) => {
  const item = e.target.closest('[data-action="playRecent"]');
  if (item) {
    e.stopPropagation();
    previewAudio(item.dataset.path);
  }
});

// ── Previous / Next / Shuffle ──
function prevTrack() {
  if (recentlyPlayed.length < 2) return;
  // Find current in recently played, go to next older one
  const idx = recentlyPlayed.findIndex(r => r.path === audioPlayerPath);
  const nextIdx = idx >= 0 && idx < recentlyPlayed.length - 1 ? idx + 1 : 0;
  previewAudio(recentlyPlayed[nextIdx].path);
}

function nextTrack() {
  if (audioShuffling) {
    // Random from filtered samples
    if (filteredAudioSamples.length === 0) return;
    const rand = filteredAudioSamples[Math.floor(Math.random() * filteredAudioSamples.length)];
    previewAudio(rand.path);
  } else {
    // Next in filtered list after current
    const idx = filteredAudioSamples.findIndex(s => s.path === audioPlayerPath);
    const nextIdx = (idx + 1) % filteredAudioSamples.length;
    if (filteredAudioSamples.length > 0) previewAudio(filteredAudioSamples[nextIdx].path);
  }
}

function toggleShuffle() {
  audioShuffling = !audioShuffling;
  const btn = document.getElementById('npBtnShuffle');
  if (btn) btn.classList.toggle('active', audioShuffling);
}

function toggleMute() {
  const btn = document.getElementById('npBtnMute');
  const slider = document.getElementById('npVolume');
  if (audioMuted) {
    audioPlayer.volume = savedVolume;
    audioMuted = false;
    if (btn) btn.innerHTML = '&#128264;';
    if (slider) slider.value = Math.round(savedVolume * 100);
    document.getElementById('npVolumePct').textContent = Math.round(savedVolume * 100) + '%';
  } else {
    savedVolume = audioPlayer.volume;
    audioPlayer.volume = 0;
    audioMuted = true;
    if (btn) btn.innerHTML = '&#128263;';
    if (slider) slider.value = 0;
    document.getElementById('npVolumePct').textContent = '0%';
  }
}

// ── Waveform rendering ──
let _audioCtx = null;
let _waveformCache = {};

async function drawWaveform(filePath) {
  const canvas = document.getElementById('npWaveformCanvas');
  if (!canvas) return;
  const container = canvas.parentElement;
  canvas.width = container.offsetWidth * (window.devicePixelRatio || 1);
  canvas.height = container.offsetHeight * (window.devicePixelRatio || 1);
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Check cache
  if (_waveformCache[filePath]) {
    renderWaveformData(ctx, canvas, _waveformCache[filePath]);
    return;
  }

  try {
    if (!_audioCtx) _audioCtx = new AudioContext();
    const src = convertFileSrc(filePath);
    const resp = await fetch(src);
    const buf = await resp.arrayBuffer();
    const audioBuf = await _audioCtx.decodeAudioData(buf);
    const raw = audioBuf.getChannelData(0);

    // Downsample to canvas width
    const bars = Math.min(canvas.width, 200);
    const step = Math.floor(raw.length / bars);
    const peaks = [];
    for (let i = 0; i < bars; i++) {
      let max = 0;
      const start = i * step;
      for (let j = start; j < start + step && j < raw.length; j++) {
        const abs = Math.abs(raw[j]);
        if (abs > max) max = abs;
      }
      peaks.push(max);
    }

    _waveformCache[filePath] = peaks;
    renderWaveformData(ctx, canvas, peaks);
  } catch {
    // Fallback: draw a simple centered line
    ctx.strokeStyle = 'rgba(5,217,232,0.3)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, canvas.height / 2);
    ctx.lineTo(canvas.width, canvas.height / 2);
    ctx.stroke();
  }
}

function renderWaveformData(ctx, canvas, peaks) {
  const w = canvas.width;
  const h = canvas.height;
  const barW = w / peaks.length;
  const mid = h / 2;

  ctx.clearRect(0, 0, w, h);
  for (let i = 0; i < peaks.length; i++) {
    const barH = peaks[i] * mid * 0.9;
    const x = i * barW;
    // Gradient from cyan to magenta
    const t = i / peaks.length;
    const r = Math.round(5 + t * 250);
    const g = Math.round(217 - t * 175);
    const b = Math.round(232 - t * 23);
    ctx.fillStyle = `rgba(${r},${g},${b},0.6)`;
    ctx.fillRect(x, mid - barH, barW - 0.5, barH * 2);
  }
}

async function drawMetaWaveform(filePath) {
  const canvas = document.getElementById('metaWaveformCanvas');
  if (!canvas) return;
  const container = canvas.parentElement;
  canvas.width = container.offsetWidth * (window.devicePixelRatio || 1);
  canvas.height = container.offsetHeight * (window.devicePixelRatio || 1);
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (_waveformCache[filePath]) {
    renderWaveformData(ctx, canvas, _waveformCache[filePath]);
    return;
  }

  try {
    if (!_audioCtx) _audioCtx = new AudioContext();
    const src = convertFileSrc(filePath);
    const resp = await fetch(src);
    const buf = await resp.arrayBuffer();
    const audioBuf = await _audioCtx.decodeAudioData(buf);
    const raw = audioBuf.getChannelData(0);

    const bars = Math.min(canvas.width, 300);
    const step = Math.floor(raw.length / bars);
    const peaks = [];
    for (let i = 0; i < bars; i++) {
      let max = 0;
      const start = i * step;
      for (let j = start; j < start + step && j < raw.length; j++) {
        const abs = Math.abs(raw[j]);
        if (abs > max) max = abs;
      }
      peaks.push(max);
    }

    _waveformCache[filePath] = peaks;
    renderWaveformData(ctx, canvas, peaks);
  } catch {
    ctx.strokeStyle = 'rgba(5,217,232,0.3)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, canvas.height / 2);
    ctx.lineTo(canvas.width, canvas.height / 2);
    ctx.stroke();
  }
}

function seekMetaWaveform(event) {
  if (!audioPlayerPath || !audioPlayer.duration) return;
  const box = document.getElementById('metaWaveformBox');
  if (!box) return;
  const rect = box.getBoundingClientRect();
  const pct = (event.clientX - rect.left) / rect.width;
  audioPlayer.currentTime = pct * audioPlayer.duration;
}

function updateMetaLine() {
  const el = document.getElementById('npMetaLine');
  if (!el || !audioPlayerPath) { if (el) el.textContent = ''; return; }
  const sample = allAudioSamples.find(s => s.path === audioPlayerPath);
  if (!sample) { el.textContent = audioPlayerPath.split('/').pop(); return; }
  const parts = [sample.format, sample.sizeFormatted];
  if (_bpmCache[audioPlayerPath]) parts.push(_bpmCache[audioPlayerPath] + ' BPM');
  if (sample.directory) parts.push(sample.directory);
  el.textContent = parts.join(' \u2022 ');
}

// ── Visualizer bars init ──
(function initVisualizer() {
  const viz = document.getElementById('npVisualizer');
  if (!viz) return;
  const BAR_COUNT = 24;
  for (let i = 0; i < BAR_COUNT; i++) {
    const bar = document.createElement('div');
    bar.className = 'np-viz-bar';
    // Randomize timing and height for organic look
    const dur = (0.4 + Math.random() * 0.6).toFixed(2);
    const minH = (3 + Math.random() * 4).toFixed(0);
    const maxH = (12 + Math.random() * 20).toFixed(0);
    const delay = (Math.random() * -1.5).toFixed(2);
    bar.style.setProperty('--viz-dur', dur + 's');
    bar.style.setProperty('--viz-min', minH + 'px');
    bar.style.setProperty('--viz-max', maxH + 'px');
    bar.style.animationDelay = delay + 's';
    viz.appendChild(bar);
  }
})();

// ── Drag-to-dock ──
(function initPlayerDrag() {
  const np = document.getElementById('audioNowPlaying');
  const handle = document.getElementById('npDragHandle');
  const overlay = document.getElementById('dockOverlay');
  const zones = { tl: 'dockTL', tr: 'dockTR', bl: 'dockBL', br: 'dockBR' };
  let dragging = false, startX, startY, origX, origY;

  function getCurrentDock() {
    for (const cls of np.classList) {
      if (cls.startsWith('dock-')) return cls;
    }
    return 'dock-br';
  }

  function setDock(dock) {
    np.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
    np.classList.add(dock);
    prefs.setItem('playerDock', dock);
  }

  // Restore saved dock position
  const saved = prefs.getItem('playerDock');
  if (saved && ['dock-tl', 'dock-tr', 'dock-bl', 'dock-br'].includes(saved)) {
    setDock(saved);
  }

  function nearestDock(x, y) {
    const cx = window.innerWidth / 2;
    const cy = window.innerHeight / 2;
    if (x < cx && y < cy) return 'dock-tl';
    if (x >= cx && y < cy) return 'dock-tr';
    if (x < cx && y >= cy) return 'dock-bl';
    return 'dock-br';
  }

  function highlightZone(dock) {
    Object.values(zones).forEach(id => document.getElementById(id).classList.remove('active'));
    const map = { 'dock-tl': 'dockTL', 'dock-tr': 'dockTR', 'dock-bl': 'dockBL', 'dock-br': 'dockBR' };
    const el = document.getElementById(map[dock]);
    if (el) el.classList.add('active');
  }

  const toolbar = np.querySelector('.np-toolbar');

  function onDragStart(e) {
    if (e.button !== 0) return;
    // Don't drag if clicking toolbar buttons
    if (e.target.closest('.np-toolbar-actions')) return;
    e.preventDefault();
    dragging = true;
    startX = e.clientX;
    startY = e.clientY;
    const rect = np.getBoundingClientRect();
    origX = rect.left;
    origY = rect.top;

    // Switch to absolute positioning for free drag
    np.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
    np.style.left = origX + 'px';
    np.style.top = origY + 'px';
    np.style.right = 'auto';
    np.style.bottom = 'auto';
    np.classList.add('dragging');
    overlay.classList.add('visible');
  }

  handle.addEventListener('mousedown', onDragStart);
  toolbar.addEventListener('mousedown', onDragStart);

  document.addEventListener('mousemove', (e) => {
    if (!dragging) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    np.style.left = (origX + dx) + 'px';
    np.style.top = (origY + dy) + 'px';
    highlightZone(nearestDock(e.clientX, e.clientY));
  });

  document.addEventListener('mouseup', (e) => {
    if (!dragging) return;
    dragging = false;
    np.classList.remove('dragging');
    overlay.classList.remove('visible');
    Object.values(zones).forEach(id => document.getElementById(id).classList.remove('active'));

    // Clear inline styles and snap to dock
    np.style.left = '';
    np.style.top = '';
    np.style.right = '';
    np.style.bottom = '';

    const dock = nearestDock(e.clientX, e.clientY);
    np.classList.add('snapping');
    setDock(dock);
    setTimeout(() => np.classList.remove('snapping'), 300);
  });
})();

// ── Corner resize ──
(function initPlayerResize() {
  const np = document.getElementById('audioNowPlaying');
  let resizing = false;
  let corner = '';
  let startX, startY, startW, startH, startLeft, startTop;

  np.addEventListener('mousedown', (e) => {
    const handle = e.target.closest('[data-resize]');
    if (!handle) return;
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    corner = handle.dataset.resize;
    startX = e.clientX;
    startY = e.clientY;
    const rect = np.getBoundingClientRect();
    startW = rect.width;
    startH = rect.height;
    startLeft = rect.left;
    startTop = rect.top;

    // Switch to absolute positioning for resize
    np.classList.remove('dock-tl', 'dock-tr', 'dock-bl', 'dock-br');
    np.style.left = startLeft + 'px';
    np.style.top = startTop + 'px';
    np.style.right = 'auto';
    np.style.bottom = 'auto';
    np.style.width = startW + 'px';
    np.style.height = startH + 'px';
    document.body.style.userSelect = 'none';
  });

  document.addEventListener('mousemove', (e) => {
    if (!resizing) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    const minW = 280;
    const minH = 200;

    if (corner === 'br') {
      np.style.width = Math.max(minW, startW + dx) + 'px';
      np.style.height = Math.max(minH, startH + dy) + 'px';
    } else if (corner === 'bl') {
      const newW = Math.max(minW, startW - dx);
      np.style.width = newW + 'px';
      np.style.left = (startLeft + startW - newW) + 'px';
      np.style.height = Math.max(minH, startH + dy) + 'px';
    } else if (corner === 'tr') {
      np.style.width = Math.max(minW, startW + dx) + 'px';
      const newH = Math.max(minH, startH - dy);
      np.style.height = newH + 'px';
      np.style.top = (startTop + startH - newH) + 'px';
    } else if (corner === 'tl') {
      const newW = Math.max(minW, startW - dx);
      np.style.width = newW + 'px';
      np.style.left = (startLeft + startW - newW) + 'px';
      const newH = Math.max(minH, startH - dy);
      np.style.height = newH + 'px';
      np.style.top = (startTop + startH - newH) + 'px';
    } else if (corner === 'r') {
      np.style.width = Math.max(minW, startW + dx) + 'px';
    } else if (corner === 'l') {
      const newW = Math.max(minW, startW - dx);
      np.style.width = newW + 'px';
      np.style.left = (startLeft + startW - newW) + 'px';
    } else if (corner === 'b') {
      np.style.height = Math.max(minH, startH + dy) + 'px';
    } else if (corner === 't') {
      const newH = Math.max(minH, startH - dy);
      np.style.height = newH + 'px';
      np.style.top = (startTop + startH - newH) + 'px';
    }
  });

  document.addEventListener('mouseup', (e) => {
    if (!resizing) return;
    resizing = false;
    document.body.style.userSelect = '';

    // Snap to nearest dock with the new size
    const rect = np.getBoundingClientRect();
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    const wx = window.innerWidth / 2;
    const wy = window.innerHeight / 2;
    let dock;
    if (cx < wx && cy < wy) dock = 'dock-tl';
    else if (cx >= wx && cy < wy) dock = 'dock-tr';
    else if (cx < wx && cy >= wy) dock = 'dock-bl';
    else dock = 'dock-br';

    // Keep the resized width/height, clear position styles, re-dock
    const w = np.style.width;
    const h = np.style.height;
    np.style.left = '';
    np.style.top = '';
    np.style.right = '';
    np.style.bottom = '';
    np.style.width = w;
    np.style.height = h;
    np.classList.add('snapping');
    np.classList.add(dock);
    prefs.setItem('playerDock', dock);
    prefs.setItem('playerWidth', w);
    prefs.setItem('playerHeight', h);
    setTimeout(() => np.classList.remove('snapping'), 300);
  });

  // Restore saved size
  const savedW = prefs.getItem('playerWidth');
  const savedH = prefs.getItem('playerHeight');
  if (savedW) np.style.width = savedW;
  if (savedH) np.style.height = savedH;
})();
