// ── Multi-select filter dropdowns ──
// Converts .filter-select elements into multi-select checkbox dropdowns.
// Large option lists (>200) get a search input and capped rendering so
// 17k sample packs don't create 17k DOM nodes.

const MULTI_FILTER_RENDER_CAP = 200;
const MULTI_FILTER_DROPDOWN_MIN_WIDTH = 160;
const MULTI_FILTER_VIEWPORT_PADDING = 8;

/* Position a `position: fixed` dropdown so it sits flush below its trigger button.
 * Falls back to right-aligned / above placement if the natural placement clips
 * the viewport — same pattern as native `<select>` popovers. Called on open and
 * on window scroll/resize while open. */
function positionMultiFilterDropdown(btn, dropdown) {
    if (!btn || !dropdown) return;
    const r = btn.getBoundingClientRect();
    const vw = window.innerWidth || document.documentElement.clientWidth || 0;
    const vh = window.innerHeight || document.documentElement.clientHeight || 0;
    const pad = MULTI_FILTER_VIEWPORT_PADDING;

    const naturalWidth = Math.max(r.width, MULTI_FILTER_DROPDOWN_MIN_WIDTH);
    const maxWidth = Math.max(MULTI_FILTER_DROPDOWN_MIN_WIDTH, vw - pad * 2);
    const width = Math.min(naturalWidth, maxWidth);
    dropdown.style.width = width + 'px';

    const roomBelow = Math.max(0, vh - r.bottom - pad);
    const roomAbove = Math.max(0, r.top - pad);
    const placeAbove = roomBelow < 160 && roomAbove > roomBelow;
    const maxH = Math.max(120, placeAbove ? roomAbove : roomBelow);
    dropdown.style.maxHeight = maxH + 'px';

    let left = r.left;
    if (left + width > vw - pad) left = Math.max(pad, vw - pad - width);
    if (left < pad) left = pad;
    dropdown.style.left = left + 'px';

    if (placeAbove) {
        dropdown.style.top = '';
        dropdown.style.bottom = (vh - r.top + 2) + 'px';
    } else {
        dropdown.style.bottom = '';
        dropdown.style.top = (r.bottom + 2) + 'px';
    }
}

function _repositionOpenMultiFilterDropdowns() {
    const open = document.querySelector('.multi-filter-dropdown.open');
    if (!open) return;
    const wrapper = open._wrapper;
    const btn = wrapper && wrapper.querySelector('.multi-filter-btn');
    if (btn) positionMultiFilterDropdown(btn, open);
}

if (typeof window !== 'undefined') {
    window.addEventListener('resize', _repositionOpenMultiFilterDropdowns);
    window.addEventListener('scroll', _repositionOpenMultiFilterDropdowns, true);
}

function initMultiFilters() {
    document.querySelectorAll('.filter-select').forEach(select => {
        if (select.dataset.multiInit) return;
        select.dataset.multiInit = '1';

        const action = select.dataset.action;

        // Hide original select
        select.style.display = 'none';

        // Create multi-select widget
        const wrapper = document.createElement('div');
        wrapper.className = 'multi-filter';
        wrapper.title = select.title || '';

        const btn = document.createElement('button');
        btn.className = 'multi-filter-btn';
        btn.type = 'button';
        btn.innerHTML = `<span class="multi-filter-label">All</span><span class="multi-filter-arrow">&#9660;</span>`;
        wrapper.appendChild(btn);

        const dropdown = document.createElement('div');
        dropdown.className = 'multi-filter-dropdown';
        /* Append to `document.body`, NOT inside the wrapper. `position: fixed` is
         * normally viewport-relative, BUT if any ancestor has `transform`/`filter`/
         * `will-change`/etc. that ancestor becomes the containing block and "viewport
         * coords" silently shift to it — the dropdown lands hundreds of px off.
         * Body-mount sidesteps every ancestor. */
        document.body.appendChild(dropdown);

        select.parentNode.insertBefore(wrapper, select.nextSibling);

        // State
        wrapper._selected = new Set(); // empty = all
        wrapper._select = select;
        wrapper._action = action;
        /* Pull current `<select>` options at init time so static filters (audio
         * format, preset format, video format, etc.) render their hardcoded entries
         * without waiting for an external `refreshMultiFilter(...)` call. */
        const _initOpts = [...select.options].filter(o => o.value !== '' && o.value !== 'all');
        wrapper._allOptions = _initOpts.map(o => ({value: o.value, text: o.text}));
        wrapper._allLabel = select.options[0]?.text || 'All';
        wrapper._search = '';
        wrapper._dropdown = dropdown;
        dropdown._wrapper = wrapper;

        // Toggle dropdown
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            document.querySelectorAll('.multi-filter-dropdown.open').forEach(d => {
                if (d !== dropdown) d.classList.remove('open');
            });
            const willOpen = !dropdown.classList.contains('open');
            dropdown.classList.toggle('open');
            if (willOpen) {
                positionMultiFilterDropdown(btn, dropdown);
                const input = dropdown.querySelector('.multi-filter-search');
                if (input) input.focus();
            }
        });

        // Delegated change handler for checkboxes
        dropdown.addEventListener('change', (e) => {
            const cb = e.target;
            if (!cb.matches('input[type="checkbox"]')) return;
            const val = cb.dataset.value;

            if (val === 'all') {
                if (cb.checked) {
                    wrapper._selected.clear();
                    dropdown.querySelectorAll('input[data-value]').forEach(c => {
                        c.checked = c.dataset.value === 'all';
                    });
                }
            } else {
                if (cb.checked) {
                    wrapper._selected.add(val);
                } else {
                    wrapper._selected.delete(val);
                }
                const allCb = dropdown.querySelector('input[data-value="all"]');
                if (wrapper._selected.size === 0) {
                    if (allCb) allCb.checked = true;
                } else {
                    if (allCb) allCb.checked = false;
                }
            }

            updateMultiFilterLabel(wrapper, wrapper._allLabel);
            syncMultiToSelect(wrapper);
            triggerFilter(wrapper._action);
        });

        dropdown.addEventListener('click', (e) => e.stopPropagation());

        // Build initial content from current <select> options
        _rebuildDropdown(wrapper);
    });

    // Close all dropdowns on outside click
    document.addEventListener('click', () => {
        document.querySelectorAll('.multi-filter-dropdown.open').forEach(d => d.classList.remove('open'));
    });
}

/// Internal: rebuild dropdown DOM from wrapper._allOptions, filtered by wrapper._search.
/// Only renders up to MULTI_FILTER_RENDER_CAP items; shows a "N more" hint when truncated.
function _rebuildDropdown(wrapper) {
    const dropdown = wrapper._dropdown;
    if (!dropdown) return;
    const opts = wrapper._allOptions;
    const allLabel = wrapper._allLabel;
    const search = wrapper._search.toLowerCase();
    const needsSearch = opts.length > MULTI_FILTER_RENDER_CAP;

    // Filter
    const filtered = search
        ? opts.filter(o => o.text.toLowerCase().includes(search))
        : opts;
    const capped = filtered.slice(0, MULTI_FILTER_RENDER_CAP);
    const overflow = filtered.length - capped.length;

    // Build HTML in one shot — no per-item createElement
    let html = '';

    // Search input (only for large lists)
    if (needsSearch) {
        const v = wrapper._search.replace(/"/g, '&quot;');
        html += `<div class="multi-filter-search-row"><input type="text" class="multi-filter-search" placeholder="Search\u2026" value="${v}" spellcheck="false"></div>`;
    }

    // "All" toggle
    const allChecked = wrapper._selected.size === 0 ? ' checked' : '';
    html += `<label class="multi-filter-item multi-filter-all"><input type="checkbox"${allChecked} data-value="all"> <span>${allLabel}</span></label>`;
    html += `<div class="multi-filter-sep"></div>`;

    // Selected items always shown at top (regardless of search/cap)
    const selectedInView = new Set();
    if (wrapper._selected.size > 0) {
        for (const val of wrapper._selected) {
            const opt = opts.find(o => o.value === val);
            if (opt) {
                html += `<label class="multi-filter-item multi-filter-pinned"><input type="checkbox" checked data-value="${opt.value}"> <span>${opt.text}</span></label>`;
                selectedInView.add(val);
            }
        }
        if (selectedInView.size > 0) {
            html += `<div class="multi-filter-sep"></div>`;
        }
    }

    // Visible items (skip those already pinned)
    for (const opt of capped) {
        if (selectedInView.has(opt.value)) continue;
        const chk = wrapper._selected.has(opt.value) ? ' checked' : '';
        html += `<label class="multi-filter-item"><input type="checkbox"${chk} data-value="${opt.value}"> <span>${opt.text}</span></label>`;
    }

    if (overflow > 0) {
        html += `<div class="multi-filter-overflow">${overflow.toLocaleString()} more \u2014 type to search</div>`;
    }

    dropdown.innerHTML = html;

    // Wire up search input
    if (needsSearch) {
        const input = dropdown.querySelector('.multi-filter-search');
        if (input) {
            let debounce;
            input.addEventListener('input', () => {
                clearTimeout(debounce);
                debounce = setTimeout(() => {
                    wrapper._search = input.value;
                    _rebuildDropdown(wrapper);
                    // Re-focus after rebuild
                    const newInput = dropdown.querySelector('.multi-filter-search');
                    if (newInput) {
                        newInput.focus();
                        newInput.selectionStart = newInput.selectionEnd = newInput.value.length;
                    }
                }, 150);
            });
            // Prevent dropdown close on typing
            input.addEventListener('click', (e) => e.stopPropagation());
        }
    }
}

/// Rebuild the multi-filter dropdown options from the current `<select>` options.
/// Call after `renderCrateFacets()` or any time the hidden `<select>` innerHTML changes.
function refreshMultiFilter(selectId) {
    const select = document.getElementById(selectId);
    if (!select) return;
    const wrapper = select.nextElementSibling;
    if (!wrapper || !wrapper.classList.contains('multi-filter')) return;

    const options = [...select.options].filter(o => o.value !== '' && o.value !== 'all');
    wrapper._allLabel = select.options[0]?.text || 'All';
    wrapper._allOptions = options.map(o => ({value: o.value, text: o.text}));
    wrapper._search = '';

    // Prune stale selections
    const validValues = new Set(options.map(o => o.value));
    for (const v of wrapper._selected) {
        if (!validValues.has(v)) wrapper._selected.delete(v);
    }

    _rebuildDropdown(wrapper);
    updateMultiFilterLabel(wrapper, wrapper._allLabel);
}

function updateMultiFilterLabel(wrapper, allLabel) {
    const label = wrapper.querySelector('.multi-filter-label');
    if (!label) return;
    if (wrapper._selected.size === 0) {
        label.textContent = allLabel;
        label.classList.remove('multi-filter-active');
    } else if (wrapper._selected.size === 1) {
        const val = [...wrapper._selected][0];
        const opt = (wrapper._allOptions || []).find(o => o.value === val);
        label.textContent = opt ? opt.text : val;
        label.classList.add('multi-filter-active');
    } else {
        const n = wrapper._selected.size;
        label.textContent = typeof appFmt === 'function'
            ? appFmt('menu.batch_selected', {n})
            : n + ' selected';
        label.classList.add('multi-filter-active');
    }
}

function syncMultiToSelect(wrapper) {
    const select = wrapper._select;
    if (wrapper._selected.size === 0) {
        select.value = '';
    } else {
        select.value = [...wrapper._selected][0];
    }
    // Fire change on the hidden <select> so listeners (e.g. crate tab filters) pick it up.
    if (typeof Event !== 'undefined' && typeof select.dispatchEvent === 'function') {
        select.dispatchEvent(new Event('change', {bubbles: true}));
    }
}

function triggerFilter(action) {
    if (action === 'filterPlugins' && typeof filterPlugins === 'function') filterPlugins();
    else if (action === 'filterAudioSamples' && typeof filterAudioSamples === 'function') filterAudioSamples();
    else if (action === 'filterDawProjects' && typeof filterDawProjects === 'function') filterDawProjects();
    else if (action === 'filterPresets' && typeof filterPresets === 'function') filterPresets();
    else if (action === 'filterFavorites' && typeof renderFavorites === 'function') renderFavorites();
    else if (_filterRegistry && _filterRegistry[action]) applyFilter(action);
}

// Get selected values for a multi-filter. Returns null if "all", or a Set of values.
function getMultiFilterValues(selectId) {
    const select = document.getElementById(selectId);
    if (!select) return null;
    const wrapper = select.nextElementSibling;
    if (!wrapper || !wrapper.classList.contains('multi-filter')) return null;
    if (wrapper._selected.size === 0) return null; // all
    return wrapper._selected;
}

// Programmatically set a multi-filter value (used by autoSelectDropdown)
function setMultiFilterValue(selectId, value) {
    const select = document.getElementById(selectId);
    if (!select) return;
    const wrapper = select.nextElementSibling;
    if (!wrapper || !wrapper.classList.contains('multi-filter')) return;

    if (value === 'all') {
        wrapper._selected.clear();
    } else {
        wrapper._selected.add(String(value));
        // Remove 'all' from selected
    }
    const allLabel = wrapper._allLabel || select.options[0]?.text || 'All';
    _rebuildDropdown(wrapper);
    updateMultiFilterLabel(wrapper, allLabel);
    syncMultiToSelect(wrapper);
}
