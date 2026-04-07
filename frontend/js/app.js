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
                } catch (e) {
                    if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
                }
            }, 500);
        }

        await win.onResized(saveWindow);
        await win.onMoved(saveWindow);
    } catch (e) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.window_listener_failed', {err: e.message || e}), 4000, 'error');
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

/** Debounced file-watcher event: scan only subtree roots from `roots_by_category`. */
async function handleFileWatcherChange(event) {
    const payload = event && event.payload ? event.payload : {};
    const rootsByCat = payload.roots_by_category || {};
    const cats = Object.keys(rootsByCat).length > 0
        ? Object.keys(rootsByCat).sort()
        : (payload.categories || []);
    if (!cats.length) return;
    const parts = cats.map((cat) => {
        const r = rootsByCat[cat];
        if (Array.isArray(r) && r.length > 0) {
            return `${cat} (${r.join(', ')})`;
        }
        return cat;
    });
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.files_changed_rescan', {cats: parts.join(', ')}));
    }
    for (const cat of cats) {
        const roots = rootsByCat[cat];
        const targeted = Array.isArray(roots) && roots.length > 0;
        const rootsArg = targeted ? roots : null;
        try {
            if (cat === 'audio' && typeof scanAudioSamples === 'function') {
                await scanAudioSamples(false, null, rootsArg);
            } else if (cat === 'daw' && typeof scanDawProjects === 'function') {
                await scanDawProjects(false, null, rootsArg);
            } else if (cat === 'preset' && typeof scanPresets === 'function') {
                await scanPresets(false, null, rootsArg);
            } else if (cat === 'plugin' && typeof scanPlugins === 'function') {
                await scanPlugins(false, rootsArg);
            } else if (cat === 'pdf' && typeof scanPdfs === 'function') {
                await scanPdfs(false, null, rootsArg);
            } else if (cat === 'midi' && typeof scanMidi === 'function') {
                await scanMidi(false, rootsArg);
            }
        } catch (err) {
            if (typeof showToast === 'function' && err) showToast(String(err), 4000, 'error');
        }
    }
}

(async function loadLastScan() {
    showGlobalProgress();
    await (window.__toastReady || Promise.resolve());
    // Load file-backed preferences before anything else
    await prefs.load();
    const uiLoc = prefs.getItem('uiLocale');
    if (typeof reloadAppStrings === 'function') {
        const locs = window.SUPPORTED_UI_LOCALES;
        await reloadAppStrings(
            Array.isArray(locs) && locs.includes(uiLoc) ? uiLoc : 'en'
        );
    }
    // Ensure stop/resume buttons are hidden on fresh start (do not hide Stop All if Scan All
    // already began — `loadLastScan` awaits i18n and can finish after the user clicked Scan All).
    const _stopAll = document.getElementById('btnStopAll');
    const _resumeAll = document.getElementById('btnResumeAll');
    const _scanAllBtn = document.getElementById('btnScanAll');
    if (_stopAll && !(_scanAllBtn && _scanAllBtn.disabled)) _stopAll.style.display = 'none';
    if (_resumeAll) _resumeAll.style.display = 'none';
    restoreSettings();
    // Restore audio player state — must run post-prefs-load (the IIFEs run too early).
    if (typeof restorePlayerDock === 'function') restorePlayerDock();
    if (typeof restorePlayerDimensions === 'function') restorePlayerDimensions();
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
    if (typeof loadXrefCache === 'function') loadXrefCache().catch(e => {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    });
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
    // renderFzfSettings is invoked from refreshSettingsUI (after reloadAppStrings)

    // Start folder watcher if enabled
    if (prefs.getItem('folderWatch') === 'on' && typeof startFolderWatch === 'function') {
        startFolderWatch();
    }

    // Listen for file watcher change events — backend sends `roots_by_category`
    // so each scan walks only the directories that contained changes (debounced).
    try {
        const {listen} = window.__TAURI__.event || {};
        if (listen) {
            listen('file-watcher-change', (event) => {
                void handleFileWatcherChange(event);
            });
        }
    } catch (e) {
        if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
    }

    // Dismiss splash screen before loading data so errors are visible
    const splash = document.getElementById('splashScreen');
    if (splash) {
        const ver = document.getElementById('splashVersion');
        try {
            if (ver) ver.textContent = 'v' + await window.vstUpdater.getVersion();
        } catch {
            if (ver) ver.textContent = 'Ready';
        }
        splash.classList.add('fade-out');
        setTimeout(() => splash.remove(), 600);
    }

    // Restore last active tab after splash
    const savedTab = prefs.getItem('activeTab');
    if (savedTab) switchTab(savedTab);

    // Load plugins lazily on first tab view to avoid blocking startup
    loadPluginsFromDb();


    // Auto-load first page of each SQLite-backed tab in parallel (separate query seq per tab).
    const _startupLibTasks = [];

    _startupLibTasks.push((async () => {
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
                if (typeof startBackgroundAnalysis === 'function' && prefs.getItem('autoAnalysis') !== 'off') startBackgroundAnalysis();
            }
        } catch (err) {
            showToast(toastFmt('toast.failed_load_audio_scan', {err: err.message || err}), 4000, 'error');
        }
    })());

    if (typeof fetchDawPage === 'function') {
        _dawOffset = 0;
        _startupLibTasks.push((async () => {
            try {
                await fetchDawPage();
                if (typeof refreshDawStatsSnapshot === 'function') refreshDawStatsSnapshot();
            } catch (err) {
                showToast(toastFmt('toast.failed_load_daw_scan', {err}), 4000, 'error');
            }
        })());
    }

    if (typeof fetchPresetPage === 'function') {
        _presetOffset = 0;
        _startupLibTasks.push((async () => {
            try {
                await fetchPresetPage();
                if (typeof updatePresetExportButton === 'function') updatePresetExportButton();
                else {
                    const btn = document.getElementById('btnExportPresets');
                    if (btn) btn.style.display = allPresets.length > 0 ? '' : 'none';
                }
                if (typeof rebuildPresetStats === 'function') rebuildPresetStats();
                if (typeof loadMidiFiles === 'function') loadMidiFiles();
            } catch (err) {
                showToast(toastFmt('toast.failed_load_preset_scan', {err}), 4000, 'error');
            }
        })());
    }

    if (typeof fetchPdfPage === 'function') {
        _pdfOffset = 0;
        _startupLibTasks.push((async () => {
            try {
                await fetchPdfPage();
                if (typeof rebuildPdfStats === 'function') rebuildPdfStats();
            } catch (err) {
                showToast(toastFmt('toast.failed_load_pdf_scan', {err}), 4000, 'error');
            }
        })());
    }

    await Promise.all(_startupLibTasks);

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
    // Refresh process stats every 3s, but pause when tab is hidden
    let _headerInterval = setInterval(updateHeaderInfo, 3000);
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            clearInterval(_headerInterval);
            _headerInterval = null;
        } else if (!_headerInterval) {
            updateHeaderInfo();
            _headerInterval = setInterval(updateHeaderInfo, 3000);
        }
    });

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
        {value: allPlugins.length, label: catalogFmt('ui.welcome.plugins'), color: 'var(--cyan)'},
        {value: allAudioSamples.length, label: catalogFmt('ui.welcome.samples'), color: 'var(--yellow)'},
        {value: allDawProjects.length, label: catalogFmt('ui.welcome.daw_projects'), color: 'var(--magenta)'},
        {value: allPresets.length, label: catalogFmt('ui.welcome.presets'), color: 'var(--orange)'},
        {value: favCount, label: catalogFmt('ui.welcome.favorites'), color: 'var(--yellow)'},
        {value: noteCount, label: catalogFmt('ui.welcome.notes'), color: 'var(--green)'},
        {value: tagCount, label: catalogFmt('ui.welcome.tags'), color: 'var(--accent)'},
        {value: recentCount, label: catalogFmt('ui.welcome.recently_played'), color: 'var(--cyan)'},
    ].filter(s => s.value > 0).map(s =>
        `<div class="welcome-stat" style="border-left-color: ${s.color};">
      <div class="welcome-stat-value" style="color: ${s.color};">${s.value}</div>
      <div class="welcome-stat-label">${s.label}</div>
    </div>`
    ).join('');
}

function formatBytes(bytes) {
    const u = (k, fb) => catalogFmtOrUnit(k, fb);
    if (!bytes || bytes === 0) return '0 ' + u('ui.unit.byte', 'B');
    const keys = ['ui.unit.byte', 'ui.unit.kb', 'ui.unit.mb', 'ui.unit.gb', 'ui.unit.tb'];
    const fallbacks = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), keys.length - 1);
    return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + u(keys[i], fallbacks[i]);
}

function formatUptime(secs) {
    const u = (k, fb) => catalogFmtOrUnit(k, fb);
    if (!secs) return '0' + u('ui.unit.sec', 's');
    if (secs < 60) return secs + u('ui.unit.sec', 's');
    if (secs < 3600) return Math.floor(secs / 60) + u('ui.unit.min', 'm') + ' ' + (secs % 60) + u('ui.unit.sec', 's');
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return h + u('ui.unit.hr', 'h') + ' ' + m + u('ui.unit.min', 'm');
}

/** Pairs: top header strip + stats bar — must stay identical for each category. */
const INVENTORY_COUNT_PAIR_IDS = {
    plugins: ['headerPlugins', 'totalCount'],
    samples: ['headerSamples', 'sampleCount'],
    daw: ['headerDaw', 'dawProjectCount'],
    presets: ['headerPresets', 'presetCountHeader'],
    midi: ['headerMidi', 'midiScanCount'],
    pdf: ['headerPdf', 'pdfCountHeader'],
};

function _fmtInventoryCount(n) {
    const x = Number(n);
    if (!Number.isFinite(x) || x < 0) return '0';
    return x.toLocaleString();
}

/** Map `get_active_scan_inventory_counts` payload to stats-bar keys. */
function mapActiveScanInv(raw) {
    if (!raw || typeof raw !== 'object') return null;
    return {
        plugins: Number(raw.plugins) || 0,
        samples: Number(raw.audio_samples) || 0,
        daw: Number(raw.daw_projects) || 0,
        presets: Number(raw.presets) || 0,
        pdf: Number(raw.pdfs) || 0,
        midi: Number(raw.midi_files) || 0,
    };
}

/**
 * Fallback when IPC is unavailable: table_counts-style totals + in-memory scan progress.
 * Prefer `get_active_scan_inventory_counts` (library: one row per path, all scans) for idle UI.
 */
function computeInventoryCountsLegacy(tc) {
    const tc0 = tc || {};
    let plugins = _pluginTotalUnfiltered || tc0.plugins || 0;
    if (typeof scanProgressCleanup !== 'undefined' && scanProgressCleanup && typeof allPlugins !== 'undefined') {
        plugins = Math.max(plugins, allPlugins.length);
    }
    let samples = audioTotalUnfiltered || tc0.audio_samples || 0;
    if (typeof audioScanProgressCleanup !== 'undefined' && audioScanProgressCleanup) {
        const p = typeof window.__audioScanPendingFound === 'number' ? window.__audioScanPendingFound : 0;
        samples = Math.max(samples, p);
    }
    let daw = (typeof _dawTotalUnfiltered !== 'undefined' && _dawTotalUnfiltered) || tc0.daw_projects || 0;
    if (typeof dawScanProgressCleanup !== 'undefined' && dawScanProgressCleanup) {
        const p = typeof window.__dawScanPendingFound === 'number' ? window.__dawScanPendingFound : 0;
        daw = Math.max(daw, p);
    }
    let presets = (typeof _presetTotalUnfiltered !== 'undefined' && _presetTotalUnfiltered) || tc0.presets || 0;
    if (typeof presetScanProgressCleanup !== 'undefined' && presetScanProgressCleanup) {
        const p = typeof window.__presetScanPendingFound === 'number' ? window.__presetScanPendingFound : 0;
        presets = Math.max(presets, p);
    }
    let pdf = (typeof _pdfTotalUnfiltered !== 'undefined' && _pdfTotalUnfiltered) || tc0.pdfs || 0;
    if (typeof pdfScanProgressCleanup !== 'undefined' && pdfScanProgressCleanup) {
        const p = typeof window.__pdfScanPendingFound === 'number' ? window.__pdfScanPendingFound : 0;
        pdf = Math.max(pdf, p);
    }
    let midi = typeof getMidiCount === 'function' ? getMidiCount() : (tc0.midi_files || 0);
    if (typeof _midiScanProgressCleanup !== 'undefined' && _midiScanProgressCleanup) {
        const p = typeof window.__midiScanPendingFound === 'number' ? window.__midiScanPendingFound : 0;
        midi = Math.max(midi, p);
    }
    return {plugins, samples, daw, presets, pdf, midi};
}

async function resolveInventoryCounts(tc, _scanner) {
    let dbCounts = null;
    try {
        if (typeof window.vstUpdater.getActiveScanInventoryCounts === 'function') {
            const raw = await window.vstUpdater.getActiveScanInventoryCounts();
            dbCounts = mapActiveScanInv(raw);
        }
    } catch (_) {
        dbCounts = null;
    }
    if (!dbCounts) {
        dbCounts = computeInventoryCountsLegacy(tc);
    }
    window.__inventoryCounts = dbCounts;
    applyInventoryCounts(dbCounts);
}

let _refreshInvTimer = null;

function scheduleRefreshInventoryFromDb() {
    if (_refreshInvTimer) clearTimeout(_refreshInvTimer);
    _refreshInvTimer = setTimeout(async () => {
        _refreshInvTimer = null;
        try {
            if (typeof window.vstUpdater.getActiveScanInventoryCounts !== 'function') return;
            const raw = await window.vstUpdater.getActiveScanInventoryCounts();
            const dbCounts = mapActiveScanInv(raw);
            if (!dbCounts) return;
            window.__inventoryCounts = dbCounts;
            applyInventoryCounts(dbCounts);
        } catch (_) {
            /* ignore */
        }
    }, 100);
}

function applyInventoryCounts(counts) {
    if (!counts || typeof counts !== 'object') return;
    const set = (id, val) => {
        const el = document.getElementById(id);
        if (el) el.textContent = val;
    };
    for (const [k, v] of Object.entries(counts)) {
        const ids = INVENTORY_COUNT_PAIR_IDS[k];
        if (!ids || v === undefined || v === null) continue;
        const t = _fmtInventoryCount(v);
        ids.forEach((id) => set(id, t));
    }
}

/** All inventory totals come from SQLite (`get_active_scan_inventory_counts`); scan progress triggers throttled refresh. */
function applyInventoryCountsPartial(_overrides) {
    scheduleRefreshInventoryFromDb();
}

window.applyInventoryCounts = applyInventoryCounts;
window.applyInventoryCountsPartial = applyInventoryCountsPartial;
window.scheduleRefreshInventoryFromDb = scheduleRefreshInventoryFromDb;

async function updateHeaderInfo() {
    if (typeof document !== 'undefined' && document.hidden) return;
    try {
        const s = await window.vstUpdater.getProcessStats();
        window.__scannerFlags = s.scanner || {};
        const set = (id, val) => {
            const el = document.getElementById(id);
            if (el) el.textContent = val;
        };
        set('headerCores', s.numCpus || navigator.hardwareConcurrency || '?');
        set('headerCpu', (s.cpuPercent || 0).toFixed(1) + '%');
        set('headerMem', formatBytes(s.rssBytes));
        set('headerVirt', formatBytes(s.virtualBytes));
        set('headerThreads', s.threads);
        set('headerPool', s.rayonThreads);
        set('headerFds', s.openFds);
        set('headerUptime', formatUptime(s.uptimeSecs));
        set('headerPid', s.pid);
        const tc = s.database?.tables || {};
        await resolveInventoryCounts(tc, s.scanner);

        // Scan status badge
        const sc = s.scanner || {};
        const active = [];
        if (sc.pluginScanning) active.push(catalogFmt('ui.scan_status.plugins'));
        if (sc.audioScanning) active.push(catalogFmt('ui.scan_status.samples'));
        if (sc.dawScanning) active.push(catalogFmt('ui.scan_status.daw'));
        if (sc.presetScanning) active.push(catalogFmt('ui.scan_status.presets'));
        if (sc.pdfScanning) active.push(catalogFmt('ui.scan_status.pdfs'));
        if (sc.midiScanning) active.push(catalogFmt('ui.scan_status.midi'));
        if (sc.updateChecking) active.push(catalogFmt('ui.scan_status.updates'));
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
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.stats_update_failed', {err: err.message || err}), 4000, 'error');
    }
}

if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', () => {
        if (!document.hidden) updateHeaderInfo();
    });
}

let scanAllRunning = false;

/** One in-app toast after Scan All — uses SQLite library counts (not just the current page). */
async function showPostScanAllToast() {
    if (typeof showToast !== 'function' || typeof toastFmt !== 'function') return;
    const stopped =
        document.getElementById('btnResumeScan')?.style.display === '' ||
        document.getElementById('btnResumeAudio')?.style.display === '' ||
        document.getElementById('btnResumeDaw')?.style.display === '' ||
        document.getElementById('btnResumePresets')?.style.display === '' ||
        document.getElementById('btnResumeMidi')?.style.display === '' ||
        document.getElementById('btnResumePdf')?.style.display === '';
    let c;
    try {
        c = await window.vstUpdater.getActiveScanInventoryCounts();
    } catch {
        return;
    }
    const fmt = (n) => (Number(n) || 0).toLocaleString();
    const vars = {
        plugins: fmt(c.plugins),
        samples: fmt(c.audio_samples),
        daw: fmt(c.daw_projects),
        presets: fmt(c.presets),
        pdfs: fmt(c.pdfs),
        midi: fmt(c.midi_files),
    };
    const key = stopped ? 'toast.post_scan_all_stopped' : 'toast.post_scan_all_complete';
    showToast(toastFmt(key, vars), stopped ? 4500 : 3500, stopped ? 'warning' : '');
}

async function scanAll(resume = false) {
    const btn = document.getElementById('btnScanAll');
    const stopBtn = document.getElementById('btnStopAll');
    const resumeBtn = document.getElementById('btnResumeAll');
    if (typeof btnLoading === 'function') btnLoading(btn, true);
    btn.disabled = true;
    {
        btn.textContent = resume ? catalogFmt('ui.js.resuming_btn') : catalogFmt('ui.js.scanning_btn');
    }
    stopBtn.style.display = '';
    resumeBtn.style.display = 'none';
    scanAllRunning = true;

    try {
        // Clear stale unified stop flags from a previous run, then register listeners.
        // `scan_unified` runs after a short delay; without this, it used to reset
        // stop_scan at entry and wipe Stop All pressed during that window.
        if (typeof window.vstUpdater?.prepareUnifiedScan === 'function') {
            await window.vstUpdater.prepareUnifiedScan();
        }
        window.__suppressPostScanToasts = true;
        // Resolve per-type custom roots + resume excludes from prefs / current state.
        const rootsOf = (k) => {
            const v = (prefs.getItem(k) || '').split('\n').map(s => s.trim()).filter(Boolean);
            return v.length ? v : null;
        };
        const audioCustomRoots = rootsOf('audioScanDirs');
        const dawCustomRoots = rootsOf('dawScanDirs');
        const presetCustomRoots = rootsOf('presetScanDirs');
        const pdfCustomRoots = rootsOf('pdfScanDirs');
        const audioExcludePaths = resume && typeof allAudioSamples !== 'undefined'
            ? allAudioSamples.map(s => s.path) : null;
        const dawExcludePaths = resume && typeof allDawProjects !== 'undefined'
            ? allDawProjects.map(p => p.path) : null;
        const presetExcludePaths = resume && typeof allPresets !== 'undefined'
            ? allPresets.map(p => p.path) : null;
        const pdfExcludePaths = resume && typeof allPdfs !== 'undefined'
            ? allPdfs.map(p => p.path) : null;

        // ONE backend walk for all four file types. We use a deferred promise so
        // we can delay the actual backend invocation until AFTER every scanXxx
        // has registered its `*-scan-progress` event listener. Tauri's `listen()`
        // is async (needs a JS↔Rust roundtrip to subscribe); events emitted
        // before registration are dropped. Each scanXxx registers its listener
        // synchronously at the top of its body — so a short timer after Promise
        // microtasks flush is sufficient.
        let unifiedResolve, unifiedReject;
        const unifiedP = new Promise((res, rej) => {
            unifiedResolve = res;
            unifiedReject = rej;
        });
        // scan_unified now streams each batch directly into the DB as the walker
        // runs. The returned result has COUNTS only (no file arrays) so memory
        // stays bounded regardless of scale (works at 6M+ files). Frontend paginates
        // via dbQueryX(offset, limit) after scan.
        const audioP = unifiedP.then(r => ({
            samples: [],
            count: r.audioCount,
            roots: r.audioRoots,
            stopped: r.stopped,
            streamed: true
        }));
        const dawP = unifiedP.then(r => ({
            projects: [],
            count: r.dawCount,
            roots: r.dawRoots,
            stopped: r.stopped,
            streamed: true
        }));
        const presetP = unifiedP.then(r => ({
            presets: [],
            count: r.presetCount,
            roots: r.presetRoots,
            stopped: r.stopped,
            streamed: true
        }));
        const pdfP = unifiedP.then(r => ({
            pdfs: [],
            count: r.pdfCount,
            roots: r.pdfRoots,
            stopped: r.stopped,
            streamed: true
        }));

        // Start per-tab scan functions — each registers its event listener + UI
        // then awaits its derived promise. MIDI runs as its own independent scan
        // (separate walker, separate DB) since scan_unified doesn't yet classify
        // into the MIDI bucket.
        const scansP = Promise.all([
            scanPlugins(resume),
            scanAudioSamples(resume, audioP),
            scanDawProjects(resume, dawP),
            scanPresets(resume, presetP),
            typeof scanPdfs === 'function' ? scanPdfs(resume, pdfP) : Promise.resolve(),
            typeof scanMidi === 'function' ? scanMidi(resume) : Promise.resolve(),
        ]);

        // Kick off the backend walker after listeners are up (100ms is imperceptible
        // next to a filesystem scan but comfortably covers the Tauri listen handshake).
        setTimeout(async () => {
            try {
                unifiedResolve(await window.vstUpdater.scanUnified({
                    audioCustomRoots, audioExcludePaths,
                    dawCustomRoots, dawExcludePaths,
                    dawIncludeBackups: false,
                    presetCustomRoots, presetExcludePaths,
                    pdfCustomRoots, pdfExcludePaths,
                }));
            } catch (e) {
                unifiedReject(e);
            }
        }, 100);

        await scansP;
        await showPostScanAllToast();
    } catch (err) {
        showToast(toastFmt('toast.scan_all_failed', {err: err.message || err}), 4000, 'error');
    } finally {
        window.__suppressPostScanToasts = false;
    }

    scanAllRunning = false;
    btn.disabled = false;
    if (typeof btnLoading === 'function') btnLoading(btn, false);
    btn.innerHTML = catalogFmt('ui.btn.9889_scan_all');
    stopBtn.style.display = 'none';

    // Show resume only if any per-tab resume button is visible (scan was stopped)
    const anyResumeVisible = document.getElementById('btnResumeScan')?.style.display === '' ||
        document.getElementById('btnResumeAudio')?.style.display === '' ||
        document.getElementById('btnResumeDaw')?.style.display === '' ||
        document.getElementById('btnResumePresets')?.style.display === '' ||
        document.getElementById('btnResumeMidi')?.style.display === '' ||
        document.getElementById('btnResumePdf')?.style.display === '';
    resumeBtn.style.display = anyResumeVisible ? '' : 'none';
}

async function stopAll() {
    await Promise.all([
        window.vstUpdater.stopScan().catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        }),
        // One IPC for audio+daw+preset+pdf unified walker (same flags as stop_* per type).
        window.vstUpdater.stopUnifiedScan().catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        }),
        window.vstUpdater.stopMidiScan().catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        }),
        window.vstUpdater.stopUpdates().catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        }),
    ]);
}

async function resumeAll() {
    await scanAll(true);
}
