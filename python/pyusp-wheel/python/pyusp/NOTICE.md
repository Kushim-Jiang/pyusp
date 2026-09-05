# usp10.dll notice

The file `usp10.dll` bundled in this wheel is the Microsoft Windows system
component `usp10.dll` (Uniscribe), copied verbatim from the build machine's
`C:\Windows\System32\usp10.dll` for app-local loading. Version recorded at
build time: see `pyusp.dll_version` / `python/build.ps1` output.

Redistribution notice: usp10.dll is a Windows operating-system component and
is not a separately-licensed redistributable. Bundling it app-locally is
intended for development/testing on the matching Windows build only. For any
other use, prefer the engine's system-dll fallback (delete this file from the
wheel, or build without it), which loads `usp10.dll` from the OS directly.
