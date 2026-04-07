#!/usr/bin/env node
/**
 * Build the JUCE `audio-engine` AudioEngine (CMake + Ninja) into `target/<debug|release>/audio-engine`.
 * Used by `beforeDevCommand` (debug) and `prepare-audio-engine-audioengine.mjs` (release).
 *
 * On Windows, CMake must use MSVC (JUCE dropped MinGW). We re-exec under `vcvars64.bat` so the
 * rest of the process inherits `cl`/Windows SDK without polluting CI (e.g. `cargo test` would hit
 * STATUS_ENTRYPOINT_NOT_FOUND if the job used a global MSVC PATH from `msvc-dev-cmd`).
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const scriptPath = fileURLToPath(import.meta.url);
const root = path.join(__dirname, '..');

if (process.platform === 'win32' && !process.env.__AUDIO_ENGINE_VCVARS) {
  const vswhere = path.join(
    process.env['ProgramFiles(x86)'] || '',
    'Microsoft Visual Studio',
    'Installer',
    'vswhere.exe',
  );
  if (!fs.existsSync(vswhere)) {
    console.error('build-audio-engine: vswhere.exe not found (install Visual Studio Build Tools)');
    process.exit(1);
  }
  const vsPath = execFileSync(vswhere, ['-latest', '-property', 'installationPath'], {
    encoding: 'utf8',
  }).trim();
  if (!vsPath) {
    console.error('build-audio-engine: no Visual Studio installation found');
    process.exit(1);
  }
  const vcvars = path.join(vsPath, 'VC', 'Auxiliary', 'Build', 'vcvars64.bat');
  if (!fs.existsSync(vcvars)) {
    console.error(`build-audio-engine: missing ${vcvars}`);
    process.exit(1);
  }
  const node = process.execPath;
  // Invoke cmd.exe explicitly — execSync(..., { shell: true }) still failed on GitHub windows-latest
  // (vcvars path quoted incorrectly). One /c string lets cmd parse `call "…\vcvars64.bat" && …` reliably.
  const line = `call "${vcvars}" && set __AUDIO_ENGINE_VCVARS=1&& "${node}" "${scriptPath}"`;
  const sysRoot = process.env.SystemRoot || 'C:\\Windows';
  const cmdExe = path.join(sysRoot, 'System32', 'cmd.exe');
  if (!fs.existsSync(cmdExe)) {
    console.error(`build-audio-engine: missing ${cmdExe}`);
    process.exit(1);
  }
  try {
    execFileSync(cmdExe, ['/d', '/c', line], { stdio: 'inherit', cwd: root, env: process.env });
  } catch (e) {
    process.exit(e && typeof e.status === 'number' ? e.status : 1);
  }
  process.exit(0);
}

const buildType = process.env.AUDIO_ENGINE_BUILD_TYPE === 'release' ? 'Release' : 'Debug';
const buildDir = path.join(root, 'audio-engine', 'build');
const ext = process.platform === 'win32' ? '.exe' : '';

fs.mkdirSync(buildDir, { recursive: true });

const cmakeArgs = [
  '-S',
  path.join(root, 'audio-engine'),
  '-B',
  buildDir,
  '-G',
  'Ninja',
  `-DCMAKE_BUILD_TYPE=${buildType}`,
];

execFileSync('cmake', cmakeArgs, { stdio: 'inherit', cwd: root });
execFileSync('cmake', ['--build', buildDir, '--parallel'], { stdio: 'inherit', cwd: root });

const outDir = path.join(root, 'target', buildType === 'Debug' ? 'debug' : 'release');
const outName = `audio-engine${ext}`;
const built = path.join(outDir, outName);
if (!fs.existsSync(built)) {
  console.error(`build-audio-engine: expected ${built}`);
  process.exit(1);
}
console.log(`build-audio-engine: ${built}`);
