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
| **per-lookup trace (server-satisfying)** | **DONE** — `python/uniscribe_trace_shaper.py` |
| native usp10 per-lookup hook | **blocked** (see RE findings) |

### Per-lookup trace delivered (babelmap server contract)

`python/uniscribe_trace_shaper.py` (+ drop-in `python/babelmap_dropin/`,
wiring doc `python/babelmap_wiring.md`, regression
`python/test_trace_shaper.py`, sample `python/samples/mong_trace.json`):

- `final` = **real Uniscribe** output (via the `pyusp` wheel).
- `stages` = **per-lookup HarfBuzz timeline** (Crowbar format, per-feature
  labels such as `init`/`medi`/`fina`/`rclt`), adopted only when a per-call
  assertion proves HarfBuzz's final == Uniscribe's final exactly; otherwise it
  falls back to Uniscribe's own single stage with a message.
- Provenance is stated in `messages[0]` (never mislabelled as raw usp10
  internals). Regression: Mongolian 36 stages (golden final), Latin 7, Arabic
  17, Hebrew 6; `lastStage == final` for all.

### Watershed evidence — usp10 final state converges with HarfBuzz

`python/compare_pyusp.py` (vs uharfbuzz, gids **and** advances):

| case | font | == HarfBuzz |
|---|---|---|
| mongolian-golden | `mongolian-utn/temp/hudum.otf` `ᠰᠠᠢᠬᠠᠨ` | ✅ gids `[675,281,303,471,281,351]` + advances |
| latin | `arial.ttf` `AVATAR Office` | ✅ |
| arabic | `segoeui.ttf` `سلام` | ✅ |
| hebrew | `segoeui.ttf` `שלום` | ✅ |
| devanagari | `Nirmala.ttc` `कर्म` | ❌ structural (below) |

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
per-lookup trace needs a self-hook of the internal single-lookup dispatcher
inside **gdi32full.dll** — the same class of reverse-engineering that
pydwshape's `build/dwc_poc` did for DWriteCore (frida probes + Ghidra, over a
long effort). Public symbols are absent, so it must be found by
behaviour/disassembly. **Session result**: main-thread Stalker (all call forms
+ following every thread) still only observes ~5 direct calls inside
`ScriptShapeOpenType` itself; the deep engine is reached indirectly / on a
worker thread (gdi32full imports threadpool). A *native* usp10 per-lookup
hook is therefore **not** completed — this is why the delivered per-lookup
trace uses the HarfBuzz timeline under a verified-equal-final assertion
(see Status).

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
  app-local/system fallback). `python/pyusp/__init__.py` →
  `pyusp.shape_with_uniscribe(font_bytes, text, ...)`.
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

## TODO — native usp10 per-lookup hook (optional, deep RE)

1. Confirm where gdi32full runs the OT engine (suspected worker thread;
   tools/frida_probe_calls.py follows every thread and still sees ~5 calls,
   so cross-thread capture or full static disassembly of gdi32full is needed).
2. Record its RVA + 12-byte prologue → fill the `HookSpec` in
   `src/selfhook.rs`.
3. Map the glyph-buffer object layout (record pointer / count / stride).
4. Assemble per-lookup stages natively; validate against
   `python/uniscribe_trace_shaper.py` output (same final).
