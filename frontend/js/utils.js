function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

// Fuzzy match: all characters of needle appear in haystack in order
function fuzzyMatch(needle, haystack) {
  let ni = 0;
  for (let hi = 0; hi < haystack.length && ni < needle.length; hi++) {
    if (haystack[hi] === needle[ni]) ni++;
  }
  return ni === needle.length;
}

// Unified search: checks one or more fields against the query.
// mode: 'fuzzy' (default) or 'regex'
function searchMatch(query, fields, mode) {
  if (!query) return true;
  if (mode === 'regex') {
    try {
      const re = new RegExp(query, 'i');
      return fields.some(f => re.test(f));
    } catch {
      return fields.some(f => f.toLowerCase().includes(query.toLowerCase()));
    }
  }
  const q = query.toLowerCase();
  return fields.some(f => fuzzyMatch(q, f.toLowerCase()));
}

// Get search mode for a tab's regex toggle
function getSearchMode(toggleId) {
  const btn = document.getElementById(toggleId);
  return btn && btn.classList.contains('active') ? 'regex' : 'fuzzy';
}

function toggleRegex(btn) {
  btn.classList.toggle('active');
  const isRegex = btn.classList.contains('active');
  const input = btn.closest('.search-box').querySelector('input');
  if (input) {
    const base = input.placeholder.replace(/^(Fuzzy|Regex) /, '');
    input.placeholder = (isRegex ? 'Regex ' : 'Fuzzy ') + base;
    // Re-trigger the filter
    const action = btn.dataset.target;
    if (action === 'filterPlugins') filterPlugins();
    else if (action === 'filterAudioSamples') filterAudioSamples();
    else if (action === 'filterDawProjects') filterDawProjects();
    else if (action === 'filterPresets') filterPresets();
  }
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

// ── Tab drag reorder ──
function initTabDragReorder() {
  const nav = document.querySelector('.tab-nav');
  let draggedTab = null;

  nav.addEventListener('dragstart', (e) => {
    const btn = e.target.closest('.tab-btn');
    if (!btn) return;
    draggedTab = btn;
    btn.classList.add('tab-dragging');
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', '');
  });

  nav.addEventListener('dragend', (e) => {
    const btn = e.target.closest('.tab-btn');
    if (btn) btn.classList.remove('tab-dragging');
    nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
    draggedTab = null;
  });

  nav.addEventListener('dragover', (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    const target = e.target.closest('.tab-btn');
    nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
    if (target && target !== draggedTab) {
      target.classList.add('tab-drag-over');
    }
  });

  nav.addEventListener('dragleave', (e) => {
    const target = e.target.closest('.tab-btn');
    if (target) target.classList.remove('tab-drag-over');
  });

  nav.addEventListener('drop', (e) => {
    e.preventDefault();
    const target = e.target.closest('.tab-btn');
    if (!target || !draggedTab || target === draggedTab) return;
    target.classList.remove('tab-drag-over');

    // Reorder in DOM
    const tabs = [...nav.querySelectorAll('.tab-btn')];
    const dragIdx = tabs.indexOf(draggedTab);
    const dropIdx = tabs.indexOf(target);
    if (dragIdx < dropIdx) {
      nav.insertBefore(draggedTab, target.nextSibling);
    } else {
      nav.insertBefore(draggedTab, target);
    }

    saveTabOrder();
  });

  // Make tabs draggable
  nav.querySelectorAll('.tab-btn').forEach(btn => {
    btn.setAttribute('draggable', 'true');
  });

  // Restore saved order
  restoreTabOrder();
}

function saveTabOrder() {
  const tabs = [...document.querySelectorAll('.tab-nav .tab-btn')].map(b => b.dataset.tab);
  prefs.setItem('tabOrder', JSON.stringify(tabs));
}

function restoreTabOrder() {
  const saved = prefs.getItem('tabOrder');
  if (!saved) return;
  try {
    const order = JSON.parse(saved);
    if (!Array.isArray(order)) return;
    const nav = document.querySelector('.tab-nav');
    const tabs = [...nav.querySelectorAll('.tab-btn')];
    const tabMap = {};
    tabs.forEach(btn => { tabMap[btn.dataset.tab] = btn; });
    // Re-append in saved order, skip any missing
    for (const key of order) {
      if (tabMap[key]) nav.appendChild(tabMap[key]);
    }
    // Append any tabs not in saved order (new tabs)
    tabs.forEach(btn => {
      if (!order.includes(btn.dataset.tab)) nav.appendChild(btn);
    });
  } catch {}
}

function settingResetTabOrder() {
  prefs.removeItem('tabOrder');
  const nav = document.querySelector('.tab-nav');
  const defaultOrder = ['plugins', 'samples', 'daw', 'presets', 'history', 'settings'];
  const tabMap = {};
  nav.querySelectorAll('.tab-btn').forEach(btn => { tabMap[btn.dataset.tab] = btn; });
  for (const key of defaultOrder) {
    if (tabMap[key]) nav.appendChild(tabMap[key]);
  }
  showToast('Tab order reset');
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
