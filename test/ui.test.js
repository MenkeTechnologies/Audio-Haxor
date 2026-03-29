const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// Pure utility functions replicated from frontend/index.html for testing.
// escapeHtml uses DOM APIs in the original; replicated here with equivalent
// string replacements so it can run under Node.

describe('escapeHtml', () => {
  function escapeHtml(str) {
    return (str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

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
});

describe('escapePath', () => {
  function escapePath(str) {
    return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
  }

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
  function slugify(str) {
    return str
      .replace(/([a-z])([A-Z])/g, '$1-$2')
      .replace(/([a-zA-Z])(\d)/g, '$1-$2')
      .replace(/(\d)([a-zA-Z])/g, '$1-$2')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

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
});

describe('buildKvrUrl', () => {
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

  function slugify(str) {
    return str
      .replace(/([a-z])([A-Z])/g, '$1-$2')
      .replace(/([a-zA-Z])(\d)/g, '$1-$2')
      .replace(/(\d)([a-zA-Z])/g, '$1-$2')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  function buildKvrUrl(name, manufacturer) {
    const nameSlug = slugify(name);
    if (manufacturer && manufacturer !== 'Unknown') {
      const mfgLower = manufacturer.toLowerCase().replace(/[^a-z0-9]+/g, '');
      const mfgSlug = KVR_MANUFACTURER_MAP[mfgLower] || slugify(manufacturer);
      return `https://www.kvraudio.com/product/${nameSlug}-by-${mfgSlug}`;
    }
    return `https://www.kvraudio.com/product/${nameSlug}`;
  }

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
});

describe('kvrCacheKey', () => {
  function kvrCacheKey(plugin) {
    return `${(plugin.manufacturer || 'Unknown').toLowerCase()}|||${plugin.name.toLowerCase()}`;
  }

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
