param([string]$BuildDir = 'native/target/release', [string]$Makensis = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$version = (Get-Content -LiteralPath package.json -Raw | ConvertFrom-Json).version
$buildPath = [IO.Path]::GetFullPath((Join-Path $projectRoot $BuildDir))
if (!(Test-Path -LiteralPath (Join-Path $buildPath 'photocraft.exe'))) { throw 'Build the native app first (npm run build)' }
$delivery = Join-Path $projectRoot 'dist'
$stage = Join-Path $delivery "CosKit_${version}_x64_portable"
$licenses = Join-Path $stage 'licenses'
New-Item -ItemType Directory -Force -Path $stage, $licenses | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'scripts') | Out-Null
Copy-Item -LiteralPath (Join-Path $buildPath 'photocraft.exe') -Destination (Join-Path $stage 'coskit.exe')
Copy-Item -LiteralPath (Join-Path $buildPath 'photocraft-cli.exe') -Destination (Join-Path $stage 'coskit-cli.exe')
Copy-Item -LiteralPath ACKNOWLEDGEMENTS.md -Destination $stage
Copy-Item -LiteralPath docs/p0-performance.md,docs/p1-harness-and-review.md -Destination $stage
Copy-Item -LiteralPath docs/getting-started-v1.md,docs/release-1.0.0.md -Destination $stage
Copy-Item -LiteralPath LICENSE -Destination (Join-Path $licenses 'CosKit-MIT.txt')
Copy-Item -LiteralPath native/LICENSE-MIT -Destination (Join-Path $licenses 'PhotoCraft-MIT.txt')
Copy-Item -LiteralPath native/LICENSE-APACHE -Destination (Join-Path $licenses 'PhotoCraft-Apache.txt')
Copy-Item -LiteralPath native/NOTICE -Destination (Join-Path $licenses 'PhotoCraft-NOTICE.txt')
Copy-Item -LiteralPath native/ATTRIBUTION.md -Destination $licenses
Copy-Item -LiteralPath THIRD_PARTY_NOTICES.md -Destination $licenses
Copy-Item -LiteralPath native/assets/icons/LICENSE-lucide.txt -Destination $licenses
Copy-Item -LiteralPath native/assets/icons/LICENSE-noun-magnetic-lasso.txt -Destination $licenses
Copy-Item -LiteralPath native/assets/dict/LICENSE-SCOWL.txt -Destination $licenses
Copy-Item -LiteralPath native/crates/ui-egui/src/i18n/LICENSE-translations.txt -Destination $licenses
Copy-Item -LiteralPath native/assets/app-icon/LICENSE.txt -Destination (Join-Path $licenses 'PhotoCraft-icon-LICENSE.txt')
Copy-Item -LiteralPath native/assets/brushes/deevad-2023/LICENSE-CC0.txt -Destination (Join-Path $licenses 'Deevad-CC0.txt')
Copy-Item -LiteralPath native/assets/brushes/deevad-2023/README.md -Destination (Join-Path $licenses 'Deevad-brushes.md')
Copy-Item -LiteralPath scripts/start-coskit-mcp.ps1 -Destination (Join-Path $stage 'scripts')
Copy-Item -LiteralPath docs/coskit-mcp.md,docs/cosplay-validation.md,docs/autonomous-retouch-harness.md,docs/ckpipe-project-format.md,docs/beta4-validation.md,docs/fluent-workspace.md -Destination $stage
Copy-Item -LiteralPath docs/native-studio-guide.md,docs/native-release.md -Destination $stage
Set-Content -LiteralPath (Join-Path $stage 'portable.txt') -Value 'CosKit portable mode. Settings and recovery live in CosKitData.' -Encoding utf8
$allowed = @('coskit.exe','coskit-cli.exe','native-studio-guide.md','native-release.md','portable.txt','licenses/CosKit-MIT.txt','licenses/PhotoCraft-MIT.txt','licenses/PhotoCraft-Apache.txt','licenses/PhotoCraft-NOTICE.txt','licenses/ATTRIBUTION.md','licenses/THIRD_PARTY_NOTICES.md','licenses/LICENSE-lucide.txt','licenses/LICENSE-noun-magnetic-lasso.txt','licenses/LICENSE-SCOWL.txt','licenses/LICENSE-translations.txt','licenses/PhotoCraft-icon-LICENSE.txt')
$allowed += @('ACKNOWLEDGEMENTS.md','p0-performance.md','p1-harness-and-review.md')
$allowed += @('getting-started-v1.md','release-1.0.0.md')
$allowed += @('fluent-workspace.md','autonomous-retouch-harness.md','ckpipe-project-format.md','beta4-validation.md','coskit-mcp.md','cosplay-validation.md','licenses/Deevad-CC0.txt','licenses/Deevad-brushes.md','scripts/start-coskit-mcp.ps1')
foreach ($file in Get-ChildItem -LiteralPath $stage -File -Force -Recurse) {
    $relative = [IO.Path]::GetRelativePath($stage,$file.FullName).Replace('\','/')
    if ($relative -notin $allowed) { throw "Unexpected file in package staging: $relative" }
}
Compress-Archive -LiteralPath $stage -DestinationPath (Join-Path $delivery "CosKit_${version}_x64_portable.zip") -Force
if (!$Makensis) {
    $found = Get-Command makensis -ErrorAction SilentlyContinue
    if ($found) { $Makensis = $found.Source }
    else { $Makensis = Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe' }
}
if (!(Test-Path -LiteralPath $Makensis)) { throw 'NSIS makensis not found; install NSIS or pass -Makensis' }
& $Makensis /V2 "/DVERSION=$version" "/DSTAGE=$stage" "/DOUTFILE=$delivery/CosKit_${version}_x64-setup.exe" scripts/native-installer.nsi
if ($LASTEXITCODE -ne 0) { throw 'NSIS build failed' }
Get-ChildItem -LiteralPath $delivery -File | Where-Object { $_.Name -like "CosKit_${version}_*" -and $_.Extension -in '.exe','.zip' } | ForEach-Object {
    $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
    '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), $_.Name
} | Set-Content -LiteralPath (Join-Path $delivery "SHA256SUMS-${version}.txt") -Encoding ascii
Get-ChildItem -LiteralPath $delivery -File | Where-Object { $_.Name -like "*${version}*" } | Select-Object Name,Length
