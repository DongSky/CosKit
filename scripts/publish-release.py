"""Promote explicitly selected, verified desktop CI artifacts to a GitHub Release."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CONFIG = json.loads((ROOT / '.github/release.json').read_text())
REPO = os.environ.get('GITHUB_REPOSITORY', 'DongSky/CosKit')
PLATFORMS = {'CosKit-Windows-x64', 'CosKit-macOS-arm64', 'CosKit-macOS-x64'}


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def api(path):
    return json.loads(command('gh', 'api', f'repos/{REPO}/{path}'))


def verify():
    tag = CONFIG['tag']
    assert re.fullmatch(r'v\d+\.\d+\.\d+', tag), 'Expected a stable version tag'
    assert command('git', 'rev-parse', f'{tag}^{{commit}}') == CONFIG['tag_commit'], 'Tag moved'
    run = api(f"actions/runs/{CONFIG['build_run']}")
    assert run['conclusion'] == 'success' and run['status'] == 'completed', 'Build not successful'
    assert run['head_sha'] == CONFIG['build_commit'], 'Unexpected build commit'
    assert run['path'] == '.github/workflows/build.yml' and run['event'] == 'push'
    assert run['head_repository']['full_name'] == REPO and run['head_branch'] == 'main'
    # v1.0.0 already exists. Packaging/CI and documentation were completed later;
    # never silently promote changed application code under the old tag.
    allowed = {
        '.github/workflows/build.yml', '.github/workflows/check.yml',
        'README.md', 'README.en.md', 'docs/ci.md',
        'docs/images/coskit-v1-workspace.jpg', 'docs/images/coskit-v1-before-after.jpg',
        'native/apps/photocraft/tests/mac_menu_appkit.rs',
        'native/integration-diff.patch', 'native/upstream-manifest.json',
        'scripts/package-macos.py',
    }
    changed = set(command('git', 'diff', '--name-only', CONFIG['tag_commit'], CONFIG['build_commit']).splitlines())
    assert changed <= allowed, f'Unapproved source differences: {changed - allowed}'
    artifacts = api(f"actions/runs/{CONFIG['build_run']}/artifacts?per_page=100")['artifacts']
    assert {a['name'] for a in artifacts} == PLATFORMS and len(artifacts) == 3
    assert not any(a['expired'] for a in artifacts), 'Build artifacts expired'
    if os.environ.get('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a') as stream:
            stream.write(f"run_id={CONFIG['build_run']}\n")
    print('Build provenance and all three platform artifacts verified.')


def assets():
    version = CONFIG['tag'][1:]
    binaries = {
        f'CosKit_{version}_x64-setup.exe', f'CosKit_{version}_x64_portable.zip',
        f'CosKit_{version}_macOS_arm64.zip', f'CosKit_{version}_macOS_x64.zip',
    }
    manifests = {
        f'SHA256SUMS-{version}.txt', f'SHA256SUMS-{version}-macOS-arm64.txt',
        f'SHA256SUMS-{version}-macOS-x64.txt',
    }
    files = {}
    for path in (ROOT / 'release-assets').rglob('*'):
        if not path.is_file():
            continue
        assert not path.is_symlink() and path.name not in files, 'Unexpected or duplicate asset'
        files[path.name] = path
    assert set(files) == binaries | manifests, 'Missing or unexpected release files'
    verified = set()
    for name in manifests:
        for line in files[name].read_text(encoding='utf-8-sig').splitlines():
            digest, filename = line.split(maxsplit=1)
            assert filename in binaries and filename not in verified
            assert re.fullmatch('[0-9a-f]{64}', digest)
            with files[filename].open('rb') as stream:
                assert hashlib.file_digest(stream, 'sha256').hexdigest() == digest, f'Checksum failed: {filename}'
            verified.add(filename)
    assert verified == binaries
    return files


def publish():
    verify()
    files = assets()
    tag = CONFIG['tag']
    existing = next((r for r in api('releases?per_page=100') if r['tag_name'] == tag), None)
    if existing and not existing['draft']:
        expected = {name: path.stat().st_size for name, path in files.items()}
        actual = {a['name']: a['size'] for a in existing['assets']}
        assert actual == expected, 'Published release differs; refusing to overwrite it'
        print(existing['html_url'])
        return
    if not existing:
        command('gh', 'release', 'create', tag, '--repo', REPO, '--verify-tag', '--draft',
                '--title', f'CosKit {tag} — First stable release / 首个正式版本',
                '--notes-file', str(ROOT / CONFIG['notes']))
    command('gh', 'release', 'upload', tag, '--repo', REPO, '--clobber', *map(str, files.values()))
    release = api(f'releases/tags/{tag}')
    assert {a['name']: a['size'] for a in release['assets']} == {n: p.stat().st_size for n, p in files.items()}
    command('gh', 'release', 'edit', tag, '--repo', REPO, '--draft=false', '--latest',
            '--notes-file', str(ROOT / CONFIG['notes']))
    print(f'https://github.com/{REPO}/releases/tag/{tag}')


if __name__ == '__main__':
    {'verify': verify, 'publish': publish}[sys.argv[1]]()
