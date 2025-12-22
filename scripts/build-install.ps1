$ErrorActionPreference = "Stop"

cargo build --release

$src = Join-Path $PSScriptRoot "..\target\release\ghc.exe"
$dstDir = Join-Path $env:USERPROFILE "bin"
$dst = Join-Path $dstDir "ghc.exe"

New-Item -ItemType Directory -Force -Path $dstDir | Out-Null
Copy-Item -Force $src $dst

Write-Host "Installed ghc -> $dst"
