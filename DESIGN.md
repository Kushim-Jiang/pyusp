# pyusp — Uniscribe (usp10) OpenType shaping tracer

Rust engine + PyO3 abi3 wheel that drives **Uniscribe's OpenType path**
(`ScriptItemizeOpenType` → `ScriptShapeOpenType` → `ScriptPlaceOpenType`) on a
real GDI font and emits the babelsoft `/api/opentype/shape` engine dict — the
Uniscribe analogue of the pydwshape/DWriteCore tracer and the HarfBuzz engines.

## Status (2026-09-05)

| milestone | state |
|---|---|
| Step-1 watershed probe (final == golden) | **DONE, PASS** |
| final-state engine + CLI (`pyusp` crate) | DONE, verified |
| PyO3 abi3 wheel (bundles usp10.dll) | DONE, verified |
| `pe` signature scanner / relocation infra | DONE, verified on gdi32full |
| `selfhook` inline-hook engine | compiled; target requires native-RE (below) |
| **per-lookup trace (server-satisfying)** | **DONE** — genuine wineusp (see below) |
| native usp10 (gdi32full) per-lookup hook | **blocked** (SPIKE sessions 1–7: no once-per-lookup dispatcher) |
| Wine GSUB engine deep fixes (rclt/T6/T8/marks) | DONE — Mongolian byte-identical to usp10 |

### Per-lookup trace delivered (babelmap server contract) — genuine, no HB

`python/uniscribe_trace_shaper.py` (+ drop-in `python/babelmap_dropin/`,
wiring doc `python/babelmap_wiring.md`, regression
`python/test_trace_shaper.py`, sample `python/samples/mong_trace.json`):

- The bundled `wineusp.dll` is **Wine's open-source Uniscribe port** compiled
  standalone (LGPL 2.1+). After the GSUB engine deep fixes it is
  **byte-identical to system usp10** on every case we test (Mongolian golden,
  Latin, Arabic, Hebrew — same gids **and** advances).
- `stages` = **genuine** cmap → per-GSUB-lookup → final snapshots recorded
  *inside* the port (`tools/wine_usp/port/src/trace.c`) — no HarfBuzz proxy,
  no adopted timeline; the stages come from the same code that produced
  `final`. Crowbar format, per-feature labels such as `rclt`.
- Default `backend="wineusp"` returns both from that engine. `backend="usp10"`
  returns the authoritative Microsoft usp10 `final` and adopts the genuine
  wineusp stages only when the two finals match exactly (asserted per call),
  otherwise a single-stage Uniscribe trace with a message.
- Regression: Mongolian **52** stages (golden final `[675,281,303,471,281,351]`),
  Latin 5, Arabic 9, Hebrew 25; `lastStage == final` for all; genuine trace
  asserted (`engine == "wineusp"`, ≥ 2 stages).

### Watershed evidence — usp10 final state converges with HarfBuzz

`python/compare_pyusp.py` (vs uharfbuzz, gids **and** advances):

| case | font | == HarfBuzz |
|---|---|---|
| mongolian-golden | `mongolian-utn/temp/hudum.otf` `ᠰᠠᠢᠬᠠᠨ` | ✅ gids `[675,281,303,471,281,351]` + advances |
| latin | `arial.ttf` `AVATAR Office` | ✅ |
| arabic | `segoeui.ttf` `سلام` | ✅ |
| hebrew | `segoeui.ttf` `שלום` | ✅ |
| devanagari | `Nirmala.ttc` `कर्म` | ❌ structural (below) |

(The trace source `wineusp.dll` is itself asserted byte-identical to usp10 on
these same cases in `python/test_trace_shaper.py` — same gids and advances —
so the per-lookup trace's `final` is also usp10's final.)

**Devanagari finding**: usp10's OpenType engine only recognises *legacy* script
tags (`deva`, …). Nirmala (like all modern Indic fonts) exposes only `dev2`;
both legacy `ScriptShape` and OpenType paths return `USP_E_SCRIPT_NOT_IN_FONT
(0x80040200)`. This is a **real usp10 limitation** (old Win32 apps that used
usp10 could not shape these either), not a tracer bug.

## Critical RE facts (change the original roadmap)

1. **`usp10.pdb` on the MS symbol server is a stripped public-symbol PDB**
   (77 KB) — no internal function names. Roadmap step "load usp10 PDB → find
   internal ApplyLookup RVA" **does not work**.
2. **On Win11 24H2 (usp10 10.0.26100.1) `usp10.dll` is a 100 KB forwarder**:
   every `Script*` export forwards to **`gdi32full.dll`** (verified: the
   addresses returned by `GetProcAddress` for `ScriptShapeOpenType`,
   `ScriptItemizeOpenType`, `ScriptPlace`, … all land inside `gdi32full`).
   → the internal per-lookup engine (the thing to self-hook) lives in
   **gdi32full.dll**, not usp10.dll.

### Why this matters for the per-lookup trace

Uniscribe has no public buffer-message / per-feature callback, so a *native*
per-lookup trace would need a self-hook of the internal single-lookup
dispatcher inside **gdi32full.dll** — the same class of reverse-engineering
that pydwshape's `build/dwc_poc` did for DWriteCore (frida probes + Ghidra,
over a long effort). Public symbols are absent, so it must be found by
behaviour/disassembly. `tools/native_spike/SPIKE.md` (sessions 1–7)
documented the full investigation: the OT engine is a **table-driven object
framework** (dispatch table `.rdata 0x1800b4988`, per-type size dispatcher
`0x4e970`, header getter `0x4eb50`) running inline on the shaping thread;
hooking every dispatch-table member during a Mongolian shape showed they are
all **per-glyph scanning primitives** (strictly 9/glyph, 5/glyph — not
per-lookup). There is **no standalone once-per-lookup dispatcher to hook**, so
a native per-lookup trace is not obtainable. The delivered per-lookup trace
therefore comes from the **genuine Wine Uniscribe port** (`wineusp.dll`), which
is byte-identical to usp10 on our corpus and records per-lookup stages by
construction (see Status).

## Layout

- `src/lib.rs` — engine: dynamic usp10 loader, GDI font driver
  (`AddFontResourceEx FR_PRIVATE` → `CreateFontIndirectW`), OpenType shaping,
  babelsoft JSON assembly (`pyusp::shape_json` / `shape_value`).
- `src/pe.rs` — `pe_lookup_rva_for_sig`, `find_all_sig_rvas`,
  `verify_or_relocate` (signature-locked + auto-relocated hook addressing).
- `src/selfhook.rs` — inline-hook engine (12B patch + trampoline + stub),
  parameterised over the dispatcher RVA / arg slot / glyph-record layout.
- `src/main.rs` — CLI (`--font/--text/--script/--language/--direction/
  --features/--usp10/--out`) + RE helper `--find-sig <hex>`
  (`--scan-module <dll>` to pick the module to scan, default usp10.dll).
- `python/pyusp-wheel/` — maturin abi3 wheel `pyusp`, bundles `usp10.dll`
  (copied from the OS by `python/build_wheel.ps1`; `NOTICE.md` documents the
  app-local/system fallback) **and `wineusp.dll`** (standalone Wine Uniscribe
  port that records the genuine per-lookup trace). `python/pyusp/__init__.py`
  → `pyusp.shape_with_uniscribe(font_bytes, text, ..., backend=...)`.
- `tools/wine_usp/` — the standalone Wine `dlls/gdi32/uniscribe` port
  (`port/` builds `wineusp.dll`); `port/src/trace.c` is the in-engine
  per-lookup stage recorder.
- `tools/native_spike/` — gdi32full native-RE investigation (SPIKE.md,
  sessions 1–7: thread model, dispatch table, per-glyph refutation).
- `python/compare_pyusp.py` — uharfbuzz regression matrix (above).
- `tools/get_usp10_pdb.py` — downloads usp10.pdb (kept in `tools/pdb/`).
- `tools/pdb_strings.py` — dumps PDB identifier strings.

## Engine notes

- usp10 is loaded **dynamically** (`LoadLibraryW`, bundled copy preferred) so
  the same handle can be self-hooked later and so the wheel can ship an
  app-local usp10.dll.
- Font must be registered (private) before `CreateFontIndirectW`, otherwise
  uninstalled font files fall back to a default font (classic bug the old
  python shaper had). `PYUSP_NO_PRIVATE=1` disables it for debugging.
- Script tag: prefer the font's own tags via `ScriptGetFontScriptTags` (it
  reports `dev2` etc. that itemize can't); fall back to the item tag. LangSys
  fixed to `'dflt'` (`ScriptGetFontLanguageTags` is unreliable against
  installed fonts → `USP_E_SCRIPT_NOT_IN_FONT`).
- RTL runs are shaped in logical order (`fLogicalOrder`) and reversed to
  *visual* order at the boundary so `final` matches HarfBuzz/uharfbuzz
  (clusters stay logical char indices).
- Advances are returned in font design units (lfHeight = −upem ⇒ 1 px ≈ 1
  unit), matching HarfBuzz's font units.

## Native usp10 per-lookup hook — investigated, not feasible (SPIKE)

`tools/native_spike/SPIKE.md` (sessions 1–7) is the full record. Summary:

1. On Win11 24H2 `usp10.dll` forwards into `gdi32full.dll` (stripped PDBs).
2. The OT engine runs **inline on the shaping thread** (no threadpool), as a
   **table-driven object framework**: dispatch table `.rdata 0x1800b4988`,
   per-type size dispatcher `0x4e970`, header getter `0x4eb50`.
3. Hooking every dispatch-table member during a Mongolian shape showed they
   are all **per-glyph scanning primitives** (9/glyph, 5/glyph — latin 0/glyph),
   never once-per-lookup → **no standalone per-lookup dispatcher exists** to
   self-hook, and the glyph-buffer snapshots between lookups have no clean
   boundary function.
4. A native per-lookup trace therefore is not obtainable from gdi32full
   without a full framework RE (pydwshape/DWriteCore-scale effort).

The delivered per-lookup trace instead uses the **genuine Wine Uniscribe
port** (`wineusp.dll`): byte-identical to usp10 on our corpus, per-lookup
stages recorded by construction (`tools/wine_usp/port/src/trace.c`).
