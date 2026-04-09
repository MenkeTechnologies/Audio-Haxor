/**
 * Parity: `src-tauri` extension tables ↔ `frontend/index.html` filter `<select>`s
 * and `frontend/js/file-browser.js` AUDIO_EXTS. Fails when a scanner adds/removes
 * a type but the UI dropdown (or Files-tab audio list) is not updated.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');

const ROOT = path.join(__dirname, '..');

function read(rel) {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

/**
 * First `const NAME: ... = &[ ... ];` block after `marker` (e.g. `const AUDIO_EXTENSIONS`).
 */
function extractRustStrArray(src, constName) {
  const marker = `const ${constName}`;
  const i = src.indexOf(marker);
  if (i === -1) {
    throw new Error(`missing ${marker}`);
  }
  const from = src.slice(i);
  const open = from.indexOf('[');
  const rest = from.slice(open);
  let depth = 0;
  let end = -1;
  for (let j = 0; j < rest.length; j++) {
    const c = rest[j];
    if (c === '[') depth++;
    else if (c === ']') {
      depth--;
      if (depth === 0) {
        const semi = rest.indexOf(';', j);
        if (semi === -1) throw new Error(`no ';' after ${constName} array`);
        end = open + semi + 1;
        break;
      }
    }
  }
  if (end === -1) throw new Error(`unclosed array for ${constName}`);
  const body = src.slice(i + open, i + end - 1);
  const out = [];
  for (const m of body.matchAll(/"([^"]+)"/g)) {
    out.push(m[1]);
  }
  return out;
}

function rustDotExtToFilterValue(dotExt) {
  return dotExt.replace(/^\./, '').toUpperCase();
}

function extractHtmlSelectValues(html, selectId) {
  const idAttr = `id="${selectId}"`;
  const idx = html.indexOf(idAttr);
  if (idx === -1) {
    throw new Error(`missing <select ${idAttr}>`);
  }
  const selStart = html.lastIndexOf('<select', idx);
  const selEnd = html.indexOf('</select>', selStart);
  if (selStart === -1 || selEnd === -1) {
    throw new Error(`malformed select ${selectId}`);
  }
  const block = html.slice(selStart, selEnd);
  const values = [];
  for (const m of block.matchAll(/<option[^>]*value="([^"]+)"/g)) {
    if (m[1] !== 'all') values.push(m[1]);
  }
  return values;
}

function extractDawNamesFromRust(src) {
  const start = src.indexOf('pub fn daw_name_for_format');
  if (start === -1) throw new Error('missing daw_name_for_format');
  const brace = src.indexOf('{', start);
  const matchKw = src.indexOf('match format', brace);
  const open = src.indexOf('{', matchKw);
  let depth = 0;
  let end = -1;
  for (let i = open; i < src.length; i++) {
    const c = src[i];
    if (c === '{') depth++;
    else if (c === '}') {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end === -1) throw new Error('unclosed daw_name_for_format match');
  const block = src.slice(open, end + 1);
  const names = new Set();
  for (const m of block.matchAll(/=>\s*"([^"]+)"/g)) {
    if (m[1] !== 'Unknown') names.add(m[1]);
  }
  return [...names];
}

/** `'wav', 'mp3'` or single-quoted strings in `const AUDIO_EXTS = [ ... ];` */
function extractJsQuotedArray(src, constName) {
  const marker = `const ${constName}`;
  const i = src.indexOf(marker);
  if (i === -1) throw new Error(`missing ${constName}`);
  const from = src.slice(i);
  const open = from.indexOf('[');
  const rest = from.slice(open);
  let depth = 0;
  let end = -1;
  for (let j = 0; j < rest.length; j++) {
    const c = rest[j];
    if (c === '[') depth++;
    else if (c === ']') {
      depth--;
      if (depth === 0) {
        const semi = rest.indexOf(';', j);
        if (semi === -1) throw new Error(`no ';' after ${constName}`);
        end = open + semi + 1;
        break;
      }
    }
  }
  const body = src.slice(i + open, i + end - 1);
  const out = [];
  for (const m of body.matchAll(/'([^']+)'/g)) {
    out.push(m[1]);
  }
  return out;
}

/** Map lines like `'wav': 'WAV',` between audio comment and `// Plugin` */
function extractUtilsExtToFilterAudio(utilsSrc) {
  const start = utilsSrc.indexOf('// Audio formats');
  const end = utilsSrc.indexOf('// Plugin types', start);
  if (start === -1 || end === -1) throw new Error('EXT_TO_FILTER audio block not found');
  const block = utilsSrc.slice(start, end);
  const map = {};
  for (const m of block.matchAll(/'([^']+)':\s*'([^']+)'/g)) {
    map[m[1]] = m[2];
  }
  return map;
}

function assertSameSet(label, a, b) {
  const sa = [...new Set(a)].sort();
  const sb = [...new Set(b)].sort();
  assert.deepStrictEqual(sa, sb, `${label}: expected same set, got different lengths or values`);
}

describe('scanner ↔ unified_walker ↔ dropdown parity', () => {
  const audioScanner = read('src-tauri/src/audio_scanner.rs');
  const unified = read('src-tauri/src/unified_walker.rs');
  const presetScanner = read('src-tauri/src/preset_scanner.rs');
  const dawScanner = read('src-tauri/src/daw_scanner.rs');
  const indexHtml = read('frontend/index.html');
  const fileBrowser = read('frontend/js/file-browser.js');
  const utilsSrc = read('frontend/js/utils.js');

  it('AUDIO_EXTENSIONS matches between audio_scanner.rs and unified_walker.rs', () => {
    const a = extractRustStrArray(audioScanner, 'AUDIO_EXTENSIONS');
    const b = extractRustStrArray(unified, 'AUDIO_EXTENSIONS');
    assertSameSet('AUDIO_EXTENSIONS', a, b);
  });

  it('PRESET_EXTENSIONS matches between preset_scanner.rs and unified_walker.rs', () => {
    const a = extractRustStrArray(presetScanner, 'PRESET_EXTENSIONS');
    const b = extractRustStrArray(unified, 'PRESET_EXTENSIONS');
    assertSameSet('PRESET_EXTENSIONS', a, b);
  });

  it('DAW_EXTENSIONS matches between daw_scanner.rs and unified_walker.rs', () => {
    const a = extractRustStrArray(dawScanner, 'DAW_EXTENSIONS');
    const b = extractRustStrArray(unified, 'DAW_EXTENSIONS');
    assertSameSet('DAW_EXTENSIONS', a, b);
  });

  it('Samples #audioFormatFilter options match AUDIO_EXTENSIONS (uppercase)', () => {
    const rust = extractRustStrArray(audioScanner, 'AUDIO_EXTENSIONS');
    const expected = rust.map(rustDotExtToFilterValue);
    const html = extractHtmlSelectValues(indexHtml, 'audioFormatFilter');
    assertSameSet('audioFormatFilter', expected, html);
  });

  it('Presets #presetFormatFilter options match PRESET_EXTENSIONS (uppercase)', () => {
    const rust = extractRustStrArray(presetScanner, 'PRESET_EXTENSIONS');
    const expected = rust.map(rustDotExtToFilterValue);
    const html = extractHtmlSelectValues(indexHtml, 'presetFormatFilter');
    assertSameSet('presetFormatFilter', expected, html);
  });

  it('DAW #dawDawFilter options match daw_name_for_format display names', () => {
    const expected = extractDawNamesFromRust(dawScanner);
    const html = extractHtmlSelectValues(indexHtml, 'dawDawFilter');
    assertSameSet('dawDawFilter', expected, html);
  });

  it('file-browser AUDIO_EXTS (lowercase) matches AUDIO_EXTENSIONS', () => {
    const rust = extractRustStrArray(audioScanner, 'AUDIO_EXTENSIONS');
    const expected = rust.map((e) => e.replace(/^\./, '').toLowerCase());
    const js = extractJsQuotedArray(fileBrowser, 'AUDIO_EXTS');
    assertSameSet('AUDIO_EXTS', expected, js);
  });

  it('utils EXT_TO_FILTER audio keys cover every AUDIO_EXTENSIONS entry', () => {
    const rust = extractRustStrArray(audioScanner, 'AUDIO_EXTENSIONS');
    const map = extractUtilsExtToFilterAudio(utilsSrc);
    for (const dot of rust) {
      const ext = dot.replace(/^\./, '').toLowerCase();
      const want = rustDotExtToFilterValue(dot);
      assert.ok(
        Object.prototype.hasOwnProperty.call(map, ext),
        `EXT_TO_FILTER missing key '${ext}' (for ${dot})`
      );
      assert.strictEqual(
        map[ext],
        want,
        `EXT_TO_FILTER['${ext}'] should be '${want}'`
      );
    }
  });
});
