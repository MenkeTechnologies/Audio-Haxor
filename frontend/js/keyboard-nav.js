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
    if (id === 'tabFavorites') return [...activeTab.querySelectorAll('.fav-item')];
    return [];
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
    }
}

// Vim g-prefix state
let _vimGPending = false;
let _vimGTimer = null;

function _getSelectedPath() {
    const items = getNavigableItems();
    if (_navIndex < 0 || _navIndex >= items.length) return null;
    const item = items[_navIndex];
    return item.getAttribute('data-audio-path') || item.dataset.dawPath || item.dataset.presetPath || item.dataset.path || '';
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

    // ── Movement ──
    if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault();
        setNavIndex(_navIndex + 1);
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
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
        setNavIndex(_navIndex + 15);
    } else if (e.key === 'u' && e.ctrlKey) {
        e.preventDefault();
        setNavIndex(_navIndex - 15);

        // ── Actions ──
    } else if (e.key === 'Enter') {
        if (_navIndex >= 0) {
            e.preventDefault();
            activateNavItem();
        }
    } else if (e.key === ' ' && activeTab === 'tabSamples') {
        // Global shortcut (shortcuts.js capture) handles Space for play/pause; skip row preview if so.
        if (e.defaultPrevented) return;
        if (_navIndex >= 0) {
            e.preventDefault();
            activateNavItem();
        }

    } else if (e.key === 'o') {
        // o = open/reveal in Finder
        e.preventDefault();
        const path = _getSelectedPath();
        if (path) {
            if (typeof openFolder === 'function') openFolder(path);
            else if (typeof openAudioFolder === 'function') openAudioFolder(path);
        }

    } else if (e.key === 'y') {
        // y = yank (copy path)
        e.preventDefault();
        const path = _getSelectedPath();
        if (path && typeof copyToClipboard === 'function') copyToClipboard(path);

    } else if (e.key === 'x') {
        // x = toggle favorite
        e.preventDefault();
        const path = _getSelectedPath();
        const name = _getSelectedName();
        if (path && typeof isFavorite === 'function') {
            if (isFavorite(path)) {
                if (typeof removeFavorite === 'function') removeFavorite(path);
            } else {
                if (typeof addFavorite === 'function') addFavorite('item', path, name);
            }
        }

    } else if (e.key === 'p') {
        // p = preview/play audio
        e.preventDefault();
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
        if (batchSelected.size > 0) deselectAll();
        else selectAllVisible();

    } else if (e.key === 'd' && !e.ctrlKey) {
        // dd would be handled by g-prefix pattern, single d = delete
        // Just use Backspace behavior
    } else if (e.key === '?') {
        e.preventDefault();
        toggleHelpOverlay();
    }
}, true); // capture: run before nested scroll regions (e.g. audio player recently played) steal Arrow keys on Samples tab

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
