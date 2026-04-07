// ── Settings search ──
// Filters .settings-row elements by matching against title / description /
// section heading text. Hides whole sections when none of their rows match.
// Pure client-side DOM filter — no async, no debounce needed (fast).

(function () {
    let _settingsDebounce = null;

    function normalize(s) {
        return (s || '').toLowerCase().trim();
    }

    function collectRowText(row) {
        // Title + description + label text. Also include control labels (option text, settings-desc).
        const parts = [];
        const title = row.querySelector('.settings-title');
        if (title) parts.push(title.textContent || '');
        const desc = row.querySelector('.settings-desc');
        if (desc) parts.push(desc.textContent || '');
        // Include input/select/textarea values + option labels so e.g. searching
        // "cyberpunk" finds the Color Scheme dropdown.
        const selects = row.querySelectorAll('select');
        for (const s of selects) {
            for (const opt of s.options) parts.push(opt.textContent || '');
        }
        return normalize(parts.join(' '));
    }

    function filterSettings() {
        const input = document.getElementById('settingsSearchInput');
        if (!input) return;
        const q = normalize(input.value);
        const clearBtn = document.getElementById('clearSettingsSearchBtn');
        if (clearBtn) clearBtn.style.display = q ? '' : 'none';

        const sections = document.querySelectorAll('#tabSettings .settings-section');
        let totalVisible = 0;

        for (const section of sections) {
            const heading = section.querySelector('.settings-heading');
            const sectionText = normalize(heading ? heading.textContent : '');
            const sectionMatches = !q || sectionText.includes(q);

            let sectionHasVisible = false;
            const rows = section.querySelectorAll('.settings-row');
            for (const row of rows) {
                if (!q) {
                    row.style.display = '';
                    sectionHasVisible = true;
                    continue;
                }
                const rowText = collectRowText(row);
                // A row matches if the row's own text matches OR the whole section
                // heading matches (so clicking "Appearance" shows all appearance rows).
                const match = sectionMatches || rowText.includes(q);
                row.style.display = match ? '' : 'none';
                if (match) sectionHasVisible = true;
            }
            // Hide the whole section if no rows visible (and no query-match on heading).
            section.style.display = (sectionHasVisible || sectionMatches) ? '' : 'none';
            if (sectionHasVisible || sectionMatches) totalVisible++;
        }

        const emptyEl = document.getElementById('settingsSearchEmpty');
        if (emptyEl) emptyEl.style.display = (q && totalVisible === 0) ? '' : 'none';
    }

    function filterSettingsDebounced() {
        clearTimeout(_settingsDebounce);
        _settingsDebounce = setTimeout(filterSettings, 80);
    }

    function clearSettingsSearch() {
        const input = document.getElementById('settingsSearchInput');
        if (input) {
            input.value = '';
            input.focus();
        }
        filterSettings();
    }

    // Expose for action dispatchers
    window.filterSettings = filterSettings;
    window.filterSettingsDebounced = filterSettingsDebounced;
    window.clearSettingsSearch = clearSettingsSearch;

    // Escape key clears the search while focused in the input
    document.addEventListener('keydown', (e) => {
        if (e.key !== 'Escape') return;
        const input = document.getElementById('settingsSearchInput');
        if (!input || document.activeElement !== input) return;
        if (input.value) {
            e.stopPropagation();
            e.preventDefault();
            clearSettingsSearch();
        }
    }, true);
})();
