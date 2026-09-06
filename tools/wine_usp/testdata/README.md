# Test font for the cross-platform shaping parity tests

`NotoSansMongolian-Regular.ttf` is shaped in **bytes mode** on every OS
(Linux/macOS via `libwineusp.so/.dylib` built with `port/build_posix.sh`,
Windows via the MinGW-built `wineusp.dll`) and the per-lookup traces must
match stage-for-stage against the committed Windows reference.

## Files

- `NotoSansMongolian-Regular.ttf` — Google Noto Sans Mongolian (Mongolian
  script, has ccmp/rlig/rclt/calt GSUB lookups → a rich per-lookup trace).
  License: **SIL Open Font License 1.1** (`OFL.txt`, bundled).
- `OFL.txt` — the SIL OFL 1.1 text for the font above.
- `noto-saikhan-windows-golden.json` — the **Windows reference** trace
  (pyusp `--wine-bytes`, wineusp.dll) for text "ᠰᠠᠢᠬᠠᠨ" (Mongolian
  "saikhan"): 161 per-lookup stages (cmap → ccmp → rlig → rclt×78 → calt×78
  → final) and the final glyph run `[1409, 10, 1382, 1396, 1490, 10, 76]`.
  Linux/macOS bytes-mode shaping must reproduce these stages exactly.

## Regenerating the golden (Windows)

```
pyusp.exe --font tools\wine_usp\testdata\NotoSansMongolian-Regular.ttf ^
          --text <saikhan> --usp10 tools\wine_usp\port\wineusp.dll ^
          --trace --wine-bytes
```
