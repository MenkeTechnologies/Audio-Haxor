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

audioPlayer.addEventListener('ended', () => {
  if (!audioLooping) {
    updatePlayBtnStates();
    updateNowPlayingBtn();
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
  const FLUSH_INTERVAL = parseInt(prefs.getItem('flushInterval') || '300', 10);
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
    btn.innerHTML = `&#8635; ${pendingFound} found`;
    progressFill.style.width = '';
    progressFill.style.animation = 'progress-indeterminate 1.5s ease-in-out infinite';

    // Incrementally append matching rows (cap DOM at 2000 during scan)
    const search = (document.getElementById('audioSearchInput').value || '').toLowerCase();
    const fmt = document.getElementById('audioFormatFilter').value;
    const matching = toAdd.filter(s => {
      if (fmt !== 'all' && s.format !== fmt) return false;
      if (search && !s.name.toLowerCase().includes(search) && !s.path.toLowerCase().includes(search) && !s.format.toLowerCase().includes(search)) return false;
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
    try { await window.vstUpdater.saveAudioScan(allAudioSamples, result.roots); } catch (e) { console.error('Failed to save audio scan history:', e); }
    if (result.stopped && allAudioSamples.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    if (audioScanProgressCleanup) { audioScanProgressCleanup(); audioScanProgressCleanup = null; }
    flushPendingSamples();
    tableWrap.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${err.message}</p></div>`;
  }

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
        <th data-action="sortAudio" data-key="name" style="width: 28%;">Name <span class="sort-arrow" id="sortArrowName">&#9660;</span><span class="col-resize"></span></th>
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

function filterAudioSamples() {
  const search = (document.getElementById('audioSearchInput').value || '').toLowerCase();
  const format = document.getElementById('audioFormatFilter').value;

  filteredAudioSamples = allAudioSamples.filter(s => {
    if (format !== 'all' && s.format !== format) return false;
    if (search && !s.name.toLowerCase().includes(search) && !s.path.toLowerCase().includes(search) && !s.format.toLowerCase().includes(search)) return false;
    return true;
  });

  sortAudioArray();
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
  return `<tr${rowClass} data-audio-path="${ep}" data-action="toggleMetadata" data-path="${ep}">
    <td class="col-name" title="${escapeHtml(s.name)}">${escapeHtml(s.name)}</td>
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
    document.getElementById('npName').textContent = sample ? `${sample.name}.${sample.format.toLowerCase()}` : filePath.split('/').pop();
    updatePlayBtnStates();
    updateNowPlayingBtn();
  } catch (err) {
    console.error('Preview failed:', err);
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
  document.getElementById('audioNowPlaying').classList.remove('active');
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
}

function updateNowPlayingBtn() {
  const btn = document.getElementById('npBtnPlay');
  if (audioPlayer.paused) {
    btn.innerHTML = '&#9654;';
    btn.classList.remove('playing');
  } else {
    btn.innerHTML = '&#9646;&#9646;';
    btn.classList.add('playing');
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
