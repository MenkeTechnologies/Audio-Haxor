/**
 * Defense-in-depth: Ensure that NO internationalization catalog (all app_i18n_*.json files)
 * contains dangerous HTML tags or event handlers.
 *
 * Harmless tags like <b>, <i>, <u>, <br>, <span> are allowed for basic formatting
 * when the UI uses innerHTML (e.g. performance overlay).
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Dangerous tags that should NEVER be in a translation catalog. */
const FORBIDDEN_TAGS = [
  'script',
  'iframe',
  'object',
  'embed',
  'base',
  'link',
  'meta',
  'style',
  'svg',
  'math',
  'form',
  'input',
  'textarea',
  'select',
  'button',
  'canvas',
  'img',
  'video',
  'audio',
  'frame',
  'frameset',
  'applet',
];

/** Event handler attributes (on*) are strictly forbidden. */
const ON_EVENT_RE = /\bon[a-z]+\s*=/i;

function getLocaleFiles() {
  return readdirSync(i18nDir)
    .filter((f) => f.startsWith('app_i18n_') && f.endsWith('.json'));
}

for (const fname of getLocaleFiles()) {
  const loc = fname.match(/app_i18n_(.*)\.json/)[1];
  const map = JSON.parse(readFileSync(join(i18nDir, fname), 'utf8'));

  for (const [key, value] of Object.entries(map)) {
    if (typeof value !== 'string') continue;

    test(`locale ${loc} key ${key} has no forbidden HTML or event handlers`, () => {
      // Check for forbidden tags: <tag or </tag
      for (const tag of FORBIDDEN_TAGS) {
        const tagRe = new RegExp(`<\\/?${tag}\\b`, 'i');
        assert.ok(
          !tagRe.test(value),
          `${fname} key ${key}: forbidden tag <${tag}> found in value: ${JSON.stringify(value)}`
        );
      }

      // Check for event handlers
      assert.ok(
        !ON_EVENT_RE.test(value),
        `${fname} key ${key}: forbidden event handler found in value: ${JSON.stringify(value)}`
      );

      // Check for javascript: pseudo-protocol
      assert.ok(
        !/javascript:/i.test(value),
        `${fname} key ${key}: forbidden "javascript:" protocol found in value: ${JSON.stringify(value)}`
      );
    });
  }
}
