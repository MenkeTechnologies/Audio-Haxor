// Apply SQLite-backed strings to elements annotated with data-i18n* (see scripts/gen_app_i18n_en.py, i18n/*.json).
/** Catalogs store line breaks as the literal text `&#10;` (same as HTML attributes). JS `.placeholder` needs real newlines. */
function decodeI18nPlaceholderValue(s) {
    if (typeof s !== 'string') return s;
    return s.replace(/&#10;/g, '\n').replace(/&#13;/g, '\r');
}

function applyUiI18n() {
    const m = window.__appStr || {};
    if (!m || typeof m !== 'object') return;
    document.querySelectorAll('[data-i18n]').forEach((el) => {
        const k = el.dataset.i18n;
        if (k && m[k] != null && m[k] !== '') el.textContent = m[k];
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
        const k = el.dataset.i18nPlaceholder;
        if (k && m[k] != null && m[k] !== '') el.placeholder = decodeI18nPlaceholderValue(m[k]);
    });
    document.querySelectorAll('[data-i18n-title]').forEach((el) => {
        const k = el.dataset.i18nTitle;
        if (k && m[k] != null && m[k] !== '') el.title = m[k];
    });
}

window.applyUiI18n = applyUiI18n;
