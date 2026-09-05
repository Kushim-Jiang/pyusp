"""pyusp — Uniscribe (usp10) OpenType shaping tracer (Windows only).

Returns the babelsoft ``/api/opentype/shape`` engine dict (Crowbar-style
stages), with the shaping done **in-process** by a Uniscribe implementation
loaded through the native PyO3 extension ``pyusp._pyusp``.

Two engines are available (``backend``):

* ``"usp10"`` — Microsoft Uniscribe. Prefers the app-local ``usp10.dll``
  bundled in this wheel (version-pinned, dev/test only) and falls back to
  the system copy. No per-lookup trace is possible (its engine runs inside
  ``gdi32full.dll`` with no public callback) — a single whole-run stage is
  returned.
* ``"wineusp"`` — **Wine's open-source Uniscribe reimplementation**
  (``dlls/gdi32/uniscribe``, LGPL 2.1+), compiled standalone as the bundled
  ``wineusp.dll``. It is genuinely redistributable and exposes a real
  per-lookup trace: when ``trace=True`` the returned ``stages`` are the
  actual cmap → per-GSUB-lookup → final snapshots recorded inside the
  shaper (see ``NOTICE-wineusp.md``).
* ``"auto"`` — ``wineusp`` when bundled, else system ``usp10``.

This package is Windows-only. Importing on any other OS raises OSError.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

if os.name != "nt":
    raise OSError(
        "pyusp is Windows-only (it drives Microsoft Uniscribe / usp10.dll)."
    )

try:
    from . import _pyusp
except ImportError as e:  # pragma: no cover - only when the wheel is broken
    raise ImportError(
        "pyusp native module missing — the wheel is corrupt, or it was built "
        "for a different Python/ABI. Reinstall the Windows wheel."
    ) from e

_DLL = Path(__file__).with_name("usp10.dll")
_WINE_DLL = Path(__file__).with_name("wineusp.dll")


def _require_dll() -> str:
    # Bundled app-local copy first (version-pinned); else let the engine load
    # the system usp10.dll by name.
    if _DLL.exists():
        return str(_DLL)
    return "usp10.dll"


def _require_wineusp() -> str:
    if not _WINE_DLL.exists():
        raise FileNotFoundError(
            "wineusp.dll is not bundled in this wheel (backend='wineusp' needs "
            "a build that includes the Wine Uniscribe port)."
        )
    return str(_WINE_DLL)


def _resolve_backend(backend: str, trace: bool) -> tuple[str | None, bool]:
    """Return (dll_path, trace_on). System usp10 cannot trace."""
    if backend not in ("auto", "wineusp", "usp10"):
        raise ValueError("backend must be one of 'auto', 'usp10', 'wineusp'")
    if backend == "usp10":
        return _require_dll(), False
    if backend == "wineusp":
        return _require_wineusp(), trace
    # auto: prefer the open-source, redistributable engine
    if _WINE_DLL.exists():
        return _require_wineusp(), trace
    return _require_dll(), False


def _features_to_str(features) -> str | None:
    """Normalise ``{tag: bool|int}`` (or a ready string) to ``+tag,-tag,tag=N``."""
    if isinstance(features, str):
        s = features.strip()
        return s or None
    if not features:
        return None
    parts: list[str] = []
    for tag, val in features.items():
        tag = str(tag)
        if val is True or val == 1:
            parts.append(f"+{tag}")
        elif val is False or val == 0:
            parts.append(f"-{tag}")
        else:
            parts.append(f"{tag}={val}")
    return ",".join(parts) if parts else None


def shape_with_uniscribe(
    data: bytes,
    text: str,
    *,
    direction: str = "auto",
    script: str = "",
    language: str = "",
    features: dict | None = None,
    backend: str = "auto",
    trace: bool = False,
) -> dict:
    """Shape ``text`` with a Uniscribe engine and return the babelsoft
    ``/api/opentype/shape`` dict.

    ``backend`` selects the engine (``"usp10"`` / ``"wineusp"`` / ``"auto"``,
    see module docstring). ``trace=True`` requests the genuine per-lookup
    timeline — only meaningful for ``backend="wineusp"`` (system usp10 has no
    trace hook; it silently returns the single-stage form).

    ``features`` uses the same ``{tag: bool}`` convention as the HarfBuzz /
    harfrust engines. Note Uniscribe only recognises legacy OpenType script
    tags (e.g. ``deva``, not ``dev2``), so modern Indic shaping that depends
    on newer tags is not available — matching what old Win32 apps that use
    usp10 actually do.
    """
    if not isinstance(data, (bytes, bytearray)):
        raise TypeError("data must be font file bytes")
    dll, trace_on = _resolve_backend(backend, trace)
    fs = _features_to_str(features)
    out = _pyusp.shape_json(
        bytes(data),
        text,
        script=script,
        language=language,
        direction=direction,
        features=fs,
        usp10=dll,
        trace=trace_on,
    )
    return json.loads(out)


__all__ = ["shape_with_uniscribe", "_require_dll", "_require_wineusp"]
