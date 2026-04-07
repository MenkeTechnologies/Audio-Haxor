// ── Native OS drag-out (tauri-plugin-drag): drop files onto DAW, Finder, Desktop ──
// Single implementation for all tabs — pointer threshold avoids accidental drags vs clicks.
// Drag preview icon: canvas PNG themed from --cyan / --magenta (matches color schemes).

const _NATIVE_DRAG_FALLBACK_ICON = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const _NATIVE_DRAG_THRESHOLD_SQ = 8 * 8;
let _nativeDragPointer = null;
let _cyberDragIconDataUrl = null;

function invalidateNativeDragIconCache() {
    _cyberDragIconDataUrl = null;
}

if (typeof window !== 'undefined') {
    window.invalidateNativeDragIconCache = invalidateNativeDragIconCache;
}

/**
 * Cyberpunk drag ghost: neon frame, scanlines, mini spectrum — reads current CSS theme.
 */
function getCyberpunkDragIconDataUrl() {
    if (_cyberDragIconDataUrl) return _cyberDragIconDataUrl;
    const W = 128;
    const H = 128;
    let canvas;
    try {
        canvas = document.createElement('canvas');
        canvas.width = W;
        canvas.height = H;
    } catch {
        _cyberDragIconDataUrl = _NATIVE_DRAG_FALLBACK_ICON;
        return _cyberDragIconDataUrl;
    }
    const ctx = canvas.getContext('2d');
    if (!ctx) {
        _cyberDragIconDataUrl = _NATIVE_DRAG_FALLBACK_ICON;
        return _cyberDragIconDataUrl;
    }

    const root = document.documentElement;
    const cs = typeof getComputedStyle === 'function' ? getComputedStyle(root) : null;
    const cyan = (cs && cs.getPropertyValue('--cyan').trim()) || '#05d9e8';
    const magenta = (cs && cs.getPropertyValue('--magenta').trim()) || '#d300c5';
    let accent = (cs && cs.getPropertyValue('--accent').trim()) || cyan;
    if (!/^#([0-9a-f]{6})$/i.test(accent)) accent = cyan;

    ctx.fillStyle = '#06060c';
    ctx.fillRect(0, 0, W, H);

    const glow = ctx.createRadialGradient(W * 0.35, H * 0.25, 0, W * 0.5, H * 0.45, W * 0.65);
    glow.addColorStop(0, hexToRgba(cyan, 0.22));
    glow.addColorStop(0.45, hexToRgba(magenta, 0.08));
    glow.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, W, H);

    ctx.strokeStyle = 'rgba(5, 217, 232, 0.06)';
    ctx.lineWidth = 1;
    for (let x = 0; x < W; x += 12) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, H);
        ctx.stroke();
    }
    for (let y = 0; y < H; y += 12) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(W, y);
        ctx.stroke();
    }

    ctx.globalAlpha = 0.12;
    ctx.fillStyle = '#000';
    for (let y = 0; y < H; y += 2) ctx.fillRect(0, y, W, 1);
    ctx.globalAlpha = 1;

    const pad = 7;
    const rw = W - pad * 2;
    const rh = H - pad * 2;
    ctx.shadowColor = cyan;
    ctx.shadowBlur = 14;
    ctx.strokeStyle = cyan;
    ctx.lineWidth = 2.5;
    strokeRoundRect(ctx, pad, pad, rw, rh, 10);
    ctx.stroke();
    ctx.shadowBlur = 0;

    ctx.strokeStyle = magenta;
    ctx.lineWidth = 1;
    ctx.globalAlpha = 0.85;
    strokeRoundRect(ctx, pad + 4, pad + 4, rw - 8, rh - 8, 7);
    ctx.stroke();
    ctx.globalAlpha = 1;

    drawCornerTicks(ctx, pad + 2, pad + 2, rw - 4, rh - 4, cyan, magenta);

    ctx.font = 'bold 10px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = magenta;
    ctx.fillText('COPY', W / 2 + 1, 22 + 1);
    ctx.fillStyle = cyan;
    ctx.shadowColor = cyan;
    ctx.shadowBlur = 6;
    ctx.fillText('COPY', W / 2, 22);
    ctx.shadowBlur = 0;

    const bars = 14;
    const bw = 5;
    const gap = 2;
    const totalBw = bars * (bw + gap) - gap;
    const startX = (W - totalBw) / 2;
    const baseY = H - 22;
    const heights = [0.28, 0.42, 0.35, 0.55, 0.72, 0.88, 0.95, 0.92, 0.78, 0.5, 0.38, 0.45, 0.6, 0.32];
    for (let i = 0; i < bars; i++) {
        const bh = heights[i] * 44;
        const x = startX + i * (bw + gap);
        const y = baseY - bh;
        const g = ctx.createLinearGradient(x, y, x, baseY);
        g.addColorStop(0, cyan);
        g.addColorStop(0.55, accent);
        g.addColorStop(1, magenta);
        ctx.fillStyle = g;
        ctx.fillRect(x, y, bw, bh);
        ctx.fillStyle = 'rgba(255,255,255,0.15)';
        ctx.fillRect(x, y, Math.min(2, bw), bh);
    }

    ctx.strokeStyle = hexToRgba(cyan, 0.35);
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(startX, baseY + 1.5);
    ctx.lineTo(startX + totalBw, baseY + 1.5);
    ctx.stroke();

    let url;
    try {
        url = canvas.toDataURL('image/png');
    } catch {
        url = _NATIVE_DRAG_FALLBACK_ICON;
    }
    _cyberDragIconDataUrl = url || _NATIVE_DRAG_FALLBACK_ICON;
    return _cyberDragIconDataUrl;
}

function hexToRgba(hex, a) {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return `rgba(5, 217, 232, ${a})`;
    const n = parseInt(m[1], 16);
    const r = (n >> 16) & 255;
    const g = (n >> 8) & 255;
    const b = n & 255;
    return `rgba(${r},${g},${b},${a})`;
}

function strokeRoundRect(ctx, x, y, w, h, r) {
    const rr = Math.min(r, w / 2, h / 2);
    ctx.beginPath();
    if (typeof ctx.roundRect === 'function') {
        ctx.roundRect(x, y, w, h, rr);
        return;
    }
    ctx.moveTo(x + rr, y);
    ctx.lineTo(x + w - rr, y);
    ctx.arcTo(x + w, y, x + w, y + rr, rr);
    ctx.lineTo(x + w, y + h - rr);
    ctx.arcTo(x + w, y + h, x + w - rr, y + h, rr);
    ctx.lineTo(x + rr, y + h);
    ctx.arcTo(x, y + h, x, y + h - rr, rr);
    ctx.lineTo(x, y + rr);
    ctx.arcTo(x, y, x + rr, y, rr);
    ctx.closePath();
}

function drawCornerTicks(ctx, x, y, w, h, c1, c2) {
    const L = 12;
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = c1;
    ctx.beginPath();
    ctx.moveTo(x, y + L);
    ctx.lineTo(x, y);
    ctx.lineTo(x + L, y);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x + w - L, y);
    ctx.lineTo(x + w, y);
    ctx.lineTo(x + w, y + L);
    ctx.stroke();
    ctx.strokeStyle = c2;
    ctx.beginPath();
    ctx.moveTo(x, y + h - L);
    ctx.lineTo(x, y + h);
    ctx.lineTo(x + L, y + h);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x + w - L, y + h);
    ctx.lineTo(x + w, y + h);
    ctx.lineTo(x + w, y + h - L);
    ctx.stroke();
}

function pathsWithBatch(primaryPath) {
    if (typeof batchSelected !== 'undefined' && batchSelected.size > 0 && batchSelected.has(primaryPath)) {
        return [...batchSelected];
    }
    return [primaryPath];
}

/**
 * Resolve absolute file path(s) to drag from a pointer event target, or null.
 */
function resolveNativeDragPathsFromTarget(t) {
    if (!t || typeof t.closest !== 'function') return null;

    const simPanel = document.getElementById('similarPanel');
    if (simPanel && simPanel.contains(t)) {
        const row = t.closest('[data-similar-path]');
        if (row && row.dataset.similarPath) return {paths: [row.dataset.similarPath]};
    }

    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return null;
    const id = activeTab.id;

    if (id === 'tabSamples') {
        const tr = t.closest('#audioTableBody tr[data-audio-path]');
        if (!tr || tr.id === 'audioLoadMore' || t.closest('[data-action-stop]')) return null;
        const p = tr.dataset.audioPath;
        return p ? {paths: pathsWithBatch(p)} : null;
    }

    if (id === 'tabDaw') {
        const tr = t.closest('#dawTableBody tr[data-daw-path]');
        if (!tr || t.closest('[data-action-stop]')) return null;
        const p = tr.dataset.dawPath;
        return p ? {paths: pathsWithBatch(p)} : null;
    }

    if (id === 'tabPresets') {
        const tr = t.closest('#presetTableBody tr[data-preset-path]');
        if (!tr || t.closest('[data-action-stop]')) return null;
        const p = tr.dataset.presetPath;
        return p ? {paths: pathsWithBatch(p)} : null;
    }

    if (id === 'tabMidi') {
        const tr = t.closest('#midiTableBody tr[data-midi-path]');
        if (!tr || t.closest('[data-action-stop]')) return null;
        const p = tr.dataset.midiPath;
        return p ? {paths: pathsWithBatch(p)} : null;
    }

    if (id === 'tabPdf') {
        const tr = t.closest('#pdfTableBody tr[data-pdf-path]');
        if (!tr || t.closest('[data-action-stop]')) return null;
        const p = tr.dataset.pdfPath;
        return p ? {paths: pathsWithBatch(p)} : null;
    }

    if (id === 'tabPlugins') {
        const card = t.closest('#pluginList .plugin-card[data-path]');
        if (!card || t.closest('.plugin-actions')) return null;
        const p = card.dataset.path;
        return p ? {paths: [p]} : null;
    }

    if (id === 'tabFavorites') {
        const item = t.closest('#favList .fav-item[data-path]');
        if (!item || t.closest('.fav-actions')) return null;
        const p = item.dataset.path;
        return p ? {paths: [p]} : null;
    }

    if (id === 'tabNotes') {
        const card = t.closest('#notesList .note-card[data-path]');
        if (!card || t.closest('[data-action-stop]')) return null;
        const p = card.dataset.path;
        return p ? {paths: [p]} : null;
    }

    if (id === 'tabFiles') {
        const row = t.closest('#fileList .file-row[data-file-path]');
        if (!row || t.closest('.fb-meta-panel')) return null;
        const p = row.dataset.filePath;
        return p ? {paths: [p]} : null;
    }

    return null;
}

async function startNativeFileDrag(filePaths) {
    const tauri = typeof window !== 'undefined' ? window.__TAURI__ : null;
    if (!tauri || typeof tauri.drag?.startDrag !== 'function') return;
    const paths = filePaths.filter(Boolean);
    if (paths.length === 0) return;
    try {
        await tauri.drag.startDrag({
            item: paths,
            icon: getCyberpunkDragIconDataUrl(),
            mode: 'copy',
        });
    } catch (err) {
        if (typeof showToast === 'function') {
            showToast(String(err && err.message ? err.message : err), 4000, 'error');
        }
    }
}

function initNativeFileDrag() {
    if (typeof document === 'undefined' || initNativeFileDrag._done) return;
    initNativeFileDrag._done = true;

    document.addEventListener('click', (e) => {
        if (typeof window === 'undefined' || !window.__suppressNextDelegatedClick) return;
        window.__suppressNextDelegatedClick = false;
        e.preventDefault();
        e.stopImmediatePropagation();
    }, true);

    document.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        const resolved = resolveNativeDragPathsFromTarget(e.target);
        if (!resolved || !resolved.paths.length) return;
        _nativeDragPointer = {
            pointerId: e.pointerId,
            x: e.clientX,
            y: e.clientY,
            didDrag: false,
            paths: resolved.paths,
        };
    }, true);

    document.addEventListener('pointermove', (e) => {
        if (!_nativeDragPointer || e.pointerId !== _nativeDragPointer.pointerId) return;
        const d = _nativeDragPointer;
        const dx = e.clientX - d.x;
        const dy = e.clientY - d.y;
        if (dx * dx + dy * dy < _NATIVE_DRAG_THRESHOLD_SQ) return;
        if (d.didDrag) return;
        d.didDrag = true;
        e.preventDefault();
        void startNativeFileDrag(d.paths);
    }, true);

    document.addEventListener('pointerup', (e) => {
        if (!_nativeDragPointer || e.pointerId !== _nativeDragPointer.pointerId) return;
        const d = _nativeDragPointer;
        _nativeDragPointer = null;
        if (d.didDrag && typeof window !== 'undefined') {
            window.__suppressNextDelegatedClick = true;
            setTimeout(() => {
                if (typeof window !== 'undefined' && window.__suppressNextDelegatedClick) {
                    window.__suppressNextDelegatedClick = false;
                }
            }, 500);
        }
    }, true);

    document.addEventListener('pointercancel', (e) => {
        if (_nativeDragPointer && e.pointerId === _nativeDragPointer.pointerId) {
            _nativeDragPointer = null;
        }
    }, true);

    const warm = () => {
        try {
            getCyberpunkDragIconDataUrl();
        } catch {
            invalidateNativeDragIconCache();
        }
    };
    if (typeof requestIdleCallback === 'function') {
        requestIdleCallback(warm, {timeout: 4000});
    } else {
        setTimeout(warm, 2000);
    }
}

if (typeof document !== 'undefined') {
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => initNativeFileDrag());
    } else {
        initNativeFileDrag();
    }
}
