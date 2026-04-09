// ── Disk Usage Visualization ──
// Renders horizontal stacked bar charts showing space breakdown by format/type

/** Label → hue key; colors from `index.html` `.disk-usage [data-kind]` (not inline var() — WKWebView). */
const DISK_LABEL_KIND = {
    WAV: 'cyan', MP3: 'accent', AIFF: 'green', AIF: 'green', FLAC: 'yellow', OGG: 'magenta',
    M4A: 'orange', AAC: 'orange',
    OPUS: 'magenta', WMA: 'muted', REX: 'accent', RX2: 'accent', SF2: 'green', SFZ: 'green',
    VST2: 'cyan', VST3: 'accent', AU: 'green', CLAP: 'orange',
    'Ableton Live': 'cyan', 'Logic Pro': 'green', 'FL Studio': 'orange', REAPER: 'yellow',
    Cubase: 'accent', Nuendo: 'accent', 'Pro Tools': 'magenta', 'Bitwig Studio': 'accent',
    'Studio One': 'orange', Reason: 'magenta', GarageBand: 'green', Audacity: 'cyan',
    Other: 'muted',
    FXP: 'cyan', FXB: 'accent', VSTPRESET: 'green', AUPRESET: 'yellow',
    ADG: 'orange', ADV: 'magenta', NKI: 'cyan', H2P: 'accent', SYX: 'green',
};

function diskLabelKind(label) {
    const k = label != null ? String(label) : '';
    return DISK_LABEL_KIND[k] || 'muted';
}

/** For title="…" — avoid breaking HTML when labels contain quotes. */
function diskAttrEscape(s) {
    return String(s ?? '')
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/</g, '&lt;');
}

function renderDiskUsageBar(containerId, data, totalBytes) {
    const el = document.getElementById(containerId);
    if (!el) return;
    if (!data || data.length === 0) {
        el.style.display = 'none';
        return;
    }

    // Sort by size descending
    data.sort((a, b) => b.bytes - a.bytes);

    const weights = data.map((d) => Number(d.bytes) || 0);
    const sumBytes = weights.reduce((s, w) => s + w, 0);
    if (sumBytes === 0 && (!totalBytes || totalBytes === 0)) {
        el.style.display = 'none';
        return;
    }
    el.style.display = '';

    const denom = sumBytes > 0 ? sumBytes : totalBytes;
    let pcts = weights.map((w) => (denom > 0 ? (w / denom) * 100 : 0));
    const sumP = pcts.reduce((a, b) => a + b, 0);
    if (sumP > 0 && Math.abs(sumP - 100) > 1e-9) {
        const s = 100 / sumP;
        pcts = pcts.map((p) => p * s);
    }

    /** Table-cell + width:% — % resolves against the bar (table); flex % on WKWebView stayed at min-width. */
    const segments = data.map((d, i) => {
        const pctLabel = ((d.bytes / denom) * 100).toFixed(1);
        const kind = diskLabelKind(d.label);
        const wPct = pcts[i].toFixed(4);
        return `<div class="disk-segment" data-kind="${kind}" style="width:${wPct}%" title="${diskAttrEscape(d.label)}: ${diskAttrEscape(d.sizeStr)} (${pctLabel}%)"></div>`;
    }).join('');

    const legend = data.filter(d => d.bytes > 0).map(d => {
        const kind = diskLabelKind(d.label);
        const pct = ((d.bytes / denom) * 100).toFixed(1);
        return `<span class="disk-legend-item">
      <span class="disk-legend-dot" data-kind="${kind}"></span>
      ${d.label} <span class="disk-legend-size">${d.sizeStr} (${pct}%)</span>
    </span>`;
    }).join('');

    el.innerHTML = `
    <div class="disk-bar-wrap">
      <div class="disk-bar">${segments}</div>
    </div>
    <div class="disk-legend">${legend}</div>
  `;
    if (typeof el.querySelector === 'function') {
        const bar = el.querySelector('.disk-bar');
        if (bar) void bar.offsetWidth;
    }
}

// Reads already-fetched aggregate bytesByType from module caches populated by
// rebuildAudioStats / rebuildDawFilterStats — no extra IPC round-trip.
function updateAudioDiskUsage() {
    const bytes = (typeof _audioBytesByType !== 'undefined' ? _audioBytesByType : null) || {};
    const total = Object.values(bytes).reduce((a, b) => a + b, 0);
    const data = Object.entries(bytes).map(([label, b]) => ({label, bytes: b, sizeStr: formatAudioSize(b)}));
    renderDiskUsageBar('audioDiskUsage', data, total);
}

function updateDawDiskUsage() {
    const snap = (typeof _dawStatsSnapshot !== 'undefined' ? _dawStatsSnapshot : null);
    const bytes = (snap && snap.bytesByType) || {};
    const total = Object.values(bytes).reduce((a, b) => a + b, 0);
    const data = Object.entries(bytes).map(([label, b]) => ({label, bytes: b, sizeStr: formatAudioSize(b)}));
    renderDiskUsageBar('dawDiskUsage', data, total);
}

/** Presets tab: stacked bar under stats — `bytesByType` from `db_preset_filter_stats` (or count-weighted fallback). */
function updatePresetDiskUsage(bytesByType) {
    const raw = bytesByType && typeof bytesByType === 'object' ? bytesByType : {};
    const total = Object.values(raw).reduce((a, b) => a + Number(b || 0), 0);
    const data = Object.entries(raw).map(([label, b]) => ({
        label,
        bytes: Number(b) || 0,
        sizeStr: formatAudioSize(Number(b) || 0),
    }));
    renderDiskUsageBar('presetDiskUsage', data, total);
}

// Build disk usage data from plugin types + populate the plugin stats row
// (styled like the samples-tab audio-stats row: Total, VST3, VST2, AU, CLAP, Other, Size).
// Reflects the current search + type filter via db_plugin_filter_stats.
let _lastPluginAggKey = null;
let _pluginAggCache = null;

async function updatePluginDiskUsage(force) {
    const statsEl = document.getElementById('pluginStats');
    const search = document.getElementById('searchInput')?.value || '';
    const typeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('typeFilter') : null;
    const typeFilter = typeSet ? [...typeSet].join(',') : null;
    const regexOn = typeof getSearchMode === 'function' && getSearchMode('regexPlugins') === 'regex';
    const key = search.trim() + '|' + (typeFilter || '') + '|' + (regexOn ? 'r' : 'f');
    let counts = {}, bytes = {}, total = 0, unfiltered = 0, totalBytes = 0;
    let countCapped = false;
    const cacheHit = !force && key === _lastPluginAggKey && _pluginAggCache;
    try {
        const agg = cacheHit ? _pluginAggCache : await window.vstUpdater.dbPluginFilterStats(search.trim(), typeFilter, regexOn);
        if (!cacheHit) {
            _lastPluginAggKey = key;
            _pluginAggCache = agg;
        }
        counts = agg.byType || {};
        bytes = agg.bytesByType || {};
        total = agg.count || 0;
        totalBytes = agg.totalBytes || 0;
        unfiltered = agg.totalUnfiltered || 0;
        countCapped = agg.countCapped === true;
    } catch {
        // Fallback to local data
        if (typeof allPlugins === 'undefined' || allPlugins.length === 0) return;
        for (const p of allPlugins) {
            const sz = typeof p.sizeBytes === 'number' && isFinite(p.sizeBytes) ? p.sizeBytes : 0;
            counts[p.type] = (counts[p.type] || 0) + 1;
            bytes[p.type] = (bytes[p.type] || 0) + sz;
            totalBytes += sz;
        }
        total = allPlugins.length;
        unfiltered = total;
    }
    if (statsEl) {
        const vst3 = counts['VST3'] || 0;
        const vst2 = counts['VST2'] || 0;
        const au = counts['AU'] || 0;
        const clap = counts['CLAP'] || 0;
        const other = Math.max(0, total - vst3 - vst2 - au - clap);
        statsEl.style.display = (total > 0 || unfiltered > 0) ? 'flex' : 'none';
        const set = (id, v) => {
            const e = document.getElementById(id);
            if (e) e.textContent = v;
        };
        const isFiltered = unfiltered > 0 && total > 0 && total < unfiltered;
        const totalPart = countCapped ? total.toLocaleString() + '+' : total.toLocaleString();
        set('pluginStatsTotal', isFiltered ? totalPart + ' / ' + unfiltered.toLocaleString() : totalPart);
        set('pluginStatsVst3', vst3.toLocaleString());
        set('pluginStatsVst2', vst2.toLocaleString());
        set('pluginStatsAu', au.toLocaleString());
        set('pluginStatsClap', clap.toLocaleString());
        set('pluginStatsOther', other.toLocaleString());
        set('pluginStatsSize', formatAudioSize(totalBytes));
        if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({plugins: unfiltered});
    }
    const data = Object.entries(bytes).map(([label, b]) => ({
        label, bytes: b, sizeStr: formatAudioSize(b),
    }));
    renderDiskUsageBar('pluginDiskUsage', data, totalBytes);
}
