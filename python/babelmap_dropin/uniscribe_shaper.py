"""Uniscribe per-lookup shaping-trace shaper — babelmap ``/api/opentype/shape``
compatible, drop-in for ``babelmap.backend.uniscribe_shaper``.

How the trace is produced (read this before wiring it in):

  * Uniscribe (usp10) has **no public per-lookup / buffer-message callback**,
    and on Win11 24H2 its internal engine runs inside ``gdi32full.dll`` with
    stripped PDB symbols (no way to observe per-lookup state from the public
    API). A native usp10 per-lookup timeline is therefore not obtainable.
  * We verified (``python/compare_pyusp.py``) that for every script usp10 can
    shape, Uniscribe's **final** glyph run equals HarfBuzz's exactly (gids +
    advances) — Mongolian golden, Latin, Arabic, Hebrew.
  * So this shaper returns:
      - ``final`` = the **real Uniscribe** output (authoritative, from the
        bundled/system usp10 via the ``pyusp`` wheel);
      - ``stages`` = the **per-lookup HarfBuzz timeline** (the OT-conformant
        decomposition that provably leads to the identical final), only when
        the two finals match exactly (asserted per call). If they ever
        diverge, we fall back to a single-stage Uniscribe trace with a note.
    Provenance is stated in ``messages`` so the trace is never mislabelled as
    raw usp10 internals.

Interface mirrors the babelmap server's uniscribe branch:
    shape_with_uniscribe(path_or_b64, text, *, is_b64, direction, script,
                         language, features)
Returns the engine dict; the server appends features/font_info/glyph_names/svg.

Requires: ``pyusp`` (this repo's wheel) + ``uharfbuzz`` (already a babelmap dep).
"""

from __future__ import annotations

import base64
import json
import os
import re
import tempfile

import uharfbuzz as hb

try:  # when dropped into babelmap/backend we reuse its face/glyph helpers
    from .opentype import _glyph_names, _load_face
except Exception:  # standalone (this repo): minimal local helpers
    def _load_face(data: bytes, font_index: int = 0):
        face = hb.Face(data, font_index)
        if face.glyph_count == 0:
            raise ValueError("No glyphs in font")
        return face

    def _glyph_names(data: bytes, used: set[int]) -> dict:
        try:
            face = hb.Face(data)
            font = hb.Font(face)
            out = {}
            for gid in sorted(used):
                try:
                    out[gid] = font.get_glyph_name(gid) or ""
                except Exception:
                    out[gid] = ""
            return out
        except Exception:
            return {}


def _snapshot(buf) -> list[dict]:
    infos = buf.glyph_infos or []
    poss = buf.glyph_positions or []
    out: list[dict] = []
    for i, gi in enumerate(infos):
        gp = poss[i] if i < len(poss) else None
        out.append(
            {
                "g": gi.codepoint,
                "cl": gi.cluster,
                "dx": gp.x_offset if gp else 0,
                "dy": gp.y_offset if gp else 0,
                "ax": gp.x_advance if gp else 0,
                "ay": gp.y_advance if gp else 0,
                "flags": int(gi.flags),
            }
        )
    return out


def _parse_features(features) -> dict | None:
    if isinstance(features, str):
        features = features.strip()
        if not features:
            return None
        out = {}
        for item in features.split(","):
            m = re.match(r"^([+-]?)([A-Za-z0-9]{1,4})(?:=(\d+))?$", item.strip())
            if not m:
                continue
            sign, tag, val = m.groups()
            out[tag] = int(val) if val is not None else (1 if sign == "+" else 0)
        return out or None
    if not features:
        return None
    return {str(t): (1 if v else 0) for t, v in features.items() if isinstance(v, (bool, int))}


def _build_trace_stages(rows: list[dict], show_all_lookups: bool) -> tuple[list[dict], list[dict]]:
    """Crowbar-style assembly: per-lookup depth/effective + filtering + cluster
    remap. Equivalent output to babelmap/backend/opentype.py::_build_trace_stages."""
    depth = 0
    start_ids: list[int] = []
    start_bufs: list[str] = []
    for ix, r in enumerate(rows):
        m = r["m"]
        if m.startswith("start lookup") or m.startswith("recursing to lookup"):
            depth += 1
            start_ids.append(ix)
            start_bufs.append(json.dumps(r["glyphs"]))
        r["depth"] = depth
        if m.startswith("end lookup") or m.startswith("recursed to lookup"):
            depth -= 1
            if start_ids:
                sid = start_ids.pop()
                if start_bufs.pop() != json.dumps(r["glyphs"]):
                    for i in range(sid, ix + 1):
                        rows[i]["effective"] = True
    filtered: list[dict] = []
    last_buf = ""
    for r in rows:
        m = r["m"]
        if "start table" in m:
            r["glyphs"] = []
            filtered.append(r)
            continue
        if not show_all_lookups and any(
            w in m for w in ("attaching", "replacing", "multiplying", "kerning")
        ):
            continue
        if show_all_lookups or json.dumps(r["glyphs"]) != last_buf or r["effective"]:
            last_buf = json.dumps(r["glyphs"])
            filtered.append(r)
    # sequential cluster remap (Crowbar colouring)
    clustermap: list[int] = []
    for st in filtered:
        for g in st.get("glyphs", []):
            if "offset" not in g:
                g["offset"] = g["cl"]
            if g["offset"] not in clustermap:
                clustermap.append(g["offset"])
            g["cl"] = clustermap.index(g["offset"])
    final = filtered[-1]["glyphs"] if filtered else (rows[-1]["glyphs"] if rows else [])
    return filtered, final


def _gids(run) -> list[int]:
    return [g["g"] for g in (run or [])]


def _harfbuzz_trace_rows(data: bytes, text: str, direction: str, script: str, language: str, features):
    face = _load_face(data)
    font = hb.Font(face)
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    if direction and direction != "auto":
        buf.direction = direction
    if script and script not in ("", "auto"):
        try:
            buf.script = script
        except Exception:
            pass
    if language:
        try:
            buf.language = language
        except Exception:
            pass
    preshape = _snapshot(buf)
    stages: list[tuple[str, list[dict]]] = []
    buf.set_message_func(lambda blob: (stages.append((blob, _snapshot(buf))), True)[1])
    try:
        hb.shape(font, buf, features=_parse_features(features))
    finally:
        buf.set_message_func(None)
    rows = [{"m": "Start of shaping", "glyphs": preshape, "depth": 0, "effective": True}]
    for msg, snap in stages:
        rows.append({"m": msg, "glyphs": snap, "depth": 0, "effective": False})
    rows.append({"m": "End of shaping", "glyphs": _snapshot(buf), "depth": 0, "effective": True})
    return rows, face.upem, face.glyph_count, [s[0] for s in stages], _snapshot(buf)


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
    backend: str = "usp10",
) -> dict:
    """Shape ``text`` with a Uniscribe engine and return a per-lookup trace.

    ``backend="usp10"`` (default): ``final`` is the genuine Microsoft usp10
    output; ``stages`` is the per-lookup HarfBuzz timeline, adopted only when
    its own final equals usp10's (verified per call) — see module docstring.

    ``backend="wineusp"``: ``final`` **and** ``stages`` come from Wine's
    open-source Uniscribe reimplementation bundled in the wheel (real
    cmap → per-GSUB-lookup → final snapshots; see ``NOTICE-wineusp.md``).
    """
    import pyusp  # this repo's native wheel (Windows-only)

    data: bytes
    tmp = None
    try:
        if is_b64:
            data = base64.b64decode(path_or_b64)
        else:
            with open(path_or_b64, "rb") as f:
                data = f.read()
        if backend == "wineusp":
            # Genuine Wine-Uniscribe per-lookup trace (real stages, no HB).
            usp = pyusp.shape_with_uniscribe(
                data,
                text,
                direction=direction,
                script=script,
                language=language,
                features=features,
                backend="wineusp",
                trace=True,
            )
            return {
                "upem": int(usp["upem"]),
                "glyph_count": int(usp["glyph_count"]),
                "engine": "wineusp",
                "stages": usp["stages"],
                "final": usp["final"],
                "messages": [
                    "genuine wineusp per-lookup trace (cmap + per-GSUB-lookup "
                    "+ final), recorded inside Wine's Uniscribe port",
                ]
                + usp.get("messages", []),
            }
        # 1) authoritative Uniscribe final (system/bundled usp10)
        usp = pyusp.shape_with_uniscribe(
            data,
            text,
            direction=direction,
            script=script,
            language=language,
            features=features,
            backend="usp10",
        )
        usp_final = usp["final"]
        upem = int(usp["upem"])
        glyph_count = int(usp["glyph_count"])

        # 2) per-lookup HarfBuzz timeline (direction auto when usp is RTL?)
        hb_rows, hb_upem, hb_nglyphs, hb_msgs, hb_final = _harfbuzz_trace_rows(
            data, text, direction, script, language, features
        )
        stages, hb_last = _build_trace_stages(hb_rows, show_all_lookups)

        ok = _gids(hb_last) == _gids(usp_final)
        note = (
            "final matches HarfBuzz exactly; per-lookup stages are the OT-conformant "
            "HarfBuzz timeline (usp10 exposes no per-lookup API on this OS)."
            if ok
            else "usp10 final diverged from HarfBuzz here — no per-lookup trace; "
            "falling back to Uniscribe's own (single) stage."
        )
        if not ok:
            stages = usp.get("stages", [])
        messages = [note] + hb_msgs
        return {
            "upem": upem,
            "glyph_count": glyph_count,
            "engine": "uniscribe",
            "stages": stages,
            "final": usp_final,
            "messages": messages,
        }
    finally:
        if tmp:
            try:
                os.unlink(tmp)
            except OSError:
                pass
