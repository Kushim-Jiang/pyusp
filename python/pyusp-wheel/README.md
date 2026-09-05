# pyusp — Uniscribe (usp10) OpenType shaping tracer (Windows-only wheel)

PyO3 abi3 wheel (`pyusp-…-py3-none-win_amd64.whl`) wrapping the `pyusp` Rust
engine. Shapes in-process via usp10.dll (Uniscribe) and returns the babelsoft
`/api/opentype/shape` engine dict.

```python
from pyusp import shape_with_uniscribe
res = shape_with_uniscribe(font_bytes, text)          # script auto-detected
res = shape_with_uniscribe(font_bytes, text, script="mong")
```

The wheel bundles an app-local copy of `usp10.dll` (copied from the OS at
build time by `python/build_wheel.ps1`) which the engine prefers; delete that
file from the wheel (or build without it) to fall back to the system
`usp10.dll`. See `python/pyusp/NOTICE.md`.

**Scope note**: usp10 only recognises legacy OpenType script tags. Fonts that
use only modern tags (e.g. Devanagari faces exposing `dev2`, not `deva`) are
not shaped by Uniscribe at all — this matches what old Win32 apps that rely on
usp10 actually experience.
