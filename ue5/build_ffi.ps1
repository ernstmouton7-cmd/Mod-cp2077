# Builds the Gridlock engine library and copies it into the Unreal plugin.
# Run from anywhere:  powershell -ExecutionPolicy Bypass -File ue5\build_ffi.ps1
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$engine = Join-Path $repo "engine"
Push-Location $engine
try {
    cargo build --release -p gridlock-ffi
} finally {
    Pop-Location
}
$out = Join-Path $engine "target\release"
$dest = Join-Path $PSScriptRoot "Gridlock\Source\ThirdParty\GridlockFFI\lib\Win64"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item (Join-Path $out "gridlock_ffi.dll") $dest -Force
Copy-Item (Join-Path $out "gridlock_ffi.dll.lib") $dest -Force
Copy-Item (Join-Path $engine "crates\ffi\include\gridlock.h") (Join-Path $PSScriptRoot "Gridlock\Source\ThirdParty\GridlockFFI\include") -Force
Write-Host "Copied gridlock_ffi.dll + .dll.lib to $dest"
