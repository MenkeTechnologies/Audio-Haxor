// ── Sort State Persistence ──
// Saves and restores the last-used sort column and direction per tab.
// Sort state is saved directly when sort functions are called (no MutationObserver needed).

function saveSortState(tab, key, asc) {
    prefs.setItem(`sort_${tab}`, JSON.stringify({key, asc}));
}

function restoreSortState(tab) {
    const saved = prefs.getItem(`sort_${tab}`);
    if (!saved) return null;
    try {
        return JSON.parse(saved);
    } catch {
        return null;
    }
}

function restoreAllSortStates() {
    const plugin = restoreSortState('plugin');
    if (plugin && typeof _pluginSortKey !== 'undefined') {
        _pluginSortKey = plugin.key;
        _pluginSortAsc = plugin.asc;
    }
    // Fallback: seed from settings dropdown if no runtime sort saved
    if (!plugin && typeof _pluginSortKey !== 'undefined') {
        const def = prefs.getItem('pluginSort');
        if (def) {
            const [k, d] = def.split('-');
            _pluginSortKey = k || 'name';
            _pluginSortAsc = d !== 'desc';
        }
    }
    const audio = restoreSortState('audio');
    if (audio && typeof audioSortKey !== 'undefined') {
        audioSortKey = audio.key;
        audioSortAsc = audio.asc;
    }
    const daw = restoreSortState('daw');
    if (daw && typeof dawSortKey !== 'undefined') {
        dawSortKey = daw.key;
        dawSortAsc = daw.asc;
    }
    const preset = restoreSortState('preset');
    if (preset && typeof presetSortKey !== 'undefined') {
        presetSortKey = preset.key;
        presetSortAsc = preset.asc;
    }
    if (!preset && typeof presetSortKey !== 'undefined') {
        const def = prefs.getItem('presetSort');
        if (def) {
            presetSortKey = def;
            presetSortAsc = true;
        }
    }
    const midi = restoreSortState('midi');
    if (midi && typeof midiSortKey !== 'undefined') {
        midiSortKey = midi.key;
        midiSortAsc = midi.asc;
    }
    if (!midi && typeof midiSortKey !== 'undefined') {
        const def = prefs.getItem('midiSort');
        if (def) {
            midiSortKey = def;
            midiSortAsc = true;
        }
    }
    const pdf = restoreSortState('pdf');
    if (pdf && typeof pdfSortKey !== 'undefined') {
        pdfSortKey = pdf.key;
        pdfSortAsc = pdf.asc;
    }
    if (!pdf && typeof pdfSortKey !== 'undefined') {
        const def = prefs.getItem('pdfSort');
        if (def) {
            pdfSortKey = def;
            pdfSortAsc = true;
        }
    }
}

function initSortPersistence() {
    restoreAllSortStates();
}
