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
  // Load file-backed preferences before anything else
  await prefs.load();
  restoreSettings();

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
    console.error('Failed to load last plugin scan:', err);
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
    console.error('Failed to load last audio scan:', err);
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
    console.error('Failed to load last DAW scan:', err);
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
    console.error('Failed to load last preset scan:', err);
  }

  // Apply default type filter from settings
  const defaultType = prefs.getItem('defaultTypeFilter');
  if (defaultType && defaultType !== 'all') {
    document.getElementById('typeFilter').value = defaultType;
    filterPlugins();
  }

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

async function updateHeaderInfo() {
  try {
    const stats = await window.vstUpdater.getProcessStats();
    const memEl = document.getElementById('headerMem');
    const thrEl = document.getElementById('headerThreads');
    if (memEl) memEl.textContent = formatBytes(stats.memBytes);
    if (thrEl) thrEl.textContent = stats.threads;
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
    console.error('Scan all failed:', err);
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
