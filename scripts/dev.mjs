// Keep development fixtures and configuration separate from installed CosKit.
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const cli = resolve(root, 'node_modules/@tauri-apps/cli/tauri.js');
const child = spawn(process.execPath, [cli, 'dev', ...process.argv.slice(2)], {
  cwd: root, stdio: 'inherit',
  env: { ...process.env, COSKIT_DATA_DIR: process.env.COSKIT_DATA_DIR || resolve(root, '.dev-data') },
});
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code || 0; });
