/**
 * Loads real utils.js + plugins.js — plugin card HTML used during scan streaming and KVR refresh.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/plugins.js buildPluginCardHtml (vm-loaded)', () => {
  let P;

  before(() => {
    P = loadFrontendScripts(['utils.js', 'plugins.js'], {
      appFmt: (k) => k,
      _lastPluginSearch: '',
      _lastPluginMode: 'fuzzy',
      rowBadges: () => '',
      KVR_MANUFACTURER_MAP: { 'native-instruments': 'native-instruments' },
    });
  });

  function basePlugin(overrides) {
    return {
      name: 'Test',
      type: 'VST3',
      version: '1.0',
      manufacturer: 'Acme',
      path: '/Plugins/Test.vst3',
      size: '1 MB',
      sizeFormatted: '1 MB',
      modified: '2025-01-01',
      architectures: ['ARM64'],
      ...overrides,
    };
  }

  it('uses type-specific CSS classes for VST2, VST3, AU', () => {
    assert.ok(P.buildPluginCardHtml(basePlugin({ type: 'VST2' })).includes('type-vst2'));
    assert.ok(P.buildPluginCardHtml(basePlugin({ type: 'VST3' })).includes('type-vst3'));
    assert.ok(P.buildPluginCardHtml(basePlugin({ type: 'AU' })).includes('type-au'));
  });

  it('uses type-clap badge class for CLAP plugins', () => {
    const html = P.buildPluginCardHtml(basePlugin({ type: 'CLAP' }));
    assert.ok(html.includes('type-clap'));
    assert.ok(html.includes('>CLAP<'));
  });

  it('embeds data attributes for incremental scan filtering', () => {
    const html = P.buildPluginCardHtml(
      basePlugin({ name: 'MyPlugin', manufacturer: 'Big Co' })
    );
    assert.ok(html.includes('data-plugin-name="myplugin"'));
    assert.ok(html.includes('data-plugin-mfg="big co"'));
    assert.ok(html.includes('data-path='));
  });

  it('renders manufacturer website button when manufacturerUrl is set', () => {
    const html = P.buildPluginCardHtml(
      basePlugin({ manufacturerUrl: 'https://acme.example/plugin' })
    );
    assert.ok(html.includes('data-action="openUpdate"'));
    assert.ok(html.includes('btn-mfg'));
    assert.ok(html.includes('btn-no-web') === false);
  });

  it('renders update + download row when hasUpdate and updateUrl present', () => {
    const html = P.buildPluginCardHtml(
      basePlugin({
        hasUpdate: true,
        currentVersion: '1',
        latestVersion: '2',
        updateUrl: 'https://vendor.example/dl.pkg',
        source: 'kvr',
      })
    );
    assert.ok(html.includes('version-arrow'));
    assert.ok(html.includes('badge-update'));
    assert.ok(html.includes('btn-download'));
  });

  it('shows unknown-latest badge when source is not-found', () => {
    const html = P.buildPluginCardHtml(
      basePlugin({
        hasUpdate: false,
        source: 'not-found',
      })
    );
    assert.ok(html.includes('badge-unknown'));
  });
});
