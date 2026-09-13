# Build the pyusp Windows wheel (PyO3 abi3 >= 3.9, py3-none-win_amd64).
#
# Steps:
#   1. Copy the current OS usp10.dll into the package (git-ignored).
#   2. maturin build (abi3 -> py3-none-win_amd64).
#
# Run from this directory. The interpreter needs maturin: set $env:PYUSP_PY to
# one that has it, otherwise the `python` on PATH is used. NOTE: the babelsoft
# venv (which runs the tests) does NOT have maturin — the system Python 3.13
# does, e.g.
#   $env:PYUSP_PY = 'C:\Softwares\Python\py313\python.exe'
#   powershell -ExecutionPolicy Bypass -File ..\..\python\build_wheel.ps1
# (then reinstall the wheel into the test venv:
#   & 'D:\Github\babelsoft-py\.venv\Scripts\python.exe' -m pip install `
#       --force-reinstall --no-deps dist\pyusp-<ver>-cp39-abi3-win_amd64.whl )

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path   # python/
$wheel = Join-Path $here 'pyusp-wheel'
$pkg = Join-Path $wheel 'python\pyusp'

# Interpreter used to run maturin (and reported to it via --interpreter).
$py = $env:PYUSP_PY
if (-not $py) {
    $cmd = Get-Command python -ErrorAction SilentlyContinue
    if (-not $cmd) {
        throw "no 'python' on PATH — set `$env:PYUSP_PY to an interpreter that has maturin"
    }
    $py = $cmd.Source
}
& $py -m maturin --version | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "maturin is not available for $py — set `$env:PYUSP_PY to an interpreter that has it (the babelsoft venv has none)."
}

$src = "$env:WINDIR\System32\usp10.dll"
if (-not (Test-Path $src)) { throw "system usp10.dll not found: $src" }
Copy-Item $src (Join-Path $pkg 'usp10.dll') -Force
Write-Host "bundled $((Get-Item (Join-Path $pkg 'usp10.dll')).VersionInfo.FileVersion) usp10.dll"

Push-Location $wheel
try {
    & $py -m maturin build --release --interpreter $py -o dist
} finally {
    Pop-Location
}
