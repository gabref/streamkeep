import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

const [, , command, ...args] = process.argv;

if (!command) {
  console.error('usage: node scripts/with-windows-sdk.mjs <command> [...args]');
  process.exit(1);
}

const env = { ...process.env };

if (process.platform === 'win32') {
  const rcDir = findWindowsResourceCompilerDirectory();

  if (rcDir) {
    env.RC_X86_64_PC_WINDOWS_MSVC = join(rcDir, 'rc.exe');
    env.PATH = `${rcDir};${env.PATH ?? ''}`;
  }

  if (env.JAVA_HOME) {
    const javaBin = join(env.JAVA_HOME, 'bin');

    if (existsSync(javaBin)) {
      env.PATH = `${javaBin};${env.PATH ?? ''}`;
    }
  }
}

const executable = process.platform === 'win32' && command === 'tauri' ? 'pnpm' : command;
const executableArgs =
  process.platform === 'win32' && command === 'tauri' ? ['exec', 'tauri', ...args] : args;

const child = spawn(executable, executableArgs, {
  env,
  shell: process.platform === 'win32' && executable === 'pnpm',
  stdio: 'inherit',
});

child.on('exit', (code, signal) => {
  if (signal) {
    console.error(`${command} terminated with signal ${signal}`);
    process.exit(1);
  }

  process.exit(code ?? 0);
});

function findWindowsResourceCompilerDirectory() {
  const sdkRoot = 'C:\\Program Files (x86)\\Windows Kits\\10\\bin';

  if (!existsSync(sdkRoot)) {
    return null;
  }

  const versions = readdirSync(sdkRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort(compareVersionLikeNames)
    .reverse();

  for (const version of versions) {
    const candidate = join(sdkRoot, version, 'x64');

    if (existsSync(join(candidate, 'rc.exe'))) {
      return candidate;
    }
  }

  return null;
}

function compareVersionLikeNames(left, right) {
  const leftParts = left.split('.').map(Number);
  const rightParts = right.split('.').map(Number);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const difference = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);

    if (difference !== 0) {
      return difference;
    }
  }

  return left.localeCompare(right);
}
