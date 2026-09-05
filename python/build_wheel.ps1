# Build the pyusp Windows wheel (PyO3 abi3 >= 3.9, py3-none-win_amd64).
#
# Steps:
#   1. Copy the current OS usp10.dll into the package (git-ignored).
#   2. maturin build (abi3 -> py3-none-win_amd64).
#
# Run from this directory with the babelsoft venv python (has maturin) or any
# python with maturin installed:
#   & 'D:\Github\babelsoft-py\.venv\Scripts\python.exe' ..\..\python\build_wheel.ps1

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path   # python/
$wheel = Join-Path $here 'pyusp-wheel'
$pkg = Join-Path $wheel 'python\pyusp'

$src = "$env:WINDIR\System32\usp10.dll"
if (-not (Test-Path $src)) { throw "system usp10.dll not found: $src" }
Copy-Item $src (Join-Path $pkg 'usp10.dll') -Force
Write-Host "bundled $((Get-Item (Join-Path $pkg 'usp10.dll')).VersionInfo.FileVersion) usp10.dll"

Push-Location $wheel
try {
    python -m maturin build --release --interpreter "$env:PYUSP_PY" -o dist
} finally {
    Pop-Location
}
