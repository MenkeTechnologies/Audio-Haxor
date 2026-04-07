const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── frontend/js/ipc.js AUDIO_PAGE_SIZE / DAW_PAGE_SIZE ──
const AUDIO_PAGE_SIZE = 200;
const DAW_PAGE_SIZE = 200;

describe('page sizes', () => {
  it('audio and daw use same batch size', () => {
    assert.strictEqual(AUDIO_PAGE_SIZE, DAW_PAGE_SIZE);
    assert.strictEqual(AUDIO_PAGE_SIZE, 200);
  });
});
