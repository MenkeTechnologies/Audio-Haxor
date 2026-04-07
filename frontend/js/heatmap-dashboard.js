// ── Audio Heatmap Dashboard ──
// Full-screen analytics overview: format distribution, size breakdown,
// folder heatmap, BPM histogram, key wheel, activity timeline.

function _hmFmt(key, vars) {
    return catalogFmt(key, vars);
}

function showHeatmapDashboard() {
    let existing = document.getElementById('heatmapDashModal');
    if (existing) existing.remove();

    const samples = typeof allAudioSamples !== 'undefined' ? allAudioSamples : [];
    const plugins = typeof allPlugins !== 'undefined' ? allPlugins : [];
    const projects = typeof allDawProjects !== 'undefined' ? allDawProjects : [];
    const presets = typeof allPresets !== 'undefined' ? allPresets : [];

    const html = `<div class="modal-overlay" id="heatmapDashModal" data-action-modal="closeHeatmapDash">
    <div class="modal-content modal-wide" style="max-width:95vw;width:95vw;max-height:95vh;height:95vh;">
      <div class="modal-header">
        <h2>${escapeHtml(_hmFmt('ui.hm.title'))}</h2>
        <button class="modal-close" data-action-modal="closeHeatmapDash" title="${escapeHtml(_hmFmt('ui.hm.close'))}">&#10005;</button>
      </div>
      <div class="modal-body" style="overflow-y:auto;max-height:calc(90vh - 60px);">
        <div class="hm-overview" id="hmOverview"></div>
        <div class="hm-grid" id="hmGrid"></div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);

    // Double-rAF: first frame makes modal visible, second frame renders with correct widths
    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            renderDashboard(samples, plugins, projects, presets);
        });
    });
}

function closeHeatmapDash() {
    const el = document.getElementById('heatmapDashModal');
    if (el) el.remove();
}

function renderDashboard(samples, plugins, projects, presets) {
    const overview = document.getElementById('hmOverview');
    const grid = document.getElementById('hmGrid');
    if (!overview || !grid) return;

    const totalSize = samples.reduce((s, a) => s + (a.size || a.sizeBytes || 0), 0);

    // Overview stats
    overview.innerHTML = `
    <div class="hm-stat"><span class="hm-stat-val">${samples.length.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_samples'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${plugins.length.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_plugins'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${projects.length.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_daw'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${presets.length.toLocaleString()}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_presets'))}</span></div>
    <div class="hm-stat"><span class="hm-stat-val">${typeof formatAudioSize === 'function' ? formatAudioSize(totalSize) : (totalSize / (1024 * 1024 * 1024)).toFixed(1) + ' GB'}</span><span class="hm-stat-label">${escapeHtml(_hmFmt('ui.hm.overview_total_size'))}</span></div>
  `;

    let cards = '';

    // 1. Format distribution (pie-like horizontal bars)
    cards += buildFormatCard(samples);

    // 2. Size distribution histogram
    cards += buildSizeCard(samples);

    // 3. Folder heatmap (top directories by file count)
    cards += buildFolderCard(samples);

    // 4. BPM distribution (if any cached)
    cards += buildBpmCard();

    // 5. Key distribution (if any cached)
    cards += buildKeyCard();

    // 6. Activity timeline (files by modified month)
    cards += buildTimelineCard(samples);

    // 7. Plugin type breakdown
    cards += buildPluginTypeCard(plugins);

    // 8. DAW format breakdown
    cards += buildDawFormatCard(projects);

    grid.innerHTML = cards;

    // Make dashboard cards draggable to reorder
    if (typeof initDragReorder === 'function') {
        initDragReorder(grid, '.hm-card', 'hmCardOrder', {
            getKey: (el) => el.dataset.hmCard || '',
        });
    }

    // Render canvases after DOM insertion
    renderBpmHistogram();
    renderKeyWheel();
    renderTimelineChart(samples);

    // Apply bar widths after layout resolves (flex containers need a frame to get correct widths)
    requestAnimationFrame(() => {
        grid.querySelectorAll('[data-bar-pct]').forEach(el => {
            el.style.width = el.dataset.barPct + '%';
            el.style.transition = 'width 0.3s ease-out';
        });
    });
}

// ── Card Builders ──

function buildFormatCard(samples) {
    // Use full DB stats if available (samples is only the current page)
    const counts = (typeof audioStatCounts !== 'undefined' && Object.keys(audioStatCounts).length > 0)
        ? {...audioStatCounts}
        : (() => {
            const c = {};
            for (const s of samples) c[s.format] = (c[s.format] || 0) + 1;
            return c;
        })();
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

function buildSizeCard(samples) {
    const buckets = [
        {labelKey: 'ui.hm.bucket_lt_100kb', max: 100 * 1024},
        {labelKey: 'ui.hm.bucket_100kb_1mb', max: 1024 * 1024},
        {labelKey: 'ui.hm.bucket_1_10mb', max: 10 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_10_50mb', max: 50 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_50_100mb', max: 100 * 1024 * 1024},
        {labelKey: 'ui.hm.bucket_gt_100mb', max: Infinity},
    ];
    const counts = new Array(buckets.length).fill(0);
    for (const s of samples) {
        const sz = s.size || s.sizeBytes || 0;
        for (let i = 0; i < buckets.length; i++) {
            if (sz < buckets[i].max || i === buckets.length - 1) {
                counts[i]++;
                break;
            }
        }
    }
    const total = samples.length || 1;
    const maxBucket = Math.max(...counts, 1);
    const bars = buckets.map((b, i) => {
        const barPct = (counts[i] / maxBucket) * 100;
        const share = ((counts[i] / total) * 100).toFixed(1);
        const bl = _hmFmt(b.labelKey);
        return `<div class="hm-bar-row">
      <span class="hm-bar-label">${escapeHtml(bl)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-magenta" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${counts[i].toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');

    return `<div class="hm-card" data-hm-card="size"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_size_dist'))}</h3>${bars}</div>`;
}

function buildFolderCard(samples) {
    const dirs = {};
    for (const s of samples) {
        const dir = s.directory || s.path?.replace(/\/[^/]+$/, '') || _hmFmt('ui.hm.unknown');
        // Use top 2 path components for grouping
        const parts = dir.split('/').filter(Boolean);
        const key = '/' + parts.slice(0, Math.min(parts.length, 3)).join('/');
        dirs[key] = (dirs[key] || 0) + 1;
    }
    const sorted = Object.entries(dirs).sort((a, b) => b[1] - a[1]).slice(0, 12);
    const total = samples.length || 1;
    const maxFolder = sorted.length > 0 ? sorted[0][1] : 1;

    const bars = sorted.map(([dir, count]) => {
        const barPct = (count / maxFolder) * 100;
        const share = ((count / total) * 100).toFixed(1);
        const name = dir.split('/').pop() || dir;
        return `<div class="hm-bar-row" title="${escapeHtml(dir)}">
      <span class="hm-bar-label" style="max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${escapeHtml(name)}</span>
      <div class="hm-bar-track"><div class="hm-bar-fill hm-bar-green" data-bar-pct="${barPct.toFixed(1)}" style="width:0"></div></div>
      <span class="hm-bar-val">${count.toLocaleString()} (${share}%)</span>
    </div>`;
    }).join('');

    return `<div class="hm-card" data-hm-card="folders"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_top_folders'))}</h3>${bars || `<span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.empty_no_data'))}</span>`}</div>`;
}

function buildBpmCard() {
    const bpms = typeof _bpmCache !== 'undefined' ? Object.values(_bpmCache).filter(v => v && v > 0) : [];
    if (bpms.length === 0) {
        return `<div class="hm-card" data-hm-card="bpm"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_bpm_dist'))}</h3><span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.card_bpm_empty'))}</span></div>`;
    }
    return `<div class="hm-card" data-hm-card="bpm"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_bpm_title_analyzed', {n: bpms.length}))}</h3><canvas id="hmBpmCanvas" width="400" height="120" style="width:100%;height:120px;" title="${escapeHtml(_hmFmt('ui.hm.card_bpm_canvas_title'))}"></canvas></div>`;
}

function buildKeyCard() {
    const keys = typeof _keyCache !== 'undefined' ? Object.values(_keyCache).filter(Boolean) : [];
    if (keys.length === 0) {
        return `<div class="hm-card" data-hm-card="key"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_key_dist'))}</h3><span class="hm-empty">${escapeHtml(_hmFmt('ui.hm.card_key_empty'))}</span></div>`;
    }
    return `<div class="hm-card" data-hm-card="key"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_key_title_analyzed', {n: keys.length}))}</h3><canvas id="hmKeyCanvas" width="400" height="200" style="width:100%;height:200px;" title="${escapeHtml(_hmFmt('ui.hm.card_key_canvas_title'))}"></canvas></div>`;
}

function buildTimelineCard(samples) {
    if (samples.length === 0) return '';
    return `<div class="hm-card hm-card-wide" data-hm-card="timeline"><h3 class="hm-card-title">${escapeHtml(_hmFmt('ui.hm.card_timeline'))}</h3><canvas id="hmTimelineCanvas" width="800" height="100" style="width:100%;height:100px;" title="${escapeHtml(_hmFmt('ui.hm.card_timeline_canvas_title'))}"></canvas></div>`;
}

function buildPluginTypeCard(plugins) {
    if (plugins.length === 0) return '';
    const types = {};
    for (const p of plugins) types[p.type || _hmFmt('ui.hm.unknown')] = (types[p.type || _hmFmt('ui.hm.unknown')] || 0) + 1;
    const sorted = Object.entries(types).sort((a, b) => b[1] - a[1]);
    const total = plugins.length || 1;
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

function buildDawFormatCard(projects) {
    if (projects.length === 0) return '';
    const fmts = {};
    for (const p of projects) {
        const fmt = p.daw || p.format || _hmFmt('ui.hm.unknown');
        fmts[fmt] = (fmts[fmt] || 0) + 1;
    }
    const sorted = Object.entries(fmts).sort((a, b) => b[1] - a[1]);
    const total = projects.length || 1;
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

function renderBpmHistogram() {
    const canvas = document.getElementById('hmBpmCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const bpms = typeof _bpmCache !== 'undefined' ? Object.values(_bpmCache).filter(v => v && v > 0) : [];
    if (bpms.length === 0) return;

    // Histogram: 50-220 BPM in 5 BPM bins
    const minBpm = 50, maxBpm = 220, binWidth = 5;
    const numBins = Math.ceil((maxBpm - minBpm) / binWidth);
    const bins = new Array(numBins).fill(0);
    for (const bpm of bpms) {
        const idx = Math.floor((bpm - minBpm) / binWidth);
        if (idx >= 0 && idx < numBins) bins[idx]++;
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

function renderKeyWheel() {
    const canvas = document.getElementById('hmKeyCanvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const keys = typeof _keyCache !== 'undefined' ? Object.values(_keyCache).filter(Boolean) : [];
    if (keys.length === 0) return;

    // Count by key
    const counts = {};
    for (const k of keys) counts[k] = (counts[k] || 0) + 1;
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
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

        // Count + percentage
        const keyTotal = keys.length || 1;
        const keyShare = ((count / keyTotal) * 100).toFixed(0);
        ctx.textAlign = 'left';
        ctx.fillStyle = 'rgba(122,139,168,0.8)';
        ctx.fillText(`${count} (${keyShare}%)`, 82 + barW + 4, y + barH - 3);
    }
}

function renderTimelineChart(samples) {
    const canvas = document.getElementById('hmTimelineCanvas');
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
    if (e.target.closest('[data-action="showHeatmapDash"]')) {
        showHeatmapDashboard();
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('heatmapDashModal')) {
        closeHeatmapDash();
    }
});
