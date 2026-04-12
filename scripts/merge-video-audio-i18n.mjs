#!/usr/bin/env node
/**
 * Merges video-audio-route strings into app_i18n_*.json from English + scripts/video-audio-i18n-overrides.json.
 * Locales not listed in overrides (e.g. hi, id, vi) get English strings. Re-sorts keys alphabetically.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const i18nDir = path.join(__dirname, '..', 'i18n');
const overridesPath = path.join(__dirname, 'video-audio-i18n-overrides.json');

const KEYS = [
    'menu.palette_video_audio_engine',
    'menu.palette_video_audio_html5',
    'menu.video_audio_route_engine',
    'menu.video_audio_route_html5',
    'toast.video_audio_route',
    'ui.opt.video_audio_route_engine',
    'ui.opt.video_audio_route_html5',
    'ui.sd.video_audio_route',
    'ui.shortcut.video_audio_route_engine',
    'ui.shortcut.video_audio_route_html5',
    'ui.st.video_audio_route',
    'ui.tt.video_audio_route',
];

const en = JSON.parse(fs.readFileSync(path.join(i18nDir, 'app_i18n_en.json'), 'utf8'));
const overrides = JSON.parse(fs.readFileSync(overridesPath, 'utf8'));

for (const name of fs.readdirSync(i18nDir)) {
    if (!name.startsWith('app_i18n_') || !name.endsWith('.json') || name === 'app_i18n_en.json') continue;
    const code = name.slice('app_i18n_'.length, -'.json'.length);
    const p = path.join(i18nDir, name);
    const data = JSON.parse(fs.readFileSync(p, 'utf8'));
    const tr = overrides[code] || {};
    for (const k of KEYS) {
        data[k] = tr[k] ?? en[k];
    }
    const sorted = {};
    for (const k of Object.keys(data).sort()) sorted[k] = data[k];
    fs.writeFileSync(p, JSON.stringify(sorted, null, 2) + '\n');
    console.log('merged', name);
}
