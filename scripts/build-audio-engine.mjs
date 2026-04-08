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
import os from 'node:os';
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
  // Inline `cmd /c "call … && …"` broke on current windows-latest (Node 22 / runner image): cmd
  // reported vcvars64.bat as not found due to quote parsing. A temp .bat avoids nested quoting.
  const tmpBat = path.join(
    os.tmpdir(),
    `audio-engine-vcvars-${process.pid}-${Date.now()}.bat`,
  );
  const bat = [
    '@echo off',
    `call "${vcvars}"`,
    'if errorlevel 1 exit /b 1',
    'set __AUDIO_ENGINE_VCVARS=1',
    `"${node}" "${scriptPath}"`,
  ].join('\r\n');
  fs.writeFileSync(tmpBat, bat, 'utf8');
  const sysRoot = process.env.SystemRoot || 'C:\\Windows';
  const cmdExe = path.join(sysRoot, 'System32', 'cmd.exe');
  if (!fs.existsSync(cmdExe)) {
    console.error(`build-audio-engine: missing ${cmdExe}`);
    process.exit(1);
  }
  try {
    execFileSync(cmdExe, ['/d', '/c', tmpBat], { stdio: 'inherit', cwd: root, env: process.env });
  } catch (e) {
    try {
      fs.unlinkSync(tmpBat);
    } catch (_) {}
    process.exit(e && typeof e.status === 'number' ? e.status : 1);
  }
  try {
    fs.unlinkSync(tmpBat);
  } catch (_) {}
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
