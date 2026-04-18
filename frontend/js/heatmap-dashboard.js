// ── Audio Heatmap Dashboard ──
// Full-screen analytics overview: format distribution, size breakdown,
// folder heatmap, BPM histogram, key wheel, activity timeline.

function _hmFmt(key, vars) {
    return catalogFmt(key, vars);
}

/** Parallel DB aggregates (whole library, no search/filter) for correct counts when tabs are paginated. */
async function fetchHeatmapDbAggregates() {
    const vu = typeof window !== 'undefined' ? window.vstUpdater : null;
    if (!vu || typeof vu.dbAudioFilterStats !== 'function') return null;
    try {
        const [audio, plugins, daw, presets] = await Promise.all([
            vu.dbAudioFilterStats(null, null, false),
            vu.dbPluginFilterStats(null, null, false),
            vu.dbDawFilterStats(null, null, false),
            vu.dbPresetFilterStats(null, null, false),
        ]);
        return {audio, plugins, daw, presets};
    } catch (e) {
        if (typeof console !== 'undefined' && console.warn) console.warn('heatmap DB aggregates', e);
        return null;
    }
}

function _hmOverviewTotals(agg, samples, plugins, projects, presets) {
    if (agg) {
        const sz = agg.audio?.totalBytes ?? 0;
        return {
            nSamples: Number(agg.audio?.count) || 0,
            nPlugins: Number(agg.plugins?.count) || 0,
            nDaw: Number(agg.daw?.count) || 0,
            nPresets: Number(agg.presets?.count) || 0,
            totalBytes: typeof sz === 'bigint' ? Number(sz) : sz,
        };
    }
    const ns = Number(typeof audioTotalCount !== 'undefined' ? audioTotalCount : 0)
        || Number(typeof audioTotalUnfiltered !== 'undefined' ? audioTotalUnfiltered : 0)
        || samples.length;
    const np = Number(typeof _pluginTotalCount !== 'undefined' ? _pluginTotalCount : 0)
        || Number(typeof _pluginTotalUnfiltered !== 'undefined' ? _pluginTotalUnfiltered : 0)
        || plugins.length;
    const nd = Number(typeof _dawTotalCount !== 'undefined' ? _dawTotalCount : 0)
        || Number(typeof _dawTotalUnfiltered !== 'undefined' ? _dawTotalUnfiltered : 0)
        || projects.length;
    const npr = Number(typeof _presetTotalCount !== 'undefined' ? _presetTotalCount : 0)
        || Number(typeof _presetTotalUnfiltered !== 'undefined' ? _presetTotalUnfiltered : 0)
        || presets.length;
    let totalBytes = 0;
    if (typeof audioStatBytes !== 'undefined' && audioStatBytes > 0) {
        totalBytes = audioStatBytes;
    } else {
        totalBytes = samples.reduce((s, a) => s + (a.size || a.sizeBytes || 0), 0);
    }
    return {nSamples: ns, nPlugins: np, nDaw: nd, nPresets: npr, totalBytes};
}

function _hmPartialSampleHintCard(agg, samples) {
    const lib = agg?.audio?.count ?? (Number(typeof audioTotalCount !== 'undefined' ? audioTotalCount : 0) || 0);
    if (!lib || samples.length >= lib) return '';
    // `allAudioSamples` is often still empty on first open (Samples tab not hydrated yet). DB-backed
    // cards can still show full-library stats — avoid a bogus "0 of N" that clears on second open.
    if (samples.length === 0) return '';
    return `<div class="hm-card hm-card-wide" data-hm-card="partialHint" style="padding:8px 12px;margin-bottom:8px;border:1px dashed var(--text-dim);">
  <span style="font-size:11px;color:var(--text-muted);">${escapeHtml(_hmFmt('ui.hm.partial_sample_rows', {shown: samples.length.toLocaleString(), total: lib.toLocaleString()}))}</span>
</div>`;
}

function showHeatmapDashboard() {
    // Remove every overlay (invalid duplicate ids can leave multiple nodes; getElementById only removes one).
    document.querySelectorAll('#heatmapDashModal').forEach((el) => el.remove());

    const samples = typeof allAudioSamples !== 'undefined' ? allAudioSamples : [];
    const plugins = typeof allPlugins !== 'undefined' ? allPlugins : [];
    const projects = typeof allDawProjects !== 'undefined' ? allDawProjects : [];
    const presets = typeof allPresets !== 'undefined' ? allPresets : [];

    const html = `<div class="modal-overlay modal-visible" id="heatmapDashModal" data-action-modal="closeHeatmapDash">
    <div class="modal-content modal-wide" style="max-width:95vw;width:95vw;max-height:95vh;height:95vh;">
      <div class="modal-header">
        <h2>${escapeHtml(_hmFmt('ui.hm.title'))}</h2>
        <button class="modal-close" data-action-modal="closeHeatmapDash" title="${escapeHtml(_hmFmt('ui.hm.close'))}">&#10005;</button>
      </div>
      <div class="modal-body" style="overflow-y:auto;max-height:calc(90vh - 60px);">
        <div class="hm-overview" id="hmOverview"><div style="display:flex;align-items:center;justify-content:center;padding:24px;"><div class="spinner"></div></div></div>
        <div class="hm-grid" id="hmGrid"></div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);
    const root = document.getElementById('heatmapDashModal');
    if (!root) return;

    // DB aggregates were blocking modal insert; fetch after paint so the shell appears immediately.
    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            void (async () => {
                let agg = null;
                try {
                    agg = await fetchHeatmapDbAggregates();
                } catch (e) {
                    if (typeof console !== 'undefined' && console.warn) console.warn('heatmap DB aggregates', e);
                }
                if (!document.body.contains(root)) return;
                renderDashboard(root, samples, plugins, projects, presets, agg);
            })();
        });
    });
}

function closeHeatmapDash() {
    document.querySelectorAll('#heatmapDashModal').forEach((el) => el.remove());
}

function renderDashboard(root, samples, plugins, projects, presets, agg) {
    const overview = root.querySelector('.hm-overview');
    const grid = root.querySelector('.hm-grid');
    if (!overview || !grid) return;

    const t = _hmOverviewTotals(agg, samples, plugins, projects, presets);
    const totalSize = Number(t.totalBytes) || 0;

    // Overview stats
    overview.innerHTML = `
    <div class="hm-stat"><span class="hm-stat-val">${t.nSamples.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_samples'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${t.nPlugins.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_plugins'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${t.nDaw.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_daw'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${t.nPresets.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_presets'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${typeof formatAudioSize === 'function' ? formatAudioSize(totalSize) : (totalSize / (1024 * 1024 * 1024)).toFixed(1) + ' GB'}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_total_size'))}</span></div>
  `;

    let cards = _hmPartialSampleHintCard(agg, samples);

    // 1. Format distribution (pie-like horizontal bars)
    cards += buildFormatCard(samples, agg);

    // 2. Size distribution histogram
    cards += buildSizeCard(samples, agg);

    // 3. Folder heatmap (top directories by file count)
    cards += buildFolderCard(samples, agg);

    // 4. BPM distribution (DB aggregates; fallback to in-memory cache)
    cards += buildBpmCard(agg);

    // 5. Key distribution (DB aggregates; fallback to in-memory cache)
    cards += buildKeyCard(agg);

    // 6. Activity timeline (files by modified month)
    cards += buildTimelineCard(samples);

    // 7. Plugin type breakdown
    cards += buildPluginTypeCard(plugins, agg);

    // 8. DAW format breakdown
    cards += buildDawFormatCard(projects, agg);

    grid.innerHTML = cards;

    // Make dashboard cards draggable to reorder
    if (typeof initDragReorder === 'function') {
        initDragReorder(grid, '.hm-card', 'hmCardOrder', {
            getKey: (el) => el.dataset.hmCard || '',
        });
    }

    // Render canvases after DOM insertion (scoped to this modal — getElementById would hit the wrong overlay if ids ever duplicated)
    renderBpmHistogram(root, agg);
    renderKeyWheel(root, agg);
    renderTimelineChart(root, samples);

    // Apply bar widths after layout resolves (flex containers need a frame to get correct widths)
    requestAnimationFrame(() => {
        grid.querySelectorAll('[data-bar-pct]').forEach(el => {
            el.style.width = el.dataset.barPct + '%';
            el.style.transition = 'width 0.3s ease-out';
        });
    });
}

// ── Card Builders ──

function buildFormatCard(samples, agg) {
    let counts = {};
    if (agg?.audio?.byType && Object.keys(agg.audio.byType).length > 0) {
        counts = {...agg.audio.byType};
    } else if (typeof audioStatCounts !== 'undefined' && Object.keys(audioStatCounts).length > 0) {
        counts = {...audioStatCounts};
    } else {
        for (const s of samples) counts[s.format] = (counts[s.format] || 0) + 1;
    }
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    const max = sorted.length > 0 ? sorted[0][1] : 1;
    const total = sorted.reduce((sum, [, c]) => sum + c, 0) || 1;

    const bars = sorted.slice(0, 10).map(([fmt, count]) => {
        const barPct = (count / max) * 100;
        const share = ((count / total) * 100).toFixed(1);
        return `<div class="hm-bar-row">
      <span class="hm-bar-label">${escapeHtml(fmt)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-cyan" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${count.toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');

    return `<div class="hm-card" data-hm-card="format"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_format_dist'))}</h3>${bars || `<span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.empty_no_samples'))}</span>`}</div>`;
}

function buildSizeCard(samples, agg) {
    const buckets = [
        {labelKey: 'ui.hm.bucket_lt_100kb', max: 100 * 1024},
        {labelKey: 'ui.hm.bucket_100kb_1mb', max: 1024 * 1024},
        {labelKey: 'ui.hm.bucket_1_10mb', max: 10 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_10_50mb', max: 50 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_50_100mb', max: 100 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_gt_100mb', max: Infinity},
    ];
    const counts = new Array(buckets.length).fill(0);
    const sb = agg?.audio?.sizeBuckets;
    if (Array.isArray(sb) && sb.length === buckets.length) {
        for (let i = 0; i < buckets.length; i++) counts[i] = Number(sb[i]) || 0;
    } else {
        for (const s of samples) {
            const sz = s.size || s.sizeBytes || 0;
            for (let i = 0; i < buckets.length; i++) {
                if (sz < buckets[i].max || i === buckets.length - 1) {
                    counts[i]++;
                    break;
                }
            }
        }
    }
    const libCount = agg?.audio?.count;
    const pageTotal = (typeof libCount === 'number' && !Number.isNaN(libCount) && libCount > 0)
        ? libCount
        : (samples.length || 1);
    const maxBucket = Math.max(...counts, 1);
    const bars = buckets.map((b, i) => {
        const barPct = (counts[i] / maxBucket) * 100;
        const share = ((counts[i] / pageTotal) * 100).toFixed(1);
        const bl = _hmFmt(b.labelKey);
        return `<div class="hm-bar-row">
      <span class="hm-bar-label">${escapeHtml(bl)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-magenta" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${counts[i].toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');

    return `<div class="hm-card" data-hm-card="size"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_size_dist'))}</h3>${bars}</div>`;
}

function buildFolderCard(samples, agg) {
    const libCount = agg?.audio?.count;
    /** `topFolders` is always `[]` or rows from SQLite (never omitted) so we do not fall back to paginated `samples` when the DB list is empty. */
    const useDbFolders = agg?.audio != null && Array.isArray(agg.audio.topFolders);
    let sorted;
    let pageTotal;
    if (useDbFolders) {
        sorted = agg.audio.topFolders.map((r) => [r.path, Number(r.count) || 0]);
        pageTotal = (typeof libCount === 'number' && !Number.isNaN(libCount) && libCount > 0)
            ? libCount
            : sorted.reduce((s, [, c]) => s + c, 0) || 1;
    } else {
        const dirs = {};
        for (const s of samples) {
            const dir = s.directory || s.path?.replace(/\/[^/]+$/, '') || _hmFmt('ui.hm.unknown');
            const parts = dir.split(/[/\\]/).filter(Boolean);
            const key = '/' + parts.slice(0, Math.min(parts.length, 3)).join('/');
            dirs[key] = (dirs[key] || 0) + 1;
        }
        sorted = Object.entries(dirs).sort((a, b) => b[1] - a[1]).slice(0, 12);
        pageTotal = (typeof libCount === 'number' && !Number.isNaN(libCount) && libCount > 0)
            ? libCount
            : (samples.length || 1);
    }
    const maxFolder = sorted.length > 0 ? sorted[0][1] : 1;

    const bars = sorted.map(([dir, count]) => {
        const barPct = (count / maxFolder) * 100;
        const share = ((count / pageTotal) * 100).toFixed(1);
        const name = dir.split('/').pop() || dir;
        return `<div class="hm-bar-row" title="${escapeHtml(dir)}">
      <span class="hm-bar-label" style="max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(name)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-green" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${count.toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');

    return `<div class="hm-card" data-hm-card="folders"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_top_folders'))}</h3>${bars || `<span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.empty_no_data'))}</span>`}</div>`;
}

function buildBpmCard(agg) {
    const a = agg?.audio;
    const dbCount = a && typeof a.bpmAnalyzedCount === 'number' ? a.bpmAnalyzedCount : 0;
    const dbBuckets = a && Array.isArray(a.bpmBuckets) && a.bpmBuckets.length === 34;
    const bpms = typeof _bpmCache !== 'undefined' ? Object.values(_bpmCache).filter(v => v && v > 0) : [];
    const hasDb = dbBuckets && (dbCount > 0 || (a.bpmBuckets || []).some((x) => Number(x) > 0));
    const hasCache = bpms.length > 0;
    if (!hasDb && !hasCache) {
        return `<div class="hm-card" data-hm-card="bpm"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_bpm_dist'))}</h3><span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.card_bpm_empty'))}</span></div>`;
    }
    const n = hasDb ? dbCount : bpms.length;
    return `<div class="hm-card" data-hm-card="bpm"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_bpm_title_analyzed', {n}))}</h3><canvas id="hmBpmCanvas" width="400" height="120" style="width:100%;height:120px;" title="${escapeHtml(_hmFmt('ui.hm.card_bpm_canvas_title'))}"></canvas></div>`;
}

function buildKeyCard(agg) {
    const a = agg?.audio;
    const dbCount = a && typeof a.keyAnalyzedCount === 'number' ? a.keyAnalyzedCount : 0;
    const dbKeys = a && a.keyCounts && typeof a.keyCounts === 'object' && Object.keys(a.keyCounts).length > 0;
    const keys = typeof _keyCache !== 'undefined' ? Object.values(_keyCache).filter(Boolean) : [];
    const hasDb = dbKeys && dbCount > 0;
    const hasCache = keys.length > 0;
    if (!hasDb && !hasCache) {
        return `<div class="hm-card" data-hm-card="key"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_key_dist'))}</h3><span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.card_key_empty'))}</span></div>`;
    }
    const n = hasDb ? dbCount : keys.length;
    return `<div class="hm-card" data-hm-card="key"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_key_title_analyzed', {n}))}</h3><canvas id="hmKeyCanvas" width="400" height="200" style="width:100%;height:200px;" title="${escapeHtml(_hmFmt('ui.hm.card_key_canvas_title'))}"></canvas></div>`;
}

function buildTimelineCard(samples) {
    if (samples.length === 0) return '';
    let hasMod = false;
    for (const s of samples) {
        if (s.modified && String(s.modified).length >= 7) {
            hasMod = true;
            break;
        }
    }
    if (!hasMod) return '';
    return `<div class="hm-card hm-card-wide" data-hm-card="timeline"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_timeline'))}</h3><canvas id="hmTimelineCanvas" width="800" height="100" style="width:100%;height:100px;" title="${escapeHtml(_hmFmt('ui.hm.card_timeline_canvas_title'))}"></canvas></div>`;
}

function buildPluginTypeCard(plugins, agg) {
    const types = {};
    if (agg?.plugins?.byType && Object.keys(agg.plugins.byType).length > 0) {
        Object.assign(types, agg.plugins.byType);
    } else if (plugins.length > 0) {
        for (const p of plugins) types[p.type || _hmFmt('ui.hm.unknown')] = (types[p.type || _hmFmt('ui.hm.unknown')] || 0) + 1;
    }
    if (Object.keys(types).length === 0) return '';
    const sorted = Object.entries(types).sort((a, b) => b[1] - a[1]);
    const total = Object.values(types).reduce((a, b) => a + b, 0) || 1;
    const maxType = sorted[0]?.[1] || 1;
    const bars = sorted.map(([type, count]) => {
        const barPct = (count / maxType) * 100;
        const share = ((count / total) * 100).toFixed(1);
        return `<div class="hm-bar-row">
      <span class="hm-bar-label">${escapeHtml(type)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-yellow" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${count.toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');
    return `<div class="hm-card" data-hm-card="pluginTypes"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_plugin_types'))}</h3>${bars}</div>`;
}

function buildDawFormatCard(projects, agg) {
    const fmts = {};
    if (agg?.daw?.byType && Object.keys(agg.daw.byType).length > 0) {
        Object.assign(fmts, agg.daw.byType);
    } else if (typeof _dawStatsSnapshot !== 'undefined' && _dawStatsSnapshot && _dawStatsSnapshot.counts && Object.keys(_dawStatsSnapshot.counts).length > 0) {
        Object.assign(fmts, _dawStatsSnapshot.counts);
    } else if (typeof dawStatCounts !== 'undefined' && dawStatCounts && Object.keys(dawStatCounts).length > 0) {
        Object.assign(fmts, dawStatCounts);
    } else if (projects.length > 0) {
        for (const p of projects) {
            const fmt = p.daw || p.format || _hmFmt('ui.hm.unknown');
            fmts[fmt] = (fmts[fmt] || 0) + 1;
        }
    }
    if (Object.keys(fmts).length === 0) return '';
    const sorted = Object.entries(fmts).sort((a, b) => b[1] - a[1]);
    const total = sorted.reduce((sum, [, c]) => sum + c, 0) || 1;
    const maxFmt = sorted[0]?.[1] || 1;
    const bars = sorted.map(([fmt, count]) => {
        const barPct = (count / maxFmt) * 100;
        const share = ((count / total) * 100).toFixed(1);
        return `<div class="hm-bar-row">
      <span class="hm-bar-label">${escapeHtml(fmt)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-orange" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${count.toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');
    return `<div class="hm-card" data-hm-card="dawFormats"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_daw_formats'))}</h3>${bars}</div>`;
}

// ── Canvas Renderers ──

function renderBpmHistogram(root, agg) {
    const canvas = root.querySelector('#hmBpmCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const minBpm = 50, maxBpm = 220, binWidth = 5;
    const numBins = Math.ceil((maxBpm - minBpm) / binWidth);
    const a = agg?.audio;
    let bins;
    if (a && Array.isArray(a.bpmBuckets) && a.bpmBuckets.length === numBins) {
        bins = a.bpmBuckets.map((x) => Number(x) || 0);
    } else {
        const bpms = typeof _bpmCache !== 'undefined' ? Object.values(_bpmCache).filter(v => v && v > 0) : [];
        if (bpms.length === 0) return;
        bins = new Array(numBins).fill(0);
        for (const bpm of bpms) {
            const idx = Math.floor((bpm - minBpm) / binWidth);
            if (idx >= 0 && idx < numBins) bins[idx]++;
        }
    }
    const maxCount = Math.max(...bins, 1);

    const barW = w / numBins;
    for (let i = 0; i < numBins; i++) {
        const barH = (bins[i] / maxCount) * (h - 20);
        const x = i * barW;
        const y = h - 15 - barH;
        const intensity = bins[i] / maxCount;
        const r = Math.floor(5 + intensity * 206);
        const g = Math.floor(217 - intensity * 167);
        const b = Math.floor(232 - intensity * 35);
        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(x + 1, y, barW - 2, barH);
    }

    // Axis labels
    ctx.fillStyle = 'rgba(122,139,168,0.8)';
    ctx.font = '9px sans-serif';
    ctx.textAlign = 'center';
    for (let bpm = 60; bpm <= 200; bpm += 20) {
        const x = ((bpm - minBpm) / (maxBpm - minBpm)) * w;
        ctx.fillText(bpm.toString(), x, h - 2);
    }
}

function renderKeyWheel(root, agg) {
    const canvas = root.querySelector('#hmKeyCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const a = agg?.audio;
    let sorted;
    let keyTotal;
    if (a && a.keyCounts && typeof a.keyCounts === 'object' && Object.keys(a.keyCounts).length > 0) {
        sorted = Object.entries(a.keyCounts)
            .map(([k, c]) => [k, Number(c) || 0])
            .sort((x, y) => y[1] - x[1]);
        keyTotal = typeof a.keyAnalyzedCount === 'number' && a.keyAnalyzedCount > 0
            ? a.keyAnalyzedCount
            : sorted.reduce((s, [, c]) => s + c, 0);
    } else {
        const keys = typeof _keyCache !== 'undefined' ? Object.values(_keyCache).filter(Boolean) : [];
        if (keys.length === 0) return;
        const counts = {};
        for (const k of keys) counts[k] = (counts[k] || 0) + 1;
        sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
        keyTotal = keys.length || 1;
    }
    const maxCount = sorted[0]?.[1] || 1;

    // Draw as horizontal bars (cleaner than wheel for small datasets)
    const barH = Math.min(16, (h - 10) / sorted.length);
    for (let i = 0; i < sorted.length && i < 12; i++) {
        const [key, count] = sorted[i];
        const pct = count / maxCount;
        const y = i * (barH + 2) + 5;
        const barW = pct * (w - 120);

        // Color: major = cyan tones, minor = magenta tones
        const isMinor = key.includes('Minor');
        if (isMinor) {
            ctx.fillStyle = `rgba(211,0,197,${0.3 + pct * 0.7})`;
        } else {
            ctx.fillStyle = `rgba(5,217,232,${0.3 + pct * 0.7})`;
        }
        ctx.fillRect(80, y, barW, barH);

        // Label
        ctx.fillStyle = 'rgba(224,240,255,0.9)';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'right';
        ctx.fillText(key, 75, y + barH - 3);

        // Count + percentage (of all library rows with key, or in-memory list length)
        const keyShare = ((count / Math.max(keyTotal, 1)) * 100).toFixed(0);
        ctx.textAlign = 'left';
        ctx.fillStyle = 'rgba(122,139,168,0.8)';
        ctx.fillText(`${count} (${keyShare}%)`, 82 + barW + 4, y + barH - 3);
    }
}

function renderTimelineChart(root, samples) {
    const canvas = root.querySelector('#hmTimelineCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Group by month
    const months = {};
    for (const s of samples) {
        if (!s.modified) continue;
        const m = s.modified.slice(0, 7); // "YYYY-MM"
        if (m.length === 7 && m[4] === '-') months[m] = (months[m] || 0) + 1;
    }
    const sorted = Object.entries(months).sort((a, b) => a[0].localeCompare(b[0]));
    if (sorted.length === 0) return;

    // Take last 24 months max
    const recent = sorted.slice(-24);
    const maxCount = Math.max(...recent.map(m => m[1]), 1);
    const barW = w / recent.length;

    for (let i = 0; i < recent.length; i++) {
        const [month, count] = recent[i];
        const barH = (count / maxCount) * (h - 20);
        const x = i * barW;
        const y = h - 15 - barH;
        const intensity = count / maxCount;
        ctx.fillStyle = `rgba(57,255,20,${0.2 + intensity * 0.8})`;
        ctx.fillRect(x + 1, y, barW - 2, barH);
    }

    // X-axis labels (every 3 months)
    ctx.fillStyle = 'rgba(122,139,168,0.8)';
    ctx.font = '8px sans-serif';
    ctx.textAlign = 'center';
    const labelStep = Math.max(1, Math.ceil(recent.length / 8));
    for (let i = 0; i < recent.length; i += labelStep) {
        const label = recent[i][0].slice(2); // "YY-MM"
        ctx.fillText(label, i * barW + barW / 2, h - 2);
    }
}

// ── Event Handlers ──

document.addEventListener('click', (e) => {
    const close = e.target.closest('[data-action-modal="closeHeatmapDash"]');
    if (close) {
        if (e.target === close || close.classList.contains('modal-close')) {
            closeHeatmapDash();
        }
    }
    // showHeatmapDash is handled by ipc.js delegated click — do not duplicate here or two opens can stack overlays.
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('heatmapDashModal')) {
        closeHeatmapDash();
    }
});
