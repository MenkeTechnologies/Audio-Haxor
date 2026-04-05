/**
 * Contract: every registerFilter() tab routes search through utils.js searchScore / searchMatch
 * (fzf extended syntax: quotes, ^prefix, suffix$, !negate, | OR, regex toggle).
 */
const fs = require('fs');
const path = require('path');
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

const repoRoot = path.join(__dirname, '..');

/** Each file that calls registerFilter — must use unified search (not raw includes alone). */
const REGISTER_FILTER_TABS = [
  ['frontend/js/daw.js', ['filterDawProjects']],
  ['frontend/js/audio.js', ['filterAudioSamples', 'filterNowPlaying']],
  ['frontend/js/presets.js', ['filterPresets']],
  ['frontend/js/pdf.js', ['filterPdfs']],
  ['frontend/js/plugins.js', ['filterPlugins']],
  ['frontend/js/file-browser.js', ['filterFiles']],
  ['frontend/js/midi.js', ['filterMidi']],
  ['frontend/js/shortcuts.js', ['filterShortcuts']],
  ['frontend/js/favorites.js', ['filterFavorites']],
  ['frontend/js/notes.js', ['filterNotes', 'filterTags']],
];

describe('registerFilter tabs use fzf searchScore / searchMatch', () => {
  for (const [rel, names] of REGISTER_FILTER_TABS) {
    it(`${rel} registers ${names.join(', ')} and calls searchScore or searchMatch`, () => {
      const abs = path.join(repoRoot, rel);
      const src = fs.readFileSync(abs, 'utf8');
      for (const n of names) {
        assert.ok(
          src.includes(`registerFilter('${n}'`),
          `expected registerFilter('${n}') in ${rel}`
        );
      }
      assert.ok(
        /\bsearchScore\s*\(/.test(src) || /\bsearchMatch\s*\(/.test(src),
        `${rel} must call searchScore() or searchMatch() for filtering`
      );
    });
  }
});
