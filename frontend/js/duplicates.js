// ── Duplicate Detection ──
// Find duplicate files by name + size across all scanned items

function findDuplicates(items, keyFn) {
  const groups = {};
  for (const item of items) {
    const key = keyFn(item);
    if (!groups[key]) groups[key] = [];
    groups[key].push(item);
  }
  return Object.values(groups).filter(g => g.length > 1);
}

function showDuplicateReport() {
  const results = [];

  // Plugin duplicates (same name, different paths)
  const pluginDups = findDuplicates(allPlugins, p => p.name.toLowerCase());
  if (pluginDups.length > 0) {
    results.push({ type: 'Plugins', icon: '&#9889;', groups: pluginDups.map(g => ({
      key: g[0].name,
      items: g.map(p => ({ name: p.name, detail: `${p.type} | ${p.version} | ${p.size}`, path: p.path }))
    }))});
  }

  // Sample duplicates (same name + format)
  const sampleDups = findDuplicates(allAudioSamples, s => `${s.name.toLowerCase()}.${s.format.toLowerCase()}`);
  if (sampleDups.length > 0) {
    results.push({ type: 'Samples', icon: '&#127925;', groups: sampleDups.map(g => ({
      key: `${g[0].name}.${g[0].format}`,
      items: g.map(s => ({ name: s.name, detail: `${s.format} | ${s.sizeFormatted}`, path: s.path }))
    }))});
  }

  // DAW duplicates (same name + format)
  const dawDups = findDuplicates(allDawProjects, p => `${p.name.toLowerCase()}.${p.format.toLowerCase()}`);
  if (dawDups.length > 0) {
    results.push({ type: 'DAW Projects', icon: '&#127911;', groups: dawDups.map(g => ({
      key: `${g[0].name}.${g[0].format}`,
      items: g.map(p => ({ name: p.name, detail: `${p.daw} | ${p.sizeFormatted}`, path: p.path }))
    }))});
  }

  // Preset duplicates
  const presetDups = findDuplicates(allPresets, p => `${p.name.toLowerCase()}.${p.format.toLowerCase()}`);
  if (presetDups.length > 0) {
    results.push({ type: 'Presets', icon: '&#127924;', groups: presetDups.map(g => ({
      key: `${g[0].name}.${g[0].format}`,
      items: g.map(p => ({ name: p.name, detail: `${p.format} | ${p.sizeFormatted || ''}`, path: p.path }))
    }))});
  }

  renderDuplicateModal(results);
}

function renderDuplicateModal(results) {
  let existing = document.getElementById('dupModal');
  if (existing) existing.remove();

  const totalGroups = results.reduce((sum, r) => sum + r.groups.length, 0);
  const totalItems = results.reduce((sum, r) => sum + r.groups.reduce((s, g) => s + g.items.length, 0), 0);

  let html = `<div class="modal-overlay" id="dupModal" data-action-modal="closeDup">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Duplicate Detection</h2>
        <button class="modal-close" data-action-modal="closeDup" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">`;

  if (totalGroups === 0) {
    html += '<div class="state-message"><div class="state-icon">&#10003;</div><h2>No Duplicates Found</h2><p>All items have unique names.</p></div>';
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

  html += '</div></div></div>';
  document.body.insertAdjacentHTML('beforeend', html);
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
});
