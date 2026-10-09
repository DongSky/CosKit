"""Fail if an upstream functional file disappears or an unaudited implementation changes.

The frozen manifest makes this independent of the adjacent reference checkout.
The overlay list permits additive CosKit integration, branding and build configuration only.
"""
import hashlib,json,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
native=root/'native'
manifest=json.loads((native/'upstream-manifest.json').read_text(encoding='utf-8'))
problems=[]
preserved=0
for name,digest in manifest['files'].items():
    p=native/name
    if not p.is_file(): problems.append(f'missing: {name}');continue
    if name in manifest['overlays']: continue
    actual=hashlib.sha256(p.read_bytes().replace(b'\r\n',b'\n')).hexdigest()
    if actual!=digest: problems.append(f'changed without integration review: {name}')
    else: preserved+=1
# All original registrations remain; only the two explicit CosKit modules are additive.
commands=(native/'crates/engine/src/commands.rs').read_text(encoding='utf-8').replace('    v.extend(crate::coskit_ai::specs());\n','').replace('    v.extend(crate::pipeline_cmds::specs());\n','')
if hashlib.sha256(commands.encode()).hexdigest()!=manifest['files']['crates/engine/src/commands.rs']:
    problems.append('original command registrations changed')
if problems:
    print('\n'.join(problems));sys.exit(1)
print(f'PASS: {preserved} unchanged upstream source/assets; {len(manifest["overlays"])} documented integration overlays; no original commands removed.')
