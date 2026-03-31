function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

// ── fzf-style fuzzy matching with scoring ──

// Scoring constants (from fzf)
const SCORE_MATCH = 16;
const SCORE_GAP_START = -3;
const SCORE_GAP_EXTENSION = -1;
const BONUS_BOUNDARY = 9;
const BONUS_NON_WORD = 8;
const BONUS_CAMEL = 7;
const BONUS_CONSECUTIVE = 4;
const BONUS_FIRST_CHAR_MULT = 2;

function charClass(c) {
  if (c >= 'a' && c <= 'z') return 1; // lower
  if (c >= 'A' && c <= 'Z') return 2; // upper
  if (c >= '0' && c <= '9') return 3; // digit
  return 0; // non-word
}

function positionBonus(prev, curr) {
  const pc = charClass(prev);
  const cc = charClass(curr);
  if (pc === 0 && cc !== 0) return BONUS_BOUNDARY;       // word boundary
  if (pc === 1 && cc === 2) return BONUS_CAMEL;           // camelCase
  if (cc !== 0 && pc !== 0 && pc !== cc) return BONUS_NON_WORD;
  return 0;
}

// Fuzzy match with fzf-style scoring. Returns { score, indices } or null.
function fzfMatch(needle, haystack) {
  const nLen = needle.length, hLen = haystack.length;
  if (nLen === 0) return { score: 0, indices: [] };
  if (nLen > hLen) return null;

  const nLower = needle.toLowerCase();
  const hLower = haystack.toLowerCase();

  // Quick check: all chars present in order
  let ni = 0;
  for (let hi = 0; hi < hLen && ni < nLen; hi++) {
    if (hLower[hi] === nLower[ni]) ni++;
  }
  if (ni < nLen) return null;

  // Find best match using greedy-with-backtrack
  // Try to find the match that maximizes score
  let bestScore = -Infinity, bestIndices = null;

  // Find all positions of first char
  const starts = [];
  for (let i = 0; i <= hLen - nLen; i++) {
    if (hLower[i] === nLower[0]) starts.push(i);
  }

  for (const start of starts) {
    const indices = [start];
    let si = start;
    let valid = true;

    for (let n = 1; n < nLen; n++) {
      let found = false;
      for (let h = si + 1; h < hLen; h++) {
        if (hLower[h] === nLower[n]) {
          indices.push(h);
          si = h;
          found = true;
          break;
        }
      }
      if (!found) { valid = false; break; }
    }
    if (!valid) continue;

    // Score this match
    let score = 0;
    let prevIdx = -2;
    for (let i = 0; i < indices.length; i++) {
      const idx = indices[i];
      score += SCORE_MATCH;

      // Position bonus
      const prev = idx > 0 ? haystack[idx - 1] : ' ';
      let bonus = positionBonus(prev, haystack[idx]);
      if (i === 0) bonus *= BONUS_FIRST_CHAR_MULT;
      score += bonus;

      // Consecutive bonus / gap penalty
      if (prevIdx === idx - 1) {
        score += BONUS_CONSECUTIVE;
      } else if (i > 0) {
        const gap = idx - prevIdx - 1;
        score += SCORE_GAP_START + SCORE_GAP_EXTENSION * (gap - 1);
      }
      prevIdx = idx;
    }

    if (score > bestScore) {
      bestScore = score;
      bestIndices = indices;
    }
  }

  if (!bestIndices) return null;
  return { score: bestScore, indices: bestIndices };
}

// Parse fzf extended search syntax: 'exact, ^prefix, suffix$, !negate, term1 | term2
function parseFzfQuery(query) {
  // Split by spaces, but group | as OR
  const tokens = query.split(/\s+/).filter(Boolean);
  const groups = []; // array of OR-groups, each is array of terms
  let currentGroup = [];

  for (const token of tokens) {
    if (token === '|') continue; // standalone pipe
    if (token.startsWith('|')) {
      currentGroup.push(parseToken(token.slice(1)));
    } else if (token.endsWith('|')) {
      currentGroup.push(parseToken(token.slice(0, -1)));
      groups.push(currentGroup);
      currentGroup = [];
    } else {
      if (currentGroup.length > 0) {
        groups.push(currentGroup);
        currentGroup = [];
      }
      currentGroup = [parseToken(token)];
    }
  }
  if (currentGroup.length > 0) groups.push(currentGroup);
  return groups;
}

function parseToken(token) {
  let negate = false, type = 'fuzzy', text = token;
  if (text.startsWith('!')) { negate = true; text = text.slice(1); }
  if (text.startsWith("'") && text.endsWith("'") && text.length > 2) {
    type = 'exact'; text = text.slice(1, -1);
  } else if (text.startsWith("'")) {
    type = 'exact'; text = text.slice(1);
  } else if (text.startsWith('^')) {
    type = 'prefix'; text = text.slice(1);
  } else if (text.endsWith('$')) {
    type = 'suffix'; text = text.slice(0, -1);
  }
  return { type, text, negate };
}

function matchToken(token, value) {
  const v = value.toLowerCase(), t = token.text.toLowerCase();
  switch (token.type) {
    case 'exact': return v.includes(t);
    case 'prefix': return v.startsWith(t);
    case 'suffix': return v.endsWith(t);
    case 'fuzzy': {
      const m = fzfMatch(token.text, value);
      return m !== null;
    }
  }
  return false;
}

// Unified search: checks fields against fzf-style query.
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
  const groups = parseFzfQuery(query);
  // All groups must match (AND between groups)
  return groups.every(orGroup => {
    // Any term in the OR group must match
    return orGroup.some(token => {
      const matched = fields.some(f => matchToken(token, f));
      return token.negate ? !matched : matched;
    });
  });
}

// Get best fuzzy match indices for highlighting a single field
function getMatchIndices(query, text, mode) {
  if (!query || !text || mode === 'regex') {
    if (mode === 'regex' && query) {
      try {
        const re = new RegExp(query, 'ig');
        const indices = [];
        let m;
        while ((m = re.exec(text)) !== null) {
          for (let i = m.index; i < m.index + m[0].length; i++) indices.push(i);
        }
        return indices;
      } catch { return []; }
    }
    return [];
  }
  // For fzf mode, collect indices from all fuzzy tokens
  const groups = parseFzfQuery(query);
  const allIndices = new Set();
  for (const group of groups) {
    for (const token of group) {
      if (token.negate) continue;
      if (token.type === 'fuzzy') {
        const m = fzfMatch(token.text, text);
        if (m) m.indices.forEach(i => allIndices.add(i));
      } else {
        const t = token.text.toLowerCase();
        const idx = text.toLowerCase().indexOf(t);
        if (idx >= 0) {
          for (let i = idx; i < idx + t.length; i++) allIndices.add(i);
        }
      }
    }
  }
  return [...allIndices].sort((a, b) => a - b);
}

// Highlight matched characters in text
function highlightMatch(text, query, mode) {
  if (!query || !text) return escapeHtml(text);
  const indices = getMatchIndices(query, text, mode);
  if (indices.length === 0) return escapeHtml(text);
  const idxSet = new Set(indices);
  let result = '';
  let inMark = false;
  for (let i = 0; i < text.length; i++) {
    const ch = escapeHtml(text[i]);
    if (idxSet.has(i)) {
      if (!inMark) { result += '<mark class="fzf-hl">'; inMark = true; }
      result += ch;
    } else {
      if (inMark) { result += '</mark>'; inMark = false; }
      result += ch;
    }
  }
  if (inMark) result += '</mark>';
  return result;
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
  let dragStartX = 0;
  let isDragging = false;
  let didMove = false;

  nav.addEventListener('mousedown', (e) => {
    const btn = e.target.closest('.tab-btn');
    if (!btn || e.button !== 0) return;
    draggedTab = btn;
    dragStartX = e.clientX;
    isDragging = false;
    didMove = false;
  });

  document.addEventListener('mousemove', (e) => {
    if (!draggedTab) return;
    // Start drag after 5px threshold to avoid accidental drags on click
    if (!isDragging && Math.abs(e.clientX - dragStartX) > 5) {
      isDragging = true;
      draggedTab.classList.add('tab-dragging');
    }
    if (!isDragging) return;
    didMove = true;

    const target = document.elementFromPoint(e.clientX, e.clientY)?.closest('.tab-btn');
    nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
    if (target && target !== draggedTab) {
      target.classList.add('tab-drag-over');
    }
  });

  document.addEventListener('mouseup', (e) => {
    if (!draggedTab) return;

    if (isDragging) {
      const target = document.elementFromPoint(e.clientX, e.clientY)?.closest('.tab-btn');
      nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
      draggedTab.classList.remove('tab-dragging');

      if (target && target !== draggedTab) {
        const tabs = [...nav.querySelectorAll('.tab-btn')];
        const dragIdx = tabs.indexOf(draggedTab);
        const dropIdx = tabs.indexOf(target);
        if (dragIdx < dropIdx) {
          nav.insertBefore(draggedTab, target.nextSibling);
        } else {
          nav.insertBefore(draggedTab, target);
        }
        saveTabOrder();
      }
    }

    // Suppress the click that follows mouseup if we actually dragged
    if (didMove) {
      const suppress = (ev) => { ev.stopPropagation(); ev.preventDefault(); };
      nav.addEventListener('click', suppress, { capture: true, once: true });
    }

    draggedTab = null;
    isDragging = false;
    didMove = false;
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
