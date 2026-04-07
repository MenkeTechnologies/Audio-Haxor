// ── Disk Usage Visualization ──
// Renders horizontal stacked bar charts showing space breakdown by format/type

function renderDiskUsageBar(containerId, data, totalBytes) {
  const el = document.getElementById(containerId);
  if (!el) return;
  if (!data || data.length === 0 || totalBytes === 0) {
    el.style.display = 'none';
    return;
  }
  el.style.display = '';

  const colors = {
    'WAV': 'var(--cyan)', 'MP3': 'var(--accent)', 'AIFF': 'var(--green)',
    'AIF': 'var(--green)', 'FLAC': 'var(--yellow)', 'OGG': 'var(--magenta)',
    'M4A': 'var(--orange)', 'AAC': 'var(--orange)',
    'VST2': 'var(--cyan)', 'VST3': 'var(--accent)', 'AU': 'var(--green)', 'CLAP': 'var(--orange)',
    'Ableton Live': 'var(--cyan)', 'Logic Pro': 'var(--green)',
    'FL Studio': 'var(--orange)', 'REAPER': 'var(--yellow)',
    'Cubase': 'var(--accent)', 'Pro Tools': 'var(--magenta)',
    'Other': 'var(--text-muted)',
  };
  const defaultColor = 'var(--text-dim)';

  // Sort by size descending
  data.sort((a, b) => b.bytes - a.bytes);

  const segments = data.map(d => {
    const pct = ((d.bytes / totalBytes) * 100).toFixed(1);
    const color = colors[d.label] || defaultColor;
    return `<div class="disk-segment" data-bar-pct="${pct}" style="width:0; background: ${color};"
      title="${d.label}: ${d.sizeStr} (${pct}%)"></div>`;
  }).join('');

  const legend = data.filter(d => d.bytes > 0).map(d => {
    const color = colors[d.label] || defaultColor;
    const pct = ((d.bytes / totalBytes) * 100).toFixed(1);
    return `<span class="disk-legend-item">
      <span class="disk-legend-dot" style="background: ${color};"></span>
      ${d.label} <span class="disk-legend-size">${d.sizeStr} (${pct}%)</span>
    </span>`;
  }).join('');

  el.innerHTML = `
    <div class="disk-bar">${segments}</div>
    <div class="disk-legend">${legend}</div>
  `;
  requestAnimationFrame(() => {
    el.querySelectorAll('[data-bar-pct]').forEach(s => {
      s.style.width = s.dataset.barPct + '%';
      s.style.transition = 'width 0.3s ease-out';
    });
  });
}

// Reads already-fetched aggregate bytesByType from module caches populated by
// rebuildAudioStats / refreshDawStatsSnapshot — no extra IPC round-trip.
function updateAudioDiskUsage() {
  const bytes = (typeof _audioBytesByType !== 'undefined' ? _audioBytesByType : null) || {};
  const total = Object.values(bytes).reduce((a, b) => a + b, 0);
  const data = Object.entries(bytes).map(([label, b]) => ({ label, bytes: b, sizeStr: formatAudioSize(b) }));
  renderDiskUsageBar('audioDiskUsage', data, total);
}

function updateDawDiskUsage() {
  const snap = (typeof _dawStatsSnapshot !== 'undefined' ? _dawStatsSnapshot : null);
  const bytes = (snap && snap.bytesByType) || {};
  const total = Object.values(bytes).reduce((a, b) => a + b, 0);
  const data = Object.entries(bytes).map(([label, b]) => ({ label, bytes: b, sizeStr: formatAudioSize(b) }));
  renderDiskUsageBar('dawDiskUsage', data, total);
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
  const key = search.trim() + '|' + (typeFilter || '');
  let counts = {}, bytes = {}, total = 0, unfiltered = 0, totalBytes = 0;
  const cacheHit = !force && key === _lastPluginAggKey && _pluginAggCache;
  try {
    const agg = cacheHit ? _pluginAggCache : await window.vstUpdater.dbPluginFilterStats(search.trim(), typeFilter);
    if (!cacheHit) { _lastPluginAggKey = key; _pluginAggCache = agg; }
    counts = agg.byType || {};
    bytes = agg.bytesByType || {};
    total = agg.count || 0;
    totalBytes = agg.totalBytes || 0;
    unfiltered = agg.totalUnfiltered || 0;
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
    const set = (id, v) => { const e = document.getElementById(id); if (e) e.textContent = v; };
    const isFiltered = unfiltered > 0 && total > 0 && total < unfiltered;
    set('pluginStatsTotal', isFiltered ? total.toLocaleString() + ' / ' + unfiltered.toLocaleString() : total.toLocaleString());
    set('pluginStatsVst3', vst3.toLocaleString());
    set('pluginStatsVst2', vst2.toLocaleString());
    set('pluginStatsAu', au.toLocaleString());
    set('pluginStatsClap', clap.toLocaleString());
    set('pluginStatsOther', other.toLocaleString());
    set('pluginStatsSize', formatAudioSize(totalBytes));
    if (typeof applyInventoryCountsPartial === 'function') applyInventoryCountsPartial({ plugins: unfiltered });
  }
  const data = Object.entries(bytes).map(([label, b]) => ({
    label, bytes: b, sizeStr: formatAudioSize(b),
  }));
  renderDiskUsageBar('pluginDiskUsage', data, totalBytes);
}
