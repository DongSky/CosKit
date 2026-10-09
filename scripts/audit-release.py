"""Audit files proposed for publication without printing private values.

Run before staging, then with --staged before pushing. Reads local .env files only
to compare sensitive values; it never prints those values or writes them to a report.
"""
import argparse, json, os, re, subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser();p.add_argument('--staged',action='store_true');args=p.parse_args()
    cmd=['git','diff','--cached','--name-only','--diff-filter=ACMR','-z'] if args.staged else ['git','ls-files','--cached','--others','--exclude-standard','-z']
    names=set(subprocess.check_output(cmd,cwd=ROOT).decode('utf-8').split('\0'))-{''}
    secrets=set()
    envs=[ROOT/'.env',ROOT.parent/'.env',ROOT/'.dev-data/harness/ai/.env']
    for f in envs:
        if not f.exists():continue
        for line in f.read_text(encoding='utf-8-sig').splitlines():
            if '=' not in line or line.lstrip().startswith('#'):continue
            key,value=line.split('=',1);value=value.strip().strip('\"\'')
            if re.search(r'key|token|secret|password',key,re.I) and len(value)>=8:secrets.add(value.encode())
    token=ROOT/'.dev-data/cosplay-control.token'
    if token.exists():secrets.add(token.read_bytes().strip())
    findings=[]
    forbidden=re.compile(r'(^|/)(\.env(?:\..*)?|\.dev-data|test_output|CosKitData|target(?:-release|-wasm)?|corpus|node_modules)(/|$)|^website/coskit/media/')
    private_path=re.compile(r'(?i)(?:[A-Z]:[/\\]+Users[/\\]+[^/\\\s]+|[A-Z]:[/\\]+code[/\\]+CosKitV2)')
    text_ext={'.md','.txt','.json','.toml','.py','.ps1','.rs','.js','.mjs','.cjs','.html','.css','.yml','.yaml','.lock','.sh','.bat','.tsv'}
    for name in sorted(names):
        f=ROOT/name
        if not f.is_file():continue
        data=subprocess.check_output(['git','show',':'+name],cwd=ROOT) if args.staged else f.read_bytes()
        if forbidden.search(name):findings.append({'file':name,'kind':'private-artifact-path'})
        if any(s and s in data for s in secrets):findings.append({'file':name,'kind':'local-secret-match'})
        if len(data)>90*1024*1024:findings.append({'file':name,'kind':'oversized-file'})
        if f.suffix in text_ext:
            text=data.decode('utf-8-sig',errors='replace')
            for i,line in enumerate(text.splitlines(),1):
                if private_path.search(line):findings.append({'file':name,'line':i,'kind':'private-absolute-path'})
    print(json.dumps({'mode':'staged' if args.staged else 'working-tree','files':len(names),'findings':findings},ensure_ascii=False,indent=2))
    return bool(findings)

if __name__=='__main__':raise SystemExit(main())
