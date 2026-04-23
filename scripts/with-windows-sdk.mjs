import { existsSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { spawn } from 'node:child_process';
import { homedir } from 'node:os';

const [, , command, ...args] = process.argv;

if (!command) {
  console.error('usage: node scripts/with-windows-sdk.mjs <command> [...args]');
  process.exit(1);
}

const env = { ...process.env };

if (process.platform === 'win32') {
  prependPath(dirname(process.execPath));

  const rcDir = findWindowsResourceCompilerDirectory();

  if (rcDir) {
    env.RC_X86_64_PC_WINDOWS_MSVC = join(rcDir, 'rc.exe');
    prependPath(rcDir);
  }

  const cargoBin = findCargoBinDirectory();
  if (cargoBin) {
    prependPath(cargoBin);
  }

  if (env.JAVA_HOME) {
    const javaBin = join(env.JAVA_HOME, 'bin');

    if (existsSync(javaBin)) {
      prependPath(javaBin);
    }
  }
}

const resolvedCommand = resolveCommand(command, args);

const child = spawn(resolvedCommand.executable, resolvedCommand.args, {
  env,
  shell: false,
  stdio: 'inherit',
});

child.on('exit', (code, signal) => {
  if (signal) {
    console.error(`${command} terminated with signal ${signal}`);
    process.exit(1);
  }

  process.exit(code ?? 0);
});

function resolveCommand(command, args) {
  if (command !== 'tauri') {
    return { executable: command, args };
  }

  const localCli = join(process.cwd(), 'node_modules', '@tauri-apps', 'cli', 'tauri.js');

  if (existsSync(localCli)) {
    return {
      executable: process.execPath,
      args: [localCli, ...args],
    };
  }

  return { executable: command, args };
}

function prependPath(directory) {
  const currentPath = env.PATH ?? '';
  const entries = currentPath.split(';').filter(Boolean);
  const alreadyPresent = entries.some((entry) => entry.toLowerCase() === directory.toLowerCase());

  if (!alreadyPresent) {
    env.PATH = `${directory};${currentPath}`;
  }
}

function findCargoBinDirectory() {
  const candidates = [
    process.env.CARGO_HOME ? join(process.env.CARGO_HOME, 'bin') : null,
    join(homedir(), '.cargo', 'bin'),
  ].filter(Boolean);

  for (const candidate of candidates) {
    if (existsSync(join(candidate, 'cargo.exe')) || existsSync(join(candidate, 'cargo'))) {
      return candidate;
    }
  }

  return null;
}

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
