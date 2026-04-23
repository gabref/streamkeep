import { spawn } from 'node:child_process';

const [, , scriptName, ...scriptArgs] = process.argv;

if (!scriptName) {
  console.error('usage: node scripts/run-package-script.mjs <script> [...args]');
  process.exit(1);
}

const packageManagerExec = process.env.npm_execpath;
const command = packageManagerExec
  ? process.execPath
  : process.platform === 'win32'
    ? 'npm.cmd'
    : 'npm';
const args = packageManagerExec
  ? [packageManagerExec, 'run', scriptName, ...scriptArgs]
  : ['run', scriptName, ...scriptArgs];

const child = spawn(command, args, {
  env: process.env,
  stdio: 'inherit',
});

child.on('exit', (code, signal) => {
  if (signal) {
    console.error(`${scriptName} terminated with signal ${signal}`);
    process.exit(1);
  }

  process.exit(code ?? 0);
});
