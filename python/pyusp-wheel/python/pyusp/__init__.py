"""pyusp — Uniscribe (usp10) OpenType shaping tracer.

Returns the babelsoft ``/api/opentype/shape`` engine dict (Crowbar-style
stages), with the shaping done **in-process** by a Uniscribe implementation
loaded through the native PyO3 extension ``pyusp._pyusp``.

Two engines are available (``backend``):

* ``"usp10"`` — Microsoft Uniscribe (Windows only). Prefers the app-local
  ``usp10.dll`` bundled in the wheel (version-pinned, dev/test only) and
  falls back to the system copy. No per-lookup trace is possible (its engine
  runs inside ``gdi32full.dll`` with no public callback) — a single
  whole-run stage is returned.
* ``"wineusp"`` — **Wine's open-source Uniscribe reimplementation**
  (``dlls/gdi32/uniscribe``, LGPL 2.1+), compiled standalone per platform as
  ``wineusp.dll`` / ``libwineusp.dylib`` / ``libwineusp.so``. It is genuinely
  redistributable and exposes a real per-lookup trace: when ``trace=True`` the
  returned ``stages`` are the actual cmap → per-GSUB-lookup → final snapshots
  recorded inside the shaper (see ``NOTICE-wineusp.md``). On non-Windows the
  engine shapes in "bytes mode" (raw font bytes + NULL hdc) — identical
  results to Windows (cross-platform parity is CI-tested).
* ``"textshaping"`` — **Windows-only**: drive the *system* Microsoft Uniscribe
  (``usp10.dll`` → ``TextShaping.dll``) with a real GDI font and capture a
  **native per-application trace** straight from the Microsoft engine: every
  time it applies an OT operation over the glyph run the run state is
  recorded (``trace=True``; stages are named ``textshaping apply N``). This
  has no lookup *names* (the engine does not expose them) — it is the genuine
  engine's per-op glyph-state timeline, and is only enabled on validated
  ``TextShaping.dll`` builds (Win11 24H2 / 10.0.26100 x64); anything else
  raises a clear error (see ``NOTICE-native-trace.md``).
* ``"auto"`` — ``wineusp`` when bundled, else system ``usp10``.

Cross-platform: ``wineusp`` runs on Windows, Linux and macOS. ``usp10`` and
``textshaping`` are Windows-only (they drive Microsoft Uniscribe).
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

try:
    from . import _pyusp
except ImportError as e:  # pragma: no cover - only when the wheel is broken
    raise ImportError(
        "pyusp native module missing — the wheel is corrupt, or it was built "
        "for a different Python/ABI. Reinstall the platform wheel."
    ) from e

# Native engine version (kept in lockstep with pyproject.toml/Cargo.toml).
# Check it when a fix seems to have no effect — a stale install in
# site-packages is the usual cause.
__version__ = _pyusp.__version__

_DLL = Path(__file__).with_name("usp10.dll")
if os.name == "nt":
    _WINE_NAME = "wineusp.dll"
    _SYSTEM_USP10 = "usp10.dll"
elif sys.platform == "darwin":
    _WINE_NAME = "libwineusp.dylib"
    _SYSTEM_USP10 = "libwineusp.dylib"  # never used; usp10 backend is nt-only
else:
    _WINE_NAME = "libwineusp.so"
    _SYSTEM_USP10 = "libwineusp.so"
_WINE_DLL = Path(__file__).with_name(_WINE_NAME)


def _require_dll() -> str:
    # Bundled app-local copy first (version-pinned); else let the engine load
    # the system usp10 by name. (usp10 backend is Windows-only.)
    if _DLL.exists():
        return str(_DLL)
    return _SYSTEM_USP10


def _require_wineusp() -> str:
    if not _WINE_DLL.exists():
        raise FileNotFoundError(
            f"{_WINE_NAME} is not bundled in this wheel (backend='wineusp' needs "
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

    ``backend`` selects the engine (``"usp10"`` / ``"wineusp"`` /
    ``"textshaping"`` / ``"auto"``, see module docstring). ``trace=True``
    requests a real trace timeline:

    * ``backend="wineusp"`` → genuine per-lookup stages (cross-platform),
    * ``backend="textshaping"`` (Windows) → genuine native per-application
      stages from the system Microsoft TextShaping engine (errors if the local
      ``TextShaping.dll`` is outside the validated set),
    * ``backend="usp10"`` → no trace hook; a single whole-run stage.

    ``features`` uses the same ``{tag: bool|int}`` convention as the HarfBuzz /
    harfrust engines (``True``/``1`` → ``+tag``, ``False``/``0`` → ``-tag``,
    any other int → ``tag=N``, i.e. the ``lParameter`` alternate index), or a
    ready HarfBuzz-style string (``"kern"``, ``"+kern"``, ``"-kern"``,
    ``"kern=0"``, ``"aalt=2"``). An explicit value overrides a ``+``/``-``
    prefix exactly like HarfBuzz (``+kern=0`` is *off*), and a malformed item
    raises instead of being silently dropped.

    **Non-empty ``features`` require ``backend="usp10"`` or
    ``backend="textshaping"``.** The bundled Wine port does not apply feature
    records at all — its ``ScriptShapeOpenType``/``ScriptPlaceOpenType`` log
    ``FIXME("Ranges not supported yet")`` and drop them — so passing features to
    it raises ``RuntimeError`` rather than returning a shape that ignored them.

    Note also that the list **replaces** the script's default GSUB feature set
    for the run instead of adding to it (system usp10 semantics for the range
    properties): ``features={"liga": True}`` applies exactly ``liga``.

    Note Uniscribe only recognises legacy OpenType script tags (e.g. ``deva``,
    not ``dev2``), so modern Indic shaping that depends on newer tags is not
    available — matching what old Win32 apps that use usp10 actually do.
    """
    if not isinstance(data, (bytes, bytearray)):
        raise TypeError("data must be font file bytes")
    if backend == "textshaping":
        if os.name != "nt":
            raise ValueError(
                "backend='textshaping' is Windows-only (it drives the system "
                "TextShaping engine via usp10); use backend='wineusp' for the "
                "cross-platform per-lookup trace."
            )
        dll, trace_on, textshaping = None, trace, bool(trace)
    else:
        dll, trace_on = _resolve_backend(backend, trace)
        textshaping = False
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
        textshaping=textshaping,
    )
    return json.loads(out)


__all__ = ["shape_with_uniscribe", "_require_dll", "_require_wineusp"]
