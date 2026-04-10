// Tauri IPC bridge — replaces Electron's preload.js window.vstUpdater API
const {invoke, convertFileSrc} = window.__TAURI__.core;
const {listen} = window.__TAURI__.event;
/** Used by `audio.js` and other scripts — not every build exposes `convertFileSrc` as a global binding. */
window.convertFileSrc = convertFileSrc;

// App i18n — strings loaded from SQLite via get_app_strings (seeded from i18n/app_i18n_en.json at build).
window.__appStr = {};
window.__toastStr = window.__appStr;

function appFmt(key, vars) {
    const map = window.__appStr;
    let s = map && map[key];
    if (s == null || s === '') return key;
    if (vars && typeof vars === 'object') {
        s = s.replace(/\{(\w+)\}/g, (_, name) => (vars[name] != null && vars[name] !== '') ? String(vars[name]) : '');
    }
    return s;
}

window.appFmt = appFmt;
window.toastFmt = appFmt;

/** Locale codes with seeded i18n in SQLite (must match shipped `i18n/app_i18n_*.json`). */
const SUPPORTED_UI_LOCALES = Object.freeze([
    'cs', 'da', 'de', 'el', 'en', 'es', 'es-419', 'fi', 'fr', 'hi', 'hu', 'id', 'it',
    'ja', 'ko', 'nb', 'nl', 'pl', 'pt', 'pt-BR', 'ro', 'ru', 'sv', 'tr', 'uk', 'vi', 'zh',
]);

function normalizeUiLocale(locale) {
    if (locale == null || locale === '') return null;
    return SUPPORTED_UI_LOCALES.includes(locale) ? locale : null;
}

window.SUPPORTED_UI_LOCALES = SUPPORTED_UI_LOCALES;
window.normalizeUiLocale = normalizeUiLocale;

/** Async exports return promises; catch so menu/click handlers never leave unhandled rejections. */
function runExport(fn) {
    if (typeof fn !== 'function') return;
    Promise.resolve(fn()).catch((e) => {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.export_failed', {err: e.message || e}), 4000, 'error');
        }
    });
}

window.runExport = runExport;
window.__appReady = invoke('get_app_strings', {locale: null}).then((m) => {
    window.__appStr = m || {};
    window.__toastStr = window.__appStr;
    if (typeof applyUiI18n === 'function') applyUiI18n();
}).catch(() => {
});
window.__toastReady = window.__appReady;

async function reloadAppStrings(locale) {
    const loc = normalizeUiLocale(locale);
    try {
        const m = await invoke('get_app_strings', {locale: loc});
        window.__appStr = m || {};
        window.__toastStr = window.__appStr;
        if (typeof applyUiI18n === 'function') applyUiI18n();
        if (typeof window.invalidatePaletteStaticCache === 'function') window.invalidatePaletteStaticCache();
        if (typeof renderFavDirs === 'function') renderFavDirs();
        if (typeof updateBookmarkBtn === 'function') updateBookmarkBtn();
        if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
        if (typeof renderShortcutSettings === 'function') {
            const sf = document.getElementById('shortcutsFilter');
            renderShortcutSettings(sf && sf.value ? sf.value : '');
        }
        try {
            await invoke('refresh_native_menu');
        } catch (_) {
        }
        if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
    } catch (_) {
    }
}

window.reloadAppStrings = reloadAppStrings;

/** Host `playback_status` poll (Rust) when the WebView timer is deferred — same EOF edge as `runEnginePlaybackStatusTick`. */
listen('audio-engine-playback-eof', () => {
    if (typeof window.handleEnginePlaybackEofFromPoll === 'function') {
        window.handleEnginePlaybackEofFromPoll();
    }
});

// ── Menu bar event handler ──
listen('menu-action', (event) => {
    const id = event.payload;
    switch (id) {
        // File
        case 'scan_all':
            if (typeof scanAll === 'function') scanAll();
            break;
        case 'stop_all':
            if (typeof stopAll === 'function') stopAll();
            break;
        case 'export_plugins':
            runExport(exportPlugins);
            break;
        case 'import_plugins':
            if (typeof importPlugins === 'function') importPlugins();
            break;
        case 'export_audio':
            runExport(exportAudio);
            break;
        case 'import_audio':
            if (typeof importAudio === 'function') importAudio();
            break;
        case 'export_daw':
            runExport(exportDaw);
            break;
        case 'import_daw':
            if (typeof importDaw === 'function') importDaw();
            break;
        case 'export_presets':
            runExport(exportPresets);
            break;
        case 'import_presets':
            if (typeof importPresets === 'function') importPresets();
            break;
        case 'open_prefs':
            if (typeof openPrefsFile === 'function') openPrefsFile();
            break;
        case 'open_prefs_app':
            if (typeof switchTab === 'function') switchTab('settings');
            break;
        // Scan
        case 'scan_plugins':
            if (typeof scanPlugins === 'function') scanPlugins();
            break;
        case 'scan_audio':
            if (typeof scanAudioSamples === 'function') scanAudioSamples();
            break;
        case 'scan_daw':
            if (typeof scanDawProjects === 'function') scanDawProjects();
            break;
        case 'scan_presets':
            if (typeof scanPresets === 'function') scanPresets();
            break;
        case 'scan_pdfs':
            if (typeof scanPdfs === 'function') scanPdfs();
            break;
        case 'check_updates':
            if (typeof checkUpdates === 'function') checkUpdates();
            break;
        // View — tabs
        case 'tab_plugins':
            if (typeof switchTab === 'function') switchTab('plugins');
            break;
        case 'tab_samples':
            if (typeof switchTab === 'function') switchTab('samples');
            break;
        case 'tab_daw':
            if (typeof switchTab === 'function') switchTab('daw');
            break;
        case 'tab_presets':
            if (typeof switchTab === 'function') switchTab('presets');
            break;
        case 'tab_favorites':
            if (typeof switchTab === 'function') switchTab('favorites');
            break;
        case 'tab_notes':
            if (typeof switchTab === 'function') switchTab('notes');
            break;
        case 'tab_history':
            if (typeof switchTab === 'function') switchTab('history');
            break;
        case 'tab_settings':
            if (typeof switchTab === 'function') switchTab('settings');
            break;
        case 'tab_files':
            if (typeof switchTab === 'function') switchTab('files');
            break;
        case 'tab_audio_engine':
            if (typeof switchTab === 'function') switchTab('audioEngine');
            break;
        // View — appearance
        case 'toggle_theme':
            if (typeof settingToggleTheme === 'function') settingToggleTheme();
            break;
        case 'toggle_crt':
            if (typeof settingToggleCrt === 'function') settingToggleCrt();
            break;
        case 'reset_columns':
            if (typeof settingResetColumns === 'function') settingResetColumns();
            break;
        case 'reset_tabs':
            if (typeof settingResetTabOrder === 'function') settingResetTabOrder();
            break;
        // Data
        case 'clear_history':
            if (typeof settingClearAllHistory === 'function') settingClearAllHistory();
            break;
        case 'clear_all_databases':
            if (typeof settingClearAllDatabases === 'function') settingClearAllDatabases();
            break;
        case 'clear_kvr':
            if (typeof settingClearKvrCache === 'function') settingClearKvrCache();
            break;
        case 'clear_favorites':
            if (typeof clearFavorites === 'function') clearFavorites();
            break;
        case 'reset_all':
            if (typeof resetAllScans === 'function') resetAllScans();
            break;
        // Playback
        case 'play_pause':
            if (typeof toggleAudioPlayback === 'function') toggleAudioPlayback();
            break;
        case 'toggle_loop':
            if (typeof toggleAudioLoop === 'function') toggleAudioLoop();
            break;
        case 'stop_playback':
            if (typeof stopAudioPlayback === 'function') stopAudioPlayback();
            break;
        case 'expand_player':
            if (typeof togglePlayerExpanded === 'function') togglePlayerExpanded();
            break;
        case 'next_track':
            if (typeof nextTrack === 'function') nextTrack();
            break;
        case 'prev_track':
            if (typeof prevTrack === 'function') prevTrack();
            break;
        case 'toggle_shuffle':
            if (typeof toggleShuffle === 'function') toggleShuffle();
            break;
        case 'toggle_mute':
            if (typeof toggleMute === 'function') toggleMute();
            break;
        // Tools
        case 'find_duplicates':
            if (typeof showDuplicateReport === 'function') showDuplicateReport();
            break;
        case 'dep_graph':
            if (typeof showDepGraph === 'function') showDepGraph();
            break;
        case 'cmd_palette':
            if (typeof openPalette === 'function') openPalette();
            break;
        case 'help_overlay':
            if (typeof toggleHelpOverlay === 'function') toggleHelpOverlay();
            break;
        // Help
        case 'github':
            if (typeof showToast === 'function') showToast(toastFmt('toast.opening_github'));
            if (typeof openUpdate === 'function') openUpdate('https://github.com/MenkeTechnologies/Audio-Haxor');
            break;
        case 'docs':
            if (typeof showToast === 'function') showToast(toastFmt('toast.opening_docs'));
            if (typeof openUpdate === 'function') openUpdate('https://github.com/MenkeTechnologies/Audio-Haxor');
            break;
        // Find (handled by existing Cmd+F)
        case 'find': {
            const activeTab = document.querySelector('.tab-content.active');
            const input = activeTab?.querySelector('input[type="text"]');
            if (input) {
                input.focus();
                input.select();
            }
            break;
        }
    }
});

// Event delegation — replaces inline onclick/oninput/onchange for Tauri v2 CSP compatibility
document.addEventListener('click', (e) => {
    const t = e.target;
    const clickEl = t && t.nodeType === Node.ELEMENT_NODE ? t : t?.parentElement;
    if (!clickEl) return;
    if (clickEl.closest('.col-resize')) return;
    const el = clickEl.closest('[data-action]');
    if (!el) return;
    // If there's a data-action-stop container between the target and the matched action element, skip parent actions
    if (el.dataset.action === 'toggleMetadata') {
        const stop = clickEl.closest('[data-action-stop]');
        if (stop && el.contains(stop)) return;
    }
    const action = el.dataset.action;
    try {
        switch (action) {
            case 'stopCurrentOperation':
                stopCurrentOperation();
                break;
            case 'scanAll':
                scanAll();
                break;
            case 'stopAll':
                stopAll();
                break;
            case 'resumeAll':
                resumeAll();
                break;
            case 'scanPlugins':
                scanPlugins();
                break;
            case 'resumePluginScan':
                scanPlugins(true);
                break;
            case 'stopPluginScan':
                window.vstUpdater.stopScan();
                break;
            case 'checkUpdates':
                checkUpdates();
                break;
            case 'switchTab':
                switchTab(el.dataset.tab);
                break;
            case 'skipUpdate':
                skipUpdate();
                break;
            case 'openNextUpdate':
                openNextUpdate();
                break;
            case 'toggleDirs':
                toggleDirs();
                break;
            case 'clearAllHistory':
                clearAllHistory();
                break;
            case 'scanAudioSamples':
                scanAudioSamples();
                break;
            case 'resumeAudioScan':
                scanAudioSamples(true);
                break;
            case 'stopAudioScan':
                stopAudioScan();
                break;
            case 'toggleAudioPlayback':
                toggleAudioPlayback();
                break;
            case 'toggleAudioLoop':
                toggleAudioLoop();
                break;
            case 'stopAudioPlayback':
                stopAudioPlayback();
                break;
            case 'openUpdate':
                showToast(toastFmt('toast.opening_link'));
                openUpdate(el.dataset.url);
                break;
            case 'openKvr':
                openKvr(el, el.dataset.url, el.dataset.name);
                break;
            case 'openFolder':
                openFolder(el.dataset.path);
                break;
            case 'openAudioFolder':
                openAudioFolder(el.dataset.path);
                break;
            case 'selectScan':
                selectScan(el.dataset.id, el.dataset.type);
                break;
            case 'runDiff':
                runDiff(el.dataset.id);
                break;
            case 'runAudioDiff':
                runAudioDiff(el.dataset.id);
                break;
            case 'deleteScanEntry':
                deleteScanEntry(el.dataset.id);
                break;
            case 'deleteAudioScanEntry':
                deleteAudioScanEntry(el.dataset.id);
                break;
            case 'sortAudio':
                sortAudio(el.dataset.key);
                break;
            case 'loadMoreAudio':
                loadMoreAudio();
                break;
            case 'loadMorePlugins':
                if (typeof loadMorePlugins === 'function') loadMorePlugins();
                break;
            case 'loadMoreMidi':
                if (typeof loadMoreMidi === 'function') loadMoreMidi();
                break;
            case 'loadMoreFavs':
                if (typeof loadMoreFavs === 'function') loadMoreFavs();
                break;
            case 'toggleMetadata':
                toggleMetadata(el.dataset.path, e);
                break;
            case 'previewAudio':
                previewAudio(el.dataset.path);
                break;
            case 'toggleRowLoop':
                toggleRowLoop(el.dataset.path, e);
                break;
            case 'scanDawProjects':
                scanDawProjects();
                break;
            case 'resumeDawScan':
                scanDawProjects(true);
                break;
            case 'stopDawScan':
                stopDawScan();
                break;
            case 'buildXrefIndex':
                buildXrefIndex().then(() => filterDawProjects());
                break;
            case 'showDepGraph':
                showDepGraph();
                break;
            case 'showHeatmapDash':
                if (typeof showHeatmapDashboard === 'function') void showHeatmapDashboard();
                break;
            case 'scanPresets':
                scanPresets();
                break;
            case 'resumePresetScan':
                scanPresets(true);
                break;
            case 'stopPresetScan':
                stopPresetScan();
                break;
            case 'openPresetFolder':
                openPresetFolder(el.dataset.path);
                break;
            case 'sortPreset':
                sortPreset(el.dataset.key);
                break;
            case 'loadMorePresets':
                loadMorePresets();
                break;
            case 'scanMidi':
                if (typeof scanMidi === 'function') scanMidi();
                break;
            case 'resumeMidiScan':
                if (typeof scanMidi === 'function') scanMidi(true);
                break;
            case 'stopMidiScan':
                if (typeof stopMidiScan === 'function') stopMidiScan();
                break;
            case 'scanPdfs':
                scanPdfs();
                break;
            case 'resumePdfScan':
                scanPdfs(true);
                break;
            case 'stopPdfScan':
                stopPdfScan();
                break;
            case 'stopPdfMetadataExtraction':
                if (typeof stopPdfMetadataExtractionUser === 'function') void stopPdfMetadataExtractionUser();
                break;
            case 'openPdfFile':
                openPdfFile(el.dataset.path);
                break;
            case 'sortPdf':
                sortPdf(el.dataset.key);
                break;
            case 'loadMorePdfs':
                loadMorePdfs();
                break;
            case 'filterPdfs':
                filterPdfs();
                break;
            case 'exportPdfs':
                if (typeof exportPdfs === 'function') runExport(exportPdfs);
                break;
            case 'importPdfs':
                if (typeof importPdfs === 'function') importPdfs();
                break;
            case 'filterSettings':
                if (typeof filterSettings === 'function') filterSettings();
                break;
            case 'clearSettingsSearch':
                if (typeof clearSettingsSearch === 'function') clearSettingsSearch();
                break;
            case 'openDawFolder':
                openDawFolder(el.dataset.path);
                break;
            case 'sortDaw':
                sortDaw(el.dataset.key);
                break;
            case 'loadMoreDaw':
                loadMoreDaw();
                break;
            case 'runDawDiff':
                runDawDiff(el.dataset.id);
                break;
            case 'deleteDawScanEntry':
                deleteDawScanEntry(el.dataset.id);
                break;
            case 'runPresetDiff':
                runPresetDiff(el.dataset.id);
                break;
            case 'deletePresetScanEntry':
                deletePresetScanEntry(el.dataset.id);
                break;
            case 'runPdfDiff':
                runPdfDiff(el.dataset.id);
                break;
            case 'deletePdfScanEntry':
                deletePdfScanEntry(el.dataset.id);
                break;
            case 'runMidiDiff':
                runMidiDiff(el.dataset.id);
                break;
            case 'deleteMidiScanEntry':
                deleteMidiScanEntry(el.dataset.id);
                break;
            case 'exportPlugins':
                runExport(exportPlugins);
                break;
            case 'importPlugins':
                importPlugins();
                break;
            case 'exportAudio':
                runExport(exportAudio);
                break;
            case 'importAudio':
                importAudio();
                break;
            case 'exportDaw':
                runExport(exportDaw);
                break;
            case 'exportXrefPlugins':
                if (typeof exportXrefPlugins === 'function') exportXrefPlugins();
                break;
            case 'importDaw':
                importDaw();
                break;
            case 'exportPresets':
                runExport(exportPresets);
                break;
            case 'importPresets':
                importPresets();
                break;
            case 'exportMidi':
                if (typeof exportMidi === 'function') runExport(exportMidi);
                break;
            case 'exportXref':
                if (typeof exportXref === 'function') exportXref();
                break;
            case 'exportSmartPlaylists':
                if (typeof exportSmartPlaylists === 'function') exportSmartPlaylists();
                break;
            case 'settingToggleTheme':
                settingToggleTheme();
                break;
            case 'settingToggleCrt':
                settingToggleCrt();
                break;
            case 'settingToggleNeonGlow':
                settingToggleNeonGlow();
                break;
            case 'settingToggleTagBar':
                if (typeof toggleTagFilterBarVisibility === 'function') toggleTagFilterBarVisibility();
                break;
            case 'settingTagBarPosition': {
                const pos = document.getElementById('settingTagBarPosition')?.value || 'top';
                prefs.setItem('tagBarPosition', pos);
                const bar = document.getElementById('globalTagBar');
                const tabNav = document.querySelector('.tab-nav');
                if (bar && tabNav) {
                    if (pos === 'bottom') {
                        const lastTab = [...document.querySelectorAll('.tab-content')].pop();
                        if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
                    } else {
                        tabNav.parentNode.insertBefore(bar, tabNav);
                    }
                }
                showToast(toastFmt('toast.tag_bar_moved', {pos}));
            }
                break;
            case 'clearFavorites':
                clearFavorites();
                break;
            case 'exportFavorites':
                exportFavorites();
                break;
            case 'importFavorites':
                importFavorites();
                break;
            case 'exportNotes':
                exportNotes();
                break;
            case 'importNotes':
                importNotes();
                break;
            case 'clearAllNotes':
                clearAllNotes();
                break;
            case 'clearGlobalTag':
                clearGlobalTag();
                break;
            case 'hideTagBar': {
                const bar = document.getElementById('globalTagBar');
                if (bar) bar.style.display = 'none';
                prefs.setItem('tagBarVisible', 'off');
                showToast(toastFmt('toast.tag_bar_hidden_filter'));
            }
                break;
            case 'moveTagBar': {
                const bar = document.getElementById('globalTagBar');
                if (!bar) break;
                const main = bar.parentNode;
                const tabNav = document.querySelector('.tab-nav');
                const isTop = bar.compareDocumentPosition(tabNav) & Node.DOCUMENT_POSITION_FOLLOWING;
                if (isTop) {
                    // Move to bottom (after tab content area)
                    const lastTab = [...document.querySelectorAll('.tab-content')].pop();
                    if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
                    prefs.setItem('tagBarPosition', 'bottom');
                    showToast(toastFmt('toast.tag_bar_bottom'));
                } else {
                    // Move to top (before tab nav)
                    if (tabNav) tabNav.parentNode.insertBefore(bar, tabNav);
                    prefs.setItem('tagBarPosition', 'top');
                    showToast(toastFmt('toast.tag_bar_top'));
                }
            }
                break;
            case 'settingResetAllUI':
                settingResetAllUI();
                break;
            case 'settingResetColumns':
                settingResetColumns();
                break;
            case 'settingResetSectionOrder':
                resetSettingsSectionOrder();
                break;
            case 'settingResetTabOrder':
                settingResetTabOrder();
                break;
            case 'settingClearAllHistory':
                settingClearAllHistory();
                break;
            case 'settingClearKvrCache':
                settingClearKvrCache();
                break;
            case 'runBpmKeyLufsAnalysis':
                if (typeof triggerBackgroundBpmKeyLufsAnalysis === 'function') triggerBackgroundBpmKeyLufsAnalysis();
                break;
            case 'stopBpmKeyLufsAnalysis':
                if (typeof triggerStopBackgroundBpmKeyLufsAnalysis === 'function') triggerStopBackgroundBpmKeyLufsAnalysis();
                break;
            case 'settingClearAnalysisCache':
                window.vstUpdater.dbClearCaches().then(() => {
                    if (typeof _bpmCache !== 'undefined') {
                        _bpmCache = {};
                        _keyCache = {};
                        _lufsCache = {};
                    }
                    if (typeof _waveformCache !== 'undefined') {
                        _waveformCache = {};
                        _spectrogramCache = {};
                    }
                    showToast(toastFmt('toast.all_caches_cleared'));
                    if (typeof invalidateDbCacheStatsSnapshot === 'function') invalidateDbCacheStatsSnapshot();
                    if (typeof renderCacheStats === 'function') renderCacheStats();
                }).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
                break;
            case 'settingClearAllDatabases':
                if (typeof settingClearAllDatabases === 'function') settingClearAllDatabases();
                break;
            case 'clearCacheTable': {
                const c = el.dataset.cache;
                if (c) window.vstUpdater.dbClearCacheTable(c).then(() => {
                    if (c === 'bpm' && typeof _bpmCache !== 'undefined') _bpmCache = {};
                    if (c === 'key' && typeof _keyCache !== 'undefined') _keyCache = {};
                    if (c === 'lufs' && typeof _lufsCache !== 'undefined') _lufsCache = {};
                    if (c === 'waveform' && typeof _waveformCache !== 'undefined') _waveformCache = {};
                    if (c === 'spectrogram' && typeof _spectrogramCache !== 'undefined') _spectrogramCache = {};
                    showToast(toastFmt('toast.cache_type_cleared', {cache: c.toUpperCase()}));
                    if (typeof invalidateDbCacheStatsSnapshot === 'function') invalidateDbCacheStatsSnapshot();
                    if (typeof renderCacheStats === 'function') renderCacheStats();
                }).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
            }
                break;
            case 'buildCacheTable': {
                const c = el.dataset.cache;
                if (c === 'xref' && typeof buildXrefIndex === 'function') {
                    showToast(toastFmt('toast.building_xref_index_all_daw'));
                    buildXrefIndex().then(() => {
                        if (typeof renderCacheStats === 'function') renderCacheStats();
                    });
                } else if (c === 'fingerprint') {
                    void (async () => {
                        const paths = await fetchAudioLibraryPathsForFingerprint();
                        if (paths.length === 0) {
                            showToast(toastFmt('toast.no_audio_samples_scan_first'), 4000, 'error');
                            return;
                        }
                        showToast(toastFmt('toast.fingerprint_building_n_slow', {n: paths.length.toLocaleString()}), 4000);
                        try {
                            const res = await window.vstUpdater.buildFingerprintCache(paths);
                            showToast(toastFmt('toast.fingerprint_build_complete_n_cached', {
                                built: res.built.toLocaleString(),
                                cached: res.cached.toLocaleString()
                            }));
                            if (typeof renderCacheStats === 'function') renderCacheStats();
                        } catch (e) {
                            showToast(toastFmt('toast.fingerprint_build_failed', {err: e.message || e}), 4000, 'error');
                        }
                    })();
                }
                break;
            }
            case 'exportSettingsPdf':
                if (typeof exportSettingsPdf === 'function') exportSettingsPdf();
                break;
            case 'exportLogPdf':
                if (typeof exportLogPdf === 'function') exportLogPdf();
                break;
            case 'clearAppLog':
                window.vstUpdater.clearLog().then(() => showToast(toastFmt('toast.log_cleared'))).catch(() => showToast(toastFmt('toast.failed_clear_log'), 4000, 'error'));
                break;
            case 'openLogFile':
                showToast(toastFmt('toast.opening_log_file'));
                window.vstUpdater.getPrefsPath().then(p => {
                    const logPath = p.replace(/preferences\.toml$/, 'app.log');
                    window.vstUpdater.openWithApp(logPath, 'TextEdit').catch(() => window.vstUpdater.openDawProject(logPath).catch(e => {
                        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                    }));
                });
                break;
            case 'openDataDir':
                showToast(toastFmt('toast.opening_data_dir'));
                window.vstUpdater.getPrefsPath().then(p => {
                    const dir = p.replace(/[/\\][^/\\]+$/, '');
                    window.vstUpdater.openPluginFolder(dir).catch(e => {
                        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                    });
                });
                break;
            case 'refreshCacheList':
                if (typeof renderCacheFilesList === 'function') {
                    renderCacheFilesList();
                    showToast(toastFmt('toast.cache_list_refreshed'));
                }
                break;
            case 'refreshCacheStats':
                if (typeof renderCacheStats === 'function') {
                    renderCacheStats();
                    showToast(toastFmt('toast.cache_stats_refreshed'));
                }
                break;
            case 'revealDataFile':
                if (el.dataset.path) {
                    showToast(toastFmt('toast.revealing_file'));
                    window.vstUpdater.openAudioFolder(el.dataset.path).catch(() => showToast(toastFmt('toast.failed_reveal_file'), 4000, 'error'));
                }
                break;
            case 'deleteDataFile':
                if (el.dataset.name && confirm(appFmt('confirm.delete_data_file', {name: el.dataset.name}))) {
                    window.vstUpdater.deleteDataFile(el.dataset.name).then(() => {
                        showToast(toastFmt('toast.deleted_name', {name: el.dataset.name}));
                        if (typeof renderCacheFilesList === 'function') renderCacheFilesList();
                    }).catch(e => showToast(toastFmt('toast.delete_failed', {err: e}), 4000, 'error'));
                }
                break;
            case 'resetAllScans':
                resetAllScans();
                break;
            case 'settingColorScheme':
                settingColorScheme(el.dataset.scheme);
                break;
            case 'settingToggleAutoAnalysis':
                settingToggleAutoAnalysis();
                break;
            case 'settingTogglePdfMetadataAutoExtract':
                settingTogglePdfMetadataAutoExtract();
                break;
            case 'settingToggleAutoScan':
                settingToggleAutoScan();
                break;
            case 'settingToggleFolderWatch':
                settingToggleFolderWatch();
                break;
            case 'settingToggleIncrementalDirectoryScan':
                settingToggleIncrementalDirectoryScan();
                break;
            case 'settingToggleAutoUpdate':
                settingToggleAutoUpdate();
                break;
            case 'settingToggleSingleClickPlay':
                settingToggleSingleClickPlay();
                break;
            case 'settingToggleAutoPlaySampleOnSelect':
                settingToggleAutoPlaySampleOnSelect();
                break;
            case 'settingToggleExpandOnClick':
                settingToggleExpandOnClick();
                break;
            case 'settingToggleShowPlayerOnStartup':
                settingToggleShowPlayerOnStartup();
                break;
            case 'settingToggleAutoplayNext':
                settingToggleAutoplayNext();
                break;
            case 'resetFzfParams':
                resetFzfParams();
                break;
            case 'settingToggleIncludeBackups':
                settingToggleIncludeBackups();
                break;
            case 'settingTogglePruneOldScans':
                settingTogglePruneOldScans();
                break;
            case 'saveBlacklist': {
                const el = document.getElementById('settingBlacklist');
                if (el) {
                    prefs.setItem('blacklistDirs', el.value);
                    showSavedMsg('savedMsgBlacklist');
                    showToast(toastFmt('toast.directory_blacklist_saved'));
                }
            }
                break;
            case 'applyCustomScheme':
                applyCustomScheme();
                break;
            case 'showSavePreset':
                showSavePreset();
                break;
            case 'confirmSavePreset':
                confirmSavePreset();
                break;
            case 'cancelSavePreset':
                cancelSavePreset();
                break;
            case 'deleteCustomSchemes':
                deleteCustomSchemes();
                break;
            case 'loadCustomPreset':
                loadCustomPreset(el.dataset.idx);
                break;
            case 'browseDir':
                browseDir(el.dataset.target);
                break;
            case 'saveCustomDirs':
                saveCustomDirs();
                break;
            case 'saveAudioScanDirs':
                saveAudioScanDirs();
                break;
            case 'saveDawScanDirs':
                saveDawScanDirs();
                break;
            case 'savePresetScanDirs':
                savePresetScanDirs();
                break;
            case 'saveMidiScanDirs':
                if (typeof saveMidiScanDirs === 'function') saveMidiScanDirs();
                break;
            case 'savePdfScanDirs':
                savePdfScanDirs();
                break;
            case 'openPrefsFile':
                showToast(toastFmt('toast.opening_preferences'));
                openPrefsFile();
                break;
            case 'toggleRegex':
                toggleRegex(el);
                break;
            case 'collapsePlayer':
                collapsePlayer();
                break;
            case 'hidePlayer':
                hidePlayer();
                break;
            case 'showPlayer':
                showPlayer();
                break;
            case 'prevTrack':
                prevTrack();
                break;
            case 'nextTrack':
                nextTrack();
                break;
            case 'toggleShuffle':
                toggleShuffle();
                break;
            case 'favCurrentTrack':
                favCurrentTrack();
                break;
            case 'tagCurrentTrack':
                tagCurrentTrack();
                break;
            case 'toggleMute':
                toggleMute();
                break;
            case 'toggleReversePlayback':
                if (typeof toggleReversePlayback === 'function') void toggleReversePlayback();
                break;
            case 'resetEq':
                resetEq();
                break;
            case 'clearRecentlyPlayed':
                clearRecentlyPlayed();
                break;
            case 'exportRecentlyPlayed':
                exportRecentlyPlayed();
                break;
            case 'importRecentlyPlayed':
                importRecentlyPlayed();
                break;
            case 'toggleMono':
                toggleMono();
                break;
            case 'toggleEqSection':
                toggleEqSection();
                break;
            case 'setAbA':
                setAbLoopStart();
                break;
            case 'setAbB':
                setAbLoopEnd();
                break;
            case 'clearAbLoop':
                clearAbLoop();
                break;
            case 'createTag':
                createNewTag();
                break;
            case 'closeMetaRow':
                closeMetaRow();
                break;
        }
    } catch (err) {
        console.error('Action error:', action, err);
        showToast(toastFmt('toast.action_error', {err: err.message || err}), 4000, 'error');
    }
});
document.addEventListener('dblclick', (e) => {
    // DAW projects — open in DAW
    const dawRow = e.target.closest('#dawTableBody tr[data-daw-path]');
    if (dawRow) {
        e.preventDefault();
        const filePath = dawRow.dataset.dawPath;
        const name = dawRow.querySelector('.col-name')?.textContent || filePath.split('/').pop();
        const dawName = dawRow.querySelector('.format-badge')?.textContent || 'DAW';
        dawRow.classList.remove('row-opening');
        void dawRow.offsetWidth;
        dawRow.classList.add('row-opening');
        showToast(toastFmt('toast.opening_in_daw', {name, daw: dawName}));
        window.vstUpdater.openDawProject(filePath).catch(err => {
            showToast(toastFmt('toast.daw_not_installed', {daw: dawName, err}), 4000, 'error');
        });
        return;
    }

    // Audio samples — start playback
    const audioRow = e.target.closest('#audioTableBody tr[data-audio-path]');
    if (audioRow && !e.target.closest('.col-actions')) {
        e.preventDefault();
        previewAudio(audioRow.getAttribute('data-audio-path'));
        return;
    }

    // Plugins — open on KVR
    const pluginCard = e.target.closest('#pluginList .plugin-card');
    if (pluginCard && !e.target.closest('.plugin-actions')) {
        e.preventDefault();
        const kvrBtn = pluginCard.querySelector('[data-action="openKvr"]');
        if (kvrBtn) {
            openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name);
        }
        return;
    }

    // Presets — reveal in Finder
    const presetRow = e.target.closest('#presetTableBody tr[data-preset-path]');
    if (presetRow && !e.target.closest('.col-actions')) {
        e.preventDefault();
        const presetName = presetRow.querySelector('td')?.textContent || 'preset';
        openPresetFolder(presetRow.dataset.presetPath);
        showToast(toastFmt('toast.revealing_preset_finder', {presetName}));
        return;
    }

    // PDFs — open in default app (double-click)
    const pdfRow = e.target.closest('#pdfTableBody tr[data-pdf-path]');
    if (pdfRow && !e.target.closest('.col-actions')) {
        e.preventDefault();
        const name = pdfRow.querySelector('td:nth-child(2)')?.textContent?.trim()
            || pdfRow.dataset.pdfPath?.split('/').pop() || 'PDF';
        window.vstUpdater.openFileDefault(pdfRow.dataset.pdfPath)
            .then(() => showToast(toastFmt('toast.opening_pdf_default_app', {name})))
            .catch(err => showToast(toastFmt('toast.failed_open_pdf', {err: err.message || err}), 4000, 'error'));
        return;
    }
});
document.addEventListener('input', (e) => {
    const action = e.target.dataset.action;
    if (_filterRegistry[action]) {
        applyFilterDebounced(action);
        return;
    }
    if (action === 'filterSettings') {
        if (typeof filterSettingsDebounced === 'function') filterSettingsDebounced();
        return;
    }
    if (action === 'setVolume') setAudioVolume(e.target.value);
    else if (action === 'setPlaybackSpeed') setPlaybackSpeed(e.target.value);
    else if (action === 'setEqLow') setEqBand('low', e.target.value);
    else if (action === 'setEqMid') setEqBand('mid', e.target.value);
    else if (action === 'setEqHigh') setEqBand('high', e.target.value);
    else if (action === 'setGain') setPreampGain(e.target.value);
    else if (action === 'setPan') setPan(e.target.value);
    else if (action === 'settingPageSize') settingUpdatePageSize(e.target.value);
    else if (action === 'settingFlushInterval') settingUpdateFlushInterval(e.target.value);
    else if (action === 'settingTooltipHoverDelay') settingUpdateTooltipHoverDelay(e.target.value);
    else if (action === 'settingThreadMultiplier') settingUpdateThreadMultiplier(e.target.value);
    else if (action === 'settingSqliteReadPoolExtra') settingUpdateSqliteReadPoolExtra(e.target.value);
    else if (action === 'settingPruneOldScansKeep') settingUpdatePruneOldScansKeep(e.target.value);
    else if (action === 'settingChannelBuffer') settingUpdateChannelBuffer(e.target.value);
    else if (action === 'settingBatchSize') settingUpdateBatchSize(e.target.value);
    else if (action === 'settingFdLimit') settingUpdateFdLimit(e.target.value);
    else if (action === 'settingVizFps') settingUpdateVizFps(e.target.value);
    else if (action === 'settingWfCacheMax') settingUpdateWfCacheMax(e.target.value);
    else if (action === 'settingAnalysisPause') settingUpdateAnalysisPause(e.target.value);
    else if (action === 'settingMaxRecent') settingUpdateMaxRecent(e.target.value);
});
document.addEventListener('change', (e) => {
    const action = e.target.dataset.action;
    if (_filterRegistry[action]) {
        applyFilter(action);
        return;
    }
    if (action === 'setPlaybackSpeed') {
        setPlaybackSpeed(e.target.value);
        showToast(toastFmt('toast.speed_value', {value: e.target.value}));
    } else if (action === 'settingDefaultTypeFilter') {
        settingSaveSelect('defaultTypeFilter', e.target.value);
        showToast(toastFmt('toast.default_type_filter', {value: e.target.value}));
    } else if (action === 'settingPluginSort') {
        settingSaveSelect('pluginSort', e.target.value);
        showToast(toastFmt('toast.plugin_sort', {value: e.target.value}));
    } else if (action === 'settingAudioSort') {
        settingSaveSelect('audioSort', e.target.value);
        showToast(toastFmt('toast.audio_sort', {value: e.target.value}));
    } else if (action === 'settingDawSort') {
        settingSaveSelect('dawSort', e.target.value);
        showToast(toastFmt('toast.daw_sort', {value: e.target.value}));
    } else if (action === 'settingPresetSort') {
        settingSaveSelect('presetSort', e.target.value);
        showToast(toastFmt('toast.preset_sort', {value: e.target.value}));
    } else if (action === 'settingMidiSort') {
        settingSaveSelect('midiSort', e.target.value);
        showToast(toastFmt('toast.midi_sort', {value: e.target.value}));
    } else if (action === 'settingPdfSort') {
        settingSaveSelect('pdfSort', e.target.value);
        showToast(toastFmt('toast.pdf_sort', {value: e.target.value}));
    } else if (action === 'settingTagBarPosition') {
        const pos = e.target.value || 'top';
        prefs.setItem('tagBarPosition', pos);
        const bar = document.getElementById('globalTagBar');
        const tabNav = document.querySelector('.tab-nav');
        if (bar && tabNav) {
            if (pos === 'bottom') {
                const lastTab = [...document.querySelectorAll('.tab-content')].pop();
                if (lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
            } else {
                tabNav.parentNode.insertBefore(bar, tabNav);
            }
        }
        showToast(toastFmt('toast.tag_bar_moved', {pos}));
    } else if (action === 'settingUiLocale') {
        const v = e.target.value || 'en';
        prefs.setItem('uiLocale', v);
        if (typeof showToast === 'function') showToast(toastFmt('toast.locale_changed'), 4000, '');
    } else if (action === 'settingLogVerbosity') {
        settingSaveSelect('logVerbosity', e.target.value);
        if (typeof showToast === 'function') showToast(toastFmt('toast.log_verbosity_saved'));
    } else if (action === 'settingAutoplayNextSource') {
        if (typeof settingSetAutoplayNextSource === 'function') settingSetAutoplayNextSource(e.target.value);
    }
});

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    const isMac = navigator.platform.includes('Mac');
    const mod = isMac ? e.metaKey : e.ctrlKey;

    // Escape — clear search or stop operation
    if (e.key === 'Escape') {
        const focused = document.activeElement;
        if (focused?.tagName === 'INPUT' && focused.value) {
            focused.value = '';
            focused.dispatchEvent(new Event('input', {bubbles: true}));
        } else if (currentOperation) {
            stopCurrentOperation();
        }
    }

    // Cmd/Ctrl+1-7 — handled by native menu accelerators
    // Cmd+F — handled by native menu accelerator (find)
});

function showToast(message, duration = 2500, type = '') {
    if (type === 'error' && window.vstUpdater?.appendLog) {
        window.vstUpdater.appendLog('TOAST_ERROR: ' + message);
    }
    if (typeof window.isUiIdleHeavyCpu === 'function' && window.isUiIdleHeavyCpu()) {
        return;
    }
    const container = document.getElementById('toastContainer');
    if (!container) return;
    const el = document.createElement('div');
    el.className = 'toast' + (type ? ` toast-${type}` : '');
    el.textContent = message;
    const fadeStart = (duration - 300) / 1000;
    el.style.animation = `toast-in 0.3s ease-out, toast-out 0.3s ease-in ${fadeStart}s forwards`;
    container.appendChild(el);
    setTimeout(() => el.remove(), duration);
}

/** When the window is backgrounded, minimized, or the page is hidden (see `ui-idle.js`), drop slide-in toasts so nothing stacks off-screen. */
if (typeof document !== 'undefined') {
    document.addEventListener('ui-idle-heavy-cpu', (e) => {
        if (!e.detail || !e.detail.idle) return;
        const container = document.getElementById('toastContainer');
        if (container) container.innerHTML = '';
    });
}

window.vstUpdater = {
    getVersion: () => invoke('get_version'),
    getAppStrings: (locale) => invoke('get_app_strings', {locale: locale ?? null}),
    getToastStrings: (locale) => invoke('get_toast_strings', {locale: locale ?? null}),
    scanPlugins: (customRoots, excludePaths) => invoke('scan_plugins', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    stopScan: () => invoke('stop_scan'),
    /**
     * `onScanProgress`, `onUpdateProgress`, `on*ScanProgress`, `onPdfMetadataProgress` return
     * `Promise<UnlistenFn>` — await before the matching `invoke` or the first backend `emit` can be dropped.
     */
    onScanProgress: (callback) => listen('scan-progress', (event) => callback(event.payload)),
    checkUpdates: (plugins) => invoke('check_updates', {plugins}),
    stopUpdates: () => invoke('stop_updates'),
    onUpdateProgress: (callback) => listen('update-progress', (event) => callback(event.payload)),
    resolveKvr: (directUrl, pluginName) => invoke('resolve_kvr', {directUrl, pluginName}),
    openUpdateUrl: (url) => invoke('open_update_url', {url}),
    openPluginFolder: (path) => invoke('open_plugin_folder', {pluginPath: path}),
    // History
    getScans: () => invoke('history_get_scans'),
    getScanDetail: (id) => invoke('history_get_detail', {id}),
    deleteScan: (id) => invoke('history_delete', {id}),
    clearHistory: () => invoke('history_clear'),
    diffScans: (oldId, newId) => invoke('history_diff', {oldId, newId}),
    getLatestScan: () => invoke('history_latest'),
    // Audio samples
    scanAudioSamples: (customRoots, excludePaths) => invoke('scan_audio_samples', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    stopAudioScan: () => invoke('stop_audio_scan'),
    onAudioScanProgress: (callback) => listen('audio-scan-progress', (event) => callback(event.payload)),
    openAudioFolder: (path) => invoke('open_audio_folder', {filePath: path}),
    getAudioMetadata: (filePath) => invoke('get_audio_metadata', {filePath}),
    // Audio history
    saveAudioScan: (samples, roots) => invoke('audio_history_save', {samples, roots: roots || null}),
    getAudioScans: () => invoke('audio_history_get_scans'),
    getAudioScanDetail: (id) => invoke('audio_history_get_detail', {id}),
    deleteAudioScan: (id) => invoke('audio_history_delete', {id}),
    clearAudioHistory: () => invoke('audio_history_clear'),
    getLatestAudioScan: () => invoke('audio_history_latest'),
    diffAudioScans: (oldId, newId) => invoke('audio_history_diff', {oldId, newId}),
    // DAW projects
    scanDawProjects: (customRoots, excludePaths) => invoke('scan_daw_projects', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    // Presets
    scanPresets: (customRoots, excludePaths) => invoke('scan_presets', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    stopPresetScan: () => invoke('stop_preset_scan'),
    onPresetScanProgress: (callback) => listen('preset-scan-progress', (event) => callback(event.payload)),
    openPresetFolder: (path) => invoke('open_preset_folder', {filePath: path}),
    savePresetScan: (presets, roots) => invoke('preset_history_save', {presets, roots: roots || null}),
    getPresetScans: () => invoke('preset_history_get_scans'),
    getPresetScanDetail: (id) => invoke('preset_history_get_detail', {id}),
    deletePresetScan: (id) => invoke('preset_history_delete', {id}),
    clearPresetHistory: () => invoke('preset_history_clear'),
    getLatestPresetScan: () => invoke('preset_history_latest'),
    diffPresetScans: (oldId, newId) => invoke('preset_history_diff', {oldId, newId}),
    // MIDI — fully independent from preset scanner.
    scanMidiFiles: (customRoots, excludePaths) => invoke('scan_midi_files', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    stopMidiScan: () => invoke('stop_midi_scan'),
    onMidiScanProgress: (callback) => listen('midi-scan-progress', (event) => callback(event.payload)),
    saveMidiScan: (midiFiles, roots) => invoke('midi_history_save', {midiFiles, roots: roots || null}),
    getMidiScans: () => invoke('midi_history_get_scans'),
    getMidiScanDetail: (id) => invoke('midi_history_get_detail', {id}),
    deleteMidiScan: (id) => invoke('midi_history_delete', {id}),
    clearMidiHistory: () => invoke('midi_history_clear'),
    getLatestMidiScan: () => invoke('midi_history_latest'),
    diffMidiScans: (oldId, newId) => invoke('midi_history_diff', {oldId, newId}),
    dbQueryMidi: (params) => invoke('db_query_midi', params || {}),
    dbMidiFilterStats: (search, formatFilter, searchRegex) => invoke('db_midi_filter_stats', {
        search: search || null,
        format_filter: formatFilter || null,
        search_regex: !!searchRegex,
    }),
    // PDFs
    scanPdfs: (customRoots, excludePaths) => invoke('scan_pdfs', {
        customRoots: customRoots || null,
        excludePaths: excludePaths || null
    }),
    stopPdfScan: () => invoke('stop_pdf_scan'),
    // Unified scan: walks the union of audio/daw/preset/pdf roots ONCE and
    // classifies files in place. Emits the same per-type progress events as
    // the individual scans, so existing frontend listeners work unchanged.
    scanUnified: (args) => invoke('scan_unified', {
        audioCustomRoots: args.audioCustomRoots || null,
        audioExcludePaths: args.audioExcludePaths || null,
        dawCustomRoots: args.dawCustomRoots || null,
        dawExcludePaths: args.dawExcludePaths || null,
        dawIncludeBackups: args.dawIncludeBackups || false,
        presetCustomRoots: args.presetCustomRoots || null,
        presetExcludePaths: args.presetExcludePaths || null,
        midiCustomRoots: args.midiCustomRoots || null,
        midiExcludePaths: args.midiExcludePaths || null,
        pdfCustomRoots: args.pdfCustomRoots || null,
        pdfExcludePaths: args.pdfExcludePaths || null,
    }),
    prepareUnifiedScan: () => invoke('prepare_unified_scan'),
    stopUnifiedScan: () => invoke('stop_unified_scan'),
    getUnifiedScanRun: () => invoke('get_unified_scan_run', {}),
    onPdfScanProgress: (callback) => listen('pdf-scan-progress', (event) => callback(event.payload)),
    openPdfFile: (path) => invoke('open_pdf_file', {filePath: path}),
    openFileDefault: (path) => invoke('open_file_default', {filePath: path}),
    savePdfScan: (pdfs, roots) => invoke('pdf_history_save', {pdfs, roots: roots || null}),
    getPdfScans: () => invoke('pdf_history_get_scans'),
    getPdfScanDetail: (id) => invoke('pdf_history_get_detail', {id}),
    deletePdfScan: (id) => invoke('pdf_history_delete', {id}),
    clearPdfHistory: () => invoke('pdf_history_clear'),
    getLatestPdfScan: () => invoke('pdf_history_latest'),
    diffPdfScans: (oldId, newId) => invoke('pdf_history_diff', {oldId, newId}),
    exportPdfsJson: (pdfs, filePath) => invoke('export_pdfs_json', {pdfs, filePath}),
    exportPdfsDsv: (pdfs, filePath) => invoke('export_pdfs_dsv', {pdfs, filePath}),
    importPdfsJson: (filePath) => invoke('import_pdfs_json', {filePath}),
    exportPresetsJson: (presets, filePath) => invoke('export_presets_json', {presets, filePath}),
    importPresetsJson: (filePath) => invoke('import_presets_json', {filePath}),
    importAudioJson: (filePath) => invoke('import_audio_json', {filePath}),
    importDawJson: (filePath) => invoke('import_daw_json', {filePath}),
    stopDawScan: () => invoke('stop_daw_scan'),
    onDawScanProgress: (callback) => listen('daw-scan-progress', (event) => callback(event.payload)),
    openDawFolder: (path) => invoke('open_daw_folder', {filePath: path}),
    openDawProject: (path) => invoke('open_daw_project', {filePath: path}),
    extractProjectPlugins: (path) => invoke('extract_project_plugins', {filePath: path}),
    estimateBpm: (path) => invoke('estimate_bpm', {filePath: path}),
    detectAudioKey: (path) => invoke('detect_audio_key', {filePath: path}),
    measureLufs: (path) => invoke('measure_lufs', {filePath: path}),
    readCacheFile: (name) => invoke('read_cache_file', {name}),
    writeCacheFile: (name, data) => invoke('write_cache_file', {name, data}),
    getWalkerStatus: () => invoke('get_walker_status'),
    appendLog: (msg) => invoke('append_log', {msg}).catch(e => {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    }),
    readLog: () => invoke('read_log'),
    clearLog: () => invoke('clear_log'),
    listDataFiles: () => invoke('list_data_files'),
    readBwproject: (path) => invoke('read_bwproject', {filePath: path}),
    deleteDataFile: (name) => invoke('delete_data_file', {name}),
    // DAW history
    saveDawScan: (projects, roots) => invoke('daw_history_save', {projects, roots: roots || null}),
    getDawScans: () => invoke('daw_history_get_scans'),
    getDawScanDetail: (id) => invoke('daw_history_get_detail', {id}),
    deleteDawScan: (id) => invoke('daw_history_delete', {id}),
    clearDawHistory: () => invoke('daw_history_clear'),
    getLatestDawScan: () => invoke('daw_history_latest'),
    diffDawScans: (oldId, newId) => invoke('daw_history_diff', {oldId, newId}),
    // KVR cache
    getKvrCache: () => invoke('kvr_cache_get'),
    updateKvrCache: (entries) => invoke('kvr_cache_update', {entries}),
    // Export / Import
    exportJson: (plugins, filePath) => invoke('export_plugins_json', {plugins, filePath}),
    exportCsv: (plugins, filePath) => invoke('export_plugins_csv', {plugins, filePath}),
    importJson: (filePath) => invoke('import_plugins_json', {filePath}),
    exportAudioJson: (samples, filePath) => invoke('export_audio_json', {samples, filePath}),
    exportAudioDsv: (samples, filePath) => invoke('export_audio_dsv', {samples, filePath}),
    exportDawJson: (projects, filePath) => invoke('export_daw_json', {projects, filePath}),
    exportDawDsv: (projects, filePath) => invoke('export_daw_dsv', {projects, filePath}),
    exportPresetsDsv: (presets, filePath) => invoke('export_presets_dsv', {presets, filePath}),
    exportToml: (data, filePath) => invoke('export_toml', {data, filePath}),
    importToml: (filePath) => invoke('import_toml', {filePath}),
    exportPdf: (title, headers, rows, filePath) => invoke('export_pdf', {title, headers, rows, filePath}),
    writeTextFile: (filePath, contents) => invoke('write_text_file', {filePath, contents}),
    openWithApp: (filePath, appName) => invoke('open_with_app', {filePath, appName}),
    // File browser
    listDirectory: (dirPath) => invoke('fs_list_dir', {dirPath}),
    deleteFile: (filePath) => invoke('delete_file', {filePath}),
    renameFile: (oldPath, newPath) => invoke('rename_file', {oldPath, newPath}),
    getHomeDir: () => invoke('get_home_dir'),
    // Similarity
    findSimilarSamples: (filePath, candidatePaths, maxResults) => invoke('find_similar_samples', {
        filePath,
        candidatePaths,
        maxResults: maxResults || 20
    }),
    buildFingerprintCache: (candidatePaths) => invoke('build_fingerprint_cache', {candidatePaths}),
    findContentDuplicates: () => invoke('find_content_duplicates', {}),
    pdfMetadataGet: (paths) => invoke('pdf_metadata_get', {paths}),
    pdfMetadataExtractAbort: () => invoke('pdf_metadata_extract_abort'),
    pdfMetadataExtractBatch: (paths) => invoke('pdf_metadata_extract_batch', {paths}),
    pdfMetadataUnindexed: (limit) => invoke('pdf_metadata_unindexed', {limit: limit || 100000}),
    onPdfMetadataProgress: (callback) => listen('pdf-metadata-progress', (event) => callback(event.payload)),
    readAlsXml: (filePath) => invoke('read_als_xml', {filePath}),
    readProjectFile: (filePath) => invoke('read_project_file', {filePath}),
    // Preferences (file-backed)
    getProcessStats: () => invoke('get_process_stats'),
    /** Subprocess `audio-engine`: same RSS/VIRT/CPU/thread/FD probes as the main header (`get_process_stats`). */
    getAudioEngineProcessStats: () => invoke('get_audio_engine_process_stats'),
    getActiveScanInventoryCounts: () => invoke('get_active_scan_inventory_counts'),
    openPrefsFile: () => invoke('open_prefs_file'),
    getPrefsPath: () => invoke('get_prefs_path'),
    prefsGetAll: () => invoke('prefs_get_all'),
    prefsSet: (key, value) => invoke('prefs_set', {key, value}),
    prefsRemove: (key) => invoke('prefs_remove', {key}),
    prefsSaveAll: (prefs) => invoke('prefs_save_all', {prefs}),
    // Database-backed queries (SQLite)
    dbQueryAudio: (params) => invoke('db_query_audio', {params}),
    dbAudioStats: (scanId) => invoke('db_audio_stats', {scanId: scanId || null}),
    dbListScans: () => invoke('db_list_scans'),
    dbUpdateBpm: (path, bpm) => invoke('db_update_bpm', {path, bpm}),
    dbUpdateKey: (path, key) => invoke('db_update_key', {path, key}),
    dbUpdateLufs: (path, lufs) => invoke('db_update_lufs', {path, lufs}),
    dbUpdateAnalysis: (path, bpm, key, lufs) => invoke('db_update_analysis', {path, bpm, key, lufs}),
    dbGetAnalysis: (path) => invoke('db_get_analysis', {path}),
    dbBackfillAudioMeta: (paths) => invoke('db_backfill_audio_meta', {paths}),
    dbUnanalyzedPaths: (limit) => invoke('db_unanalyzed_paths', {limit: limit || 100}),
    dbMigrateJson: () => invoke('db_migrate_json'),
    dbCacheStats: () => invoke('db_cache_stats'),
    dbClearCaches: () => invoke('db_clear_caches'),
    dbClearCacheTable: (table) => invoke('db_clear_cache_table', {table}),
    // File watcher
    startFileWatcher: (dirs) => invoke('start_file_watcher', {dirs}),
    stopFileWatcher: () => invoke('stop_file_watcher'),
    getFileWatcherStatus: () => invoke('get_file_watcher_status'),
    // MIDI
    getMidiInfo: (filePath) => invoke('get_midi_info', {filePath}),
    /** AudioEngine (persistent stdin loop): `{ cmd, ... }` → JSON. Includes `engine_state`, `start_output_stream` (`start_playback` for file PCM decode), `playback_load` / `playback_pause` / `playback_seek` / `playback_set_dsp` / `playback_set_speed` / `playback_set_reverse` / `playback_set_loop` / `playback_status` (optional `spectrum` + `spectrum_fft_size` / `spectrum_bins` / `spectrum_sr_hz` when output is running) / `playback_stop`, `playback_set_inserts` (VST3/AU bundle paths or cached `fileOrIdentifier`; stop stream first), `playback_open_insert_editor` / `playback_close_insert_editor` (native plug-in UI; `slot` is chain index), `stop_output_stream`, `start_input_stream` / `stop_input_stream`, `set_output_tone`, `plugin_chain` (scan + inserts; while scanning: `scan_done` / `scan_total` / `scan_skipped` / `scan_cache_loaded` / `scan_current_format` / `scan_current_name`). Stream `output_stream_status` / `input_stream_status` include `current_buffer_frames` (actual buffer length). UI: `audio-engine.js` + `audio.js` (`enginePlaybackStart`, waveform `pointerdown` seek, DSP). */
    audioEngineInvoke: (request) => invoke('audio_engine_invoke', {request}),
    audioEngineRestart: () => invoke('audio_engine_restart'),
    audioEngineEofWatchdogStart: () => invoke('audio_engine_eof_watchdog_start'),
    audioEngineEofWatchdogStop: () => invoke('audio_engine_eof_watchdog_stop'),
    batchAnalyze: (paths) => invoke('batch_analyze', {paths}),
    dbQueryPlugins: (params) => invoke('db_query_plugins', params || {}),
    dbQueryDaw: (params) => invoke('db_query_daw', params || {}),
    dbQueryPresets: (params) => invoke('db_query_presets', params || {}),
    dbQueryPdfs: (params) => invoke('db_query_pdfs', params || {}),
    /** One blocking-pool task: six `LIMIT 6` inventory queries (Cmd+K preview). */
    dbQueryPalettePreview: (search) => invoke('db_query_palette_preview', {search: search || ''}),
    /** Full audio library paths (SQLite `audio_library`) — not the in-memory paginated slice. */
    dbAudioLibraryPaths: () => invoke('db_audio_library_paths', {}),
    dbPdfStats: (scanId) => invoke('db_pdf_stats', {scanId: scanId || null}),
    dbAudioFilterStats: (search, formatFilter, searchRegex) => invoke('db_audio_filter_stats', {
        search: search || null,
        format_filter: formatFilter || null,
        search_regex: !!searchRegex,
    }),
    dbDawFilterStats: (search, dawFilter, searchRegex) => invoke('db_daw_filter_stats', {
        search: search || null,
        daw_filter: dawFilter || null,
        search_regex: !!searchRegex,
    }),
    dbPresetFilterStats: (search, formatFilter, searchRegex) => invoke('db_preset_filter_stats', {
        search: search || null,
        format_filter: formatFilter || null,
        search_regex: !!searchRegex,
    }),
    dbPluginFilterStats: (search, typeFilter, searchRegex) => invoke('db_plugin_filter_stats', {
        search: search || null,
        type_filter: typeFilter || null,
        search_regex: !!searchRegex,
    }),
    dbPdfFilterStats: (search, searchRegex) => invoke('db_pdf_filter_stats', {
        search: search || null,
        search_regex: !!searchRegex,
    }),
};

// ── Preferences layer (file-backed, survives reboots) ──
// In-memory cache loaded from Rust on startup; writes go to both cache and disk.
const prefs = {
    _cache: {},
    _loaded: false,
    async load() {
        this._cache = await window.vstUpdater.prefsGetAll();
        this._loaded = true;
    },
    getItem(key) {
        const val = this._cache[key];
        if (val === undefined || val === null) return null;
        return typeof val === 'string' ? val : JSON.stringify(val);
    },
    getObject(key, fallback) {
        const val = this._cache[key];
        if (val === undefined || val === null) return fallback;
        if (typeof val === 'string') {
            try {
                return JSON.parse(val);
            } catch {
                return fallback;
            }
        }
        return val;
    },
    setItem(key, value) {
        this._cache[key] = value;
        window.vstUpdater.prefsSet(key, value).catch(() => showToast(toastFmt('toast.failed_save_preference'), 4000, 'error'));
    },
    removeItem(key) {
        delete this._cache[key];
        window.vstUpdater.prefsRemove(key).catch(() => showToast(toastFmt('toast.failed_save_preference'), 4000, 'error'));
    },
};

let allPlugins = [];
let pluginsWithUpdates = [];
let currentOperation = null; // 'scan' or 'updates'
let AUDIO_PAGE_SIZE = 200;
let DAW_PAGE_SIZE = 200;
/** Paginated MIDI / PDF tabs — kept in sync with Settings → Performance → Table Page Size. */
let MIDI_PAGE_SIZE = 200;
let PDF_PAGE_SIZE = 200;

// Common audio plugin manufacturers where bundle ID doesn't match KVR slug
const KVR_MANUFACTURER_MAP = {
    'madronalabs': 'madrona-labs',
    'audiothing': 'audio-thing',
    'audiodamage': 'audio-damage',
    'soundtoys': 'soundtoys',
    'native-instruments': 'native-instruments',
    'plugin-alliance': 'plugin-alliance',
    'softube': 'softube',
    'izotope': 'izotope',
    'eventide': 'eventide',
    'arturia': 'arturia',
    'u-he': 'u-he',
};

// Display app version in header
window.vstUpdater.getVersion().then(v => {
    const vStr = 'v' + v;
    const el = document.getElementById('appVersion');
    if (el) el.textContent = vStr;
    const runAboutLine = () => {
        if (typeof window.updateSettingsAboutVersionLine === 'function') {
            window.updateSettingsAboutVersionLine();
        }
    };
    runAboutLine();
    setTimeout(runAboutLine, 0);
}).catch(() => {
});

function showStopButton() {
    const btn = document.getElementById('btnStop') || document.getElementById('btnStopAll');
    if (btn) btn.style.display = '';
}

/** True while **Scan All** holds `btnScanAll` disabled (only `scanAll()` in app.js does this). */
function isScanAllInProgress() {
    const scanAllBtn = document.getElementById('btnScanAll');
    return !!(scanAllBtn && scanAllBtn.disabled);
}

function hideStopButton() {
    const btn = document.getElementById('btnStop') || document.getElementById('btnStopAll');
    if (btn) {
        // `checkUpdates` / `resolveKvrDownloads` reuse `#btnStopAll`; hiding it when they finish
        // must not clear Stop All while Scan All is still running (other scanners or unified walk).
        if (btn.id === 'btnStopAll' && isScanAllInProgress()) {
            currentOperation = null;
            return;
        }
        btn.style.display = 'none';
    }
    currentOperation = null;
}

async function stopCurrentOperation() {
    if (currentOperation === 'scan') {
        await window.vstUpdater.stopScan();
    } else if (currentOperation === 'updates') {
        await window.vstUpdater.stopUpdates();
    } else if (currentOperation === 'kvr-resolve') {
        stopKvrResolve();
    } else if (currentOperation === 'audio-scan') {
        await window.vstUpdater.stopAudioScan();
    } else if (currentOperation === 'daw-scan') {
        await window.vstUpdater.stopDawScan();
    } else if (currentOperation === 'preset-scan') {
        await window.vstUpdater.stopPresetScan();
    }
}

// ── Global error logging ──
window.addEventListener('error', (e) => {
    const msg = `ERROR: ${e.message} at ${e.filename}:${e.lineno}:${e.colno}`;
    window.vstUpdater?.appendLog(msg);
});
window.addEventListener('unhandledrejection', (e) => {
    const msg = `UNHANDLED_REJECTION: ${e.reason?.message || e.reason || 'unknown'}`;
    window.vstUpdater?.appendLog(msg);
});
