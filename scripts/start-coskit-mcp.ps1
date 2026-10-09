param(
    [Parameter(Mandatory=$true)][string]$Workspace,
    [int]$Port = 50507,
    [string]$BinaryDir = ''
)
$ErrorActionPreference = 'Stop'
if ($Port -lt 1024 -or $Port -gt 65535) { throw 'Port must be in 1024..65535' }
$workspacePath = (Resolve-Path -LiteralPath $Workspace).Path
if (!(Test-Path -LiteralPath $workspacePath -PathType Container)) { throw 'Workspace must be a directory' }
if (!$BinaryDir) {
    $packageDir = Split-Path -Parent $PSScriptRoot
    if (Test-Path -LiteralPath (Join-Path $packageDir 'coskit.exe')) { $BinaryDir = $packageDir }
    else { $BinaryDir = Join-Path $packageDir 'native/target-release/release' }
}
$app = Join-Path $BinaryDir 'coskit.exe'
$cli = Join-Path $BinaryDir 'coskit-cli.exe'
if (!(Test-Path -LiteralPath $app)) { $app = Join-Path $BinaryDir 'photocraft.exe'; $cli = Join-Path $BinaryDir 'photocraft-cli.exe' }
if (!(Test-Path -LiteralPath $app) -or !(Test-Path -LiteralPath $cli)) { throw 'CosKit desktop and CLI binaries were not found' }
$privateDir = Join-Path $workspacePath '.coskit-mcp'
New-Item -ItemType Directory -Force -Path $privateDir | Out-Null
$token = Join-Path $privateDir 'control.token'
$connection = [Net.Sockets.TcpClient]::new()
try { $connection.Connect('127.0.0.1',$Port); $running = $true } catch { $running = $false } finally { $connection.Dispose() }
if (!$running) {
    # Keep paths as separate quoted argv values for Start-Process on Windows.
    foreach ($path in @($workspacePath,$token)) { if ($path.Contains('"')) { throw 'A path cannot contain a quote' } }
    $appArgs = @('--control',"$Port",'--control-token-file',('"'+$token+'"'),'--automation-read-root',('"'+$workspacePath+'"'),'--automation-write-root',('"'+$workspacePath+'"'))
    $process = Start-Process -FilePath $app -ArgumentList $appArgs -WindowStyle Hidden -RedirectStandardError (Join-Path $privateDir 'desktop.log') -PassThru
    $ready = $false
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        if ($process.HasExited) { throw 'CosKit exited; inspect .coskit-mcp/desktop.log' }
        $probe = [Net.Sockets.TcpClient]::new()
        try { $probe.Connect('127.0.0.1',$Port); $ready = Test-Path -LiteralPath $token } catch {} finally { $probe.Dispose() }
        if ($ready) { break }
        Start-Sleep -Milliseconds 100
    }
    if (!$ready) { throw 'CosKit control server did not become ready' }
}
if (!(Test-Path -LiteralPath $token)) { throw 'Port is occupied by another app/session; choose a different port' }
# The CLI authenticates before any method is dispatched. stdout is exclusively MCP.
& $cli mcp --bridge "127.0.0.1:$Port" --control-token-file $token
exit $LASTEXITCODE
