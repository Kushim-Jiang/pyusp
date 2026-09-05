# Wiring the Uniscribe per-lookup trace into babelmap

Goal: the `uniscribe` engine in babelmap's `/api/opentype/shape` returns a real
**per-lookup** shaping trace (Crowbar-style `stages`) with an authoritative
Uniscribe `final`.

## Files
- `python/uniscribe_trace_shaper.py` — the trace shaper (also used standalone).
- `python/babelmap_dropin/uniscribe_shaper.py` — **drop-in**: same public name
  & signature as `babelmap/backend/uniscribe_shaper.py`, so the existing server
  import keeps working.
- `python/samples/mong_trace.json` — Mongolian sample output (52 genuine
  per-lookup stages; final == golden `[675,281,303,471,281,351]`).
- `python/test_trace_shaper.py` — regression (mongolian-golden / latin / arabic
  / hebrew; asserts last-stage == final, genuine wineusp trace).

## Server wiring (babelmap)
`server.py` already dispatches `engine == "uniscribe"` to
`babelmap.backend.uniscribe_shaper.shape_with_uniscribe(path_or_b64, text,
is_b64=..., direction=..., script=..., language=..., features=...)` and then
adds `features`/`font_info`/`glyph_names`/`svg`. So:

1. Install the native engine in the server env (Windows only):
   ```
   pip install python/pyusp-wheel/dist/pyusp-0.1.0-cp39-abi3-win_amd64.whl
   ```
   (the wheel bundles `wineusp.dll`, the standalone Wine Uniscribe port that
   records the genuine per-lookup trace; `uharfbuzz` is not used by this
   shaper.)
2. Replace `babelmap/backend/uniscribe_shaper.py` with
   `python/babelmap_dropin/uniscribe_shaper.py` (same filename → the server
   import is unchanged; the file's `try: from .opentype import …` reuses the
   backend's glyph-name helpers).
3. Restart the server; pick engine **Uniscribe** in the UI → the response now
   carries genuine per-lookup `stages` (cmap, per-GSUB-lookup labels like
   `rclt`, final) and a `final` produced by Wine's Uniscribe port, which is
   byte-identical to system usp10 on the whole corpus we test.

## What the trace is / is not (important)
- `final` (default `backend="wineusp"`) = output of **Wine's open-source
  Uniscribe reimplementation** (`wineusp.dll`, bundled in the wheel). It is
  byte-identical to Microsoft usp10 on every case we test — Mongolian golden,
  Latin, Arabic, Hebrew — same gids **and** advances (`python/test_trace_shaper.py`).
- `stages` = a **genuine** cmap → per-GSUB-lookup → final timeline, recorded
  *inside* the port (`tools/wine_usp/port/src/trace.c`). No HarfBuzz proxy and
  no adopted timeline: the stages come from the same code that produced
  `final`.
- `backend="usp10"` returns the **authoritative Microsoft usp10** `final` and
  adopts the genuine wineusp per-lookup stages only when the two finals match
  exactly (asserted per call); if they ever diverge on a font/script, it falls
  back to Uniscribe's own single-stage output with an explanatory message.
- Provenance is stated in `messages` so the trace is never mislabelled.

## Why not a native usp10 (gdi32full) trace
On Win11 24H2, `usp10.dll` is a forwarder into `gdi32full.dll`, whose OT
engine is a table-driven object framework with stripped PDBs and no public
per-lookup callback. A native per-lookup hook was investigated in depth
(`tools/native_spike/SPIKE.md`, sessions 1–7): the engine runs inline on the
shaping thread but there is **no standalone once-per-lookup dispatcher to
hook** — the dispatch-table members are all per-glyph scanning primitives.
A genuine, engine-internal per-lookup trace is therefore only obtainable from
the open-source Wine Uniscribe port, which is byte-identical to usp10 on our
corpus and exposes the trace by construction.

## PyPI note
For a public PyPI release, build the wheel **without** the bundled
`usp10.dll` (delete `python/pyusp-wheel/python/pyusp/usp10.dll` before
`maturin build`; the engine falls back to the system copy). usp10 is a Windows
system component and not licensed for redistribution inside a public package.
`wineusp.dll` is LGPL 2.1+ and redistributable (see `NOTICE-wineusp.md`).
