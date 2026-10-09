param([Parameter(Mandatory=$true)][string]$Directory)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Speech
$narrator = New-Object System.Speech.Synthesis.SpeechSynthesizer
$narrator.SelectVoice('Microsoft Huihui Desktop')
$narrator.Rate=1
try {
    $scenes=Get-Content -LiteralPath (Join-Path $Directory 'storyboard.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($scene in $scenes) {
        if ($scene.id -notmatch '^[a-z]+$') { throw 'Invalid scene identifier' }
        $narrator.SetOutputToWaveFile((Join-Path $Directory ($scene.id+'.wav')))
        $narrator.Speak([string]$scene.zh)
        $narrator.SetOutputToNull()
    }
} finally { $narrator.Dispose() }
