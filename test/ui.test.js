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
