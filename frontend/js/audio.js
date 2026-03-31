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

function loadRecentlyPlayed() {
  recentlyPlayed = prefs.getObject('recentlyPlayed', []);
}
function saveRecentlyPlayed() {
  prefs.setItem('recentlyPlayed', recentlyPlayed);
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
    `<tr id="audioLoadMore"><td colspan="6" style="text-align: center; padding: 12px; color: var(--text-muted); cursor: pointer;" data-action="loadMoreAudio">
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
  const ep = escapePath(s.path);
  const isPlaying = audioPlayerPath === s.path;
  const rowClass = isPlaying ? ' class="row-playing"' : '';
  const checked = batchSelected.has(s.path) ? ' checked' : '';
  return `<tr${rowClass} data-audio-path="${ep}" data-action="toggleMetadata" data-path="${ep}">
    <td class="col-cb" data-action-stop><input type="checkbox" class="batch-cb"${checked}></td>
    <td class="col-name" title="${escapeHtml(s.name)}">${noteIndicator(s.path)}${highlightMatch(s.name, _lastAudioSearch, _lastAudioMode)}</td>
    <td class="col-format"><span class="format-badge ${fmtClass}">${s.format}</span></td>
    <td class="col-size">${s.sizeFormatted}</td>
    <td class="col-date">${s.modified}</td>
    <td class="col-path" title="${escapeHtml(s.path)}">${escapeHtml(s.directory)}</td>
    <td class="col-actions" data-action-stop>
      <button class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${ep}" title="Preview">
        ${isPlaying && !audioPlayer.paused ? '&#9646;&#9646;' : '&#9654;'}
      </button>
      <button class="btn-small btn-loop${isPlaying && audioLooping ? ' active' : ''}" data-action="toggleRowLoop" data-path="${ep}" title="Loop">&#8634;</button>
      <button class="btn-small btn-folder" data-action="openAudioFolder" data-path="${ep}" title="Reveal in Finder">&#128193;</button>
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
    audioPlayer.src = convertFileSrc(filePath);
    audioPlayer.loop = audioLooping;
    audioPlayerPath = filePath;
    audioPlayer.play();

    // Show now-playing bar
    const np = document.getElementById('audioNowPlaying');
    np.classList.add('active');
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
  document.getElementById('npTime').textContent = `${formatTime(cur)} / ${formatTime(dur)}`;
  if (dur > 0) {
    document.getElementById('npProgress').style.width = ((cur / dur) * 100) + '%';
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
  metaRow.innerHTML = `<td colspan="6"><div class="audio-meta-panel" style="justify-items: center;"><div class="spinner" style="width: 18px; height: 18px;"></div></div></td>`;
  row.after(metaRow);

  // Fetch metadata
  try {
    const meta = await window.vstUpdater.getAudioMetadata(filePath);
    if (expandedMetaPath !== filePath) return; // user closed it

    let items = '';
    items += metaItem('File Name', meta.fileName);
    items += metaItem('Format', meta.format);
    items += metaItem('Size', formatAudioSize(meta.sizeBytes));
    items += metaItem('Full Path', meta.fullPath);

    if (meta.sampleRate) items += metaItem('Sample Rate', meta.sampleRate.toLocaleString() + ' Hz');
    if (meta.bitsPerSample) items += metaItem('Bit Depth', meta.bitsPerSample + '-bit');
    if (meta.channels) items += metaItem('Channels', meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch');
    if (meta.duration) items += metaItem('Duration', formatTime(meta.duration));
    if (meta.byteRate) items += metaItem('Byte Rate', formatAudioSize(meta.byteRate) + '/s');

    items += metaItem('Created', new Date(meta.created).toLocaleString());
    items += metaItem('Modified', new Date(meta.modified).toLocaleString());
    items += metaItem('Accessed', new Date(meta.accessed).toLocaleString());
    items += metaItem('Permissions', meta.permissions);

    metaRow.innerHTML = `<td colspan="6"><div class="audio-meta-panel">${items}</div></td>`;
  } catch (err) {
    metaRow.innerHTML = `<td colspan="6"><div class="audio-meta-panel"><span style="color: var(--red);">Failed to load metadata</span></div></td>`;
  }
}

function metaItem(label, value) {
  return `<div class="meta-item"><span class="meta-label">${label}</span><span class="meta-value">${escapeHtml(String(value || '—'))}</span></div>`;
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
  list.innerHTML = recentlyPlayed.map(r => {
    const isActive = r.path === audioPlayerPath;
    const isPlaying = isActive && !audioPlayer.paused;
    return `<div class="np-history-item${isActive ? ' active' : ''}" data-action="playRecent" data-path="${escapeHtml(r.path)}">
      <span class="np-h-icon">${isPlaying ? '&#9654;' : '&#9835;'}</span>
      <span class="np-h-name" title="${escapeHtml(r.path)}">${escapeHtml(r.name)}</span>
      <span class="np-h-format">${r.format}</span>
      ${r.size ? `<span class="np-h-dur">${r.size}</span>` : ''}
    </div>`;
  }).join('');
}

function togglePlayerExpanded() {
  const np = document.getElementById('audioNowPlaying');
  np.classList.toggle('expanded');
  if (np.classList.contains('expanded')) {
    renderRecentlyPlayed();
  }
}

function collapsePlayer() {
  document.getElementById('audioNowPlaying').classList.remove('expanded');
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

function updateMetaLine() {
  const el = document.getElementById('npMetaLine');
  if (!el || !audioPlayerPath) { if (el) el.textContent = ''; return; }
  const sample = allAudioSamples.find(s => s.path === audioPlayerPath);
  if (!sample) { el.textContent = audioPlayerPath.split('/').pop(); return; }
  const parts = [sample.format, sample.sizeFormatted];
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
