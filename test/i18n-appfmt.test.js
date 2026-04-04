/**
 * Contract tests for `appFmt` / `toastFmt` in `frontend/js/ipc.js`.
 * Logic is duplicated here (no Tauri / no browser) so `node --test` can pin behavior.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

/** Same semantics as `ipc.js` `appFmt` — map is `window.__appStr` at runtime. */
function appFmt(map, key, vars) {
  let s = map && map[key];
  if (s == null || s === '') return key;
  if (vars && typeof vars === 'object') {
    s = s.replace(/\{(\w+)\}/g, (_, name) =>
      vars[name] != null && vars[name] !== '' ? String(vars[name]) : ''
    );
  }
  return s;
}

test('appFmt returns key when string missing from map', () => {
  assert.equal(appFmt({}, 'menu.scan_all'), 'menu.scan_all');
  assert.equal(appFmt({ foo: 'bar' }, 'menu.scan_all'), 'menu.scan_all');
});

test('appFmt returns key when value is empty string', () => {
  assert.equal(appFmt({ k: '' }, 'k'), 'k');
});

test('appFmt substitutes single placeholder', () => {
  const map = { 'menu.batch_selected': '{n} selected' };
  assert.equal(appFmt(map, 'menu.batch_selected', { n: 3 }), '3 selected');
});

test('appFmt substitutes multiple placeholders', () => {
  const map = { t: '{a} and {b}' };
  assert.equal(appFmt(map, 't', { a: 'x', b: 'y' }), 'x and y');
});

test('appFmt treats null or undefined var as empty segment', () => {
  const map = { t: 'x{n}y' };
  assert.equal(appFmt(map, 't', { n: null }), 'xy');
  assert.equal(appFmt(map, 't', {}), 'xy');
});

test('appFmt treats empty string var as empty segment', () => {
  const map = { t: '>{name}<' };
  assert.equal(appFmt(map, 't', { name: '' }), '><');
});

test('appFmt coerces numbers to string', () => {
  const map = { t: 'n={n}' };
  assert.equal(appFmt(map, 't', { n: 0 }), 'n=0');
});

test('appFmt does not replace brace segments that are not {word}', () => {
  const map = { t: 'x {not-a-token} y' };
  assert.equal(appFmt(map, 't', { not: 1 }), 'x {not-a-token} y');
});
