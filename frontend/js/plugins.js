let scanProgressCleanup = null;
/** After user runs a DB filter during scan, skip streaming DOM appends until scan ends. */
let _pluginScanDbView = false;
let _pluginsLoaded = false;

function _ui(k, vars) {
    if (typeof appFmt !== 'function') return k;
    return vars ? appFmt(k, vars) : appFmt(k);
}

let _pluginOffset = 0;
let _pluginTotalCount = 0;
let _pluginTotalCountCapped = false;
let _pluginTotalUnfiltered = 0;
/** Monotonic id so stale `dbQueryPlugins` results never overwrite a newer filter. */
let _pluginQuerySeq = 0;

function showPluginQueryLoading(isLoadMore) {
    const list = document.getElementById('pluginList');
    if (!list) return;
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (x) => String(x);
    const label = typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…';
    document.getElementById('pluginQueryLoading')?.remove();
    const div = document.createElement('div');
    div.id = 'pluginQueryLoading';
    div.setAttribute('role', 'status');
    div.style.cssText = 'text-align:center;padding:32px 16px;';
    div.innerHTML = `<div class="spinner" style="width:26px;height:26px;margin:0 auto 10px;"></div><span style="color:var(--text-muted);font-size:12px;">${esc(label)}</span>`;
    if (isLoadMore) {
        document.getElementById('pluginLoadMore')?.remove();
        list.appendChild(div);
    } else {
        list.innerHTML = '';
        list.appendChild(div);
    }
}

function clearPluginQueryLoading() {
    document.getElementById('pluginQueryLoading')?.remove();
}

let _pluginSortKey = 'name';
let _pluginSortAsc = true;

async function loadPluginsFromDb() {
    if (_pluginsLoaded) return;
    const list = document.getElementById('pluginList');
    if (list && list.querySelector('#emptyState')) {
        list.innerHTML = `<div class="state-message"><div class="spinner"></div><h2>${_ui('ui.js.loading_plugins')}</h2></div>`;
    }
    try {
        _pluginOffset = 0;
        enablePluginCardAnimation(); // initial render gets the slide-in animation
        await fetchPluginPage();
        _pluginsLoaded = true;

        if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: _pluginTotalUnfiltered || allPlugins.length || 0});
        else document.getElementById('totalCount').textContent = (_pluginTotalUnfiltered || allPlugins.length || 0).toLocaleString();
        if (_pluginTotalCount > 0) {
            document.getElementById('btnCheckUpdates').disabled = false;
            const toolbar = document.getElementById('toolbar');
            if (toolbar) toolbar.style.display = 'flex';
        }

        // KVR cache for visible plugins
        try {
            const kvrCache = await window.vstUpdater.getKvrCache();
            applyKvrCache(allPlugins, kvrCache);
        } catch (e) {
            if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
        }

        resolveKvrDownloads();
    } catch (err) {
        showToast(toastFmt('toast.failed_load_plugin_scan', {err: err.message || err}), 4000, 'error');
    }
}

async function fetchPluginPage() {
    const search = document.getElementById('searchInput')?.value || '';
    const typeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('typeFilter') : null;
    const typeFilter = typeSet ? [...typeSet].join(',') : null;
    const statusSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('statusFilter') : null;
    const statusFilter = statusSet ? [...statusSet].join(',') : null;
    const seq = ++_pluginQuerySeq;
    const isLoadMore = _pluginOffset > 0;
    showPluginQueryLoading(isLoadMore);
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('searchInput', true);
    if (typeof yieldForFilterFieldPaint === 'function') await yieldForFilterFieldPaint();
    else await new Promise((r) => requestAnimationFrame(r));
    try {
        const result = await window.vstUpdater.dbQueryPlugins({
            search: search || null,
            type_filter: typeFilter,
            status_filter: statusFilter,
            sort_key: _pluginSortKey,
            sort_asc: _pluginSortAsc,
            search_regex: _lastPluginMode === 'regex',
            offset: _pluginOffset,
            limit: AUDIO_PAGE_SIZE,
        });
        if (seq !== _pluginQuerySeq) return;
        let plugins = result.plugins || [];
        _pluginTotalCount = result.totalCount || 0;
        _pluginTotalCountCapped = result.totalCountCapped === true;
        _pluginTotalUnfiltered = result.totalUnfiltered || 0;

        // Re-sort by fzf relevance score on the frontend (SQL can only do subsequence LIKE)
        if (search && plugins.length > 1) {
            const scored = plugins.map(p => ({
                p,
                score: searchScore(search, [p.name, p.manufacturer || ''], _lastPluginMode)
            }));
            scored.sort((a, b) => b.score - a.score);
            plugins = scored.map(s => s.p);
        }

        if (typeof yieldToBrowser === 'function') await yieldToBrowser();
        if (seq !== _pluginQuerySeq) return;

        // Keep allPlugins in sync for KVR/export compat
        if (_pluginOffset === 0) {
            allPlugins = plugins;
            _renderedPlugins = plugins;
            renderPlugins(allPlugins);
        } else {
            allPlugins.push(...plugins);
            _renderedPlugins = allPlugins;
            // Append new batch to DOM without re-rendering everything
            loadMorePlugins();
        }
        if (scanProgressCleanup) _pluginScanDbView = true;
        if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: _pluginTotalUnfiltered || allPlugins.length || 0});
        else document.getElementById('totalCount').textContent = (_pluginTotalUnfiltered || allPlugins.length || 0).toLocaleString();
    } catch (e) {
        if (seq !== _pluginQuerySeq) return;
        clearPluginQueryLoading();
        showToast(toastFmt('toast.plugin_query_failed', {err: e}), 4000, 'error');
    } finally {
        if (seq === _pluginQuerySeq && typeof setFilterFieldLoading === 'function') setFilterFieldLoading('searchInput', false);
    }
}

/** Full list for export when SQLite-backed UI has only paginated `allPlugins` (or scan-in-progress buffer). */
const _PLUGIN_EXPORT_MAX = 100000;

async function fetchPluginsForExport() {
    const search = document.getElementById('searchInput')?.value || '';
    const typeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('typeFilter') : null;
    const typeFilter = typeSet ? [...typeSet].join(',') : null;
    const statusSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('statusFilter') : null;
    const statusFilter = statusSet ? [...statusSet].join(',') : null;
    let total = _pluginTotalCount || 0;
    if (total <= 0) {
        try {
            const probe = await window.vstUpdater.dbQueryPlugins({
                search: search || null,
                type_filter: typeFilter,
                status_filter: statusFilter,
                sort_key: _pluginSortKey,
                sort_asc: _pluginSortAsc,
                search_regex: _lastPluginMode === 'regex',
                offset: 0,
                limit: 1,
            });
            total = probe.totalCount || 0;
        } catch {
            return [];
        }
    }
    const n = Math.min(total, _PLUGIN_EXPORT_MAX);
    if (n <= 0) return [];
    const result = await window.vstUpdater.dbQueryPlugins({
        search: search || null,
        type_filter: typeFilter,
        status_filter: statusFilter,
        sort_key: _pluginSortKey,
        sort_asc: _pluginSortAsc,
        search_regex: _lastPluginMode === 'regex',
        offset: 0,
        limit: n,
    });
    let plugins = result.plugins || [];
    if (search && plugins.length > 1) {
        const scored = plugins.map((p) => ({
            p,
            score: searchScore(search, [p.name, p.manufacturer || ''], _lastPluginMode)
        }));
        scored.sort((a, b) => b.score - a.score);
        plugins = scored.map((s) => s.p);
    }
    return plugins;
}

function getPluginExportableCount() {
    if (typeof scanProgressCleanup !== 'undefined' && scanProgressCleanup) {
        if (typeof _pluginScanDbView !== 'undefined' && _pluginScanDbView) {
            return Math.max(_pluginTotalCount || 0, typeof allPlugins !== 'undefined' ? allPlugins.length : 0);
        }
        return typeof allPlugins !== 'undefined' ? allPlugins.length : 0;
    }
    return Math.max(_pluginTotalCount || 0, typeof allPlugins !== 'undefined' ? allPlugins.length : 0);
}

async function scanPlugins(resume = false, overrideRoots = null) {
    showGlobalProgress();
    try {
        currentOperation = 'scan';
        const btn = document.getElementById('btnScan');
        const resumeBtn = document.getElementById('btnResumeScan');
        const progress = document.getElementById('progressBar');
        const progressFill = progress.querySelector('.progress-fill');
        const list = document.getElementById('pluginList');

        const stopBtn = document.getElementById('btnStopPlugins');
        const excludePaths = resume ? allPlugins.map(p => p.path) : null;

        if (typeof btnLoading === 'function') btnLoading(btn, true);
        btn.disabled = true;
        resumeBtn.style.display = 'none';
        stopBtn.style.display = '';
        btn.innerHTML = resume ? `&#8635; ${_ui('ui.js.resuming_btn')}` : `&#8635; ${_ui('ui.js.scanning_btn')}`;
        progress.classList.add('active');
        progressFill.style.animation = 'none';
        progressFill.style.width = '0%';

        if (!resume) {
            _pluginScanDbView = false;
        }

        let scanPluginDomActive = false;
        let firstScanningChunk = true;
        const eta = createETA();
        let streamPluginCount = 0;
        try {
            if (scanProgressCleanup) scanProgressCleanup();
            // `listen()` is async — must await subscription before invoke or `scan-progress` events are lost.
            scanProgressCleanup = await window.vstUpdater.onScanProgress((data) => {
                if (data.phase === 'start') {
                    _pluginScanDbView = false;
                    btn.innerHTML = `&#8635; 0 / ${data.total}`;
                    eta.start();
                } else if (data.phase === 'scanning') {
                    const total = data.total || 0;
                    const pct = total ? Math.round((data.processed / total) * 100) : 0;
                    progressFill.style.width = pct + '%';
                    const etaStr = eta.estimate(data.processed, data.total);
                    btn.innerHTML = `&#8635; ${data.processed} / ${data.total}${etaStr ? ' — ' + etaStr : ''}`;
                    const batch = data.plugins || [];
                    streamPluginCount += batch.length;
                    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: streamPluginCount});
                    else document.getElementById('totalCount').textContent = streamPluginCount.toLocaleString();

                    const allowDom =
                        scanPluginDomActive ||
                        (typeof isPluginListEmptyForScan === 'function' && isPluginListEmptyForScan());
                    if (!allowDom) {
                        return;
                    }
                    scanPluginDomActive = true;

                    if (firstScanningChunk) {
                        firstScanningChunk = false;
                        if (!resume) {
                            allPlugins = [];
                            list.innerHTML = '';
                        }
                    }
                    allPlugins.push(...batch);

                    const fragment = document.createDocumentFragment();
                    const temp = document.createElement('div');
                    temp.innerHTML = batch.map(p => buildPluginCardHtml(p)).join('');
                    const scanTypeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('typeFilter') : null;
                    const scanStatusSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('statusFilter') : null;
                    const scanSearch = (document.getElementById('searchInput')?.value || '').trim().toLowerCase();
                    const hasFilter = !!(scanTypeSet || scanStatusSet || scanSearch);
                    while (temp.firstChild) {
                        const c = temp.firstChild;
                        if (hasFilter && c.dataset) {
                            const t = c.dataset.pluginType;
                            const n = c.dataset.pluginName || '';
                            const m = c.dataset.pluginMfg || '';
                            let match = true;
                            if (scanTypeSet && t && !scanTypeSet.has(t)) match = false;
                            if (match && scanStatusSet && c.dataset.pluginStatus && !scanStatusSet.has(c.dataset.pluginStatus)) match = false;
                            if (match && scanSearch && !n.includes(scanSearch) && !m.includes(scanSearch)) match = false;
                            if (!match) c.style.display = 'none';
                        }
                        fragment.appendChild(c);
                    }
                    if (!_pluginScanDbView) list.appendChild(fragment);
                }
            });

            const customDirs = (overrideRoots && overrideRoots.length > 0)
                ? overrideRoots
                : (prefs.getItem('customDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
            const result = await window.vstUpdater.scanPlugins(customDirs.length ? customDirs : undefined, excludePaths);
            // Final state -- merge with existing on resume
            if (resume) {
                allPlugins = [...allPlugins, ...result.plugins];
            } else {
                allPlugins = result.plugins;
            }

            if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: allPlugins.length});
            else document.getElementById('totalCount').textContent = allPlugins.length;
            // Refresh header count immediately — scan already saved server-side; don't wait for next fetchPluginPage
            _pluginTotalUnfiltered = allPlugins.length;
            document.getElementById('btnCheckUpdates').disabled = allPlugins.length === 0;

            const dirsSection = document.getElementById('dirsSection');
            dirsSection.style.display = 'block';
            document.getElementById('dirsList').innerHTML = buildDirsTable(result.directories, allPlugins);

            if (allPlugins.length === 0) {
                list.innerHTML = `<div class="state-message"><div class="state-icon">&#128270;</div><h2>${_ui('ui.js.no_plugins_found')}</h2><p>${_ui('ui.js.no_plugins_found_body')}</p></div>`;
            } else {
                enablePluginCardAnimation(); // post-scan render gets the slide-in animation
                renderPlugins(allPlugins);
                resolveKvrDownloads();
            }
            if (result.stopped && allPlugins.length > 0) {
                resumeBtn.style.display = '';
            }
            if (typeof postScanCompleteToast === 'function') {
                postScanCompleteToast(
                    !!result.stopped,
                    'toast.post_scan_plugins_complete',
                    'toast.post_scan_plugins_stopped',
                    {n: allPlugins.length.toLocaleString()},
                );
            }
        } catch (err) {
            const errMsg = err.message || err || 'Unknown error';
            list.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>${_ui('ui.js.scan_error')}</h2><p>${errMsg}</p></div>`;
            showToast(toastFmt('toast.plugin_scan_failed', {errMsg}), 4000, 'error');
        }

        if (scanProgressCleanup) {
            scanProgressCleanup();
            scanProgressCleanup = null;
        }
        _pluginScanDbView = false;
        scanPluginDomActive = false;
        hideGlobalProgress();
        stopBtn.style.display = 'none';
        btn.disabled = false;
        if (typeof btnLoading === 'function') btnLoading(btn, false);
        btn.innerHTML = `&#8635; ${_ui('ui.js.scan_plugins_btn')}`;
        progressFill.style.width = '100%';
        setTimeout(() => {
            progress.classList.remove('active');
            progressFill.style.animation = '';
            progressFill.style.width = '0%';
        }, 400);
    } finally {
        if (currentOperation === 'scan') currentOperation = null;
    }
}

/** Matches `kvr_cache` + SQL `query_plugins` status_filter (update / current / unknown). */
function pluginStatusCategory(p) {
    if (p.hasUpdate === true) return 'update';
    if (p.hasUpdate === undefined) return 'unknown';
    if (p.source === 'not-found') return 'unknown';
    return 'current';
}

function buildPluginCardHtml(p) {
    const typeClass = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : p.type === 'CLAP' ? 'type-clap' : 'type-au';
    let versionHtml = `<span class="version-current">v${p.version}</span>`;
    let badgeHtml = '';
    const mfgUrl = p.manufacturerUrl || null;
    const mfgBtn = mfgUrl
        ? `<button class="btn-small btn-mfg" data-action="openUpdate" data-url="${mfgUrl}" title="${mfgUrl}">&#127760;</button>`
        : `<button class="btn-small btn-no-web" disabled title="${_ui('ui.js.no_mfg_website')}">&#128683;</button>`;
    const kvrUrl = p.kvrUrl || buildKvrUrl(p.name, p.manufacturer);
    const kvrBtn = `<button class="btn-small btn-kvr" data-action="openKvr" data-url="${kvrUrl.replace(/'/g, '&apos;')}" data-name="${escapeHtml(p.name)}" title="${escapeHtml(kvrUrl)}">KVR</button>`;
    // Show download button only if plugin has an update available
    const dlUrl = (p.hasUpdate && p.updateUrl) ? p.updateUrl : null;
    const dlBtn = dlUrl
        ? `<button class="btn-small btn-download btn-dl-kvr" data-action="openUpdate" data-url="${dlUrl.replace(/'/g, '&apos;')}" title="${escapeHtml(dlUrl)}">&#11015; ${_ui('ui.js.download')}</button>`
        : '';
    let actionsHtml = dlBtn + kvrBtn + mfgBtn + `<button class="btn-small btn-folder" data-action="openFolder" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}">&#128193;</button>`;

    if (p.hasUpdate !== undefined) {
        if (p.hasUpdate) {
            versionHtml = `<span class="version-current">v${p.currentVersion}</span>
        <span class="version-arrow">&#8594;</span>
        <span class="version-latest">v${p.latestVersion}</span>`;
            badgeHtml = `<span class="badge badge-update">${_ui('ui.js.badge_update_available')}</span>`;
        } else if (p.source === 'not-found') {
            badgeHtml = `<span class="badge badge-unknown">${_ui('ui.js.badge_unknown_latest')}</span>`;
        } else {
            badgeHtml = `<span class="badge badge-current">${_ui('ui.js.badge_up_to_date')}</span>`;
        }
    }

    return `
    <div class="plugin-card" data-path="${escapeHtml(p.path)}" data-plugin-type="${escapeHtml(p.type)}" data-plugin-status="${escapeHtml(pluginStatusCategory(p))}" data-plugin-name="${escapeHtml((p.name || '').toLowerCase())}" data-plugin-mfg="${escapeHtml((p.manufacturer || '').toLowerCase())}">
      <div class="plugin-info">
        <h3 title="${escapeHtml(p.name)}">${_lastPluginSearch ? highlightMatch(p.name, _lastPluginSearch, _lastPluginMode) : escapeHtml(p.name)}${typeof rowBadges === 'function' ? ' ' + rowBadges(p.path) : ''}</h3>
        <div class="plugin-meta">
          <span class="plugin-type ${typeClass}">${p.type}</span>
          <span>${_lastPluginSearch ? highlightMatch(p.manufacturer || '', _lastPluginSearch, _lastPluginMode) : escapeHtml(p.manufacturer || '')}</span>
          <span>${p.size}</span>
          <span>${p.modified}</span>
          ${(p.architectures && p.architectures.length) ? p.architectures.map(a => `<span class="arch-badge arch-${a.toLowerCase()}">${a}</span>`).join('') : ''}
        </div>
      </div>
      <div class="plugin-version">${versionHtml}</div>
      ${badgeHtml}
      <div class="plugin-actions">${actionsHtml}</div>
    </div>`;
}

let updateProgressCleanup = null;

async function checkUpdates() {
    showGlobalProgress();
    const btn = document.getElementById('btnCheckUpdates');
    const progress = document.getElementById('progressBar');
    const progressFill = progress.querySelector('.progress-fill');

    const statusBar = document.getElementById('statusBar');
    const statusText = document.getElementById('statusText');
    const statusStats = document.getElementById('statusStats');

    currentOperation = 'updates';
    showStopButton();
    btn.disabled = true;
    btn.innerHTML = `&#9889; ${_ui('ui.js.checking_updates_btn')}`;
    progress.classList.add('active');
    progressFill.style.animation = 'none';
    progressFill.style.width = '0%';
    statusBar.classList.add('active');
    statusText.textContent = _ui('ui.js.init_update_check');
    statusStats.innerHTML = '';

    // Track which plugins have been updated (by path)
    const updatedByPath = new Map();

    const updateEta = createETA();
    if (updateProgressCleanup) updateProgressCleanup();
    updateProgressCleanup = window.vstUpdater.onUpdateProgress((data) => {
        if (data.phase === 'start') {
            btn.innerHTML = `&#9889; 0 / ${data.total}`;
            statusText.textContent = _ui('ui.js.searching_updates', {n: data.total});
            updateEta.start();
        } else if (data.phase === 'checking') {
            const total = data.total || 0;
            const pct = total ? Math.round((data.processed / total) * 100) : 0;
            progressFill.style.width = pct + '%';
            const updateEtaStr = updateEta.estimate(data.processed, data.total);
            btn.innerHTML = `&#9889; ${data.processed} / ${data.total}${updateEtaStr ? ' — ' + updateEtaStr : ''}`;

            // Show current plugin being checked
            const lastPlugin = data.plugins[data.plugins.length - 1];
            if (lastPlugin) {
                const mfg = lastPlugin.manufacturer !== 'Unknown' ? lastPlugin.manufacturer + ' ' : '';
                const updateRemaining = updateEta.estimate(data.processed, data.total);
                statusText.textContent = _ui('ui.js.status_checking_plugin', {
                    mfg,
                    name: lastPlugin.name,
                    processed: data.processed,
                    total: data.total,
                    remaining: updateRemaining ? _ui('ui.js.remaining', {eta: updateRemaining}) : '',
                });
            }

            // Update individual plugin cards in-place and save to KVR cache
            const cacheEntries = [];
            for (const p of data.plugins) {
                updatedByPath.set(p.path, p);
                // Update allPlugins entry in-place (O(1) via path index instead of O(N) findIndex).
                const existing = findByPath(allPlugins, p.path);
                if (existing) Object.assign(existing, p);
                // Queue cache entry
                if (p.source && p.source !== 'not-found') {
                    cacheEntries.push({
                        key: kvrCacheKey(p),
                        kvrUrl: p.kvrUrl || null,
                        updateUrl: p.updateUrl || null,
                        latestVersion: p.latestVersion || null,
                        hasUpdate: p.hasUpdate || false,
                        source: p.source,
                    });
                }
                // Find and replace the card in the DOM
                const htmlPath = escapeHtml(p.path);
                const card = document.querySelector(`.plugin-card[data-path="${CSS.escape(htmlPath)}"]`);
                if (card) {
                    const temp = document.createElement('div');
                    temp.innerHTML = buildPluginCardHtml(p);
                    const newCard = temp.firstElementChild;
                    card.replaceWith(newCard);
                }
            }

            // Persist to KVR cache
            if (cacheEntries.length > 0) {
                window.vstUpdater.updateKvrCache(cacheEntries).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
            }

            // Update live stats
            const checkedPlugins = [...updatedByPath.values()];
            const withUpdates = checkedPlugins.filter(p => p.hasUpdate).length;
            const unknown = checkedPlugins.filter(p => !p.hasUpdate && p.source === 'not-found').length;
            const upToDate = checkedPlugins.filter(p => !p.hasUpdate && p.source !== 'not-found').length;
            const kvrFound = checkedPlugins.filter(p => p.source === 'kvr' || p.source === 'kvr-ddg').length;
            const pending = data.total - data.processed;
            document.getElementById('updateCount').textContent = withUpdates;
            document.getElementById('upToDateCount').textContent = upToDate;
            const ucEl = document.getElementById('unknownCount');
            if (ucEl) ucEl.textContent = unknown;

            statusStats.innerHTML =
                `<span class="stat-avail">${withUpdates} ${_ui('ui.js.stat_updates')}</span>` +
                `<span class="stat-up">${upToDate} ${_ui('ui.js.stat_current')}</span>` +
                `<span style="color: var(--text-muted);">${unknown} ${_ui('ui.js.stat_unknown')}</span>` +
                `<span style="color: var(--yellow);">${kvrFound} ${_ui('ui.js.stat_kvr_label')}</span>` +
                `<span class="stat-pending">${pending} ${_ui('ui.js.stat_pending')}</span>`;
        }
    });

    try {
        allPlugins = await window.vstUpdater.checkUpdates(allPlugins);
        pluginsWithUpdates = allPlugins.filter(p => p.hasUpdate);

        const finalUnknown = allPlugins.filter(p => !p.hasUpdate && p.source === 'not-found').length;
        document.getElementById('upToDateCount').textContent =
            allPlugins.filter(p => !p.hasUpdate && p.source !== 'not-found').length;
        document.getElementById('updateCount').textContent = pluginsWithUpdates.length;
        const ucEl2 = document.getElementById('unknownCount');
        if (ucEl2) ucEl2.textContent = finalUnknown;

        if (pluginsWithUpdates.length > 0) {
            const batchBar = document.getElementById('batchBar');
            batchBar.classList.add('visible');
            const bc = document.getElementById('pluginBatchUpdateCount');
            if (bc) {
                const n = pluginsWithUpdates.length;
                bc.textContent =
                    n === 1
                        ? catalogFmt('menu.plugins_with_updates_one')
                        : catalogFmt('menu.plugins_with_updates', {n});
            }
            batchIndex = 0;
            updateBatchUI();
        }

        renderPlugins(allPlugins);
    } catch (err) {
        const updateErr = err.message || err || catalogFmt('toast.unknown_error');
        if (updateErr !== 'stopped') {
            showToast(toastFmt('toast.update_check_failed', {updateErr}), 4000, 'error');
        }
    }

    if (updateProgressCleanup) {
        updateProgressCleanup();
        updateProgressCleanup = null;
    }
    hideGlobalProgress();
    hideStopButton();
    statusBar.classList.remove('active');
    btn.disabled = false;
    btn.innerHTML = `&#9889; ${_ui('ui.js.check_updates_btn')}`;
    progressFill.style.width = '100%';
    setTimeout(() => {
        progress.classList.remove('active');
        progressFill.style.animation = '';
        progressFill.style.width = '0%';
    }, 400);
}

// Uses global page size from settings (AUDIO_PAGE_SIZE)
let _pluginRenderCount = 0;
let _renderedPlugins = [];

function renderPlugins(plugins) {
    clearPluginQueryLoading();
    updateExportButton();
    _renderedPlugins = plugins;
    _pluginRenderCount = 0;
    const list = document.getElementById('pluginList');

    if (plugins.length === 0) {
        list.innerHTML = `<div class="state-message"><div class="state-icon">&#128269;</div><h2>${_ui('ui.js.no_matching_plugins')}</h2></div>`;
        return;
    }

    // Strip entrance-animation modifier for filter re-renders so cards don't
    // restart their slide-in animation on every keystroke (causes visible flash).
    list.classList.remove('plugin-list-animated');

    // Render first 50 immediately, then progressively add more
    const INITIAL = 50;
    const batch = plugins.slice(0, INITIAL);
    list.innerHTML = batch.map(p => buildPluginCardHtml(p)).join('');
    _pluginRenderCount = batch.length;

    if (plugins.length > INITIAL) {
        const totalShown = _pluginTotalCountCapped ? _pluginTotalCount.toLocaleString() + '+' : _pluginTotalCount;
        list.insertAdjacentHTML('beforeend',
            `<div class="plugin-load-more" id="pluginLoadMore" data-action="loadMorePlugins" style="text-align:center;padding:16px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        ${_ui('ui.js.load_more_hint', {shown: _pluginRenderCount, total: totalShown})}
      </div>`);
    }

    list.classList.add('fade-in');
    if (typeof updatePluginDiskUsage === 'function') updatePluginDiskUsage();
}

// Re-enable entrance animation for the next renderPlugins call. Used by scan
// completion and initial load so the first-draw still gets the slide-in effect.
function enablePluginCardAnimation() {
    const list = document.getElementById('pluginList');
    if (list) list.classList.add('plugin-list-animated');
}

function loadMorePlugins() {
    clearPluginQueryLoading();
    // Remove the load-more button
    const loadMore = document.getElementById('pluginLoadMore');
    if (loadMore) loadMore.remove();

    // Render next batch from already-loaded plugins
    const list = document.getElementById('pluginList');
    const BATCH = 50;
    const nextBatch = _renderedPlugins.slice(_pluginRenderCount, _pluginRenderCount + BATCH);
    list.insertAdjacentHTML('beforeend', nextBatch.map(p => buildPluginCardHtml(p)).join(''));
    _pluginRenderCount += nextBatch.length;

    // If more plugins exist in DB beyond what we've fetched, fetch next page
    const pageSize = typeof AUDIO_PAGE_SIZE !== 'undefined' ? AUDIO_PAGE_SIZE : 200;
    const canFetchMoreFromDb = _pluginTotalCountCapped
        ? (allPlugins.length > 0 && allPlugins.length % pageSize === 0)
        : (allPlugins.length < _pluginTotalCount);
    if (_pluginRenderCount >= allPlugins.length && canFetchMoreFromDb) {
        _pluginOffset = allPlugins.length;
        fetchPluginPage();
        return;
    }

    // If more to render locally, show load more
    const totalShown = _pluginTotalCountCapped ? _pluginTotalCount.toLocaleString() + '+' : _pluginTotalCount;
    const moreFromDbHint = _pluginTotalCountCapped
        ? (allPlugins.length > 0 && allPlugins.length % pageSize === 0)
        : (allPlugins.length < _pluginTotalCount);
    if (_pluginRenderCount < _renderedPlugins.length || moreFromDbHint) {
        list.insertAdjacentHTML('beforeend',
            `<div class="plugin-load-more" id="pluginLoadMore" data-action="loadMorePlugins" style="text-align:center;padding:16px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        ${_ui('ui.js.load_more_hint', {shown: _pluginRenderCount, total: totalShown})}
      </div>`);
    }
}


let _lastPluginSearch = '';
let _lastPluginMode = 'fuzzy';

registerFilter('filterPlugins', {
    inputId: 'searchInput',
    regexToggleId: 'regexPlugins',
    resetOffset() {
        _pluginOffset = 0;
    },
    fetchFn() {
        _lastPluginSearch = this.lastSearch || '';
        _lastPluginMode = this.lastMode || 'fuzzy';
        fetchPluginPage();
    },
});

function filterPlugins() {
    applyFilter('filterPlugins');
}

function openUpdate(url) {
    window.vstUpdater.openUpdateUrl(url);
}

async function openKvr(btn, directUrl, pluginName) {
    const origText = btn.textContent;
    btn.textContent = '...';
    btn.disabled = true;
    try {
        const result = await window.vstUpdater.resolveKvr(directUrl, pluginName);
        const productUrl = result.productUrl || directUrl;
        btn.title = productUrl;
        btn.onclick = () => openUpdate(productUrl);
        window.vstUpdater.openUpdateUrl(productUrl);

        // If a download link was found and plugin has an update, add download button
        if (result.downloadUrl) {
            const card = btn.closest('.plugin-card');
            const pluginPath = card ? card.dataset.path : null;
            const plugin = pluginPath && findByPath(allPlugins, pluginPath);
            if (plugin && plugin.hasUpdate && card && !card.querySelector('.btn-dl-kvr')) {
                const dlBtn = document.createElement('button');
                dlBtn.className = 'btn-small btn-download btn-dl-kvr';
                dlBtn.title = result.downloadUrl;
                dlBtn.innerHTML = `&#11015; ${_ui('ui.js.download')}`;
                dlBtn.onclick = () => openUpdate(result.downloadUrl);
                btn.parentNode.insertBefore(dlBtn, btn);
            }
        }
    } catch {
        window.vstUpdater.openUpdateUrl(directUrl);
    }
    btn.textContent = origText;
    btn.disabled = false;
}

function openFolder(pluginPath) {
    window.vstUpdater.openPluginFolder(pluginPath).then(() => showToast(toastFmt('toast.revealed_in_finder'))).catch(e => showToast(toastFmt('toast.failed', {err: e}), 4000, 'error'));
}

let batchIndex = 0;

function updateBatchUI() {
    const progress = document.getElementById('batchProgress');
    const nameEl = document.getElementById('batchCurrentName');
    const btnNext = document.getElementById('btnNext');
    const btnSkip = document.getElementById('btnSkip');

    if (batchIndex >= pluginsWithUpdates.length) {
        progress.textContent = _ui('ui.js.batch_all_done');
        nameEl.textContent = '';
        btnNext.disabled = true;
        btnNext.textContent = _ui('ui.js.batch_all_done_btn');
        btnSkip.style.display = 'none';
        return;
    }

    const current = pluginsWithUpdates[batchIndex];
    progress.textContent = _ui('ui.js.batch_progress', {n: batchIndex + 1, total: pluginsWithUpdates.length});
    nameEl.textContent = _ui('ui.js.batch_next', {name: current.name});
    btnNext.disabled = false;
    btnNext.textContent = _ui('ui.js.batch_open_next');
    btnSkip.style.display = '';
}

function openNextUpdate() {
    if (batchIndex >= pluginsWithUpdates.length) return;
    const plugin = pluginsWithUpdates[batchIndex];
    if (plugin.updateUrl) {
        window.vstUpdater.openUpdateUrl(plugin.updateUrl);
    }
    batchIndex++;
    updateBatchUI();
}

function skipUpdate() {
    if (batchIndex >= pluginsWithUpdates.length) return;
    batchIndex++;
    updateBatchUI();
}

