function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

function escapePath(str) {
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
}

function slugify(str) {
  return str
    // Insert hyphen before uppercase letters in camelCase (e.g. MadronaLabs -> Madrona-Labs)
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    // Insert hyphen between letters and digits (e.g. Plugin3 -> Plugin-3)
    .replace(/([a-zA-Z])(\d)/g, '$1-$2')
    .replace(/(\d)([a-zA-Z])/g, '$1-$2')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function buildKvrUrl(name, manufacturer) {
  const nameSlug = slugify(name);
  if (manufacturer && manufacturer !== 'Unknown') {
    const mfgLower = manufacturer.toLowerCase().replace(/[^a-z0-9]+/g, '');
    const mfgSlug = KVR_MANUFACTURER_MAP[mfgLower] || slugify(manufacturer);
    return `https://www.kvraudio.com/product/${nameSlug}-by-${mfgSlug}`;
  }
  return `https://www.kvraudio.com/product/${nameSlug}`;
}

function buildDirsTable(directories, plugins) {
  if (!directories || directories.length === 0) return '';
  const rows = directories.map(dir => {
    const count = plugins.filter(p => p.path.startsWith(dir + '/')).length;
    const types = {};
    plugins.filter(p => p.path.startsWith(dir + '/')).forEach(p => {
      types[p.type] = (types[p.type] || 0) + 1;
    });
    const typeStr = Object.entries(types)
      .map(([t, c]) => `<span class="plugin-type ${t === 'VST2' ? 'type-vst2' : t === 'VST3' ? 'type-vst3' : 'type-au'}">${t}: ${c}</span>`)
      .join(' ');
    return `<tr>
      <td style="padding: 4px 8px 4px 0; color: var(--cyan); opacity: 0.7;">${dir}</td>
      <td style="padding: 4px 8px; text-align: right; font-family: Orbitron, sans-serif; color: var(--text);">${count}</td>
      <td style="padding: 4px 0 4px 8px;">${typeStr}</td>
    </tr>`;
  });
  return `<table style="width: 100%; border-collapse: collapse; margin-top: 6px;">
    <tr style="color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: 1px;">
      <th style="text-align: left; padding: 2px 8px 2px 0;">Directory</th>
      <th style="text-align: right; padding: 2px 8px;">Plugins</th>
      <th style="text-align: left; padding: 2px 0 2px 8px;">Types</th>
    </tr>
    ${rows.join('')}
  </table>`;
}

function toggleDirs() {
  const list = document.getElementById('dirsList');
  const arrow = document.getElementById('dirsArrow');
  list.classList.toggle('open');
  arrow.innerHTML = list.classList.contains('open') ? '&#9660;' : '&#9654;';
}

// ── Tab switching ──
function switchTab(tab) {
  document.querySelectorAll('.tab-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === tab);
  });
  document.getElementById('tabPlugins').classList.toggle('active', tab === 'plugins');
  document.getElementById('tabHistory').classList.toggle('active', tab === 'history');
  document.getElementById('tabSamples').classList.toggle('active', tab === 'samples');
  document.getElementById('tabDaw').classList.toggle('active', tab === 'daw');
  document.getElementById('tabPresets').classList.toggle('active', tab === 'presets');
  document.getElementById('tabSettings').classList.toggle('active', tab === 'settings');
  if (tab === 'history') loadHistory();
  if (tab === 'settings') refreshSettingsUI();
}
