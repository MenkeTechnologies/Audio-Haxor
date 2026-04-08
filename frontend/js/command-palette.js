// ── Command Palette (Cmd+K) ──

let _paletteOpen = false;
let _paletteQuery = '';
let _paletteResults = [];
let _paletteSelected = 0;

const PALETTE_MAX = 50;

/** Monotonic id so stale palette DB searches never repaint after a newer query. */
let _paletteDbSeq = 0;

/** Cached palette rows (tabs + actions + toggles); labels from appFmt. Cleared on locale change. */
let _paletteStaticItemsCache = null;

/** Full list for one palette session (static + dynamic). Avoids rebuilding bookmarks on every keystroke. */
let _paletteSessionItems = null;

/** Row type badges: one catalogFmt per type per session, not per row. */
let _paletteOpenTypeLabels = null;

function _paletteSwitchTab(id) {
    if (typeof switchTab === 'function') switchTab(id);
}

/**
 * Library hits for Cmd+K from SQLite (same scope as tab search: deduped by path across scans).
 * In-memory `allPlugins` / `allDawProjects` / etc. are only one paginated page (or empty) after
 * restart — the palette must not rely on them for discovery.
 */
async function fetchPaletteDatabaseItems(query) {
    const q = query.trim();
    if (q.length < 2) return [];
    const vu = window.vstUpdater;
    if (!vu || typeof vu.dbQueryPalettePreview !== 'function') return [];

    let rPlug;
    let rAud;
    let rDaw;
    let rPreset;
    let rPdf;
    let rMidi;
    try {
        const combined = await vu.dbQueryPalettePreview(q);
        if (!combined) return [];
        rPlug = combined.plugins;
        rAud = combined.audio;
        rDaw = combined.daw;
        rPreset = combined.presets;
        rPdf = combined.pdfs;
        rMidi = combined.midi;
    } catch {
        return [];
    }

    const out = [];

    if (rPlug && rPlug.plugins) {
        for (const p of rPlug.plugins) {
            out.push({
                type: 'plugin',
                name: p.name,
                detail: p.type + (p.manufacturer ? ' · ' + p.manufacturer : ''),
                fields: [p.name, p.type, p.manufacturer || ''],
                icon: '&#9889;',
                action: () => {
                    _paletteSwitchTab('plugins');
                    setTimeout(() => {
                        const el = document.getElementById('pluginSearchInput');
                        if (el) {
                            el.value = p.name;
                            if (typeof filterPlugins === 'function') filterPlugins();
                        }
                    }, 100);
                },
            });
        }
    }

    if (rAud && rAud.samples) {
        for (const s of rAud.samples) {
            out.push({
                type: 'sample',
                name: s.name,
                detail: (s.format || '') + (s.sizeFormatted ? ' · ' + s.sizeFormatted : ''),
                fields: [s.name, s.path || '', s.format || ''],
                icon: '&#127925;',
                action: () => {
                    _paletteSwitchTab('samples');
                    setTimeout(() => {
                        const el = document.getElementById('audioSearchInput');
                        if (el) {
                            el.value = s.name;
                            if (typeof filterAudioSamples === 'function') filterAudioSamples();
                        }
                    }, 100);
                },
            });
        }
    }

    if (rDaw && rDaw.projects) {
        for (const d of rDaw.projects) {
            out.push({
                type: 'daw',
                name: d.name,
                detail: d.daw + ' · ' + (d.sizeFormatted || ''),
                fields: [d.name, d.daw, d.format],
                icon: '&#127911;',
                action: () => {
                    _paletteSwitchTab('daw');
                    setTimeout(() => {
                        const el = document.getElementById('dawSearchInput');
                        if (el) {
                            el.value = d.name;
                            if (typeof filterDawProjects === 'function') filterDawProjects();
                        }
                    }, 100);
                },
            });
        }
    }

    if (rPreset && rPreset.presets) {
        for (const p of rPreset.presets) {
            out.push({
                type: 'preset',
                name: p.name,
                detail: p.format,
                fields: [p.name, p.format],
                icon: '&#127924;',
                action: () => {
                    _paletteSwitchTab('presets');
                    setTimeout(() => {
                        const el = document.getElementById('presetSearchInput');
                        if (el) {
                            el.value = p.name;
                            if (typeof filterPresets === 'function') filterPresets();
                        }
                    }, 100);
                },
            });
        }
    }

    if (rPdf && rPdf.pdfs) {
        for (const p of rPdf.pdfs) {
            out.push({
                type: 'pdf',
                name: p.name,
                detail: p.sizeFormatted || '',
                fields: [p.name, p.directory || ''],
                icon: '&#128196;',
                action: () => {
                    _paletteSwitchTab('pdf');
                    setTimeout(() => {
                        const el = document.getElementById('pdfSearchInput');
                        if (el) {
                            el.value = p.name;
                            if (typeof filterPdfs === 'function') filterPdfs();
                        }
                    }, 100);
                },
            });
        }
    }

    const midiFiles = rMidi && (rMidi.midiFiles || rMidi.midi_files);
    if (midiFiles) {
        for (const m of midiFiles) {
            out.push({
                type: 'midi',
                name: m.name,
                detail: (m.format || '') + (m.sizeFormatted ? ' · ' + m.sizeFormatted : ''),
                fields: [m.name, m.path || '', m.format || ''],
                icon: '&#127929;',
                action: () => {
                    _paletteSwitchTab('midi');
                    setTimeout(() => {
                        const el = document.getElementById('midiSearchInput');
                        if (el) {
                            el.value = m.name;
                            if (typeof filterMidi === 'function') filterMidi();
                        }
                    }, 100);
                },
            });
        }
    }

    return out;
}

function buildPaletteStaticItems() {
    const items = [];

    // Tabs — always available
    const tabs = [
        {type: 'tab', name: appFmt('menu.tab_plugins'), icon: '&#9889;', action: () => _paletteSwitchTab('plugins')},
        {type: 'tab', name: appFmt('menu.tab_samples'), icon: '&#127925;', action: () => _paletteSwitchTab('samples')},
        {type: 'tab', name: appFmt('menu.tab_daw'), icon: '&#127911;', action: () => _paletteSwitchTab('daw')},
        {type: 'tab', name: appFmt('menu.tab_presets'), icon: '&#127924;', action: () => _paletteSwitchTab('presets')},
        {
            type: 'tab',
            name: appFmt('menu.tab_favorites'),
            icon: '&#9733;',
            action: () => _paletteSwitchTab('favorites')
        },
        {type: 'tab', name: appFmt('menu.tab_notes'), icon: '&#128221;', action: () => _paletteSwitchTab('notes')},
        {type: 'tab', name: appFmt('menu.tab_tags'), icon: '&#127991;', action: () => _paletteSwitchTab('tags')},
        {type: 'tab', name: appFmt('menu.tab_history'), icon: '&#128197;', action: () => _paletteSwitchTab('history')},
        {type: 'tab', name: appFmt('menu.tab_files'), icon: '&#128193;', action: () => _paletteSwitchTab('files')},
        {
            type: 'tab',
            name: appFmt('menu.tab_visualizer'),
            icon: '&#127911;',
            action: () => _paletteSwitchTab('visualizer')
        },
        {type: 'tab', name: appFmt('menu.tab_walkers'), icon: '&#128270;', action: () => _paletteSwitchTab('walkers')},
        {type: 'tab', name: appFmt('menu.tab_audio_engine'), icon: '&#127898;', action: () => _paletteSwitchTab('audioEngine')},
        {type: 'tab', name: appFmt('menu.tab_midi'), icon: '&#127929;', action: () => _paletteSwitchTab('midi')},
        {type: 'tab', name: appFmt('menu.tab_pdf'), icon: '&#128196;', action: () => _paletteSwitchTab('pdf')},
        {type: 'tab', name: appFmt('menu.tab_settings'), icon: '&#9881;', action: () => _paletteSwitchTab('settings')},
    ];
    items.push(...tabs);

    // Actions — all trigger toast confirmation
    items.push({
        type: 'action', name: appFmt('menu.scan_plugins'), icon: '&#8635;', action: () => {
            showToast(toastFmt('toast.scanning_plugins'));
            typeof scanPlugins === 'function' && scanPlugins();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.scan_samples'), icon: '&#8635;', action: () => {
            showToast(toastFmt('toast.scanning_samples'));
            typeof scanAudioSamples === 'function' && scanAudioSamples();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.scan_daw'), icon: '&#8635;', action: () => {
            showToast(toastFmt('toast.scanning_daw_projects'));
            typeof scanDawProjects === 'function' && scanDawProjects();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.scan_presets'), icon: '&#8635;', action: () => {
            showToast(toastFmt('toast.scanning_presets'));
            typeof scanPresets === 'function' && scanPresets();
        }
    });
    items.push({
        type: 'action', name: appFmt('ui.btn.scan_pdfs'), icon: '&#8635;', action: () => {
            showToast(toastFmt('toast.scanning_pdfs_progress'));
            typeof scanPdfs === 'function' && scanPdfs();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.stop_pdf_scan'), icon: '&#9632;', action: () => {
            if (typeof stopPdfScan === 'function') stopPdfScan();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.export_pdfs'), icon: '&#8615;', action: () => {
            if (typeof exportPdfs === 'function' && typeof runExport === 'function') runExport(exportPdfs); else if (typeof exportPdfs === 'function') exportPdfs();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.import_pdfs'), icon: '&#8613;', action: () => {
            if (typeof importPdfs === 'function') importPdfs();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.extract_pdf_page_counts'), icon: '&#128196;', action: () => {
            if (typeof buildPdfPagesCache === 'function') buildPdfPagesCache();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.build_fingerprint_cache'), icon: '&#127925;', action: () => {
            void (async () => {
                const paths = await fetchAudioLibraryPathsForFingerprint();
                if (paths.length === 0) {
                    showToast(toastFmt('toast.no_audio_samples_scan_first'), 4000, 'error');
                    return;
                }
                showToast(toastFmt('toast.fingerprint_building_n', {n: paths.length.toLocaleString()}), 4000);
                const vu = window.vstUpdater;
                if (!vu || typeof vu.buildFingerprintCache !== 'function') return;
                try {
                    const res = await vu.buildFingerprintCache(paths);
                    showToast(toastFmt('toast.fingerprint_build_complete_n', {n: res.built.toLocaleString()}));
                } catch (e) {
                    showToast(toastFmt('toast.fingerprint_build_failed', {err: e.message || e}), 4000, 'error');
                }
            })();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.check_updates'), icon: '&#9889;', action: () => {
            showToast(toastFmt('toast.checking_updates'));
            typeof checkUpdates === 'function' && checkUpdates();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.find_duplicates'), icon: '&#128270;', action: () => {
            showToast(toastFmt('toast.scanning_duplicates'));
            typeof showDuplicateReport === 'function' && showDuplicateReport();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.reset_all_scans'), icon: '&#128465;', action: () => {
            showToast(toastFmt('toast.resetting_scans'));
            typeof resetAllScans === 'function' && resetAllScans();
        }
    });
    if (typeof buildXrefIndex === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.build_plugin_index'), icon: '&#9889;', action: () => {
                showToast(toastFmt('toast.building_plugin_index'));
                buildXrefIndex();
            }
        });
    }
    if (typeof showDepGraph === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.dep_graph'), icon: '&#128200;', action: () => {
                showToast(toastFmt('toast.opening_dep_graph'));
                showDepGraph();
            }
        });
    }
    if (typeof showHeatmapDashboard === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.heatmap_dashboard'), icon: '&#128202;', action: () => {
                showToast(toastFmt('toast.opening_dashboard'));
                void showHeatmapDashboard();
            }
        });
    }
    if (typeof showSmartPlaylistEditor === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.new_smart_playlist'), icon: '&#127926;', action: () => {
                showToast(toastFmt('toast.creating_smart_playlist'));
                showSmartPlaylistEditor(null);
            }
        });
    }
    if (typeof exportSettingsPdf === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.export_settings_keybindings'), icon: '&#128196;', action: () => {
                showToast(toastFmt('toast.exporting_settings_pdf'));
                exportSettingsPdf();
            }
        });
    }
    if (typeof exportLogPdf === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.export_app_log'), icon: '&#128196;', action: () => {
                showToast(toastFmt('toast.exporting_log'));
                exportLogPdf();
            }
        });
    }
    if (typeof exportMidi === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.export_midi_files'), icon: '&#127929;', action: () => {
                showToast(toastFmt('toast.exporting_midi'));
                if (typeof runExport === 'function') runExport(exportMidi); else exportMidi();
            }
        });
    }
    if (typeof exportXref === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.export_plugin_xref'), icon: '&#9889;', action: () => {
                showToast(toastFmt('toast.exporting_xref'));
                exportXref();
            }
        });
    }
    if (typeof exportSmartPlaylists === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.export_smart_playlists'), icon: '&#127926;', action: () => {
                showToast(toastFmt('toast.exporting_playlists'));
                exportSmartPlaylists();
            }
        });
    }
    items.push({
        type: 'action', name: appFmt('menu.clear_all_caches'), icon: '&#128465;', action: () => {
            const vu = window.vstUpdater;
            if (!vu || typeof vu.dbClearCaches !== 'function') return;
            showToast(toastFmt('toast.clearing_caches'));
            vu.dbClearCaches().then(() => {
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
            }).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
        }
    });
    if (typeof settingToggleTheme === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_dark_light_theme'),
            icon: '&#127912;',
            action: () => settingToggleTheme()
        });
    }
    items.push({
        type: 'action', name: appFmt('menu.scan_all'), icon: '&#9889;', action: () => {
            showToast(toastFmt('toast.scanning_all'));
            typeof scanAll === 'function' && scanAll();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.stop_all_scans'), icon: '&#9632;', action: () => {
            showToast(toastFmt('toast.stopping_scans'));
            typeof stopAll === 'function' && stopAll();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.export_current_tab'), icon: '&#8615;', action: () => {
            typeof _exportCurrentTab === 'function' && _exportCurrentTab();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.import_to_current_tab'), icon: '&#8613;', action: () => {
            typeof _importCurrentTab === 'function' && _importCurrentTab();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.help_keyboard_shortcuts'), icon: '&#10068;', action: () => {
            typeof toggleHelpOverlay === 'function' && toggleHelpOverlay();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.open_log_file'), icon: '&#128196;', action: () => {
            const vu = window.vstUpdater;
            if (!vu || typeof vu.getPrefsPath !== 'function' || typeof vu.openWithApp !== 'function') return;
            showToast(toastFmt('toast.opening_log'));
            vu.getPrefsPath().then(p => {
                const lp = p.replace(/preferences\.toml$/, 'app.log');
                vu.openWithApp(lp, 'TextEdit').catch(e => {
                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                });
            });
        }
    });

    // Toggles
    if (typeof settingToggleCrt === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_crt'),
        icon: '&#128187;',
        action: () => settingToggleCrt()
    });
    if (typeof settingToggleNeonGlow === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_neon_glow'),
        icon: '&#10024;',
        action: () => settingToggleNeonGlow()
    });
    if (typeof toggleTagFilterBarVisibility === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_tag_filter_bar'),
        icon: '&#127991;',
        action: () => toggleTagFilterBarVisibility()
    });
    if (typeof settingToggleAutoAnalysis === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_auto_analyze_startup'),
        icon: '&#127925;',
        action: () => settingToggleAutoAnalysis()
    });
    if (typeof settingToggleAutoScan === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_auto_scan_launch'),
        icon: '&#8635;',
        action: () => settingToggleAutoScan()
    });
    if (typeof settingToggleAutoUpdate === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_auto_check_updates'),
        icon: '&#9889;',
        action: () => settingToggleAutoUpdate()
    });
    if (typeof settingToggleFolderWatch === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_folder_watch'),
        icon: '&#128065;',
        action: () => settingToggleFolderWatch()
    });
    if (typeof settingToggleSingleClickPlay === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_single_click_play'),
        icon: '&#9654;',
        action: () => settingToggleSingleClickPlay()
    });
    if (typeof settingToggleAutoPlaySampleOnSelect === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_play_sample_on_keyboard_select'),
        icon: '&#9835;',
        action: () => settingToggleAutoPlaySampleOnSelect()
    });
    if (typeof settingToggleAutoplayNext === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_autoplay_next'),
        icon: '&#9197;',
        action: () => settingToggleAutoplayNext()
    });
    if (typeof settingToggleShowPlayerOnStartup === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_show_player_startup'),
        icon: '&#9835;',
        action: () => settingToggleShowPlayerOnStartup()
    });
    if (typeof settingToggleExpandOnClick === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_expand_on_click'),
        icon: '&#8597;',
        action: () => settingToggleExpandOnClick()
    });
    if (typeof settingToggleIncludeBackups === 'function') items.push({
        type: 'action',
        name: appFmt('menu.toggle_include_ableton_backups'),
        icon: '&#128190;',
        action: () => settingToggleIncludeBackups()
    });

    // Resets & Clears
    if (typeof resetTabOrder === 'function') items.push({
        type: 'action',
        name: appFmt('menu.reset_tabs'),
        icon: '&#8634;',
        action: () => {
            resetTabOrder();
            showToast(toastFmt('toast.tab_order_reset'));
        }
    });
    if (typeof resetSettingsSectionOrder === 'function') items.push({
        type: 'action',
        name: appFmt('menu.reset_settings_layout'),
        icon: '&#8634;',
        action: () => {
            resetSettingsSectionOrder();
            showToast(toastFmt('toast.settings_layout_reset'));
        }
    });
    if (typeof resetFzfParams === 'function') items.push({
        type: 'action',
        name: appFmt('menu.reset_search_weights'),
        icon: '&#8634;',
        action: () => resetFzfParams()
    });
    if (typeof settingResetAllUI === 'function') items.push({
        type: 'action',
        name: appFmt('menu.reset_all_ui_layout'),
        icon: '&#9888;',
        action: () => {
            settingResetAllUI();
            showToast(toastFmt('toast.all_ui_layout_reset'));
        }
    });
    if (typeof settingResetColumns === 'function') items.push({
        type: 'action',
        name: appFmt('menu.reset_columns'),
        icon: '&#8634;',
        action: () => {
            settingResetColumns();
            showToast(toastFmt('toast.column_widths_reset'));
        }
    });
    if (typeof settingClearAllHistory === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_all_scan_history'),
        icon: '&#128465;',
        action: () => {
            settingClearAllHistory();
            showToast(toastFmt('toast.all_history_cleared'));
        }
    });
    if (typeof settingClearAllDatabases === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_all_databases'),
        icon: '&#128465;',
        action: () => settingClearAllDatabases()
    });
    if (typeof settingClearKvrCache === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_kvr'),
        icon: '&#128465;',
        action: () => {
            settingClearKvrCache();
            showToast(toastFmt('toast.kvr_cache_cleared_palette'));
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.clear_app_log'), icon: '&#128465;', action: () => {
            const vu = window.vstUpdater;
            if (!vu || typeof vu.clearLog !== 'function') return;
            vu.clearLog().then(() => showToast(toastFmt('toast.log_cleared'))).catch(() => showToast(toastFmt('toast.failed_clear_log'), 4000, 'error'));
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.preferences'), icon: '&#128196;', action: () => {
            showToast(toastFmt('toast.opening_preferences'));
            typeof openPrefsFile === 'function' && openPrefsFile();
        }
    });
    items.push({
        type: 'action', name: appFmt('menu.open_data_directory'), icon: '&#128193;', action: () => {
            const vu = window.vstUpdater;
            if (!vu || typeof vu.getPrefsPath !== 'function' || typeof vu.openPluginFolder !== 'function') return;
            showToast(toastFmt('toast.opening_data_dir'));
            vu.getPrefsPath().then(p => {
                const dir = p.replace(/[/\\][^/\\]+$/, '');
                vu.openPluginFolder(dir);
            });
        }
    });
    if (typeof clearRecentlyPlayed === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_play_history'),
        icon: '&#128465;',
        action: () => clearRecentlyPlayed()
    });
    if (typeof clearFavorites === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_favorites'),
        icon: '&#128465;',
        action: () => clearFavorites()
    });
    if (typeof clearAllNotes === 'function') items.push({
        type: 'action',
        name: appFmt('menu.clear_all_notes_tags'),
        icon: '&#128465;',
        action: () => clearAllNotes()
    });
    items.push({
        type: 'action', name: appFmt('menu.focus_search'), icon: '&#128269;', action: () => {
            const tab = document.querySelector('.tab-content.active');
            const input = tab?.querySelector('input[type="text"]');
            if (input) {
                input.focus();
                input.select();
            }
        }
    });

    // Player controls
    if (typeof toggleAudioPlayback === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.play_pause'),
            icon: '&#9654;',
            action: () => toggleAudioPlayback()
        });
    }
    if (typeof nextTrack === 'function') {
        items.push({type: 'action', name: appFmt('menu.next_track'), icon: '&#9193;', action: () => nextTrack()});
    }
    if (typeof prevTrack === 'function') {
        items.push({type: 'action', name: appFmt('menu.prev_track'), icon: '&#9194;', action: () => prevTrack()});
    }
    if (typeof toggleAudioLoop === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_loop'),
            icon: '&#128257;',
            action: () => toggleAudioLoop()
        });
    }
    if (typeof toggleShuffle === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_shuffle'),
            icon: '&#128256;',
            action: () => toggleShuffle()
        });
    }
    if (typeof toggleMute === 'function') {
        items.push({type: 'action', name: appFmt('menu.toggle_mute'), icon: '&#128263;', action: () => toggleMute()});
    }
    if (typeof toggleMono === 'function') {
        items.push({type: 'action', name: appFmt('menu.toggle_mono'), icon: '&#127897;', action: () => toggleMono()});
    }
    if (typeof toggleEqSection === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_eq_panel'),
            icon: '&#127900;',
            action: () => toggleEqSection()
        });
    }
    if (typeof togglePlayerExpanded === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.expand_player'),
            icon: '&#9744;',
            action: () => togglePlayerExpanded()
        });
    }
    if (typeof setAbLoopStart === 'function') {
        items.push({
            type: 'action', name: appFmt('menu.toggle_ab_loop'), icon: '&#128260;', action: () => {
                if (typeof _abLoop !== 'undefined' && _abLoop) {
                    if (typeof clearAbLoop === 'function') clearAbLoop();
                } else {
                    setAbLoopStart();
                }
            }
        });
    }

    // Selection
    if (typeof selectAllVisible === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_select_all_visible'),
            icon: '&#9745;',
            action: () => selectAllVisible()
        });
    }
    if (typeof deselectAll === 'function') {
        items.push({
            type: 'action',
            name: appFmt('menu.toggle_deselect_all'),
            icon: '&#9744;',
            action: () => deselectAll()
        });
    }

    // Data items (plugins, samples, DAW, presets) are searched lazily
    // in filterPaletteResults to avoid blocking UI on palette open.
    // Bookmarks/player/similar are appended in buildPaletteDynamicItems().

    return items;
}

/** Rows that depend on current track, player visibility, or live bookmark list. */
function buildPaletteDynamicItems() {
    const items = [];
    if (typeof findSimilarSamples === 'function' && typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
        items.push({
            type: 'action', name: appFmt('menu.find_similar_current'), icon: '&#128270;', action: () => {
                showToast(toastFmt('toast.finding_similar'));
                findSimilarSamples(audioPlayerPath);
            }
        });
    }
    if (typeof showPlayer === 'function') {
        const np = document.getElementById('audioNowPlaying');
        const visible = np && np.classList.contains('active');
        items.push({
            type: 'action',
            name: visible ? appFmt('menu.hide_audio_player') : appFmt('menu.show_audio_player'),
            icon: '&#9835;',
            action: () => {
                visible ? (typeof hidePlayer === 'function' && hidePlayer()) : (typeof showPlayer === 'function' && showPlayer());
                showToast(visible ? toastFmt('toast.player_hidden') : toastFmt('toast.player_shown'));
            }
        });
    }
    if (typeof getFavDirs === 'function') {
        for (const d of getFavDirs()) {
            items.push({
                type: 'bookmark', name: d.name, detail: d.path,
                icon: '&#128278;', fields: [d.name, d.path],
                action: () => {
                    _paletteSwitchTab('files');
                    typeof loadDirectory === 'function' && loadDirectory(d.path);
                }
            });
        }
    }
    return items;
}

function getPaletteStaticItems() {
    if (_paletteStaticItemsCache === null) {
        _paletteStaticItemsCache = buildPaletteStaticItems();
    }
    return _paletteStaticItemsCache;
}

function collectPaletteItems() {
    return getPaletteStaticItems().concat(buildPaletteDynamicItems());
}

/** Call after reloadAppStrings so menu labels match the active locale. */
function invalidatePaletteStaticCache() {
    _paletteStaticItemsCache = null;
    _paletteSessionItems = null;
    _paletteOpenTypeLabels = null;
}
window.invalidatePaletteStaticCache = invalidatePaletteStaticCache;

function ensurePaletteTypeLabels() {
    if (!_paletteOpenTypeLabels) {
        _paletteOpenTypeLabels = {
            tab: catalogFmt('ui.palette.type_tab'),
            action: catalogFmt('ui.palette.type_action'),
            plugin: catalogFmt('ui.palette.type_plugin'),
            sample: catalogFmt('ui.palette.type_sample'),
            daw: catalogFmt('ui.palette.type_daw'),
            pdf: catalogFmt('ui.palette.type_pdf'),
            preset: catalogFmt('ui.palette.type_preset'),
            midi: catalogFmt('ui.palette.type_midi'),
            bookmark: catalogFmt('ui.palette.type_bookmark'),
        };
    }
    return _paletteOpenTypeLabels;
}

/**
 * @returns {{item: object, score: number}[]}
 */
function filterPaletteItemsScored(query, items) {
    if (!query) {
        return items.filter(i => i.type === 'tab' || i.type === 'action').map(item => ({item, score: 1}));
    }
    const scored = [];
    for (const item of items) {
        const fields = item.fields || [item.name];
        const score = searchScore(query, fields, 'fuzzy');
        if (score > 0) scored.push({item, score});
    }
    // Inventory hits (2+ chars) come from `fetchPaletteDatabaseItems` in `renderPaletteResults`
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, PALETTE_MAX);
}

function filterPaletteItems(query, items) {
    return filterPaletteItemsScored(query, items).map(s => s.item);
}

/** Session list: built when the palette opens; cleared on close or locale/static invalidation. */
function getPaletteSearchItems() {
    if (_paletteSessionItems === null && _paletteOpen) {
        _paletteSessionItems = collectPaletteItems();
    }
    return _paletteSessionItems;
}

function openPalette() {
    if (_paletteOpen) return;
    _paletteOpen = true;
    _paletteQuery = '';
    _paletteSelected = 0;

    const ph = catalogFmt('ui.palette.placeholder');
    const html = `<div class="palette-overlay" id="paletteOverlay">
    <div class="palette-box">
      <input type="text" class="palette-input" id="paletteInput" placeholder="" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">
      <div class="palette-results" id="paletteResults"></div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);

    const input = document.getElementById('paletteInput');
    if (input) input.placeholder = ph;
    if (input) input.focus();
    _paletteSessionItems = collectPaletteItems();
    void renderPaletteResults();

    let _palTimer;
    if (input) {
        input.addEventListener('input', () => {
            _paletteQuery = input.value;
            _paletteSelected = 0;
            clearTimeout(_palTimer);
            _palTimer = setTimeout(() => {
                void renderPaletteResults();
            }, 150);
        });
    }
}

function closePalette() {
    if (!_paletteOpen) return;
    _paletteOpen = false;
    _paletteSessionItems = null;
    _paletteOpenTypeLabels = null;
    const overlay = document.getElementById('paletteOverlay');
    if (overlay) overlay.remove();
}

function paintPaletteRows(container) {
    if (_paletteResults.length === 0) {
        const empty = catalogFmt('ui.palette.empty');
        container.innerHTML = '<div class="palette-empty">' + escapeHtml(empty) + '</div>';
        return;
    }

    const typeLabels = ensurePaletteTypeLabels();
    container.innerHTML = _paletteResults.map((item, i) => {
        const typeCls = 'palette-type-' + item.type;
        const sel = i === _paletteSelected ? ' palette-selected' : '';
        const typeLabel = typeLabels[item.type] || item.type;
        const detail = item.detail ? `<span class="palette-detail">${escapeHtml(item.detail)}</span>` : '';
        return `<div class="palette-row${sel}" data-palette-idx="${i}">
      <span class="palette-icon">${item.icon}</span>
      <span class="palette-name">${_paletteQuery ? highlightMatch(item.name, _paletteQuery, 'fuzzy') : escapeHtml(item.name)}</span>
      ${detail}
      <span class="palette-badge ${typeCls}">${typeLabel}</span>
    </div>`;
    }).join('');
}

/** Arrow keys: toggle selection class only (no full innerHTML + highlight rebuild). */
function updatePaletteRowSelection(container) {
    if (_paletteResults.length === 0) return;
    const rows = container.querySelectorAll('.palette-row[data-palette-idx]');
    rows.forEach((row) => {
        const i = parseInt(row.dataset.paletteIdx, 10);
        row.classList.toggle('palette-selected', i === _paletteSelected);
    });
    scrollPaletteSelection();
}

async function renderPaletteResults() {
    const container = document.getElementById('paletteResults');
    if (!container) return;

    const allItems = getPaletteSearchItems();
    if (!allItems) return;

    const q = _paletteQuery;
    const qTrim = q.trim();

    let scoredLocal = filterPaletteItemsScored(qTrim, allItems);
    let merged = scoredLocal.map(s => s.item);

    // Paint in-memory fuzzy matches immediately. The 2+ char branch used to await SQLite
    // before any paint, so typed queries looked ignored until IPC returned.
    _paletteResults = merged;
    paintPaletteRows(container);

    if (qTrim.length >= 2 && typeof window.vstUpdater?.dbQueryPalettePreview === 'function') {
        const seq = ++_paletteDbSeq;
        const dbItems = await fetchPaletteDatabaseItems(qTrim);
        if (seq !== _paletteDbSeq) return;

        const scored = scoredLocal.slice();
        for (const item of dbItems) {
            const fields = item.fields || [item.name];
            scored.push({item, score: searchScore(qTrim, fields, 'fuzzy')});
        }
        scored.sort((a, b) => b.score - a.score);
        merged = scored.slice(0, PALETTE_MAX).map((s) => s.item);
        _paletteResults = merged;
        paintPaletteRows(container);
    }
}

function executePaletteItem(idx) {
    const item = _paletteResults[idx];
    if (!item) return;
    closePalette();
    item.action();
}

// Keyboard navigation
document.addEventListener('keydown', (e) => {
    // Open palette: Cmd+K or Ctrl+K
    const isMac = navigator.platform.includes('Mac');
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (mod && e.key === 'k') {
        e.preventDefault();
        e.stopPropagation();
        if (_paletteOpen) closePalette();
        else openPalette();
        return;
    }

    if (!_paletteOpen) return;

    if (e.key === 'Escape') {
        e.preventDefault();
        closePalette();
        return;
    }

    if (e.key === 'ArrowDown') {
        e.preventDefault();
        _paletteSelected = Math.min(_paletteSelected + 1, Math.max(0, _paletteResults.length - 1));
        const container = document.getElementById('paletteResults');
        if (container) updatePaletteRowSelection(container);
        return;
    }

    if (e.key === 'ArrowUp') {
        e.preventDefault();
        _paletteSelected = Math.max(_paletteSelected - 1, 0);
        const container = document.getElementById('paletteResults');
        if (container) updatePaletteRowSelection(container);
        return;
    }

    if (e.key === 'Enter') {
        e.preventDefault();
        executePaletteItem(_paletteSelected);
        return;
    }
}, true);

function scrollPaletteSelection() {
    const sel = document.querySelector('.palette-selected');
    if (sel) sel.scrollIntoView({block: 'nearest'});
}

// Click handling
document.addEventListener('click', (e) => {
    if (!_paletteOpen) return;

    const row = e.target.closest('[data-palette-idx]');
    if (row) {
        executePaletteItem(parseInt(row.dataset.paletteIdx, 10));
        return;
    }

    // Click outside the palette box closes it
    if (e.target.id === 'paletteOverlay') {
        closePalette();
    }
});

// Hover to highlight — swap class on old/new row only (no querySelectorAll)
document.addEventListener('mousemove', (e) => {
    if (!_paletteOpen) return;
    const row = e.target.closest('[data-palette-idx]');
    if (row) {
        const idx = parseInt(row.dataset.paletteIdx, 10);
        if (idx !== _paletteSelected) {
            const prev = document.querySelector('.palette-row.palette-selected');
            if (prev) prev.classList.remove('palette-selected');
            row.classList.add('palette-selected');
            _paletteSelected = idx;
        }
    }
});
