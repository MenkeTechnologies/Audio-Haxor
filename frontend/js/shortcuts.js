// ── Keyboard Shortcut Customization ──

const SHORTCUT_LABEL_KEYS = {
    tab1: 'ui.shortcut.plugins_tab',
    tab2: 'ui.shortcut.samples_tab',
    tab3: 'ui.shortcut.daw_projects_tab',
    tab4: 'ui.shortcut.presets_tab',
    tab5: 'ui.shortcut.midi_tab',
    tab6: 'ui.shortcut.pdf_tab',
    tab7: 'ui.shortcut.favorites_tab',
    tab8: 'ui.shortcut.notes_tab',
    tab9: 'ui.shortcut.tags_tab',
    tab10: 'ui.shortcut.files_tab',
    tab11: 'ui.shortcut.history_tab',
    tab12: 'ui.shortcut.visualizer_tab',
    tab13: 'ui.shortcut.walkers_tab',
    tab14: 'ui.shortcut.audio_engine_tab',
    search: 'ui.shortcut.focus_search',
    help: 'ui.shortcut.help_overlay',
    playPause: 'ui.shortcut.play_pause',
    nextTrack: 'ui.shortcut.next_track',
    prevTrack: 'ui.shortcut.prev_track',
    scanAll: 'ui.shortcut.scan_all',
    stopAll: 'ui.shortcut.stop_all_scans',
    commandPalette: 'ui.shortcut.command_palette',
    toggleLoop: 'ui.shortcut.toggle_loop',
    toggleMute: 'ui.shortcut.toggle_mute',
    volumeUp: 'ui.shortcut.volume_up',
    volumeDown: 'ui.shortcut.volume_down',
    revealFile: 'ui.shortcut.reveal_in_finder',
    copyPath: 'ui.shortcut.copy_path',
    toggleFavorite: 'ui.shortcut.toggle_favorite',
    addNote: 'ui.shortcut.add_note',
    deleteItem: 'ui.shortcut.delete_selected',
    selectAll: 'ui.shortcut.select_all_visible',
    escape: 'ui.shortcut.close_clear_stop',
    exportTab: 'ui.shortcut.export_current_tab',
    importTab: 'ui.shortcut.import_current_tab',
    toggleShuffle: 'ui.shortcut.toggle_shuffle',
    findDuplicates: 'ui.shortcut.find_duplicates',
    depGraph: 'ui.shortcut.dependency_graph',
    resetAllScans: 'ui.shortcut.reset_all_scans',
    toggleTheme: 'ui.shortcut.toggle_theme',
    openPrefs: 'ui.shortcut.settings',
    nextTab: 'ui.shortcut.next_tab',
    prevTab: 'ui.shortcut.previous_tab',
    findSimilar: 'ui.shortcut.find_similar_samples',
    togglePlayerExpand: 'ui.shortcut.expand_collapse_player',
    toggleEq: 'ui.shortcut.toggle_eq',
    toggleMono: 'ui.shortcut.toggle_mono',
    newSmartPlaylist: 'ui.shortcut.new_smart_playlist',
    deselectAll: 'ui.shortcut.deselect_all',
    toggleABLoop: 'ui.shortcut.ab_loop',
    heatmapDash: 'ui.shortcut.heatmap_dashboard',
    togglePlayer: 'ui.shortcut.show_hide_player',
    toggleCrt: 'ui.shortcut.toggle_crt',
    toggleNeonGlow: 'ui.shortcut.toggle_neon_glow',
    clearPlayHistory: 'ui.shortcut.clear_play_history',
    scanPluginsOnly: 'ui.shortcut.scan_plugins_only',
    scanSamplesOnly: 'ui.shortcut.scan_samples_only',
    scanDawOnly: 'ui.shortcut.scan_daw_only',
    scanPresetsOnly: 'ui.shortcut.scan_presets_only',
    scanPdfsOnly: 'ui.shortcut.scan_pdfs_only',
    stopPdfScan: 'ui.shortcut.stop_pdf_scan',
    extractPdfMetadata: 'ui.shortcut.extract_pdf_metadata',
    stopPdfMetadata: 'ui.shortcut.stop_pdf_metadata',
    buildFingerprintCache: 'ui.shortcut.build_fingerprint_cache',
    checkUpdates: 'ui.shortcut.check_updates',
    buildPluginIndex: 'ui.shortcut.build_plugin_index',
    bpmKeyLufsStart: 'ui.shortcut.bpm_key_lufs_start',
    bpmKeyLufsStop: 'ui.shortcut.bpm_key_lufs_stop',
    clearAllCaches: 'ui.shortcut.clear_all_caches',
    exportSettingsPdf: 'ui.shortcut.export_settings_pdf',
    exportLogPdf: 'ui.shortcut.export_log_pdf',
    exportMidiPalette: 'ui.shortcut.export_midi_palette',
    exportPluginXref: 'ui.shortcut.export_plugin_xref',
    exportSmartPlaylists: 'ui.shortcut.export_smart_playlists',
    openLogFile: 'ui.shortcut.open_log_file',
    openPrefsFile: 'ui.shortcut.open_prefs_file',
    openDataDirectory: 'ui.shortcut.open_data_directory',
    toggleTagFilterBar: 'ui.shortcut.toggle_tag_filter_bar',
    toggleAutoplayNext: 'ui.shortcut.toggle_autoplay_next',
    autoplaySourcePlayer: 'ui.shortcut.autoplay_source_player',
    autoplaySourceSamples: 'ui.shortcut.autoplay_source_samples',
    clearAnalysisCache: 'ui.shortcut.clear_analysis_cache',
    cycleLogVerbosity: 'ui.shortcut.cycle_log_verbosity',
    increasePruneKeep: 'ui.shortcut.increase_prune_keep',
    decreasePruneKeep: 'ui.shortcut.decrease_prune_keep',
    increaseTablePageSize: 'ui.shortcut.increase_table_page_size',
    decreaseTablePageSize: 'ui.shortcut.decrease_table_page_size',
    toggleAutoAnalysis: 'ui.shortcut.toggle_auto_analysis',
    toggleAutoPlaySampleOnSelect: 'ui.shortcut.toggle_auto_play_sample_on_select',
    toggleAutoScan: 'ui.shortcut.toggle_auto_scan',
    toggleShowPlayerStartup: 'ui.shortcut.toggle_show_player_startup',
    toggleAutoUpdate: 'ui.shortcut.toggle_auto_update',
    toggleExpandOnClick: 'ui.shortcut.toggle_expand_on_click',
    toggleFolderWatch: 'ui.shortcut.toggle_folder_watch',
    toggleIncludeBackups: 'ui.shortcut.toggle_include_backups',
    toggleIncrementalDirectoryScan: 'ui.shortcut.toggle_incremental_directory_scan',
    togglePruneOldScans: 'ui.shortcut.toggle_prune_old_scans',
    toggleSingleClickPlay: 'ui.shortcut.toggle_single_click_play',
    togglePdfMetadataAutoExtract: 'ui.shortcut.toggle_pdf_metadata_auto_extract',
};

const DEFAULT_SHORTCUT_DEFS = {
    tab1: {key: '1', mod: true},
    tab2: {key: '2', mod: true},
    tab3: {key: '3', mod: true},
    tab4: {key: '4', mod: true},
    tab5: {key: '5', mod: true},
    tab6: {key: '6', mod: true},
    tab7: {key: '7', mod: true},
    tab8: {key: '8', mod: true},
    tab9: {key: '9', mod: true},
    tab10: {key: '0', mod: true},
    tab11: {key: 'F3', mod: false},
    tab12: {key: 'F4', mod: false},
    tab13: {key: 'F5', mod: false},
    tab14: {key: 'F6', mod: false},
    search: {key: 'f', mod: true, shift: false},
    help: {key: '?', mod: false},
    playPause: {key: ' ', mod: false},
    nextTrack: {key: 'ArrowRight', mod: true, shift: false},
    prevTrack: {key: 'ArrowLeft', mod: true, shift: false},
    scanAll: {key: 's', mod: true, shift: false},
    stopAll: {key: '.', mod: true, shift: false},
    commandPalette: {key: 'k', mod: true, shift: false},
    toggleLoop: {key: 'l', mod: false},
    toggleMute: {key: 'm', mod: false},
    volumeUp: {key: 'ArrowUp', mod: true, shift: false},
    volumeDown: {key: 'ArrowDown', mod: true, shift: false},
    revealFile: {key: 'r', mod: false},
    copyPath: {key: 'c', mod: false},
    toggleFavorite: {key: 'f', mod: false},
    addNote: {key: 'n', mod: false},
    deleteItem: {key: 'Backspace', mod: false},
    selectAll: {key: 'a', mod: true, shift: false},
    escape: {key: 'Escape', mod: false},
    exportTab: {key: 'e', mod: true, shift: false},
    importTab: {key: 'i', mod: true, shift: false},
    toggleShuffle: {key: 's', mod: false},
    findDuplicates: {key: 'd', mod: true, shift: false},
    depGraph: {key: 'g', mod: true, shift: false},
    resetAllScans: {key: 'Backspace', mod: true, shift: false},
    toggleTheme: {key: 't', mod: true, shift: false},
    openPrefs: {key: ',', mod: true, shift: false},
    nextTab: {key: ']', mod: true, shift: false},
    prevTab: {key: '[', mod: true, shift: false},
    findSimilar: {key: 'w', mod: false},
    togglePlayerExpand: {key: 'e', mod: false},
    toggleEq: {key: 'q', mod: false},
    toggleMono: {key: 'u', mod: false},
    newSmartPlaylist: {key: 'p', mod: true, shift: false},
    deselectAll: {key: 'Escape', mod: true, shift: false},
    toggleABLoop: {key: 'b', mod: false},
    heatmapDash: {key: 'd', mod: false},
    togglePlayer: {key: 'p', mod: false},
    toggleCrt: {key: 'F1', mod: false},
    toggleNeonGlow: {key: 'F2', mod: false},
    clearPlayHistory: {key: 'h', mod: true, shift: false},
    scanPluginsOnly: {key: 'p', mod: true, shift: true},
    scanSamplesOnly: {key: 's', mod: true, shift: true},
    scanDawOnly: {key: '`', mod: true, shift: true},
    scanPresetsOnly: {key: 'r', mod: true, shift: true},
    scanPdfsOnly: {key: 'f', mod: true, shift: true},
    stopPdfScan: {key: 'y', mod: true, shift: true},
    extractPdfMetadata: {key: 'm', mod: true, shift: true},
    stopPdfMetadata: {key: 'z', mod: true, shift: true},
    buildFingerprintCache: {key: 'b', mod: true, shift: true},
    checkUpdates: {key: 'u', mod: true, shift: true},
    buildPluginIndex: {key: 'x', mod: true, shift: true},
    bpmKeyLufsStart: {key: 'v', mod: true, shift: true},
    bpmKeyLufsStop: {key: 'c', mod: true, shift: true},
    clearAllCaches: {key: 'Backspace', mod: true, shift: true},
    exportSettingsPdf: {key: 'w', mod: true, shift: true},
    exportLogPdf: {key: 'l', mod: true, shift: true},
    exportMidiPalette: {key: 'i', mod: true, shift: true},
    exportPluginXref: {key: 'j', mod: true, shift: true},
    exportSmartPlaylists: {key: '\\', mod: true, shift: true},
    openLogFile: {key: 'o', mod: true, shift: true},
    openPrefsFile: {key: 'g', mod: true, shift: true},
    openDataDirectory: {key: 'd', mod: true, shift: true},
    toggleTagFilterBar: {key: 't', mod: true, shift: true},
    toggleAutoplayNext: {key: 'n', mod: true, shift: true},
    autoplaySourcePlayer: {key: '[', mod: true, shift: true},
    autoplaySourceSamples: {key: ']', mod: true, shift: true},
    clearAnalysisCache: {key: '9', mod: true, shift: true},
    cycleLogVerbosity: {key: '?', mod: true, shift: true},
    increasePruneKeep: {key: '+', mod: true, shift: true},
    decreasePruneKeep: {key: '-', mod: true, shift: true},
    increaseTablePageSize: {key: 'ArrowUp', mod: true, shift: true},
    decreaseTablePageSize: {key: 'ArrowDown', mod: true, shift: true},
    toggleAutoAnalysis: {key: 'F7', mod: true, shift: false},
    toggleAutoPlaySampleOnSelect: {key: 'F7', mod: true, shift: true},
    toggleAutoScan: {key: 'F8', mod: true, shift: false},
    toggleShowPlayerStartup: {key: 'F8', mod: true, shift: true},
    toggleAutoUpdate: {key: 'F9', mod: true, shift: false},
    toggleExpandOnClick: {key: 'F9', mod: true, shift: true},
    toggleFolderWatch: {key: 'F10', mod: true, shift: false},
    toggleIncludeBackups: {key: 'F10', mod: true, shift: true},
    toggleIncrementalDirectoryScan: {key: 'F11', mod: true, shift: false},
    togglePruneOldScans: {key: 'F11', mod: true, shift: true},
    toggleSingleClickPlay: {key: 'F12', mod: true, shift: false},
    togglePdfMetadataAutoExtract: {key: 'F12', mod: true, shift: true},
};

const TAB_MAP = ['plugins', 'samples', 'daw', 'presets', 'midi', 'pdf', 'favorites', 'notes', 'tags', 'files', 'history', 'visualizer', 'walkers', 'audioEngine', 'settings'];

function getShortcuts() {
    const saved = prefs.getObject('customShortcuts', null);
    const fmt = catalogFmt;
    const merged = {};
    for (const [id, def] of Object.entries(DEFAULT_SHORTCUT_DEFS)) {
        const lk = SHORTCUT_LABEL_KEYS[id];
        merged[id] = {
            key: def.key,
            mod: def.mod,
            label: lk ? fmt(lk) : id,
        };
        if (def.shift !== undefined) merged[id].shift = def.shift;
        if (saved && saved[id]) {
            merged[id].key = saved[id].key;
            merged[id].mod = saved[id].mod;
            if (saved[id].shift !== undefined) merged[id].shift = saved[id].shift;
        }
    }
    return merged;
}

function saveShortcuts(shortcuts) {
    const slim = {};
    for (const [id, sc] of Object.entries(shortcuts)) {
        slim[id] = {key: sc.key, mod: sc.mod};
        if (sc.shift !== undefined) slim[id].shift = sc.shift;
    }
    prefs.setItem('customShortcuts', slim);
}

function resetShortcuts() {
    prefs.removeItem('customShortcuts');
    renderShortcutSettings();
    showToast(toastFmt('toast.shortcuts_reset'));
}

/** Space bar: match on e.code (reliable) and normalize stored 'Space' vs ' '. */
function normalizeStoredShortcutKey(k) {
    if (k === 'Space' || k === ' ') return ' ';
    return k;
}

function eventKeyForShortcutMatch(e) {
    if (e.code === 'Space' || e.key === ' ' || e.key === 'Space') return ' ';
    return e.key;
}

/** Unify letter case and '=' vs '+' for US keyboards (Cmd+Shift+= often reports '+'). */
function normalizeKeyForMatch(k) {
    const n = normalizeStoredShortcutKey(k);
    if (n === '+' || n === '=') return '+';
    if (n.length === 1 && /[A-Za-z]/.test(n)) return n.toLowerCase();
    return n;
}

function shortcutShiftMatches(sc, e) {
    if (sc.shift === true) return e.shiftKey === true;
    if (sc.shift === false) return e.shiftKey === false;
    return true;
}

function formatKey(shortcut) {
    const isMac = navigator.platform.includes('Mac');
    let parts = [];
    if (shortcut.mod) parts.push(isMac ? '\u2318' : 'Ctrl');
    if (shortcut.shift === true) parts.push(isMac ? '\u21E7' : 'Shift');
    let k = shortcut.key;
    if (k === ' ') k = 'Space';
    else if (k === 'ArrowLeft') k = '\u2190';
    else if (k === 'ArrowRight') k = '\u2192';
    else if (k === 'ArrowUp') k = '\u2191';
    else if (k === 'ArrowDown') k = '\u2193';
    else if (k === 'Escape') k = 'Esc';
    else if (k === '\\') k = '\\';
    else if (k === '`') k = '`';
    else k = k.toUpperCase();
    parts.push(k);
    return parts.join('+');
}

function renderShortcutSettings(filter) {
    const list = document.getElementById('shortcutsList');
    if (!list) return;
    const shortcuts = getShortcuts();
    const q = (filter || '').trim();
    let entries;
    if (!q) {
        entries = Object.entries(shortcuts).map(([id, sc]) => [id, sc, 0]);
    } else {
        entries = [];
        for (const [id, sc] of Object.entries(shortcuts)) {
            const score = searchScore(q, [sc.label, formatKey(sc)], 'fuzzy');
            if (score > 0) entries.push([id, sc, score]);
        }
        entries.sort((a, b) => b[2] - a[2]);
    }
    const hl = typeof highlightMatch === 'function' && q
        ? (text) => highlightMatch(text, q, 'fuzzy')
        : (text) => (typeof escapeHtml === 'function' ? escapeHtml(text) : text);
    list.innerHTML = entries.map(([id, sc]) =>
        `<div class="shortcut-row" data-sc-id="${id}">
      <span class="shortcut-name">${hl(sc.label)}</span>
      <span class="shortcut-key" data-shortcut-id="${id}" title="${escapeHtml(catalogFmt('menu.rebind_shortcut'))}">${q ? hl(formatKey(sc)) : formatKey(sc)}</span>
    </div>`
    ).join('');
    if (!q && typeof initDragReorder === 'function') {
        initDragReorder(list, '.shortcut-row', 'shortcutOrder', {
            getKey: (el) => el.dataset.scId || '',
            // Drag from anywhere on the row (skip list handles buttons)
        });
    }
}

// Filter input — uses unified filter system
registerFilter('filterShortcuts', {
    inputId: 'shortcutsFilter',
    fetchFn() {
        renderShortcutSettings(this.lastSearch || '');
    },
});

// Recording state
let _recordingId = null;

document.addEventListener('click', (e) => {
    const keyEl = e.target.closest('.shortcut-key');
    if (keyEl && keyEl.dataset.shortcutId) {
        // Start recording
        if (_recordingId) {
            // Cancel previous
            document.querySelectorAll('.shortcut-key.recording').forEach(el => el.classList.remove('recording'));
        }
        _recordingId = keyEl.dataset.shortcutId;
        keyEl.classList.add('recording');
        keyEl.textContent = catalogFmt('ui.shortcut.press_key');
        e.stopPropagation();
        return;
    }
    const resetBtn = e.target.closest('[data-action="resetShortcuts"]');
    if (resetBtn) {
        resetShortcuts();
    }
});

function _recordShortcutKey(e) {
    let k = eventKeyForShortcutMatch(e);
    if (k.length === 1 && /[a-zA-Z]/.test(k)) k = k.toLowerCase();
    if (k === '=' || k === '+') k = '+';
    return k;
}

document.addEventListener('keydown', (e) => {
    if (_recordingId) {
        e.preventDefault();
        e.stopPropagation();
        const isMac = navigator.platform.includes('Mac');
        const mod = isMac ? e.metaKey : e.ctrlKey;
        if (e.key === 'Escape') {
            // Cancel recording
            _recordingId = null;
            renderShortcutSettings();
            return;
        }
        // Don't record bare modifier keys
        if (['Meta', 'Control', 'Shift', 'Alt'].includes(e.key)) return;

        const shortcuts = getShortcuts();
        const k = _recordShortcutKey(e);
        const shift = e.shiftKey;
        shortcuts[_recordingId] = {...shortcuts[_recordingId], key: k, mod, shift};
        saveShortcuts(shortcuts);
        _recordingId = null;
        renderShortcutSettings();
        showToast(toastFmt('toast.shortcut_updated'));
        return;
    }

    const isMac = navigator.platform.includes('Mac');
    const mod = isMac ? e.metaKey : e.ctrlKey;
    // Command palette: must work while the palette search input is focused
    if (mod && e.key === 'k') {
        e.preventDefault();
        e.stopPropagation();
        if (typeof toggleCommandPalette === 'function') toggleCommandPalette();
        return;
    }

    // Don't handle shortcuts when typing in inputs
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;
    if (e.target.isContentEditable || e.target.closest('[contenteditable]')) return;
    if (e.target.closest('.ctx-menu')) return;

    const shortcuts = getShortcuts();
    const eventKey = normalizeKeyForMatch(eventKeyForShortcutMatch(e));

    for (const [id, sc] of Object.entries(shortcuts)) {
        if (normalizeKeyForMatch(sc.key) !== eventKey) continue;
        if (sc.mod !== mod) continue;
        if (!shortcutShiftMatches(sc, e)) continue;
        e.preventDefault();
        executeShortcut(id);
        return;
    }
}, true); // capture phase to override other handlers

function executeShortcut(id) {
    if (id.startsWith('tab') && id.length >= 4 && id.length <= 5) {
        const num = parseInt(id.slice(3));
        const idx = num - 1;
        if (idx >= 0 && idx < TAB_MAP.length) switchTab(TAB_MAP[idx]);
    } else if (id === 'search') {
        const activeTab = document.querySelector('.tab-content.active');
        const input = activeTab?.querySelector('input[type="text"]');
        if (input) {
            input.focus();
            input.select();
        }
    } else if (id === 'help') {
        toggleHelpOverlay();
    } else if (id === 'playPause') {
        toggleAudioPlayback();
    } else if (id === 'nextTrack') {
        nextTrack();
    } else if (id === 'prevTrack') {
        prevTrack();
    } else if (id === 'scanAll') {
        if (typeof scanAll === 'function') scanAll();
    } else if (id === 'stopAll') {
        if (typeof stopAll === 'function') stopAll();
    } else if (id === 'commandPalette') {
        if (typeof toggleCommandPalette === 'function') toggleCommandPalette();
    } else if (id === 'toggleLoop') {
        if (typeof toggleAudioLoop === 'function') toggleAudioLoop();
    } else if (id === 'toggleMute') {
        if (typeof toggleMute === 'function') toggleMute();
    } else if (id === 'volumeUp') {
        _adjustVolume(5);
    } else if (id === 'volumeDown') {
        _adjustVolume(-5);
    } else if (id === 'revealFile') {
        _actionOnSelected('reveal');
    } else if (id === 'copyPath') {
        _actionOnSelected('copy');
    } else if (id === 'toggleFavorite') {
        _actionOnSelected('favorite');
    } else if (id === 'addNote') {
        _actionOnSelected('note');
    } else if (id === 'deleteItem') {
        _actionOnSelected('delete');
    } else if (id === 'selectAll') {
        if (typeof selectAllVisible === 'function') selectAllVisible();
    } else if (id === 'escape') {
        _handleEscape();
    } else if (id === 'exportTab') {
        _exportCurrentTab();
    } else if (id === 'importTab') {
        _importCurrentTab();
    } else if (id === 'toggleShuffle') {
        if (typeof toggleShuffle === 'function') toggleShuffle();
    } else if (id === 'findDuplicates') {
        if (typeof showDuplicateReport === 'function') showDuplicateReport();
    } else if (id === 'depGraph') {
        if (typeof showDepGraph === 'function') showDepGraph();
    } else if (id === 'resetAllScans') {
        if (typeof resetAllScans === 'function') resetAllScans();
    } else if (id === 'toggleTheme') {
        if (typeof settingToggleTheme === 'function') settingToggleTheme();
    } else if (id === 'openPrefs') {
        switchTab('settings');
    } else if (id === 'nextTab') {
        _cycleTab(1);
    } else if (id === 'prevTab') {
        _cycleTab(-1);
    } else if (id === 'findSimilar') {
        const path = _getSelectedPath();
        if (path && typeof findSimilarSamples === 'function') findSimilarSamples(path);
    } else if (id === 'togglePlayerExpand') {
        if (typeof togglePlayerExpanded === 'function') togglePlayerExpanded();
    } else if (id === 'toggleEq') {
        if (typeof toggleEqSection === 'function') toggleEqSection();
    } else if (id === 'toggleMono') {
        if (typeof toggleMono === 'function') toggleMono();
    } else if (id === 'newSmartPlaylist') {
        if (typeof showSmartPlaylistEditor === 'function') showSmartPlaylistEditor(null);
    } else if (id === 'deselectAll') {
        if (typeof deselectAll === 'function') deselectAll();
    } else if (id === 'toggleABLoop') {
        if (typeof _abLoop !== 'undefined' && _abLoop) {
            if (typeof clearAbLoop === 'function') clearAbLoop();
        } else {
            if (typeof setAbLoopStart === 'function') setAbLoopStart();
        }
    } else if (id === 'heatmapDash') {
        if (typeof showHeatmapDashboard === 'function') void showHeatmapDashboard();
    } else if (id === 'togglePlayer') {
        const np = document.getElementById('audioNowPlaying');
        if (np && np.classList.contains('active')) {
            if (typeof hidePlayer === 'function') hidePlayer();
        } else {
            if (typeof showPlayer === 'function') showPlayer();
        }
    } else if (id === 'toggleCrt') {
        if (typeof settingToggleCrt === 'function') settingToggleCrt();
    } else if (id === 'toggleNeonGlow') {
        if (typeof settingToggleNeonGlow === 'function') settingToggleNeonGlow();
    } else if (id === 'clearPlayHistory') {
        if (typeof clearRecentlyPlayed === 'function') clearRecentlyPlayed();
    } else if (id === 'scanPluginsOnly') {
        if (typeof scanPlugins === 'function') scanPlugins();
    } else if (id === 'scanSamplesOnly') {
        if (typeof scanAudioSamples === 'function') scanAudioSamples();
    } else if (id === 'scanDawOnly') {
        if (typeof scanDawProjects === 'function') scanDawProjects();
    } else if (id === 'scanPresetsOnly') {
        if (typeof scanPresets === 'function') scanPresets();
    } else if (id === 'scanPdfsOnly') {
        if (typeof scanPdfs === 'function') scanPdfs();
    } else if (id === 'stopPdfScan') {
        if (typeof stopPdfScan === 'function') stopPdfScan();
    } else if (id === 'extractPdfMetadata') {
        if (typeof buildPdfPagesCache === 'function') buildPdfPagesCache();
    } else if (id === 'stopPdfMetadata') {
        if (typeof stopPdfMetadataExtractionUser === 'function') void stopPdfMetadataExtractionUser();
    } else if (id === 'buildFingerprintCache') {
        void (async () => {
            const paths = typeof fetchAudioLibraryPathsForFingerprint === 'function'
                ? await fetchAudioLibraryPathsForFingerprint()
                : [];
            if (paths.length === 0) {
                if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                    showToast(toastFmt('toast.no_audio_samples_scan_first'), 4000, 'error');
                }
                return;
            }
            if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                showToast(toastFmt('toast.fingerprint_building_n', {n: paths.length.toLocaleString()}), 4000);
            }
            const vu = window.vstUpdater;
            if (!vu || typeof vu.buildFingerprintCache !== 'function') return;
            try {
                const res = await vu.buildFingerprintCache(paths);
                if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                    showToast(toastFmt('toast.fingerprint_build_complete_n', {n: res.built.toLocaleString()}));
                }
            } catch (err) {
                if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                    showToast(toastFmt('toast.fingerprint_build_failed', {err: err.message || err}), 4000, 'error');
                }
            }
        })();
    } else if (id === 'checkUpdates') {
        if (typeof checkUpdates === 'function') checkUpdates();
    } else if (id === 'buildPluginIndex') {
        if (typeof buildXrefIndex === 'function') buildXrefIndex();
    } else if (id === 'bpmKeyLufsStart') {
        if (typeof triggerBackgroundBpmKeyLufsAnalysis === 'function') triggerBackgroundBpmKeyLufsAnalysis();
    } else if (id === 'bpmKeyLufsStop') {
        if (typeof triggerStopBackgroundBpmKeyLufsAnalysis === 'function') triggerStopBackgroundBpmKeyLufsAnalysis();
    } else if (id === 'clearAllCaches') {
        const vu = window.vstUpdater;
        if (!vu || typeof vu.dbClearCaches !== 'function') return;
        if (typeof showToast === 'function' && typeof toastFmt === 'function') showToast(toastFmt('toast.clearing_caches'));
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
            if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                showToast(toastFmt('toast.all_caches_cleared'));
            }
        }).catch((err) => {
            if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                showToast(toastFmt('toast.failed', {err}), 4000, 'error');
            }
        });
    } else if (id === 'exportSettingsPdf') {
        if (typeof exportSettingsPdf === 'function') exportSettingsPdf();
    } else if (id === 'exportLogPdf') {
        if (typeof exportLogPdf === 'function') exportLogPdf();
    } else if (id === 'exportMidiPalette') {
        const run = typeof runExport === 'function' ? runExport : (fn) => {
            if (typeof fn === 'function') fn();
        };
        if (typeof exportMidi === 'function') run(exportMidi);
    } else if (id === 'exportPluginXref') {
        if (typeof exportXref === 'function') exportXref();
    } else if (id === 'exportSmartPlaylists') {
        if (typeof exportSmartPlaylists === 'function') exportSmartPlaylists();
    } else if (id === 'openLogFile') {
        const vu = window.vstUpdater;
        if (!vu || typeof vu.getPrefsPath !== 'function' || typeof vu.openWithApp !== 'function') return;
        if (typeof showToast === 'function' && typeof toastFmt === 'function') showToast(toastFmt('toast.opening_log'));
        vu.getPrefsPath().then((p) => {
            const lp = p.replace(/preferences\.toml$/, 'app.log');
            vu.openWithApp(lp, 'TextEdit').catch((err) => {
                if (typeof showToast === 'function') showToast(String(err), 4000, 'error');
            });
        });
    } else if (id === 'openPrefsFile') {
        if (typeof openPrefsFile === 'function') openPrefsFile();
    } else if (id === 'openDataDirectory') {
        const vu = window.vstUpdater;
        if (!vu || typeof vu.getPrefsPath !== 'function' || typeof vu.openPluginFolder !== 'function') return;
        if (typeof showToast === 'function' && typeof toastFmt === 'function') showToast(toastFmt('toast.opening_data_dir'));
        vu.getPrefsPath().then((p) => {
            const dir = p.replace(/[/\\][^/\\]+$/, '');
            vu.openPluginFolder(dir);
        });
    } else if (id === 'toggleTagFilterBar') {
        if (typeof toggleTagFilterBarVisibility === 'function') toggleTagFilterBarVisibility();
    } else if (id === 'toggleAutoplayNext') {
        if (typeof settingToggleAutoplayNext === 'function') settingToggleAutoplayNext();
    } else if (id === 'autoplaySourcePlayer') {
        if (typeof settingSetAutoplayNextSource === 'function') settingSetAutoplayNextSource('player');
    } else if (id === 'autoplaySourceSamples') {
        if (typeof settingSetAutoplayNextSource === 'function') settingSetAutoplayNextSource('samples');
    } else if (id === 'clearAnalysisCache') {
        if (typeof settingClearAnalysisCache === 'function') void settingClearAnalysisCache();
    } else if (id === 'cycleLogVerbosity') {
        if (typeof paletteCycleLogVerbosity === 'function') paletteCycleLogVerbosity();
    } else if (id === 'increasePruneKeep') {
        if (typeof paletteNudgePruneKeep === 'function') paletteNudgePruneKeep(1);
    } else if (id === 'decreasePruneKeep') {
        if (typeof paletteNudgePruneKeep === 'function') paletteNudgePruneKeep(-1);
    } else if (id === 'increaseTablePageSize') {
        if (typeof paletteNudgeTablePageSize === 'function') paletteNudgeTablePageSize(100);
    } else if (id === 'decreaseTablePageSize') {
        if (typeof paletteNudgeTablePageSize === 'function') paletteNudgeTablePageSize(-100);
    } else if (id === 'toggleAutoAnalysis') {
        if (typeof settingToggleAutoAnalysis === 'function') settingToggleAutoAnalysis();
    } else if (id === 'toggleAutoPlaySampleOnSelect') {
        if (typeof settingToggleAutoPlaySampleOnSelect === 'function') settingToggleAutoPlaySampleOnSelect();
    } else if (id === 'toggleAutoScan') {
        if (typeof settingToggleAutoScan === 'function') settingToggleAutoScan();
    } else if (id === 'toggleShowPlayerStartup') {
        if (typeof settingToggleShowPlayerOnStartup === 'function') settingToggleShowPlayerOnStartup();
    } else if (id === 'toggleAutoUpdate') {
        if (typeof settingToggleAutoUpdate === 'function') settingToggleAutoUpdate();
    } else if (id === 'toggleExpandOnClick') {
        if (typeof settingToggleExpandOnClick === 'function') settingToggleExpandOnClick();
    } else if (id === 'toggleFolderWatch') {
        if (typeof settingToggleFolderWatch === 'function') settingToggleFolderWatch();
    } else if (id === 'toggleIncludeBackups') {
        if (typeof settingToggleIncludeBackups === 'function') settingToggleIncludeBackups();
    } else if (id === 'toggleIncrementalDirectoryScan') {
        if (typeof settingToggleIncrementalDirectoryScan === 'function') settingToggleIncrementalDirectoryScan();
    } else if (id === 'togglePruneOldScans') {
        if (typeof settingTogglePruneOldScans === 'function') settingTogglePruneOldScans();
    } else if (id === 'toggleSingleClickPlay') {
        if (typeof settingToggleSingleClickPlay === 'function') settingToggleSingleClickPlay();
    } else if (id === 'togglePdfMetadataAutoExtract') {
        if (typeof settingTogglePdfMetadataAutoExtract === 'function') settingTogglePdfMetadataAutoExtract();
    }
}

function _exportCurrentTab() {
    const active = document.querySelector('.tab-content.active')?.id;
    const run = typeof runExport === 'function' ? runExport : (fn) => {
        if (typeof fn === 'function') fn();
    };
    if (active === 'tabPlugins' && typeof exportPlugins === 'function') run(exportPlugins);
    else if (active === 'tabSamples' && typeof exportAudio === 'function') run(exportAudio);
    else if (active === 'tabDaw' && typeof exportDaw === 'function') run(exportDaw);
    else if (active === 'tabPresets' && typeof exportPresets === 'function') run(exportPresets);
    else if (active === 'tabFavorites' && typeof exportFavorites === 'function') exportFavorites();
    else if (active === 'tabNotes' && typeof exportNotes === 'function') exportNotes();
    else if (active === 'tabMidi' && typeof exportMidi === 'function') run(exportMidi);
    else if (active === 'tabPdf' && typeof exportPdfs === 'function') run(exportPdfs);
}

function _importCurrentTab() {
    const active = document.querySelector('.tab-content.active')?.id;
    if (active === 'tabPlugins' && typeof importPlugins === 'function') importPlugins();
    else if (active === 'tabSamples' && typeof importAudio === 'function') importAudio();
    else if (active === 'tabDaw' && typeof importDaw === 'function') importDaw();
    else if (active === 'tabPresets' && typeof importPresets === 'function') importPresets();
    else if (active === 'tabFavorites' && typeof importFavorites === 'function') importFavorites();
    else if (active === 'tabNotes' && typeof importNotes === 'function') importNotes();
}

function _cycleTab(dir) {
    const tabs = [...document.querySelectorAll('.tab-btn')];
    const activeIdx = tabs.findIndex(t => t.classList.contains('active'));
    const next = (activeIdx + dir + tabs.length) % tabs.length;
    const tab = tabs[next]?.dataset?.tab;
    if (tab) switchTab(tab);
}

function _adjustVolume(delta) {
    const slider = document.getElementById('npVolume');
    if (!slider) return;
    const val = Math.max(0, Math.min(100, parseInt(slider.value) + delta));
    slider.value = val;
    if (typeof setAudioVolume === 'function') setAudioVolume(val);
}

function _actionOnSelected(action) {
    const items = getNavigableItems();
    if (_navIndex < 0 || _navIndex >= items.length) return;
    const item = items[_navIndex];
    const path = item.getAttribute('data-audio-path') || item.dataset.dawPath || item.dataset.presetPath || item.dataset.path || '';
    const name = item.querySelector('.col-name,.plugin-name')?.textContent?.trim() || '';
    if (!path) return;

    if (action === 'reveal') {
        if (typeof openFolder === 'function') openFolder(path);
        else if (typeof openAudioFolder === 'function') openAudioFolder(path);
    } else if (action === 'copy') {
        if (typeof copyToClipboard === 'function') copyToClipboard(path);
    } else if (action === 'favorite') {
        if (typeof isFavorite === 'function' && typeof addFavorite === 'function' && typeof removeFavorite === 'function') {
            isFavorite(path) ? removeFavorite(path) : addFavorite('item', path, name);
        }
    } else if (action === 'note') {
        if (typeof showNoteEditor === 'function') showNoteEditor(path, name);
    } else if (action === 'delete') {
        if (typeof deleteFile === 'function') {
            if (confirm(appFmt('confirm.delete_shortcuts', {name: name || path}))) deleteFile(path);
        }
    }
}

function _handleEscape() {
    // Close modals first
    const modal = document.querySelector('.modal-overlay');
    if (modal) {
        modal.remove();
        return;
    }
    // Close context menu
    const ctx = document.querySelector('.ctx-menu.visible');
    if (ctx) {
        ctx.classList.remove('visible');
        return;
    }
    // Close command palette
    const palette = document.getElementById('paletteOverlay');
    if (palette) {
        palette.remove();
        return;
    }
    // Clear search in active tab
    const activeTab = document.querySelector('.tab-content.active');
    const input = activeTab?.querySelector('input[type="text"]');
    if (input && input.value) {
        input.value = '';
        input.dispatchEvent(new Event('input'));
        return;
    }
    // Stop current operation
    if (typeof stopAll === 'function') stopAll();
}
