import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const mode=process.argv[2] ?? 'dev';
const env={...process.env};
if(mode==='dev') {
  env.COSKIT_DATA_DIR=path.join(root,'.dev-data');
  env.COSKIT_NATIVE_CONFIG_DIR=path.join(root,'.dev-data','native');
}
const args=mode==='dev' ? ['run','--manifest-path','native/Cargo.toml','-p','photocraft','--features','heif','--',...process.argv.slice(3)]
  : ['build','--release','--manifest-path','native/Cargo.toml','-p','photocraft','-p','photocraft-cli','--features','photocraft/heif,photocraft-cli/heif',...process.argv.slice(3)];
const child=spawn('cargo',args,{cwd:root,env,stdio:'inherit'});
child.on('error',e=>{console.error(e.message);process.exitCode=1;});
child.on('exit',code=>{process.exitCode=code ?? 1;});
