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
        } catch(e) { if(typeof showToast==='function'&&e) showToast(String(e),4000,'error'); }
      }, 500);
    }

    await win.onResized(saveWindow);
    await win.onMoved(saveWindow);
  } catch (e) {
    if (typeof showToast === 'function') showToast(toastFmt('toast.window_listener_failed', { err: e.message || e }), 4000, 'error');
  }
})();

// Auto-load last scan on startup
// Logo click → GitHub (no inline onclick for CSP compliance)
document.getElementById('appLogo')?.addEventListener('click', () => {
  const shell = window.__TAURI_PLUGIN_SHELL__;
  if (shell && shell.open) shell.open('https://github.com/MenkeTechnologies/Audio-Haxor');
  else if (typeof openUpdate === 'function') openUpdate('https://github.com/MenkeTechnologies/Audio-Haxor');
});

// Prevent header stats clicks from bubbling
document.getElementById('headerStats')?.addEventListener('click', (e) => e.stopPropagation());

(async function loadLastScan() {
  showGlobalProgress();
  await (window.__toastReady || Promise.resolve());
  // Load file-backed preferences before anything else
  await prefs.load();
  const uiLoc = prefs.getItem('uiLocale');
  if (typeof reloadAppStrings === 'function') {
    await reloadAppStrings(['de', 'es', 'sv', 'fr'].includes(uiLoc) ? uiLoc : 'en');
  }
  // Ensure stop/resume buttons are hidden on fresh start
  const _stopAll = document.getElementById('btnStopAll');
  const _resumeAll = document.getElementById('btnResumeAll');
  if (_stopAll) _stopAll.style.display = 'none';
  if (_resumeAll) _resumeAll.style.display = 'none';
  restoreSettings();
  initTabDragReorder();
  initMultiFilters();
  initSortPersistence();
  initSettingsSectionDrag();
  loadRecentlyPlayed();
  // BPM/Key/LUFS and waveform/spectrogram caches are now in SQLite —
  // skip eager bulk load (was causing startup hang on large DBs).
  // Data is fetched per-file on demand or via paginated queries.
  renderGlobalTagBar();
  // Xref cache is in SQLite — load lazily when xref tab is used
  if (typeof loadXrefCache === 'function') loadXrefCache().catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); });
  if (typeof restoreFilterStates === 'function') restoreFilterStates();
  if (typeof loadFzfParams === 'function') loadFzfParams();
  if (typeof initSmartPlaylists === 'function') initSmartPlaylists();

  // Show player on startup if enabled and there's play history
  if (prefs.getItem('showPlayerOnStartup') === 'on' && typeof recentlyPlayed !== 'undefined' && recentlyPlayed.length > 0) {
    const np = document.getElementById('audioNowPlaying');
    if (np) {
      np.classList.add('active');
      if (prefs.getItem('playerExpanded') === 'on') {
        np.classList.add('expanded');
        if (typeof renderRecentlyPlayed === 'function') renderRecentlyPlayed();
      }
    }
  }
  if (typeof renderFzfSettings === 'function') renderFzfSettings();

  // Start folder watcher if enabled
  if (prefs.getItem('folderWatch') === 'on' && typeof startFolderWatch === 'function') {
    startFolderWatch();
  }

  // Listen for file watcher change events
  try {
    const { listen } = window.__TAURI__.event || {};
    if (listen) {
      listen('file-watcher-change', (event) => {
        const cats = event.payload?.categories || [];
        showToast(toastFmt('toast.files_changed_rescan', { cats: cats.join(', ') }));
        for (const cat of cats) {
          if (cat === 'audio' && typeof scanAudioSamples === 'function') scanAudioSamples();
          else if (cat === 'daw' && typeof scanDawProjects === 'function') scanDawProjects();
          else if (cat === 'preset' && typeof scanPresets === 'function') scanPresets();
          else if (cat === 'plugin' && typeof scanPlugins === 'function') scanPlugins();
        }
      });
    }
  } catch(e) { if(typeof showToast==='function'&&e) showToast(String(e),4000,'error'); }

  // Dismiss splash screen before loading data so errors are visible
  const splash = document.getElementById('splashScreen');
  if (splash) {
    const ver = document.getElementById('splashVersion');
    try { if (ver) ver.textContent = 'v' + await window.vstUpdater.getVersion(); } catch { if (ver) ver.textContent = 'Ready'; }
    splash.classList.add('fade-out');
    setTimeout(() => splash.remove(), 600);
  }

  // Restore last active tab after splash
  const savedTab = prefs.getItem('activeTab');
  if (savedTab) switchTab(savedTab);

  // Load plugins lazily on first tab view to avoid blocking startup
  loadPluginsFromDb();


  // Auto-load last audio scan from SQLite (paginated — no full array in memory)
  try {
    const stats = await window.vstUpdater.dbAudioStats();
    if (stats && stats.sampleCount > 0) {
      audioTotalUnfiltered = stats.sampleCount;
      audioStatCounts = stats.formatCounts || {};
      audioStatBytes = stats.totalBytes || 0;
      // Don't set array length — creates undefined slots that crash iterators
      updateAudioStats();
      audioCurrentOffset = 0;
      await fetchAudioPage();
      if (typeof startBackgroundAnalysis === 'function') startBackgroundAnalysis();
    }
  } catch (err) {
    showToast(toastFmt('toast.failed_load_audio_scan', { err: err.message || err }), 4000, 'error');
  }

  // Auto-load last DAW scan (paginated from SQLite)
  if (typeof fetchDawPage === 'function') {
    _dawOffset = 0;
    fetchDawPage().catch(err => showToast(toastFmt('toast.failed_load_daw_scan', { err }), 4000, 'error'));
  }

  // Auto-load last preset scan (paginated from SQLite)
  if (typeof fetchPresetPage === 'function') {
    _presetOffset = 0;
    fetchPresetPage().then(() => {
      document.getElementById('btnExportPresets').style.display = allPresets.length > 0 ? '' : 'none';
      if (typeof loadMidiFiles === 'function') loadMidiFiles();
    }).catch(err => showToast(toastFmt('toast.failed_load_preset_scan', { err }), 4000, 'error'));
  }

  // Apply default type filter from settings
  const defaultType = prefs.getItem('defaultTypeFilter');
  if (defaultType && defaultType !== 'all') {
    document.getElementById('typeFilter').value = defaultType;
    filterPlugins();
  }

  hideGlobalProgress();
  renderWelcomeDashboard();
  renderShortcutSettings();
  updateHeaderInfo();
  setInterval(updateHeaderInfo, 1000); // refresh process stats every 1s

  // Auto-scan on launch
  if (prefs.getItem('autoScan') === 'on' && allPlugins.length === 0) {
    scanPlugins().then(() => {
      if (prefs.getItem('autoUpdate') === 'on' && allPlugins.length > 0) {
        checkUpdates();
      }
    });
  }
})();

function renderWelcomeDashboard() {
  const el = document.getElementById('welcomeDashboard');
  if (!el) return;
  const favCount = getFavorites().length;
  const noteCount = Object.keys(getNotes()).length;
  const tagCount = getAllTags().length;
  const recentCount = recentlyPlayed.length;
  el.innerHTML = [
    { value: allPlugins.length, label: 'Plugins', color: 'var(--cyan)' },
    { value: allAudioSamples.length, label: 'Samples', color: 'var(--yellow)' },
    { value: allDawProjects.length, label: 'DAW Projects', color: 'var(--magenta)' },
    { value: allPresets.length, label: 'Presets', color: 'var(--orange)' },
    { value: favCount, label: 'Favorites', color: 'var(--yellow)' },
    { value: noteCount, label: 'Notes', color: 'var(--green)' },
    { value: tagCount, label: 'Tags', color: 'var(--accent)' },
    { value: recentCount, label: 'Recently Played', color: 'var(--cyan)' },
  ].filter(s => s.value > 0).map(s =>
    `<div class="welcome-stat" style="border-left-color: ${s.color};">
      <div class="welcome-stat-value" style="color: ${s.color};">${s.value}</div>
      <div class="welcome-stat-label">${s.label}</div>
    </div>`
  ).join('');
}

function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
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
  if (typeof document !== 'undefined' && document.hidden) return;
  try {
    const s = await window.vstUpdater.getProcessStats();
    const set = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
    set('headerCores', s.numCpus || navigator.hardwareConcurrency || '?');
    set('headerCpu', (s.cpuPercent || 0).toFixed(1) + '%');
    set('headerMem', formatBytes(s.rssBytes));
    set('headerVirt', formatBytes(s.virtualBytes));
    set('headerThreads', s.threads);
    set('headerPool', s.rayonThreads);
    set('headerFds', s.openFds);
    set('headerUptime', formatUptime(s.uptimeSecs));
    set('headerPid', s.pid);
    // Scan counts
    set('headerPlugins', typeof allPlugins !== 'undefined' ? allPlugins.length : 0);
    set('headerSamples', typeof allAudioSamples !== 'undefined' ? allAudioSamples.length : 0);
    set('headerDaw', typeof allDawProjects !== 'undefined' ? allDawProjects.length : 0);
    set('headerPresets', typeof allPresets !== 'undefined' ? allPresets.length : 0);
    set('headerMidi', typeof getMidiCount === 'function' ? getMidiCount() : 0);

    // Scan status badge
    const sc = s.scanner || {};
    const active = [];
    if (sc.pluginScanning) active.push('Plugins');
    if (sc.audioScanning) active.push('Samples');
    if (sc.dawScanning) active.push('DAW');
    if (sc.presetScanning) active.push('Presets');
    if (sc.updateChecking) active.push('Updates');
    const badge = document.getElementById('scanStatusBadge');
    if (badge) {
      if (active.length > 0) {
        badge.style.display = 'flex';
        badge.innerHTML = active.map(s =>
          `<span class="scan-status-item"><span class="spinner" style="width:24px;height:24px;"></span> ${s}</span>`
        ).join('');
      } else {
        badge.style.display = 'none';
      }
    }
  } catch (err) { if (typeof showToast === 'function') showToast(toastFmt('toast.stats_update_failed', { err: err.message || err }), 4000, 'error'); }
}

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) updateHeaderInfo();
  });
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
    showToast(toastFmt('toast.scan_all_failed', { err: err.message || err }), 4000, 'error');
  }

  scanAllRunning = false;
  btn.disabled = false;
  btn.innerHTML = '&#9889; Scan All';
  stopBtn.style.display = 'none';

  // Show resume only if any per-tab resume button is visible (scan was stopped)
  const anyResumeVisible = document.getElementById('btnResumeScan')?.style.display === '' ||
    document.getElementById('btnResumeAudio')?.style.display === '' ||
    document.getElementById('btnResumeDaw')?.style.display === '' ||
    document.getElementById('btnResumePresets')?.style.display === '';
  resumeBtn.style.display = anyResumeVisible ? '' : 'none';
}

async function stopAll() {
  await Promise.all([
    window.vstUpdater.stopScan().catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }),
    window.vstUpdater.stopAudioScan().catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }),
    window.vstUpdater.stopDawScan().catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }),
    window.vstUpdater.stopPresetScan().catch(e => { if(typeof showToast==='function') showToast(String(e),4000,'error'); }),
  ]);
}

async function resumeAll() {
  await scanAll(true);
}
