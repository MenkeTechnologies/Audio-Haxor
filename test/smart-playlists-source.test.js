/**
 * Real smart-playlists.js: rule matching and evaluateSmartPlaylist all/any modes.
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

function loadSpSandbox(overrides = {}) {
  return loadFrontendScripts(['utils.js', 'smart-playlists.js'], {
    showToast: () => {},
    toastFmt: (k, vars) => (vars ? `${k}:${JSON.stringify(vars)}` : k),
    appFmt: (k) => k,
    allAudioSamples: [
      { path: '/kick.wav', name: 'Kick Drum', format: 'WAV', sizeBytes: 512 * 1024 },
      { path: '/line.mp3', name: 'Bass line', format: 'MP3', sizeBytes: 3 * 1024 * 1024 },
      { path: '/hidden.wav', name: 'Other', format: 'WAV', sizeBytes: 100 },
    ],
    recentlyPlayed: [{ path: '/kick.wav' }],
    getNote: (path) => (path === '/kick.wav' ? { tags: ['drums'] } : { tags: [] }),
    isFavorite: (path) => path === '/line.mp3',
    ...overrides,
  });
}

describe('frontend/js/smart-playlists.js (vm-loaded)', () => {
  it('matchesSmartRule format parses comma-separated list case-insensitively', () => {
    const S = loadSpSandbox();
    const wav = { path: '/a', name: 'a', format: 'WAV', sizeBytes: 0 };
    assert.strictEqual(S.matchesSmartRule(wav, { type: 'format', value: 'mp3, wav' }), true);
    assert.strictEqual(S.matchesSmartRule(wav, { type: 'format', value: 'MP3' }), false);
  });

  it('matchesSmartRule name_contains and path_contains are case-insensitive', () => {
    const S = loadSpSandbox();
    const s = { path: '/Samples/KICK.wav', name: 'Foo', format: 'WAV', sizeBytes: 0 };
    assert.strictEqual(S.matchesSmartRule(s, { type: 'name_contains', value: 'FOO' }), true);
    assert.strictEqual(S.matchesSmartRule(s, { type: 'path_contains', value: 'samples' }), true);
  });

  it('matchesSmartRule size_max and size_min use MB in rule value', () => {
    const S = loadSpSandbox();
    const small = { path: '/s', name: 's', format: 'WAV', sizeBytes: 0.5 * 1024 * 1024 };
    const big = { path: '/b', name: 'b', format: 'WAV', sizeBytes: 5 * 1024 * 1024 };
    assert.strictEqual(S.matchesSmartRule(small, { type: 'size_max', value: '1' }), true);
    assert.strictEqual(S.matchesSmartRule(big, { type: 'size_max', value: '1' }), false);
    assert.strictEqual(S.matchesSmartRule(big, { type: 'size_min', value: '1' }), true);
  });

  it('matchesSmartRule tag uses getNote when present', () => {
    const S = loadSpSandbox();
    const kick = S.allAudioSamples[0];
    assert.strictEqual(S.matchesSmartRule(kick, { type: 'tag', value: 'drums' }), true);
    assert.strictEqual(S.matchesSmartRule(kick, { type: 'tag', value: 'vocals' }), false);
  });

  it('matchesSmartRule favorite and recently_played consult stubs', () => {
    const S = loadSpSandbox();
    const fav = S.allAudioSamples[1];
    const recent = S.allAudioSamples[0];
    assert.strictEqual(S.matchesSmartRule(fav, { type: 'favorite', value: '' }), true);
    assert.strictEqual(S.matchesSmartRule(recent, { type: 'recently_played', value: '' }), true);
    assert.strictEqual(S.matchesSmartRule(fav, { type: 'recently_played', value: '' }), false);
  });

  it('evaluateSmartPlaylist requires every rule when matchMode is all', () => {
    const S = loadSpSandbox();
    const pl = {
      matchMode: 'all',
      rules: [
        { type: 'format', value: 'WAV' },
        { type: 'name_contains', value: 'Bass' },
      ],
    };
    const paths = S.evaluateSmartPlaylist(pl).map((x) => x.path).join(',');
    assert.strictEqual(paths, '');
  });

  it('evaluateSmartPlaylist matches any rule when matchMode is any', () => {
    const S = loadSpSandbox();
    const pl = {
      matchMode: 'any',
      rules: [
        { type: 'name_contains', value: 'Kick' },
        { type: 'name_contains', value: 'Snare' },
      ],
    };
    const paths = S.evaluateSmartPlaylist(pl).map((x) => x.path).join(',');
    assert.strictEqual(paths, '/kick.wav');
  });

  it('evaluateSmartPlaylist returns empty when rules array is empty', () => {
    const S = loadSpSandbox();
    assert.strictEqual(S.evaluateSmartPlaylist({ rules: [], matchMode: 'all' }).length, 0);
  });
});
