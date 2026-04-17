// ── Crate tab ────────────────────────────────────────────────────────────────
// Sample browser with category tree, facet filters, favorite packs, vim nav,
// auto-preview on cursor move, inline waveform, drag-out to DAW, and
// right-click "Similar" using the existing fingerprint cache. Data source:
// `sample_analysis` + `sample_packs` + `favorite_sample_packs` via `crate_*` IPC.
// Row audio playback reuses `previewAudio` (same path as Samples tab).

const _crate = {
    initialized: false,
    loaded: false,
    // Filter state
    filters: {
        category: '',   // category name OR parent bucket name; '' = all
        loop: 'any',    // 'any' | 'loop' | 'oneshot'
        packId: null,
        mfrId: null,
        bpmMin: null,
        bpmMax: null,
        key: '',
        favPacksOnly: false,
    },
    // Data caches
    categoryCounts: [],  // [{name, parent_name, count}]
    facets: null,        // {packs, manufacturers, keys}
    favoritePackIds: new Set(),
    rows: [],            // currently displayed rows
    totalRows: 0,
    navIndex: -1,
    pageSize: 200,
    // Vim nav auto-preview debouncer
    autoPlayTimer: null,
    lastAutoPlayPath: null,
    // Wavform rendering: we reuse audio.js's _waveformCache + fetchWaveformPreviewFromEngine.
    // Only track which row paths are currently being fetched so we don't double-schedule.
    waveformInflight: new Set(),
    // Bpm/search non-registry debouncer
    bpmTimer: null,
    // Collapsed parent buckets in the tree
    collapsedParents: new Set(),
    // Last applied search state (set by registerFilter fetchFn).
    lastSearch: '',
    lastMode: 'fuzzy',
};

// Register with the shared filter registry so the standard search-box + regex button work
// identically to Samples/DAW/Presets. `applyFilter('filterCrate')` fires fetchFn with
// cfg.lastSearch / cfg.lastMode set — we snapshot those into _crate and re-query.
if (typeof registerFilter === 'function') {
    registerFilter('filterCrate', {
        inputId: 'crateSearchInput',
        regexToggleId: 'regexCrate',
        debounceMs: 250,
        resetOffset() {
            _crate.pageOffset = 0;
        },
        fetchFn() {
            _crate.lastSearch = this.lastSearch || '';
            _crate.lastMode = this.lastMode || 'fuzzy';
            void runCrateQuery();
        },
    });
}

function filterCrate() {
    if (typeof applyFilter === 'function') applyFilter('filterCrate');
}
window.filterCrate = filterCrate;

async function loadCrateTab() {
    if (_crate.loaded) return;
    _crate.initialized = true;
    const host = document.getElementById('crateResults');
    if (host && !host.innerHTML.trim()) {
        const label = typeof catalogFmt === 'function' ? catalogFmt('ui.crate.loading') : 'Loading...';
        host.innerHTML = `<div class="crate-empty" style="text-align:center;padding:40px;"><div class="spinner" style="width:28px;height:28px;margin:0 auto 12px;"></div><div style="color:var(--text-muted);font-size:12px;">${typeof escapeHtml === 'function' ? escapeHtml(label) : label}</div></div>`;
    }
    await refreshCrateAll();
    _crate.loaded = true;
}
window.loadCrateTab = loadCrateTab;
window.runCrateSampleAnalysis = runCrateSampleAnalysis;
window.stopCrateSampleAnalysis = stopCrateSampleAnalysis;

async function refreshCrateAll() {
    try {
        // Seed categories + manufacturers first so the tree always lists every known drum/bass/
        // melodic/fx category even before any sample_analysis rows exist. Safe to re-run — it's
        // an upsert on `name`.
        if (window.vstUpdater?.sampleAnalysisSeed) {
            try { await window.vstUpdater.sampleAnalysisSeed(); } catch { /* non-fatal */ }
        }
        const [counts, facets, favIds] = await Promise.all([
            window.vstUpdater.crateCategoryCounts(),
            window.vstUpdater.crateFacets(),
            window.vstUpdater.crateFavoritePacksList(),
        ]);
        _crate.categoryCounts = Array.isArray(counts) ? counts : [];
        _crate.facets = facets || {packs: [], manufacturers: [], keys: []};
        _crate.favoritePackIds = new Set(Array.isArray(favIds) ? favIds : []);
    } catch (err) {
        _crate.categoryCounts = [];
        _crate.facets = {packs: [], manufacturers: [], keys: []};
        _crate.favoritePackIds = new Set();
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_load_failed', {err}), 4000, 'error');
    }
    renderCrateCategoryTree();
    renderCrateFacets();
    renderCrateFavoritePacks();
    updateCrateAnalyzedStatus();
    void refreshCratePrefetchStats();
    await runCrateQuery();
}

function updateCrateAnalyzedStatus() {
    const el = document.getElementById('crateAnalyzedStatus');
    if (!el) return;
    const total = _crate.categoryCounts.reduce((s, r) => s + (r.count || 0), 0);
    if (total > 0) {
        el.textContent = `${total.toLocaleString()} analyzed`;
        el.classList.remove('crate-needs-analysis');
        el.innerHTML = `${total.toLocaleString()} analyzed`;
        return;
    }
    el.classList.add('crate-needs-analysis');
    // Surface a one-click Run Analysis button when the library has never been scored.
    // Same IPC the ALS Generator uses — `sampleAnalysisStart`.
    el.innerHTML = `<button type="button" class="btn-small crate-run-analysis" data-action="crate-run-analysis" title="Detect categories + packs + BPM + key for all samples">Run Analysis</button>`;
}

async function runCrateSampleAnalysis() {
    if (!window.vstUpdater?.sampleAnalysisStart) return;
    try {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_analysis_started'), 2000);
        updateCrateAnalysisButtons(true);
        await window.vstUpdater.sampleAnalysisStart();
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_analysis_failed', {err}), 4000, 'error');
        updateCrateAnalysisButtons(false);
    }
}

async function stopCrateSampleAnalysis() {
    if (!window.vstUpdater?.sampleAnalysisStop) return;
    try {
        await window.vstUpdater.sampleAnalysisStop();
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_analysis_stopped'), 2000);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_analysis_stop_failed', {err}), 4000, 'error');
    }
}

function updateCrateAnalysisButtons(running) {
    const startBtn = document.getElementById('crateAnalysisStartBtn');
    const stopBtn = document.getElementById('crateAnalysisStopBtn');
    if (startBtn) startBtn.hidden = running;
    if (stopBtn) stopBtn.hidden = !running;
}

/** Update global status bar badge for sample analysis progress. */
function updateCrateSampleAnalysisBadge(payload) {
    const badge = document.getElementById('bgSampleAnalysisBadge');
    if (!badge) return;

    const phase = payload?.phase;
    const running = phase === 'started' || phase === 'analyzing';

    window.__statusBarSampleAnalysisJob = running;
    updateCrateAnalysisButtons(running);

    if (running) {
        const analyzed = Number(payload.analyzed) || 0;
        const total = Number(payload.total) || 0;
        const pct = total > 0 ? Math.round((analyzed / total) * 100) : 0;
        badge.textContent = typeof formatBgJobBadgeLine === 'function'
            ? formatBgJobBadgeLine('sampleAnalysis', 'ui.stats.sample_analysis_progress', {n: analyzed, total, pct})
            : `Analysis: ${analyzed} / ${total} (${pct}%)`;
    } else if (phase === 'completed') {
        const analyzed = Number(payload.analyzed) || 0;
        const total = Number(payload.total) || 0;
        badge.textContent = typeof catalogFmt === 'function'
            ? catalogFmt('ui.stats.sample_analysis_done', {n: analyzed, total})
            : `Analysis done: ${analyzed} / ${total}`;
        setTimeout(() => {
            if (!window.__statusBarSampleAnalysisJob) badge.textContent = '';
            if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
        }, 4000);
    } else {
        badge.textContent = '';
    }

    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
}

// ── Waveform prefetch panel ──
// Start/stop the background job + render progress. When rows land in waveform_cache
// we invalidate the in-memory `_waveformCache[path]` entries for currently-visible
// rows and re-render — so users see the waveforms fill in live, not on next scroll.

async function refreshCratePrefetchStats() {
    if (!window.vstUpdater?.waveformPrefetchStats) return;
    try {
        const stats = await window.vstUpdater.waveformPrefetchStats();
        applyCratePrefetchStats(stats);
    } catch { /* ignore */ }
}

function applyCratePrefetchStats(stats) {
    const cached = Number(stats?.cached) || 0;
    const total = Number(stats?.total) || 0;
    const running = !!stats?.running;
    const statsEl = document.getElementById('cratePrefetchStats');
    const bar = document.getElementById('cratePrefetchBar');
    const startBtn = document.getElementById('cratePrefetchStartBtn');
    const stopBtn = document.getElementById('cratePrefetchStopBtn');
    if (statsEl) {
        const pct = total > 0 ? Math.round((cached / total) * 100) : 0;
        statsEl.textContent = total > 0
            ? `${cached.toLocaleString()} / ${total.toLocaleString()} · ${pct}%`
            : '—';
    }
    if (bar) {
        const pct = total > 0 ? Math.min(100, Math.round((cached / total) * 100)) : 0;
        bar.style.width = `${pct}%`;
    }
    if (startBtn) {
        startBtn.hidden = running || (total > 0 && cached >= total);
        startBtn.disabled = running || total === 0;
    }
    if (stopBtn) stopBtn.hidden = !running;
}

async function startCrateWaveformPrefetch() {
    if (!window.vstUpdater?.waveformPrefetchStart) return;
    try {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_waveform_prefetch_started'), 2000);
        await window.vstUpdater.waveformPrefetchStart();
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_waveform_prefetch_failed', {err}), 4000, 'error');
    }
}

async function stopCrateWaveformPrefetch() {
    if (!window.vstUpdater?.waveformPrefetchStop) return;
    try {
        await window.vstUpdater.waveformPrefetchStop();
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_waveform_prefetch_stopped'), 2000);
    } catch { /* ignore */ }
}

/** Global entry point — called from `triggerStartAllBackgroundJobs` and wherever else
 * a one-shot start is needed. Skips when everything's already cached so re-running
 * "Start all jobs" doesn't kick off a no-op job. */
async function triggerStartWaveformPrefetch() {
    if (!window.vstUpdater?.waveformPrefetchStats) {
        return startCrateWaveformPrefetch();
    }
    try {
        const stats = await window.vstUpdater.waveformPrefetchStats();
        if (stats?.running) return;
        const cached = Number(stats?.cached) || 0;
        const total = Number(stats?.total) || 0;
        if (total > 0 && cached >= total) return; // nothing to do
    } catch { /* fall through to start */ }
    return startCrateWaveformPrefetch();
}
window.triggerStartWaveformPrefetch = triggerStartWaveformPrefetch;

/** Called on every progress event. Updates: sidebar progress panel + status-bar badge
 * + visible row waveforms (rows fill in left-to-right as the bg job walks the library
 * rather than waiting for scroll). */
function onCratePrefetchProgress(payload) {
    if (!payload) return;
    const running = payload.phase === 'started' || payload.phase === 'building';
    applyCratePrefetchStats({
        cached: payload.cached,
        total: payload.total,
        running,
    });
    // Status bar badge — mirrors the fingerprint/sample-analysis pattern.
    updateWaveformPrefetchStatusBadge(payload);
    // Live-fill visible rows. Render helper checks `_waveformCache[path]` first and
    // falls through to `hydrateWaveformPeaksFromSqlite`, so newly-upserted rows
    // materialize here without any extra IPC.
    if (!isCrateTabActive()) return;
    const host = document.getElementById('crateResults');
    if (!host) return;
    const rows = host.querySelectorAll('.crate-row');
    const sample = Math.min(20, rows.length);
    for (let i = 0; i < sample; i++) {
        const row = rows[Math.floor(Math.random() * rows.length)];
        const path = row?.dataset?.samplePath;
        if (!path) continue;
        const canvas = row.querySelector('.crate-row-wave-canvas');
        if (canvas && canvas.dataset.painted === '1') continue;
        renderCrateWaveformFor(row, path).then(() => {
            if (canvas) canvas.dataset.painted = '1';
        }).catch(() => {});
    }
}

/** Status-bar bg-job badge for waveform prefetch. Same pattern as bgFingerprintBadge:
 * set the global flag, write the badge line, trigger syncStatusBgJobRows. */
function updateWaveformPrefetchStatusBadge(payload) {
    const badge = document.getElementById('bgWaveformPrefetchBadge');
    if (!badge) return;
    const running = payload.phase === 'started' || payload.phase === 'building';
    window.__statusBarWaveformPrefetchJob = running;
    if (running) {
        const cached = Number(payload.cached) || 0;
        const total = Number(payload.total) || 0;
        const pct = total > 0 ? Math.round((cached / total) * 100) : 0;
        const line = typeof formatBgJobBadgeLine === 'function'
            ? formatBgJobBadgeLine('waveformPrefetch', 'ui.stats.bg_job_waveform_prefetch_detail', {cached, total, pct})
            : `Waveforms: ${cached} / ${total} (${pct}%)`;
        badge.textContent = line;
    } else {
        // Completed / stopped / error — clear after a short delay so the user sees the final state.
        const cached = Number(payload.cached) || 0;
        const total = Number(payload.total) || 0;
        if (payload.phase === 'completed') {
            const line = typeof catalogFmt === 'function'
                ? catalogFmt('ui.stats.bg_job_waveform_prefetch_done', {cached, total})
                : `Waveforms cached: ${cached} / ${total}`;
            badge.textContent = line;
            setTimeout(() => {
                if (!window.__statusBarWaveformPrefetchJob) badge.textContent = '';
                if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
            }, 4000);
        } else {
            badge.textContent = '';
        }
    }
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    else if (typeof syncStatusBgJobRows === 'function') syncStatusBgJobRows();
}

function renderCrateCategoryTree() {
    const host = document.getElementById('crateCategoryTree');
    if (!host) return;

    const byParent = new Map();
    let totalAll = 0;
    for (const c of _crate.categoryCounts) {
        const p = c.parent_name || '(other)';
        if (!byParent.has(p)) byParent.set(p, []);
        byParent.get(p).push(c);
        totalAll += c.count || 0;
    }

    const parents = [...byParent.keys()].sort();
    const active = _crate.filters.category;

    const parts = [];
    parts.push(`<div class="crate-cat-node ${active === '' ? 'active' : ''}" data-cat="" role="button" tabindex="0">
        <span class="crate-cat-name">All</span>
        <span class="crate-cat-count">${totalAll.toLocaleString()}</span>
    </div>`);

    for (const p of parents) {
        const kids = byParent.get(p);
        const total = kids.reduce((s, k) => s + (k.count || 0), 0);
        const collapsed = _crate.collapsedParents.has(p);
        parts.push(`<div class="crate-cat-parent ${active === p ? 'active' : ''}" data-parent="${_crateEsc(p)}">
            <button type="button" class="crate-cat-toggle" data-parent-toggle="${_crateEsc(p)}" aria-expanded="${!collapsed}">${collapsed ? '▶' : '▼'}</button>
            <span class="crate-cat-name" data-cat="${_crateEsc(p)}" role="button" tabindex="0">${_crateEsc(p)}</span>
            <span class="crate-cat-count">${total.toLocaleString()}</span>
        </div>`);
        if (!collapsed) {
            parts.push(`<div class="crate-cat-children">`);
            kids.sort((a, b) => (b.count || 0) - (a.count || 0));
            for (const k of kids) {
                const cls = active === k.name ? 'active' : '';
                parts.push(`<div class="crate-cat-node crate-cat-child ${cls}" data-cat="${_crateEsc(k.name)}" role="button" tabindex="0">
                    <span class="crate-cat-name">${_crateEsc(k.name)}</span>
                    <span class="crate-cat-count">${(k.count || 0).toLocaleString()}</span>
                </div>`);
            }
            parts.push(`</div>`);
        }
    }

    host.innerHTML = parts.join('');
}

function renderCrateFacets() {
    const packSel = document.getElementById('cratePackSelect');
    const mfrSel = document.getElementById('crateMfrSelect');
    const keySel = document.getElementById('crateKeySelect');
    if (!packSel || !mfrSel || !keySel) return;

    const packs = _crate.facets?.packs || [];
    const mfrs = _crate.facets?.manufacturers || [];
    const keys = _crate.facets?.keys || [];

    packSel.innerHTML = `<option value="">Any pack (${packs.length})</option>` + packs.map(p => {
        const star = p.is_favorite ? '★ ' : '';
        const mfr = p.manufacturer_name ? ` — ${_crateEsc(p.manufacturer_name)}` : '';
        return `<option value="${p.id}">${star}${_crateEsc(p.name)}${mfr} (${p.sample_count.toLocaleString()})</option>`;
    }).join('');
    if (_crate.filters.packId != null) packSel.value = String(_crate.filters.packId);

    mfrSel.innerHTML = `<option value="">Any (${mfrs.length})</option>` + mfrs.map(m =>
        `<option value="${m.id}">${_crateEsc(m.name)} (${m.sample_count.toLocaleString()})</option>`
    ).join('');
    if (_crate.filters.mfrId != null) mfrSel.value = String(_crate.filters.mfrId);

    keySel.innerHTML = `<option value="">Any</option>` + keys.map(k =>
        `<option value="${_crateEsc(k)}">${_crateEsc(k)}</option>`
    ).join('');
    if (_crate.filters.key) keySel.value = _crate.filters.key;
}

function renderCrateFavoritePacks() {
    const host = document.getElementById('crateFavPacks');
    if (!host) return;
    const packs = (_crate.facets?.packs || []).filter(p => p.is_favorite);
    if (packs.length === 0) {
        host.innerHTML = `<div class="crate-fav-empty">Star packs in the Pack dropdown or by pressing <kbd>f</kbd> with a row focused.</div>`;
        return;
    }
    host.innerHTML = packs.map(p => `
        <div class="crate-fav-pack" data-pack-id="${p.id}">
            <span class="crate-fav-star" data-action="crate-fav-toggle" data-pack-id="${p.id}" title="Remove from favorites">★</span>
            <button type="button" class="crate-fav-name" data-action="crate-fav-pick" data-pack-id="${p.id}">${_crateEsc(p.name)}</button>
            <span class="crate-fav-count">${p.sample_count.toLocaleString()}</span>
        </div>
    `).join('');
}

async function runCrateQuery() {
    const host = document.getElementById('crateResults');
    if (!host) return;
    host.classList.add('loading');
    if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('crateSearchInput', true);
    // snake_case matches the Rust CrateQueryParams serde defaults (convention used across this codebase).
    const params = {
        category: _crate.filters.category || null,
        is_loop: _crate.filters.loop === 'loop' ? true : (_crate.filters.loop === 'oneshot' ? false : null),
        pack_id: _crate.filters.packId,
        manufacturer_id: _crate.filters.mfrId,
        bpm_min: _crate.filters.bpmMin,
        bpm_max: _crate.filters.bpmMax,
        key: _crate.filters.key || null,
        favorite_packs_only: _crate.filters.favPacksOnly,
        search: _crate.lastSearch || null,
        search_regex: _crate.lastMode === 'regex',
        limit: _crate.pageSize,
        offset: 0,
    };

    let result;
    try {
        result = await window.vstUpdater.crateQuery(params);
    } catch (err) {
        host.classList.remove('loading');
        if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('crateSearchInput', false);
        host.innerHTML = `<div class="crate-empty">${_crateEsc(toastFmt('toast.crate_query_failed', {err: String(err)}))}</div>`;
        return;
    } finally {
        if (typeof setFilterFieldLoading === 'function') setFilterFieldLoading('crateSearchInput', false);
    }

    _crate.rows = Array.isArray(result?.rows) ? result.rows : [];
    _crate.totalRows = result?.total || 0;
    _crate.navIndex = _crate.rows.length > 0 ? 0 : -1;

    renderCrateResults();
    host.classList.remove('loading');

    const countEl = document.getElementById('crateResultCount');
    if (countEl) {
        countEl.textContent = _crate.totalRows > _crate.rows.length
            ? `${_crate.rows.length.toLocaleString()} of ${_crate.totalRows.toLocaleString()}`
            : `${_crate.totalRows.toLocaleString()}`;
    }

    // Prime the first row's waveform + audition panel (no auto-play on query change).
    if (_crate.navIndex >= 0) {
        setCrateNavIndex(_crate.navIndex, {autoPreview: false});
    } else {
        renderCrateAuditionEmpty();
    }
}

function renderCrateResults() {
    const host = document.getElementById('crateResults');
    if (!host) return;
    if (_crate.rows.length === 0) {
        host.innerHTML = `<div class="crate-empty">No samples match these filters.</div>`;
        return;
    }
    const parts = [];
    for (let i = 0; i < _crate.rows.length; i++) {
        const r = _crate.rows[i];
        parts.push(renderCrateRow(r, i));
    }
    host.innerHTML = parts.join('');

    // Async waveform backfill for visible rows. Uses the same peak pipeline as the
    // Samples tab expanded waveform — cached once per (path, width) via waveform_preview.
    queueCrateVisibleWaveforms();
}

function renderCrateRow(r, idx) {
    const bpm = r.parsed_bpm ? `${r.parsed_bpm} BPM` : '';
    const key = r.parsed_key ? `${_crateEsc(r.parsed_key)}` : '';
    const loopTag = r.is_loop ? '<span class="crate-tag loop">LOOP</span>' : '<span class="crate-tag one">1-SHOT</span>';
    const dur = r.duration && r.duration > 0 ? `${r.duration.toFixed(1)}s` : '';
    const cat = r.category ? `<span class="crate-tag cat">${_crateEsc(r.category)}</span>` : '';
    const pack = r.pack_name ? `<span class="crate-meta-pack" title="${_crateEsc(r.pack_name)}">${_hl(r.pack_name)}</span>` : '';
    const confBar = typeof r.category_confidence === 'number'
        ? `<div class="crate-row-conf" style="width:${Math.round(Math.max(0, Math.min(1, r.category_confidence)) * 100)}%"></div>`
        : '';
    // Highlight matches on name + trailing path segment using the shared highlightMatch helper.
    const nameHl = _hl(r.name);
    const pathHl = _hl(r.path);
    return `<div class="crate-row" data-idx="${idx}" data-sample-id="${r.sample_id}" data-sample-path="${_crateEsc(r.path)}" role="button" tabindex="-1">
        <div class="crate-row-wave" data-wave-path="${_crateEsc(r.path)}">
            <canvas class="crate-row-wave-canvas" width="200" height="40"></canvas>
        </div>
        <div class="crate-row-body">
            <div class="crate-row-name" title="${_crateEsc(r.path)}">${nameHl}</div>
            <div class="crate-row-meta">
                ${loopTag}
                ${cat}
                ${bpm ? `<span class="crate-meta-bpm">${bpm}</span>` : ''}
                ${key ? `<span class="crate-meta-key">${key}</span>` : ''}
                ${dur ? `<span class="crate-meta-dur">${dur}</span>` : ''}
                ${pack}
            </div>
            <div class="crate-row-path" title="${_crateEsc(r.path)}">${pathHl}</div>
            ${confBar}
        </div>
        <div class="crate-row-actions">
            <button type="button" class="crate-row-btn" data-action="crate-similar" data-sample-id="${r.sample_id}" title="Find similar (s)">≈</button>
            ${r.pack_id != null ? `<button type="button" class="crate-row-btn crate-row-star ${_crate.favoritePackIds.has(r.pack_id) ? 'on' : ''}" data-action="crate-fav-toggle" data-pack-id="${r.pack_id}" title="Toggle favorite pack (f)">★</button>` : ''}
        </div>
    </div>`;
}

/** Escape + highlight helper. Uses the shared `highlightMatch` (utils.js) when available so
 * matched chars pick up the same cyan underline as Samples/DAW/Presets tables. Falls through
 * to plain HTML escape when no search is active or the helper isn't loaded yet. */
function _hl(text) {
    if (!text) return '';
    const q = _crate.lastSearch || '';
    if (!q) return _crateEsc(text);
    if (typeof highlightMatch === 'function') {
        try { return highlightMatch(String(text), q, _crate.lastMode || 'fuzzy'); }
        catch { /* fall through */ }
    }
    return _crateEsc(text);
}

// ── Waveform rendering ──
// Piggyback on audio.js's pipeline: `_waveformCache` (in-memory) +
// `hydrateWaveformPeaksFromSqlite` (persistent SQLite cache) +
// `fetchWaveformPreviewFromEngine` (audio-engine). `renderWaveformData` is the
// same draw routine used by the Samples expanded row and the now-playing bar,
// so Crate gets the gradient+min/max envelope rendering for free.

function queueCrateVisibleWaveforms() {
    const host = document.getElementById('crateResults');
    if (!host) return;
    const rows = host.querySelectorAll('.crate-row');
    const concurrency = 4;
    let i = 0;
    const next = () => {
        if (i >= rows.length) return;
        const row = rows[i++];
        const path = row.dataset.samplePath;
        renderCrateWaveformFor(row, path).finally(next);
    };
    for (let k = 0; k < concurrency; k++) next();
}

async function renderCrateWaveformFor(row, path) {
    const canvas = row.querySelector('.crate-row-wave-canvas');
    if (!canvas || !path) return;

    // 1. In-memory cache shared with audio.js — instant if we already rendered it.
    let peaks = (typeof _waveformCache !== 'undefined' && _waveformCache) ? _waveformCache[path] : null;

    // 2. SQLite-backed cache (populated by Samples-tab preview / tray etc.).
    if (!peaks && typeof hydrateWaveformPeaksFromSqlite === 'function') {
        try {
            await hydrateWaveformPeaksFromSqlite(path);
            peaks = _waveformCache ? _waveformCache[path] : null;
        } catch { /* fall through to engine fetch */ }
    }

    // 3. Engine preview (computes + caches). Dedupe so the 4-wide walker doesn't race.
    if (!peaks && typeof fetchWaveformPreviewFromEngine === 'function' && !_crate.waveformInflight.has(path)) {
        _crate.waveformInflight.add(path);
        try {
            peaks = await fetchWaveformPreviewFromEngine(path, canvas.width);
            if (peaks && typeof storeWaveformPeaksInCache === 'function') {
                storeWaveformPeaksInCache(path, peaks);
            }
        } catch { /* noop */ }
        finally {
            _crate.waveformInflight.delete(path);
        }
    }

    if (!peaks || peaks.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    // Match pixel ratio for crisp rendering — same trick as drawWaveform.
    const dpr = window.devicePixelRatio || 1;
    const cssW = 200;
    const cssH = 40;
    if (canvas.width !== Math.round(cssW * dpr) || canvas.height !== Math.round(cssH * dpr)) {
        canvas.width = Math.max(1, Math.round(cssW * dpr));
        canvas.height = Math.max(1, Math.round(cssH * dpr));
        canvas.style.width = `${cssW}px`;
        canvas.style.height = `${cssH}px`;
    }
    if (typeof renderWaveformData === 'function') {
        renderWaveformData(ctx, canvas, peaks);
    }
}

// ── Audition panel ──

function renderCrateAuditionEmpty() {
    const host = document.getElementById('crateAudition');
    if (!host) return;
    host.innerHTML = `<div class="crate-audition-empty">Navigate with j/k to audition. Drag any row into your DAW.</div>`;
}

function renderCrateAuditionFor(row) {
    const host = document.getElementById('crateAudition');
    if (!host || !row) return;
    const bpm = row.parsed_bpm ? `${row.parsed_bpm} BPM` : '—';
    const key = row.parsed_key || '—';
    const cat = row.category || '—';
    const pack = row.pack_name || '—';
    const mfr = row.manufacturer_name || '';
    const dur = row.duration > 0 ? `${row.duration.toFixed(2)}s` : '—';
    const fmt = row.format || '—';
    const kind = row.is_loop ? 'Loop' : 'One-shot';
    host.innerHTML = `
        <div class="crate-audition-path" title="${_crateEsc(row.path)}">${_crateEsc(row.name)}</div>
        <div class="crate-audition-canvas-wrap">
            <canvas id="crateAuditionCanvas" width="360" height="72"></canvas>
        </div>
        <div class="crate-audition-grid">
            <div><span class="crate-ak">Kind</span><span class="crate-av">${kind}</span></div>
            <div><span class="crate-ak">Category</span><span class="crate-av">${_crateEsc(cat)}</span></div>
            <div><span class="crate-ak">BPM</span><span class="crate-av">${_crateEsc(bpm)}</span></div>
            <div><span class="crate-ak">Key</span><span class="crate-av">${_crateEsc(key)}</span></div>
            <div><span class="crate-ak">Duration</span><span class="crate-av">${_crateEsc(dur)}</span></div>
            <div><span class="crate-ak">Format</span><span class="crate-av">${_crateEsc(fmt)}</span></div>
            <div class="crate-audition-pack-row"><span class="crate-ak">Pack</span><span class="crate-av">${_crateEsc(pack)}${mfr ? ` (${_crateEsc(mfr)})` : ''}</span></div>
        </div>
        <div class="crate-audition-actions">
            <button type="button" class="btn-small" data-action="crate-similar" data-sample-id="${row.sample_id}">Similar</button>
            ${row.pack_id != null ? `<button type="button" class="btn-small" data-action="crate-fav-toggle" data-pack-id="${row.pack_id}">${_crate.favoritePackIds.has(row.pack_id) ? 'Unstar pack' : 'Star pack'}</button>` : ''}
        </div>
    `;
    // Paint a bigger waveform on the audition canvas via the same pipeline.
    const bigCanvas = document.getElementById('crateAuditionCanvas');
    if (bigCanvas) {
        void renderCrateAuditionWaveform(bigCanvas, row.path);
    }
}

async function renderCrateAuditionWaveform(canvas, path) {
    if (!canvas || !path) return;
    let peaks = (typeof _waveformCache !== 'undefined' && _waveformCache) ? _waveformCache[path] : null;
    if (!peaks && typeof hydrateWaveformPeaksFromSqlite === 'function') {
        try {
            await hydrateWaveformPeaksFromSqlite(path);
            peaks = _waveformCache ? _waveformCache[path] : null;
        } catch { /* noop */ }
    }
    if (!peaks && typeof fetchWaveformPreviewFromEngine === 'function') {
        try {
            peaks = await fetchWaveformPreviewFromEngine(path, 720);
            if (peaks && typeof storeWaveformPeaksInCache === 'function') storeWaveformPeaksInCache(path, peaks);
        } catch { /* noop */ }
    }
    if (!peaks || peaks.length === 0) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    const cssW = Math.max(240, Math.round(rect.width || 360));
    const cssH = 72;
    canvas.width = Math.max(1, Math.round(cssW * dpr));
    canvas.height = Math.max(1, Math.round(cssH * dpr));
    canvas.style.width = `${cssW}px`;
    canvas.style.height = `${cssH}px`;
    if (typeof renderWaveformData === 'function') renderWaveformData(ctx, canvas, peaks);
}

// ── Navigation + auto-preview ──

function setCrateNavIndex(idx, opts) {
    const options = opts || {};
    const host = document.getElementById('crateResults');
    if (!host) return;
    const rows = [..._crate.rows];
    if (rows.length === 0) return;
    idx = Math.max(0, Math.min(idx, rows.length - 1));
    _crate.navIndex = idx;

    host.querySelectorAll('.crate-row.active').forEach(el => el.classList.remove('active'));
    const rowEl = host.querySelector(`.crate-row[data-idx="${idx}"]`);
    if (rowEl) {
        rowEl.classList.add('active');
        rowEl.scrollIntoView({block: 'nearest', behavior: 'smooth'});
    }

    const row = rows[idx];
    renderCrateAuditionFor(row);

    if (options.autoPreview !== false) {
        scheduleCrateAutoPreview(row.path);
    }
}

function scheduleCrateAutoPreview(path) {
    if (!path || path === _crate.lastAutoPlayPath) return;
    clearTimeout(_crate.autoPlayTimer);
    _crate.autoPlayTimer = setTimeout(() => {
        _crate.lastAutoPlayPath = path;
        if (typeof previewAudio === 'function') {
            previewAudio(path, {minimizeFloatingPlayer: true});
        }
    }, 60);
}

function crateMoveNav(delta) {
    if (_crate.rows.length === 0) return;
    const next = _crate.navIndex < 0 ? 0 : (_crate.navIndex + delta);
    setCrateNavIndex(next, {autoPreview: true});
}

// ── Event wiring ──

function initCrateTab() {
    if (initCrateTab._done) return;
    initCrateTab._done = true;

    // Run-analysis button in the status line (only shown when nothing is analyzed yet).
    document.getElementById('crateLeft')?.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-action="crate-run-analysis"]');
        if (btn) {
            e.preventDefault();
            void runCrateSampleAnalysis();
        }
    });

    // Sample analysis panel buttons.
    document.getElementById('crateAnalysisStartBtn')?.addEventListener('click', () => {
        void runCrateSampleAnalysis();
    });
    document.getElementById('crateAnalysisStopBtn')?.addEventListener('click', () => {
        void stopCrateSampleAnalysis();
    });

    // Waveform prefetch panel buttons.
    document.getElementById('cratePrefetchStartBtn')?.addEventListener('click', () => {
        void startCrateWaveformPrefetch();
    });
    document.getElementById('cratePrefetchStopBtn')?.addEventListener('click', () => {
        void stopCrateWaveformPrefetch();
    });

    // Prime stats + subscribe to progress events (fires regardless of active tab so the
    // panel updates even if user browses elsewhere while the bg job runs).
    void refreshCratePrefetchStats();
    if (window.vstUpdater?.onWaveformPrefetchProgress) {
        window.vstUpdater.onWaveformPrefetchProgress((payload) => onCratePrefetchProgress(payload));
    }

    // Re-seed facets + category counts whenever the sample-analysis job emits progress — keeps
    // the tree live as new rows land, so the user sees categories populate as analysis runs.
    // Also updates the global status bar badge so users know analysis is running.
    if (window.vstUpdater?.onSampleAnalysisProgress) {
        window.vstUpdater.onSampleAnalysisProgress(async (payload) => {
            // Update global status bar badge for any tab
            updateCrateSampleAnalysisBadge(payload);

            if (!isCrateTabActive()) return;
            try {
                const [counts, facets] = await Promise.all([
                    window.vstUpdater.crateCategoryCounts(),
                    window.vstUpdater.crateFacets(),
                ]);
                _crate.categoryCounts = Array.isArray(counts) ? counts : _crate.categoryCounts;
                _crate.facets = facets || _crate.facets;
                renderCrateCategoryTree();
                renderCrateFacets();
                updateCrateAnalyzedStatus();
            } catch { /* ignore */ }
        });
    }

    // Category tree clicks (delegation).
    const tree = document.getElementById('crateCategoryTree');
    tree?.addEventListener('click', (e) => {
        const toggle = e.target.closest('[data-parent-toggle]');
        if (toggle) {
            const p = toggle.dataset.parentToggle;
            if (_crate.collapsedParents.has(p)) _crate.collapsedParents.delete(p);
            else _crate.collapsedParents.add(p);
            renderCrateCategoryTree();
            return;
        }
        const node = e.target.closest('[data-cat]');
        if (!node) return;
        _crate.filters.category = node.dataset.cat;
        renderCrateCategoryTree();
        void runCrateQuery();
    });

    // Loop / oneshot chip group.
    document.querySelectorAll('.crate-chip-group .crate-chip').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.crate-chip-group .crate-chip').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            _crate.filters.loop = btn.dataset.loop || 'any';
            void runCrateQuery();
        });
    });

    const packSel = document.getElementById('cratePackSelect');
    packSel?.addEventListener('change', () => {
        const v = packSel.value;
        _crate.filters.packId = v ? Number(v) : null;
        void runCrateQuery();
    });
    const mfrSel = document.getElementById('crateMfrSelect');
    mfrSel?.addEventListener('change', () => {
        const v = mfrSel.value;
        _crate.filters.mfrId = v ? Number(v) : null;
        void runCrateQuery();
    });
    const keySel = document.getElementById('crateKeySelect');
    keySel?.addEventListener('change', () => {
        _crate.filters.key = keySel.value || '';
        void runCrateQuery();
    });
    document.getElementById('crateBpmMin')?.addEventListener('input', onCrateBpmInput);
    document.getElementById('crateBpmMax')?.addEventListener('input', onCrateBpmInput);
    document.getElementById('crateFavPacksOnly')?.addEventListener('change', (e) => {
        _crate.filters.favPacksOnly = !!e.target.checked;
        void runCrateQuery();
    });

    document.getElementById('crateClearFilters')?.addEventListener('click', () => {
        _crate.filters = {
            category: '', loop: 'any', packId: null, mfrId: null,
            bpmMin: null, bpmMax: null, key: '', favPacksOnly: false,
        };
        _crate.lastSearch = '';
        _crate.lastMode = 'fuzzy';
        const s = document.getElementById('crateSearchInput');
        if (s) s.value = '';
        const bl = document.getElementById('crateBpmMin');
        const bh = document.getElementById('crateBpmMax');
        if (bl) bl.value = '';
        if (bh) bh.value = '';
        const fp = document.getElementById('crateFavPacksOnly');
        if (fp) fp.checked = false;
        document.querySelectorAll('.crate-chip-group .crate-chip').forEach(b => {
            b.classList.toggle('active', b.dataset.loop === 'any');
        });
        renderCrateFacets();
        renderCrateCategoryTree();
        void runCrateQuery();
    });

    // Search input is wired through `data-action="filterCrate"` + `registerFilter` (see top of file).
    // The search-box dispatcher in ipc.js routes input → applyFilterDebounced → our fetchFn.

    // Results list delegation: click, dblclick, action buttons.
    const results = document.getElementById('crateResults');
    results?.addEventListener('click', async (e) => {
        const actionBtn = e.target.closest('[data-action]');
        if (actionBtn) {
            const action = actionBtn.dataset.action;
            if (action === 'crate-fav-toggle') {
                e.stopPropagation();
                const pid = Number(actionBtn.dataset.packId);
                if (Number.isFinite(pid)) await toggleCrateFavoritePack(pid);
                return;
            }
            if (action === 'crate-similar') {
                e.stopPropagation();
                const sid = Number(actionBtn.dataset.sampleId);
                if (Number.isFinite(sid)) await runCrateSimilar(sid);
                return;
            }
        }
        const row = e.target.closest('.crate-row');
        if (!row) return;
        const idx = Number(row.dataset.idx);
        if (Number.isFinite(idx)) setCrateNavIndex(idx, {autoPreview: true});
    });

    // Keyboard: j/k/arrows/space/enter/f/s/esc/slash when Crate is active.
    // Capture-phase listener so we beat keyboard-nav.js's document-level bubble listener
    // (that one is a no-op on tabCrate anyway, but defensive coupling is cheap here).
    document.addEventListener('keydown', onCrateKeydown, true);
    document.addEventListener('keydown', onCrateGlobalKeydown);

    // Favorite packs side panel.
    document.getElementById('crateFavPacks')?.addEventListener('click', async (e) => {
        const actionBtn = e.target.closest('[data-action]');
        if (!actionBtn) return;
        const pid = Number(actionBtn.dataset.packId);
        if (!Number.isFinite(pid)) return;
        if (actionBtn.dataset.action === 'crate-fav-toggle') {
            await toggleCrateFavoritePack(pid);
        } else if (actionBtn.dataset.action === 'crate-fav-pick') {
            _crate.filters.packId = pid;
            const sel = document.getElementById('cratePackSelect');
            if (sel) sel.value = String(pid);
            await runCrateQuery();
        }
    });

    // Audition panel action buttons (Similar / Star).
    document.getElementById('crateAudition')?.addEventListener('click', async (e) => {
        const actionBtn = e.target.closest('[data-action]');
        if (!actionBtn) return;
        if (actionBtn.dataset.action === 'crate-similar') {
            const sid = Number(actionBtn.dataset.sampleId);
            if (Number.isFinite(sid)) await runCrateSimilar(sid);
        } else if (actionBtn.dataset.action === 'crate-fav-toggle') {
            const pid = Number(actionBtn.dataset.packId);
            if (Number.isFinite(pid)) await toggleCrateFavoritePack(pid);
        }
    });
}

function onCrateBpmInput() {
    const lo = document.getElementById('crateBpmMin')?.value;
    const hi = document.getElementById('crateBpmMax')?.value;
    _crate.filters.bpmMin = lo ? Number(lo) : null;
    _crate.filters.bpmMax = hi ? Number(hi) : null;
    clearTimeout(_crate.bpmTimer);
    _crate.bpmTimer = setTimeout(() => {
        void runCrateQuery();
    }, 180);
}

function onCrateKeydown(e) {
    if (!isCrateTabActive()) return;
    // Don't hijack typing in inputs / selects.
    const tag = (e.target.tagName || '').toUpperCase();
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || e.target.isContentEditable) return;
    // Respect Cmd/Ctrl modifiers (command palette, volume, etc. belong to shortcuts.js).
    if (e.metaKey || e.ctrlKey) return;
    const key = e.key;
    if (key === 'j' || key === 'ArrowDown') { e.preventDefault(); crateMoveNav(+1); return; }
    if (key === 'k' || key === 'ArrowUp')   { e.preventDefault(); crateMoveNav(-1); return; }
    if (key === 'g' && !e.shiftKey)         { e.preventDefault(); setCrateNavIndex(0, {autoPreview: true}); return; }
    if (key === 'G' || (key === 'g' && e.shiftKey)) { e.preventDefault(); setCrateNavIndex(_crate.rows.length - 1, {autoPreview: true}); return; }
    if (key === 'Enter') {
        e.preventDefault();
        const row = _crate.rows[_crate.navIndex];
        if (row && typeof previewAudio === 'function') {
            // Persist play: clear the dedupe so a repeat-play counts as a new start.
            _crate.lastAutoPlayPath = null;
            previewAudio(row.path, {minimizeFloatingPlayer: false});
        }
        return;
    }
    if (key === ' ' || key === 'Spacebar') {
        e.preventDefault();
        const row = _crate.rows[_crate.navIndex];
        if (row && typeof previewAudio === 'function') previewAudio(row.path, {minimizeFloatingPlayer: true});
        return;
    }
    if (key === 'f') {
        e.preventDefault();
        const row = _crate.rows[_crate.navIndex];
        if (row && row.pack_id != null) {
            void toggleCrateFavoritePack(row.pack_id);
        }
        return;
    }
    if (key === 'Escape') {
        e.preventDefault();
        if (typeof isAudioPlaying === 'function' && isAudioPlaying() && typeof previewAudio === 'function') {
            const row = _crate.rows[_crate.navIndex];
            if (row) previewAudio(row.path, {minimizeFloatingPlayer: true}); // toggles to pause
        }
        return;
    }
}

/** `/` focuses the search input even when focus is on the results list. */
function onCrateGlobalKeydown(e) {
    if (!isCrateTabActive()) return;
    if (e.key !== '/') return;
    const tag = (e.target.tagName || '').toUpperCase();
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || e.target.isContentEditable) return;
    e.preventDefault();
    const s = document.getElementById('crateSearchInput');
    if (s) { s.focus(); s.select(); }
}

function isCrateTabActive() {
    return document.querySelector('.tab-content.active')?.id === 'tabCrate';
}

async function toggleCrateFavoritePack(packId) {
    try {
        const nowOn = await window.vstUpdater.crateFavoritePackToggle(packId);
        if (nowOn) _crate.favoritePackIds.add(packId);
        else _crate.favoritePackIds.delete(packId);
        // Update facet cache without a round trip.
        const p = (_crate.facets?.packs || []).find(x => x.id === packId);
        if (p) p.is_favorite = nowOn;
        renderCrateFavoritePacks();
        renderCrateFacets();
        // Update row stars for any rows showing this pack.
        document.querySelectorAll(`.crate-row .crate-row-star[data-pack-id="${packId}"]`).forEach(el => {
            el.classList.toggle('on', nowOn);
        });
        if (typeof showToast === 'function') {
            showToast(toastFmt(nowOn ? 'toast.crate_pack_favorited' : 'toast.crate_pack_unfavorited'), 1500);
        }
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_favorite_failed', {err}), 4000, 'error');
    }
}

async function runCrateSimilar(sampleId) {
    const row = _crate.rows.find(r => r.sample_id === sampleId);
    // If row not found in current view, try to get path from the DOM element
    if (!row) {
        const el = document.querySelector(`.crate-row[data-sample-id="${sampleId}"]`);
        if (el) {
            const path = el.dataset.samplePath;
            const name = el.querySelector('.crate-row-name')?.textContent || '';
            if (path) {
                // Fall back to global findSimilarSamples with path
                if (typeof findSimilarSamples === 'function') {
                    findSimilarSamples(path);
                }
                return;
            }
        }
        return;
    }
    if (!row) return;
    if (typeof showToast === 'function') showToast(toastFmt('toast.crate_finding_similar', {name: row.name}), 2000);
    try {
        const candidates = await window.vstUpdater.crateSimilarCandidates(sampleId, 500);
        if (!Array.isArray(candidates) || candidates.length === 0) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.crate_no_similar_candidates'), 2500);
            return;
        }
        const ranked = await window.vstUpdater.findSimilarSamples(row.path, candidates, 30);
        if (!Array.isArray(ranked) || ranked.length === 0) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.crate_no_similar_scored'), 2500);
            return;
        }
        // Narrow the Crate view to exactly these paths.
        const pathSet = new Set(ranked.map(r => r.path).filter(Boolean));
        _crate.rows = _crate.rows.filter(r => pathSet.has(r.path));
        // If the filter dropped many, keep them in score order.
        const scoreByPath = new Map(ranked.map(r => [r.path, r.score ?? r.distance ?? 0]));
        _crate.rows.sort((a, b) => (scoreByPath.get(a.path) || 0) - (scoreByPath.get(b.path) || 0));
        _crate.totalRows = _crate.rows.length;
        renderCrateResults();
        const countEl = document.getElementById('crateResultCount');
        if (countEl) countEl.textContent = `${_crate.rows.length} similar`;
        setCrateNavIndex(0, {autoPreview: true});
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.crate_similarity_failed', {err}), 4000, 'error');
    }
}

function _crateEsc(s) {
    if (s == null) return '';
    return String(s)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

// ── Exports for context-menu.js and ipc.js menu-action handler ──
window.runCrateSimilar = runCrateSimilar;
window.toggleCrateFavoritePack = toggleCrateFavoritePack;
window.startCrateWaveformPrefetch = startCrateWaveformPrefetch;
window.stopCrateWaveformPrefetch = stopCrateWaveformPrefetch;

// ── Boot ──
if (typeof document !== 'undefined') {
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initCrateTab);
    } else {
        initCrateTab();
    }
}
