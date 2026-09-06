# Native TextShaping per-application trace — supported TextShaping.dll versions

The engine has an optional **in-process "native run" capture**: it hooks the
`TextShaping.dll` driver that applies an OT operation over a glyph run and
records the glyph-run state at every application (per-glyph base resolution,
then whole-run passes) — a real glyph-state trace straight from the Microsoft
engine, no frida needed.

**Activation (diagnostic).** Set the environment variable `PYUSP_NATIVE_RUN=1`
and shape with the **system usp10** backend (system `usp10.dll` drives
`TextShaping.dll`). Per-application glyph runs are printed to stderr, and the
final captured run is cross-checked against the shaped output.

**How support is decided at runtime.** The engine scans the *loaded*
`TextShaping.dll` image for the driver's **full prologue signature** —
`push rbp; push rsi; push r12..r15; lea rbp,[rsp-0x218]; sub rsp,0x318`
(a 24-byte pattern). The native trace is enabled only when the signature is
found **exactly once and with the exact validated frame layout**; otherwise it
falls back to the single-stage/final form (no native stages). So only builds
whose engine kept the exact validated layout are traced; nothing is ever hooked
on assumption.

## Supported TextShaping.dll versions

**Validated — native per-application trace enabled** (Win11 24H2, build
10.0.26100, x64; driver RVA drifts `0x15250`–`0x15650` but the layout matches):
- 26100.2033, 26100.2454, 26100.3624, 26100.4343, 26100.6725,
  26100.8972, 26100.9278

(`26100.9278` is the build this wheel was developed and validated against.)

**Structurally similar but NOT enabled** (would need runtime validation on that
build before supporting): Win11 22H2, build 10.0.22621 (x64) — e.g. .317,
.3672, .4036, .4317, .4391, .4541, .2428. These share the driver-entry prefix
but use different internal frame offsets, so the glyph-run layout is not
verified there.

**Different engine / signature not matched (falls back):**
- Win11 24H2 RTM `26100.1` (its driver prologue differs from later revisions)
- any `TextShaping.dll` whose driver prologue does not match the 24-byte
  signature (e.g. Windows 10-era builds seen in the wild)

## Notes

- **x64 only.** The hook targets the x64 engine; x86 / ARM64 builds of
  `TextShaping.dll` are not traced by this engine.
- This is a best-effort in-process hook on a Windows OS component, version
  gated and diagnostics-only, and only on the matching builds above.
- The authoritative **per-lookup (named)** trace is provided by the bundled
  `wineusp.dll` backend (`backend="wineusp"`, `trace=True`) — see
  `NOTICE-wineusp.md`.
