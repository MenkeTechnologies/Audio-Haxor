const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── Shared utility functions (replicated from frontend/index.html) ──
// These live at module scope so multiple describe blocks can reference them.

function escapeHtml(str) {
  return (str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function escapePath(str) {
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
}

function slugify(str) {
  return str
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .replace(/([a-zA-Z])(\d)/g, '$1-$2')
    .replace(/(\d)([a-zA-Z])/g, '$1-$2')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

const KVR_MANUFACTURER_MAP = {
  'madronalabs': 'madrona-labs',
  'audiothing': 'audio-thing',
  'audiodamage': 'audio-damage',
  'soundtoys': 'soundtoys',
  'native-instruments': 'native-instruments',
  'plugin-alliance': 'plugin-alliance',
  'softube': 'softube',
  'izotope': 'izotope',
  'eventide': 'eventide',
  'arturia': 'arturia',
  'u-he': 'u-he',
};

function buildKvrUrl(name, manufacturer) {
  const nameSlug = slugify(name);
  if (manufacturer && manufacturer !== 'Unknown') {
    const mfgLower = manufacturer.toLowerCase().replace(/[^a-z0-9]+/g, '');
    const mfgSlug = KVR_MANUFACTURER_MAP[mfgLower] || slugify(manufacturer);
    return `https://www.kvraudio.com/product/${nameSlug}-by-${mfgSlug}`;
  }
  return `https://www.kvraudio.com/product/${nameSlug}`;
}

function kvrCacheKey(plugin) {
  return `${(plugin.manufacturer || 'Unknown').toLowerCase()}|||${plugin.name.toLowerCase()}`;
}

function buildPluginCardHtml(p) {
  const typeClass = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : 'type-au';
  let versionHtml = `<span class="version-current">v${p.version}</span>`;
  let badgeHtml = '';
  const mfgUrl = p.manufacturerUrl || null;
  const mfgBtn = mfgUrl
    ? `<button class="btn-small btn-mfg" data-action="openUpdate" data-url="${mfgUrl}" title="${mfgUrl}">&#127760;</button>`
    : `<button class="btn-small btn-no-web" disabled title="No manufacturer website">&#128683;</button>`;
  const kvrUrl = p.kvrUrl || buildKvrUrl(p.name, p.manufacturer);
  const kvrBtn = `<button class="btn-small btn-kvr" data-action="openKvr" data-url="${kvrUrl.replace(/'/g, '&apos;')}" data-name="${escapePath(p.name)}" title="${escapeHtml(kvrUrl)}">KVR</button>`;
  const dlUrl = (p.hasUpdate && p.updateUrl) ? p.updateUrl : null;
  const dlBtn = dlUrl
    ? `<button class="btn-small btn-download btn-dl-kvr" data-action="openUpdate" data-url="${dlUrl.replace(/'/g, '&apos;')}" title="${escapeHtml(dlUrl)}">&#11015; Download</button>`
    : '';
  let actionsHtml = dlBtn + kvrBtn + mfgBtn + `<button class="btn-small btn-folder" data-action="openFolder" data-path="${escapePath(p.path)}" title="${escapePath(p.path)}">&#128193;</button>`;

  if (p.hasUpdate !== undefined) {
    if (p.hasUpdate) {
      versionHtml = `<span class="version-current">v${p.currentVersion}</span>
            <span class="version-arrow">&#8594;</span>
            <span class="version-latest">v${p.latestVersion}</span>`;
      badgeHtml = '<span class="badge badge-update">Update Available</span>';
    } else if (p.source === 'not-found') {
      badgeHtml = '<span class="badge badge-unknown">Unknown Latest</span>';
    } else {
      badgeHtml = '<span class="badge badge-current">Up to Date</span>';
    }
  }

  return `
        <div class="plugin-card" data-path="${escapePath(p.path)}">
          <div class="plugin-info">
            <h3>${escapeHtml(p.name)}</h3>
            <div class="plugin-meta">
              <span class="plugin-type ${typeClass}">${p.type}</span>
              <span>${escapeHtml(p.manufacturer)}</span>
              <span>${p.size}</span>
              <span>${p.modified}</span>
            </div>
          </div>
          <div class="plugin-version">${versionHtml}</div>
          ${badgeHtml}
          <div class="plugin-actions">${actionsHtml}</div>
        </div>`;
}

function buildDirsTable(directories, plugins) {
  if (!directories || directories.length === 0) return '';
  const rows = directories.map(dir => {
    const count = plugins.filter(p => p.path.startsWith(dir + '/')).length;
    const types = {};
    plugins.filter(p => p.path.startsWith(dir + '/')).forEach(p => {
      types[p.type] = (types[p.type] || 0) + 1;
    });
    const typeStr = Object.entries(types)
      .map(([t, c]) => `<span class="plugin-type ${t === 'VST2' ? 'type-vst2' : t === 'VST3' ? 'type-vst3' : 'type-au'}">${t}: ${c}</span>`)
      .join(' ');
    return `<tr>
          <td style="padding: 4px 8px 4px 0; color: var(--cyan); opacity: 0.7;">${dir}</td>
          <td style="padding: 4px 8px; text-align: right; font-family: Orbitron, sans-serif; color: var(--text);">${count}</td>
          <td style="padding: 4px 0 4px 8px;">${typeStr}</td>
        </tr>`;
  });
  return `<table style="width: 100%; border-collapse: collapse; margin-top: 6px;">
        <tr style="color: var(--text-muted); font-size: 10px; text-transform: uppercase; letter-spacing: 1px;">
          <th style="text-align: left; padding: 2px 8px 2px 0;">Directory</th>
          <th style="text-align: right; padding: 2px 8px;">Plugins</th>
          <th style="text-align: left; padding: 2px 0 2px 8px;">Types</th>
        </tr>
        ${rows.join('')}
      </table>`;
}

function applyKvrCache(plugins, cache) {
  for (const p of plugins) {
    const cached = cache[kvrCacheKey(p)];
    if (cached) {
      p.kvrUrl = cached.kvrUrl || p.kvrUrl;
      p.source = cached.source || p.source;
      if (cached.latestVersion && cached.latestVersion !== p.version) {
        p.latestVersion = cached.latestVersion;
        p.currentVersion = p.version;
        p.hasUpdate = cached.hasUpdate || false;
      }
      if (cached.updateUrl && p.hasUpdate) {
        p.updateUrl = cached.updateUrl;
      }
    }
  }
}

function metaItem(label, value) {
  return `<div class="meta-item"><span class="meta-label">${label}</span><span class="meta-value">${escapeHtml(String(value || '\u2014'))}</span></div>`;
}

// ── Tests ──

describe('escapeHtml', () => {
  it('escapes ampersand', () => {
    assert.strictEqual(escapeHtml('a & b'), 'a &amp; b');
  });

  it('escapes angle brackets', () => {
    assert.strictEqual(escapeHtml('<script>'), '&lt;script&gt;');
  });

  it('escapes double quotes', () => {
    assert.strictEqual(escapeHtml('"hello"'), '&quot;hello&quot;');
  });

  it('escapes single quotes', () => {
    assert.strictEqual(escapeHtml("it's"), "it&#039;s");
  });

  it('handles null/undefined', () => {
    assert.strictEqual(escapeHtml(null), '');
    assert.strictEqual(escapeHtml(undefined), '');
  });

  it('returns empty string for empty input', () => {
    assert.strictEqual(escapeHtml(''), '');
  });

  it('escapes multiple special chars together', () => {
    assert.strictEqual(escapeHtml('<a href="x">&'), '&lt;a href=&quot;x&quot;&gt;&amp;');
  });

  it('handles numbers (coerced to string)', () => {
    // escapeHtml receives (str || '') so a number is truthy and toString is called via replace
    assert.strictEqual(escapeHtml(String(42)), '42');
  });

  it('does not double-escape existing entities', () => {
    assert.strictEqual(escapeHtml('&amp;'), '&amp;amp;');
  });
});

describe('escapePath', () => {
  it('escapes backslashes', () => {
    assert.strictEqual(escapePath('C:\\Users\\test'), 'C:\\\\Users\\\\test');
  });

  it('escapes single quotes', () => {
    assert.strictEqual(escapePath("it's a path"), "it\\'s a path");
  });

  it('escapes both backslashes and quotes', () => {
    assert.strictEqual(escapePath("C:\\it's"), "C:\\\\it\\'s");
  });

  it('leaves normal paths unchanged', () => {
    assert.strictEqual(escapePath('/usr/local/bin'), '/usr/local/bin');
  });
});

describe('slugify', () => {
  it('lowercases and hyphenates spaces', () => {
    assert.strictEqual(slugify('Hello World'), 'hello-world');
  });

  it('splits camelCase', () => {
    assert.strictEqual(slugify('MadronaLabs'), 'madrona-labs');
  });

  it('splits letters and digits', () => {
    assert.strictEqual(slugify('Plugin3'), 'plugin-3');
    assert.strictEqual(slugify('3rdParty'), '3-rd-party');
  });

  it('removes special characters', () => {
    assert.strictEqual(slugify('foo@bar!baz'), 'foo-bar-baz');
  });

  it('trims leading/trailing hyphens', () => {
    assert.strictEqual(slugify('--hello--'), 'hello');
  });

  it('collapses multiple separators', () => {
    assert.strictEqual(slugify('a   b   c'), 'a-b-c');
  });

  it('empty string returns empty', () => {
    assert.strictEqual(slugify(''), '');
  });

  it('all special chars returns empty', () => {
    assert.strictEqual(slugify('!@#$%^&*()'), '');
  });

  it('numbers are preserved', () => {
    assert.strictEqual(slugify('12345'), '12345');
  });
});

describe('buildKvrUrl', () => {
  it('builds URL without manufacturer', () => {
    assert.strictEqual(
      buildKvrUrl('Serum', null),
      'https://www.kvraudio.com/product/serum'
    );
  });

  it('builds URL with Unknown manufacturer', () => {
    assert.strictEqual(
      buildKvrUrl('Serum', 'Unknown'),
      'https://www.kvraudio.com/product/serum'
    );
  });

  it('builds URL with manufacturer', () => {
    assert.strictEqual(
      buildKvrUrl('Serum', 'Xfer Records'),
      'https://www.kvraudio.com/product/serum-by-xfer-records'
    );
  });

  it('uses KVR_MANUFACTURER_MAP for known manufacturers', () => {
    assert.strictEqual(
      buildKvrUrl('Aalto', 'MadronaLabs'),
      'https://www.kvraudio.com/product/aalto-by-madrona-labs'
    );
  });

  it('uses KVR_MANUFACTURER_MAP for AudioThing', () => {
    assert.strictEqual(
      buildKvrUrl('FogConvolver', 'AudioThing'),
      'https://www.kvraudio.com/product/fog-convolver-by-audio-thing'
    );
  });

  it('slugifies special chars in name', () => {
    assert.strictEqual(
      buildKvrUrl('My Plugin!', 'SomeCompany'),
      'https://www.kvraudio.com/product/my-plugin-by-some-company'
    );
  });

  it('handles empty name', () => {
    assert.strictEqual(
      buildKvrUrl('', 'SomeCompany'),
      'https://www.kvraudio.com/product/-by-some-company'
    );
  });

  it('excludes manufacturer when it is "Unknown"', () => {
    assert.strictEqual(
      buildKvrUrl('Reverb', 'Unknown'),
      'https://www.kvraudio.com/product/reverb'
    );
  });
});

describe('formatAudioSize', () => {
  function formatAudioSize(bytes) {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
  }

  it('formats 0 bytes', () => {
    assert.strictEqual(formatAudioSize(0), '0 B');
  });

  it('formats bytes', () => {
    assert.strictEqual(formatAudioSize(500), '500.0 B');
  });

  it('formats kilobytes', () => {
    assert.strictEqual(formatAudioSize(1024), '1.0 KB');
  });

  it('formats megabytes', () => {
    assert.strictEqual(formatAudioSize(1048576), '1.0 MB');
  });

  it('formats gigabytes', () => {
    assert.strictEqual(formatAudioSize(1073741824), '1.0 GB');
  });

  it('formats terabytes', () => {
    assert.strictEqual(formatAudioSize(1099511627776), '1.0 TB');
  });

  it('formats fractional values', () => {
    assert.strictEqual(formatAudioSize(1536), '1.5 KB');
  });
});

describe('formatTime', () => {
  function formatTime(sec) {
    if (!sec || !isFinite(sec)) return '0:00';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return m + ':' + String(s).padStart(2, '0');
  }

  it('returns 0:00 for 0', () => {
    assert.strictEqual(formatTime(0), '0:00');
  });

  it('returns 0:00 for NaN', () => {
    assert.strictEqual(formatTime(NaN), '0:00');
  });

  it('returns 0:00 for Infinity', () => {
    assert.strictEqual(formatTime(Infinity), '0:00');
  });

  it('returns 0:00 for null', () => {
    assert.strictEqual(formatTime(null), '0:00');
  });

  it('formats seconds only', () => {
    assert.strictEqual(formatTime(5), '0:05');
    assert.strictEqual(formatTime(45), '0:45');
  });

  it('formats minutes and seconds', () => {
    assert.strictEqual(formatTime(65), '1:05');
    assert.strictEqual(formatTime(130), '2:10');
  });

  it('formats hours worth of seconds', () => {
    assert.strictEqual(formatTime(3661), '61:01');
  });

  it('floors fractional seconds', () => {
    assert.strictEqual(formatTime(5.7), '0:05');
  });
});

describe('getFormatClass', () => {
  function getFormatClass(format) {
    const f = format.toLowerCase();
    if (['wav', 'mp3', 'aiff', 'aif', 'flac', 'ogg', 'm4a', 'aac'].includes(f)) return 'format-' + f;
    return 'format-default';
  }

  it('returns format-wav for WAV', () => {
    assert.strictEqual(getFormatClass('WAV'), 'format-wav');
  });

  it('returns format-mp3 for MP3', () => {
    assert.strictEqual(getFormatClass('MP3'), 'format-mp3');
  });

  it('returns format-flac for flac', () => {
    assert.strictEqual(getFormatClass('flac'), 'format-flac');
  });

  it('returns format-aiff for AIFF', () => {
    assert.strictEqual(getFormatClass('AIFF'), 'format-aiff');
  });

  it('returns format-aif for aif', () => {
    assert.strictEqual(getFormatClass('aif'), 'format-aif');
  });

  it('returns format-ogg for ogg', () => {
    assert.strictEqual(getFormatClass('ogg'), 'format-ogg');
  });

  it('returns format-m4a for m4a', () => {
    assert.strictEqual(getFormatClass('m4a'), 'format-m4a');
  });

  it('returns format-aac for aac', () => {
    assert.strictEqual(getFormatClass('aac'), 'format-aac');
  });

  it('returns format-default for unknown format', () => {
    assert.strictEqual(getFormatClass('wma'), 'format-default');
    assert.strictEqual(getFormatClass('opus'), 'format-default');
  });
});

describe('timeAgo', () => {
  function timeAgo(date) {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
    if (seconds < 60) return 'just now';
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    const months = Math.floor(days / 30);
    return `${months}mo ago`;
  }

  it('returns just now for recent dates', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 10 * 1000)), 'just now');
  });

  it('returns minutes ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 5 * 60 * 1000)), '5m ago');
  });

  it('returns hours ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 3 * 60 * 60 * 1000)), '3h ago');
  });

  it('returns days ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 7 * 24 * 60 * 60 * 1000)), '7d ago');
  });

  it('returns months ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 60 * 24 * 60 * 60 * 1000)), '2mo ago');
  });

  it('boundary: 59 seconds is just now', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 59 * 1000)), 'just now');
  });

  it('boundary: 60 seconds is 1m ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 60 * 1000)), '1m ago');
  });

  it('exactly 1 minute ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 60 * 1000)), '1m ago');
  });

  it('exactly 1 hour ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 60 * 60 * 1000)), '1h ago');
  });

  it('exactly 1 day ago', () => {
    assert.strictEqual(timeAgo(new Date(Date.now() - 24 * 60 * 60 * 1000)), '1d ago');
  });

  it('future date returns just now', () => {
    // seconds will be negative, which is < 60
    assert.strictEqual(timeAgo(new Date(Date.now() + 60 * 60 * 1000)), 'just now');
  });
});

describe('kvrCacheKey', () => {
  it('builds key from manufacturer and name', () => {
    assert.strictEqual(
      kvrCacheKey({ manufacturer: 'Xfer Records', name: 'Serum' }),
      'xfer records|||serum'
    );
  });

  it('defaults manufacturer to Unknown', () => {
    assert.strictEqual(
      kvrCacheKey({ name: 'Serum' }),
      'unknown|||serum'
    );
  });

  it('handles null manufacturer', () => {
    assert.strictEqual(
      kvrCacheKey({ manufacturer: null, name: 'Serum' }),
      'unknown|||serum'
    );
  });

  it('lowercases both parts', () => {
    assert.strictEqual(
      kvrCacheKey({ manufacturer: 'NATIVE INSTRUMENTS', name: 'MASSIVE' }),
      'native instruments|||massive'
    );
  });
});

// ── New describe blocks ──

describe('buildDirsTable', () => {
  it('empty directories returns empty string', () => {
    assert.strictEqual(buildDirsTable([], []), '');
  });

  it('null directories returns empty string', () => {
    assert.strictEqual(buildDirsTable(null, []), '');
  });

  it('counts plugins per directory correctly', () => {
    const dirs = ['/usr/lib/vst'];
    const plugins = [
      { path: '/usr/lib/vst/PluginA.vst', type: 'VST2' },
      { path: '/usr/lib/vst/PluginB.vst', type: 'VST2' },
    ];
    const html = buildDirsTable(dirs, plugins);
    // The count cell should contain 2
    assert.ok(html.includes('>2</td>'));
  });

  it('groups plugin types (VST2, VST3, AU) per directory', () => {
    const dirs = ['/usr/lib/vst'];
    const plugins = [
      { path: '/usr/lib/vst/A.vst', type: 'VST2' },
      { path: '/usr/lib/vst/B.vst3', type: 'VST3' },
      { path: '/usr/lib/vst/C.component', type: 'AU' },
    ];
    const html = buildDirsTable(dirs, plugins);
    assert.ok(html.includes('type-vst2'));
    assert.ok(html.includes('VST2: 1'));
    assert.ok(html.includes('type-vst3'));
    assert.ok(html.includes('VST3: 1'));
    assert.ok(html.includes('type-au'));
    assert.ok(html.includes('AU: 1'));
  });

  it('handles directory with zero matching plugins', () => {
    const dirs = ['/empty/dir'];
    const plugins = [
      { path: '/other/dir/A.vst', type: 'VST2' },
    ];
    const html = buildDirsTable(dirs, plugins);
    assert.ok(html.includes('>0</td>'));
  });
});

describe('applyKvrCache', () => {
  it('applies cached kvrUrl to plugin', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer Records', version: '1.0' }];
    const cache = { 'xfer records|||serum': { kvrUrl: 'https://kvr.example.com/serum' } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].kvrUrl, 'https://kvr.example.com/serum');
  });

  it('applies latestVersion and sets hasUpdate', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0' }];
    const cache = { 'xfer|||serum': { latestVersion: '2.0', hasUpdate: true } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].latestVersion, '2.0');
    assert.strictEqual(plugins[0].hasUpdate, true);
  });

  it('sets currentVersion when update available', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0' }];
    const cache = { 'xfer|||serum': { latestVersion: '2.0', hasUpdate: true } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].currentVersion, '1.0');
  });

  it('applies updateUrl only when hasUpdate is true', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0' }];
    const cache = { 'xfer|||serum': { latestVersion: '2.0', hasUpdate: true, updateUrl: 'https://dl.example.com' } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].updateUrl, 'https://dl.example.com');

    // When hasUpdate is false, updateUrl should not be applied
    const plugins2 = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0' }];
    const cache2 = { 'xfer|||serum': { latestVersion: '2.0', hasUpdate: false, updateUrl: 'https://dl.example.com' } };
    applyKvrCache(plugins2, cache2);
    assert.strictEqual(plugins2[0].updateUrl, undefined);
  });

  it('skips plugins not in cache', () => {
    const plugins = [{ name: 'Vital', manufacturer: 'Matt Tytel', version: '1.0' }];
    const cache = { 'xfer|||serum': { kvrUrl: 'https://kvr.example.com/serum' } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].kvrUrl, undefined);
  });

  it('does not overwrite existing kvrUrl with undefined', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0', kvrUrl: 'https://existing.com' }];
    const cache = { 'xfer|||serum': { kvrUrl: undefined } };
    applyKvrCache(plugins, cache);
    // cached.kvrUrl is undefined so (undefined || p.kvrUrl) keeps existing
    assert.strictEqual(plugins[0].kvrUrl, 'https://existing.com');
  });

  it('handles empty cache', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', version: '1.0' }];
    applyKvrCache(plugins, {});
    assert.strictEqual(plugins[0].kvrUrl, undefined);
    assert.strictEqual(plugins[0].hasUpdate, undefined);
  });
});

describe('metaItem', () => {
  it('renders label and value', () => {
    const html = metaItem('Sample Rate', '44100 Hz');
    assert.ok(html.includes('Sample Rate'));
    assert.ok(html.includes('44100 Hz'));
    assert.ok(html.includes('meta-label'));
    assert.ok(html.includes('meta-value'));
  });

  it('handles null value (shows \u2014)', () => {
    const html = metaItem('Codec', null);
    assert.ok(html.includes('\u2014'));
  });

  it('handles undefined value', () => {
    const html = metaItem('Codec', undefined);
    assert.ok(html.includes('\u2014'));
  });

  it('escapes HTML in value', () => {
    const html = metaItem('Tag', '<script>alert("xss")</script>');
    assert.ok(html.includes('&lt;script&gt;'));
    assert.ok(!html.includes('<script>'));
  });
});

describe('buildPluginCardHtml', () => {
  function makePlugin(overrides) {
    return {
      name: 'TestPlugin',
      type: 'VST3',
      manufacturer: 'TestCo',
      version: '1.0.0',
      path: '/usr/lib/vst3/TestPlugin.vst3',
      size: '2.5 MB',
      modified: '2025-01-01',
      ...overrides,
    };
  }

  it('renders basic plugin card with name, type, manufacturer', () => {
    const html = buildPluginCardHtml(makePlugin());
    assert.ok(html.includes('TestPlugin'));
    assert.ok(html.includes('VST3'));
    assert.ok(html.includes('TestCo'));
    assert.ok(html.includes('plugin-card'));
  });

  it('shows correct type class for VST2', () => {
    const html = buildPluginCardHtml(makePlugin({ type: 'VST2' }));
    assert.ok(html.includes('type-vst2'));
  });

  it('shows correct type class for VST3', () => {
    const html = buildPluginCardHtml(makePlugin({ type: 'VST3' }));
    assert.ok(html.includes('type-vst3'));
  });

  it('shows correct type class for AU', () => {
    const html = buildPluginCardHtml(makePlugin({ type: 'AU' }));
    assert.ok(html.includes('type-au'));
  });

  it('shows update badge when hasUpdate is true', () => {
    const html = buildPluginCardHtml(makePlugin({
      hasUpdate: true,
      currentVersion: '1.0.0',
      latestVersion: '2.0.0',
    }));
    assert.ok(html.includes('badge-update'));
    assert.ok(html.includes('Update Available'));
    assert.ok(html.includes('v1.0.0'));
    assert.ok(html.includes('v2.0.0'));
  });

  it('shows "Up to Date" badge when hasUpdate is false', () => {
    const html = buildPluginCardHtml(makePlugin({ hasUpdate: false }));
    assert.ok(html.includes('badge-current'));
    assert.ok(html.includes('Up to Date'));
  });

  it('shows "Unknown Latest" badge when source is "not-found"', () => {
    const html = buildPluginCardHtml(makePlugin({ hasUpdate: false, source: 'not-found' }));
    assert.ok(html.includes('badge-unknown'));
    assert.ok(html.includes('Unknown Latest'));
  });

  it('shows download button only when hasUpdate and updateUrl present', () => {
    const withUpdate = buildPluginCardHtml(makePlugin({
      hasUpdate: true,
      currentVersion: '1.0.0',
      latestVersion: '2.0.0',
      updateUrl: 'https://download.example.com/update',
    }));
    assert.ok(withUpdate.includes('btn-download'));
    assert.ok(withUpdate.includes('Download'));

    const withoutUpdate = buildPluginCardHtml(makePlugin({ hasUpdate: false }));
    assert.ok(!withoutUpdate.includes('btn-download'));
  });

  it('shows disabled manufacturer button when no manufacturerUrl', () => {
    const html = buildPluginCardHtml(makePlugin());
    assert.ok(html.includes('btn-no-web'));
    assert.ok(html.includes('disabled'));
  });

  it('shows active manufacturer button when manufacturerUrl present', () => {
    const html = buildPluginCardHtml(makePlugin({ manufacturerUrl: 'https://testco.com' }));
    assert.ok(html.includes('btn-mfg'));
    assert.ok(html.includes('https://testco.com'));
    assert.ok(!html.includes('btn-no-web'));
  });

  it('escapes HTML in plugin name', () => {
    const html = buildPluginCardHtml(makePlugin({ name: '<b>Bold</b>' }));
    // The <h3> should contain the escaped version
    assert.ok(html.includes('&lt;b&gt;Bold&lt;/b&gt;'));
  });
});

// ── CSV escape helper (mirrors backend csv_escape) ──

describe('csvEscape', () => {
  function csvEscape(s) {
    if (s.includes(',') || s.includes('"') || s.includes('\n')) {
      return '"' + s.replace(/"/g, '""') + '"';
    }
    return s;
  }

  it('returns plain string unchanged', () => {
    assert.strictEqual(csvEscape('hello'), 'hello');
  });

  it('wraps string with comma in quotes', () => {
    assert.strictEqual(csvEscape('a,b'), '"a,b"');
  });

  it('escapes double quotes inside', () => {
    assert.strictEqual(csvEscape('say "hi"'), '"say ""hi"""');
  });

  it('wraps string with newline', () => {
    assert.strictEqual(csvEscape('line1\nline2'), '"line1\nline2"');
  });

  it('handles empty string', () => {
    assert.strictEqual(csvEscape(''), '');
  });

  it('handles comma and quotes together', () => {
    assert.strictEqual(csvEscape('a,"b"'), '"a,""b"""');
  });
});

// ── Plugin filtering logic ──

describe('filterPlugins logic', () => {
  function filterPlugins(plugins, search, typeFilter, statusFilter) {
    return plugins.filter(p => {
      const matchesSearch = p.name.toLowerCase().includes(search) ||
        (p.manufacturer && p.manufacturer.toLowerCase().includes(search));
      const matchesType = typeFilter === 'all' || p.type === typeFilter;
      let matchesStatus = true;
      if (statusFilter === 'update') matchesStatus = p.hasUpdate === true;
      if (statusFilter === 'current') matchesStatus = p.hasUpdate === false && p.source !== 'not-found';
      if (statusFilter === 'unknown') matchesStatus = !p.hasUpdate && p.source === 'not-found';
      return matchesSearch && matchesType && matchesStatus;
    });
  }

  const plugins = [
    { name: 'Serum', type: 'VST3', manufacturer: 'Xfer', hasUpdate: true, source: 'kvr' },
    { name: 'Massive', type: 'VST2', manufacturer: 'Native Instruments', hasUpdate: false, source: 'kvr' },
    { name: 'Diva', type: 'VST3', manufacturer: 'u-he', hasUpdate: false, source: 'not-found' },
    { name: 'Compressor', type: 'AU', manufacturer: 'Apple', hasUpdate: false, source: 'kvr' },
  ];

  it('returns all plugins with no filters', () => {
    assert.strictEqual(filterPlugins(plugins, '', 'all', 'all').length, 4);
  });

  it('filters by search term on name', () => {
    const result = filterPlugins(plugins, 'serum', 'all', 'all');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Serum');
  });

  it('filters by search term on manufacturer', () => {
    const result = filterPlugins(plugins, 'native', 'all', 'all');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Massive');
  });

  it('filters by type VST3', () => {
    const result = filterPlugins(plugins, '', 'VST3', 'all');
    assert.strictEqual(result.length, 2);
  });

  it('filters by type AU', () => {
    const result = filterPlugins(plugins, '', 'AU', 'all');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Compressor');
  });

  it('filters by status update', () => {
    const result = filterPlugins(plugins, '', 'all', 'update');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Serum');
  });

  it('filters by status current (up to date)', () => {
    const result = filterPlugins(plugins, '', 'all', 'current');
    assert.strictEqual(result.length, 2);
    assert.ok(result.some(p => p.name === 'Massive'));
    assert.ok(result.some(p => p.name === 'Compressor'));
  });

  it('filters by status unknown', () => {
    const result = filterPlugins(plugins, '', 'all', 'unknown');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Diva');
  });

  it('combines search and type filter', () => {
    const result = filterPlugins(plugins, 'u-he', 'VST3', 'all');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0].name, 'Diva');
  });

  it('combines all filters returning empty', () => {
    const result = filterPlugins(plugins, 'serum', 'AU', 'all');
    assert.strictEqual(result.length, 0);
  });

  it('search is case insensitive (caller lowercases)', () => {
    const result = filterPlugins(plugins, 'SERUM'.toLowerCase(), 'all', 'all');
    assert.strictEqual(result.length, 1);
  });
});

// ── escapePath edge cases ──

describe('escapePath edge cases', () => {
  it('handles consecutive backslashes', () => {
    assert.strictEqual(escapePath('C:\\\\share'), 'C:\\\\\\\\share');
  });

  it('handles path with both backslash and quote', () => {
    assert.strictEqual(escapePath("C:\\it's\\file"), "C:\\\\it\\'s\\\\file");
  });

  it('handles empty string', () => {
    assert.strictEqual(escapePath(''), '');
  });
});

// ── slugify edge cases ──

describe('slugify edge cases', () => {
  it('handles consecutive uppercase letters (no split within run)', () => {
    // The regex only splits lowercase→uppercase, so XML stays together
    assert.strictEqual(slugify('XMLParser'), 'xmlparser');
  });

  it('handles mixed case with numbers', () => {
    assert.strictEqual(slugify('Pro3Editor'), 'pro-3-editor');
  });

  it('handles single character', () => {
    assert.strictEqual(slugify('X'), 'x');
  });

  it('handles unicode by stripping', () => {
    assert.strictEqual(slugify('café'), 'caf');
  });
});

// ── buildKvrUrl edge cases ──

describe('buildKvrUrl edge cases', () => {
  it('handles manufacturer with all special chars', () => {
    const url = buildKvrUrl('Test', '!!!');
    assert.strictEqual(url, 'https://www.kvraudio.com/product/test-by-');
  });

  it('handles camelCase plugin name', () => {
    const url = buildKvrUrl('ProChannel', 'SomeCo');
    assert.strictEqual(url, 'https://www.kvraudio.com/product/pro-channel-by-some-co');
  });

  it('handles AudioDamage mapping', () => {
    const url = buildKvrUrl('Dubstation', 'AudioDamage');
    assert.strictEqual(url, 'https://www.kvraudio.com/product/dubstation-by-audio-damage');
  });

  it('handles izotope mapping', () => {
    const url = buildKvrUrl('Ozone', 'iZotope');
    assert.strictEqual(url, 'https://www.kvraudio.com/product/ozone-by-izotope');
  });

  it('handles eventide mapping', () => {
    const url = buildKvrUrl('H3000', 'Eventide');
    assert.strictEqual(url, 'https://www.kvraudio.com/product/h-3000-by-eventide');
  });
});

// ── applyKvrCache edge cases ──

describe('applyKvrCache edge cases', () => {
  it('does not set hasUpdate when latestVersion equals current', () => {
    const plugins = [{ name: 'Test', manufacturer: 'Co', version: '1.0' }];
    const cache = { 'co|||test': { latestVersion: '1.0', hasUpdate: false } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].hasUpdate, undefined);
  });

  it('handles multiple plugins with different cache states', () => {
    const plugins = [
      { name: 'A', manufacturer: 'X', version: '1.0' },
      { name: 'B', manufacturer: 'Y', version: '2.0' },
    ];
    const cache = {
      'x|||a': { latestVersion: '2.0', hasUpdate: true, kvrUrl: 'https://kvr/a' },
      'y|||b': { kvrUrl: 'https://kvr/b' },
    };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].hasUpdate, true);
    assert.strictEqual(plugins[0].latestVersion, '2.0');
    assert.strictEqual(plugins[1].kvrUrl, 'https://kvr/b');
    assert.strictEqual(plugins[1].hasUpdate, undefined);
  });

  it('applies source from cache', () => {
    const plugins = [{ name: 'Test', manufacturer: 'Co', version: '1.0' }];
    const cache = { 'co|||test': { source: 'kvr-direct' } };
    applyKvrCache(plugins, cache);
    assert.strictEqual(plugins[0].source, 'kvr-direct');
  });
});

// ── metaItem edge cases ──

describe('metaItem edge cases', () => {
  it('handles zero value (falsy becomes dash)', () => {
    const html = metaItem('Count', 0);
    // 0 is falsy, so (value || '—') yields '—'
    assert.ok(html.includes('\u2014'));
  });

  it('handles boolean false (falsy becomes dash)', () => {
    const html = metaItem('Active', false);
    assert.ok(html.includes('\u2014'));
  });

  it('handles long value', () => {
    const long = 'a'.repeat(500);
    const html = metaItem('Data', long);
    assert.ok(html.includes(long));
  });
});

// ── buildPluginCardHtml edge cases ──

describe('buildPluginCardHtml edge cases', () => {
  function makePlugin(overrides) {
    return {
      name: 'TestPlugin',
      type: 'VST3',
      manufacturer: 'TestCo',
      version: '1.0.0',
      path: '/usr/lib/vst3/TestPlugin.vst3',
      size: '2.5 MB',
      modified: '2025-01-01',
      ...overrides,
    };
  }

  it('renders KVR button for every plugin', () => {
    const html = buildPluginCardHtml(makePlugin());
    assert.ok(html.includes('btn-kvr'));
    assert.ok(html.includes('KVR'));
  });

  it('renders folder button for every plugin', () => {
    const html = buildPluginCardHtml(makePlugin());
    assert.ok(html.includes('btn-folder'));
  });

  it('does not show download when hasUpdate but no updateUrl', () => {
    const html = buildPluginCardHtml(makePlugin({
      hasUpdate: true,
      currentVersion: '1.0',
      latestVersion: '2.0',
      updateUrl: null,
    }));
    assert.ok(!html.includes('btn-download'));
  });

  it('escapes path with special characters', () => {
    const html = buildPluginCardHtml(makePlugin({ path: "/usr/lib/it's a plugin.vst3" }));
    assert.ok(html.includes("\\'s"));
  });

  it('shows version arrow when update available', () => {
    const html = buildPluginCardHtml(makePlugin({
      hasUpdate: true,
      currentVersion: '1.0',
      latestVersion: '2.0',
    }));
    assert.ok(html.includes('version-arrow'));
    assert.ok(html.includes('v1.0'));
    assert.ok(html.includes('v2.0'));
  });

  it('shows plugin size and modified date', () => {
    const html = buildPluginCardHtml(makePlugin({ size: '15.3 MB', modified: '2025-06-15' }));
    assert.ok(html.includes('15.3 MB'));
    assert.ok(html.includes('2025-06-15'));
  });
});

// ── Settings helpers ──

describe('getSettingValue', () => {
  // Simulates the getSettingValue function from frontend
  function getSettingValue(store, key, defaultVal) {
    return store[key] || defaultVal;
  }

  it('returns default when key missing', () => {
    assert.strictEqual(getSettingValue({}, 'pageSize', '500'), '500');
  });

  it('returns stored value when present', () => {
    assert.strictEqual(getSettingValue({ pageSize: '1000' }, 'pageSize', '500'), '1000');
  });

  it('returns default for falsy stored value', () => {
    assert.strictEqual(getSettingValue({ pageSize: '' }, 'pageSize', '500'), '500');
  });
});

describe('COLOR_SCHEMES', () => {
  const COLOR_SCHEMES = {
    cyberpunk: { label: 'Cyberpunk', vars: {} },
    midnight: { label: 'Midnight', vars: { '--accent': '#7c3aed', '--cyan': '#38bdf8' } },
    matrix: { label: 'Matrix', vars: { '--accent': '#22c55e', '--cyan': '#39ff14' } },
    ember: { label: 'Ember', vars: { '--accent': '#f59e0b', '--cyan': '#fb923c' } },
    arctic: { label: 'Arctic', vars: { '--accent': '#0ea5e9', '--cyan': '#67e8f9' } },
  };

  it('has 5 schemes', () => {
    assert.strictEqual(Object.keys(COLOR_SCHEMES).length, 5);
  });

  it('cyberpunk has empty vars (uses defaults)', () => {
    assert.deepStrictEqual(COLOR_SCHEMES.cyberpunk.vars, {});
  });

  it('all schemes have a label', () => {
    for (const [name, scheme] of Object.entries(COLOR_SCHEMES)) {
      assert.ok(scheme.label, `${name} should have a label`);
    }
  });

  it('non-default schemes override --accent and --cyan', () => {
    for (const [name, scheme] of Object.entries(COLOR_SCHEMES)) {
      if (name === 'cyberpunk') continue;
      assert.ok(scheme.vars['--accent'], `${name} should override --accent`);
      assert.ok(scheme.vars['--cyan'], `${name} should override --cyan`);
    }
  });

  it('all color values are valid hex', () => {
    const hexRegex = /^#[0-9a-f]{6}$/i;
    for (const [name, scheme] of Object.entries(COLOR_SCHEMES)) {
      for (const [key, val] of Object.entries(scheme.vars)) {
        if (val.startsWith('#')) {
          assert.ok(hexRegex.test(val), `${name} ${key}: "${val}" should be valid hex`);
        }
      }
    }
  });
});

describe('filterPlugins with sort settings', () => {
  function sortPlugins(plugins, sortKey) {
    const sorted = [...plugins];
    switch (sortKey) {
      case 'name-asc': sorted.sort((a, b) => a.name.localeCompare(b.name)); break;
      case 'name-desc': sorted.sort((a, b) => b.name.localeCompare(a.name)); break;
      case 'type': sorted.sort((a, b) => a.type.localeCompare(b.type)); break;
      case 'manufacturer': sorted.sort((a, b) => (a.manufacturer || '').localeCompare(b.manufacturer || '')); break;
    }
    return sorted;
  }

  const plugins = [
    { name: 'Zebra', type: 'VST3', manufacturer: 'u-he' },
    { name: 'Analog', type: 'AU', manufacturer: 'Ableton' },
    { name: 'Massive', type: 'VST2', manufacturer: 'NI' },
  ];

  it('sorts by name ascending', () => {
    const sorted = sortPlugins(plugins, 'name-asc');
    assert.strictEqual(sorted[0].name, 'Analog');
    assert.strictEqual(sorted[2].name, 'Zebra');
  });

  it('sorts by name descending', () => {
    const sorted = sortPlugins(plugins, 'name-desc');
    assert.strictEqual(sorted[0].name, 'Zebra');
    assert.strictEqual(sorted[2].name, 'Analog');
  });

  it('sorts by type', () => {
    const sorted = sortPlugins(plugins, 'type');
    assert.strictEqual(sorted[0].type, 'AU');
    assert.strictEqual(sorted[1].type, 'VST2');
    assert.strictEqual(sorted[2].type, 'VST3');
  });

  it('sorts by manufacturer', () => {
    const sorted = sortPlugins(plugins, 'manufacturer');
    assert.strictEqual(sorted[0].manufacturer, 'Ableton');
    assert.strictEqual(sorted[2].manufacturer, 'u-he');
  });
});

describe('debounce', () => {
  function debounce(fn, ms) {
    let timer;
    return function(...args) {
      clearTimeout(timer);
      timer = setTimeout(() => fn.apply(this, args), ms);
    };
  }

  it('delays execution', (t, done) => {
    let called = 0;
    const debounced = debounce(() => { called++; }, 50);
    debounced();
    debounced();
    debounced();
    assert.strictEqual(called, 0);
    setTimeout(() => {
      assert.strictEqual(called, 1);
      done();
    }, 100);
  });

  it('passes arguments through', (t, done) => {
    let received;
    const debounced = debounce((a, b) => { received = [a, b]; }, 20);
    debounced(1, 2);
    setTimeout(() => {
      assert.deepStrictEqual(received, [1, 2]);
      done();
    }, 50);
  });
});

describe('ensureSearchCache', () => {
  function ensureSearchCache(plugins) {
    for (const p of plugins) {
      if (p._nameLower === undefined) {
        p._nameLower = p.name.toLowerCase();
        p._mfgLower = (p.manufacturer || '').toLowerCase();
      }
    }
  }

  it('adds lowercase cache properties', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer Records' }];
    ensureSearchCache(plugins);
    assert.strictEqual(plugins[0]._nameLower, 'serum');
    assert.strictEqual(plugins[0]._mfgLower, 'xfer records');
  });

  it('does not overwrite existing cache', () => {
    const plugins = [{ name: 'Serum', manufacturer: 'Xfer', _nameLower: 'cached', _mfgLower: 'cached' }];
    ensureSearchCache(plugins);
    assert.strictEqual(plugins[0]._nameLower, 'cached');
  });

  it('handles null manufacturer', () => {
    const plugins = [{ name: 'Test', manufacturer: null }];
    ensureSearchCache(plugins);
    assert.strictEqual(plugins[0]._mfgLower, '');
  });

  it('handles missing manufacturer', () => {
    const plugins = [{ name: 'Test' }];
    ensureSearchCache(plugins);
    assert.strictEqual(plugins[0]._mfgLower, '');
  });
});

describe('filterPlugins with cached search', () => {
  function ensureSearchCache(plugins) {
    for (const p of plugins) {
      if (p._nameLower === undefined) {
        p._nameLower = p.name.toLowerCase();
        p._mfgLower = (p.manufacturer || '').toLowerCase();
      }
    }
  }

  function filterPlugins(plugins, search, typeFilter, statusFilter) {
    ensureSearchCache(plugins);
    return plugins.filter(p => {
      const matchesSearch = !search || p._nameLower.includes(search) || p._mfgLower.includes(search);
      const matchesType = typeFilter === 'all' || p.type === typeFilter;
      let matchesStatus = true;
      if (statusFilter === 'update') matchesStatus = p.hasUpdate === true;
      if (statusFilter === 'current') matchesStatus = p.hasUpdate === false && p.source !== 'not-found';
      if (statusFilter === 'unknown') matchesStatus = !p.hasUpdate && p.source === 'not-found';
      return matchesSearch && matchesType && matchesStatus;
    });
  }

  const plugins = [
    { name: 'Serum', type: 'VST3', manufacturer: 'Xfer', hasUpdate: true, source: 'kvr' },
    { name: 'Massive', type: 'VST2', manufacturer: 'NI', hasUpdate: false, source: 'kvr' },
  ];

  it('empty search matches all', () => {
    assert.strictEqual(filterPlugins(plugins, '', 'all', 'all').length, 2);
  });

  it('uses cached lowercase for search', () => {
    const result = filterPlugins(plugins, 'serum', 'all', 'all');
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0]._nameLower, 'serum');
  });

  it('searches manufacturer via cache', () => {
    const result = filterPlugins(plugins, 'xfer', 'all', 'all');
    assert.strictEqual(result.length, 1);
  });
});

describe('hexToRgba', () => {
  function hexToRgba(hex, alpha) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  it('converts red to rgba', () => {
    assert.strictEqual(hexToRgba('#ff0000', 0.5), 'rgba(255, 0, 0, 0.5)');
  });

  it('converts black to rgba', () => {
    assert.strictEqual(hexToRgba('#000000', 1), 'rgba(0, 0, 0, 1)');
  });

  it('converts white to rgba', () => {
    assert.strictEqual(hexToRgba('#ffffff', 0.4), 'rgba(255, 255, 255, 0.4)');
  });

  it('converts cyberpunk accent', () => {
    assert.strictEqual(hexToRgba('#ff2a6d', 0.4), 'rgba(255, 42, 109, 0.4)');
  });

  it('handles zero alpha', () => {
    assert.strictEqual(hexToRgba('#05d9e8', 0), 'rgba(5, 217, 232, 0)');
  });
});

describe('custom scheme var generation', () => {
  function hexToRgba(hex, alpha) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  function buildCustomVars(inputs) {
    const vars = {};
    for (const { var: v, value: hex } of inputs) {
      vars[v] = hex;
      if (v === '--accent') {
        vars['--accent-light'] = hex;
        vars['--accent-glow'] = hexToRgba(hex, 0.4);
      } else if (v === '--cyan') {
        vars['--cyan-glow'] = hexToRgba(hex, 0.4);
        vars['--cyan-dim'] = hexToRgba(hex, 0.15);
      }
    }
    return vars;
  }

  it('generates glow variants for accent', () => {
    const vars = buildCustomVars([{ var: '--accent', value: '#ff0000' }]);
    assert.strictEqual(vars['--accent'], '#ff0000');
    assert.strictEqual(vars['--accent-light'], '#ff0000');
    assert.strictEqual(vars['--accent-glow'], 'rgba(255, 0, 0, 0.4)');
  });

  it('generates glow and dim for cyan', () => {
    const vars = buildCustomVars([{ var: '--cyan', value: '#00ff00' }]);
    assert.strictEqual(vars['--cyan'], '#00ff00');
    assert.strictEqual(vars['--cyan-glow'], 'rgba(0, 255, 0, 0.4)');
    assert.strictEqual(vars['--cyan-dim'], 'rgba(0, 255, 0, 0.15)');
  });

  it('passes through non-special vars unchanged', () => {
    const vars = buildCustomVars([{ var: '--red', value: '#dc2626' }]);
    assert.strictEqual(vars['--red'], '#dc2626');
    assert.strictEqual(vars['--red-glow'], undefined);
  });
});

describe('page size parsing', () => {
  it('parses valid page size', () => {
    assert.strictEqual(parseInt('500', 10), 500);
    assert.strictEqual(parseInt('1000', 10), 1000);
    assert.strictEqual(parseInt('2000', 10), 2000);
  });

  it('falls back to default on invalid', () => {
    assert.strictEqual(parseInt(null || '500', 10), 500);
    assert.strictEqual(parseInt('' || '500', 10), 500);
    assert.strictEqual(parseInt(undefined || '500', 10), 500);
  });

  it('clamps to range boundaries', () => {
    const clamp = (v, min, max) => Math.max(min, Math.min(max, v));
    assert.strictEqual(clamp(50, 100, 2000), 100);
    assert.strictEqual(clamp(3000, 100, 2000), 2000);
    assert.strictEqual(clamp(500, 100, 2000), 500);
  });
});
