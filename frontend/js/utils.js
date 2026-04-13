const _escDiv = document.createElement('div');

function escapeHtml(str) {
    _escDiv.textContent = str || '';
    return _escDiv.innerHTML;
}

/** `Commit:` + full SHA (when present) and `Commit date:` (authoritative commit timestamp) — no Version line. */
function formatBuildCommitDateLine(info) {
    if (!info || typeof info !== 'object') return '';
    const parts = [];
    const full = info.gitShaFull && String(info.gitShaFull).trim();
    if (full && full !== 'unknown') parts.push('Commit: ' + full);
    else if (info.gitShaShort && info.gitShaShort !== 'unknown') parts.push('Commit: ' + info.gitShaShort);
    if (info.gitCommitDate && String(info.gitCommitDate).length >= 10) {
        parts.push('Commit date: ' + String(info.gitCommitDate).slice(0, 10));
    }
    return parts.join(' · ');
}

/** `Version:` / `Commit:` / `Commit date:` with full git SHA when available (splash, tray, meta, window title). */
function formatBuildMetaLine(info) {
    if (!info || !info.version) return '';
    const head = 'Version: v' + String(info.version);
    const tail = formatBuildCommitDateLine(info);
    return tail ? head + ' · ' + tail : head;
}

/** Throttle: invoke at most once per `ms` milliseconds (trailing call guaranteed). */
function throttle(fn, ms) {
    let last = 0, timer = null;
    return function (...args) {
        const now = performance.now();
        const remaining = ms - (now - last);
        if (remaining <= 0) {
            if (timer) {
                clearTimeout(timer);
                timer = null;
            }
            last = now;
            fn.apply(this, args);
        } else if (!timer) {
            timer = setTimeout(() => {
                last = performance.now();
                timer = null;
                fn.apply(this, args);
            }, remaining);
        }
    };
}

/** Debounce: invoke after `ms` milliseconds of inactivity. */
/** Yield to the event loop so pending input/paint runs before a heavy synchronous chunk (e.g. table `innerHTML`). */
function yieldToBrowser() {
    return new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

/**
 * After `setFilterFieldLoading(true)`, yield until the WebView has had a chance to paint.
 * Release WKWebView often needs two animation frames plus a macrotask; a single `rAF` before
 * `invoke` can leave the filter spinner invisible for the whole query.
 */
function yieldForFilterFieldPaint() {
    return new Promise((resolve) => {
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                setTimeout(resolve, 0);
            });
        });
    });
}

function debounce(fn, ms) {
    let timer = null;
    return function (...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), ms);
    };
}

/**
 * Resolve a catalog key to the current locale string. No English fallbacks — if `appFmt`
 * is unavailable (e.g. VM tests), returns the key string. See `test/i18n-proof-contract.test.js`.
 */
function catalogFmt(key, vars) {
    if (typeof appFmt === 'function') return appFmt(key, vars);
    return key;
}

/**
 * Unit/abbreviation only (B, KB, s, m). When `appFmt` is absent, uses ASCII `asciiFallback`
 * so VM tests still show compact suffixes. Do not use for full phrases — use `catalogFmt`.
 */
function catalogFmtOrUnit(key, asciiFallback) {
    if (typeof appFmt === 'function') return appFmt(key);
    return asciiFallback;
}

/** Table column header label — uses `appFmt` when IPC strings are loaded. */
function appTableCol(key) {
    if (typeof appFmt !== 'function') return key;
    return appFmt(key);
}

/**
 * Label for in-table SQLite query spinner (filter / sort / load-more).
 * Shared across inventory tabs via `ui.js.query_loading`.
 */
function queryLoadingLabel() {
    if (typeof appFmt === 'function') return appFmt('ui.js.query_loading');
    return 'Loading…';
}

/**
 * Show a centered spinner row while a paginated `db_query_*` round-trip is in flight.
 * Pair with `yieldForFilterFieldPaint` (or `requestAnimationFrame`) before `await invoke` so the shell paints.
 */
function showTableQueryLoadingRow(opts) {
    const {
        tbodyId,
        rowId,
        tableId,
        colspan,
        append,
        label,
    } = opts;
    const tbody = document.getElementById(tbodyId);
    if (!tbody) return;
    const esc = typeof escapeHtml === 'function' ? escapeHtml : (x) => String(x);
    const safeLabel = esc(label || queryLoadingLabel());
    const old = document.getElementById(rowId);
    if (old) old.remove();
    const loadMore = tbody.querySelector('tr[id$="LoadMore"]');
    if (loadMore) loadMore.remove();
    const rowHtml = `<tr id="${rowId}"><td colspan="${colspan}" style="text-align:center;padding:28px 16px;"><div class="spinner" style="width:26px;height:26px;margin:0 auto 10px;"></div><span style="color:var(--text-muted);font-size:12px;">${safeLabel}</span></td></tr>`;
    if (append) {
        tbody.insertAdjacentHTML('beforeend', rowHtml);
    } else {
        tbody.innerHTML = rowHtml;
    }
    const tbl = tableId && document.getElementById(tableId);
    if (tbl) tbl.setAttribute('aria-busy', 'true');
}

function clearTableQueryLoadingRow(rowId, tableId) {
    const row = document.getElementById(rowId);
    if (row) row.remove();
    const tbl = tableId && document.getElementById(tableId);
    if (tbl) tbl.removeAttribute('aria-busy');
}

/**
 * Compact spinner inside `.search-box` while a `db_query_*` round-trip is in flight.
 * Call with `loading: true` before `await yieldForFilterFieldPaint()` then `await invoke`, and `false` in `finally` when `seq` matches
 * the active query id (same pattern as table loading rows).
 *
 * Positioning uses **inline styles** (not only CSS) so release WKWebView builds that omit or
 * mishandle `:has()` still place the spinner above the input. `z-index` keeps it visible over
 * the input’s opaque background.
 */
function setFilterFieldLoading(inputId, loading) {
    const input = document.getElementById(inputId);
    if (!input) return;
    const box = input.closest('.search-box');
    if (!box) return;
    const hasRegex = !!box.querySelector('.btn-regex');
    const rightPx = hasRegex ? 40 : 12;
    const padPx = hasRegex ? 72 : 48;
    if (loading) {
        box.classList.add('filter-field-loading');
        box.classList.toggle('filter-field-loading--regex', hasRegex);
        box.classList.toggle('filter-field-loading--plain', !hasRegex);
        input.setAttribute('aria-busy', 'true');
        let sp = box.querySelector('.filter-field-spinner');
        if (!sp) {
            sp = document.createElement('span');
            sp.className = 'filter-field-spinner';
            sp.setAttribute('aria-hidden', 'true');
            sp.title = typeof queryLoadingLabel === 'function' ? queryLoadingLabel() : 'Loading…';
            const regexBtn = box.querySelector('.btn-regex');
            if (regexBtn) regexBtn.insertAdjacentElement('beforebegin', sp);
            else input.insertAdjacentElement('afterend', sp);
        }
        sp.style.display = 'block';
        sp.style.position = 'absolute';
        sp.style.right = `${rightPx}px`;
        sp.style.top = '50%';
        sp.style.zIndex = '5';
        sp.style.width = '18px';
        sp.style.height = '18px';
        sp.style.pointerEvents = 'none';
        sp.style.boxSizing = 'border-box';
        sp.style.border = '2px solid var(--border)';
        sp.style.borderTopColor = 'var(--cyan)';
        sp.style.borderRadius = '50%';
        sp.style.animation = 'filter-field-spin 0.75s linear infinite';
        if (input.dataset._filterPadSaved == null) {
            input.dataset._filterPadSaved = input.style.paddingRight || '';
        }
        input.style.paddingRight = `${padPx}px`;
    } else {
        box.classList.remove('filter-field-loading', 'filter-field-loading--regex', 'filter-field-loading--plain');
        input.removeAttribute('aria-busy');
        if (input.dataset._filterPadSaved != null) {
            input.style.paddingRight = input.dataset._filterPadSaved;
            delete input.dataset._filterPadSaved;
        }
        box.querySelector('.filter-field-spinner')?.remove();
    }
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

const FZF_DEFAULTS = {
    SCORE_MATCH: 16,
    SCORE_GAP_START: -3,
    SCORE_GAP_EXTENSION: -1,
    BONUS_BOUNDARY: 9,
    BONUS_NON_WORD: 8,
    BONUS_CAMEL: 7,
    BONUS_CONSECUTIVE: 4,
    BONUS_FIRST_CHAR_MULT: 2
};

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
    if (typeof renderFzfSettings === 'function') renderFzfSettings();
}

function saveFzfParams() {
    prefs.setItem('fzfParams', {
        SCORE_MATCH,
        SCORE_GAP_START,
        SCORE_GAP_EXTENSION,
        BONUS_BOUNDARY,
        BONUS_NON_WORD,
        BONUS_CAMEL,
        BONUS_CONSECUTIVE,
        BONUS_FIRST_CHAR_MULT
    });
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
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.search_weights_reset'));
    }
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
    if (nLen === 0) return {score: 0, indices: []};
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
            if (!found) {
                valid = false;
                break;
            }
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
    return {score: bestScore, indices: bestIndices};
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
    if (text.startsWith('!')) {
        negate = true;
        text = text.slice(1);
    }
    if (text.startsWith("'") && text.endsWith("'") && text.length > 2) {
        type = 'exact';
        text = text.slice(1, -1);
    } else if (text.startsWith("'")) {
        type = 'exact';
        text = text.slice(1);
    } else if (text.startsWith('^')) {
        type = 'prefix';
        text = text.slice(1);
    } else if (text.endsWith('$')) {
        type = 'suffix';
        text = text.slice(0, -1);
    }
    return {type, text, negate};
}

// Score bonus for substring/exact matches over fuzzy-only
const SCORE_SUBSTRING_BONUS = 1000;
const SCORE_EXACT_BONUS = 2000;      // full string match
const SCORE_PREFIX_BONUS = 1500;

// Score a single token against a value. Returns score > 0 for match, 0 for no match.
function scoreToken(token, value) {
    const v = value.toLowerCase(), t = token.text.toLowerCase();
    switch (token.type) {
        case 'exact':
            return v.includes(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
        case 'prefix':
            return v.startsWith(t) ? SCORE_PREFIX_BONUS + t.length * SCORE_MATCH : 0;
        case 'suffix':
            return v.endsWith(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
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
            } catch {
                return [];
            }
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

function highlightWithIndices(text, indices) {
    if (!text) return '';
    if (!indices || indices.length === 0) return escapeHtml(text);
    const idxSet = new Set(indices);
    let result = '';
    let inMark = false;
    for (let i = 0; i < text.length; i++) {
        const ch = escapeHtml(text[i]);
        if (idxSet.has(i)) {
            if (!inMark) {
                result += '<mark class="fzf-hl">';
                inMark = true;
            }
            result += ch;
        } else {
            if (inMark) {
                result += '</mark>';
                inMark = false;
            }
            result += ch;
        }
    }
    if (inMark) result += '</mark>';
    return result;
}

// Highlight matched characters in text
function highlightMatch(text, query, mode) {
    if (!query || !text) return escapeHtml(text);
    return highlightWithIndices(text, getMatchIndices(query, text, mode));
}

/**
 * When the DB matches on full `path` (FTS) but the UI shows only the basename, match indices may
 * exist only on `path`. Map those indices onto `name` when basename equals `name`.
 */
function highlightBasenameFromPath(path, name, query, mode) {
    if (!query || !name) return escapeHtml(name);
    let idx = getMatchIndices(query, name, mode);
    if (idx.length) return highlightWithIndices(name, idx);
    if (!path) return escapeHtml(name);
    idx = getMatchIndices(query, path, mode);
    if (!idx.length) return escapeHtml(name);
    const basenameStart = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1;
    const pBase = path.slice(basenameStart);
    if (pBase.length !== name.length || pBase.toLowerCase() !== name.toLowerCase()) return escapeHtml(name);
    const mapped = idx.filter(i => i >= basenameStart && i < path.length).map(i => i - basenameStart);
    if (mapped.length === 0) return escapeHtml(name);
    return highlightWithIndices(name, mapped);
}

/**
 * Same idea for the directory column: matches may only appear in the full path prefix.
 */
function highlightPathPrefixFromPath(path, dirField, query, mode) {
    if (!query || !dirField) return escapeHtml(dirField);
    let idx = getMatchIndices(query, dirField, mode);
    if (idx.length) return highlightWithIndices(dirField, idx);
    if (!path) return escapeHtml(dirField);
    idx = getMatchIndices(query, path, mode);
    if (!idx.length) return escapeHtml(dirField);
    const nPath = path.replace(/\\/g, '/');
    const nDir = dirField.replace(/\\/g, '/');
    if (!nPath.startsWith(nDir)) return escapeHtml(dirField);
    const mapped = idx.filter(i => i < nDir.length);
    if (mapped.length === 0) return escapeHtml(dirField);
    return highlightWithIndices(dirField, mapped);
}

/**
 * Apply search highlights to a name cell during scan DOM-toggle filtering.
 * Preserves .row-badge spans while replacing the text portion with highlighted HTML.
 */
function applyScanCellHighlight(cell, originalText, search, mode, hlFn) {
    if (!cell) return;
    // Preserve badge spans
    const badges = cell.querySelectorAll('.row-badge');
    const badgeHtml = Array.from(badges).map(b => b.outerHTML).join('');
    cell.innerHTML = (search ? hlFn(originalText, search, mode) : escapeHtml(originalText)) + badgeHtml;
}

// Extension-to-dropdown value mapping for auto-select
const EXT_TO_FILTER = {
    // Audio formats (`AUDIO_EXTENSIONS` in `src-tauri/src/audio_extensions.rs`)
    'wav': 'WAV', 'mp3': 'MP3', 'aiff': 'AIFF', 'aif': 'AIF',
    'flac': 'FLAC', 'ogg': 'OGG', 'm4a': 'M4A', 'aac': 'AAC',
    'opus': 'OPUS', 'wma': 'WMA', 'rex': 'REX', 'rx2': 'RX2', 'sf2': 'SF2', 'sfz': 'SFZ',
    // Plugin types
    'vst2': 'VST2', 'vst3': 'VST3', 'au': 'Audio Units', 'component': 'Audio Units', 'clap': 'CLAP',
    // DAW formats → DAW names
    'als': 'Ableton Live', 'alp': 'Ableton Live', 'ableton': 'Ableton Live',
    'logicx': 'Logic Pro', 'logic': 'Logic Pro',
    'flp': 'FL Studio', 'fl': 'FL Studio',
    'cpr': 'Cubase', 'cubase': 'Cubase',
    'npr': 'Nuendo', 'nuendo': 'Nuendo',
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

// ── Unified Filter System ──
// Single filter implementation for all tabs. Register once, use everywhere.
const _filterRegistry = {};
let _filterDebounceTimers = {};

function registerFilter(action, config) {
    // config: { inputId, regexToggleId, formatDropdownId, resetOffset, fetchFn, clientFilter, debounceMs? }
    _filterRegistry[action] = config;
}

function getSearchMode(toggleId) {
    const btn = document.getElementById(toggleId);
    return btn && btn.classList.contains('active') ? 'regex' : 'fuzzy';
}

function applyFilter(action) {
    const cfg = _filterRegistry[action];
    if (!cfg) return;
    if (typeof saveAllFilterStates === 'function') saveAllFilterStates();
    const input = document.getElementById(cfg.inputId);
    const search = input ? input.value.trim() : '';
    const mode = cfg.regexToggleId ? getSearchMode(cfg.regexToggleId) : 'fuzzy';
    cfg.lastSearch = search;
    cfg.lastMode = mode;
    if (cfg.resetOffset) cfg.resetOffset();
    // Bind cfg so fetchFn can read this.lastSearch / this.lastMode (set above).
    if (cfg.fetchFn) cfg.fetchFn.call(cfg);
}

function applyFilterDebounced(action) {
    clearTimeout(_filterDebounceTimers[action]);
    const cfg = _filterRegistry[action];
    const ms = cfg && typeof cfg.debounceMs === 'number' ? cfg.debounceMs : 250;
    _filterDebounceTimers[action] = setTimeout(() => applyFilter(action), ms);
}

/**
 * Throttled scheduler used by every streaming scan's flush loop. Coalesces
 * rapid scan-progress events into at-most one flush every `intervalMs` ms,
 * aligned to a `requestAnimationFrame` so DOM writes land inside a browser
 * paint tick. Returns the scheduler function — call it from every scan
 * event; it is idempotent if a flush is already pending.
 *
 *   const scheduleFlush = createScanFlusher(flushPendingMidi, 100);
 *   onScanProgress(...) { pending.push(...batch); scheduleFlush(); }
 */
function createScanFlusher(flushFn, intervalMs) {
    let scheduled = false;
    let last = 0;
    return function schedule() {
        if (scheduled) return;
        scheduled = true;
        const elapsed = performance.now() - last;
        const delay = Math.max(0, intervalMs - elapsed);
        setTimeout(() => requestAnimationFrame(() => {
            scheduled = false;
            try {
                flushFn();
            } finally {
                last = performance.now();
            }
        }), delay);
    };
}

/** Loading / load-more rows — not inventory data. */
const SCAN_TABLE_IGNORE_ROW_IDS = new Set([
    'audioQueryLoadingRow', 'audioLoadMore', 'dawLoadMore', 'presetLoadMore', 'midiLoadMore', 'pdfLoadMore',
]);

/** True if tbody has no data rows (missing tbody counts as empty). */
function scanTableTbodyIsEmpty(tbodyId) {
    const tbody = document.getElementById(tbodyId);
    if (!tbody) return true;
    for (const tr of tbody.querySelectorAll('tr')) {
        if (SCAN_TABLE_IGNORE_ROW_IDS.has(tr.id)) continue;
        return false;
    }
    return true;
}

function isAudioScanTableEmpty() {
    const wrap = document.getElementById('audioTableWrap');
    if (!wrap) return true;
    if (!wrap.querySelector('#audioTable')) return true;
    return scanTableTbodyIsEmpty('audioTableBody');
}

function isPluginListEmptyForScan() {
    const list = document.getElementById('pluginList');
    if (!list) return true;
    return list.querySelectorAll('.plugin-card').length === 0;
}

function isDawScanTableEmpty() {
    const wrap = document.getElementById('dawTableWrap');
    if (!wrap) return true;
    if (!wrap.querySelector('#dawTable')) return true;
    return scanTableTbodyIsEmpty('dawTableBody');
}

function isPresetScanTableEmpty() {
    const wrap = document.getElementById('presetTableWrap');
    if (!wrap) return true;
    if (!wrap.querySelector('#presetTable')) return true;
    return scanTableTbodyIsEmpty('presetTableBody');
}

function isPdfScanTableEmpty() {
    const wrap = document.getElementById('pdfTableWrap');
    if (!wrap) return true;
    if (!wrap.querySelector('#pdfTable')) return true;
    return scanTableTbodyIsEmpty('pdfTableBody');
}

function isMidiScanTableEmpty() {
    const wrap = document.getElementById('midiTableWrap');
    if (!wrap) return true;
    if (!wrap.querySelector('#midiTable')) return true;
    return scanTableTbodyIsEmpty('midiTableBody');
}

function toggleRegex(btn) {
    btn.classList.toggle('active');
    const input = btn.closest('.search-box')?.querySelector('input');
    if (input && typeof window.applyI18nPlaceholders === 'function') {
        window.applyI18nPlaceholders();
    }
    const action = btn.dataset.target;
    if (action) applyFilter(action);
}

// ── Confirm dialog (in-app modal matches theme; falls back to Tauri ask / window.confirm) ──

/** @returns {string} */
function _confirmBtnLabel(key, english) {
    const s = catalogFmt(key);
    return s === key ? english : s;
}

/**
 * Themed confirm modal (same `.modal-overlay` / `.modal-content` as export & notes).
 * @param {string} message
 * @param {string} [title]
 * @returns {Promise<boolean>}
 */
function showAppConfirmModal(message, title) {
    return new Promise((resolve) => {
        document.getElementById('appConfirmModal')?.remove();

        const t = title || catalogFmt('confirm.dialog_title');
        const yesL = _confirmBtnLabel('confirm.btn_yes', 'Yes');
        const noL = _confirmBtnLabel('confirm.btn_no', 'No');
        const msgHtml = escapeHtml(message);
        const html = `<div class="modal-overlay" id="appConfirmModal" role="alertdialog" aria-modal="true" aria-labelledby="appConfirmTitle" aria-describedby="appConfirmMsg">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2 id="appConfirmTitle">${escapeHtml(t)}</h2>
        <button type="button" class="modal-close" data-app-confirm="cancel" title="${escapeHtml(noL)}" aria-label="${escapeHtml(noL)}">&#10005;</button>
      </div>
      <div class="modal-body">
        <p id="appConfirmMsg" class="app-confirm-message">${msgHtml}</p>
        <div class="export-actions app-confirm-actions">
          <button type="button" class="btn btn-secondary" data-app-confirm="cancel">${escapeHtml(noL)}</button>
          <button type="button" class="btn btn-primary" data-app-confirm="ok">${escapeHtml(yesL)}</button>
        </div>
      </div>
    </div>
  </div>`;
        document.body.insertAdjacentHTML('beforeend', html);
        const modal = document.getElementById('appConfirmModal');
        if (!modal) {
            resolve(false);
            return;
        }

        let settled = false;
        const finish = (v) => {
            if (settled) return;
            settled = true;
            document.removeEventListener('keydown', keyHandler, true);
            modal.remove();
            resolve(v);
        };

        const keyHandler = (e) => {
            if (e.key !== 'Escape') return;
            e.preventDefault();
            e.stopImmediatePropagation();
            finish(false);
        };
        document.addEventListener('keydown', keyHandler, true);

        modal.addEventListener('click', (e) => {
            const btn = e.target.closest('[data-app-confirm]');
            if (btn) {
                finish(btn.dataset.appConfirm === 'ok');
                return;
            }
            if (e.target === modal) finish(false);
        });

        const noBtn = modal.querySelector('[data-app-confirm="cancel"]');
        requestAnimationFrame(() => (noBtn || modal.querySelector('button'))?.focus());
    });
}

async function confirmAction(message, title) {
    if (typeof document !== 'undefined' && document.body) {
        return showAppConfirmModal(String(message ?? ''), title);
    }
    const t = title || catalogFmt('confirm.dialog_title');
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (dialogApi && dialogApi.ask) {
        return dialogApi.ask(message, {title: t, kind: 'warning'});
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

/**
 * Command palette + native Tools menu — status-bar bg jobs plus KVR plugin update check
 * (`checkUpdates` / `stopUpdates`). KVR URL resolve uses `stopKvrResolve`.
 */
function triggerStartAllBackgroundJobs() {
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.start_all_background_jobs'));
    }
    if (typeof triggerBackgroundBpmKeyLufsAnalysis === 'function') void triggerBackgroundBpmKeyLufsAnalysis();
    if (typeof triggerStartBackgroundContentDupScan === 'function') void triggerStartBackgroundContentDupScan();
    if (typeof buildPdfPagesCache === 'function') buildPdfPagesCache();
    if (typeof triggerStartFingerprintCacheBuild === 'function') void triggerStartFingerprintCacheBuild();
    if (typeof checkUpdates === 'function') void checkUpdates();
}

function triggerStopAllBackgroundJobs() {
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.stop_all_background_jobs'));
    }
    if (typeof triggerStopBackgroundBpmKeyLufsAnalysis === 'function') triggerStopBackgroundBpmKeyLufsAnalysis();
    if (typeof triggerStopBackgroundContentDupScan === 'function') triggerStopBackgroundContentDupScan();
    if (typeof stopPdfScan === 'function') void stopPdfScan();
    if (typeof stopPdfMetadataExtractionUser === 'function') void stopPdfMetadataExtractionUser();
    const vu = window.vstUpdater;
    if (vu && typeof vu.stopFingerprintCache === 'function') void vu.stopFingerprintCache();
    if (vu && typeof vu.stopUpdates === 'function') void vu.stopUpdates();
    if (typeof stopKvrResolve === 'function') stopKvrResolve();
}

// ── Persist filter dropdowns ──
const _filterIds = ['typeFilter', 'statusFilter', 'favTypeFilter', 'audioFormatFilter', 'dawDawFilter', 'presetFormatFilter', 'videoFormatFilter'];

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
            try {
                const parsed = JSON.parse(saved);
                if (Array.isArray(parsed)) saved = parsed;
            } catch (e) {
                if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
            }
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
    setTimeout(() => {
        _filtersRestored = true;
    }, 3000);
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
    container.innerHTML = Array.from({length: count}, () =>
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
        start() {
            startTime = performance.now();
        },
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
            .map(([t, c]) => `<span class="plugin-type ${t === 'VST2' ? 'type-vst2' : t === 'VST3' ? 'type-vst3' : t === 'CLAP' ? 'type-clap' : 'type-au'}">${t}: ${c}</span>`)
            .join(' ');
        return `<tr>
      <td style="padding: 4px 8px 4px 0; color: var(--cyan); opacity: 0.7;">${dir}</td>
      <td style="padding: 4px 8px; text-align: right; font-family: Orbitron, sans-serif; color: var(--text);">${count}</td>
      <td style="padding: 4px 0 4px 8px;">${typeStr}</td>
    </tr>`;
    });
    const tc = typeof appTableCol === 'function' ? appTableCol : (k) => k;
    return `<table style="width: 100%; border-collapse: collapse; margin-top: 6px;">
    <tr style="color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: 1px;">
      <th style="text-align: left; padding: 2px 8px 2px 0;">${tc('ui.col.directory')}</th>
      <th style="text-align: right; padding: 2px 8px;">${tc('ui.col.plugins')}</th>
      <th style="text-align: left; padding: 2px 0 2px 8px;">${tc('ui.col.types')}</th>
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
            if (ghost) {
                ghost.remove();
                ghost = null;
            }
            placeholder = null;
            saveTabOrder();
        }

        // Suppress the click that follows mouseup if we actually dragged
        if (didMove) {
            const suppress = (ev) => {
                ev.stopPropagation();
                ev.preventDefault();
            };
            nav.addEventListener('click', suppress, {capture: true, once: true});
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
    invalidateTabButtonCache();
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
        tabs.forEach(btn => {
            tabMap[btn.dataset.tab] = btn;
        });
        // Re-append in saved order, skip any missing
        for (const key of order) {
            if (tabMap[key]) nav.appendChild(tabMap[key]);
        }
        // Append any tabs not in saved order (new tabs)
        tabs.forEach(btn => {
            if (!order.includes(btn.dataset.tab)) nav.appendChild(btn);
        });
    } catch (e) {
        if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
    }
    invalidateTabButtonCache();
}

function settingResetTabOrder() {
    prefs.removeItem('tabOrder');
    const nav = document.querySelector('.tab-nav');
    const defaultOrder = ['plugins', 'samples', 'daw', 'presets', 'favorites', 'notes', 'tags', 'files', 'history', 'audioEngine', 'settings'];
    const tabMap = {};
    nav.querySelectorAll('.tab-btn').forEach(btn => {
        tabMap[btn.dataset.tab] = btn;
    });
    for (const key of defaultOrder) {
        if (tabMap[key]) nav.appendChild(tabMap[key]);
    }
    showToast(toastFmt('toast.tab_order_reset'));
    invalidateTabButtonCache();
}

// ── Tab switching ──
// Cache tab strip buttons once — same idea as _tabPanels; invalidate when order changes.
let _tabButtonsCache = null;

function invalidateTabButtonCache() {
    _tabButtonsCache = null;
}

function _ensureTabButtonCache() {
    if (_tabButtonsCache != null) return;
    const nav = document.querySelector('.tab-nav');
    _tabButtonsCache = nav ? [...nav.querySelectorAll('.tab-btn')] : [];
}

// Cache tab panel elements once — avoids N getElementById calls per switch.
const _tabPanels = {};
const _tabPanelIds = [
    'plugins', 'history', 'samples', 'daw', 'presets', 'favorites',
    'notes', 'tags', 'files', 'midi', 'pdf', 'videos', 'visualizer', 'walkers', 'audioEngine', 'settings',
];

function _ensureTabCache() {
    if (_tabPanels._ready) return;
    for (const t of _tabPanelIds) {
        const id = 'tab' + t.charAt(0).toUpperCase() + t.slice(1);
        _tabPanels[t] = document.getElementById(id);
    }
    _tabPanels._ready = true;
}

function switchTab(tab) {
    _ensureTabCache();
    _ensureTabButtonCache();
    // Do not call `closeVideoMetaRow` when leaving Videos — that runs `stopVideoPlayback()` and kills
    // engine/HTML5 audio. The meta row stays in the (hidden) panel until the user closes it or previews another file.
    // Batch selection is stored per inventory tab (`batchByTab` in batch-select.js); clear on switch so counts/actions never mix tables.
    if (typeof deselectAll === 'function') deselectAll();
    // Toggle tab buttons + panels in one pass — pure class mutations, no layout reads.
    for (const b of _tabButtonsCache) {
        b.classList.toggle('active', b.dataset.tab === tab);
    }
    const activeStripBtn = _tabButtonsCache.find((b) => b.dataset.tab === tab);
    if (activeStripBtn && typeof activeStripBtn.scrollIntoView === 'function') {
        activeStripBtn.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
    for (const t of _tabPanelIds) {
        _tabPanels[t]?.classList.toggle('active', t === tab);
    }
    prefs.setItem('activeTab', tab);
    // Two rAF frames: first lets the active tab strip + panel visibility update paint;
    // then yield one macrotask so the shell can process input before History IPC / large DOM
    // (otherwise a quick tab switch can stack work from an older switch after `await`).
    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            const expectedTab = tab;
            function tabWorkStillRelevant() {
                try {
                    return typeof prefs !== 'undefined' && prefs.getItem('activeTab') === expectedTab;
                } catch {
                    return true;
                }
            }
            function runPerTabWork() {
                if (!tabWorkStillRelevant()) return;
                /* Real-time graphs: inactive tab panels use `display:none` (see `.tab-content`), but
                 * canvas rAF loops must stop here too — Audio Engine diagnostics (`aeGraph*`) and the
                 * Visualizer grid both use `requestAnimationFrame`. Window minimize / hidden WebView is
                 * handled separately via `ui-idle-heavy-cpu` (`audio-engine.js`, `visualizer.js`, `audio.js`). */
                if (expectedTab !== 'audioEngine' && typeof window.stopAeTabMeterAndGraph === 'function') {
                    window.stopAeTabMeterAndGraph();
                }
                if (expectedTab !== 'visualizer' && typeof stopVisualizer === 'function') stopVisualizer();
                if (expectedTab === 'visualizer' && typeof startVisualizer === 'function') startVisualizer();
                if (expectedTab === 'walkers' && typeof startWalkerPolling === 'function') startWalkerPolling();
                if (expectedTab === 'audioEngine' && typeof initAudioEngineTab === 'function') initAudioEngineTab();
                if (expectedTab === 'history' && typeof loadHistory === 'function') loadHistory({preferCache: true});
                if (expectedTab === 'favorites' && typeof _favHasRendered !== 'undefined' && (!_favHasRendered || _favDirty)) renderFavorites();
                if (expectedTab === 'notes') renderNotesTab();
                if (expectedTab === 'tags' && typeof _tagsHasRendered !== 'undefined' && (!_tagsHasRendered || _tagsDirty)) renderTagsManager();
                if (expectedTab === 'files') initFileBrowser();
                if (expectedTab === 'midi' && typeof loadMidiFiles === 'function' && typeof _midiLoaded !== 'undefined' && !_midiLoaded) loadMidiFiles();
                if (expectedTab === 'pdf' && typeof loadPdfPagesForVisible === 'function') void loadPdfPagesForVisible();
                if (expectedTab === 'videos' && typeof loadVideoFiles === 'function' && typeof _videoLoaded !== 'undefined' && !_videoLoaded) void loadVideoFiles();
                if (expectedTab === 'settings') {
                    refreshSettingsUI();
                    if (typeof renderCacheStats === 'function') renderCacheStats();
                    // Force reflow so system-info and databases/caches panes resolve
                    // their layout in release WebView (without this, columns and
                    // percentage-based widths can render at 0 until a scroll/resize).
                    const settingsPanel = document.getElementById('tabSettings') || document.querySelector('.tab-content.active');
                    if (settingsPanel) void settingsPanel.offsetWidth;
                }
            }
            if (typeof yieldToBrowser === 'function') {
                void yieldToBrowser().then(runPerTabWork);
            } else {
                setTimeout(runPerTabWork, 0);
            }
        });
    });
}

// ── O(1) lookup by path for large arrays ──
// Maintains a per-array path→item index in a WeakMap, incrementally built as
// items are appended. Callers that swap the array (e.g. `allAudioSamples = [...]`)
// get a fresh index automatically because WeakMap key is the array instance.
// Callers that truncate/replace items in-place (rare) must signal via
// `findByPath(arr, path, /*reindex*/ true)`.
//
// This is critical for scans with millions of items where linear `.find()` on
// user-interactive paths (play selection, context menu, favorites, xref) would
// cost 10–100ms per call.
const _pathIndexCache = new WeakMap();

function findByPath(arr, path, reindex) {
    if (!arr || !path) return undefined;
    let entry = _pathIndexCache.get(arr);
    // Rebuild when absent, after truncation, or when caller forces reindex.
    if (!entry || reindex || arr.length < entry.indexedUpTo) {
        entry = {map: new Map(), indexedUpTo: 0};
        _pathIndexCache.set(arr, entry);
    }
    // Incrementally index newly-appended items.
    for (let i = entry.indexedUpTo; i < arr.length; i++) {
        const item = arr[i];
        if (item && item.path) entry.map.set(item.path, item);
    }
    entry.indexedUpTo = arr.length;
    return entry.map.get(path);
}

/**
 * In-app toast when a scan finishes successfully (skipped when `window.__suppressPostScanToasts`
 * is true — used by Scan All so only one summary toast appears).
 * @param {boolean} stopped - User cancelled; uses `stoppedKey` and warning styling.
 * @param {string} completeKey - `toastFmt` key
 * @param {string} stoppedKey - `toastFmt` key
 * @param {Record<string, string|number>} vars - Placeholders for `toastFmt`
 */
function postScanCompleteToast(stopped, completeKey, stoppedKey, vars) {
    if (typeof window.__suppressPostScanToasts === 'boolean' && window.__suppressPostScanToasts) return;
    if (typeof showToast !== 'function' || typeof toastFmt !== 'function') return;
    const key = stopped ? stoppedKey : completeKey;
    showToast(toastFmt(key, vars), stopped ? 4500 : 3500, stopped ? 'warning' : '');
}

window.postScanCompleteToast = postScanCompleteToast;

/**
 * Paths for fingerprint cache build: SQLite `audio_library` (full library), not the in-memory
 * paginated `allAudioSamples` slice. Falls back to `allAudioSamples` if IPC is unavailable.
 */
async function fetchAudioLibraryPathsForFingerprint() {
    const vu = window.vstUpdater;
    if (vu && typeof vu.dbAudioLibraryPaths === 'function') {
        try {
            const paths = await vu.dbAudioLibraryPaths();
            if (Array.isArray(paths) && paths.length > 0) return paths;
        } catch {
            /* fall through */
        }
    }
    const arr = typeof allAudioSamples !== 'undefined' && Array.isArray(allAudioSamples) ? allAudioSamples : [];
    return arr.map((s) => s && s.path).filter(Boolean);
}

/** Label keys for `ui.stats.bg_job_badge_fmt` (status-bar background job badges). */
const BG_JOB_BADGE_KIND_TO_LABEL_KEY = {
    bpm: 'ui.stats.bg_job_type_bpm',
    contentDup: 'ui.stats.bg_job_type_content_dup',
    pdfMeta: 'ui.stats.bg_job_type_pdf_meta',
    pdfScan: 'ui.stats.bg_job_type_pdf_scan',
    fingerprint: 'ui.stats.bg_job_type_fingerprint',
};

/**
 * @param {'bpm'|'contentDup'|'pdfMeta'|'pdfScan'|'fingerprint'} kind
 * @param {string} detailCatalogKey
 * @param {Record<string, unknown>} [vars]
 */
function formatBgJobBadgeLine(kind, detailCatalogKey, vars) {
    const labelKey = BG_JOB_BADGE_KIND_TO_LABEL_KEY[kind];
    const detail = typeof catalogFmt === 'function' ? catalogFmt(detailCatalogKey, vars) : detailCatalogKey;
    if (!labelKey) return detail;
    const job = typeof catalogFmt === 'function' ? catalogFmt(labelKey) : kind;
    if (typeof catalogFmt === 'function') {
        return catalogFmt('ui.stats.bg_job_badge_fmt', { job, detail });
    }
    return `${job}: ${detail}`;
}

const STATUS_BG_JOB_BADGE_IDS = ['bgAnalysisBadge', 'bgContentDupBadge', 'bgPdfMetaBadge', 'bgPdfScanBadge', 'bgFingerprintBadge'];

/** Show/hide each `.status-bg-job-row` from job flags + badge text (spinners live in the row). */
function syncStatusBgJobRows() {
    for (const id of STATUS_BG_JOB_BADGE_IDS) {
        const badge = document.getElementById(id);
        if (!badge) continue;
        const row = badge.parentElement;
        if (!row || !row.classList.contains('status-bg-job-row')) continue;
        let jobFlag = false;
        if (typeof window !== 'undefined') {
            if (id === 'bgAnalysisBadge') jobFlag = !!window.__statusBarBpmJob;
            else if (id === 'bgPdfMetaBadge') jobFlag = !!window.__statusBarPdfMetaJob;
            else if (id === 'bgContentDupBadge') jobFlag = !!window.__statusBarContentDupJob;
            else if (id === 'bgPdfScanBadge') jobFlag = !!window.__statusBarPdfScanJob;
            else if (id === 'bgFingerprintBadge') jobFlag = !!window.__statusBarFingerprintJob;
        }
        const hasText = !!(badge.textContent && String(badge.textContent).trim());
        row.style.display = jobFlag || hasText ? 'flex' : 'none';
    }
}

/** True after `syncStatusBgJobRows()` if any `.status-bg-job-row` is shown (inline `display: flex`). */
function statusBarHasVisibleJobRow() {
    for (const id of STATUS_BG_JOB_BADGE_IDS) {
        const badge = document.getElementById(id);
        const row = badge && badge.parentElement;
        if (!row || !row.classList.contains('status-bg-job-row')) continue;
        if (row.style.display === 'flex') return true;
    }
    return false;
}

/** Bottom `#statusBar`: visible only while KVR/update check is running (`.active`) or at least one bg job row is open. */
function syncAppStatusBarVisibility() {
    try {
        syncStatusBgJobRows();
        const bar = document.getElementById('statusBar');
        if (!bar) return;
        const show = bar.classList.contains('active') || statusBarHasVisibleJobRow();
        bar.classList.toggle('status-bar-visible', show);
    } catch {
        /* ignore */
    }
}

/** Join a directory path and file name using the path style already present in `dir`. */
function joinPathDirFile(dir, file) {
    const d = String(dir || '').replace(/[/\\]+$/, '');
    if (!d) return String(file || '');
    const sep = d.indexOf('\\') >= 0 ? '\\' : '/';
    return d + sep + String(file || '');
}

/**
 * Export a canvas as PNG: writes to `snapshotExportDir` (prefs) via Tauri when set, else `<a download>`.
 * @param {HTMLCanvasElement|null} canvas
 * @param {string} labelForFilename Human-readable label → slug in filename
 */
async function exportCanvasSnapshotPng(canvas, labelForFilename) {
    if (!canvas) return;
    const raw = String(labelForFilename || 'snapshot').trim() || 'snapshot';
    const slug = raw
        .replace(/\s+/g, '_')
        .toLowerCase()
        .replace(/[^a-z0-9._-]+/g, '_')
        .replace(/_+/g, '_')
        .replace(/^_|_$/g, '') || 'snapshot';
    const filename = `${slug}_${Date.now()}.png`;
    let dir =
        typeof prefs !== 'undefined' && prefs.getItem
            ? String(prefs.getItem('snapshotExportDir') || '').trim()
            : '';
    const dataUrl = canvas.toDataURL('image/png');
    const tryTauriWrite = async () => {
        const invoke = window.__TAURI__?.core?.invoke;
        if (!invoke) return false;
        if (!dir) {
            try {
                dir =
                    typeof window.vstUpdater?.ensureSnapshotExportDir === 'function'
                        ? await window.vstUpdater.ensureSnapshotExportDir()
                        : await invoke('ensure_snapshot_export_dir');
            } catch {
                return false;
            }
        }
        if (!dir) return false;
        const fullPath = joinPathDirFile(dir, filename);
        const b64 = dataUrl.split(',')[1];
        if (!b64) return false;
        const bin = atob(b64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        await invoke('write_binary_file', {filePath: fullPath, contents: Array.from(bytes)});
        return true;
    };
    try {
        if (await tryTauriWrite()) {
            if (typeof showToast === 'function') showToast(toastFmt('toast.snapshot_exported'));
            return;
        }
    } catch (err) {
        if (typeof showToast === 'function') {
            showToast(String(err && err.message ? err.message : err), 4000, 'error');
        }
    }
    const link = document.createElement('a');
    link.download = filename;
    link.href = dataUrl;
    link.click();
    if (typeof showToast === 'function') showToast(toastFmt('toast.snapshot_exported'));
}

if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
    });
}
