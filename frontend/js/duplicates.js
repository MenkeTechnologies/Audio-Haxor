// ── Duplicate Detection ──
// By name: heuristic groups (plugins: same name; samples/DAW/presets: name+format).
// By content: SHA-256 of file bytes after grouping by stored size (library scope from SQLite).

function findDuplicates(items, keyFn) {
    const groups = {};
    for (const item of items) {
        const key = keyFn(item);
        if (!groups[key]) groups[key] = [];
        groups[key].push(item);
    }
    return Object.values(groups).filter(g => g.length > 1);
}

/** Map backend `kind` to main-tab i18n label. */
function dupKindLabel(kind) {
    const map = {
        plugins: 'menu.tab_plugins',
        audio: 'menu.tab_samples',
        daw: 'menu.tab_daw',
        presets: 'menu.tab_presets',
        pdf: 'menu.tab_pdf',
        midi: 'menu.tab_midi'
    };
    const k = map[kind];
    if (k) return catalogFmt(k);
    return kind || '';
}

function formatDupBytes(n) {
    if (typeof formatAudioSize === 'function') return formatAudioSize(n);
    if (!n) return '0 B';
    const u = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), u.length - 1);
    return `${(n / 1024 ** i).toFixed(1)} ${u[i]}`;
}

// ── Background byte-duplicate (SHA-256) scan: same stop-between-chunks idea as BPM batches ──
let _contentDupRunning = false;
let _contentDupUnlisten = null;
let _contentDupLastProgress = null;

function _shouldUpdateContentDupBadgeUi() {
    return !(typeof isUiIdleHeavyCpu === 'function' && isUiIdleHeavyCpu());
}

function setContentDupBadgeWorking() {
    const badge = document.getElementById('bgContentDupBadge');
    if (!badge || !_shouldUpdateContentDupBadgeUi()) return;
    badge.textContent = formatBgJobBadgeLine('contentDup', 'ui.stats.content_dup_bg_working');
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function applyContentDupProgressPayload(pl) {
    _contentDupLastProgress = pl;
    const status = document.getElementById('dupContentStatus');
    if (
        status &&
        pl &&
        pl.done != null &&
        pl.total != null &&
        Number.isFinite(pl.total)
    ) {
        status.textContent = catalogFmt('ui.dup.content_progress', {
            done: pl.done,
            total: pl.total
        });
    }
    const badge = document.getElementById('bgContentDupBadge');
    if (
        badge &&
        pl &&
        pl.done != null &&
        pl.total != null &&
        Number.isFinite(pl.total)
    ) {
        if (!_shouldUpdateContentDupBadgeUi()) {
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            return;
        }
        badge.textContent = formatBgJobBadgeLine('contentDup', 'ui.stats.content_dup_bg_progress', {
            done: pl.done,
            total: pl.total
        });
    }
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function clearContentDupBadge() {
    _contentDupLastProgress = null;
    const badge = document.getElementById('bgContentDupBadge');
    if (!badge) return;
    badge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

async function ensureContentDupProgressListener() {
    if (_contentDupUnlisten || !window.__TAURI__?.event?.listen) return;
    _contentDupUnlisten = await window.__TAURI__.event.listen('content-dup-progress', (e) => {
        applyContentDupProgressPayload(e.payload);
    });
}

function teardownContentDupProgressListener() {
    if (typeof _contentDupUnlisten === 'function') {
        try {
            _contentDupUnlisten();
        } catch {
            /* ignore */
        }
    }
    _contentDupUnlisten = null;
}

function setContentDupButtonsRunning(running) {
    const scan = document.getElementById('dupContentScanBtn');
    const stop = document.getElementById('dupContentStopBtn');
    if (scan) scan.disabled = running;
    if (stop) stop.disabled = !running;
}

document.addEventListener('ui-idle-heavy-cpu', (ev) => {
    try {
        if (!ev.detail || ev.detail.idle !== false) return;
        if (!_contentDupRunning) return;
        const badge = document.getElementById('bgContentDupBadge');
        if (!badge) return;
        const pl = _contentDupLastProgress;
        if (pl && pl.done != null && pl.total != null && Number.isFinite(pl.total)) {
            if (!_shouldUpdateContentDupBadgeUi()) return;
            badge.textContent = formatBgJobBadgeLine('contentDup', 'ui.stats.content_dup_bg_progress', {
                done: pl.done,
                total: pl.total
            });
        } else {
            setContentDupBadgeWorking();
        }
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    } catch {
        /* ignore */
    }
});

function buildNameDuplicateSections() {
    const results = [];

    const pluginDups = findDuplicates(allPlugins, p => p.name.toLowerCase());
    if (pluginDups.length > 0) {
        results.push({
            type: 'Plugins',
            icon: '&#9889;',
            groups: pluginDups.map(g => ({
                key: g[0].name,
                items: g.map(p => ({
                    name: p.name,
                    detail: `${p.type} | ${p.version} | ${p.size}`,
                    path: p.path
                }))
            }))
        });
    }

    const sampleDups = findDuplicates(
        allAudioSamples,
        s => `${s.name.toLowerCase()}.${s.format.toLowerCase()}`
    );
    if (sampleDups.length > 0) {
        results.push({
            type: 'Samples',
            icon: '&#127925;',
            groups: sampleDups.map(g => ({
                key: `${g[0].name}.${g[0].format}`,
                items: g.map(s => ({
                    name: s.name,
                    detail: `${s.format} | ${s.sizeFormatted}`,
                    path: s.path
                }))
            }))
        });
    }

    const dawDups = findDuplicates(
        allDawProjects,
        p => `${p.name.toLowerCase()}.${p.format.toLowerCase()}`
    );
    if (dawDups.length > 0) {
        results.push({
            type: 'DAW Projects',
            icon: '&#127911;',
            groups: dawDups.map(g => ({
                key: `${g[0].name}.${g[0].format}`,
                items: g.map(p => ({
                    name: p.name,
                    detail: `${p.daw} | ${p.sizeFormatted}`,
                    path: p.path
                }))
            }))
        });
    }

    const presetDups = findDuplicates(
        allPresets,
        p => `${p.name.toLowerCase()}.${p.format.toLowerCase()}`
    );
    if (presetDups.length > 0) {
        results.push({
            type: 'Presets',
            icon: '&#127924;',
            groups: presetDups.map(g => ({
                key: `${g[0].name}.${g[0].format}`,
                items: g.map(p => ({
                    name: p.name,
                    detail: `${p.format} | ${p.sizeFormatted || ''}`,
                    path: p.path
                }))
            }))
        });
    }

    return results;
}

function renderNameDupBody(results) {
    const totalGroups = results.reduce((sum, r) => sum + r.groups.length, 0);
    const totalItems = results.reduce(
        (sum, r) => sum + r.groups.reduce((s, g) => s + g.items.length, 0),
        0
    );

    let html = '';
    if (totalGroups === 0) {
        html += `<div class="state-message"><div class="state-icon">&#10003;</div><h2>${escapeHtml(
            catalogFmt('ui.dup.name_empty')
        )}</h2></div>`;
    } else {
        html += `<p class="dup-summary">${totalGroups} groups with ${totalItems} total duplicates</p>`;
        for (const section of results) {
            html += `<div class="dup-section">
        <h3>${section.icon} ${section.type} (${section.groups.length} groups)</h3>`;
            for (const group of section.groups.slice(0, 50)) {
                html += `<div class="dup-group">
          <div class="dup-group-key">${escapeHtml(group.key)} <span class="dup-count">${group.items.length} copies</span></div>`;
                for (const item of group.items) {
                    html += `<div class="dup-item">
            <span class="dup-item-detail">${escapeHtml(item.detail)}</span>
            <span class="dup-item-path" title="${escapeHtml(item.path)}">${escapeHtml(item.path)}</span>
          </div>`;
                }
                html += '</div>';
            }
            if (section.groups.length > 50) {
                html += `<p style="color: var(--text-muted); padding: 8px;">...and ${section.groups.length - 50} more groups</p>`;
            }
            html += '</div>';
        }
    }
    return html;
}

function renderContentDupPlaceholder() {
    const hint = catalogFmt('ui.dup.content_hint');
    const btn = catalogFmt('ui.dup.content_scan_btn');
    const stopLabel = catalogFmt('ui.dup.content_stop_btn');
    const stopTt = escapeHtml(catalogFmt('ui.tt.content_dup_stop'));
    return `<p class="dup-content-hint">${escapeHtml(hint)}</p>
    <div class="dup-content-actions">
    <button type="button" class="btn btn-secondary" data-action="dupContentScan" id="dupContentScanBtn">${escapeHtml(
        btn
    )}</button>
    <button type="button" class="btn btn-stop" data-action="dupContentStop" id="dupContentStopBtn" disabled title="${stopTt}">${escapeHtml(
        stopLabel
    )}</button>
    </div>
    <p id="dupContentStatus" style="margin-top:12px;font-size:12px;color:var(--text-muted);"></p>
    <div id="dupContentResults"></div>`;
}

function renderContentDupResults(payload) {
    if (!payload) {
        return `<p class="state-message" style="padding:12px 0;">${escapeHtml(catalogFmt('ui.dup.content_err'))}</p>`;
    }
    const cancelledNote =
        payload.cancelled && payload.groups && payload.groups.length > 0
            ? `<p class="dup-summary" style="margin-bottom:8px;color:var(--text-muted);">${escapeHtml(
                  catalogFmt('ui.dup.content_cancelled_note', {
                      done: payload.files_hashed ?? 0,
                      total: payload.candidates_total ?? 0
                  })
              )}</p>`
            : '';
    if (!payload.groups || payload.groups.length === 0) {
        const emptyKey = payload.cancelled ? 'ui.dup.content_empty_cancelled' : 'ui.dup.content_empty';
        let extra = '';
        if (payload.cancelled && (payload.candidates_total > 0 || (payload.files_hashed ?? 0) > 0)) {
            extra = `<p style="color:var(--text-muted);font-size:12px;margin-top:8px;">${escapeHtml(
                catalogFmt('ui.dup.content_cancelled_progress', {
                    done: payload.files_hashed ?? 0,
                    total: payload.candidates_total ?? 0
                })
            )}</p>`;
        }
        return `<p class="state-message" style="padding:12px 0;">${escapeHtml(catalogFmt(emptyKey))}</p>${extra}`;
    }
    let html = cancelledNote;
    const skippedPart =
        payload.skipped > 0 ? catalogFmt('ui.dup.skipped_suffix', {n: payload.skipped}) : '';
    const zeroPart =
        payload.skipped_zero_stored_size > 0
            ? catalogFmt('ui.dup.skipped_zero_suffix', {n: payload.skipped_zero_stored_size})
            : '';
    const sum = catalogFmt('ui.dup.content_summary', {
        groups: payload.groups.length,
        files: payload.files_hashed || 0,
        skipped: skippedPart + zeroPart
    });
    html += `<p class="dup-summary" style="margin-bottom:12px;">${escapeHtml(sum)}</p>`;
    for (const g of payload.groups.slice(0, 100)) {
        const sz = formatDupBytes(g.size_bytes);
        html += `<div class="dup-group">
      <div class="dup-group-key">${escapeHtml(g.hash_hex.slice(0, 16))}… <span class="dup-count">${escapeHtml(sz)}</span></div>`;
        for (const p of g.paths) {
            const kind = dupKindLabel(p.kind);
            html += `<div class="dup-item">
            <span class="dup-item-detail">${escapeHtml(kind)}</span>
            <span class="dup-item-path" title="${escapeHtml(p.path)}">${escapeHtml(p.path)}</span>
          </div>`;
        }
        html += '</div>';
    }
    if (payload.groups.length > 100) {
        html += `<p style="color: var(--text-muted); padding: 8px;">…and ${payload.groups.length - 100} more groups</p>`;
    }
    return html;
}

function switchDupTab(which) {
    const modal = document.getElementById('dupModal');
    if (!modal) return;
    const namePanel = modal.querySelector('#dupPanelName');
    const contentPanel = modal.querySelector('#dupPanelContent');
    const tabs = modal.querySelectorAll('[data-dup-tab]');
    tabs.forEach(t => {
        const on = t.dataset.dupTab === which;
        t.classList.toggle('dup-tab-active', on);
        t.setAttribute('aria-selected', on ? 'true' : 'false');
    });
    if (namePanel) namePanel.style.display = which === 'name' ? '' : 'none';
    if (contentPanel) contentPanel.style.display = which === 'content' ? '' : 'none';
}

function showDuplicateReport() {
    const nameResults = buildNameDuplicateSections();
    let existing = document.getElementById('dupModal');
    if (existing) existing.remove();

    const tabName = catalogFmt('ui.dup.tab_name');
    const tabContent = catalogFmt('ui.dup.tab_content');
    const title = catalogFmt('menu.find_duplicates');

    const html = `<div class="modal-overlay modal-visible" id="dupModal" data-action-modal="closeDup">
    <div class="modal-content dup-modal-content">
      <div class="modal-header dup-modal-header">
        <div class="dup-header-main">
          <h2>${escapeHtml(title)}</h2>
          <div class="dup-tab-strip" role="tablist">
            <button type="button" class="dup-tab-btn dup-tab-active" data-dup-tab="name" role="tab" aria-selected="true">${escapeHtml(
        tabName
    )}</button>
            <button type="button" class="dup-tab-btn" data-dup-tab="content" role="tab" aria-selected="false">${escapeHtml(
        tabContent
    )}</button>
          </div>
        </div>
        <button class="modal-close" data-action-modal="closeDup" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div id="dupPanelName">${renderNameDupBody(nameResults)}</div>
        <div id="dupPanelContent" style="display:none;">${renderContentDupPlaceholder()}</div>
      </div>
    </div></div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    setContentDupButtonsRunning(_contentDupRunning);
    if (_contentDupRunning) {
        if (_contentDupLastProgress) {
            applyContentDupProgressPayload(_contentDupLastProgress);
        } else {
            const st = document.getElementById('dupContentStatus');
            if (st) st.textContent = catalogFmt('ui.dup.content_loading');
        }
    }
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

function triggerStopBackgroundContentDupScan() {
    if (!_contentDupRunning) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.content_dup_not_running'), 2500);
        }
        return;
    }
    if (typeof window.vstUpdater?.cancelContentDuplicateScan === 'function') {
        void window.vstUpdater.cancelContentDuplicateScan().catch(() => {});
    }
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.content_dup_stop_requested'), 3500);
    }
}

function triggerStartBackgroundContentDupScan() {
    void runContentDupScanInternal();
}

async function runContentDupScanInternal() {
    if (_contentDupRunning) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.content_dup_already_running'), 3500);
        }
        return;
    }
    if (typeof window.vstUpdater?.findContentDuplicates !== 'function') {
        const status = document.getElementById('dupContentStatus');
        if (status) status.textContent = catalogFmt('ui.dup.content_err');
        return;
    }

    _contentDupRunning = true;
    if (typeof window !== 'undefined') window.__statusBarContentDupJob = true;
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    setContentDupButtonsRunning(true);
    const status = document.getElementById('dupContentStatus');
    const out = document.getElementById('dupContentResults');
    if (status) status.textContent = catalogFmt('ui.dup.content_loading');
    setContentDupBadgeWorking();

    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.content_dup_started'), 3500);
    }

    try {
        await ensureContentDupProgressListener();
        const res = await window.vstUpdater.findContentDuplicates();
        if (out) out.innerHTML = renderContentDupResults(res);
        if (status) status.textContent = '';
        if (typeof showToast === 'function' && typeof toastFmt === 'function' && res) {
            if (res.cancelled) {
                showToast(
                    toastFmt('toast.content_dup_cancelled', {
                        done: res.files_hashed ?? 0,
                        total: res.candidates_total ?? 0,
                        groups: (res.groups || []).length,
                        files: res.files_hashed || 0
                    }),
                    4500
                );
            } else {
                showToast(
                    toastFmt('toast.content_dup_done', {
                        groups: (res.groups || []).length,
                        files: res.files_hashed || 0
                    }),
                    4000
                );
            }
        }
    } catch (e) {
        const msg = e && e.message ? e.message : String(e);
        if (window.vstUpdater?.appendLog) {
            window.vstUpdater.appendLog(`CONTENT DUP SCAN ERROR — UI invoke: ${msg}`);
        }
        if (status) status.textContent = catalogFmt('ui.dup.content_err');
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.content_dup_failed', {err: msg}), 5000, 'error');
        }
    } finally {
        teardownContentDupProgressListener();
        clearContentDupBadge();
        _contentDupRunning = false;
        if (typeof window !== 'undefined') window.__statusBarContentDupJob = false;
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
        setContentDupButtonsRunning(false);
    }
}

// Event delegation for duplicates modal
document.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action-modal="closeDup"]');
    if (action) {
        if (e.target === action || action.classList.contains('modal-close')) {
            const modal = document.getElementById('dupModal');
            if (modal) modal.remove();
        }
    }
    const tab = e.target.closest('[data-dup-tab]');
    if (tab && document.getElementById('dupModal')?.contains(tab)) {
        switchDupTab(tab.dataset.dupTab);
    }
    if (e.target.closest('[data-action="dupContentScan"]')) {
        triggerStartBackgroundContentDupScan();
    }
    if (e.target.closest('[data-action="dupContentStop"]')) {
        triggerStopBackgroundContentDupScan();
    }
});

// Export for menu-action handler in ipc.js
window.showDuplicateReport = showDuplicateReport;
window.triggerStartBackgroundContentDupScan = triggerStartBackgroundContentDupScan;
window.triggerStopBackgroundContentDupScan = triggerStopBackgroundContentDupScan;
