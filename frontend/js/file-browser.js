// ── File Browser ──
const _ctxMenuNoEcho = {skipEchoToast: true};

function _ctxShortcutTip(id) {
    return typeof shortcutTip === 'function' ? shortcutTip(id) : {};
}

/** Rust `fs_list_dir` may return `\` on Windows; normalize for split/join logic. */
function normalizePathSeparators(p) {
    if (p == null || typeof p !== 'string') return '';
    return p.replace(/\\/g, '/');
}

/** Parent directory for Unix paths and Windows `C:/...` paths (after normalization). */
function parentDirectoryPath(p) {
    const n = normalizePathSeparators(p).replace(/\/+$/, '');
    if (n === '' || n === '/') return '/';
    if (/^[A-Za-z]:$/.test(n)) return n + '/';
    const i = n.lastIndexOf('/');
    if (i < 0) return '/';
    if (i === 0) return '/';
    const parent = n.slice(0, i);
    if (/^[A-Za-z]:$/.test(parent)) return parent + '/';
    return parent || '/';
}

/** Last path segment for labels (handles Windows `\\` from `fs_list_dir`). */
function pathFileName(p) {
    const n = normalizePathSeparators(p);
    const segs = n.split('/').filter(Boolean);
    return segs.length ? segs[segs.length - 1] : n;
}

// `var` (not `let`) on the module-state binding so the VM test sandbox can
// seed `_fileBrowserEntries` (and inspect path/inited state) via globalThis.
// Same rationale as the sort/select state below.
var _fileBrowserPath = null;
var _fileBrowserEntries = [];
var _fileBrowserInited = false;
/** Invalidate in-flight chunked file list renders when directory or filter changes. */
var _fileListRenderSeq = 0;
/** Search mode for `filterFiles` (`registerFilter`); used by `renderFileList`. */
var _lastFilesMode = 'fuzzy';
const FILE_LIST_CHUNK = 80;

// ── Favorite Directories ──
function getFavDirs() {
    return prefs.getObject('favDirs', []);
}

function saveFavDirs(dirs) {
    prefs.setItem('favDirs', dirs);
}

function isFavDir(dirPath) {
    return getFavDirs().some(d => d.path === dirPath);
}

function addFavDir(dirPath) {
    const dirs = getFavDirs();
    if (dirs.some(d => d.path === dirPath)) return;
    const name = pathFileName(dirPath) || dirPath;
    dirs.push({path: dirPath, name});
    saveFavDirs(dirs);
    renderFavDirs();
    updateBookmarkBtn();
    showToast(toastFmt('toast.bookmarked_name', {name}));
}

function removeFavDir(dirPath) {
    saveFavDirs(getFavDirs().filter(d => d.path !== dirPath));
    renderFavDirs();
    updateBookmarkBtn();
    showToast(toastFmt('toast.bookmark_removed'));
}

function renderFavDirs() {
    const container = document.getElementById('fileFavs');
    const grid = document.getElementById('fileFavsGrid');
    if (!container || !grid) return;
    const dirs = getFavDirs();
    if (dirs.length === 0) {
        container.classList.add('fb-hidden');
        return;
    }
    container.classList.remove('fb-hidden');
    const rmTitle = catalogFmt('ui.tt.remove_bookmark_from_chip');
    grid.innerHTML = dirs.map(d =>
        `<div class="file-fav-chip" data-fav-dir="${escapeHtml(d.path)}" title="${escapeHtml(d.path)}">
      <span class="fav-chip-icon">&#128193;</span>
      <span class="fav-chip-name">${escapeHtml(d.name)}</span>
      <span class="fav-chip-remove" data-remove-fav-dir="${escapeHtml(d.path)}" title="${escapeHtml(rmTitle)}">&#10005;</span>
    </div>`
    ).join('');
}

function updateBookmarkBtn() {
    const btn = document.getElementById('btnFileFav');
    if (!btn || !_fileBrowserPath) return;
    const fav = isFavDir(_fileBrowserPath);
    const fmt = catalogFmt;
    const label = fav ? fmt('ui.btn.9733_unbookmark') : fmt('ui.btn.9733_bookmark');
    btn.innerHTML = `&#9733; <span>${escapeHtml(label)}</span>`;
    btn.title = fav ? fmt('ui.tt.remove_current_directory_from_bookmarks') : fmt('ui.tt.bookmark_current_directory');
}

/** Keep in sync with `AUDIO_EXTENSIONS` in `src-tauri/src/audio_extensions.rs`. */
const AUDIO_EXTS = [
    'wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'wma', 'aac', 'opus', 'rex', 'rx2', 'sf2', 'sfz',
];
/** Keep in sync with `DAW_EXTENSIONS` in `src-tauri/src/daw_scanner.rs` (lowercase, no dot). */
const DAW_EXTS = ['als', 'logicx', 'flp', 'cpr', 'npr', 'bwproject', 'rpp', 'rpp-bak', 'ptx', 'ptf', 'song', 'reason', 'aup', 'aup3', 'band', 'ardour', 'dawproject'];
const PLUGIN_EXTS = ['vst', 'vst3', 'component', 'aaxplugin'];

function fileIcon(entry) {
    if (entry.isDir) return '&#128193;';
    const ext = entry.ext;
    if (AUDIO_EXTS.includes(ext)) return '&#127925;';
    if (DAW_EXTS.includes(ext)) return '&#127911;';
    if (PLUGIN_EXTS.includes(ext)) return '&#9889;';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(ext)) return '&#128247;';
    if (['pdf'].includes(ext)) return '&#128196;';
    if (['json', 'toml', 'xml', 'yaml', 'yml'].includes(ext)) return '&#128203;';
    if (['zip', 'gz', 'tar', 'rar', '7z', 'dmg'].includes(ext)) return '&#128230;';
    return '&#128196;';
}

// ── Bulk rename modal ──
// Multi-select rows → toolbar Rename... → modal with pattern fields.
// Pure JS — backend just receives per-file `rename_file` calls in a loop.
// Live preview table updates as the user types; rows highlighted red when
// a new name would conflict with another row's new name (would overwrite).

function _fbBulkRenameSelectedEntries() {
    if (!Array.isArray(_fileBrowserEntries) || _fileSelected.size === 0) return [];
    const out = [];
    for (const p of _fileSelected) {
        const e = _fileEntryByPath ? _fileEntryByPath(p) : null;
        if (e) out.push(e);
    }
    return out;
}

/** Compute the new name for a single entry given the current pattern fields. */
function _fbBulkRenameComputeName(entry, params, index) {
    const orig = entry.name;
    const dot = orig.lastIndexOf('.');
    const hasExt = !entry.isDir && dot > 0;
    let base = hasExt ? orig.slice(0, dot) : orig;
    const ext = hasExt ? orig.slice(dot) : '';
    // Find/replace on the basename only — preserve extension.
    if (params.find) {
        try {
            if (params.regex) {
                const re = new RegExp(params.find, 'g');
                base = base.replace(re, params.replace);
            } else {
                base = base.split(params.find).join(params.replace);
            }
        } catch (_) {
            // Invalid regex — leave base unchanged; UI will surface no change.
        }
    }
    // Placeholder substitution. {n} = padded number; {name}, {ext} for templates.
    const padded = String((params.numStart || 0) + index).padStart(Math.max(1, params.numPad || 1), '0');
    const subs = (s) => String(s || '')
        .replace(/\{n\}/g, padded)
        .replace(/\{name\}/g, base)
        .replace(/\{ext\}/g, ext.replace(/^\./, ''));
    const prefix = subs(params.prefix);
    const suffix = subs(params.suffix);
    return `${prefix}${base}${suffix}${ext}`;
}

function _fbBulkRenameCollectParams() {
    const $ = (id) => document.getElementById(id);
    return {
        find: $('fbBulkRenameFind')?.value || '',
        replace: $('fbBulkRenameReplace')?.value || '',
        regex: !!$('fbBulkRenameRegex')?.checked,
        prefix: $('fbBulkRenamePrefix')?.value || '',
        suffix: $('fbBulkRenameSuffix')?.value || '',
        numStart: Number($('fbBulkRenameNumStart')?.value) || 0,
        numPad: Number($('fbBulkRenameNumPad')?.value) || 1,
    };
}

function _fbBulkRenameRenderPreview() {
    if (typeof document === 'undefined') return;
    const params = _fbBulkRenameCollectParams();
    const entries = _fbBulkRenameSelectedEntries();
    const previewEl = document.getElementById('fbBulkRenamePreview');
    const status = document.getElementById('fbBulkRenameStatus');
    const apply = document.getElementById('fbBulkRenameApply');
    if (!previewEl) return;
    const rows = [];
    const newNameCounts = new Map();
    let changed = 0;
    for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        const newName = _fbBulkRenameComputeName(e, params, i);
        newNameCounts.set(newName, (newNameCounts.get(newName) || 0) + 1);
        rows.push({orig: e.name, next: newName, path: e.path});
        if (newName !== e.name) changed++;
    }
    const tbody = rows.map((r) => {
        const conflict = newNameCounts.get(r.next) > 1;
        const unchanged = r.next === r.orig;
        const cls = conflict ? 'conflict' : (unchanged ? 'unchanged' : '');
        return `<tr class="${cls}"><td>${escapeHtml(r.orig)}</td><td>${escapeHtml(r.next)}</td></tr>`;
    }).join('');
    previewEl.innerHTML = `<table>
        <thead><tr><th>Current</th><th>New</th></tr></thead>
        <tbody>${tbody}</tbody>
    </table>`;
    const conflicts = [...newNameCounts.values()].filter((n) => n > 1).length;
    if (status) {
        status.textContent = conflicts > 0
            ? `${conflicts} conflict${conflicts === 1 ? '' : 's'} · ${changed} change${changed === 1 ? '' : 's'}`
            : `${changed} change${changed === 1 ? '' : 's'} pending`;
    }
    if (apply) {
        // Disable Apply when there are conflicts (would overwrite each other)
        // or when nothing would change.
        apply.disabled = conflicts > 0 || changed === 0;
    }
}

function showFileBulkRenameModal() {
    if (typeof document === 'undefined') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal) return;
    const entries = _fbBulkRenameSelectedEntries();
    if (entries.length === 0) return;
    const countEl = document.getElementById('fbBulkRenameCount');
    if (countEl) countEl.textContent = `(${entries.length} selected)`;
    // Reset fields to sensible defaults on open.
    const reset = (id, val) => { const el = document.getElementById(id); if (el) el.value = val; };
    reset('fbBulkRenameFind', '');
    reset('fbBulkRenameReplace', '');
    reset('fbBulkRenamePrefix', '');
    reset('fbBulkRenameSuffix', '');
    reset('fbBulkRenameNumStart', '1');
    reset('fbBulkRenameNumPad', '3');
    const regex = document.getElementById('fbBulkRenameRegex');
    if (regex) regex.checked = false;
    modal.classList.add('modal-visible');
    _fbBulkRenameRenderPreview();
    // Focus the Find field for keyboard-first workflow.
    requestAnimationFrame(() => {
        const find = document.getElementById('fbBulkRenameFind');
        if (find) find.focus();
    });
}

function hideFileBulkRenameModal() {
    if (typeof document === 'undefined') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (modal) modal.classList.remove('modal-visible');
}

async function applyFileBulkRename() {
    const params = _fbBulkRenameCollectParams();
    const entries = _fbBulkRenameSelectedEntries();
    if (entries.length === 0) return;
    let success = 0;
    let fail = 0;
    for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        const newName = _fbBulkRenameComputeName(e, params, i);
        if (newName === e.name) continue;
        const dir = e.path.replace(/\/[^/]+$/, '');
        const newPath = `${dir}/${newName}`;
        try {
            await window.vstUpdater.renameFile(e.path, newPath);
            success++;
        } catch (_) {
            fail++;
        }
    }
    if (typeof showToast === 'function') {
        if (fail === 0) showToast(toastFmt('toast.deleted_name', {name: `renamed ${success} file${success === 1 ? '' : 's'}`}));
        else showToast(toastFmt('toast.failed', {err: `${fail} of ${success + fail} rename${success + fail === 1 ? '' : 's'} failed`}), 4000, 'error');
    }
    hideFileBulkRenameModal();
    if (typeof clearFileSelection === 'function') clearFileSelection();
    if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
}

// ── PDF preview cache helpers (canvas ↔ PNG bytes) ──
// `canvas.toBlob` is async; wrap in a Promise that resolves to a Uint8Array
// of the PNG bytes (suitable for `pdf_preview_set` which expects Vec<u8>).
// Used after PDF.js renders a page so the result can be cached in SQLite.
async function _fbCanvasToPngBytes(canvas) {
    return new Promise((resolve, reject) => {
        try {
            canvas.toBlob(async (blob) => {
                if (!blob) { resolve(null); return; }
                try {
                    const buf = await blob.arrayBuffer();
                    resolve(new Uint8Array(buf));
                } catch (e) { reject(e); }
            }, 'image/png');
        } catch (e) { reject(e); }
    });
}

/** Paints raw PNG bytes (returned by `pdf_preview_get`) into a canvas.
 *  Used on cache HIT — avoids the PDF.js module load + render entirely. */
async function _fbPaintPngBytesIntoCanvas(canvas, bytes) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    // Construct a Blob with the right MIME → object URL → Image → drawImage.
    // ObjectURL revoked after draw to free memory.
    return new Promise((resolve, reject) => {
        let url = null;
        try {
            const blob = new Blob([u8], {type: 'image/png'});
            url = URL.createObjectURL(blob);
            const img = new Image();
            img.onload = () => {
                canvas.width = img.naturalWidth;
                canvas.height = img.naturalHeight;
                const ctx = canvas.getContext('2d');
                ctx.drawImage(img, 0, 0);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.onerror = (e) => {
                if (url) URL.revokeObjectURL(url);
                reject(e);
            };
            img.src = url;
        } catch (e) {
            if (url) URL.revokeObjectURL(url);
            reject(e);
        }
    });
}

// ── PDF.js lazy-loader (shared across preview pane + PDF inventory thumbs) ──
// Module-level promise so concurrent calls share the same load. PDF.js is
// ~350 KB (main) + ~1.4 MB (worker); deferring until the first PDF render
// keeps it out of the initial bundle. Absolute path from WebView root —
// frontend is served from `frontend/` so `/lib/...` resolves correctly.
// Exposed on `window` so `pdf.js` (PDF inventory tab) can reuse without
// duplicating the loader / canvas-blob helpers below.
var _pdfJsPromise = null;
function loadPdfJs() {
    if (_pdfJsPromise) return _pdfJsPromise;
    _pdfJsPromise = (async () => {
        const mod = await import('/lib/pdf.min.mjs');
        if (mod && mod.GlobalWorkerOptions) {
            mod.GlobalWorkerOptions.workerSrc = '/lib/pdf.worker.min.mjs';
        }
        return mod;
    })().catch((err) => {
        _pdfJsPromise = null; // allow retry on next preview
        throw err;
    });
    return _pdfJsPromise;
}
if (typeof window !== 'undefined') {
    window.loadPdfJs = loadPdfJs;
    window.canvasToPngBytes = _fbCanvasToPngBytes;
    window.paintPngBytesIntoCanvas = _fbPaintPngBytesIntoCanvas;
}

// ── Inline preview pane (right side of file list) ──
// Persistent side panel that updates whenever a file row is focused or
// clicked. Toggled via the toolbar Preview button or Cmd+I. Per-extension
// content:
//   - audio  → 800-wide waveform + grid of audio metadata
//   - image  → thumbnail (loaded as base64 data URL via `fs_read_file_base64`,
//              capped at 2 MiB to prevent the WebView stalling on huge JPEGs)
//   - text   → first 4 KiB via `fs_read_head`, in a scrollable `<pre>`
//   - other  → just the basic metadata (size / date / path)
const FB_PREVIEW_IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'ico'];
const FB_PREVIEW_TEXT_EXTS = ['txt', 'md', 'json', 'toml', 'xml', 'yaml', 'yml', 'log', 'csv', 'tsv', 'sh', 'js', 'ts', 'rs', 'py', 'rb', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'css', 'html', 'env', 'gitignore'];
const FB_PREVIEW_IMAGE_CAP = 2 * 1024 * 1024;

/** Currently-previewed path — used to skip redundant fetches when the same
 *  row is re-clicked, and to abandon in-flight previews when selection moves. */
var _fbPreviewPath = null;
var _fbPreviewSeq = 0;

function isPreviewPaneVisible() {
    if (typeof document === 'undefined') return false;
    const pane = document.getElementById('fbPreviewPane');
    return !!(pane && !pane.classList.contains('fb-hidden'));
}

function setPreviewPaneVisible(visible) {
    if (typeof document === 'undefined') return;
    const pane = document.getElementById('fbPreviewPane');
    if (!pane) return;
    pane.classList.toggle('fb-hidden', !visible);
    try { prefs.setItem('fileBrowserPreviewVisible', visible ? '1' : '0'); } catch (_) { /* ignore */ }
    if (visible && _fbPreviewPath) populatePreviewPane(_fbPreviewPath);
}

function toggleFileBrowserPreviewPane() {
    setPreviewPaneVisible(!isPreviewPaneVisible());
}

function _fbPreviewMimeFromExt(ext) {
    switch (ext) {
        case 'jpg': case 'jpeg': return 'image/jpeg';
        case 'png': return 'image/png';
        case 'gif': return 'image/gif';
        case 'webp': return 'image/webp';
        case 'bmp': return 'image/bmp';
        case 'svg': return 'image/svg+xml';
        case 'ico': return 'image/x-icon';
        default: return 'application/octet-stream';
    }
}

async function populatePreviewPane(filePath) {
    if (typeof document === 'undefined' || !filePath) return;
    if (!isPreviewPaneVisible()) {
        _fbPreviewPath = filePath; // remember; will populate when toggled on
        return;
    }
    _fbPreviewPath = filePath;
    const seq = ++_fbPreviewSeq;
    const title = document.getElementById('fbPreviewTitle');
    const body = document.getElementById('fbPreviewBody');
    if (!body) return;
    const name = filePath.split('/').pop();
    const ext = (name.split('.').pop() || '').toLowerCase();
    if (title) title.textContent = name;
    body.innerHTML = '<div class="fb-preview-empty">Loading…</div>';

    const isAudio = typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
    const isImage = FB_PREVIEW_IMAGE_EXTS.includes(ext);
    const isText = FB_PREVIEW_TEXT_EXTS.includes(ext);

    // Build basic metadata grid common to all types.
    const entry = (typeof _fileEntryByPath === 'function') ? _fileEntryByPath(filePath) : null;
    const metaRows = [
        ['Path', escapeHtml(filePath)],
        ['Type', escapeHtml(ext || '—')],
    ];
    if (entry && !entry.isDir) {
        if (entry.sizeFormatted) metaRows.push(['Size', escapeHtml(entry.sizeFormatted)]);
        if (entry.modified) metaRows.push(['Modified', escapeHtml(entry.modified)]);
        if (entry.created) metaRows.push(['Created', escapeHtml(entry.created)]);
    }
    const metaHtml = `<dl class="fb-preview-meta">${metaRows.map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${v}</dd>`).join('')}</dl>`;

    if (isAudio) {
        body.innerHTML = `
            <canvas class="fb-preview-wf" id="fbPreviewWf" data-wf-path="${escapeHtml(filePath)}" width="600" height="80"></canvas>
            ${metaHtml}
        `;
        if (typeof drawMiniWaveform === 'function') {
            const canvas = document.getElementById('fbPreviewWf');
            if (canvas) drawMiniWaveform(canvas, filePath);
        }
        return;
    }

    if (isImage) {
        try {
            const b64 = await window.vstUpdater.fsReadFileBase64(filePath, FB_PREVIEW_IMAGE_CAP);
            if (seq !== _fbPreviewSeq) return; // selection moved during the load
            const mime = _fbPreviewMimeFromExt(ext);
            body.innerHTML = `<img class="fb-preview-image" src="data:${mime};base64,${b64}" alt="${escapeHtml(name)}">${metaHtml}`;
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            // Most common failure is "File too large" (over the 2 MiB cap).
            body.innerHTML = `<div class="fb-preview-empty">Image too large to preview</div>${metaHtml}`;
        }
        return;
    }

    if (isText) {
        try {
            const head = await window.vstUpdater.fsReadHead(filePath, 4096);
            if (seq !== _fbPreviewSeq) return;
            body.innerHTML = `<pre class="fb-preview-text">${escapeHtml(head)}</pre>${metaHtml}`;
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            body.innerHTML = `<div class="fb-preview-empty">Could not read file</div>${metaHtml}`;
        }
        return;
    }

    if (ext === 'pdf') {
        const FB_PDF_PREVIEW_WIDTH = 320;
        const FB_PDF_PREVIEW_PAGE = 1;
        body.innerHTML = `
            <canvas class="fb-preview-pdf-canvas" id="fbPreviewPdfCanvas"></canvas>
            ${metaHtml}
        `;
        const canvas = document.getElementById('fbPreviewPdfCanvas');
        if (!canvas) return;

        // 1) Try SQLite cache first — a fresh hit skips both the PDF.js
        //    lazy module load (~50-200 ms) and the page render (~tens-of-ms
        //    for typical pages, seconds for complex ones).
        try {
            const cached = await window.vstUpdater.pdfPreviewGet(filePath, FB_PDF_PREVIEW_PAGE, FB_PDF_PREVIEW_WIDTH);
            if (seq !== _fbPreviewSeq) return;
            if (cached && cached.length > 0) {
                await _fbPaintPngBytesIntoCanvas(canvas, cached);
                if (seq !== _fbPreviewSeq) return;
                return;
            }
        } catch (_) { /* cache miss / IPC error → fall through to render */ }

        // 2) Cache miss → render via PDF.js, paint canvas, persist to cache.
        try {
            const bytes = await window.vstUpdater.fsReadFileBytes(filePath, 32 * 1024 * 1024);
            if (seq !== _fbPreviewSeq) return;
            const pdfjs = await loadPdfJs();
            if (seq !== _fbPreviewSeq) return;
            const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
            const pdf = await pdfjs.getDocument({data: u8}).promise;
            if (seq !== _fbPreviewSeq) return;
            const page = await pdf.getPage(FB_PDF_PREVIEW_PAGE);
            if (seq !== _fbPreviewSeq) return;
            const baseViewport = page.getViewport({scale: 1.0});
            const scale = Math.min(1.0, FB_PDF_PREVIEW_WIDTH / baseViewport.width);
            const viewport = page.getViewport({scale});
            canvas.width = Math.round(viewport.width);
            canvas.height = Math.round(viewport.height);
            const ctx = canvas.getContext('2d');
            await page.render({canvasContext: ctx, viewport}).promise;
            if (seq !== _fbPreviewSeq) return;
            // 3) Persist render to cache for next time. Fire-and-forget —
            //    a write failure shouldn't surface to the user.
            try {
                const pngBytes = await _fbCanvasToPngBytes(canvas);
                if (pngBytes) {
                    window.vstUpdater.pdfPreviewSet(
                        filePath, FB_PDF_PREVIEW_PAGE, FB_PDF_PREVIEW_WIDTH, pngBytes,
                    ).catch(() => {});
                }
            } catch (_) { /* ignore cache-write errors */ }
        } catch (err) {
            if (seq !== _fbPreviewSeq) return;
            const msg = (err && (err.message || err)) ? String(err.message || err) : 'PDF preview unavailable';
            body.innerHTML = `<div class="fb-preview-empty">${escapeHtml(msg)}</div>${metaHtml}`;
        }
        return;
    }

    // No type-specific preview — show just the metadata.
    body.innerHTML = metaHtml;
}

// ── Quick-look overlay (Space key → big preview modal) ──
// Builds and shows a centered modal with file metadata. Dismissed by Esc,
// click-outside, or pressing Space again. For audio files, embeds a larger
// waveform that reuses the existing `drawMiniWaveform` helper. For other
// types, shows path / size / dates only — richer per-type preview
// (image/PDF body) is the job of the dedicated preview pane task.
function _ensureQuickLookOverlay() {
    if (typeof document === 'undefined') return null;
    let overlay = document.getElementById('fbQuickLook');
    if (overlay) return overlay;
    overlay = document.createElement('div');
    overlay.id = 'fbQuickLook';
    overlay.className = 'fb-quicklook fb-hidden';
    overlay.innerHTML = `
        <div class="fb-quicklook-card">
            <div class="fb-quicklook-header">
                <span class="fb-quicklook-title" id="fbQuickLookTitle"></span>
                <button class="fb-quicklook-close" data-action="fbQuickLookClose" title="Close (Esc)">&times;</button>
            </div>
            <div class="fb-quicklook-body" id="fbQuickLookBody"></div>
        </div>
    `;
    document.body.appendChild(overlay);
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay || e.target.closest('[data-action="fbQuickLookClose"]')) hideQuickLook();
    });
    return overlay;
}

function showQuickLook(filePath) {
    if (typeof document === 'undefined' || !filePath) return;
    const overlay = _ensureQuickLookOverlay();
    if (!overlay) return;
    const title = document.getElementById('fbQuickLookTitle');
    const body = document.getElementById('fbQuickLookBody');
    const name = filePath.split('/').pop();
    const ext = (name.split('.').pop() || '').toLowerCase();
    if (title) title.textContent = name;
    if (body) {
        const isAudio = typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
        const wfHtml = isAudio
            ? `<canvas class="fb-quicklook-wf" id="fbQuickLookWf" data-wf-path="${escapeHtml(filePath)}" width="800" height="120"></canvas>`
            : '';
        body.innerHTML = `
            ${wfHtml}
            <div class="fb-quicklook-meta">
                <div class="fb-quicklook-row"><span class="fb-quicklook-label">Path</span><span class="fb-quicklook-val">${escapeHtml(filePath)}</span></div>
                <div class="fb-quicklook-row"><span class="fb-quicklook-label">Type</span><span class="fb-quicklook-val">${escapeHtml(ext || '—')}</span></div>
            </div>
            <div class="fb-quicklook-hint">Press Esc or Space to close</div>
        `;
        if (isAudio && typeof drawMiniWaveform === 'function') {
            const canvas = document.getElementById('fbQuickLookWf');
            if (canvas) drawMiniWaveform(canvas, filePath);
        }
    }
    overlay.classList.remove('fb-hidden');
}

function hideQuickLook() {
    if (typeof document === 'undefined') return;
    const overlay = document.getElementById('fbQuickLook');
    if (overlay) overlay.classList.add('fb-hidden');
}

function isQuickLookVisible() {
    if (typeof document === 'undefined') return false;
    const overlay = document.getElementById('fbQuickLook');
    return !!(overlay && !overlay.classList.contains('fb-hidden'));
}

// ── Inline rename (F2 on focused row, or via context menu) ──
// Replaces the row's `.file-name` text node with a borderless `<input>`,
// pre-filled and selected. Enter commits via `rename_file` IPC; Esc cancels;
// blur cancels (treats unintentional click-away as cancel). After successful
// rename, reload the directory so the row's path / placement update.
async function _commitFileRename(oldPath, input, row) {
    const next = (input.value || '').trim();
    const dir = oldPath.replace(/\/[^/]+$/, '');
    const newPath = `${dir}/${next}`;
    if (!next || next === oldPath.split('/').pop()) {
        _cancelFileRename(input, row);
        return;
    }
    try {
        await window.vstUpdater.renameFile(oldPath, newPath);
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `renamed to ${next}`}));
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
        _cancelFileRename(input, row);
    }
}

function _cancelFileRename(input, row) {
    if (!row || !input) return;
    const nameCell = row.querySelector('.file-name');
    if (nameCell && nameCell._origHtml != null) {
        nameCell.innerHTML = nameCell._origHtml;
        delete nameCell._origHtml;
    }
}

function startFileRename(row) {
    if (!row) return;
    const oldPath = row.dataset.filePath;
    if (!oldPath) return;
    const nameCell = row.querySelector('.file-name');
    if (!nameCell || nameCell.querySelector('input.fb-rename-input')) return;
    const baseName = oldPath.split('/').pop();
    // Stash existing HTML so cancel can restore the highlight / badges / note pin.
    nameCell._origHtml = nameCell.innerHTML;
    nameCell.innerHTML = `<input class="fb-rename-input" type="text" autocomplete="off" autocorrect="off" spellcheck="false">`;
    const input = nameCell.querySelector('input.fb-rename-input');
    input.value = baseName;
    input.focus();
    // Select everything except the extension (matches Finder / Explorer UX).
    const dot = baseName.lastIndexOf('.');
    if (dot > 0) input.setSelectionRange(0, dot);
    else input.select();
    let done = false;
    const finish = (commit) => {
        if (done) return;
        done = true;
        if (commit) _commitFileRename(oldPath, input, row);
        else _cancelFileRename(input, row);
    };
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') { e.preventDefault(); finish(true); }
        else if (e.key === 'Escape') { e.preventDefault(); finish(false); }
        e.stopPropagation();
    });
    input.addEventListener('blur', () => { finish(false); });
}

// ── New Folder (button + Cmd+Shift+N) ──
async function fileBrowserNewFolder() {
    if (!_fileBrowserPath) return;
    const name = window.prompt(appFmt('confirm.delete_file_browser', {name: 'new folder name'}).replace(/Delete.*/, 'New folder name:'), 'untitled folder');
    if (!name) return;
    const cleaned = name.trim();
    if (!cleaned) return;
    const newPath = `${_fileBrowserPath}/${cleaned}`;
    try {
        await window.vstUpdater.fsCreateDir(newPath);
        if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `created ${cleaned}`}));
        loadDirectory(_fileBrowserPath);
    } catch (err) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
    }
}

// ── Move-to-bookmark (right-click → Move to → <bookmark>) ──
// Returns an array of context-menu items, one per saved favorite dir, each
// of which moves the path to that bookmark when clicked. Empty array when
// no bookmarks exist — the caller can omit the submenu in that case.
function buildMoveToBookmarkMenuItems(srcPath) {
    if (typeof getFavDirs !== 'function') return [];
    const dirs = getFavDirs();
    if (!Array.isArray(dirs) || dirs.length === 0) return [];
    return dirs.map((d) => ({
        icon: '&#128193;',
        label: `${d.name}`,
        action: async () => {
            const base = srcPath.split('/').pop();
            const target = `${d.path}/${base}`;
            try {
                await window.vstUpdater.renameFile(srcPath, target);
                if (typeof showToast === 'function') showToast(toastFmt('toast.deleted_name', {name: `moved ${base} → ${d.name}`}));
                if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
            } catch (err) {
                if (typeof showToast === 'function') showToast(toastFmt('toast.failed', {err: err.message || err}), 4000, 'error');
            }
        },
    }));
}

// ── Forward/back navigation history ──
// Browser-style nav: each `loadDirectory` push appends to the history stack
// unless the load was itself triggered by back/forward (which sets the skip
// flag). Forward history is dropped on a new push (matches browser semantics).
var _fbHistory = [];
var _fbHistoryIdx = -1;
var _fbHistorySkipPush = false;

function _navHistoryRecord(path) {
    if (_fbHistorySkipPush) {
        _fbHistorySkipPush = false;
        return;
    }
    if (_fbHistory[_fbHistoryIdx] === path) return; // no-op nav to same dir
    // Drop forward history past the current index (new branch).
    _fbHistory = _fbHistory.slice(0, _fbHistoryIdx + 1);
    _fbHistory.push(path);
    _fbHistoryIdx = _fbHistory.length - 1;
    _updateNavButtons();
}

function _updateNavButtons() {
    if (typeof document === 'undefined') return;
    const back = document.getElementById('fbNavBack');
    const fwd = document.getElementById('fbNavFwd');
    if (back) back.disabled = _fbHistoryIdx <= 0;
    if (fwd) fwd.disabled = _fbHistoryIdx >= _fbHistory.length - 1;
}

function fileNavBack() {
    if (_fbHistoryIdx <= 0) return;
    _fbHistoryIdx--;
    _fbHistorySkipPush = true;
    loadDirectory(_fbHistory[_fbHistoryIdx]);
}

function fileNavForward() {
    if (_fbHistoryIdx >= _fbHistory.length - 1) return;
    _fbHistoryIdx++;
    _fbHistorySkipPush = true;
    loadDirectory(_fbHistory[_fbHistoryIdx]);
}

// ── Cmd+L editable path input ──
function showFilePathEditor() {
    if (typeof document === 'undefined') return;
    const input = document.getElementById('fileBrowserPathInput');
    const breadcrumb = document.getElementById('fileBreadcrumb');
    if (!input || !breadcrumb) return;
    input.value = _fileBrowserPath || '';
    input.classList.remove('fb-hidden');
    breadcrumb.classList.add('fb-hidden');
    input.focus();
    input.select();
}

function hideFilePathEditor() {
    if (typeof document === 'undefined') return;
    const input = document.getElementById('fileBrowserPathInput');
    const breadcrumb = document.getElementById('fileBreadcrumb');
    if (!input || !breadcrumb) return;
    input.classList.add('fb-hidden');
    breadcrumb.classList.remove('fb-hidden');
}

// ── Extension filter chips ──
// Categories map to existing global extension sets where available
// (`AUDIO_EXTS`, `DAW_EXTS`). Preset / MIDI / PDF / video / image use
// dedicated lists below. Folders are always visible regardless of chip so
// the user can navigate.
var _fbExtFilter = 'all';
var _fbExtCategories = null; // lazily built after globals are loaded

function _fbGetExtCategories() {
    if (_fbExtCategories) return _fbExtCategories;
    const audio = (typeof AUDIO_EXTS !== 'undefined') ? AUDIO_EXTS : [];
    const daw = (typeof DAW_EXTS !== 'undefined') ? DAW_EXTS : [];
    const preset = ['fxp', 'fxb', 'preset', 'aupreset', 'h2p', 'nksf', 'nksfx', 'adv', 'agr', 'vstpreset', 'cmb', 'patch', 'nrkt'];
    const midi = ['mid', 'midi'];
    const pdf = ['pdf'];
    const video = ['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v', 'wmv', 'flv', 'mpg', 'mpeg'];
    const image = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp', 'tiff', 'tif', 'ico'];
    _fbExtCategories = {audio, daw, preset, midi, pdf, video, image};
    return _fbExtCategories;
}

function _fbExtMatches(entry, category) {
    if (category === 'all') return true;
    if (entry.isDir) return true; // folders always visible — navigation can't break
    const cats = _fbGetExtCategories();
    if (category === 'other') {
        const inAny = [
            ...cats.audio, ...cats.daw, ...cats.preset, ...cats.midi,
            ...cats.pdf, ...cats.video, ...cats.image,
        ].includes(entry.ext);
        return !inAny;
    }
    const list = cats[category];
    return Array.isArray(list) && list.includes(entry.ext);
}

function setFileExtFilter(category) {
    _fbExtFilter = category || 'all';
    if (typeof document !== 'undefined') {
        document.querySelectorAll('.fb-ext-chips .ext-chip').forEach((c) => {
            c.classList.toggle('active', c.dataset.extFilter === _fbExtFilter);
        });
    }
    if (typeof renderFileList === 'function') renderFileList();
}

// ── Footer status line ──
function _fbFormatBytes(n) {
    if (!Number.isFinite(n) || n <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const idx = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
    return `${(n / Math.pow(1024, idx)).toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}

function updateFileListFooter(entries) {
    if (typeof document === 'undefined') return;
    const el = document.getElementById('fileListFooter');
    if (!el) return;
    const arr = Array.isArray(entries) ? entries : [];
    let audio = 0;
    let totalBytes = 0;
    const audioExts = (typeof AUDIO_EXTS !== 'undefined') ? AUDIO_EXTS : [];
    for (const e of arr) {
        if (!e.isDir) {
            if (audioExts.includes(e.ext)) audio++;
            totalBytes += Number(e.size) || 0;
        }
    }
    el.textContent = `${arr.length} items · ${audio} audio · ${_fbFormatBytes(totalBytes)}`;
}

// ── Folder scan-status badges ──
// After each directory render, query the backend for per-folder inventory
// counts (samples / presets / DAW / MIDI / PDF / video under the folder
// prefix) and inject small color-coded badges into the folder row's
// `.file-name` cell so the user can see at a glance which folders are
// already in inventory vs need scanning.
//
// Per-directory cache keyed on directory path — re-entering the same
// directory replays from cache (cheap) instead of re-querying the backend.
// Cache is cleared on full `loadDirectory` since the listing changed.
var _scanStatusCache = new Map(); // dirPath → Map<folderPath, status>

/** Token-bumped on each call so a slow IPC from a previous render can't paint
 *  badges into a newer render's DOM. */
var _scanStatusSeq = 0;

/** Build the badge HTML fragment for a single folder's scan-status. Returns
 *  empty string when nothing's been scanned (no badges = uninventoried). */
function _scanStatusBadgesHtml(status) {
    if (!status) return '';
    const parts = [];
    const add = (kind, n, label) => {
        if (n > 0) parts.push(`<span class="scan-badge sb-${kind}" title="${label}: ${n}">${kind.toUpperCase()[0]} ${n}</span>`);
    };
    add('samples', status.samples, 'Samples');
    add('presets', status.presets, 'Presets');
    add('daw', status.daw, 'DAW projects');
    add('midi', status.midi, 'MIDI files');
    add('pdf', status.pdf, 'PDFs');
    add('video', status.video, 'Videos');
    if (parts.length === 0) return '';
    return `<span class="scan-badges">${parts.join('')}</span>`;
}

/** Inject the badge fragment into the matching folder row's name cell. The
 *  size cell is owned exclusively by `refreshFolderFilesystemSizes` (the
 *  background recursive walk) — the two paint paths must not race for the
 *  same cell. Badges only touch `.file-name`; size only `_paintFolderSizeCell`
 *  touches `.file-size`. */
function _applyScanBadgesToRow(folderPath, status) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row.file-dir[data-file-path="${CSS.escape(folderPath)}"]`);
        if (!row) return;
        const nameCell = row.querySelector('.file-name');
        if (!nameCell) return;
        const prior = nameCell.querySelector('.scan-badges');
        if (prior) prior.remove();
        const html = _scanStatusBadgesHtml(status);
        if (html) nameCell.insertAdjacentHTML('beforeend', html);
    } catch (_) { /* CSS.escape unavailable */ }
}

// ── Background filesystem size for every visible folder ──
// On each directory render we kick off a recursive `fs_folder_size` walk for
// every folder row. Results stream back and replace the per-row size cell as
// they arrive (initial cells show "…" placeholder). Bounded concurrency so a
// directory with 100 folders doesn't spawn 100 simultaneous walks; a
// sequence guard prevents results from an abandoned directory from painting
// into the new one. Session cache makes revisits instant.
var _fsFolderSizeCache = new Map();
var _fsFolderSizeInFlight = new Map();
/** Bumped on each `refreshFolderFilesystemSizes` call. Painted results check
 *  this to ensure they belong to the current render (otherwise a slow walk
 *  from `/Music` could paint into `/Downloads` after the user navigated). */
var _fsFolderSizeSeq = 0;
/** Concurrent in-flight walks cap. JUCE/SMB I/O is the bottleneck — 4 keeps
 *  thrashing bounded while still finishing typical directories in seconds. */
const FS_FOLDER_SIZE_CONCURRENCY = 4;
var _fsFolderSizeQueue = [];
var _fsFolderSizeActive = 0;

async function getFsFolderSize(folderPath) {
    if (_fsFolderSizeCache.has(folderPath)) return _fsFolderSizeCache.get(folderPath);
    if (_fsFolderSizeInFlight.has(folderPath)) return _fsFolderSizeInFlight.get(folderPath);
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderSize !== 'function') return null;
    const p = window.vstUpdater.fsFolderSize(folderPath, 2000)
        .then((result) => {
            // Newer backend returns `{bytes, files}`; older builds returned a
            // bare u64. Accept both so a freshly-updated JS works against a
            // not-yet-rebuilt binary during development.
            const bytes = (result && typeof result === 'object')
                ? (Number(result.bytes) || 0)
                : (Number(result) || 0);
            const files = (result && typeof result === 'object')
                ? (Number(result.files) || 0)
                : 0;
            const r = {bytes, files};
            _fsFolderSizeCache.set(folderPath, r);
            return r;
        })
        .catch(() => null)
        .finally(() => { _fsFolderSizeInFlight.delete(folderPath); });
    _fsFolderSizeInFlight.set(folderPath, p);
    return p;
}

function _pumpFsFolderSizeQueue() {
    while (_fsFolderSizeActive < FS_FOLDER_SIZE_CONCURRENCY && _fsFolderSizeQueue.length > 0) {
        const job = _fsFolderSizeQueue.shift();
        if (job.seq !== _fsFolderSizeSeq) continue; // directory changed before we got to it
        _fsFolderSizeActive++;
        getFsFolderSize(job.folderPath)
            .then((result) => {
                if (job.seq !== _fsFolderSizeSeq) return;
                _paintFolderSizeCell(job.folderPath, result);
                // Stash the walked size/count on the entry so the sort helper
                // can see real folder sizes (the initial `e.size` is 0 because
                // `metadata.len()` returns inode size for directories). Files
                // already have their immediate size from fs_list_dir.
                if (result && Array.isArray(_fileBrowserEntries)) {
                    for (const e of _fileBrowserEntries) {
                        if (e.isDir && e.path === job.folderPath) {
                            e.size = Number(result.bytes) || 0;
                            e.itemsCount = Number(result.files) || 0;
                            break;
                        }
                    }
                }
            })
            .finally(() => {
                _fsFolderSizeActive--;
                _pumpFsFolderSizeQueue();
                // When the queue drains: if the user is sorting by Size (or
                // any sort that depends on folder data populated by the walk),
                // reorder the existing rows in place — no full re-render, so
                // the painted size cells stay intact (no flicker).
                if (_fsFolderSizeActive === 0
                    && _fsFolderSizeQueue.length === 0
                    && job.seq === _fsFolderSizeSeq
                    && (_fileSortKey === 'size' || _fileSortKey === 'items')
                    && typeof _reorderVisibleRowsInPlace === 'function') {
                    _reorderVisibleRowsInPlace();
                }
            });
    }
}

/** Reorders the existing `.file-row` elements in `#fileList` per the current
 *  sort key/direction WITHOUT touching cell contents. Used after the
 *  filesystem-size walks finish so the Size sort reflects the just-walked
 *  folder sizes (initial render saw `e.size = 0` for folders). Search-mode
 *  results are score-ordered; this is a no-op there. */
function _reorderVisibleRowsInPlace() {
    if (typeof document === 'undefined') return;
    const list = document.getElementById('fileList');
    if (!list) return;
    const searchInput = document.getElementById('fileSearchInput');
    const hasSearch = !!(searchInput && searchInput.value && searchInput.value.trim());
    if (hasSearch) return;
    if (!Array.isArray(_fileBrowserEntries)) return;
    const preFiltered = (_fbExtFilter && _fbExtFilter !== 'all')
        ? _fileBrowserEntries.filter((e) => _fbExtMatches(e, _fbExtFilter))
        : _fileBrowserEntries;
    const sorted = applyFileSort(preFiltered);
    const rowByPath = new Map();
    for (const row of list.querySelectorAll('.file-row')) {
        rowByPath.set(row.dataset.filePath, row);
    }
    // `appendChild` on an existing DOM node MOVES it — no clone, no innerHTML
    // reset. Doing this inside a DocumentFragment limits to one reflow.
    const fragment = document.createDocumentFragment();
    for (const e of sorted) {
        const row = rowByPath.get(e.path);
        if (row) fragment.appendChild(row);
    }
    list.appendChild(fragment);
}

function _fbFormatItemCount(n) {
    if (!Number.isFinite(n) || n < 0) return '';
    if (n < 1000) return `${n}`;
    if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}k`;
    return `${(n / 1_000_000).toFixed(1)}M`;
}

/** Paints both the size cell AND the items cell for a folder row from the
 *  bg-walk result `{bytes, files}`. `null` (walk failed) → dash in both. */
function _paintFolderSizeCell(folderPath, result) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row.file-dir[data-file-path="${CSS.escape(folderPath)}"]`);
        if (!row) return;
        const sizeCell = row.querySelector('.file-size');
        const itemsCell = row.querySelector('.file-items');
        if (result === null || result === undefined) {
            if (sizeCell) {
                sizeCell.textContent = '—';
                sizeCell.classList.remove('file-size-loading');
                sizeCell.classList.add('file-size-dash');
                sizeCell.title = 'Folder size unavailable (timeout, permission, or unreadable)';
            }
            if (itemsCell) {
                itemsCell.textContent = '—';
                itemsCell.classList.remove('file-items-loading');
                itemsCell.classList.add('file-items-dash');
            }
            return;
        }
        const bytes = Number(result.bytes) || 0;
        const files = Number(result.files) || 0;
        if (sizeCell) {
            sizeCell.textContent = _fbFormatBytes(bytes);
            sizeCell.classList.remove('file-size-loading', 'file-size-dash', 'file-size-inv');
            sizeCell.title = `Filesystem total: ${sizeCell.textContent} · ${files.toLocaleString()} files`;
        }
        if (itemsCell) {
            itemsCell.textContent = _fbFormatItemCount(files);
            itemsCell.classList.remove('file-items-loading', 'file-items-dash');
            itemsCell.title = `${files.toLocaleString()} files (recursive)`;
        }
    } catch (_) { /* CSS.escape unavailable */ }
}

/** Called after `renderFileList` finishes painting. Enumerates the visible
 *  folder rows and either replays from cache (instant) or queues a walk. */
function refreshFolderFilesystemSizes() {
    if (typeof document === 'undefined' || !_fileBrowserPath) return;
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderSize !== 'function') return;
    const folders = Array.from(document.querySelectorAll('.file-row.file-dir'))
        .map((row) => row.dataset.filePath)
        .filter(Boolean);
    if (folders.length === 0) return;
    // Bump the sequence so any queued jobs from a prior render are skipped
    // when the pump reaches them (see check in `_pumpFsFolderSizeQueue`).
    const seq = ++_fsFolderSizeSeq;
    for (const folderPath of folders) {
        if (_fsFolderSizeCache.has(folderPath)) {
            _paintFolderSizeCell(folderPath, _fsFolderSizeCache.get(folderPath));
            continue;
        }
        _fsFolderSizeQueue.push({folderPath, seq});
    }
    _pumpFsFolderSizeQueue();
}

/** Kick off (or replay from cache) the scan-status fetch for the currently
 *  visible folder rows. Called after `renderFileList` finishes its chunked
 *  paint. Failure is silent — badges are a hint, not a requirement. */
async function refreshFolderScanBadges() {
    if (typeof document === 'undefined' || !_fileBrowserPath) return;
    if (!window.vstUpdater || typeof window.vstUpdater.fsFolderScanStatus !== 'function') return;

    const folders = Array.isArray(_fileBrowserEntries)
        ? _fileBrowserEntries.filter((e) => e && e.isDir).map((e) => e.path)
        : [];
    if (folders.length === 0) return;

    const dirKey = _fileBrowserPath;
    const cached = _scanStatusCache.get(dirKey);
    if (cached) {
        // Replay from cache immediately — cheap, avoids backend hop on revisit.
        for (const f of folders) {
            const s = cached.get(f);
            if (s) _applyScanBadgesToRow(f, s);
        }
        return;
    }

    const seq = ++_scanStatusSeq;
    try {
        const result = await window.vstUpdater.fsFolderScanStatus(folders);
        if (seq !== _scanStatusSeq) return; // a newer render started; abandon this paint
        if (_fileBrowserPath !== dirKey) return; // directory changed mid-flight
        const map = new Map();
        for (const folder of folders) {
            const status = (result && result[folder]) || null;
            if (status) {
                map.set(folder, status);
                _applyScanBadgesToRow(folder, status);
            }
        }
        _scanStatusCache.set(dirKey, map);
    } catch (_) { /* badges are non-essential */ }
}

// ── File-list multi-select + bulk operations ──
// Selected entry paths for the CURRENT directory. Cleared on directory change
// (paths from the old dir would be invalid bulk-action targets). `var` for the
// same VM-sandbox visibility reason as the sort state below.
var _fileSelected = new Set();
// Index of the last interacted-with checkbox row (in the post-sort visible
// order). Used as the anchor for shift-click range select.
var _fileSelectLastIdx = -1;

function _fileEntryByPath(path) {
    if (!Array.isArray(_fileBrowserEntries)) return null;
    for (let i = 0; i < _fileBrowserEntries.length; i++) {
        if (_fileBrowserEntries[i].path === path) return _fileBrowserEntries[i];
    }
    return null;
}

function _setFileRowSelectedClass(path, selected) {
    if (typeof document === 'undefined' || typeof CSS === 'undefined') return;
    try {
        const row = document.querySelector(`.file-row[data-file-path="${CSS.escape(path)}"]`);
        if (row) row.classList.toggle('file-selected', selected);
    } catch (_) { /* CSS.escape unavailable in old engines or test sandbox */ }
}

function updateFileBulkBar() {
    if (typeof document === 'undefined') return;
    const bar = document.getElementById('fileBulkBar');
    const count = document.getElementById('fileBulkCount');
    if (!bar) return;
    // `.fb-hidden` (CSS class with `display: none !important`) instead of
    // `style.display` — release WebKit handles class toggles more reliably
    // than inline display on dynamic flex containers.
    bar.classList.toggle('fb-hidden', _fileSelected.size === 0);
    if (count) count.textContent = String(_fileSelected.size);
    // Header `select-all` checkbox reflects current selection: checked when
    // all visible rows are selected, otherwise unchecked.
    const all = document.getElementById('fileRowCbAll');
    if (all) {
        const cbs = document.querySelectorAll('.file-row-cb');
        all.checked = cbs.length > 0 && _fileSelected.size >= cbs.length;
    }
}

function toggleFileSelect(path, selected) {
    if (selected) _fileSelected.add(path);
    else _fileSelected.delete(path);
    _setFileRowSelectedClass(path, selected);
    updateFileBulkBar();
}

function clearFileSelection() {
    const prev = [..._fileSelected];
    _fileSelected.clear();
    for (const p of prev) _setFileRowSelectedClass(p, false);
    if (typeof document !== 'undefined') {
        document.querySelectorAll('.file-row-cb').forEach((cb) => { cb.checked = false; });
    }
    _fileSelectLastIdx = -1;
    updateFileBulkBar();
}

function selectAllVisibleFiles() {
    if (typeof document === 'undefined') return;
    document.querySelectorAll('.file-row-cb').forEach((cb) => {
        cb.checked = true;
        const path = cb.dataset.fbCb;
        if (path) {
            _fileSelected.add(path);
            _setFileRowSelectedClass(path, true);
        }
    });
    updateFileBulkBar();
}

/**
 * Single source of truth for "what paths are bulk-action targets given the
 * current selection." Filters by predicate for actions that only make sense
 * for one kind of entry (folders for scan-as-*, files for open-in-default-app
 * et al.).
 */
function _fileBulkSelectionAsPaths(filterFn) {
    const out = [];
    for (const path of _fileSelected) {
        const entry = _fileEntryByPath(path);
        if (!entry) continue;
        if (filterFn && !filterFn(entry)) continue;
        out.push(path);
    }
    return out;
}

// ── File-list column resize ──
// Column widths live in CSS custom properties on `#tabFiles` so a single
// `style.setProperty('--fb-w-…')` write resizes header + every row together.
// Drag handlers are wired once at module load (delegated). Saved widths are
// keyed per column in `prefs.fileBrowserColWidths`.
var _fbColumnResizeWired = false;
const FB_RESIZABLE_COLS = ['ext', 'size', 'items', 'date', 'created'];

function saveFileColumnWidths() {
    if (typeof document === 'undefined') return;
    const tab = document.getElementById('tabFiles');
    if (!tab) return;
    const widths = {};
    for (const col of FB_RESIZABLE_COLS) {
        const v = tab.style.getPropertyValue(`--fb-w-${col}`);
        if (v) widths[col] = v.trim();
    }
    try { prefs.setItem('fileBrowserColWidths', widths); } catch (_) { /* ignore */ }
}

function loadFileColumnWidths() {
    if (typeof document === 'undefined') return;
    const tab = document.getElementById('tabFiles');
    if (!tab) return;
    try {
        const widths = prefs.getObject('fileBrowserColWidths', null);
        if (!widths || typeof widths !== 'object') return;
        for (const [col, w] of Object.entries(widths)) {
            if (!FB_RESIZABLE_COLS.includes(col)) continue;
            if (typeof w !== 'string' || !w) continue;
            tab.style.setProperty(`--fb-w-${col}`, w);
        }
    } catch (_) { /* ignore */ }
}

function initFileColumnResize() {
    if (_fbColumnResizeWired || typeof document === 'undefined') return;
    _fbColumnResizeWired = true;
    document.addEventListener('mousedown', (e) => {
        const handle = e.target && e.target.closest
            ? e.target.closest('.file-list-header .fb-col-resize')
            : null;
        if (!handle) return;
        const col = handle.dataset.fbResize;
        if (!FB_RESIZABLE_COLS.includes(col)) return;
        const tab = document.getElementById('tabFiles');
        if (!tab) return;
        e.preventDefault();
        e.stopPropagation();
        // Use the rendered px width (computed from the current CSS var) as the
        // starting point — handles the first-drag case where the var hasn't
        // been explicitly set yet.
        const headerCell = handle.closest('[data-fb-col]');
        const startWidth = headerCell ? headerCell.offsetWidth : 70;
        const startX = e.clientX;
        const minWidth = 40;
        handle.classList.add('resizing');
        document.body.classList.add('fb-col-resizing');
        function onMove(ev) {
            const delta = ev.clientX - startX;
            const next = Math.max(minWidth, startWidth + delta);
            tab.style.setProperty(`--fb-w-${col}`, `${next}px`);
        }
        function onUp() {
            handle.classList.remove('resizing');
            document.body.classList.remove('fb-col-resizing');
            document.removeEventListener('mousemove', onMove);
            document.removeEventListener('mouseup', onUp);
            saveFileColumnWidths();
        }
        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onUp);
    });
}

// ── File-list column sort ──
// Global (not per-directory) sort state. Stored in prefs as `{key, asc}`.
// Folders always sort first regardless of direction (standard file-browser
// convention — prevents folders from getting lost mid-list when sorting
// large directories by size/date).
// `var` (not `let`) so the VM-loaded test harness can read/mutate via
// `globalThis` to exercise the sort helpers without booting the full UI.
var _fileSortKey = 'name';
var _fileSortAsc = true;

function loadFileSortFromPrefs() {
    try {
        const raw = prefs.getObject('fileSort', null);
        if (raw && typeof raw === 'object') {
            if (['name', 'size', 'date', 'type', 'items', 'created'].includes(raw.key)) _fileSortKey = raw.key;
            if (typeof raw.asc === 'boolean') _fileSortAsc = raw.asc;
        }
    } catch (_) { /* ignore */ }
}

function saveFileSortToPrefs() {
    try {
        // Pass the raw object; `prefs.setItem` JSON-stringifies internally,
        // mirroring the `favDirs` save path above (and matching what
        // `prefs.getObject` expects to read back).
        prefs.setItem('fileSort', {key: _fileSortKey, asc: _fileSortAsc});
    } catch (_) { /* ignore */ }
}

/**
 * Sort a copy of the entries array by the current global sort key/direction.
 * Folders always come first within each direction. Secondary sort is always
 * name (asc) so equal-size or equal-date files stay alphabetical.
 */
function applyFileSort(entries) {
    const key = _fileSortKey;
    const dirMul = _fileSortAsc ? 1 : -1;
    const sorted = entries.slice();
    sorted.sort((a, b) => {
        if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
        let cmp = 0;
        switch (key) {
            case 'size':
                cmp = (Number(a.size) || 0) - (Number(b.size) || 0);
                break;
            case 'items':
                // `itemsCount` is set on folder entries by the bg walk in
                // `_pumpFsFolderSizeQueue`. Files don't have one — both sides
                // fall back to 0, which leaves files alphabetical via the
                // secondary name sort below.
                cmp = (Number(a.itemsCount) || 0) - (Number(b.itemsCount) || 0);
                break;
            case 'date':
                // `modified` is `YYYY-MM-DD HH:MM` (lexically sortable per `fs_list_dir`).
                cmp = String(a.modified || '').localeCompare(String(b.modified || ''));
                break;
            case 'created':
                // Same format as `modified`; empty when FS doesn't track
                // birthtime — empties sort to the start (asc) / end (desc)
                // since localeCompare treats them as smallest.
                cmp = String(a.created || '').localeCompare(String(b.created || ''));
                break;
            case 'type':
                cmp = String(a.ext || '').localeCompare(String(b.ext || ''));
                break;
            case 'name':
            default:
                cmp = String(a.name || '').toLowerCase().localeCompare(String(b.name || '').toLowerCase());
                break;
        }
        if (cmp === 0 && key !== 'name') {
            return String(a.name || '').toLowerCase().localeCompare(String(b.name || '').toLowerCase());
        }
        return cmp * dirMul;
    });
    return sorted;
}

function updateFileSortHeaderUI() {
    const header = document.getElementById('fileListHeader');
    if (!header) return;
    header.querySelectorAll('[data-sort]').forEach((el) => {
        el.classList.remove('sort-active', 'sort-asc', 'sort-desc');
    });
    const active = header.querySelector(`[data-sort="${_fileSortKey}"]`);
    if (active) {
        active.classList.add('sort-active', _fileSortAsc ? 'sort-asc' : 'sort-desc');
    }
}

function _onFileSortHeaderClick(e) {
    const target = e.target.closest('[data-sort]');
    if (!target) return;
    const key = target.dataset.sort;
    if (!['name', 'size', 'date', 'type', 'items', 'created'].includes(key)) return;
    if (_fileSortKey === key) {
        _fileSortAsc = !_fileSortAsc;
    } else {
        _fileSortKey = key;
        // Sensible default direction per column type — text columns ascend,
        // numeric/temporal columns descend (largest / newest first).
        _fileSortAsc = (key === 'name' || key === 'type');
        // size/items/date/created → desc by default (handled by the else above).
    }
    saveFileSortToPrefs();
    updateFileSortHeaderUI();
    if (typeof renderFileList === 'function') renderFileList();
}

if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const header = document.getElementById('fileListHeader');
        if (header) header.addEventListener('click', _onFileSortHeaderClick);
    });
}

async function initFileBrowser() {
    loadFileSortFromPrefs();
    updateFileSortHeaderUI();
    loadFileColumnWidths();
    initFileColumnResize();
    renderFavDirs();
    // Tab revisit: listing is still in the DOM (panel hidden, not destroyed) — avoid IPC + full re-render.
    if (_fileBrowserInited && _fileBrowserPath) {
        return;
    }
    if (_fileBrowserPath) {
        await loadDirectory(_fileBrowserPath);
        _fileBrowserInited = true;
        return;
    }
    // Start at home or first scan dir
    try {
        const home = await window.vstUpdater.getHomeDir();
        _fileBrowserPath = home;
        await loadDirectory(home);
    } catch {
        _fileBrowserPath = '/';
        await loadDirectory('/');
    }
    _fileBrowserInited = true;
}

async function loadDirectory(dirPath) {
    _fileListRenderSeq += 1;
    _fileBrowserPath = dirPath;
    if (typeof _navHistoryRecord === 'function') _navHistoryRecord(dirPath);
    // Reset cursor cache when directory changes
    window._fbCursorPath = null;
    window._fbCursorEl = null;
    _wfQueue = [];
    // Selections are scoped to the current directory — paths from the old
    // listing would point at unrelated rows once the new listing renders.
    if (typeof clearFileSelection === 'function') clearFileSelection();
    // Drop the scan-status cache for this directory so badges re-fetch fresh
    // counts (catches the case where the user just ran a scan from another tab
    // and is revisiting the file browser).
    if (typeof _scanStatusCache !== 'undefined' && _scanStatusCache instanceof Map) {
        _scanStatusCache.delete(dirPath);
    }
    showGlobalProgress();
    try {
        const result = await window.vstUpdater.listDirectory(dirPath);
        _fileBrowserEntries = result.entries;
        renderFileList();
        renderBreadcrumb(dirPath);
        updateBookmarkBtn();
    } catch (err) {
        showToast(toastFmt('toast.failed_open_directory', {err: err.message || err}), 4000, 'error');
    } finally {
        hideGlobalProgress();
    }
}

function renderBreadcrumb(dirPath) {
    const el = document.getElementById('fileBreadcrumb');
    if (!el) return;
    const norm = normalizePathSeparators(dirPath);

    if (/^[A-Za-z]:/.test(norm)) {
        const drive = norm.slice(0, 2);
        const rest = norm.slice(2).replace(/^\/+/, '');
        const segments = rest ? rest.split('/').filter(Boolean) : [];
        let html = `<span class="file-crumb" data-file-nav="${escapeHtml(drive + '/')}" title="${escapeHtml(drive)}">${escapeHtml(drive)}</span>`;
        let acc = drive;
        for (const part of segments) {
            acc = acc.endsWith(':') ? acc + '/' + part : acc + '/' + part;
            html += `<span class="file-crumb-sep">/</span><span class="file-crumb" data-file-nav="${escapeHtml(acc)}">${escapeHtml(part)}</span>`;
        }
        el.innerHTML = html;
        return;
    }

    const parts = norm.split('/').filter(Boolean);
    let html = `<span class="file-crumb" data-file-nav="/">/</span>`;
    let accumulated = '';
    for (const part of parts) {
        accumulated += '/' + part;
        html += `<span class="file-crumb-sep">/</span><span class="file-crumb" data-file-nav="${escapeHtml(accumulated)}">${escapeHtml(part)}</span>`;
    }
    el.innerHTML = html;
}

function buildFileListRowHtml(e, search, sampleByPath, mode) {
    const note = typeof noteIndicator === 'function' ? noteIndicator(e.path) : '';
    const cls = e.isDir ? ' file-dir' : '';
    const isAudio = !e.isDir && AUDIO_EXTS.includes(e.ext);

    const parts = [];
    if (typeof isFavorite === 'function' && isFavorite(e.path)) {
        parts.push('<span class="file-meta-tag file-meta-fav" title="Favorited">&#9733;</span>');
    }
    if (typeof getNote === 'function') {
        const n = getNote(e.path);
        if (n && n.tags && n.tags.length > 0) {
            parts.push(`<span class="file-meta-tag file-meta-tags" title="Tags: ${escapeHtml(n.tags.join(', '))}">${escapeHtml(n.tags.slice(0, 2).join(', '))}${n.tags.length > 2 ? '…' : ''}</span>`);
        }
    }
    if (isAudio) {
        if (typeof _bpmCache !== 'undefined' && _bpmCache[e.path]) {
            parts.push(`<span class="file-meta-tag file-meta-bpm" title="BPM">${_bpmCache[e.path]}</span>`);
        }
        if (typeof _keyCache !== 'undefined' && _keyCache[e.path]) {
            parts.push(`<span class="file-meta-tag file-meta-key" title="Musical key">${escapeHtml(_keyCache[e.path])}</span>`);
        }
        if (sampleByPath) {
            const sample = sampleByPath.get(e.path);
            if (sample && sample.duration) {
                parts.push(`<span class="file-meta-tag file-meta-dur" title="Duration">${typeof formatTime === 'function' ? formatTime(sample.duration) : sample.duration.toFixed(1) + 's'}</span>`);
            }
        }
    }
    const extras = parts.length > 0 ? `<span class="file-meta-tags-row">${parts.join('')}</span>` : '';

    const wfBg = isAudio ? `<canvas class="file-waveform" data-wf-path="${escapeHtml(e.path)}" height="36" title="Waveform"></canvas><span class="file-wf-cursor"></span>` : '';
    const isSelected = _fileSelected.has(e.path);
    // Folders show "…" placeholders for Size + Items until the background
    // walk completes (see `refreshFolderFilesystemSizes` →
    // `_paintFolderSizeCell`). Files have known sizes from `fs_list_dir` so
    // those cells fill on first paint; the Items cell is folder-only.
    const sizeContent = e.isDir
        ? '<span class="file-size-loading-text">…</span>'
        : e.sizeFormatted;
    const sizeCls = e.isDir ? ' file-size-loading' : '';
    const itemsContent = e.isDir
        ? '<span class="file-items-loading-text">…</span>'
        : '';
    const itemsCls = e.isDir ? ' file-items-loading' : '';
    return `<div class="file-row${cls}${isAudio ? ' file-audio' : ''}${isSelected ? ' file-selected' : ''}" data-file-path="${escapeHtml(e.path)}" data-file-dir="${e.isDir}" ${isAudio ? `data-wf-file="${escapeHtml(e.path)}"` : ''}>
      <span class="file-cb"><input type="checkbox" class="file-row-cb" data-fb-cb="${escapeHtml(e.path)}"${isSelected ? ' checked' : ''}></span>
      ${wfBg}
      <span class="file-icon">${fileIcon(e)}</span>
      <span class="file-name">${search && typeof highlightMatch === 'function' ? highlightMatch(e.name, search, mode || 'fuzzy') : escapeHtml(e.name)}${extras}${note}</span>
      <span class="file-ext">${e.isDir ? 'DIR' : e.ext}</span>
      <span class="file-size${sizeCls}">${sizeContent}</span>
      <span class="file-items${itemsCls}">${itemsContent}</span>
      <span class="file-date">${e.modified}</span>
      <span class="file-created">${e.created || ''}</span>
    </div>`;
}

function renderFileList() {
    const list = document.getElementById('fileList');
    if (!list) return;
    const search = (document.getElementById('fileSearchInput')?.value || '').trim();
    const mode = _lastFilesMode;
    // Extension chip filter applies BEFORE search/sort — chips narrow the
    // candidate pool, then search ranks within that pool. Folders are always
    // kept regardless of chip (see `_fbExtMatches`) so the user can navigate.
    const preFiltered = (_fbExtFilter && _fbExtFilter !== 'all')
        ? _fileBrowserEntries.filter((e) => _fbExtMatches(e, _fbExtFilter))
        : _fileBrowserEntries;
    let filtered;
    if (search) {
        const scored = preFiltered.map(e => {
            const score = typeof searchScore === 'function' ? searchScore(search, [e.name, e.ext], mode) : (e.name.toLowerCase().includes(search.toLowerCase()) ? 1 : 0);
            return {entry: e, score};
        }).filter(s => s.score > 0);
        // Search results stay in score order (best match first) — manual
        // sorting kicks in only for the unfiltered listing where there's no
        // semantic ranking to preserve.
        scored.sort((a, b) => b.score - a.score);
        filtered = scored.map(s => s.entry);
    } else {
        filtered = applyFileSort(preFiltered);
    }
    if (typeof updateFileListFooter === 'function') updateFileListFooter(filtered);

    if (filtered.length === 0) {
        _fileListRenderSeq += 1;
        list.innerHTML = '<div class="state-message"><div class="state-icon">&#128193;</div><h2>Empty Directory</h2></div>';
        return;
    }

    let sampleByPath = null;
    if (typeof allAudioSamples !== 'undefined' && Array.isArray(allAudioSamples) && allAudioSamples.length > 0) {
        sampleByPath = new Map();
        for (let i = 0; i < allAudioSamples.length; i++) {
            const s = allAudioSamples[i];
            if (s && s.path) sampleByPath.set(s.path, s);
        }
    }

    const seq = ++_fileListRenderSeq;
    list.innerHTML = '';
    let idx = 0;

    function appendChunk() {
        if (seq !== _fileListRenderSeq) return;
        const end = Math.min(idx + FILE_LIST_CHUNK, filtered.length);
        const slice = filtered.slice(idx, end);
        const html = slice.map((e) => buildFileListRowHtml(e, search, sampleByPath, mode)).join('');
        list.insertAdjacentHTML('beforeend', html);
        idx = end;
        if (idx < filtered.length) {
            const cont = appendChunk;
            if (typeof yieldToBrowser === 'function') {
                yieldToBrowser().then(cont);
            } else {
                setTimeout(cont, 0);
            }
        } else {
            requestAnimationFrame(() => initFileBrowserWaveforms());
            // Folder scan-status badges + filesystem-size walks — fire-and-forget
            // after the full list is painted so they can't delay the visible
            // render. Badges paint instantly from one batched SQL query;
            // filesystem-size walks are bounded-concurrency and stream in as
            // each per-folder walk completes.
            if (typeof refreshFolderScanBadges === 'function') {
                requestAnimationFrame(() => { refreshFolderScanBadges(); });
            }
            if (typeof refreshFolderFilesystemSizes === 'function') {
                requestAnimationFrame(() => { refreshFolderFilesystemSizes(); });
            }
        }
    }

    appendChunk();
}

// ── Lazy waveform rendering for file browser audio rows ──
let _wfQueue = [];
let _wfActive = 0;
// Reduced from 4 to 2 to avoid saturating IPC during heavy background jobs
const _wfMaxConcurrent = 2;
// Pause waveform loading during audio playback (SMB contention)
let _wfPausedForPlayback = false;

function setWaveformPausedForPlayback(paused) {
    _wfPausedForPlayback = paused;
    if (!paused) _processWfQueue(); // Resume queue processing
}
window.setWaveformPausedForPlayback = setWaveformPausedForPlayback;

function _processWfQueue() {
    if (_wfPausedForPlayback) return; // Pause during playback load
    while (_wfActive < _wfMaxConcurrent && _wfQueue.length > 0) {
        if (_wfPausedForPlayback) return; // Check again before each item
        const {canvas, path} = _wfQueue.shift();
        _wfActive++;
        drawMiniWaveform(canvas, path).finally(() => {
            _wfActive--;
            _processWfQueue();
        });
    }
}

let _fbWfObserver = null;

function initFileBrowserWaveforms() {
    const container = document.getElementById('fileList');
    if (!container) return;
    const canvases = container.querySelectorAll('canvas.file-waveform');
    if (canvases.length === 0) return;

    // Disconnect previous observer to prevent leak
    if (_fbWfObserver) {
        _fbWfObserver.disconnect();
        _fbWfObserver = null;
    }

    const observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            const canvas = entry.target;
            if (canvas._wfDrawn) continue;
            canvas._wfDrawn = true;
            observer.unobserve(canvas);
            _wfQueue.push({canvas, path: canvas.dataset.wfPath});
        }
        _processWfQueue();
    }, {root: container.closest('.tab-content'), threshold: 0.1});

    _fbWfObserver = observer;
    canvases.forEach(c => observer.observe(c));
}

async function drawMiniWaveform(canvas, filePath) {
    // Size canvas to parent row width
    const row = canvas.closest('.file-row');
    if (row) canvas.width = row.offsetWidth;
    const ctx = canvas.getContext('2d');
    const w = canvas.width, h = canvas.height;

    // Check cache first
    if (typeof _waveformCache !== 'undefined' && _waveformCache[filePath]) {
        renderMiniWf(ctx, w, h, _waveformCache[filePath]);
        return;
    }

    const widthPx = Math.max(32, Math.min(Math.floor(w) || 800, 8192));

    try {
        let peaks = null;
        if (typeof window._fetchWaveformPeaksFromAudioEngine === 'function') {
            peaks = await window._fetchWaveformPeaksFromAudioEngine(filePath, widthPx);
        }
        if (!peaks) {
            const src = typeof convertFileSrc === 'function' ? convertFileSrc(filePath) : filePath;
            if (typeof window._decodePeaksViaWorker === 'function') {
                peaks = await window._decodePeaksViaWorker(src, widthPx);
            } else {
                if (!window._fbAudioCtx) window._fbAudioCtx = new AudioContext();
                const resp = await fetch(src);
                const buf = await resp.arrayBuffer();
                const audioBuf = await window._fbAudioCtx.decodeAudioData(buf);
                const raw = audioBuf.getChannelData(0);

                const bars = widthPx;
                const step = Math.floor(raw.length / bars);
                peaks = [];
                for (let i = 0; i < bars; i++) {
                    let max = 0, min = 0;
                    const start = i * step;
                    for (let j = start; j < start + step && j < raw.length; j++) {
                        if (raw[j] > max) max = raw[j];
                        if (raw[j] < min) min = raw[j];
                    }
                    peaks.push({max, min});
                }
            }
        }

        if (typeof window._storeWaveformPeaksInCache === 'function') {
            window._storeWaveformPeaksInCache(filePath, peaks);
        } else if (typeof _waveformCache !== 'undefined') {
            _waveformCache[filePath] = peaks;
        }
        renderMiniWf(ctx, w, h, peaks);
    } catch {
        // Draw flat line
        ctx.strokeStyle = 'rgba(5,217,232,0.2)';
        ctx.beginPath();
        ctx.moveTo(0, h / 2);
        ctx.lineTo(w, h / 2);
        ctx.stroke();
    }
}

function renderMiniWf(ctx, w, h, peaks) {
    const mid = h / 2;
    const isNew = peaks.length > 0 && typeof peaks[0] === 'object';
    ctx.clearRect(0, 0, w, h);

    if (isNew) {
        ctx.beginPath();
        ctx.moveTo(0, mid);
        for (let i = 0; i < peaks.length; i++) {
            ctx.lineTo(i, mid - peaks[i].max * mid * 0.9);
        }
        for (let i = peaks.length - 1; i >= 0; i--) {
            ctx.lineTo(i, mid - peaks[i].min * mid * 0.9);
        }
        ctx.closePath();
        const grad = ctx.createLinearGradient(0, 0, w, 0);
        grad.addColorStop(0, 'rgba(5,217,232,0.4)');
        grad.addColorStop(1, 'rgba(211,0,197,0.4)');
        ctx.fillStyle = grad;
        ctx.fill();
    } else {
        for (let i = 0; i < peaks.length; i++) {
            const barH = (typeof peaks[i] === 'number' ? peaks[i] : 0) * mid * 0.9;
            ctx.fillStyle = 'rgba(5,217,232,0.4)';
            ctx.fillRect(i, mid - barH, 1, barH * 2);
        }
    }
}

// ── Expandable metadata panel for audio files in file browser ──
let _fbExpandedPath = null;

async function toggleFileBrowserMeta(filePath) {
    const list = document.getElementById('fileList');
    if (!list) return;
    const existing = document.getElementById('fbMetaPanel');
    if (existing) {
        const wasPath = existing.dataset.metaPath;
        existing.remove();
        if (wasPath === filePath) {
            _fbExpandedPath = null;
            return;
        }
    }

    _fbExpandedPath = filePath;
    const row = list.querySelector(`.file-row[data-file-path="${CSS.escape(filePath)}"]`);
    if (!row) return;

    // Insert loading panel
    const panel = document.createElement('div');
    panel.id = 'fbMetaPanel';
    panel.dataset.metaPath = filePath;
    panel.className = 'fb-meta-panel';
    panel.innerHTML = '<div style="text-align:center;padding:12px;"><div class="spinner" style="width:14px;height:14px;margin:0 auto;"></div></div>';
    row.after(panel);

    try {
        const meta = await window.vstUpdater.getAudioMetadata(filePath);
        if (_fbExpandedPath !== filePath) return;
        const p = document.getElementById('fbMetaPanel');
        if (!p) return;

        const mi = (label, value) => value ? `<div class="fb-meta-item" title="${escapeHtml(label)}: ${escapeHtml(String(value))}"><span class="fb-meta-label">${label}</span><span class="fb-meta-val">${escapeHtml(String(value))}</span></div>` : '';

        let html = '<div class="fb-meta-grid">';
        html += mi('Format', meta.format);
        html += mi('Size', typeof formatAudioSize === 'function' ? formatAudioSize(meta.sizeBytes) : meta.sizeBytes);
        if (meta.sampleRate) html += mi('Sample Rate', meta.sampleRate.toLocaleString() + ' Hz');
        if (meta.bitsPerSample) html += mi('Bit Depth', meta.bitsPerSample + '-bit');
        if (meta.channels) html += mi('Channels', meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch');
        if (meta.duration) html += mi('Duration', typeof formatTime === 'function' ? formatTime(meta.duration) : meta.duration.toFixed(1) + 's');
        if (meta.byteRate) html += mi('Byte Rate', (typeof formatAudioSize === 'function' ? formatAudioSize(meta.byteRate) : meta.byteRate) + '/s');

        // BPM
        html += `<div class="fb-meta-item" title="BPM"><span class="fb-meta-label">BPM</span><span class="fb-meta-val" id="fbBpmVal"><span class="spinner" style="width:8px;height:8px;"></span></span></div>`;
        // Key
        html += `<div class="fb-meta-item" title="Musical Key"><span class="fb-meta-label">Key</span><span class="fb-meta-val" id="fbKeyVal"><span class="spinner" style="width:8px;height:8px;"></span></span></div>`;

        const fmtDate = (v) => {
            if (!v) return '—';
            const d = new Date(v);
            return isNaN(d) ? '—' : d.toLocaleString();
        };
        html += mi('Created', fmtDate(meta.created));
        html += mi('Modified', fmtDate(meta.modified));
        html += mi('Permissions', meta.permissions);
        html += mi('Path', meta.fullPath);
        html += '</div>';

        // Favorite, Notes, Tags as grid items
        const isFav = typeof isFavorite === 'function' && isFavorite(filePath);
        html += mi('Favorite', isFav ? '★ Yes' : '☆ No');
        const noteData = typeof getNote === 'function' ? getNote(filePath) : null;
        const tags = noteData?.tags?.length ? noteData.tags.join(', ') : '';
        html += mi('Tags', tags || '—');
        const noteText = noteData?.note || '';
        if (noteText) html += mi('Note', noteText);

        p.innerHTML = html;

        // Async BPM + Key
        const bpmFormats = ['wav', 'aiff', 'aif', 'mp3', 'flac', 'ogg', 'm4a', 'aac', 'opus'];
        if (bpmFormats.includes(meta.format?.toLowerCase() || '')) {
            // BPM
            (async () => {
                try {
                    if (typeof _bpmCache !== 'undefined' && _bpmCache[filePath] !== undefined) {
                        const el = document.getElementById('fbBpmVal');
                        if (el) el.textContent = _bpmCache[filePath] ? _bpmCache[filePath] + ' BPM' : '—';
                        return;
                    }
                    const bpm = await window.vstUpdater.estimateBpm(filePath);
                    if (typeof _bpmCache !== 'undefined') _bpmCache[filePath] = bpm;
                    const el = document.getElementById('fbBpmVal');
                    if (el && _fbExpandedPath === filePath) el.textContent = bpm ? bpm + ' BPM' : '—';
                } catch {
                    const el = document.getElementById('fbBpmVal');
                    if (el) el.textContent = '—';
                }
            })();
            // Key
            (async () => {
                try {
                    if (typeof _keyCache !== 'undefined' && _keyCache[filePath] !== undefined) {
                        const el = document.getElementById('fbKeyVal');
                        if (el) el.textContent = _keyCache[filePath] || '—';
                        return;
                    }
                    const key = await window.vstUpdater.detectAudioKey(filePath);
                    if (typeof _keyCache !== 'undefined') _keyCache[filePath] = key;
                    const el = document.getElementById('fbKeyVal');
                    if (el && _fbExpandedPath === filePath) el.textContent = key || '—';
                } catch {
                    const el = document.getElementById('fbKeyVal');
                    if (el) el.textContent = '—';
                }
            })();
        } else {
            const bEl = document.getElementById('fbBpmVal');
            const kEl = document.getElementById('fbKeyVal');
            if (bEl) bEl.textContent = '—';
            if (kEl) kEl.textContent = '—';
        }
    } catch (err) {
        const p = document.getElementById('fbMetaPanel');
        if (p) p.innerHTML = `<div style="padding:8px;color:var(--red);font-size:11px;">Failed: ${escapeHtml(err.message || String(err))}</div>`;
    }
}

// Click to navigate dirs or play/open files
document.addEventListener('click', (e) => {
    const crumb = e.target.closest('[data-file-nav]');
    if (crumb) {
        loadDirectory(crumb.dataset.fileNav);
        return;
    }

    const row = e.target.closest('.file-row');
    // Clicks inside the checkbox cell (the cell wrapper OR the input itself)
    // are exclusively for selection — they must not also trigger the row's
    // navigate / preview action. `stopPropagation` in the checkbox handler
    // (registered separately on document) doesn't help here because both
    // listeners are on the same target; only ordering + a guard does.
    if (row && !e.target.closest('.fb-meta-panel') && !e.target.closest('.file-cb')) {
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            loadDirectory(path);
        } else {
            const ext = path.split('.').pop().toLowerCase();
            if (AUDIO_EXTS.includes(ext)) {
                previewAudio(path);
                toggleFileBrowserMeta(path);
            } else {
                opener_open(path);
            }
        }
        return;
    }
});

function opener_open(path) {
    // DAW project formats first (.als / .flp / .logicx etc. → Ableton, FL Studio,
    // Logic). When the path isn't a DAW project (or the DAW isn't installed), fall
    // back to the OS default-app opener instead of `openPresetFolder` (which
    // *revealed the parent folder* — surprising for files like .txt / .png where
    // the user expected the file itself to open). With this fallback a click on
    // foo.txt opens TextEdit / Notepad / xdg-open per platform; a click on foo.als
    // still opens Ableton via the DAW path.
    window.vstUpdater.openDawProject(path).catch(() => {
        window.vstUpdater.openFileDefault(path).catch(() => {});
    });
}

// Action handlers
document.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action]');
    if (!action) return;
    if (action.dataset.action === 'fileUp') {
        if (_fileBrowserPath) {
            const n = normalizePathSeparators(_fileBrowserPath);
            if (n !== '/' && !/^[A-Za-z]:\/?$/.test(n.replace(/\/+$/, ''))) {
                const parent = parentDirectoryPath(_fileBrowserPath);
                if (normalizePathSeparators(parent) !== n.replace(/\/+$/, '')) {
                    loadDirectory(parent);
                }
            }
        }
    } else if (action.dataset.action === 'fileHome') {
        window.vstUpdater.getHomeDir().then(h => loadDirectory(h)).catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        });
    } else if (action.dataset.action === 'fileQuickNav') {
        const dir = action.dataset.dir;
        if (dir === '/') {
            loadDirectory('/');
        } else {
            window.vstUpdater.getHomeDir().then((h) => {
                const home = normalizePathSeparators(h).replace(/\/+$/, '');
                loadDirectory(home + '/' + dir);
            }).catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            });
        }
    } else if (action.dataset.action === 'fileAppDataDir') {
        const vu = window.vstUpdater;
        if (!vu || typeof vu.getPrefsPath !== 'function') {
            if (typeof showToast === 'function') showToast(String('getPrefsPath unavailable'), 4000, 'error');
            return;
        }
        vu.getPrefsPath()
            .then((p) => {
                const norm = normalizePathSeparators(String(p || ''));
                const dir = norm.replace(/\/[^/]+$/, '');
                if (!dir) {
                    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
                        showToast(toastFmt('toast.failed', {err: 'invalid preferences path'}), 4000, 'error');
                    }
                    return;
                }
                loadDirectory(dir);
            })
            .catch((e) => {
                if (typeof showToast === 'function') showToast(String(e && e.message ? e.message : e), 4000, 'error');
            });
    } else if (action.dataset.action === 'fileFav') {
        if (_fileBrowserPath) {
            if (isFavDir(_fileBrowserPath)) removeFavDir(_fileBrowserPath);
            else addFavDir(_fileBrowserPath);
        }
    }
});

// Fav directory chip clicks
document.addEventListener('click', (e) => {
    const remove = e.target.closest('[data-remove-fav-dir]');
    if (remove) {
        e.stopPropagation();
        removeFavDir(remove.dataset.removeFavDir);
        return;
    }
    const chip = e.target.closest('[data-fav-dir]');
    if (chip) {
        loadDirectory(chip.dataset.favDir);
    }
});

// ── Multi-select interactions ──
// Per-row checkbox click: toggles single row, or with Shift, range-selects from
// the last interacted-with row to this one (inclusive). `stopPropagation` so the
// row's open/preview click handler doesn't fire on the checkbox click itself.
document.addEventListener('click', (e) => {
    const cb = e.target.closest('.file-row-cb');
    if (cb) {
        // `stopImmediatePropagation` (not just `stopPropagation`) — multiple
        // click listeners are attached to `document`; we have to prevent the
        // sibling row-click handler from also firing on this same event.
        e.stopImmediatePropagation();
        const path = cb.dataset.fbCb;
        if (!path) return;
        const allRows = Array.from(document.querySelectorAll('.file-row-cb'));
        const idx = allRows.indexOf(cb);
        if (e.shiftKey && _fileSelectLastIdx >= 0 && idx >= 0 && idx !== _fileSelectLastIdx) {
            const lo = Math.min(_fileSelectLastIdx, idx);
            const hi = Math.max(_fileSelectLastIdx, idx);
            const want = cb.checked;
            for (let i = lo; i <= hi; i++) {
                const c = allRows[i];
                if (!c) continue;
                c.checked = want;
                const p = c.dataset.fbCb;
                if (!p) continue;
                if (want) _fileSelected.add(p);
                else _fileSelected.delete(p);
                _setFileRowSelectedClass(p, want);
            }
            updateFileBulkBar();
        } else {
            toggleFileSelect(path, cb.checked);
        }
        _fileSelectLastIdx = idx;
        return;
    }
    const allCb = e.target.closest('.file-row-cb-all');
    if (allCb) {
        e.stopImmediatePropagation();
        if (allCb.checked) selectAllVisibleFiles();
        else clearFileSelection();
        return;
    }
});

// ── Bulk-action toolbar buttons ──
document.addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-action^="fileBulk"]');
    if (!btn) return;
    e.stopPropagation();
    const action = btn.dataset.action;
    if (action === 'fileBulkClear') {
        clearFileSelection();
        return;
    }
    const paths = [..._fileSelected];
    if (paths.length === 0) return;
    if (action === 'fileBulkFavorite') {
        let count = 0;
        for (const p of paths) {
            const entry = _fileEntryByPath(p);
            if (!entry) continue;
            if (entry.isDir) {
                if (typeof addFavDir === 'function') { addFavDir(p); count++; }
            } else if (typeof addFavorite === 'function') {
                const name = p.split('/').pop();
                const ext = (name.split('.').pop() || '').toLowerCase();
                const kind = (typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext)) ? 'sample' : 'file';
                addFavorite(kind, p, name, {format: ext.toUpperCase()});
                count++;
            }
        }
        if (typeof showToast === 'function' && count > 0) {
            showToast(toastFmt('toast.added_favorites_batch', {n: count}));
        }
        return;
    }
    if (action === 'fileBulkRename') {
        showFileBulkRenameModal();
        return;
    }
    if (action === 'fileBulkOpen') {
        for (const p of paths) {
            if (window.vstUpdater && typeof window.vstUpdater.openFileDefault === 'function') {
                window.vstUpdater.openFileDefault(p).catch(() => {});
            }
        }
        return;
    }
    if (action === 'fileBulkDelete') {
        const msg = paths.length === 1
            ? appFmt('confirm.delete_file_browser', {name: paths[0].split('/').pop()})
            : appFmt('confirm.delete_file_browser', {name: `${paths.length} items`});
        if (!confirm(msg)) return;
        let failures = 0;
        for (const p of paths) {
            try { await window.vstUpdater.deleteFile(p); }
            catch (_) { failures++; }
        }
        clearFileSelection();
        if (typeof showToast === 'function') {
            // Reuse existing single-item toast key by passing a count-as-name. Avoids
            // adding a new i18n key for a low-frequency bulk-delete-success message.
            const survived = paths.length - failures;
            if (survived > 0) showToast(toastFmt('toast.deleted_name', {name: `${survived} item${survived === 1 ? '' : 's'}`}));
            if (failures > 0) showToast(toastFmt('toast.failed', {err: `${failures} delete${failures === 1 ? '' : 's'} failed`}), 4000, 'error');
        }
        if (_fileBrowserPath) loadDirectory(_fileBrowserPath);
        return;
    }
    if (action === 'fileBulkScan') {
        const kind = btn.dataset.scan;
        const dirs = _fileBulkSelectionAsPaths((en) => en.isDir);
        if (dirs.length === 0) return; // silent no-op — user clicked scan with no folder selected

        const tabByKind = {samples: 'samples', presets: 'presets', daw: 'daw', midi: 'midi', pdf: 'pdf', videos: 'videos'};
        const tab = tabByKind[kind];
        if (tab && typeof switchTab === 'function') switchTab(tab);
        // scanMidi / scanVideos take (resume, overrideRoots); the others take
        // (resume, unifiedResult, overrideRoots).
        if (kind === 'samples' && typeof scanAudioSamples === 'function') scanAudioSamples(false, null, dirs);
        else if (kind === 'presets' && typeof scanPresets === 'function') scanPresets(false, null, dirs);
        else if (kind === 'daw' && typeof scanDawProjects === 'function') scanDawProjects(false, null, dirs);
        else if (kind === 'midi' && typeof scanMidi === 'function') scanMidi(false, dirs);
        else if (kind === 'pdf' && typeof scanPdfs === 'function') scanPdfs(false, null, dirs);
        else if (kind === 'videos' && typeof scanVideos === 'function') scanVideos(false, dirs);
        return;
    }
});

// Cmd/Ctrl+A while the Files tab is active and focus isn't in a text input →
// select all visible file rows.
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && (e.key === 'a' || e.key === 'A'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    selectAllVisibleFiles();
});

// Space → Quick-look overlay for the focused (or single-selected) file.
// Folders intentionally don't get a Quick-look — Space on a folder would be
// weird (and Enter/click already navigates into them).
document.addEventListener('keydown', (e) => {
    if (e.key !== ' ') return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    // If Quick-look is already up, Space dismisses it.
    if (isQuickLookVisible()) {
        e.preventDefault();
        hideQuickLook();
        return;
    }
    let row = (typeof getFileRows === 'function')
        ? getFileRows()[typeof _fileNavIdx !== 'undefined' ? _fileNavIdx : -1]
        : null;
    if (!row && _fileSelected && _fileSelected.size === 1) {
        const [p] = [..._fileSelected];
        if (typeof CSS !== 'undefined') {
            try { row = document.querySelector(`.file-row[data-file-path="${CSS.escape(p)}"]`); } catch (_) { /* ignore */ }
        }
    }
    if (!row) return;
    if (row.dataset.fileDir === 'true') return;
    const path = row.dataset.filePath;
    if (!path) return;
    e.preventDefault();
    showQuickLook(path);
});

// Esc → dismiss Quick-look when visible (also handled by overlay click).
document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (!isQuickLookVisible()) return;
    e.preventDefault();
    hideQuickLook();
});

// F2 → inline-rename the focused row (or single selection). Esc cancels.
// Active only while Files tab is active and focus isn't in a text input.
document.addEventListener('keydown', (e) => {
    if (e.key !== 'F2') return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    // Prefer the keyboard-nav cursor row; fall back to the single selection.
    let row = (typeof getFileRows === 'function')
        ? getFileRows()[typeof _fileNavIdx !== 'undefined' ? _fileNavIdx : -1]
        : null;
    if (!row && _fileSelected && _fileSelected.size === 1) {
        const [path] = [..._fileSelected];
        if (typeof CSS !== 'undefined') {
            try { row = document.querySelector(`.file-row[data-file-path="${CSS.escape(path)}"]`); } catch (_) { /* ignore */ }
        }
    }
    if (!row) return;
    e.preventDefault();
    startFileRename(row);
});

// Cmd+Shift+N → create new folder in current directory.
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'n' || e.key === 'N'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    fileBrowserNewFolder();
});

// Cmd+I → toggle preview pane visibility. Common file-browser convention
// (Get Info / inspector on macOS).
document.addEventListener('keydown', (e) => {
    if (!((e.metaKey || e.ctrlKey) && !e.shiftKey && (e.key === 'i' || e.key === 'I'))) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    toggleFileBrowserPreviewPane();
});

// ── Forward/back nav + New Folder button clicks ──
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="fileNavBack"]')) {
        e.stopPropagation();
        fileNavBack();
        return;
    }
    if (e.target.closest('[data-action="fileNavFwd"]')) {
        e.stopPropagation();
        fileNavForward();
        return;
    }
    if (e.target.closest('[data-action="fileNewFolder"]')) {
        e.stopPropagation();
        fileBrowserNewFolder();
        return;
    }
    if (e.target.closest('[data-action="fileTogglePreview"]')
        || e.target.closest('[data-action="fbPreviewClose"]')) {
        e.stopPropagation();
        if (e.target.closest('[data-action="fbPreviewClose"]')) setPreviewPaneVisible(false);
        else toggleFileBrowserPreviewPane();
        return;
    }
});

// Bulk rename modal: Cancel / Apply button delegation + live preview on input.
document.addEventListener('click', (e) => {
    if (e.target.closest('[data-action="fbBulkRenameCancel"]')) {
        e.stopPropagation();
        hideFileBulkRenameModal();
        return;
    }
    if (e.target.closest('[data-action="fbBulkRenameApply"]')) {
        e.stopPropagation();
        applyFileBulkRename();
        return;
    }
    // Click on the dimmed overlay (outside the modal-content card) dismisses.
    const overlay = document.getElementById('fbBulkRenameModal');
    if (overlay && e.target === overlay) {
        hideFileBulkRenameModal();
    }
});

document.addEventListener('input', (e) => {
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    if (!modal.contains(e.target)) return;
    _fbBulkRenameRenderPreview();
});

document.addEventListener('change', (e) => {
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    if (!modal.contains(e.target)) return;
    _fbBulkRenameRenderPreview();
});

// Esc dismisses the bulk rename modal (consumed before any other Esc handler).
document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    const modal = document.getElementById('fbBulkRenameModal');
    if (!modal || !modal.classList.contains('modal-visible')) return;
    e.preventDefault();
    e.stopPropagation();
    hideFileBulkRenameModal();
}, true);

// Restore preview-pane visibility from prefs on init (so the user's choice
// persists across app restarts). Wire row clicks to populate the pane.
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        try {
            const wanted = prefs.getItem('fileBrowserPreviewVisible');
            if (wanted === '1') setPreviewPaneVisible(true);
        } catch (_) { /* ignore */ }
    });
    // Update pane content whenever a file row is clicked.
    document.addEventListener('click', (e) => {
        const row = e.target instanceof Element ? e.target.closest('.file-row') : null;
        if (!row || row.dataset.fileDir === 'true') return;
        // Skip clicks that are routed to interactive children (checkbox, etc.)
        if (e.target.closest('.file-cb') || e.target.closest('.fb-rename-input')) return;
        const path = row.dataset.filePath;
        if (path) populatePreviewPane(path);
    });
}

// ── Extension chip clicks ──
document.addEventListener('click', (e) => {
    const chip = e.target.closest('.fb-ext-chips .ext-chip[data-ext-filter]');
    if (!chip) return;
    e.stopPropagation();
    setFileExtFilter(chip.dataset.extFilter);
});

// ── Cmd+L editable path input + Cmd+[ / Cmd+] history nav shortcuts ──
document.addEventListener('keydown', (e) => {
    if (!(e.metaKey || e.ctrlKey)) return;
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    const t = e.target;
    const inField = t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
    if (e.key === 'l' || e.key === 'L') {
        if (inField && t.id !== 'fileBrowserPathInput') return;
        e.preventDefault();
        showFilePathEditor();
        return;
    }
    if (e.key === '[') {
        if (inField) return;
        e.preventDefault();
        fileNavBack();
        return;
    }
    if (e.key === ']') {
        if (inField) return;
        e.preventDefault();
        fileNavForward();
        return;
    }
});

// Path input: Enter navigates, Esc cancels, blur hides.
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const input = document.getElementById('fileBrowserPathInput');
        if (!input) return;
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const path = input.value.trim();
                hideFilePathEditor();
                if (path) loadDirectory(path);
            } else if (e.key === 'Escape') {
                e.preventDefault();
                hideFilePathEditor();
            }
        });
        input.addEventListener('blur', () => {
            // Defer so Enter's loadDirectory has a chance to fire before the
            // breadcrumb takes its slot back.
            setTimeout(() => hideFilePathEditor(), 100);
        });
    });
}

// Filter — uses unified filter system
registerFilter('filterFiles', {
    inputId: 'fileSearchInput',
    regexToggleId: 'regexFiles',
    fetchFn() {
        _lastFilesMode = this.lastMode || 'fuzzy';
        renderFileList();
    },
    debounceMs: 150,
});

// Right-click context menu
document.addEventListener('contextmenu', (e) => {
    const row = e.target.closest('.file-row');
    if (!row) return;
    const path = row.dataset.filePath;
    const isDir = row.dataset.fileDir === 'true';
    const name = row.querySelector('.file-name')?.textContent || '';
    const ext = path.split('.').pop().toLowerCase();

    const items = [];
    if (isDir) {
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.open_folder'), ..._ctxMenuNoEcho,
            action: () => loadDirectory(path)
        });
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('revealFile'),
            action: () => window.vstUpdater.openPresetFolder(path)
        });
        const dirFav = isFavDir(path);
        items.push({
            icon: dirFav ? '&#9734;' : '&#9733;',
            label: dirFav ? appFmt('menu.remove_bookmark') : appFmt('menu.bookmark_directory'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('toggleFavorite'),
            action: () => dirFav ? removeFavDir(path) : addFavDir(path)
        });
    } else {
        if (AUDIO_EXTS.includes(ext)) {
            items.push({
                icon: '&#9654;',
                label: appFmt('menu.play'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('playPause'),
                action: () => previewAudio(path)
            });
        }
        items.push({icon: '&#128194;', label: appFmt('menu.open'), ..._ctxMenuNoEcho, action: () => opener_open(path)});
        // Explicit "Open in Default App" — bypasses the DAW-first routing in
        // `opener_open`. Useful when the user wants the OS default for a file
        // type that's also a DAW project (e.g. open .als raw in a text editor)
        // or for arbitrary files (.txt, .png, .docx) where there's no in-app
        // viewer. Toast on failure so the user knows when no handler is
        // registered for the extension.
        items.push({
            icon: '&#128194;',
            label: appFmt('menu.open_default_app'), ..._ctxMenuNoEcho,
            action: () => window.vstUpdater.openFileDefault(path).catch((err) =>
                showToast(toastFmt('toast.failed_open_file', {err: err && err.message ? err.message : err}), 4000, 'error')
            ),
        });
        items.push({
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('revealFile'),
            action: () => window.vstUpdater.openPresetFolder(path)
        });
    }
    items.push('---');
    items.push({
        icon: '&#128203;',
        label: appFmt('menu.copy_path'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('copyPath'),
        action: () => copyToClipboard(path)
    });
    items.push({
        icon: '&#128203;',
        label: appFmt('menu.copy_name'), ..._ctxMenuNoEcho,
        action: () => copyToClipboard(name.replace(/^[\u{1F4DD}]/u, '').trim())
    });
    items.push('---');

    // ALS XML viewer
    if (ext === 'als' && typeof showAlsViewer === 'function') {
        items.push({
            icon: '&#128196;',
            label: appFmt('menu.explore_xml_contents'),
            action: () => showAlsViewer(path, name)
        });
        items.push('---');
    }

    // Tags & notes
    const note = getNote(path);
    items.push({
        icon: '&#128221;',
        label: note ? appFmt('menu.edit_note') : appFmt('menu.add_note'),
        ..._ctxShortcutTip('addNote'),
        action: () => showNoteEditor(path, name)
    });

    const allTags = getAllTags();
    const currentTags = note?.tags || [];
    if (allTags.length > 0) {
        items.push('---');
        for (const tag of allTags.slice(0, 6)) {
            const has = currentTags.includes(tag);
            items.push({
                icon: has ? '&#10003;' : '&#9634;',
                label: has ? appFmt('menu.remove_tag_named', {tag}) : appFmt('menu.add_tag_named', {tag}), ..._ctxMenuNoEcho,
                action: () => {
                    if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag);
                    showToast(has ? toastFmt('toast.tag_removed', {tag}) : toastFmt('toast.tag_added', {tag}));
                    renderFileList();
                }
            });
        }
    }

    items.push('---');
    const fav = isFavorite(path);
    items.push({
        icon: fav ? '&#9734;' : '&#9733;',
        label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._ctxMenuNoEcho, ..._ctxShortcutTip('toggleFavorite'),
        action: () => fav ? removeFavorite(path) : addFavorite(isDir ? 'folder' : (AUDIO_EXTS.includes(ext) ? 'sample' : 'file'), path, name, {format: ext.toUpperCase()})
    });

    items.push('---');
    items.push({
        icon: '&#128465;', label: appFmt('menu.delete'), ..._ctxShortcutTip('deleteItem'), action: async () => {
            if (!confirm(appFmt('confirm.delete_file_browser', {name}))) return;
            try {
                await window.vstUpdater.deleteFile(path);
                showToast(toastFmt('toast.deleted_name_quotes', {name}));
                loadDirectory(_fileBrowserPath);
            } catch (err) {
                showToast(toastFmt('toast.delete_failed_msg', {err: err.message || err}), 4000, 'error');
            }
        }
    });

    showContextMenu(e, items);
    e.preventDefault();
});

// ── Ableton-style keyboard navigation ──
let _fileNavIdx = -1;

function getFileRows() {
    return [...document.querySelectorAll('#fileList .file-row')];
}

function fileNavSelect(idx) {
    const rows = getFileRows();
    if (rows.length === 0) return;
    // Clear previous
    rows.forEach(r => r.classList.remove('file-selected'));
    _fileNavIdx = Math.max(0, Math.min(idx, rows.length - 1));
    const row = rows[_fileNavIdx];
    row.classList.add('file-selected');
    row.scrollIntoView({block: 'nearest', behavior: 'smooth'});
}

document.addEventListener('click', (e) => {
    const row = e.target.closest('#fileList .file-row');
    if (row) {
        const rows = getFileRows();
        const idx = rows.indexOf(row);
        if (idx >= 0) {
            rows.forEach(r => r.classList.remove('file-selected'));
            _fileNavIdx = idx;
            row.classList.add('file-selected');
        }
    }
});

document.addEventListener('keydown', (e) => {
    // Only handle when Files tab is active and not typing in an input
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab || activeTab.id !== 'tabFiles') return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;

    const rows = getFileRows();
    if (rows.length === 0) return;

    if (e.key === 'ArrowDown' || (e.key === 'j' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        fileNavSelect(_fileNavIdx + 1);
    } else if (e.key === 'ArrowUp' || (e.key === 'k' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        fileNavSelect(_fileNavIdx - 1);
    } else if (e.key === 'Home') {
        e.preventDefault();
        fileNavSelect(0);
    } else if (e.key === 'End') {
        e.preventDefault();
        fileNavSelect(rows.length - 1);
    } else if (e.key === 'ArrowRight' || e.key === 'l') {
        // Right arrow: navigate into directory or play audio
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            loadDirectory(path).then(() => {
                _fileNavIdx = -1;
                fileNavSelect(0);
            });
        } else {
            const ext = path.split('.').pop().toLowerCase();
            if (AUDIO_EXTS.includes(ext)) {
                previewAudio(path);
            } else {
                opener_open(path);
            }
        }
    } else if (e.key === 'Enter') {
        // Enter: open in Finder (dir) or open with default app (file)
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const isDir = row.dataset.fileDir === 'true';
        if (isDir) {
            openFolder(path);
        } else {
            opener_open(path);
        }
    } else if (e.key === 'ArrowLeft' || e.key === 'h') {
        // Left arrow: go to parent directory
        e.preventDefault();
        if (_fileBrowserPath) {
            const n = normalizePathSeparators(_fileBrowserPath);
            if (n !== '/' && !/^[A-Za-z]:\/?$/.test(n.replace(/\/+$/, ''))) {
                const parent = parentDirectoryPath(_fileBrowserPath);
                if (normalizePathSeparators(parent) !== n.replace(/\/+$/, '')) {
                    loadDirectory(parent).then(() => {
                        _fileNavIdx = -1;
                        fileNavSelect(0);
                    });
                }
            }
        }
    } else if (e.key === ' ') {
        // Global shortcut handles Space for play/pause; do not restart preview on top of it.
        if (e.defaultPrevented) return;
        // Space: preview audio if selected
        e.preventDefault();
        if (_fileNavIdx < 0 || _fileNavIdx >= rows.length) return;
        const row = rows[_fileNavIdx];
        const path = row.dataset.filePath;
        const ext = path.split('.').pop().toLowerCase();
        if (AUDIO_EXTS.includes(ext)) {
            previewAudio(path);
        }
    }
});
