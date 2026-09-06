"""Convert a native TextShaping per-application capture (frida_ts_trace.py
output) into the babelsoft trace shape: an ordered ``stages`` timeline where
every application is one stage (per-glyph apps show the single glyph being
resolved; whole-run apps show the full run), plus a full ``final``.

Stage glyph records carry {g, cl, ax/ay/dx/dy/flags:0} like the wineusp stages
(positions are only meaningful on ``final``). ``final`` full records (with
advances), ``upem`` and ``glyph_count`` are merged from a pyusp shape JSON.

Usage:
  python native_to_stages.py --native <native_trace.json> \
      --shape <pyusp_shape.json> --out <stages.json>
"""
import argparse
import io
import json


def load(p):
    return json.load(io.open(p, encoding="utf-8"))


def stage(m, gids, clusters):
    gs = []
    for g, cl in zip(gids, clusters):
        gs.append({"g": g, "cl": cl, "ax": 0, "ay": 0, "dx": 0, "dy": 0, "flags": 0})
    return {"depth": 0, "effective": True, "glyphs": gs, "m": m}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--native", required=True)
    ap.add_argument("--shape", required=True, help="pyusp shape JSON (final/upem/glyph_count)")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    nat = load(a.native)
    shp = load(a.shape)

    apps = nat["apps"]
    napp = nat["applications"]
    base_run = None
    # earliest whole-run enter == assembled cmap/base (reference stage 0)
    for x in apps:
        en = x.get("enter")
        if en and len(en["gids"]) > 1:
            base_run = en["gids"]
            break

    stages = []
    if base_run is not None:
        stages.append(stage("native TextShaping base/cmap (item 0)", base_run,
                            list(range(len(base_run)))))
    for x in apps:
        le = x.get("leave")
        if not le:
            continue
        gids = le["gids"]
        if len(gids) == 1:
            # per-glyph application: show the single glyph being resolved.
            idx = (x["app"] - 1) // 4 if napp else 0
            stages.append(stage("native TextShaping app %d/%d (item 0) glyph[%d]" %
                                (x["app"], napp, idx), gids, [idx]))
        else:
            stages.append(stage("native TextShaping app %d/%d (item 0)" %
                                (x["app"], napp), gids, list(range(len(gids)))))

    out = {
        "engine": "TextShaping.dll (native usp10 path, via frida 0x15650)",
        "upem": int(shp.get("upem", 0)),
        "glyph_count": int(shp.get("glyph_count", 0)),
        "stages": stages,
        "final": shp["final"],
    }
    with io.open(a.out, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)

    # validation
    last = stages[-1]["glyphs"]
    lg = [g["g"] for g in last]
    fg = [g["g"] for g in out["final"]]
    print("stages:", len(stages))
    print("lastStage gids:", lg)
    print("final     gids:", fg)
    print("lastStage == final:", lg == fg)
    print("wrote", a.out)


if __name__ == "__main__":
    main()
