"""Package native macOS binaries as CosKit.app + CLI, with ad-hoc signatures.

Run on macOS after `npm run build -- --locked`. No signing credentials required;
these CI archives are not Apple-notarized releases.
"""
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    command = [str(a) for a in args]
    print('+ ' + ' '.join(command), flush=True)
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout, end='', flush=True)
    if result.returncode:
        raise RuntimeError(f'{command[0]} failed ({result.returncode}): {result.stdout}')


def bundle_info(version):
    template = (ROOT / 'native/packaging/macos/Info.plist.in').read_bytes()
    info = plistlib.loads(template)
    for key in ('CFBundleDisplayName', 'CFBundleName', 'CFBundleExecutable', 'CFBundleIconFile'):
        info[key] = 'CosKit'
    info['CFBundleIdentifier'] = 'si.prts.coskit'
    info['CFBundleShortVersionString'] = version.split('-')[0]
    info['CFBundleVersion'] = version.split('-')[0]
    info.pop('PhotoCraftVersion', None)
    info.pop('PhotoCraftBuildCommit', None)
    info['CosKitVersion'] = version
    info['CosKitBuildCommit'] = os.environ.get('GITHUB_SHA', 'local')
    info['NSHumanReadableCopyright'] = 'CosKit © Suzutsuki. Includes PhotoCraft contributors; see bundled licenses.'
    # Preserve upstream format identifiers for interoperability, with CosKit icons.
    for item in info['UTExportedTypeDeclarations']:
        item['UTTypeIconFile'] = 'CosKit'
    for item in info['CFBundleDocumentTypes']:
        if 'CFBundleTypeIconFile' in item:
            item['CFBundleTypeIconFile'] = 'CosKit'
    info['UTExportedTypeDeclarations'].append({
        'UTTypeIdentifier': 'si.prts.coskit.ckpipe',
        'UTTypeDescription': 'CosKit Pipeline Project',
        'UTTypeConformsTo': ['public.data'],
        'UTTypeTagSpecification': {'public.filename-extension': ['ckpipe']},
    })
    info['CFBundleDocumentTypes'].insert(0, {
        'CFBundleTypeName': 'CosKit Pipeline Project',
        'CFBundleTypeRole': 'Editor', 'LSHandlerRank': 'Owner',
        'LSItemContentTypes': ['si.prts.coskit.ckpipe'],
    })
    return info


def main():
    if platform.system() != 'Darwin':
        raise SystemExit('Run this packager on macOS.')
    version = json.loads((ROOT / 'package.json').read_text())['version']
    arch = {'arm64': 'arm64', 'x86_64': 'x64'}[platform.machine()]
    build = ROOT / 'native/target/release'
    dist = ROOT / 'dist'
    dist.mkdir(exist_ok=True)
    archive = dist / f'CosKit_{version}_macOS_{arch}.zip'
    with tempfile.TemporaryDirectory(prefix='coskit-macos-') as tmp:
        stage = Path(tmp) / 'CosKit'
        app = stage / 'CosKit.app'
        contents = app / 'Contents'
        resources = contents / 'Resources'
        (contents / 'MacOS').mkdir(parents=True)
        licenses = resources / 'Licenses'
        licenses.mkdir(parents=True)
        shutil.copy2(build / 'photocraft', contents / 'MacOS/CosKit')
        shutil.copy2(build / 'photocraft-cli', stage / 'coskit-cli')
        shutil.copy2(ROOT / 'src-tauri/icons/icon.icns', resources / 'CosKit.icns')
        for src, name in {
            'LICENSE': 'CosKit-MIT.txt',
            'native/LICENSE-MIT': 'PhotoCraft-MIT.txt',
            'native/LICENSE-APACHE': 'PhotoCraft-Apache.txt',
            'native/NOTICE': 'PhotoCraft-NOTICE.txt',
            'native/ATTRIBUTION.md': 'ATTRIBUTION.md',
            'THIRD_PARTY_NOTICES.md': 'THIRD_PARTY_NOTICES.md',
            'native/assets/icons/LICENSE-lucide.txt': 'LICENSE-lucide.txt',
            'native/assets/icons/LICENSE-noun-magnetic-lasso.txt': 'LICENSE-noun-magnetic-lasso.txt',
            'native/assets/dict/LICENSE-SCOWL.txt': 'LICENSE-SCOWL.txt',
            'native/crates/ui-egui/src/i18n/LICENSE-translations.txt': 'LICENSE-translations.txt',
            'native/assets/app-icon/LICENSE.txt': 'PhotoCraft-icon-LICENSE.txt',
            'native/assets/brushes/deevad-2023/LICENSE-CC0.txt': 'Deevad-CC0.txt',
            'native/assets/brushes/deevad-2023/README.md': 'Deevad-brushes.md',
        }.items():
            shutil.copy2(ROOT / src, licenses / name)
        for name in ('ACKNOWLEDGEMENTS.md', 'README.md', 'README.en.md'):
            shutil.copy2(ROOT / name, resources / name)
        (contents / 'Info.plist').write_bytes(plistlib.dumps(bundle_info(version)))
        (contents / 'PkgInfo').write_bytes(b'APPL????')
        (stage / 'INSTALL.txt').write_text(
            'Copy CosKit.app to Applications. coskit-cli is the command-line tool.\n'
            'CI build: ad-hoc signed, not Apple notarized. Gatekeeper may block opening.\n'
            'Only open trusted builds using macOS Privacy & Security > Open Anyway.\n',
            encoding='utf-8',
        )
        run('plutil', '-lint', contents / 'Info.plist')
        for binary in (contents / 'MacOS/CosKit', stage / 'coskit-cli'):
            # Apple's lipo consumes every argument after -verify_arch as an architecture.
            run('lipo', binary, '-verify_arch', platform.machine())
            run('codesign', '--force', '--sign', '-', '--timestamp=none', binary)
        run('codesign', '--force', '--sign', '-', '--timestamp=none', app)
        run('codesign', '--verify', '--deep', '--strict', app)
        run('ditto', '-c', '-k', '--keepParent', stage, archive)
        # Verify the shipped archive, including executable modes and signatures.
        extracted = Path(tmp) / 'verify'
        run('ditto', '-x', '-k', archive, extracted)
        run('codesign', '--verify', '--deep', '--strict', extracted / 'CosKit/CosKit.app')
        run('codesign', '--verify', '--strict', extracted / 'CosKit/coskit-cli')
        run(extracted / 'CosKit/coskit-cli', '--version')
    with archive.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    (dist / f'SHA256SUMS-{version}-macOS-{arch}.txt').write_text(
        f'{digest}  {archive.name}\n', encoding='ascii',
    )
    print(archive.name)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        if os.environ.get('GITHUB_ACTIONS') == 'true':
            message = str(error).replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
            print(f'::error title=macOS packaging::{message}', flush=True)
        raise
