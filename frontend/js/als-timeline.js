// ──────────────────────────────────────────────────────────────────────────
// ALS Generator — Section Overrides Timeline Editor (Ableton-arrangement style)
//
// Six stacked lanes (Chaos, Glitch, Density, Variation, Parallelism, Scatter),
// subdivided into 8-bar blocks that match the Ableton phrase grid. Each block
// is an independently-pinnable override cell — drag the top edge to set value
// by height, scroll to fine-tune, right-click to clear.
//
// Model (2026-04 refactor):
//   _overrides = {
//     chaos:   { "1": 0.5, "9": 0.3, "65": 0.8, ... },   // bar-start → float 0..1
//     glitch:  { ... }, density: { ... }, variation: { ... }, parallelism: { ... }, scatter: { ... }
//   }
//   Keys are strings (JSON object-key rule) holding the starting bar of an
//   8-bar block (1, 9, 17, 25, …). A missing key means "use global scalar".
//
// The Rust side (`als_project::SectionValues`) uses the same flat shape as a
// `BTreeMap<String, f32>` with `#[serde(transparent)]`, so the payload flows
// straight through without field remapping.
//
// Block count per genre (since section sizes vary):
//   Techno  (224 bars): 28 blocks — 7 sections × 4 blocks of 8 bars
//   Trance  (256 bars): 32 blocks — 48-bar breakdown/outro = 6 blocks each
//   Schranz (208 bars): 26 blocks — 16-bar breakdown/outro = 2 blocks each
//
// Section names remain as visual labels only — heavier vertical dividers show
// where sections begin so the user can navigate the arrangement visually while
// editing at block resolution.
//
// Persisted to prefs under `alsSectionOverrides`. Migrates legacy section-name
// keys ("intro", "build", …) on first load by fanning out each value to every
// 8-bar block inside that section for the current genre.
// ──────────────────────────────────────────────────────────────────────────

(function () {
    'use strict';

    const PARAMS = ['chaos', 'glitch', 'density', 'variation', 'parallelism', 'scatter'];
    const PARAM_LABELS = {
        chaos: 'CHAOS',
        glitch: 'GLITCH',
        density: 'DENSITY',
        variation: 'VARIATION',
        parallelism: 'PARALLELISM',
        scatter: 'SCATTER',
    };
    const SECTIONS = ['intro', 'build', 'breakdown', 'drop1', 'drop2', 'fadedown', 'outro'];
    const SECTION_LABELS = {
        intro: 'INTRO',
        build: 'BUILD',
        breakdown: 'BREAKDOWN',
        drop1: 'DROP 1',
        drop2: 'DROP 2',
        fadedown: 'FADEDOWN',
        outro: 'OUTRO',
    };
    const BLOCK_BARS = 8;

    // Bar ranges per genre — MUST match src-tauri/src/als_project.rs::SectionBounds.
    // Inclusive lo, exclusive hi.
    const GENRE_SECTION_BARS = {
        techno:  { intro: [1, 33],  build: [33, 65],  breakdown: [65, 97],   drop1: [97, 129],  drop2: [129, 161], fadedown: [161, 193], outro: [193, 225]  },
        trance:  { intro: [1, 33],  build: [33, 65],  breakdown: [65, 113],  drop1: [113, 145], drop2: [145, 177], fadedown: [177, 209], outro: [209, 257]  },
        schranz: { intro: [1, 33],  build: [33, 65],  breakdown: [65, 81],   drop1: [81, 113],  drop2: [113, 161], fadedown: [161, 193], outro: [193, 209]  },
    };

    // Per-lane color (matches existing cyberpunk palette used elsewhere).
    const LANE_COLORS = {
        chaos:       { fill: 'rgba(255, 42, 109, 0.45)',  stroke: '#ff2a6d' },   // accent pink
        glitch:      { fill: 'rgba(211, 0, 197, 0.45)',   stroke: '#d300c5' },   // magenta
        density:     { fill: 'rgba(249, 240, 2, 0.35)',   stroke: '#f9f002' },   // yellow
        variation:   { fill: 'rgba(57, 255, 20, 0.35)',   stroke: '#39ff14' },   // green
        parallelism: { fill: 'rgba(5, 217, 232, 0.45)',   stroke: '#05d9e8' },   // cyan
        scatter:     { fill: 'rgba(255, 165, 0, 0.45)',   stroke: '#ffa500' },   // orange
    };

    // ── Mutable state ──────────────────────────────────────────────────────
    let _overrides = { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} };
    let _selected = null;   // { param: string, blockKey: string (bar start), section: string }
    let _drag = null;       // { mode: 'value', param, blockKey, laneTop, laneH }
    let _ro = null;         // ResizeObserver

    function getGenre() {
        const el = document.getElementById('alsGenre');
        return (el && GENRE_SECTION_BARS[el.value]) ? el.value : 'techno';
    }

    function sectionBars(genre) {
        return GENRE_SECTION_BARS[genre] || GENRE_SECTION_BARS.techno;
    }

    // Enumerate every 8-bar block across the arrangement for this genre.
    // Each entry carries its starting bar (1-indexed), exclusive end bar, and
    // which section it belongs to (for visual labeling + popover context).
    function blocksForGenre(genre) {
        const bars = sectionBars(genre);
        const out = [];
        for (const name of SECTIONS) {
            const [lo, hi] = bars[name];
            for (let b = lo; b < hi; b += BLOCK_BARS) {
                out.push({
                    startBar: b,
                    endBar: Math.min(b + BLOCK_BARS, hi),
                    section: name,
                    key: String(b),
                });
            }
        }
        return out;
    }

    // Layout computes pixel geometry for the current canvas size + genre.
    // Stable for one paint/hit-test cycle; recomputed on every render/event.
    function layout(canvas) {
        const dpr = window.devicePixelRatio || 1;
        const cssW = canvas.clientWidth || 600;
        const cssH = canvas.clientHeight || 220;
        // Resize backing store if needed
        if (canvas.width !== Math.round(cssW * dpr) || canvas.height !== Math.round(cssH * dpr)) {
            canvas.width = Math.round(cssW * dpr);
            canvas.height = Math.round(cssH * dpr);
        }
        const W = cssW;
        const H = cssH;

        const padL = 108;           // left gutter for lane labels
        const padR = 8;
        const headerH = 24;         // section marker row
        const laneGap = 2;
        const lanesY = headerH + 4;
        const laneAreaH = H - lanesY - 18;  // leave 18px for hint at bottom
        const laneH = Math.max(16, Math.floor((laneAreaH - laneGap * (PARAMS.length - 1)) / PARAMS.length));

        const gridX = padL;
        const gridW = Math.max(60, W - padL - padR);

        const genre = getGenre();
        const bars = sectionBars(genre);
        // Total bar span for this genre's arrangement (outro end exclusive)
        const firstBar = bars.intro[0];
        const lastBar = bars.outro[1];
        const totalBars = lastBar - firstBar;

        const blocks = blocksForGenre(genre).map((b) => {
            const offsetLo = b.startBar - firstBar;
            const offsetHi = b.endBar - firstBar;
            const x = gridX + (offsetLo / totalBars) * gridW;
            const w = ((offsetHi - offsetLo) / totalBars) * gridW;
            return { ...b, x, w };
        });

        // Section headers: one label per section, centered over its first block.
        const sections = SECTIONS.map((name) => {
            const [lo, hi] = bars[name];
            const offsetLo = lo - firstBar;
            const offsetHi = hi - firstBar;
            const x = gridX + (offsetLo / totalBars) * gridW;
            const w = ((offsetHi - offsetLo) / totalBars) * gridW;
            return { name, lo, hi, x, w };
        });

        const lanes = PARAMS.map((param, i) => ({
            param,
            top: lanesY + i * (laneH + laneGap),
            bottom: lanesY + i * (laneH + laneGap) + laneH,
            height: laneH,
        }));

        return { dpr, W, H, padL, padR, headerH, lanesY, laneH, laneGap, gridX, gridW, blocks, sections, lanes, bars, genre, totalBars };
    }

    // Hit-test: returns { param, block, lane } or null.
    function hit(x, y, L) {
        if (x < L.gridX) return null;
        const lane = L.lanes.find((ln) => y >= ln.top && y <= ln.bottom);
        if (!lane) return null;
        const block = L.blocks.find((b) => x >= b.x && x < b.x + b.w);
        if (!block) return null;
        return { param: lane.param, block, lane };
    }

    // ── Paint ──────────────────────────────────────────────────────────────
    function renderTimeline() {
        const canvas = document.getElementById('alsSectionTimeline');
        if (!canvas || canvas.offsetWidth === 0) return;
        const L = layout(canvas);
        const ctx = canvas.getContext('2d');
        ctx.setTransform(L.dpr, 0, 0, L.dpr, 0, 0);
        ctx.clearRect(0, 0, L.W, L.H);

        // Background — slightly lighter than outer card so lanes read as tracks.
        ctx.fillStyle = '#05050a';
        ctx.fillRect(0, 0, L.W, L.H);

        // Left label gutter divider
        ctx.fillStyle = '#0a0a14';
        ctx.fillRect(0, 0, L.padL, L.H);
        ctx.strokeStyle = '#1a1a28';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(L.padL + 0.5, 0);
        ctx.lineTo(L.padL + 0.5, L.H);
        ctx.stroke();

        // Section header row (▷ SECTIONNAME) — one label per section,
        // stretched across all the blocks it owns.
        ctx.font = 'bold 10px "Share Tech Mono", monospace';
        ctx.textBaseline = 'middle';
        for (const s of L.sections) {
            const sy = L.headerH / 2;
            ctx.fillStyle = '#7a8ba8';
            ctx.textAlign = 'left';
            ctx.fillText('\u25B7 ' + SECTION_LABELS[s.name], s.x + 4, sy);
            // Bar-range sublabel under it
            ctx.fillStyle = '#3a4858';
            ctx.font = '9px "Share Tech Mono", monospace';
            ctx.fillText(`${s.lo}\u2013${s.hi - 1}`, s.x + 4, sy + 10);
            ctx.font = 'bold 10px "Share Tech Mono", monospace';
        }

        // Lanes
        for (const ln of L.lanes) {
            // Lane background (alternating stripe)
            const isEven = L.lanes.indexOf(ln) % 2 === 0;
            ctx.fillStyle = isEven ? '#0c0c18' : '#0a0a14';
            ctx.fillRect(L.padL, ln.top, L.gridW, ln.height);

            // Lane label (left gutter)
            ctx.fillStyle = LANE_COLORS[ln.param].stroke;
            ctx.font = 'bold 11px "Orbitron", sans-serif';
            ctx.textAlign = 'right';
            ctx.textBaseline = 'middle';
            ctx.fillText(PARAM_LABELS[ln.param], L.padL - 8, ln.top + ln.height / 2);

            // Block dividers inside lane — subtle on every 8-bar boundary,
            // slightly brighter where a section begins.
            for (let i = 0; i < L.blocks.length; i++) {
                const b = L.blocks[i];
                const isSectionStart = i === 0 || L.blocks[i - 1].section !== b.section;
                ctx.strokeStyle = isSectionStart ? '#2a2a44' : '#141422';
                ctx.lineWidth = isSectionStart ? 1 : 1;
                ctx.beginPath();
                const gx = Math.round(b.x) + 0.5;
                ctx.moveTo(gx, ln.top);
                ctx.lineTo(gx, ln.bottom);
                ctx.stroke();
            }

            // Override cells — one rectangle per pinned 8-bar block.
            const laneOverrides = _overrides[ln.param] || {};
            const color = LANE_COLORS[ln.param];
            for (const b of L.blocks) {
                const v = laneOverrides[b.key];
                if (typeof v !== 'number') continue;
                const clamped = Math.max(0, Math.min(1, v));
                const fillH = Math.max(2, Math.round(clamped * (ln.height - 2)));
                const fy = ln.bottom - fillH;
                ctx.fillStyle = color.fill;
                ctx.fillRect(b.x + 1, fy, Math.max(1, b.w - 1), fillH);
                ctx.strokeStyle = color.stroke;
                ctx.lineWidth = 1;
                ctx.strokeRect(b.x + 1.5, fy + 0.5, Math.max(0, b.w - 2), Math.max(0, fillH - 1));
                // Value label only shown on block wide enough to read (else it clips).
                if (b.w > 32) {
                    ctx.fillStyle = '#05050a';
                    ctx.fillRect(b.x + 2, ln.top + 1, 26, 10);
                    ctx.fillStyle = color.stroke;
                    ctx.font = '9px "Share Tech Mono", monospace';
                    ctx.textAlign = 'left';
                    ctx.textBaseline = 'top';
                    ctx.fillText(clamped.toFixed(2), b.x + 3, ln.top + 2);
                }
            }

            // Selected outline
            if (_selected && _selected.param === ln.param) {
                const b = L.blocks.find((bx) => bx.key === _selected.blockKey);
                if (b) {
                    ctx.strokeStyle = '#05d9e8';
                    ctx.lineWidth = 2;
                    ctx.strokeRect(b.x + 1, ln.top + 1, Math.max(0, b.w - 2), ln.height - 2);
                    ctx.shadowBlur = 8;
                    ctx.shadowColor = 'rgba(5, 217, 232, 0.6)';
                    ctx.strokeRect(b.x + 1, ln.top + 1, Math.max(0, b.w - 2), ln.height - 2);
                    ctx.shadowBlur = 0;
                }
            }
        }

        // Bottom hint bar
        ctx.fillStyle = '#7a8ba8';
        ctx.font = '10px "Share Tech Mono", monospace';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'bottom';
        const hint = _selected
            ? `Selected: ${PARAM_LABELS[_selected.param]} / bars ${_selected.blockKey}\u2013${parseInt(_selected.blockKey, 10) + BLOCK_BARS - 1}  \u00B7  drag top edge to set value  \u00B7  scroll \u00B10.05  \u00B7  right-click to delete`
            : `Shift-click an 8-bar block to add an override  \u00B7  Click to edit  \u00B7  Drag top-edge to set value  \u00B7  Scroll wheel to fine-tune  \u00B7  Right-click to delete  \u00B7  ${L.blocks.length} blocks`;
        ctx.fillText(hint, L.padL, L.H - 4);
    }

    // ── Interactions ───────────────────────────────────────────────────────
    function setOverride(param, blockKey, value) {
        if (value == null) {
            delete _overrides[param][blockKey];
        } else {
            _overrides[param][blockKey] = Math.max(0, Math.min(1, value));
        }
        saveOverrides();
        renderTimeline();
    }

    function currentValue(param, blockKey) {
        const v = _overrides[param] && _overrides[param][blockKey];
        return typeof v === 'number' ? v : null;
    }

    function openPopover(canvas, L, param, block) {
        const pop = document.getElementById('alsTimelinePopover');
        if (!pop) return;
        pop.hidden = false;
        const title = pop.querySelector('.als-timeline-popover-title');
        const range = document.getElementById('alsTimelinePopoverValue');
        const label = document.getElementById('alsTimelinePopoverValueLabel');
        if (title) {
            title.textContent = `${PARAM_LABELS[param]} \u00B7 ${SECTION_LABELS[block.section]} \u00B7 bars ${block.startBar}\u2013${block.endBar - 1}`;
        }
        const v = currentValue(param, block.key);
        const pct = v == null ? 50 : Math.round(v * 100);
        if (range) range.value = String(pct);
        if (label) label.textContent = (pct / 100).toFixed(2);
        // Position popover under the block rect, within the wrap
        const lane = L.lanes.find((ln) => ln.param === param);
        if (lane) {
            const popW = 240;
            const wrap = canvas.parentElement;
            const wrapW = wrap ? wrap.clientWidth : L.W;
            let left = block.x + block.w / 2 - popW / 2;
            if (left < 4) left = 4;
            if (left + popW > wrapW - 4) left = wrapW - popW - 4;
            let top = lane.bottom + 6;
            const popH = 68;
            if (top + popH > L.H - 4) top = lane.top - popH - 6;
            pop.style.left = left + 'px';
            pop.style.top = top + 'px';
        }
    }

    function closePopover() {
        const pop = document.getElementById('alsTimelinePopover');
        if (pop) pop.hidden = true;
    }

    function onMouseDown(e) {
        const canvas = e.currentTarget;
        const r = canvas.getBoundingClientRect();
        const x = e.clientX - r.left;
        const y = e.clientY - r.top;
        const L = layout(canvas);
        const h = hit(x, y, L);
        if (!h) {
            _selected = null;
            closePopover();
            renderTimeline();
            return;
        }

        // Shift-click = create (or toggle default .5 if already present)
        if (e.shiftKey) {
            setOverride(h.param, h.block.key, 0.5);
            _selected = { param: h.param, blockKey: h.block.key, section: h.block.section };
            openPopover(canvas, L, h.param, h.block);
            return;
        }

        // Drag top-edge band (top 6px of lane) = "set value by height"
        const topBand = y - h.lane.top < 6;
        if (topBand && currentValue(h.param, h.block.key) != null) {
            _drag = { mode: 'value', param: h.param, blockKey: h.block.key, laneTop: h.lane.top, laneH: h.lane.height };
            e.preventDefault();
            return;
        }

        // Regular click = select + open popover (creates region with default if none)
        if (currentValue(h.param, h.block.key) == null) {
            setOverride(h.param, h.block.key, 0.5);
        }
        _selected = { param: h.param, blockKey: h.block.key, section: h.block.section };
        renderTimeline();
        openPopover(canvas, L, h.param, h.block);
    }

    function onMouseMove(e) {
        if (!_drag || _drag.mode !== 'value') return;
        const canvas = e.currentTarget;
        const r = canvas.getBoundingClientRect();
        const y = e.clientY - r.top;
        // Map y within lane to value (top=1, bottom=0)
        const rel = 1 - (y - _drag.laneTop) / _drag.laneH;
        setOverride(_drag.param, _drag.blockKey, rel);
    }

    function onMouseUp() {
        _drag = null;
    }

    function onWheel(e) {
        const canvas = e.currentTarget;
        const r = canvas.getBoundingClientRect();
        const x = e.clientX - r.left;
        const y = e.clientY - r.top;
        const L = layout(canvas);
        const h = hit(x, y, L);
        if (!h) return;
        const cur = currentValue(h.param, h.block.key);
        if (cur == null) return;
        e.preventDefault();
        const step = e.deltaY < 0 ? 0.05 : -0.05;
        setOverride(h.param, h.block.key, cur + step);
        _selected = { param: h.param, blockKey: h.block.key, section: h.block.section };
        openPopover(canvas, L, h.param, h.block);
    }

    function onContext(e) {
        const canvas = e.currentTarget;
        const r = canvas.getBoundingClientRect();
        const x = e.clientX - r.left;
        const y = e.clientY - r.top;
        const L = layout(canvas);
        const h = hit(x, y, L);
        if (!h) return;
        e.preventDefault();
        // Right-click = delete this override
        if (currentValue(h.param, h.block.key) != null) {
            setOverride(h.param, h.block.key, null);
            if (_selected && _selected.param === h.param && _selected.blockKey === h.block.key) {
                _selected = null;
                closePopover();
            }
        }
    }

    // ── Popover slider wiring ─────────────────────────────────────────────
    function onPopoverInput(e) {
        if (!_selected) return;
        const pct = parseInt(e.target.value, 10);
        const v = pct / 100;
        const label = document.getElementById('alsTimelinePopoverValueLabel');
        if (label) label.textContent = v.toFixed(2);
        setOverride(_selected.param, _selected.blockKey, v);
    }

    function onDeleteClick() {
        if (!_selected) return;
        setOverride(_selected.param, _selected.blockKey, null);
        _selected = null;
        closePopover();
    }

    function onClearAllClick() {
        _overrides = { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} };
        _selected = null;
        closePopover();
        saveOverrides();
        renderTimeline();
    }

    // ── Persistence ────────────────────────────────────────────────────────
    function saveOverrides() {
        try {
            if (typeof prefs !== 'undefined') {
                prefs.setItem('alsSectionOverrides', JSON.stringify(_overrides));
            }
        } catch { /* ignore */ }
    }

    // Legacy: old pref format used section names as keys. Fan each section's
    // single value out to every 8-bar block in that section for the current
    // genre so returning users don't silently lose their settings. Exposed so
    // tests can exercise it.
    function migrateLegacyOverrides(raw, genre) {
        const bars = sectionBars(genre);
        const out = { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} };
        if (!raw || typeof raw !== 'object') return out;
        for (const p of PARAMS) {
            if (!raw[p] || typeof raw[p] !== 'object') continue;
            const lane = raw[p];
            for (const key of Object.keys(lane)) {
                const v = lane[key];
                if (typeof v !== 'number' || v < 0 || v > 1) continue;
                // New format: numeric string key (block start bar).
                if (/^\d+$/.test(key)) {
                    // Snap to block start to normalize.
                    const n = parseInt(key, 10);
                    const snap = Math.floor((n - 1) / BLOCK_BARS) * BLOCK_BARS + 1;
                    out[p][String(snap)] = v;
                    continue;
                }
                // Legacy format: section name. Expand to every block in the section.
                if (bars[key]) {
                    const [lo, hi] = bars[key];
                    for (let b = lo; b < hi; b += BLOCK_BARS) {
                        out[p][String(b)] = v;
                    }
                }
            }
        }
        return out;
    }

    function restoreOverrides() {
        try {
            if (typeof prefs === 'undefined') return;
            const raw = prefs.getItem('alsSectionOverrides');
            if (!raw) return;
            const parsed = JSON.parse(raw);
            _overrides = migrateLegacyOverrides(parsed, getGenre());
            // Persist the migrated format so we only do the fan-out once.
            saveOverrides();
        } catch { /* ignore bad JSON */ }
    }

    // ── IPC payload — matches Rust SectionOverridesConfig shape ───────────
    function buildIpcPayload() {
        // Rust expects: { chaos: {"1":0.5,"9":0.3,...}, glitch: {...}, ... }.
        // Keys are string representations of 8-bar-block starting bars.
        // `#[serde(transparent)]` on SectionValues means no wrapper object.
        const out = {};
        for (const p of PARAMS) {
            out[p] = {};
            const lane = _overrides[p] || {};
            for (const key of Object.keys(lane)) {
                if (typeof lane[key] === 'number') out[p][key] = lane[key];
            }
        }
        return out;
    }

    // ── Init ───────────────────────────────────────────────────────────────
    function init() {
        const canvas = document.getElementById('alsSectionTimeline');
        if (!canvas || canvas._alsTimelineInit) return;
        canvas._alsTimelineInit = true;
        restoreOverrides();
        canvas.addEventListener('mousedown', onMouseDown);
        canvas.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
        canvas.addEventListener('wheel', onWheel, { passive: false });
        canvas.addEventListener('contextmenu', onContext);

        const popInput = document.getElementById('alsTimelinePopoverValue');
        if (popInput) popInput.addEventListener('input', onPopoverInput);

        // Delegated button clicks (data-action)
        document.addEventListener('click', (e) => {
            const btn = e.target.closest('[data-action]');
            if (!btn) return;
            const act = btn.dataset.action;
            if (act === 'alsOverrideDelete') onDeleteClick();
            else if (act === 'alsOverridesClearAll') onClearAllClick();
        });

        // Genre changes → block count / section bars change → repaint
        const genreSel = document.getElementById('alsGenre');
        if (genreSel) genreSel.addEventListener('change', renderTimeline);

        // ResizeObserver so the canvas reflows with its container
        if (typeof ResizeObserver === 'function') {
            _ro = new ResizeObserver(() => renderTimeline());
            _ro.observe(canvas);
        } else {
            window.addEventListener('resize', renderTimeline);
        }

        // Paint once DOM is settled
        requestAnimationFrame(() => requestAnimationFrame(renderTimeline));
    }

    // Public
    window.initAlsSectionOverridesTimeline = init;
    window.renderAlsSectionOverridesTimeline = renderTimeline;
    window.alsSectionOverridesForIpc = buildIpcPayload;
    window.alsSectionOverridesReset = () => {
        _overrides = { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} };
        _selected = null;
        closePopover();
        saveOverrides();
        renderTimeline();
    };
    // Test hook — lets tests verify the migration without poking internals.
    window.__alsTimelineMigrateLegacyOverrides = migrateLegacyOverrides;
    window.__alsTimelineBlocksForGenre = blocksForGenre;
})();
