const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

function midiToName(n, octaveOffset = 0) {
  const name = NAMES[((n % 12) + 12) % 12];
  const oct = Math.floor(n / 12) - 1 + octaveOffset;
  return name + oct;
}

describe('midiToName', () => {
  it('middle C', () => assert.strictEqual(midiToName(60), 'C4'));

  it('chromatic one octave starting at C4', () => {
    const expected = ['C4', 'C#4', 'D4', 'D#4', 'E4', 'F4', 'F#4', 'G4', 'G#4', 'A4', 'A#4', 'B4'];
    for (let i = 0; i < 12; i++) {
      assert.strictEqual(midiToName(60 + i), expected[i]);
    }
  });

  it('negative MIDI note wraps pitch class', () => {
    assert.strictEqual(midiToName(-1), 'B-2');
    assert.strictEqual(midiToName(-12), 'C-2');
  });

  it('octave offset shifts label', () => {
    assert.strictEqual(midiToName(60, 1), 'C5');
    assert.strictEqual(midiToName(60, -1), 'C3');
  });

  it('A440', () => assert.strictEqual(midiToName(69), 'A4'));

  it('lowest common MIDI 0', () => assert.strictEqual(midiToName(0), 'C-1'));

  it('highest MIDI 127 is G9', () => assert.strictEqual(midiToName(127), 'G9'));

  it('MIDI 128 wraps to G#9 (same pitch class as 0 mod 12)', () => {
    assert.strictEqual(midiToName(128), 'G#9');
    assert.strictEqual(midiToName(128) === midiToName(0), false);
  });

  it('G5 and F5 in same octave range', () => {
    assert.strictEqual(midiToName(79), 'G5');
    assert.strictEqual(midiToName(77), 'F5');
  });
});
