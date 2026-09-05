# wineusp.dll notice (Wine Uniscribe port)

The file `wineusp.dll` bundled in this wheel is a **standalone compilation of
Wine's Uniscribe implementation** (`dlls/gdi32/uniscribe` from Wine, plus
`include/usp10.h`), plus a thin port shim layer and an optional per-lookup
trace recorder added by this project.

License: **GNU Lesser General Public License version 2.1 or (at your option)
any later version** — see https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html

- Upstream Wine source: https://gitlab.winehq.org/wine/wine (dlls/gdi32/uniscribe)
- The port used to build this DLL (modified sources, shims, trace recorder,
  build commands) lives in this repository under `tools/wine_usp/port/`
  (`src/` = the LGPL sources as compiled; `include/` = native-Windows shims;
  `build/` = build outputs). That directory is the corresponding source for
  this LGPL object.

Compliance notes:

- `wineusp.dll` is loaded **dynamically** by the engine (LoadLibrary) and is
  not linked into the Python extension, so the LGPL "mere aggregation / use
  of the library through its public interface" provisions apply.
- This wheel is usable without `wineusp.dll`: delete the file (or use
  `backend="usp10"`) and the engine loads the system Uniscribe instead.

Per-lookup trace: `wineusp.dll` additionally exports `usp_trace_begin/count/
stage/stop` (a port extension) that the engine uses to record the genuine
cmap → per-GSUB-lookup → final snapshots when `shape_with_uniscribe(..., 
backend="wineusp", trace=True)` is used. These symbols are additive; the
public Uniscribe `Script*` API is unchanged.
