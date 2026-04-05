/**
 * Loads ipc.js with Tauri stubs — verifies reloadAppStrings maps locales to backend
 * and forwards null for unsupported codes (seeded locales pass through).
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

function loadIpcSandbox(invokeImpl) {
  const calls = [];
  const sandbox = {
    console,
    document: {
      getElementById: () => null,
      addEventListener: () => {},
    },
    addEventListener: () => {},
    window: {},
    __TAURI__: {
      core: {
        invoke: async (cmd, args) => {
          calls.push({ cmd, args: args ? { ...args } : args });
          return invokeImpl(cmd, args);
        },
        convertFileSrc: (s) => s,
      },
      event: {
        listen: () => () => {},
      },
    },
  };
  sandbox.window = sandbox;
  sandbox.vstUpdater = { appendLog: () => {} };
  vm.createContext(sandbox);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, '..', 'frontend', 'js', 'ipc.js'), 'utf8'),
    sandbox
  );
  return { sandbox, calls };
}

describe('frontend/js/ipc.js (vm-loaded)', () => {
  it('reloadAppStrings passes through supported locale codes to get_app_strings', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings('de');
    const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.ok(getStr.length >= 1);
    assert.strictEqual(getStr[getStr.length - 1].args.locale, 'de');
  });

  it('reloadAppStrings passes it (seed locale)', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings('it');
    const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, 'it');
  });

  it('reloadAppStrings sends null for arbitrary garbage locale', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings('xx-YY');
    const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, null);
  });

  it('reloadAppStrings passes en and fr (seed locales)', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings('en');
    let getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, 'en');
    await sandbox.reloadAppStrings('fr');
    getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, 'fr');
  });

  const supportedLocales = ['es', 'sv', 'pt', 'nl', 'pl', 'ru', 'el', 'zh', 'ja', 'ko', 'fi'];

  for (const loc of supportedLocales) {
    it(`reloadAppStrings passes ${loc} to get_app_strings`, async () => {
      const { sandbox, calls } = loadIpcSandbox(async () => ({}));
      await sandbox.reloadAppStrings(loc);
      const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
      assert.strictEqual(getStr[getStr.length - 1].args.locale, loc);
    });
  }

  it('reloadAppStrings passes null for undefined locale', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings(undefined);
    const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, null);
  });

  it('reloadAppStrings passes null for empty string locale', async () => {
    const { sandbox, calls } = loadIpcSandbox(async () => ({}));
    await sandbox.reloadAppStrings('');
    const getStr = calls.filter((c) => c.cmd === 'get_app_strings');
    assert.strictEqual(getStr[getStr.length - 1].args.locale, null);
  });
});
