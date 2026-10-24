param(
    [switch]$Build,
    [Parameter(Position = 0)][string]$File
)
$ErrorActionPreference = 'Stop'
$taskNativeExe = Join-Path $PSScriptRoot 'target\release\npzsee.exe'
if ($Build -or !(Test-Path -LiteralPath $taskNativeExe)) {
    $taskPreviousTarget = $env:CARGO_TARGET_DIR
    Push-Location -LiteralPath $PSScriptRoot
    try {
        $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'target'
        & cargo build --release --locked --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml')
        if ($LASTEXITCODE -ne 0) { throw 'Native build failed.' }
    }
    finally {
        $env:CARGO_TARGET_DIR = $taskPreviousTarget
        Pop-Location
    }
}
if ($File) { & $taskNativeExe $File } else { & $taskNativeExe }
