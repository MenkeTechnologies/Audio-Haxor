// ── Keyboard Navigation ──
// Arrow keys, Enter, Space for navigating tables and plugin lists

let _navIndex = -1;
let _navTab = null;
let _sampleSelectPlayTimer = null;
let _lastAutoPlaySamplePath = null;

function getNavigableItems() {
    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return [];
    const id = activeTab.id;
    if (id === 'tabPlugins') return [...activeTab.querySelectorAll('.plugin-card')];
    if (id === 'tabSamples') return [...activeTab.querySelectorAll('#audioTableBody tr[data-audio-path]')];
    if (id === 'tabDaw') return [...activeTab.querySelectorAll('#dawTableBody tr[data-daw-path]')];
    if (id === 'tabPresets') return [...activeTab.querySelectorAll('#presetTableBody tr[data-preset-path]')];
    if (id === 'tabMidi') return [...activeTab.querySelectorAll('#midiTableBody tr[data-midi-path]')];
    if (id === 'tabPdf') return [...activeTab.querySelectorAll('#pdfTableBody tr[data-pdf-path]')];
    if (id === 'tabFavorites') return [...activeTab.querySelectorAll('.fav-item')];
    return [];
}

function _normNavPathKey(p) {
    if (typeof normalizeFavoritePathKey === 'function') return normalizeFavoritePathKey(p);
    if (p == null || typeof p !== 'string') return '';
    return p.replace(/\\/g, '/');
}

/** When j/k run with _navIndex still -1, anchor relative movement to playing row, expanded row, or .nav-selected (e.g. after click). */
function syncNavIndexBeforeVerticalMove(activeTab) {
    if (_navIndex >= 0) return;
    const items = getNavigableItems();
    if (items.length === 0) return;

    if (activeTab === 'tabSamples') {
        if (typeof audioPlayerPath === 'string' && audioPlayerPath) {
            const ap = _normNavPathKey(audioPlayerPath);
            const idx = items.findIndex((el) => _normNavPathKey(el.getAttribute('data-audio-path') || '') === ap);
            if (idx >= 0) {
                _navIndex = idx;
                return;
            }
        }
        const exp = document.querySelector('#audioTableBody tr.row-expanded[data-audio-path]');
        if (exp) {
            const idx = items.indexOf(exp);
            if (idx >= 0) {
                _navIndex = idx;
                return;
            }
        }
    }
    const sel = document.querySelector('.nav-selected');
    if (sel && items.includes(sel)) _navIndex = items.indexOf(sel);
}

/**
 * Keep keyboard nav index aligned with mouse/touch: clicking a row (or its batch checkbox)
 * should make the next j/k move from that row, not from a stale index or always from the top.
 */
function syncNavIndexFromClick(ev) {
    const t = ev.target;
    if (!t || t.nodeType !== Node.ELEMENT_NODE) return;
    if (t.isContentEditable || t.closest('[contenteditable]')) return;
    if (t.closest('.ctx-menu')) return;
    const tag = t.tagName;
    if (tag === 'TEXTAREA' || tag === 'SELECT') return;
    if (tag === 'INPUT') {
        const batch = t.classList.contains('batch-cb') && t.type === 'checkbox';
        if (!batch) return;
    }

    const activeTab = document.querySelector('.tab-content.active');
    if (!activeTab) return;
    const id = activeTab.id;
    let row = null;
    if (id === 'tabSamples') row = t.closest('#audioTableBody tr[data-audio-path]');
    else if (id === 'tabDaw') row = t.closest('#dawTableBody tr[data-daw-path]');
    else if (id === 'tabPresets') row = t.closest('#presetTableBody tr[data-preset-path]');
    else if (id === 'tabMidi') row = t.closest('#midiTableBody tr[data-midi-path]');
    else if (id === 'tabPdf') row = t.closest('#pdfTableBody tr[data-pdf-path]');
    else if (id === 'tabPlugins') row = t.closest('.plugin-card');
    else if (id === 'tabFavorites') row = t.closest('.fav-item');
    if (!row) return;

    const items = getNavigableItems();
    const idx = items.indexOf(row);
    if (idx < 0) return;

    clearNavSelection();
    _navIndex = idx;
    row.classList.add('nav-selected');
}

function clearNavSelection() {
    document.querySelectorAll('.nav-selected').forEach(el => el.classList.remove('nav-selected'));
}

function setNavIndex(idx) {
    const items = getNavigableItems();
    if (items.length === 0) return;
    const activeTab = document.querySelector('.tab-content.active')?.id;
    clearNavSelection();
    _navIndex = Math.max(0, Math.min(idx, items.length - 1));
    const item = items[_navIndex];
    item.classList.add('nav-selected');
    item.scrollIntoView({block: 'nearest', behavior: 'smooth'});

    if (activeTab === 'tabSamples' && typeof prefs !== 'undefined') {
        const ap = prefs.getItem('autoPlaySampleOnSelect');
        if (ap !== 'off' && ap !== 'false') {
            const path = item.getAttribute('data-audio-path');
            if (path && path !== _lastAutoPlaySamplePath) {
                _lastAutoPlaySamplePath = path;
                clearTimeout(_sampleSelectPlayTimer);
                _sampleSelectPlayTimer = setTimeout(() => {
                    if (typeof previewAudio === 'function') previewAudio(path);
                    if (typeof syncExpandedMetaWithKeyboardSelection === 'function') syncExpandedMetaWithKeyboardSelection(path);
                }, 140);
            }
        }
    }
}

function activateNavItem() {
    const items = getNavigableItems();
    if (_navIndex < 0 || _navIndex >= items.length) return;
    const item = items[_navIndex];
    const activeTab = document.querySelector('.tab-content.active')?.id;

    if (activeTab === 'tabSamples') {
        const path = item.getAttribute('data-audio-path');
        if (path) previewAudio(path);
    } else if (activeTab === 'tabDaw') {
        const path = item.dataset.dawPath;
        if (path) {
            const name = item.querySelector('.col-name')?.textContent || '';
            const dawName = item.querySelector('.format-badge')?.textContent || 'DAW';
            showToast(toastFmt('toast.opening_in_daw', {name, daw: dawName}));
            window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', {
                daw: dawName,
                err
            }), 4000, 'error'));
        }
    } else if (activeTab === 'tabPresets') {
        const path = item.dataset.presetPath;
        if (path) openPresetFolder(path);
    } else if (activeTab === 'tabPlugins') {
        const kvrBtn = item.querySelector('[data-action="openKvr"]');
        if (kvrBtn) openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name);
    } else if (activeTab === 'tabMidi') {
        const path = item.dataset.midiPath;
        if (path) {
            window.vstUpdater.openDawProject(path).catch((err) => showToast(toastFmt('toast.no_midi_handler', {err}), 4000, 'error'));
        }
    } else if (activeTab === 'tabPdf') {
        const path = item.dataset.pdfPath;
        if (path) {
            const name = item.querySelector('.col-name')?.textContent?.trim()
                || path.split('/').pop()
                || 'PDF';
            window.vstUpdater.openFileDefault(path)
                .then(() => showToast(toastFmt('toast.opening_pdf_default_app', {name})))
                .catch((err) => showToast(toastFmt('toast.failed_open_pdf', {err: err.message || err}), 4000, 'error'));
        }
    } else if (activeTab === 'tabFavorites') {
        const path = item.dataset.path;
        const type = item.dataset.type;
        const name = item.dataset.name || '';
        if (!path) return;
        if (type === 'sample' && typeof previewAudio === 'function') {
            previewAudio(path);
        } else if (type === 'daw') {
            const daw = item.querySelector('.format-badge')?.textContent || 'DAW';
            showToast(toastFmt('toast.opening_in_daw', {name, daw}));
            window.vstUpdater.openDawProject(path).catch((err) => showToast(toastFmt('toast.daw_not_installed', {
                daw,
                err
            }), 4000, 'error'));
        } else if (type === 'preset' && typeof openPresetFolder === 'function') {
            openPresetFolder(path);
        } else if (type === 'plugin') {
            const plugin = (typeof allPlugins !== 'undefined' && typeof findByPath === 'function')
                ? findByPath(allPlugins, path)
                : null;
            const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
            window.vstUpdater.openUpdate(kvrUrl);
        } else if (type === 'pdf') {
            window.vstUpdater.openFileDefault(path)
                .then(() => showToast(toastFmt('toast.opening_pdf_default_app', {name: name || path.split('/').pop()})))
                .catch((err) => showToast(toastFmt('toast.failed_open_pdf', {err: err.message || err}), 4000, 'error'));
        } else if (type === 'midi') {
            window.vstUpdater.openDawProject(path).catch((err) => showToast(toastFmt('toast.no_midi_handler', {err}), 4000, 'error'));
        } else if (type === 'folder' || type === 'file') {
            if (typeof openFolder === 'function') openFolder(path);
        }
    }
}

// Vim g-prefix state
let _vimGPending = false;
let _vimGTimer = null;

function pathFromNavigableElement(item) {
    if (!item) return '';
    return item.getAttribute('data-audio-path')
        || item.dataset.dawPath
        || item.dataset.presetPath
        || item.dataset.midiPath
        || item.dataset.pdfPath
        || item.dataset.path
        || '';
}

function findNavIndexByPath(items, path) {
    if (!path || !items.length) return -1;
    const want = _normNavPathKey(path);
    return items.findIndex((el) => _normNavPathKey(pathFromNavigableElement(el)) === want);
}

/**
 * Row target for shortcuts (F favorite, R reveal, …): keyboard highlight, else playing/expanded row,
 * else exactly one batch-checked row. Updates _navIndex when resolving via batch so j/k stay aligned.
 */
function getResolvedNavItemForActions() {
    const activeTab = document.querySelector('.tab-content.active')?.id;
    if (!activeTab) return null;
    const items = getNavigableItems();
    if (items.length === 0) return null;
    syncNavIndexBeforeVerticalMove(activeTab);
    let item = (_navIndex >= 0 && _navIndex < items.length) ? items[_navIndex] : null;
    if (!item && typeof batchSetForTabId === 'function') {
        const set = batchSetForTabId(activeTab);
        if (set && set.size === 1) {
            const idx = findNavIndexByPath(items, [...set][0]);
            if (idx >= 0) {
                _navIndex = idx;
                item = items[idx];
            }
        }
    }
    if (!item) return null;
    const path = pathFromNavigableElement(item);
    if (!path) return null;
    const name = item.querySelector('.col-name,.plugin-name,h3,.fav-name')?.textContent?.trim() || '';
    return {item, path, name};
}

function _getSelectedPath() {
    const items = getNavigableItems();
    if (_navIndex < 0 || _navIndex >= items.length) return null;
    return pathFromNavigableElement(items[_navIndex]) || '';
}

function _getSelectedName() {
    const items = getNavigableItems();
    if (_navIndex < 0 || _navIndex >= items.length) return '';
    const item = items[_navIndex];
    return item.querySelector('.col-name,.plugin-name,h3,.fav-name')?.textContent?.trim() || '';
}

document.addEventListener('keydown', (e) => {
    // Don't navigate when typing in inputs
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;
    if (e.target.isContentEditable || e.target.closest('[contenteditable]')) return;
    if (e.target.closest('.ctx-menu')) return;

    const isMac = typeof navigator !== 'undefined' && navigator.platform && navigator.platform.includes('Mac');
    const mod = isMac ? e.metaKey : e.ctrlKey;
    // Let shortcuts.js own Cmd/Ctrl+Arrow (volume, prev/next track) — do not also move row selection.
    if (mod && (e.key === 'ArrowUp' || e.key === 'ArrowDown' || e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        return;
    }

    // Help overlay (? / Shift+/) — before activeTab guard so it works even if no inventory tab is focused.
    if (!mod && (e.key === '?' || (e.shiftKey && e.code === 'Slash'))) {
        if (!e.defaultPrevented) {
            e.preventDefault();
            if (typeof toggleHelpOverlay === 'function') toggleHelpOverlay();
        }
        return;
    }

    const activeTab = document.querySelector('.tab-content.active')?.id;
    if (!activeTab) return;
    const items = getNavigableItems();

    // Handle gg (go to top)
    if (_vimGPending) {
        _vimGPending = false;
        clearTimeout(_vimGTimer);
        if (e.key === 'g') {
            e.preventDefault();
            setNavIndex(0);
            return;
        }
    }

    // ── Movement ── (j/k only without Cmd/Ctrl — Cmd+K is command palette)
    if (e.key === 'ArrowDown' || (e.key === 'j' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        syncNavIndexBeforeVerticalMove(activeTab);
        setNavIndex(_navIndex + 1);
    } else if (e.key === 'ArrowUp' || (e.key === 'k' && !e.metaKey && !e.ctrlKey)) {
        e.preventDefault();
        syncNavIndexBeforeVerticalMove(activeTab);
        setNavIndex(_navIndex - 1);
    } else if (e.key === 'Home') {
        e.preventDefault();
        setNavIndex(0);
    } else if (e.key === 'G') {
        e.preventDefault();
        setNavIndex(items.length - 1);
    } else if (e.key === 'g' && !e.metaKey && !e.ctrlKey) {
        // First g — wait for second g
        _vimGPending = true;
        _vimGTimer = setTimeout(() => {
            _vimGPending = false;
        }, 500);
        return;
    } else if (e.key === 'End') {
        e.preventDefault();
        setNavIndex(items.length - 1);

        // ── Half-page scroll ──
    } else if (e.key === 'd' && e.ctrlKey) {
        e.preventDefault();
        syncNavIndexBeforeVerticalMove(activeTab);
        setNavIndex(_navIndex + 15);
    } else if (e.key === 'u' && e.ctrlKey) {
        e.preventDefault();
        syncNavIndexBeforeVerticalMove(activeTab);
        setNavIndex(_navIndex - 15);

        // ── Actions ──
    } else if (e.key === 'Enter') {
        if (items.length > 0) {
            if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
            if (_navIndex >= 0) {
                e.preventDefault();
                activateNavItem();
            }
        }
    } else if (e.key === ' ' && activeTab === 'tabSamples') {
        // Global shortcut (shortcuts.js capture) handles Space for play/pause; skip row preview if so.
        if (e.defaultPrevented) return;
        if (items.length > 0) {
            if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
            if (_navIndex >= 0) {
                e.preventDefault();
                activateNavItem();
            }
        }

    } else if (e.key === 'o') {
        // o = open/reveal in Finder
        e.preventDefault();
        if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
        const path = _getSelectedPath();
        if (path) {
            if (typeof openFolder === 'function') openFolder(path);
            else if (typeof openAudioFolder === 'function') openAudioFolder(path);
        }

    } else if (e.key === 'y') {
        // y = yank (copy path)
        e.preventDefault();
        if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
        const path = _getSelectedPath();
        if (path && typeof copyToClipboard === 'function') copyToClipboard(path);

    } else if (e.key === 'x' || e.key === 'X') {
        // x = delete selected item (same confirm flow as Del / Backspace); F is toggle favorite
        e.preventDefault();
        if (typeof _actionOnSelected === 'function') _actionOnSelected('delete');

    } else if (e.key === 'p') {
        // p = preview/play audio
        e.preventDefault();
        if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
        const path = _getSelectedPath();
        if (path && typeof previewAudio === 'function') previewAudio(path);

    } else if (e.key === '/') {
        // / = focus search (vim search)
        e.preventDefault();
        const activeContent = document.querySelector('.tab-content.active');
        const input = activeContent?.querySelector('input[type="text"]');
        if (input) {
            input.focus();
            input.select();
        }

    } else if (e.key === 'v') {
        // v = toggle batch select on current item
        e.preventDefault();
        if (_navIndex < 0) syncNavIndexBeforeVerticalMove(activeTab);
        if (_navIndex >= 0 && _navIndex < items.length) {
            const cb = items[_navIndex].querySelector('.batch-cb');
            if (cb) {
                cb.checked = !cb.checked;
                cb.dispatchEvent(new Event('change', {bubbles: true}));
            }
        }

    } else if (e.key === 'V') {
        // V = select all visible (visual line mode)
        e.preventDefault();
        if (typeof activeBatchCount === 'function' && activeBatchCount() > 0) deselectAll();
        else selectAllVisible();

    } else if (e.key === 'd' && !e.ctrlKey) {
        // dd would be handled by g-prefix pattern, single d = delete
        // Just use Backspace behavior
    }
}, true); // capture: run before nested scroll regions (e.g. audio player recently played) steal Arrow keys on Samples tab

document.addEventListener('click', syncNavIndexFromClick, true);

// Reset nav index on tab switch
const _origSwitchTab = switchTab;
switchTab = function (tab) {
    _navIndex = -1;
    clearTimeout(_sampleSelectPlayTimer);
    _sampleSelectPlayTimer = null;
    _lastAutoPlaySamplePath = null;
    clearNavSelection();
    _origSwitchTab(tab);
};
