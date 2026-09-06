# pyusp — Uniscribe (usp10) OpenType shaping tracer (Windows / Linux / macOS wheel)

PyO3 abi3 wheel (`pyusp-…-py3-none-win_amd64.whl`) wrapping the `pyusp` Rust
engine. Shapes in-process via a Uniscribe implementation and returns the
babelsoft `/api/opentype/shape` engine dict.

```python
from pyusp import shape_with_uniscribe
res = shape_with_uniscribe(font_bytes, text)          # script auto-detected
res = shape_with_uniscribe(font_bytes, text, script="mong")
```

**Engines.** The wheel ships `wineusp.dll`, Wine's open-source Uniscribe port
(LGPL), which also provides a real per-lookup trace (`trace=True`). It drives
the OS `usp10.dll` (Microsoft Uniscribe) as the authoritative reference — the
Microsoft DLL is *not* bundled in the published wheel (it is an OS component
and not redistributable); the engine loads it from the system at runtime. A
dev/test-only build (`python/build_wheel.ps1`) may additionally pin an
app-local copy of `usp10.dll`; see `python/pyusp/NOTICE.md` and
`python/pyusp/NOTICE-wineusp.md`.

**Scope note**: usp10 only recognises legacy OpenType script tags. Fonts that
use only modern tags (e.g. Devanagari faces exposing `dev2`, not `deva`) are
not shaped by Uniscribe at all — this matches what old Win32 apps that rely on
usp10 actually experience.
