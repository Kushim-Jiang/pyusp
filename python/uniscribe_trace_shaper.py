"""Uniscribe per-lookup shaping-trace shaper — babelmap ``/api/opentype/shape``
compatible, drop-in for ``babelmap.backend.uniscribe_shaper``.

How the trace is produced (read this before wiring it in):

  * Wine's open-source Uniscribe reimplementation (``wineusp.dll``, bundled in
    the ``pyusp`` wheel) is byte-identical to Microsoft usp10 on every case we
    test — Mongolian golden, Latin, Arabic, Hebrew — same gids **and**
    advances (regression: ``python/test_trace_shaper.py``).
  * Because the engine source is open, we instrument it *inside* the port
    (``tools/wine_usp/port/src/trace.c``): every GSUB lookup application records
    a genuine stage (name + glyph-id run). No HarfBuzz proxy, no adopted
    timeline: ``stages`` is a real cmap -> per-GSUB-lookup -> final trace from
    the same code that produced ``final``.
  * ``backend="usp10"`` returns the authoritative Microsoft usp10 ``final`` and
    adopts the genuine wineusp per-lookup stages only when the two finals are
    equal (asserted per call); ``backend="wineusp"`` (default) returns both from
    the Wine engine alone.

Interface mirrors the babelmap server's uniscribe branch:
    shape_with_uniscribe(path_or_b64, text, *, is_b64, direction, script,
                         language, features, backend)
Returns the engine dict; the server appends features/font_info/glyph_names/svg.

Requires: ``pyusp`` (this repo's wheel, bundles wineusp.dll). ``uharfbuzz`` is
not used anywhere in this shaper — the server provides its own glyph-name
helper (``.opentype``) when this file is dropped into ``babelmap/backend``.
"""

from __future__ import annotations

import base64

try:  # when dropped into babelmap/backend we reuse its face/glyph helpers
    from .opentype import _glyph_names, _load_face
except Exception:  # standalone (this repo): no server helper, no HB -> names {}
    def _load_face(data: bytes, font_index: int = 0):  # kept for module surface
        return None  # shaping never calls this; only the server helper is used

    def _glyph_names(data: bytes, used: set[int]) -> dict:
        return {}


def _gids(run) -> list[int]:
    return [g["g"] for g in (run or [])]


def _engine_dict(usp: dict) -> dict:
    return {
        "upem": int(usp["upem"]),
        "glyph_count": int(usp["glyph_count"]),
        "stages": usp["stages"],
        "final": usp["final"],
        "messages": usp.get("messages", []),
    }


def shape_with_uniscribe(
    path_or_b64: str,
    text: str,
    *,
    is_b64: bool = False,
    direction: str = "auto",
    script: str = "",
    language: str = "",
    features: dict | None = None,
    show_all_lookups: bool = False,
    backend: str = "wineusp",
) -> dict:
    """Shape ``text`` with a Uniscribe engine and return a per-lookup trace.

    ``backend="wineusp"`` (default): ``final`` **and** ``stages`` come from
    Wine's open-source Uniscribe reimplementation bundled in the wheel. The
    port is byte-identical to Microsoft usp10 on our whole corpus, and the
    stages are genuine cmap -> per-GSUB-lookup snapshots recorded inside the
    engine (``trace.c``) — no HarfBuzz anywhere.

    ``backend="usp10"``: ``final`` is the authoritative Microsoft usp10 output;
    ``stages`` is the genuine wineusp per-lookup timeline, adopted only when
    the two finals match exactly (verified per call), otherwise a single-stage
    Uniscribe trace with an explanatory message.
    """
    import pyusp  # this repo's native wheel (Windows-only)

    if is_b64:
        data = base64.b64decode(path_or_b64)
    else:
        with open(path_or_b64, "rb") as f:
            data = f.read()

    kw = dict(
        direction=direction,
        script=script,
        language=language,
        features=features,
    )

    if backend == "usp10":
        # 1) authoritative Microsoft usp10 final.
        usp = pyusp.shape_with_uniscribe(data, text, backend="usp10", **kw)
        usp_final = usp["final"]
        # 2) genuine wineusp per-lookup timeline, adopted when finals match.
        w = pyusp.shape_with_uniscribe(
            data, text, backend="wineusp", trace=True, **kw
        )
        ok = _gids(w["final"]) == _gids(usp_final)
        note = (
            "authoritative usp10 final; per-lookup stages are the genuine wineusp "
            "timeline (Wine's Uniscribe == usp10 here, verified per call)."
            if ok
            else "usp10 and wineusp diverged here — no per-lookup stages; "
            "single-stage Uniscribe trace."
        )
        return {
            "upem": int(usp["upem"]),
            "glyph_count": int(usp["glyph_count"]),
            "engine": "uniscribe",
            "stages": w["stages"] if ok else usp.get("stages", []),
            "final": usp_final,
            "messages": [note] + w.get("messages", []),
        }

    # default: wineusp — genuine per-lookup trace (no HB proxy).
    usp = pyusp.shape_with_uniscribe(
        data, text, backend="wineusp", trace=True, **kw
    )
    d = _engine_dict(usp)
    d["engine"] = "wineusp"
    d["messages"] = [
        "genuine wineusp per-lookup trace (cmap + per-GSUB-lookup + final), "
        "recorded inside Wine's Uniscribe port; wineusp == usp10 on this corpus",
    ] + d["messages"]
    return d
