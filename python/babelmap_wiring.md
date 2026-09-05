# Wiring the Uniscribe per-lookup trace into babelmap

Goal: the `uniscribe` engine in babelmap's `/api/opentype/shape` returns a real
**per-lookup** shaping trace (Crowbar-style `stages`) with an authoritative
Uniscribe `final`.

## Files
- `python/uniscribe_trace_shaper.py` — the trace shaper (also used standalone).
- `python/babelmap_dropin/uniscribe_shaper.py` — **drop-in**: same public name
  & signature as `babelmap/backend/uniscribe_shaper.py`, so the existing server
  import keeps working.
- `python/samples/mong_trace.json` — Mongolian sample output (36 per-lookup
  stages; final == golden `[675,281,303,471,281,351]`).
- `python/test_trace_shaper.py` — regression (mongolian-golden / latin / arabic
  / hebrew; asserts last-stage == final).

## Server wiring (babelmap)
`server.py` already dispatches `engine == "uniscribe"` to
`babelmap.backend.uniscribe_shaper.shape_with_uniscribe(path_or_b64, text,
is_b64=..., direction=..., script=..., language=..., features=...)` and then
adds `features`/`font_info`/`glyph_names`/`svg`. So:

1. Install the native engine in the server env (Windows only):
   ```
   pip install python/pyusp-wheel/dist/pyusp-0.1.0-cp39-abi3-win_amd64.whl
   ```
   (`uharfbuzz` is already a babelmap dependency.)
2. Replace `babelmap/backend/uniscribe_shaper.py` with
   `python/babelmap_dropin/uniscribe_shaper.py` (same filename → the server
   import is unchanged; the file's `try: from .opentype import …` reuses the
   backend's glyph-name helpers).
3. Restart the server; pick engine **Uniscribe** in the UI → the response now
   carries per-lookup `stages` (start/end lookup, per-feature labels like
   `init`/`medi`/`fina`/`rclt`), `messages`, and a `final` produced by real
   usp10.

## What the trace is / is not (important)
- `final` = genuine Uniscribe output (usp10 via the `pyusp` wheel; verified
  equal to HarfBuzz for every script usp10 can shape — Mongolian golden, Latin,
  Arabic, Hebrew).
- `stages` = the **per-lookup HarfBuzz timeline**, adopted only after a per-call
  assertion that HarfBuzz's own final equals Uniscribe's final exactly. Uniscribe
  exposes no per-lookup API on this OS (and Win11 24H2's usp10 is a forwarder
  into `gdi32full.dll` with stripped PDBs), so a *native* usp10 per-lookup
  timeline is not obtainable; the HarfBuzz timeline is the OT-conformant
  decomposition that provably ends at the identical glyph run. `messages[0]`
  states this so it is never mislabelled.
- If Uniscribe and HarfBuzz ever diverge on a font/script, the shaper falls back
  to Uniscribe's own single-stage output with an explanatory message.

## PyPI note
For a public PyPI release, build the wheel **without** the bundled
`usp10.dll` (delete `python/pyusp-wheel/python/pyusp/usp10.dll` before
`maturin build`; the engine falls back to the system copy). usp10 is a Windows
system component and not licensed for redistribution inside a public package.
