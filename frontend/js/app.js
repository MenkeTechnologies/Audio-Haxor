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

  // Auto-scan on launch
  if (prefs.getItem('autoScan') === 'on' && allPlugins.length === 0) {
    scanPlugins().then(() => {
      if (prefs.getItem('autoUpdate') === 'on' && allPlugins.length > 0) {
        checkUpdates();
      }
    });
  }
})();
