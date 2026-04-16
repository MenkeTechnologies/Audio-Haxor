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
    const MIN_SECTION_BARS = 8;
    const MAX_SECTION_BARS = 128;
    const BOUNDARY_HIT_PX = 5;

    // Paintbrush cursor (inline SVG data URI) — signals to users that the
    // primary gesture on the grid is click-drag to paint. Hotspot is at the
    // bristle tip (2,22). Falls back to `cell` (a crosshair/plus, widely
    // supported) on the rare WebView that rejects SVG cursors. Colors are
    // URL-encoded (# → %23) because raw `#` in a data URI is a fragment
    // separator and breaks parsing in WebKit.
    const PAINTBRUSH_CURSOR =
        "url(\"data:image/svg+xml;utf8," +
        "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'>" +
        "<path d='M21 2 L22 3 L14 11 L13 10 Z' fill='%23bf6b2e' stroke='black' stroke-width='1'/>" +
        "<path d='M10 13 L14 9 L18 13 L14 17 Z' fill='%238a8a8a' stroke='black' stroke-width='1'/>" +
        "<path d='M10 13 L2 22 L5 22 L8 20 L10 22 L14 17 Z' fill='%2305d9e8' stroke='black' stroke-width='1'/>" +
        "</svg>\") 2 22, cell";

    // Eraser cursor shown while the right-click-drag erase gesture is in
    // flight. Classic pink school eraser with a darker "worn" tip; hotspot
    // at the bottom-left tip (2,22) so the erased block is the one under the
    // actual pointer. Falls back to `not-allowed` on WebViews that reject
    // SVG cursors (signals "right-click here removes the thing").
    // Ramp cursor shown while Ctrl/Cmd is held over the grid. Signals to the
    // user that a cmd-click right now will linearly interpolate from the
    // current selection anchor to the clicked block. Stylized as a diagonal
    // gradient bar with a tiny arrow at its tip (hotspot at the arrow, 2,22).
    const RAMP_CURSOR =
        "url(\"data:image/svg+xml;utf8," +
        "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'>" +
        "<defs><linearGradient id='r' x1='0' y1='1' x2='1' y2='0'>" +
        "<stop offset='0' stop-color='%2305d9e8' stop-opacity='0.3'/>" +
        "<stop offset='1' stop-color='%2305d9e8' stop-opacity='1'/>" +
        "</linearGradient></defs>" +
        // Triangular ramp
        "<path d='M2 22 L22 22 L22 4 Z' fill='url(%23r)' stroke='black' stroke-width='1'/>" +
        // Small arrowhead at the tip to read as 'interpolate to here'
        "<path d='M2 22 L6 20 L4 22 Z' fill='black'/>" +
        "</svg>\") 2 22, crosshair";

    const ERASER_CURSOR =
        "url(\"data:image/svg+xml;utf8," +
        "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'>" +
        // Main pink body (rectangle rotated 45°): from top-right to lower-left.
        "<path d='M16 2 L22 8 L8 22 L2 16 Z' fill='%23ff6b9d' stroke='black' stroke-width='1'/>" +
        // Worn tip: darker pink chamfer at the erasing end.
        "<path d='M2 16 L8 22 L5 19 L2 19 Z' fill='%23cc3366' stroke='black' stroke-width='1'/>" +
        // Shavings / dust to read as active erasing.
        "<circle cx='3' cy='22' r='1' fill='%23ff6b9d' stroke='black' stroke-width='0.5'/>" +
        "</svg>\") 2 22, not-allowed";

    // Genre defaults — bar lengths per section. MUST match src-tauri/src/als_project.rs
    // `SectionLengths::{techno,trance,schranz}_default`. Users can override
    // per-genre via the draggable timeline boundaries (saved under prefs
    // `alsSectionLengthsByGenre`).
    const GENRE_DEFAULT_SECTION_LENGTHS = {
        techno:  { intro: 32, build: 32, breakdown: 32, drop1: 32, drop2: 32, fadedown: 32, outro: 32 },
        trance:  { intro: 32, build: 32, breakdown: 48, drop1: 32, drop2: 32, fadedown: 32, outro: 48 },
        schranz: { intro: 32, build: 32, breakdown: 16, drop1: 32, drop2: 48, fadedown: 32, outro: 16 },
    };

    // Read the user's per-genre section lengths from prefs, merged over
    // the genre defaults. Invalid/out-of-range values fall back silently.
    function currentSectionLengths(genre) {
        const defaults = GENRE_DEFAULT_SECTION_LENGTHS[genre] || GENRE_DEFAULT_SECTION_LENGTHS.techno;
        try {
            if (typeof prefs === 'undefined') return { ...defaults };
            const raw = prefs.getItem('alsSectionLengthsByGenre');
            if (!raw) return { ...defaults };
            const parsed = JSON.parse(raw);
            if (!parsed || typeof parsed !== 'object' || !parsed[genre] || typeof parsed[genre] !== 'object') {
                return { ...defaults };
            }
            const user = parsed[genre];
            const out = { ...defaults };
            for (const name of SECTIONS) {
                const v = user[name];
                if (typeof v === 'number' && v >= MIN_SECTION_BARS && v <= MAX_SECTION_BARS) {
                    // Snap to 8-bar grid defensively (in case prefs were hand-edited).
                    out[name] = Math.floor(v / 8) * 8;
                    if (out[name] < MIN_SECTION_BARS) out[name] = MIN_SECTION_BARS;
                }
            }
            return out;
        } catch {
            return { ...defaults };
        }
    }

    // Persist the user's per-genre section lengths. Preserves other genres'
    // stored values so changing trance doesn't wipe techno.
    function saveSectionLengths(genre, lengths) {
        try {
            if (typeof prefs === 'undefined') return;
            const raw = prefs.getItem('alsSectionLengthsByGenre');
            let all = {};
            if (raw) {
                try { all = JSON.parse(raw) || {}; } catch { all = {}; }
            }
            all[genre] = { ...lengths };
            prefs.setItem('alsSectionLengthsByGenre', JSON.stringify(all));
        } catch { /* ignore */ }
    }

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
    // `_selected` is the selection ANCHOR — the block the popover is attached
    // to, the last-clicked, the cmd-click ramp origin. `_selection` is the
    // full multi-select set (keyed `param:blockKey`). Invariant: when
    // `_selected` is non-null, it's also present in `_selection`. The popover
    // slider and delete button operate on the whole `_selection`, so the
    // user can bulk-adjust N blocks at once.
    let _selected = null;   // { param, blockKey, section } — anchor
    let _selection = {};    // { "param:blockKey": { param, blockKey, section } }
    let _drag = null;
    let _ro = null;
    // Whether Ctrl/Cmd is currently held — drives the ramp-cursor affordance
    // over the grid so users discover the cmd-click ramp gesture. Updated on
    // keydown/keyup and on every mousemove (mouse events carry the modifier
    // state even when focus is elsewhere).
    let _ctrlHeld = false;

    // Selection helpers. Keeping them small and local keeps the rest of the
    // module readable — everywhere else just calls these instead of poking
    // `_selection` directly.
    const selectionKey = (param, blockKey) => param + ':' + blockKey;
    function isSelected(param, blockKey) {
        return Object.prototype.hasOwnProperty.call(_selection, selectionKey(param, blockKey));
    }
    function selectionSize() { return Object.keys(_selection).length; }
    function selectionList() { return Object.values(_selection); }
    function selectionAdd(b) { _selection[selectionKey(b.param, b.blockKey)] = b; }
    function selectionRemove(param, blockKey) { delete _selection[selectionKey(param, blockKey)]; }
    function selectionClear() { _selection = {}; _selected = null; }
    function selectionSetSingle(b) {
        _selection = { [selectionKey(b.param, b.blockKey)]: b };
        _selected = b;
    }

    function getGenre() {
        const el = document.getElementById('alsGenre');
        return (el && GENRE_DEFAULT_SECTION_LENGTHS[el.value]) ? el.value : 'techno';
    }

    // Convert the user's section lengths for this genre into absolute bar
    // ranges. Inclusive lo, exclusive hi. Drives both the canvas layout and
    // the IPC payload.
    function sectionBars(genre) {
        const lengths = currentSectionLengths(genre);
        const out = {};
        let b = 1;
        for (const name of SECTIONS) {
            out[name] = [b, b + lengths[name]];
            b += lengths[name];
        }
        return out;
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
        // Right gutter needs to be wide enough that the outro's right edge
        // (which is always at gridX + gridW) has grab space on BOTH sides for
        // the boundary-drag gesture — a flush-against-canvas edge is awkward
        // to target with a mouse. 24 px gives a 5-px hit-zone on each side
        // plus a visible handle.
        const padR = 24;
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

    // Boundary hit-test: is the cursor within BOUNDARY_HIT_PX of any section's
    // right edge? Returns { section, edgeX } or null. All boundaries are
    // draggable — including the right edge of outro (which grows/shrinks the
    // total song length). Anywhere in the canvas body is a valid drag zone —
    // we don't gate on y so users can grab from the header row or any lane.
    function hitBoundary(x, y, L) {
        if (x < L.gridX || y < 0 || y > L.H) return null;
        let best = null;
        for (const s of L.sections) {
            const edgeX = s.x + s.w;
            const dx = Math.abs(x - edgeX);
            if (dx <= BOUNDARY_HIT_PX && (!best || dx < best.dx)) {
                best = { section: s.name, edgeX, dx };
            }
        }
        return best;
    }

    // Convert a pixel x into the bar number it represents in the current
    // canvas layout. Used by the boundary drag to snap to the 8-bar grid.
    function pixelToBar(x, L) {
        const firstBar = L.bars.intro[0];
        const pct = (x - L.gridX) / L.gridW;
        return firstBar + pct * L.totalBars;
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

        // Draggable section boundary handles — one per section right edge
        // (including the outro's, which resizes the whole song). Rendered as
        // a 3-px-wide bar in the header row so they're visible and grabbable
        // without cluttering the lane area. `hitBoundary` accepts anywhere
        // within BOUNDARY_HIT_PX of the edge, so the visual handle is just
        // an affordance cue — not the exact hit zone.
        for (const s of L.sections) {
            const edgeX = s.x + s.w;
            // Subtle vertical bar centered on the edge.
            ctx.fillStyle = 'rgba(122, 139, 168, 0.55)';
            ctx.fillRect(Math.round(edgeX) - 1, 2, 3, L.headerH - 4);
            // Small grip marks (two short horizontal ticks) so the handle
            // reads as "draggable", not just "section divider".
            ctx.fillStyle = 'rgba(5, 217, 232, 0.9)';
            ctx.fillRect(Math.round(edgeX) - 2, L.headerH / 2 - 3, 5, 1);
            ctx.fillRect(Math.round(edgeX) - 2, L.headerH / 2 + 2, 5, 1);
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

            // Selection outlines — every selected block in this lane gets a
            // cyan border; the anchor block gets an extra glow so the user
            // can tell at a glance which one drives the popover / cmd-click
            // ramp origin.
            for (const bx of L.blocks) {
                if (!isSelected(ln.param, bx.key)) continue;
                const isAnchor = _selected && _selected.param === ln.param && _selected.blockKey === bx.key;
                ctx.strokeStyle = '#05d9e8';
                ctx.lineWidth = isAnchor ? 2 : 1.5;
                ctx.strokeRect(bx.x + 1, ln.top + 1, Math.max(0, bx.w - 2), ln.height - 2);
                if (isAnchor) {
                    ctx.shadowBlur = 8;
                    ctx.shadowColor = 'rgba(5, 217, 232, 0.6)';
                    ctx.strokeRect(bx.x + 1, ln.top + 1, Math.max(0, bx.w - 2), ln.height - 2);
                    ctx.shadowBlur = 0;
                }
            }
        }

        // Boundary-drag preview — cyan ghost line at the would-be new edge,
        // positioned using the FROZEN pixel↔bar mapping so cursor movement
        // feels linear. Live repositioning + a bar-count label make the
        // expected result obvious before the user releases.
        if (_drag && _drag.mode === 'boundary') {
            const { gridX, gridW, firstBar, totalBars } = _drag.frozen;
            const newBoundaryBar = _drag.sectionStart + _drag.newLen;
            const ghostX = gridX + ((newBoundaryBar - firstBar) / totalBars) * gridW;
            // Shaded region between current stored boundary and the ghost
            // to make the delta visually obvious.
            const currentBoundaryBar = _drag.sectionStart + _drag.lengths[_drag.section];
            const currentX = gridX + ((currentBoundaryBar - firstBar) / totalBars) * gridW;
            ctx.fillStyle = 'rgba(5, 217, 232, 0.12)';
            const dx = ghostX - currentX;
            if (dx > 0) ctx.fillRect(currentX, L.headerH, dx, L.H - L.headerH - 18);
            else if (dx < 0) ctx.fillRect(ghostX, L.headerH, -dx, L.H - L.headerH - 18);

            // Ghost line + cap.
            ctx.strokeStyle = '#05d9e8';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(Math.round(ghostX) + 0.5, 0);
            ctx.lineTo(Math.round(ghostX) + 0.5, L.H - 18);
            ctx.stroke();
            // Label: new section length in bars.
            const label = `${SECTION_LABELS[_drag.section]}: ${_drag.newLen} bars`;
            ctx.font = 'bold 10px "Share Tech Mono", monospace';
            ctx.textAlign = 'left';
            ctx.textBaseline = 'top';
            const tw = ctx.measureText(label).width + 8;
            const tx = Math.min(ghostX + 4, L.W - tw - 2);
            ctx.fillStyle = 'rgba(5, 217, 232, 0.95)';
            ctx.fillRect(tx, L.headerH + 2, tw, 14);
            ctx.fillStyle = '#05050a';
            ctx.fillText(label, tx + 4, L.headerH + 4);
        }

        // Bottom hint bar
        ctx.fillStyle = '#7a8ba8';
        ctx.font = '10px "Share Tech Mono", monospace';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'bottom';
        const hint = _drag && _drag.mode === 'boundary'
            ? `Resizing ${SECTION_LABELS[_drag.section]} \u2192 ${_drag.newLen} bars  \u00B7  release to commit  \u00B7  8-bar snap, min ${MIN_SECTION_BARS}, max ${MAX_SECTION_BARS}`
            : _drag && _drag.mode === 'paint'
                ? `Painting @ ${_drag.paintValue.toFixed(2)}  \u00B7  ${_drag.painted.size} block${_drag.painted.size === 1 ? '' : 's'} (any lane)  \u00B7  release to finish`
                : _drag && _drag.mode === 'erase'
                    ? `Erasing  \u00B7  ${_drag.erased.size} block${_drag.erased.size === 1 ? '' : 's'} cleared (any lane)  \u00B7  release to finish`
                    : selectionSize() > 1
                        ? `${selectionSize()} blocks selected  \u00B7  slider adjusts them all  \u00B7  Shift-click: extend range  \u00B7  Click (no mods): single-select  \u00B7  Delete (\u00D7 in popover) clears all selected`
                        : _selected
                            ? `Selected: ${PARAM_LABELS[_selected.param]} / bars ${_selected.blockKey}\u2013${parseInt(_selected.blockKey, 10) + BLOCK_BARS - 1}  \u00B7  drag top edge: value  \u00B7  scroll \u00B10.05  \u00B7  right-click: delete  \u00B7  shift-click: range-select  \u00B7  \u2318/Ctrl-click: ramp`
                            : `Click-drag: paint  \u00B7  Right-click-drag: erase  \u00B7  Shift-click: range-select (anchor \u2192 clicked)  \u00B7  \u2318/Ctrl-click: ramp from anchor to clicked (y = end)  \u00B7  Drag top edge: value  \u00B7  Scroll: fine-tune  \u00B7  Drag boundary to resize  \u00B7  ${L.blocks.length} blocks`;
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
            const n = selectionSize();
            if (n > 1) {
                // Multi-select: title shows count + anchor context so users
                // know which block drives the default slider position.
                title.textContent = `${n} blocks selected \u00B7 anchor: ${PARAM_LABELS[param]} \u00B7 bars ${block.startBar}\u2013${block.endBar - 1}`;
            } else {
                title.textContent = `${PARAM_LABELS[param]} \u00B7 ${SECTION_LABELS[block.section]} \u00B7 bars ${block.startBar}\u2013${block.endBar - 1}`;
            }
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

        // Section boundary drag has priority over block hit — a click on the
        // exact edge of a section should resize, not pin an override. Only
        // left-button triggers the resize; right-button falls through so
        // users can erase a block that happens to sit under a boundary pixel.
        const b = e.button === 0 ? hitBoundary(x, y, L) : null;
        if (b) {
            const genre = getGenre();
            const lengths = currentSectionLengths(genre);
            const sectionBarsNow = sectionBars(genre);
            // Freeze the pixel↔bar mapping for the whole gesture so dragging
            // stays linear. If we let `layout()` re-run on every mousemove
            // after updating lengths, the grid auto-reflows and the
            // pixels-per-bar ratio shifts under the user's cursor — it feels
            // jumpy and non-linear. With the mapping frozen, 1 pixel moved
            // always = same number of bars, and the preview ghost line tracks
            // the cursor exactly. We only commit the new length on mouseup.
            _drag = {
                mode: 'boundary',
                section: b.section,
                genre,
                lengths, // current (un-mutated) — layout() keeps showing this
                sectionStart: sectionBarsNow[b.section][0],
                newLen: lengths[b.section],
                frozen: {
                    gridX: L.gridX,
                    gridW: L.gridW,
                    firstBar: L.bars.intro[0],
                    totalBars: L.totalBars,
                },
            };
            e.preventDefault();
            canvas.style.cursor = 'col-resize';
            renderTimeline();
            return;
        }

        const h = hit(x, y, L);
        if (!h) {
            selectionClear();
            closePopover();
            renderTimeline();
            return;
        }

        // Right-click (button 2) — FL-Studio-style erase. A click alone clears
        // this block; a drag keeps clearing every block the cursor passes over
        // IN ANY LANE. (Paint is lane-locked — erase is not: sweeping down
        // through multiple rows lets you wipe a rectangular region in one
        // gesture.) We preventDefault so the native context menu never opens.
        if (e.button === 2) {
            e.preventDefault();
            if (currentValue(h.param, h.block.key) != null) {
                setOverride(h.param, h.block.key, null);
                selectionRemove(h.param, h.block.key);
                if (_selected && _selected.param === h.param && _selected.blockKey === h.block.key) {
                    const rest = selectionList();
                    _selected = rest.length > 0 ? rest[rest.length - 1] : null;
                    if (!_selected) closePopover();
                }
            }
            _drag = {
                mode: 'erase',
                // erased set keys are `param:blockKey` (composite) so the
                // same bar block in two different lanes doesn't collide
                // and falsely short-circuit.
                erased: new Set([h.param + ':' + h.block.key]),
            };
            // Swap to the eraser cursor for the whole gesture so the user
            // gets clear visual feedback they're in erase mode, not paint.
            canvas.style.cursor = ERASER_CURSOR;
            return;
        }

        // Ignore middle-button and anything else — don't start a gesture.
        if (e.button !== 0) return;

        // Cmd/Ctrl-click — linear ramp from the existing anchor to the
        // clicked block. Start value = anchor's current value (or 0.5 if
        // unpinned); end value = click Y-height inside clicked lane. Every
        // 8-bar block between is linearly interpolated. Same-lane only —
        // the anchor must be in the clicked block's lane.
        if (e.ctrlKey || e.metaKey) {
            if (_selected && _selected.param === h.param && _selected.blockKey !== h.block.key) {
                const startBar = parseInt(_selected.blockKey, 10);
                const endBar = parseInt(h.block.key, 10);
                const vStart = currentValue(h.param, _selected.blockKey);
                const vStartResolved = typeof vStart === 'number' ? vStart : 0.5;
                const yRel = 1 - (y - h.lane.top) / h.lane.height;
                const vEnd = Math.max(0, Math.min(1, yRel));
                const lo = Math.min(startBar, endBar);
                const hi = Math.max(startBar, endBar);
                const nBlocks = Math.floor((hi - lo) / BLOCK_BARS) + 1;
                // Distance from the ANCHOR drives the fraction, so a rightward
                // cmd-click ramps vStart→vEnd and a leftward one ramps
                // vEnd→vStart (still anchored at vStart).
                for (let i = 0; i < nBlocks; i++) {
                    const bar = lo + i * BLOCK_BARS;
                    const distFromAnchor = (startBar <= endBar) ? i : (nBlocks - 1 - i);
                    const frac = nBlocks <= 1 ? 0 : distFromAnchor / (nBlocks - 1);
                    const v = vStartResolved + (vEnd - vStartResolved) * frac;
                    setOverride(h.param, String(bar), v);
                }
                // Select every block in the ramp so the popover slider can
                // adjust them as a group afterward.
                _selection = {};
                for (let i = 0; i < nBlocks; i++) {
                    const bar = lo + i * BLOCK_BARS;
                    const b = L.blocks.find((bx) => bx.key === String(bar));
                    if (b) selectionAdd({ param: h.param, blockKey: b.key, section: b.section });
                }
                _selected = { param: h.param, blockKey: h.block.key, section: h.block.section };
                renderTimeline();
                openPopover(canvas, L, h.param, h.block);
                return;
            }
            // No anchor, or anchor in a different lane, or clicked the anchor
            // itself — degrade to a plain single-selection at the click.
            setOverride(h.param, h.block.key, 0.5);
            selectionSetSingle({ param: h.param, blockKey: h.block.key, section: h.block.section });
            renderTimeline();
            openPopover(canvas, L, h.param, h.block);
            return;
        }

        // Shift-click — range-select every 8-bar block between the anchor
        // and the clicked block in the same lane. Classic Finder/spreadsheet
        // convention: click A, then shift-click B to grab the whole run.
        // Pure selection — doesn't pin values; the popover slider does that
        // across the whole range on drag. Shift-clicking in a different lane
        // degrades to a fresh single-select (no cross-lane range for now,
        // since different params have different semantics).
        if (e.shiftKey) {
            if (_selected && _selected.param === h.param) {
                const startBar = parseInt(_selected.blockKey, 10);
                const endBar = parseInt(h.block.key, 10);
                const lo = Math.min(startBar, endBar);
                const hi = Math.max(startBar, endBar);
                // Replace selection with the range [lo..hi] inclusive. Bar
                // keys step by BLOCK_BARS from the grid, so we walk that step.
                _selection = {};
                for (let bar = lo; bar <= hi; bar += BLOCK_BARS) {
                    const b = L.blocks.find((bx) => bx.key === String(bar));
                    if (b) {
                        selectionAdd({ param: h.param, blockKey: b.key, section: b.section });
                    }
                }
                // Anchor moves to the clicked block so another shift-click
                // can extend the range from here.
                _selected = { param: h.param, blockKey: h.block.key, section: h.block.section };
                renderTimeline();
                openPopover(canvas, L, h.param, h.block);
                return;
            }
            // No anchor (or anchor in another lane): fall back to a plain
            // single-select at the click so the user has something to anchor
            // future shift-clicks against.
            selectionSetSingle({ param: h.param, blockKey: h.block.key, section: h.block.section });
            renderTimeline();
            openPopover(canvas, L, h.param, h.block);
            return;
        }

        // Top-6px band of a pinned block = "set value by height" drag. Takes
        // priority over paint so users can still fine-tune a single block's
        // intensity without accidentally painting the neighbors.
        const topBand = y - h.lane.top < 6;
        if (topBand && currentValue(h.param, h.block.key) != null) {
            _drag = { mode: 'value', param: h.param, blockKey: h.block.key, laneTop: h.lane.top, laneH: h.lane.height };
            e.preventDefault();
            return;
        }

        // FL-Studio-style paint. Paint value comes from the first block:
        //   - clicking a pinned block → paint its existing value across the
        //     drag (lets you extend a region to neighbors cleanly)
        //   - clicking empty → paint 0.5 (the default new-override value)
        // Paint is CROSS-LANE — dragging vertically into another dynamics row
        // keeps painting the same value there too, matching the erase gesture
        // so both brushes behave consistently. Painted-set keys are composite
        // `param:blockKey` pairs so the same bar block in two lanes each get
        // their own paint application.
        // First block is always set (so a bare click with no drag still pins).
        // Popover opens ON MOUSEUP if the gesture turned out to be a single
        // click (no paint spread), so drags don't pop the editor.
        const existing = currentValue(h.param, h.block.key);
        const paintValue = existing != null ? existing : 0.5;
        setOverride(h.param, h.block.key, paintValue);
        // Plain paint click replaces any prior multi-selection with just the
        // starting block — consistent with "click = fresh single selection".
        selectionSetSingle({ param: h.param, blockKey: h.block.key, section: h.block.section });
        _drag = {
            mode: 'paint',
            paintValue,
            painted: new Set([h.param + ':' + h.block.key]),
            firstParam: h.param,
            // Stash the first hit so mouseup can open the popover if no spread.
            firstBlock: h.block,
        };
        renderTimeline();
    }

    function onMouseMove(e) {
        // The canvas mousemove handler drives only idle hover cursor — while
        // dragging we use `onWindowMouseMove` so the gesture follows the
        // cursor even when it leaves the canvas.
        if (_drag) return;
        // Refresh the ctrl-held flag on every move so we stay in sync even
        // if the keydown/keyup listener missed an event (focus-out, etc.).
        _ctrlHeld = e.ctrlKey || e.metaKey;
        const canvas = e.currentTarget;
        const r = canvas.getBoundingClientRect();
        const x = e.clientX - r.left;
        const y = e.clientY - r.top;
        const L = layout(canvas);

        // Cursor priority:
        //   boundary resize > top-band value drag
        //   > ramp cursor (ctrl/cmd held + anchor in this lane)
        //   > paintbrush on a block
        //   > default elsewhere.
        const b = hitBoundary(x, y, L);
        if (b) { canvas.style.cursor = 'col-resize'; return; }

        const h = hit(x, y, L);
        if (h) {
            const topBand = y - h.lane.top < 6;
            if (topBand && currentValue(h.param, h.block.key) != null) {
                canvas.style.cursor = 'ns-resize';
            } else if (_ctrlHeld && _selected && _selected.param === h.param && _selected.blockKey !== h.block.key) {
                canvas.style.cursor = RAMP_CURSOR;
            } else {
                canvas.style.cursor = PAINTBRUSH_CURSOR;
            }
            return;
        }
        canvas.style.cursor = '';
    }

    // Window-level mousemove so boundary/value drags don't stall when the
    // cursor leaves the canvas bounds (common for users extending the outro
    // who pull past the right edge).
    function onWindowMouseMove(e) {
        if (!_drag) return;
        const canvas = document.getElementById('alsSectionTimeline');
        if (!canvas) return;
        const r = canvas.getBoundingClientRect();
        const x = e.clientX - r.left;
        const y = e.clientY - r.top;

        if (_drag.mode === 'boundary') {
            // Use the FROZEN pixel→bar mapping captured at mousedown so the
            // cursor always maps linearly to bars, even as the user extends
            // past the canvas edge. We only update `_drag.newLen` (the
            // preview) — the stored lengths don't change until mouseup.
            const { gridX, gridW, firstBar, totalBars } = _drag.frozen;
            const cursorBar = firstBar + ((x - gridX) / gridW) * totalBars;
            let newLen = Math.round((cursorBar - _drag.sectionStart) / BLOCK_BARS) * BLOCK_BARS;
            if (newLen < MIN_SECTION_BARS) newLen = MIN_SECTION_BARS;
            if (newLen > MAX_SECTION_BARS) newLen = MAX_SECTION_BARS;
            if (newLen !== _drag.newLen) {
                _drag.newLen = newLen;
                renderTimeline();
            }
            return;
        }

        if (_drag.mode === 'value') {
            const rel = 1 - (y - _drag.laneTop) / _drag.laneH;
            setOverride(_drag.param, _drag.blockKey, rel);
            return;
        }

        // Paint — cross-lane. Look up whichever lane the cursor is currently
        // inside and stamp the saved paintValue on that block. Sweeping the
        // cursor diagonally down through multiple rows paints a rectangular
        // region of the same value in one gesture (mirrors erase).
        if (_drag.mode === 'paint') {
            const L = layout(canvas);
            if (x < L.gridX) return;
            const lane = L.lanes.find((ln) => y >= ln.top && y <= ln.bottom);
            if (!lane) return;
            const block = L.blocks.find((bl) => x >= bl.x && x < bl.x + bl.w);
            if (!block) return;
            const key = lane.param + ':' + block.key;
            if (_drag.painted.has(key)) return;
            setOverride(lane.param, block.key, _drag.paintValue);
            _drag.painted.add(key);
            // Every painted block joins the selection, so post-paint the
            // popover slider can bulk-adjust the whole range in one gesture.
            // Anchor follows the current paint position.
            const entry = { param: lane.param, blockKey: block.key, section: block.section };
            selectionAdd(entry);
            _selected = entry;
            renderTimeline();
            return;
        }

        // Erase — cross-lane. Look up WHATEVER lane the cursor is currently
        // inside and clear the block there, so sweeping the cursor through
        // multiple rows wipes a whole rectangular region in one gesture.
        // Erased-set keys are `param:blockKey` so the same bar block in two
        // different lanes both get erased independently.
        if (_drag.mode === 'erase') {
            const L = layout(canvas);
            if (x < L.gridX) return;
            const lane = L.lanes.find((ln) => y >= ln.top && y <= ln.bottom);
            if (!lane) return;
            const block = L.blocks.find((bl) => x >= bl.x && x < bl.x + bl.w);
            if (!block) return;
            const key = lane.param + ':' + block.key;
            if (_drag.erased.has(key)) return;
            if (currentValue(lane.param, block.key) != null) {
                setOverride(lane.param, block.key, null);
                selectionRemove(lane.param, block.key);
                if (_selected && _selected.param === lane.param && _selected.blockKey === block.key) {
                    const rest = selectionList();
                    _selected = rest.length > 0 ? rest[rest.length - 1] : null;
                    if (!_selected) closePopover();
                }
            }
            _drag.erased.add(key);
            renderTimeline();
            return;
        }
    }

    function onMouseUp() {
        if (_drag && _drag.mode === 'boundary') {
            const canvas = document.getElementById('alsSectionTimeline');
            if (canvas) canvas.style.cursor = '';
            // Commit the dragged length if it actually changed. Only here
            // do we touch prefs + re-layout; during the gesture we only
            // shift a preview ghost line.
            if (_drag.newLen !== _drag.lengths[_drag.section]) {
                const committed = { ..._drag.lengths, [_drag.section]: _drag.newLen };
                saveSectionLengths(_drag.genre, committed);
            }
            _drag = null;
            renderTimeline();
            return;
        }

        // Paint: if the gesture stayed on a single block (no drag spread),
        // treat it as a click and open the editor popover. Drags that
        // painted 2+ blocks skip the popover — the user's intent was to
        // bulk-paint, not fine-tune a single cell.
        if (_drag && _drag.mode === 'paint') {
            const soloClick = _drag.painted.size === 1;
            if (soloClick) {
                const canvas = document.getElementById('alsSectionTimeline');
                if (canvas) {
                    const L = layout(canvas);
                    openPopover(canvas, L, _drag.firstParam, _drag.firstBlock);
                }
            }
            _drag = null;
            return;
        }

        // Erase drag just ends — nothing to finalize since every block was
        // cleared incrementally during mousemove. Reset the cursor so the
        // next idle mousemove can recompute it (paintbrush over a block,
        // col-resize on a boundary, …).
        if (_drag && _drag.mode === 'erase') {
            const canvas = document.getElementById('alsSectionTimeline');
            if (canvas) canvas.style.cursor = '';
        }
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
        // Scroll-wheel targets a single block; treat it like a single-select
        // (replaces any prior multi-selection). Popover opens so the user
        // can see the new value.
        selectionSetSingle({ param: h.param, blockKey: h.block.key, section: h.block.section });
        openPopover(canvas, L, h.param, h.block);
    }

    // Ctrl/Cmd press/release: update `_ctrlHeld` and nudge the cursor on the
    // canvas so the ramp affordance appears/disappears without requiring a
    // mouse move. We read the current cursor by doing a zero-delta hit test
    // via stored last-mousemove state is overkill; simpler to just force a
    // `canvas.style.cursor = ''` reset — the next real mousemove will pick
    // the right cursor (paintbrush / ramp / etc.) based on the new flag.
    function onKeyChange(e) {
        const held = e.ctrlKey || e.metaKey;
        if (held === _ctrlHeld) return;
        _ctrlHeld = held;
        const canvas = document.getElementById('alsSectionTimeline');
        if (canvas) canvas.style.cursor = '';
    }

    // Suppress the native (and app-wide delegated) context menus on the
    // canvas — right-click is bound to the erase-drag gesture via
    // `onMouseDown` (button === 2). Without preventDefault + stopProp, WebKit
    // pops the system menu and the mouseup never reaches us, stranding
    // `_drag` in erase mode; stopImmediatePropagation also prevents any
    // document-level custom-menu handlers (context-menu.js, file-browser.js,
    // etc.) from firing while the cursor is over our canvas.
    function onContext(e) {
        e.preventDefault();
        e.stopPropagation();
        if (typeof e.stopImmediatePropagation === 'function') {
            e.stopImmediatePropagation();
        }
        return false;
    }

    // ── Popover slider wiring ─────────────────────────────────────────────
    // Slider input stamps the same value across EVERY block in the current
    // multi-selection, not just the anchor. Lets users shift-click a handful
    // of blocks (across lanes if they want), then dial in one value for all
    // of them in a single gesture. Matches the title text shown in the
    // popover ("N blocks selected") so the behavior isn't surprising.
    function onPopoverInput(e) {
        if (selectionSize() === 0) return;
        const pct = parseInt(e.target.value, 10);
        const v = pct / 100;
        const label = document.getElementById('alsTimelinePopoverValueLabel');
        if (label) label.textContent = v.toFixed(2);
        for (const b of selectionList()) {
            setOverride(b.param, b.blockKey, v);
        }
    }

    // Popover ✕ clears every block in the current selection — symmetric with
    // the slider applying to the whole group.
    function onDeleteClick() {
        if (selectionSize() === 0) return;
        for (const b of selectionList()) {
            setOverride(b.param, b.blockKey, null);
        }
        selectionClear();
        closePopover();
    }

    function onClearAllClick() {
        _overrides = { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} };
        selectionClear();
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
        window.addEventListener('mousemove', onWindowMouseMove);
        window.addEventListener('mouseup', onMouseUp);
        canvas.addEventListener('wheel', onWheel, { passive: false });
        // Capture-phase so this fires BEFORE any document-level contextmenu
        // delegates (context-menu.js et al.) — they can't intercept if we
        // preventDefault + stopImmediatePropagation at the source first.
        canvas.addEventListener('contextmenu', onContext, { capture: true });
        const wrap = canvas.parentElement;
        if (wrap) wrap.addEventListener('contextmenu', onContext, { capture: true });

        // Modifier-key tracking so the ramp cursor flips on immediately when
        // the user presses Ctrl/Cmd without having to wiggle the mouse. We
        // only need to trigger a cursor recompute, which piggybacks on the
        // existing idle-hover logic via a synthetic-style call (reusing the
        // canvas's current cursor position would require tracking it; cheaper
        // to just force a hover-equivalent recompute on the next frame).
        window.addEventListener('keydown', onKeyChange);
        window.addEventListener('keyup', onKeyChange);
        // Also recompute when the window loses focus — the user may have
        // released the modifier in another app and returned with _ctrlHeld
        // stale.
        window.addEventListener('blur', () => { _ctrlHeld = false; });

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

    // Section-length IPC — surfaces the user's (possibly overridden) bar
    // counts for the currently-selected genre so the Rust generator can
    // build the arrangement at the chosen lengths.
    window.alsSectionLengthsForIpc = () => currentSectionLengths(getGenre());

    // Reset just this genre's lengths back to the genre default. Called from
    // a UI button if/when wired; always available via the window helper.
    window.alsSectionLengthsResetForCurrentGenre = () => {
        const genre = getGenre();
        const def = { ...GENRE_DEFAULT_SECTION_LENGTHS[genre] };
        saveSectionLengths(genre, def);
        renderTimeline();
    };

    // Test hooks — lets tests/dev-console verify state without poking internals.
    window.__alsTimelineMigrateLegacyOverrides = migrateLegacyOverrides;
    window.__alsTimelineBlocksForGenre = blocksForGenre;
    window.__alsTimelineCurrentSectionLengths = currentSectionLengths;
    window.__alsTimelineGenreDefaultSectionLengths = GENRE_DEFAULT_SECTION_LENGTHS;
})();
