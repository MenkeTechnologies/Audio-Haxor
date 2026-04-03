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
    'VST2': 'var(--cyan)', 'VST3': 'var(--accent)', 'AU': 'var(--green)',
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

// Build disk usage data from audio stats
function updateAudioDiskUsage() {
  const counts = {};
  const bytes = {};
  for (const s of allAudioSamples) {
    const sz = typeof s.size === 'number' && isFinite(s.size) ? s.size : 0;
    counts[s.format] = (counts[s.format] || 0) + 1;
    bytes[s.format] = (bytes[s.format] || 0) + sz;
  }
  const data = Object.entries(bytes).map(([label, b]) => ({
    label, bytes: b, sizeStr: formatAudioSize(b),
  }));
  const total = Object.values(bytes).reduce((a, b) => a + b, 0);
  renderDiskUsageBar('audioDiskUsage', data, total);
}

// Build disk usage data from DAW stats
function updateDawDiskUsage() {
  const bytes = {};
  for (const p of allDawProjects) {
    bytes[p.daw] = (bytes[p.daw] || 0) + (p.size || 0);
  }
  const data = Object.entries(bytes).map(([label, b]) => ({
    label, bytes: b, sizeStr: formatAudioSize(b),
  }));
  const total = Object.values(bytes).reduce((a, b) => a + b, 0);
  renderDiskUsageBar('dawDiskUsage', data, total);
}

// Build disk usage data from plugin types
function updatePluginDiskUsage() {
  if (typeof allPlugins === 'undefined' || allPlugins.length === 0) return;
  const bytes = {};
  for (const p of allPlugins) {
    const sz = typeof p.sizeBytes === 'number' && isFinite(p.sizeBytes) ? p.sizeBytes : 0;
    bytes[p.type] = (bytes[p.type] || 0) + sz;
  }
  const data = Object.entries(bytes).map(([label, b]) => ({
    label, bytes: b, sizeStr: formatAudioSize(b),
  }));
  const total = Object.values(bytes).reduce((a, b) => a + b, 0);
  renderDiskUsageBar('pluginDiskUsage', data, total);
}
