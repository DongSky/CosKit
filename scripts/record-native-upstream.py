"""Record the imported reference once; reviewers inspect the explicit overlay diff."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=root.parent/'photocraft'
native=root/'native'
files={}
overlays={}
for rel in subprocess.check_output(['git','ls-files'],cwd=source,text=True).splitlines():
    if not (rel.startswith(('crates/','apps/','assets/','contributors/')) or rel in ['Cargo.toml','Cargo.lock']):continue
    if rel.endswith(('.md','.yml','.yaml','.html','.toml')) and '/src/' not in rel and not rel.endswith('Cargo.toml'):continue
    original=(source/rel).read_bytes().replace(b'\r\n',b'\n')
    files[rel]=hashlib.sha256(original).hexdigest()
    actual=(native/rel).read_bytes().replace(b'\r\n',b'\n')
    if actual!=original:overlays[rel]='CosKit additive AI integration, product identity, or build/dependency metadata; see integration-diff.patch'
manifest={'source':'https://github.com/storytold/photocraft','commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=source,text=True).strip(),'files':files,'overlays':overlays}
(native/'upstream-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
import difflib
diff=[]
for rel in overlays:
    if rel=='Cargo.lock':continue
    a=(source/rel).read_text(encoding='utf-8').splitlines(keepends=True)
    b=(native/rel).read_text(encoding='utf-8').splitlines(keepends=True)
    diff.extend(difflib.unified_diff(a,b,fromfile='upstream/'+rel,tofile='coskit/'+rel))
(native/'integration-diff.patch').write_text(''.join(diff),encoding='utf-8')
print(f'{len(files)} files, {len(overlays)} reviewed overlays')
