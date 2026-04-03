function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

// ── fzf-style fuzzy matching with scoring ──

// Scoring constants (from fzf) — configurable via settings
let SCORE_MATCH = 16;
let SCORE_GAP_START = -3;
let SCORE_GAP_EXTENSION = -1;
let BONUS_BOUNDARY = 9;
let BONUS_NON_WORD = 8;
let BONUS_CAMEL = 7;
let BONUS_CONSECUTIVE = 4;
let BONUS_FIRST_CHAR_MULT = 2;

const FZF_DEFAULTS = { SCORE_MATCH: 16, SCORE_GAP_START: -3, SCORE_GAP_EXTENSION: -1, BONUS_BOUNDARY: 9, BONUS_NON_WORD: 8, BONUS_CAMEL: 7, BONUS_CONSECUTIVE: 4, BONUS_FIRST_CHAR_MULT: 2 };

function loadFzfParams() {
  const saved = prefs.getObject('fzfParams', null);
  if (saved) {
    SCORE_MATCH = saved.SCORE_MATCH ?? 16;
    SCORE_GAP_START = saved.SCORE_GAP_START ?? -3;
    SCORE_GAP_EXTENSION = saved.SCORE_GAP_EXTENSION ?? -1;
    BONUS_BOUNDARY = saved.BONUS_BOUNDARY ?? 9;
    BONUS_NON_WORD = saved.BONUS_NON_WORD ?? 8;
    BONUS_CAMEL = saved.BONUS_CAMEL ?? 7;
    BONUS_CONSECUTIVE = saved.BONUS_CONSECUTIVE ?? 4;
    BONUS_FIRST_CHAR_MULT = saved.BONUS_FIRST_CHAR_MULT ?? 2;
  }
}

function saveFzfParams() {
  prefs.setItem('fzfParams', { SCORE_MATCH, SCORE_GAP_START, SCORE_GAP_EXTENSION, BONUS_BOUNDARY, BONUS_NON_WORD, BONUS_CAMEL, BONUS_CONSECUTIVE, BONUS_FIRST_CHAR_MULT });
}

function resetFzfParams() {
  Object.assign(window, FZF_DEFAULTS);
  SCORE_MATCH = FZF_DEFAULTS.SCORE_MATCH;
  SCORE_GAP_START = FZF_DEFAULTS.SCORE_GAP_START;
  SCORE_GAP_EXTENSION = FZF_DEFAULTS.SCORE_GAP_EXTENSION;
  BONUS_BOUNDARY = FZF_DEFAULTS.BONUS_BOUNDARY;
  BONUS_NON_WORD = FZF_DEFAULTS.BONUS_NON_WORD;
  BONUS_CAMEL = FZF_DEFAULTS.BONUS_CAMEL;
  BONUS_CONSECUTIVE = FZF_DEFAULTS.BONUS_CONSECUTIVE;
  BONUS_FIRST_CHAR_MULT = FZF_DEFAULTS.BONUS_FIRST_CHAR_MULT;
  saveFzfParams();
  if (typeof renderFzfSettings === 'function') renderFzfSettings();
}

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

// Score bonus for substring/exact matches over fuzzy-only
const SCORE_SUBSTRING_BONUS = 1000;
const SCORE_EXACT_BONUS = 2000;      // full string match
const SCORE_PREFIX_BONUS = 1500;

// Score a single token against a value. Returns score > 0 for match, 0 for no match.
function scoreToken(token, value) {
  const v = value.toLowerCase(), t = token.text.toLowerCase();
  switch (token.type) {
    case 'exact': return v.includes(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
    case 'prefix': return v.startsWith(t) ? SCORE_PREFIX_BONUS + t.length * SCORE_MATCH : 0;
    case 'suffix': return v.endsWith(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
    case 'fuzzy': {
      // Try exact/substring first — always prioritized
      if (v === t) return SCORE_EXACT_BONUS + t.length * SCORE_MATCH;
      if (v.includes(t)) return SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH;
      // Fuzzy fallback
      const m = fzfMatch(token.text, value);
      return m ? m.score : 0;
    }
  }
  return 0;
}

// Unified search: checks fields against fzf-style query.
// mode: 'fuzzy' (default) or 'regex'
// Returns score > 0 for match, 0 for no match. Use searchMatch() for boolean.
function searchScore(query, fields, mode) {
  if (!query) return 1; // empty query matches everything
  if (mode === 'regex') {
    try {
      const re = new RegExp(query, 'i');
      return fields.some(f => re.test(f)) ? 1 : 0;
    } catch {
      return fields.some(f => f.toLowerCase().includes(query.toLowerCase())) ? 1 : 0;
    }
  }
  const groups = parseFzfQuery(query);
  let totalScore = 0;
  // All groups must match (AND between groups)
  for (const orGroup of groups) {
    let bestGroupScore = 0;
    for (const token of orGroup) {
      let tokenBest = 0;
      for (let fi = 0; fi < fields.length; fi++) {
        // First field (name) gets 500 bonus, subsequent fields get less
        const fieldBonus = fi === 0 ? 500 : 0;
        const s = scoreToken(token, fields[fi]);
        if (s > 0 && s + fieldBonus > tokenBest) tokenBest = s + fieldBonus;
      }
      if (token.negate) {
        if (tokenBest > 0) return 0; // negated term matched => fail
        bestGroupScore = 1; // negated term didn't match => pass
      } else {
        if (tokenBest > bestGroupScore) bestGroupScore = tokenBest;
      }
    }
    if (bestGroupScore === 0) return 0; // group didn't match
    totalScore += bestGroupScore;
  }
  return totalScore;
}

// Boolean wrapper for backward compat
function searchMatch(query, fields, mode) {
  return searchScore(query, fields, mode) > 0;
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

// Extension-to-dropdown value mapping for auto-select
const EXT_TO_FILTER = {
  // Audio formats
  'wav': 'WAV', 'mp3': 'MP3', 'aiff': 'AIFF', 'aif': 'AIF',
  'flac': 'FLAC', 'ogg': 'OGG', 'm4a': 'M4A', 'aac': 'AAC',
  // Plugin types
  'vst2': 'VST2', 'vst3': 'VST3', 'au': 'Audio Units', 'component': 'Audio Units',
  // DAW formats → DAW names
  'als': 'Ableton Live', 'alp': 'Ableton Live', 'ableton': 'Ableton Live',
  'logicx': 'Logic Pro', 'logic': 'Logic Pro',
  'flp': 'FL Studio', 'fl': 'FL Studio',
  'cpr': 'Cubase', 'cubase': 'Cubase',
  'rpp': 'REAPER', 'reaper': 'REAPER',
  'ptx': 'Pro Tools', 'ptf': 'Pro Tools', 'protools': 'Pro Tools',
  'bwproject': 'Bitwig Studio', 'bitwig': 'Bitwig Studio',
  'song': 'Studio One', 'studioone': 'Studio One',
  'reason': 'Reason',
  'aup': 'Audacity', 'aup3': 'Audacity', 'audacity': 'Audacity',
  'band': 'GarageBand', 'garageband': 'GarageBand',
  'ardour': 'Ardour',
  'dawproject': 'DAWproject',
};

// Auto-select dropdown when search matches a format/type keyword.
// Works with both native selects and multi-filter widgets.
function autoSelectDropdown(selectEl, search) {
  if (!selectEl) return;
  const q = search.trim().toLowerCase().replace(/^\./, '');
  if (!q) {
    if (selectEl._autoSet) {
      setMultiFilterValue(selectEl.id, 'all');
      selectEl._autoSet = false;
    }
    return;
  }
  const words = q.split(/\s+/);
  for (const word of words) {
    const mapped = EXT_TO_FILTER[word];
    if (mapped) {
      const option = [...selectEl.options].find(o => o.value === mapped);
      if (option) {
        setMultiFilterValue(selectEl.id, mapped);
        selectEl._autoSet = true;
      }
      return;
    }
  }
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

// ── Confirm dialog (Tauri-safe) ──
async function confirmAction(message, title = 'Confirm') {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (dialogApi && dialogApi.ask) {
    return dialogApi.ask(message, { title, kind: 'warning' });
  }
  return Promise.resolve(confirm(message));
}

// ── Shared formatters (used across multiple modules) ──
function formatAudioSize(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function formatTime(sec) {
  if (!sec || !isFinite(sec)) return '0:00';
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return m + ':' + String(s).padStart(2, '0');
}

// ── Loading helpers ──
function showGlobalProgress() {
  document.getElementById('globalProgress')?.classList.add('active');
  // Refresh scan status badge immediately
  if (typeof updateHeaderInfo === 'function') setTimeout(updateHeaderInfo, 100);
}
function hideGlobalProgress() {
  document.getElementById('globalProgress')?.classList.remove('active');
}
// ── Persist filter dropdowns ──
const _filterIds = ['typeFilter', 'statusFilter', 'favTypeFilter', 'audioFormatFilter', 'dawDawFilter', 'presetFormatFilter'];

function saveFilterState(id) {
  const el = document.getElementById(id);
  if (!el) return;
  // Check for multi-filter (custom dropdown widget)
  const wrapper = el.nextElementSibling;
  if (wrapper && wrapper.classList.contains('multi-filter') && wrapper._selected) {
    const vals = [...wrapper._selected];
    if (vals.length > 0) {
      prefs.setItem('filter_' + id, vals);
    } else {
      prefs.removeItem('filter_' + id); // "all" = no pref needed
    }
  } else {
    if (el.value && el.value !== 'all') {
      prefs.setItem('filter_' + id, el.value);
    } else {
      prefs.removeItem('filter_' + id);
    }
  }
}

function restoreFilterStates() {
  for (const id of _filterIds) {
    let saved = prefs.getItem('filter_' + id);
    if (!saved || saved === 'all') continue;
    // Parse JSON array if stored as string
    if (typeof saved === 'string') {
      try { const parsed = JSON.parse(saved); if (Array.isArray(parsed)) saved = parsed; } catch {}
    }
    const el = document.getElementById(id);
    if (!el) continue;
    const wrapper = el.nextElementSibling;
    if (wrapper && wrapper.classList.contains('multi-filter') && typeof setMultiFilterValue === 'function') {
      if (Array.isArray(saved)) {
        if (wrapper._selected) wrapper._selected.clear();
        for (const v of saved) {
          setMultiFilterValue(id, v);
        }
        if (typeof updateMultiFilterLabel === 'function') {
          const allLabel = wrapper.querySelector('.multi-filter-item.multi-filter-all label')?.textContent?.trim() || 'All';
          updateMultiFilterLabel(wrapper, allLabel);
        }
      }
    } else if (typeof saved === 'string' && saved !== 'all') {
      el.value = saved;
    }
  }
  // Delay enabling saves until after initial data load completes
  setTimeout(() => { _filtersRestored = true; }, 3000);
}

// Save all filter states (called from filter functions)
let _filtersRestored = false;
function saveAllFilterStates() {
  if (!_filtersRestored) return; // don't overwrite prefs during initial load
  for (const id of _filterIds) {
    saveFilterState(id);
  }
}

// Auto-save on change
document.addEventListener('change', (e) => {
  if (e.target.closest('select.filter-select')) {
    saveAllFilterStates();
  }
});

function btnLoading(btn, loading) {
  if (!btn) return;
  if (loading) {
    btn.classList.add('btn-loading');
    btn.disabled = true;
  } else {
    btn.classList.remove('btn-loading');
    btn.disabled = false;
  }
}
function skeletonRows(container, count = 5) {
  container.innerHTML = Array.from({ length: count }, () =>
    `<div class="skeleton-row fade-in">
      <div class="skeleton skeleton-bar" style="flex: 2;"></div>
      <div class="skeleton skeleton-bar" style="flex: 1;"></div>
      <div class="skeleton skeleton-bar" style="width: 80px;"></div>
      <div class="skeleton skeleton-bar" style="width: 80px;"></div>
    </div>`
  ).join('');
}

// ── ETA calculator ──
function createETA() {
  let startTime = 0;
  return {
    start() { startTime = performance.now(); },
    estimate(processed, total) {
      if (!startTime || processed <= 0 || total <= 0) return '';
      const elapsed = (performance.now() - startTime) / 1000;
      const rate = processed / elapsed;
      const remaining = (total - processed) / rate;
      if (remaining < 1) return '< 1s';
      if (remaining < 60) return `~${Math.ceil(remaining)}s`;
      const mins = Math.floor(remaining / 60);
      const secs = Math.ceil(remaining % 60);
      return `~${mins}m ${secs}s`;
    },
    elapsed() {
      if (!startTime) return '';
      const secs = Math.floor((performance.now() - startTime) / 1000);
      if (secs < 60) return `${secs}s`;
      return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    }
  };
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
  let ghost = null;
  let placeholder = null;
  let dragStartX = 0;
  let offsetX = 0;
  let offsetY = 0;
  let isDragging = false;
  let didMove = false;

  nav.addEventListener('mousedown', (e) => {
    const btn = e.target.closest('.tab-btn');
    if (!btn || e.button !== 0) return;
    e.preventDefault();
    draggedTab = btn;
    dragStartX = e.clientX;
    const rect = btn.getBoundingClientRect();
    offsetX = e.clientX - rect.left;
    offsetY = e.clientY - rect.top;
    isDragging = false;
    didMove = false;
  });

  document.addEventListener('mousemove', (e) => {
    if (!draggedTab) return;
    if (!isDragging && Math.abs(e.clientX - dragStartX) > 5) {
      isDragging = true;
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'grabbing';

      // Create placeholder matching tab size
      const rect = draggedTab.getBoundingClientRect();
      placeholder = document.createElement('span');
      placeholder.className = 'tab-placeholder';
      placeholder.style.width = rect.width + 'px';
      placeholder.style.height = rect.height + 'px';
      draggedTab.parentNode.insertBefore(placeholder, draggedTab);

      // Create floating ghost
      ghost = document.createElement('span');
      ghost.className = 'tab-ghost';
      ghost.textContent = draggedTab.textContent.trim();
      ghost.style.left = (e.clientX - offsetX) + 'px';
      ghost.style.top = (e.clientY - offsetY) + 'px';
      document.body.appendChild(ghost);

      // Hide original
      draggedTab.classList.add('tab-dragging');
    }
    if (!isDragging || !ghost) return;
    didMove = true;

    // Move ghost with cursor
    ghost.style.left = (e.clientX - offsetX) + 'px';
    ghost.style.top = (e.clientY - offsetY) + 'px';

    // Find drop target
    ghost.style.display = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    ghost.style.display = '';
    const target = el?.closest('.tab-btn');

    nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
    if (target && target !== draggedTab && target !== placeholder) {
      const targetRect = target.getBoundingClientRect();
      const midX = targetRect.left + targetRect.width / 2;
      if (e.clientX < midX) {
        nav.insertBefore(placeholder, target);
      } else {
        nav.insertBefore(placeholder, target.nextSibling);
      }
    }
  });

  document.addEventListener('mouseup', (e) => {
    if (!draggedTab) return;

    if (isDragging) {
      nav.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-drag-over'));
      document.body.style.userSelect = '';
      document.body.style.cursor = '';

      // Move tab to placeholder position
      if (placeholder && placeholder.parentNode) {
        placeholder.parentNode.insertBefore(draggedTab, placeholder);
        placeholder.remove();
      }
      draggedTab.classList.remove('tab-dragging');
      if (ghost) { ghost.remove(); ghost = null; }
      placeholder = null;
      saveTabOrder();
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
  const defaultOrder = ['plugins', 'samples', 'daw', 'presets', 'favorites', 'notes', 'tags', 'files', 'history', 'settings'];
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
  document.getElementById('tabFavorites').classList.toggle('active', tab === 'favorites');
  document.getElementById('tabNotes').classList.toggle('active', tab === 'notes');
  document.getElementById('tabTags').classList.toggle('active', tab === 'tags');
  document.getElementById('tabFiles').classList.toggle('active', tab === 'files');
  document.getElementById('tabMidi')?.classList.toggle('active', tab === 'midi');
  document.getElementById('tabVisualizer')?.classList.toggle('active', tab === 'visualizer');
  if (tab === 'visualizer' && typeof startVisualizer === 'function') startVisualizer();
  document.getElementById('tabWalkers')?.classList.toggle('active', tab === 'walkers');
  document.getElementById('tabSettings').classList.toggle('active', tab === 'settings');
  if (tab === 'walkers' && typeof startWalkerPolling === 'function') startWalkerPolling();
  if (tab === 'history') loadHistory();
  if (tab === 'favorites') renderFavorites();
  if (tab === 'notes') renderNotesTab();
  if (tab === 'tags') renderTagsManager();
  if (tab === 'files') initFileBrowser();
  if (tab === 'midi' && typeof loadMidiFiles === 'function' && !_midiLoaded) loadMidiFiles();
  if (tab === 'settings') { refreshSettingsUI(); if (typeof renderCacheStats === 'function') renderCacheStats(); }
}
