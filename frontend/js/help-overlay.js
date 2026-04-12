// ── Help Overlay ──
// Press ? to show keyboard shortcuts reference

function _helpEsc(s) {
    return typeof escapeHtml === 'function' ? escapeHtml(s) : String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;');
}

function _helpModKbdDigit(d) {
    const isMac = navigator.platform.includes('Mac');
    if (isMac) return `<kbd>\u2318${d}</kbd>`;
    return `<kbd>Ctrl+${d}</kbd>`;
}

function _helpModKbdComma() {
    const isMac = navigator.platform.includes('Mac');
    if (isMac) return `<kbd>\u2318,</kbd>`;
    return `<kbd>Ctrl+,</kbd>`;
}

function _helpModArrow(dir) {
    const isMac = navigator.platform.includes('Mac');
    const sym = {Right: '\u2192', Left: '\u2190', Up: '\u2191', Down: '\u2193'}[dir] || '';
    if (isMac) return `<kbd>\u2318${sym}</kbd>`;
    return `<kbd>Ctrl+${sym}</kbd>`;
}

function _helpModKbdLetter(ch) {
    const isMac = navigator.platform.includes('Mac');
    const u = ch.toUpperCase();
    if (isMac) return `<kbd>\u2318${u}</kbd>`;
    return `<kbd>Ctrl+${u}</kbd>`;
}

function _helpModKbdKey(key) {
    const isMac = navigator.platform.includes('Mac');
    if (isMac) return `<kbd>\u2318${key}</kbd>`;
    return `<kbd>Ctrl+${key}</kbd>`;
}

/** Keys listed in Settings → Keyboard Shortcuts that are not covered above */
const HELP_MORE_SHORTCUT_IDS = [
    'tab11', 'tab12', 'tab13', 'tab14',
    'newSmartPlaylist', 'togglePlayerExpand', 'toggleEq', 'toggleMono',
    'toggleABLoop', 'heatmapDash', 'togglePlayer', 'toggleCrt', 'toggleNeonGlow',
    'clearPlayHistory', 'deselectAll', 'openPrefs',
    'scanPluginsOnly', 'scanSamplesOnly', 'scanDawOnly', 'scanPresetsOnly', 'scanPdfsOnly',
    'stopPdfScan', 'extractPdfMetadata', 'stopPdfMetadata', 'togglePdfMetadataAutoExtract',
    'buildFingerprintCache', 'checkUpdates', 'buildPluginIndex', 'bpmKeyLufsStart', 'bpmKeyLufsStop',
    'clearAllCaches', 'exportSettingsPdf', 'exportLogPdf', 'exportMidiPalette', 'exportPluginXref', 'exportSmartPlaylists',
    'openLogFile', 'openPrefsFile', 'openDataDirectory', 'toggleTagFilterBar', 'toggleAutoplayNext',
    'autoplaySourcePlayer', 'autoplaySourceSamples', 'videoAudioRouteEngine', 'videoAudioRouteHtml5',
    'clearAnalysisCache', 'cycleLogVerbosity',
    'increasePruneKeep', 'decreasePruneKeep', 'increaseTablePageSize', 'decreaseTablePageSize',
    'toggleAutoAnalysis', 'toggleAutoPlaySampleOnSelect', 'toggleAutoScan',
    'toggleAutoUpdate', 'toggleExpandOnClick', 'toggleFolderWatch', 'toggleIncludeBackups',
    'toggleIncrementalDirectoryScan', 'togglePruneOldScans', 'toggleSingleClickPlay',
];

function _helpMoreRows() {
    if (typeof getShortcuts !== 'function' || typeof formatKey !== 'function') return '';
    const sc = getShortcuts();
    const rows = [];
    for (const id of HELP_MORE_SHORTCUT_IDS) {
        const ent = sc[id];
        if (!ent) continue;
        rows.push(
            `<div class="help-row"><kbd>${_helpEsc(formatKey(ent))}</kbd> <span>${_helpEsc(ent.label)}</span></div>`
        );
    }
    return rows.join('');
}

function _helpVideoFullscreenRow(h) {
    if (typeof getShortcuts !== 'function' || typeof formatKey !== 'function') return '';
    const ent = getShortcuts().videoToggleFullscreen;
    if (!ent) return '';
    return `<div class="help-row"><kbd>${_helpEsc(formatKey(ent))}</kbd> <span>${h('help.play.video_maximize')}</span></div>`;
}

function toggleHelpOverlay() {
    let existing = document.getElementById('helpOverlay');
    if (existing) {
        existing.remove();
        return;
    }

    const h = (key) => catalogFmt(key);

    const k19 = `${_helpModKbdDigit('1')}<span style="color:var(--text-dim);">\u2013</span>${_helpModKbdDigit('9')}`;
    const k0 = _helpModKbdDigit('0');
    const moreRows = _helpMoreRows();
    const videoFsRow = _helpVideoFullscreenRow(h);

    const html = `<div class="modal-overlay" id="helpOverlay" data-action-modal="closeHelp">
    <div class="modal-content">
      <div class="modal-header">
        <h2>${h('help.title')}</h2>
        <button class="modal-close" data-action-modal="closeHelp" title="${h('help.close')}">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="help-grid">
          <div class="help-section">
            <h3>${h('help.section.navigation')}</h3>
            <div class="help-row">${k19} <span>${h('help.nav.switch_tabs_1_9_desc')}</span></div>
            <div class="help-row">${k0} <span>${h('help.nav.switch_tabs_10_desc')}</span></div>
            <div class="help-row"><kbd>F3</kbd> <kbd>F4</kbd> <kbd>F5</kbd> <kbd>F6</kbd> <span>${h('help.nav.switch_tabs_f_keys_desc')}</span></div>
            <div class="help-row">${_helpModKbdComma()} <span>${h('help.nav.switch_tabs_settings_desc')}</span></div>
            <div class="help-row">${_helpModKbdLetter('k')} <span>${h('help.nav.cmd_palette')}</span></div>
            <div class="help-row">${_helpModKbdLetter('f')} <span>${h('help.nav.focus_search')}</span></div>
            <div class="help-row"><kbd>j</kbd> / <kbd>&#8595;</kbd> <span>${h('help.nav.next_item')}</span></div>
            <div class="help-row"><kbd>k</kbd> / <kbd>&#8593;</kbd> <span>${h('help.nav.prev_item')}</span></div>
            <div class="help-row"><kbd>gg</kbd> <span>${h('help.nav.first_item')}</span></div>
            <div class="help-row"><kbd>G</kbd> <span>${h('help.nav.last_item')}</span></div>
            <div class="help-row"><kbd>Ctrl+D</kbd> <span>${h('help.nav.half_down')}</span></div>
            <div class="help-row"><kbd>Ctrl+U</kbd> <span>${h('help.nav.half_up')}</span></div>
            <div class="help-row"><kbd>/</kbd> <span>${h('help.nav.focus_search_slash')}</span></div>
            <div class="help-row"><kbd>Enter</kbd> <span>${h('help.nav.open_activate')}</span></div>
            <div class="help-row"><kbd>o</kbd> <span>${h('help.nav.reveal_finder')}</span></div>
            <div class="help-row"><kbd>y</kbd> <span>${h('help.nav.yank')}</span></div>
            <div class="help-row"><kbd>p</kbd> <span>${h('help.nav.play_preview')}</span></div>
            <div class="help-row"><kbd>x</kbd> <span>${h('help.act.delete')}</span></div>
            <div class="help-row"><kbd>v</kbd> <span>${h('help.nav.toggle_select')}</span></div>
            <div class="help-row"><kbd>V</kbd> <span>${h('help.nav.select_all')}</span></div>
          </div>
          <div class="help-section">
            <h3>${h('help.section.playback')}</h3>
            <div class="help-row"><kbd>Space</kbd> <span>${h('help.play.pause')}</span></div>
            <div class="help-row">${_helpModArrow('Right')} <span>${h('help.play.next')}</span></div>
            <div class="help-row">${_helpModArrow('Left')} <span>${h('help.play.prev')}</span></div>
            <div class="help-row"><kbd>L</kbd> <span>${h('help.play.loop')}</span></div>
            <div class="help-row"><kbd>M</kbd> <span>${h('help.play.mute')}</span></div>
            <div class="help-row">${_helpModArrow('Up')} <span>${h('help.play.vol_up')}</span></div>
            <div class="help-row">${_helpModArrow('Down')} <span>${h('help.play.vol_down')}</span></div>
            ${videoFsRow}
          </div>
          <div class="help-section">
            <h3>${h('help.section.actions')}</h3>
            <div class="help-row">${_helpModKbdLetter('s')} <span>${h('help.act.scan_all')}</span></div>
            <div class="help-row">${_helpModKbdKey('.')} <span>${h('help.act.stop_scans')}</span></div>
            <div class="help-row">${_helpModKbdLetter('a')} <span>${h('help.act.select_visible')}</span></div>
            <div class="help-row">${_helpModKbdLetter('e')} <span>${h('help.act.export_tab')}</span></div>
            <div class="help-row">${_helpModKbdLetter('i')} <span>${h('help.act.import_tab')}</span></div>
            <div class="help-row">${_helpModKbdLetter('d')} <span>${h('help.act.dupes')}</span></div>
            <div class="help-row">${_helpModKbdLetter('g')} <span>${h('help.act.deps')}</span></div>
            <div class="help-row">${_helpModKbdLetter('t')} <span>${h('help.act.theme')}</span></div>
            <div class="help-row">${_helpModKbdComma()} <span>${h('help.act.prefs_file')}</span></div>
            <div class="help-row">${_helpModKbdKey(']')} <span>${h('help.act.next_tab')}</span></div>
            <div class="help-row">${_helpModKbdKey('[')} <span>${h('help.act.prev_tab')}</span></div>
            <div class="help-row"><kbd>R</kbd> <span>${h('help.act.reveal')}</span></div>
            <div class="help-row"><kbd>C</kbd> <span>${h('help.act.copy_path')}</span></div>
            <div class="help-row"><kbd>F</kbd> <span>${h('help.act.toggle_fav')}</span></div>
            <div class="help-row"><kbd>N</kbd> <span>${h('help.act.add_note')}</span></div>
            <div class="help-row"><kbd>S</kbd> <span>${h('help.act.shuffle')}</span></div>
            <div class="help-row"><kbd>W</kbd> <span>${h('help.act.similar')}</span></div>
            <div class="help-row"><kbd>Del</kbd> <span>${h('help.act.delete')}</span></div>
            <div class="help-row"><kbd>Esc</kbd> <span>${h('help.act.esc')}</span></div>
            <div class="help-row"><kbd>?</kbd> <span>${h('help.act.toggle_help')}</span></div>
          </div>
          <div class="help-section">
            <h3>${h('help.section.fzf')}</h3>
            <div class="help-row"><code>term</code> <span>${h('help.fzf.fuzzy')}</span></div>
            <div class="help-row"><code>'exact</code> <span>${h('help.fzf.exact')}</span></div>
            <div class="help-row"><code>^prefix</code> <span>${h('help.fzf.prefix')}</span></div>
            <div class="help-row"><code>suffix$</code> <span>${h('help.fzf.suffix')}</span></div>
            <div class="help-row"><code>!term</code> <span>${h('help.fzf.exclude')}</span></div>
            <div class="help-row"><code>a | b</code> <span>${h('help.fzf.or')}</span></div>
            <div class="help-row"><code>.*</code> <span>${h('help.fzf.regex')}</span></div>
          </div>
          <div class="help-section">
            <h3>${h('help.section.mouse')}</h3>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.click')}</span> <span>${h('help.mouse.click_desc')}</span></div>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.drag_samples')}</span> <span>${h('help.mouse.drag_samples_desc')}</span></div>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.dblclick')}</span> <span>${h('help.mouse.dblclick_desc')}</span></div>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.right')}</span> <span>${h('help.mouse.right_desc')}</span></div>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.drag_tabs')}</span> <span>${h('help.mouse.drag_tabs_desc')}</span></div>
            <div class="help-row"><span style="color:var(--cyan);">${h('help.mouse.drag_player')}</span> <span>${h('help.mouse.drag_player_desc')}</span></div>
          </div>
          <div class="help-section">
            <h3>${h('help.section.more')}</h3>
            ${moreRows}
          </div>
        </div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
}

// Event delegation for help modal
document.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action-modal="closeHelp"]');
    if (action) {
        if (e.target === action || action.classList.contains('modal-close')) {
            const overlay = document.getElementById('helpOverlay');
            if (overlay) overlay.remove();
        }
    }
});
