// ── Multi-select filter dropdowns ──
// Converts .filter-select elements into multi-select checkbox dropdowns

function initMultiFilters() {
    document.querySelectorAll('.filter-select').forEach(select => {
        if (select.dataset.multiInit) return;
        select.dataset.multiInit = '1';

        const options = [...select.options].filter(o => o.value !== 'all');
        const allLabel = select.options[0]?.text || 'All';
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
        btn.innerHTML = `<span class="multi-filter-label">${allLabel}</span><span class="multi-filter-arrow">&#9660;</span>`;
        wrapper.appendChild(btn);

        const dropdown = document.createElement('div');
        dropdown.className = 'multi-filter-dropdown';

        // "All" toggle
        const allItem = document.createElement('label');
        allItem.className = 'multi-filter-item multi-filter-all';
        allItem.innerHTML = `<input type="checkbox" checked data-value="all"> <span>${allLabel}</span>`;
        dropdown.appendChild(allItem);

        // Separator
        const sep = document.createElement('div');
        sep.className = 'multi-filter-sep';
        dropdown.appendChild(sep);

        // Individual options
        for (const opt of options) {
            const item = document.createElement('label');
            item.className = 'multi-filter-item';
            item.innerHTML = `<input type="checkbox" data-value="${opt.value}"> <span>${opt.text}</span>`;
            dropdown.appendChild(item);
        }

        wrapper.appendChild(dropdown);
        select.parentNode.insertBefore(wrapper, select.nextSibling);

        // State
        wrapper._selected = new Set(); // empty = all
        wrapper._select = select;
        wrapper._action = action;

        // Toggle dropdown
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            // Close other open dropdowns
            document.querySelectorAll('.multi-filter-dropdown.open').forEach(d => {
                if (d !== dropdown) d.classList.remove('open');
            });
            dropdown.classList.toggle('open');
        });

        // Checkbox change
        dropdown.addEventListener('change', (e) => {
            const cb = e.target;
            if (!cb.matches('input[type="checkbox"]')) return;
            const val = cb.dataset.value;

            if (val === 'all') {
                // Toggle all off/on
                if (cb.checked) {
                    wrapper._selected.clear();
                    dropdown.querySelectorAll('input[data-value]').forEach(c => {
                        c.checked = c.dataset.value === 'all';
                    });
                }
            } else {
                const allCb = dropdown.querySelector('input[data-value="all"]');
                if (cb.checked) {
                    wrapper._selected.add(val);
                } else {
                    wrapper._selected.delete(val);
                }
                // If nothing selected, revert to all
                if (wrapper._selected.size === 0) {
                    allCb.checked = true;
                } else {
                    allCb.checked = false;
                }
            }

            updateMultiFilterLabel(wrapper, allLabel);
            // Sync back to hidden select for compat
            syncMultiToSelect(wrapper);
            triggerFilter(wrapper._action);
        });

        // Prevent dropdown close on click inside
        dropdown.addEventListener('click', (e) => e.stopPropagation());
    });

    // Close all dropdowns on outside click
    document.addEventListener('click', () => {
        document.querySelectorAll('.multi-filter-dropdown.open').forEach(d => d.classList.remove('open'));
    });
}

function updateMultiFilterLabel(wrapper, allLabel) {
    const label = wrapper.querySelector('.multi-filter-label');
    if (wrapper._selected.size === 0) {
        label.textContent = allLabel;
        label.classList.remove('multi-filter-active');
    } else if (wrapper._selected.size === 1) {
        label.textContent = [...wrapper._selected][0];
        label.classList.add('multi-filter-active');
    } else {
        label.textContent = wrapper._selected.size + ' selected';
        label.classList.add('multi-filter-active');
    }
}

function syncMultiToSelect(wrapper) {
    const select = wrapper._select;
    // Set a custom property for the filter functions to read
    if (wrapper._selected.size === 0) {
        select.value = 'all';
    } else {
        // Set to first selected value (for single-value compat)
        select.value = [...wrapper._selected][0];
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
    const dropdown = wrapper.querySelector('.multi-filter-dropdown');
    if (!dropdown) return;

    if (value === 'all') {
        wrapper._selected.clear();
        dropdown.querySelectorAll('input[data-value]').forEach(c => {
            c.checked = c.dataset.value === 'all';
        });
    } else {
        wrapper._selected.add(value);
        const allCb = dropdown.querySelector('input[data-value="all"]');
        if (allCb) allCb.checked = false;
        const cb = dropdown.querySelector(`input[data-value="${value}"]`);
        if (cb) cb.checked = true;
    }
    const allLabel = select.options[0]?.text || 'All';
    updateMultiFilterLabel(wrapper, allLabel);
    syncMultiToSelect(wrapper);
}
