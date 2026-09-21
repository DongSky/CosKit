#!/usr/bin/env python3
"""Configure Tauri Android release signing for CI (or a local COSKIT_ANDROID_* build).

This repo gitignores `build_android.sh` and `src-tauri/gen/`. CI therefore:
  1. runs `npx tauri android init`
  2. decodes the keystore
  3. writes `src-tauri/gen/android/keystore.properties`
  4. patches the generated `app/build.gradle.kts` so the release buildType
     uses a `signingConfigs.release` block (Tauri CLI 2.10 templates omit this).

Never print secret values. Never write the keystore into the git worktree
except under the generated (gitignored) Android project / runner temp dir.
"""

from __future__ import annotations

import argparse
import base64
import os
import sys
from pathlib import Path

SIGNING_CONFIGS_BLOCK = """    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            }
            keyAlias = keystoreProperties.getProperty("keyAlias")
            keyPassword = keystoreProperties.getProperty(
                "keyPassword",
                keystoreProperties.getProperty("password")
            )
            storePassword = keystoreProperties.getProperty(
                "storePassword",
                keystoreProperties.getProperty("password")
            )
            storeFile = file(keystoreProperties.getProperty("storeFile"))
        }
    }

"""

SIGNING_CONFIG_LINE = "            signingConfig = signingConfigs.getByName(\"release\")\n"
FILE_INPUT_STREAM_IMPORT = "import java.io.FileInputStream\n"


def decode_keystore(b64_value: str) -> bytes:
    compact = "".join(b64_value.split())
    if not compact:
        raise SystemExit("ANDROID_KEYSTORE_BASE64 is empty after stripping whitespace")
    try:
        data = base64.b64decode(compact, validate=False)
    except Exception as exc:  # noqa: BLE001
        raise SystemExit(f"Failed to base64-decode ANDROID_KEYSTORE_BASE64: {exc}") from exc
    if not data:
        raise SystemExit("Decoded ANDROID_KEYSTORE_BASE64 is empty")
    return data


def write_keystore_properties(
    android_root: Path,
    store_file: Path,
    key_alias: str,
    store_password: str,
    key_password: str,
) -> Path:
    props = android_root / "keystore.properties"
    # Official Tauri docs use a single `password=` field; CosKit's local
    # build also accepts separate store/key passwords. Write both forms.
    body = (
        f"keyAlias={key_alias}\n"
        f"password={key_password}\n"
        f"keyPassword={key_password}\n"
        f"storePassword={store_password}\n"
        f"storeFile={store_file}\n"
    )
    props.write_text(body, encoding="utf-8")
    return props


def _insert_import(text: str, import_line: str) -> str:
    if "FileInputStream" in text:
        return text
    lines = text.splitlines(keepends=True)
    last_import = -1
    for i, line in enumerate(lines):
        if line.startswith("import "):
            last_import = i
    if last_import >= 0:
        lines.insert(last_import + 1, import_line)
        return "".join(lines)
    return import_line + text


def _insert_signing_configs(text: str) -> str:
    if "signingConfigs" in text:
        return text
    needle = "    buildTypes {"
    idx = text.find(needle)
    if idx < 0:
        raise SystemExit(
            "Could not find `    buildTypes {` in app/build.gradle.kts; "
            "refusing to guess a signingConfigs insertion point"
        )
    return text[:idx] + SIGNING_CONFIGS_BLOCK + text[idx:]


def _attach_release_signing_config(text: str) -> str:
    if "signingConfig = signingConfigs.getByName(\"release\")" in text:
        return text
    marker = '        getByName("release") {'
    idx = text.find(marker)
    if idx < 0:
        raise SystemExit(
            'Could not find `getByName("release") {` in app/build.gradle.kts'
        )
    brace = idx + len(marker)
    # Keep the original newline after `{`, then insert the signingConfig line.
    if brace < len(text) and text[brace] == "\n":
        brace += 1
    return text[:brace] + SIGNING_CONFIG_LINE + text[brace:]


def patch_app_gradle(gradle_path: Path) -> None:
    original = gradle_path.read_text(encoding="utf-8")
    updated = _insert_import(original, FILE_INPUT_STREAM_IMPORT)
    updated = _insert_signing_configs(updated)
    updated = _attach_release_signing_config(updated)
    if updated != original:
        gradle_path.write_text(updated, encoding="utf-8")


def require_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise SystemExit(f"Missing required environment variable: {name}")
    return value


def configure_from_env(repo_root: Path) -> None:
    android_root = repo_root / "src-tauri" / "gen" / "android"
    gradle_path = android_root / "app" / "build.gradle.kts"
    if not gradle_path.is_file():
        raise SystemExit(
            f"{gradle_path} not found. Run `npx tauri android init` first."
        )

    keystore_b64 = require_env("ANDROID_KEYSTORE_BASE64")
    store_password = require_env("ANDROID_KEYSTORE_PASSWORD")
    key_alias = require_env("ANDROID_KEY_ALIAS")
    key_password = require_env("ANDROID_KEY_PASSWORD")

    dest_dir = Path(os.environ.get("RUNNER_TEMP") or os.environ.get("TMPDIR") or "/tmp")
    dest_dir.mkdir(parents=True, exist_ok=True)
    store_file = Path(
        os.environ.get("COSKIT_ANDROID_KEYSTORE") or (dest_dir / "coskit-release.keystore")
    )
    store_file.parent.mkdir(parents=True, exist_ok=True)
    store_file.write_bytes(decode_keystore(keystore_b64))
    os.chmod(store_file, 0o600)

    write_keystore_properties(
        android_root,
        store_file,
        key_alias=key_alias,
        store_password=store_password,
        key_password=key_password,
    )
    patch_app_gradle(gradle_path)
    print(f"Wrote signing config for {gradle_path}")
    print(f"Keystore path: {store_file}")


def _self_test() -> None:
    sample = """import java.util.Properties

plugins {
    id("com.android.application")
}

android {
    compileSdk = 36
    buildTypes {
        getByName("debug") {
            isDebuggable = true
        }
        getByName("release") {
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
}
"""
    patched = _insert_import(sample, FILE_INPUT_STREAM_IMPORT)
    patched = _insert_signing_configs(patched)
    patched = _attach_release_signing_config(patched)
    assert "import java.io.FileInputStream" in patched
    assert "signingConfigs" in patched
    assert 'signingConfig = signingConfigs.getByName("release")' in patched
    # Idempotent
    again = _insert_import(patched, FILE_INPUT_STREAM_IMPORT)
    again = _insert_signing_configs(again)
    again = _attach_release_signing_config(again)
    assert again == patched
    # Keystore decode
    raw = base64.b64encode(b"dummy-keystore").decode("ascii")
    assert decode_keystore(raw + "\n") == b"dummy-keystore"
    print("self-test ok")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        default=".",
        help="CosKit repository root (default: cwd)",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run the gradle-patcher unit checks and exit",
    )
    args = parser.parse_args()
    if args.self_test:
        _self_test()
        return 0
    configure_from_env(Path(args.repo_root).resolve())
    return 0


if __name__ == "__main__":
    sys.exit(main())
