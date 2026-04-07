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
    return `<p class="dup-content-hint" style="color:var(--text-muted);font-size:12px;margin-bottom:12px;">${escapeHtml(
        hint
    )}</p>
    <button type="button" class="cyber-btn" data-action="dupContentScan" id="dupContentScanBtn">${escapeHtml(
        btn
    )}</button>
    <p id="dupContentStatus" style="margin-top:12px;font-size:12px;color:var(--text-muted);"></p>
    <div id="dupContentResults"></div>`;
}

function renderContentDupResults(payload) {
    if (!payload || !payload.groups || payload.groups.length === 0) {
        const empty = catalogFmt('ui.dup.content_empty');
        return `<p class="state-message" style="padding:12px 0;">${escapeHtml(empty)}</p>`;
    }
    let html = '';
    const skippedPart =
        payload.skipped > 0 ? catalogFmt('ui.dup.skipped_suffix', {n: payload.skipped}) : '';
    const sum = catalogFmt('ui.dup.content_summary', {
        groups: payload.groups.length,
        files: payload.files_hashed || 0,
        skipped: skippedPart
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
        t.classList.toggle('dup-tab-active', t.dataset.dupTab === which);
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

    const html = `<div class="modal-overlay" id="dupModal" data-action-modal="closeDup">
    <div class="modal-content">
      <div class="modal-header">
        <h2>${escapeHtml(title)}</h2>
        <div class="dup-tab-strip" style="display:flex;gap:8px;margin:8px 0;flex-wrap:wrap;">
          <button type="button" class="cyber-btn dup-tab-btn dup-tab-active" data-dup-tab="name">${escapeHtml(
        tabName
    )}</button>
          <button type="button" class="cyber-btn dup-tab-btn" data-dup-tab="content">${escapeHtml(
        tabContent
    )}</button>
        </div>
        <button class="modal-close" data-action-modal="closeDup" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div id="dupPanelName">${renderNameDupBody(nameResults)}</div>
        <div id="dupPanelContent" style="display:none;">${renderContentDupPlaceholder()}</div>
      </div>
    </div></div>`;
    document.body.insertAdjacentHTML('beforeend', html);
}

async function runContentDupScan() {
    const status = document.getElementById('dupContentStatus');
    const btn = document.getElementById('dupContentScanBtn');
    const out = document.getElementById('dupContentResults');
    if (typeof window.vstUpdater?.findContentDuplicates !== 'function') {
        if (status) status.textContent = catalogFmt('ui.dup.content_err');
        return;
    }
    if (btn) btn.disabled = true;
    if (status) status.textContent = catalogFmt('ui.dup.content_loading');
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.content_dup_scanning'), 3500);
    }

    let unlistenFn;
    try {
        if (window.__TAURI__?.event?.listen) {
            unlistenFn = await window.__TAURI__.event.listen('content-dup-progress', (e) => {
                const pl = e.payload;
                if (status && pl && pl.done != null && pl.total) {
                    status.textContent = `${pl.done} / ${pl.total}`;
                }
            });
        }
        const res = await window.vstUpdater.findContentDuplicates();
        if (out) out.innerHTML = renderContentDupResults(res);
        if (status) status.textContent = '';
        if (typeof showToast === 'function' && typeof toastFmt === 'function' && res) {
            showToast(
                toastFmt('toast.content_dup_done', {
                    groups: (res.groups || []).length,
                    files: res.files_hashed || 0
                }),
                4000
            );
        }
    } catch (e) {
        const msg = e && e.message ? e.message : String(e);
        if (status) status.textContent = catalogFmt('ui.dup.content_err');
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.content_dup_failed', {err: msg}), 5000, 'error');
        }
    } finally {
        if (typeof unlistenFn === 'function') {
            try {
                unlistenFn();
            } catch {
                /* ignore */
            }
        }
        if (btn) btn.disabled = false;
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
        runContentDupScan();
    }
});
