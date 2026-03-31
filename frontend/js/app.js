// Save window size and position (debounced) using Tauri window events
(async function setupWindowListeners() {
  let _timer = null;
  let _pending = {};
  try {
    const win = window.__TAURI__.webviewWindow
      ? window.__TAURI__.webviewWindow.getCurrentWebviewWindow()
      : null;
    if (!win) return;

    function saveWindow() {
      clearTimeout(_timer);
      _timer = setTimeout(async () => {
        try {
          const size = await win.outerSize();
          const pos = await win.outerPosition();
          prefs.setItem('window', {
            width: size.width, height: size.height,
            x: pos.x, y: pos.y,
          });
        } catch {}
      }, 500);
    }

    await win.onResized(saveWindow);
    await win.onMoved(saveWindow);
  } catch (e) {
    console.error('Failed to set up window listeners:', e);
  }
})();

// Auto-load last scan on startup
(async function loadLastScan() {
  showGlobalProgress();
  // Load file-backed preferences before anything else
  await prefs.load();
  restoreSettings();
  initTabDragReorder();
  initMultiFilters();
  initSortPersistence();
  initSettingsSectionDrag();
  loadRecentlyPlayed();
  renderGlobalTagBar();

  try {
    const latest = await window.vstUpdater.getLatestScan();
    if (latest && latest.plugins && latest.plugins.length > 0) {
      allPlugins = latest.plugins;

      // Restore cached KVR results
      try {
        const kvrCache = await window.vstUpdater.getKvrCache();
        applyKvrCache(allPlugins, kvrCache);
      } catch {}

      document.getElementById('totalCount').textContent = allPlugins.length;
      document.getElementById('btnCheckUpdates').disabled = false;
      document.getElementById('toolbar').style.display = 'flex';

      // Update stat counters from cached data
      const withUpdates = allPlugins.filter(p => p.hasUpdate).length;
      const unknown = allPlugins.filter(p => p.source === 'not-found').length;
      const upToDate = allPlugins.filter(p => !p.hasUpdate && p.source && p.source !== 'not-found').length;
      if (withUpdates || unknown || upToDate) {
        document.getElementById('updateCount').textContent = withUpdates;
        document.getElementById('unknownCount').textContent = unknown;
        document.getElementById('upToDateCount').textContent = upToDate;
      }

      const dirsSection = document.getElementById('dirsSection');
      dirsSection.style.display = 'block';
      document.getElementById('dirsList').innerHTML = buildDirsTable(latest.directories || [], allPlugins);

      renderPlugins(allPlugins);
      // Resume resolving KVR links for plugins not yet cached
      resolveKvrDownloads();
    }
  } catch (err) {
    showToast(`Failed to load plugin scan — ${err.message || err}`, 4000, 'error');
  }

  // Auto-load last audio scan
  try {
    const latestAudio = await window.vstUpdater.getLatestAudioScan();
    if (latestAudio && latestAudio.samples && latestAudio.samples.length > 0) {
      allAudioSamples = latestAudio.samples;
      rebuildAudioStats();
      filterAudioSamples();
    }
  } catch (err) {
    showToast(`Failed to load audio scan — ${err.message || err}`, 4000, 'error');
  }

  // Auto-load last DAW scan
  try {
    const latestDaw = await window.vstUpdater.getLatestDawScan();
    if (latestDaw && latestDaw.projects && latestDaw.projects.length > 0) {
      allDawProjects = latestDaw.projects;
      rebuildDawStats();
      filterDawProjects();
    }
  } catch (err) {
    showToast(`Failed to load DAW scan — ${err.message || err}`, 4000, 'error');
  }

  // Auto-load last preset scan
  try {
    const latestPresets = await window.vstUpdater.getLatestPresetScan();
    if (latestPresets && latestPresets.presets && latestPresets.presets.length > 0) {
      allPresets = latestPresets.presets;
      rebuildPresetStats();
      filterPresets();
    }
  } catch (err) {
    showToast(`Failed to load preset scan — ${err.message || err}`, 4000, 'error');
  }

  // Apply default type filter from settings
  const defaultType = prefs.getItem('defaultTypeFilter');
  if (defaultType && defaultType !== 'all') {
    document.getElementById('typeFilter').value = defaultType;
    filterPlugins();
  }

  hideGlobalProgress();
  updateHeaderInfo();
  setInterval(updateHeaderInfo, 2000); // refresh process stats every 2s

  // Auto-scan on launch
  if (prefs.getItem('autoScan') === 'on' && allPlugins.length === 0) {
    scanPlugins().then(() => {
      if (prefs.getItem('autoUpdate') === 'on' && allPlugins.length > 0) {
        checkUpdates();
      }
    });
  }
})();

function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function formatUptime(secs) {
  if (!secs) return '0s';
  if (secs < 60) return secs + 's';
  if (secs < 3600) return Math.floor(secs / 60) + 'm ' + (secs % 60) + 's';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h + 'h ' + m + 'm';
}

async function updateHeaderInfo() {
  try {
    const s = await window.vstUpdater.getProcessStats();
    const set = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
    set('headerCpu', s.cpuPercent + '%');
    set('headerMem', formatBytes(s.rssBytes));
    set('headerVirt', formatBytes(s.virtualBytes));
    set('headerThreads', s.threads);
    set('headerPool', s.rayonThreads);
    set('headerFds', s.openFds);
    set('headerUptime', formatUptime(s.uptimeSecs));
    set('headerPid', s.pid);
  } catch {}
}

let scanAllRunning = false;

async function scanAll(resume = false) {
  const btn = document.getElementById('btnScanAll');
  const stopBtn = document.getElementById('btnStopAll');
  const resumeBtn = document.getElementById('btnResumeAll');
  btn.disabled = true;
  btn.textContent = resume ? 'Resuming...' : 'Scanning...';
  stopBtn.style.display = '';
  resumeBtn.style.display = 'none';
  scanAllRunning = true;

  try {
    await Promise.all([
      scanPlugins(resume),
      scanAudioSamples(resume),
      scanDawProjects(resume),
      scanPresets(resume),
    ]);
  } catch (err) {
    showToast(`Scan all failed — ${err.message || err}`, 4000, 'error');
  }

  scanAllRunning = false;
  btn.disabled = false;
  btn.innerHTML = '&#9889; Scan All';
  stopBtn.style.display = 'none';

  // Show resume if any scan was stopped with partial results
  const hasPartial = allPlugins.length > 0 || allAudioSamples.length > 0 ||
    allDawProjects.length > 0 || allPresets.length > 0;
  const anyResumeVisible = document.getElementById('btnResumeScan')?.style.display !== 'none' ||
    document.getElementById('btnResumeAudio')?.style.display !== 'none' ||
    document.getElementById('btnResumeDaw')?.style.display !== 'none' ||
    document.getElementById('btnResumePresets')?.style.display !== 'none';
  if (anyResumeVisible && hasPartial) {
    resumeBtn.style.display = '';
  }
}

async function stopAll() {
  await Promise.all([
    window.vstUpdater.stopScan().catch(() => {}),
    window.vstUpdater.stopAudioScan().catch(() => {}),
    window.vstUpdater.stopDawScan().catch(() => {}),
    window.vstUpdater.stopPresetScan().catch(() => {}),
  ]);
}

async function resumeAll() {
  await scanAll(true);
}
