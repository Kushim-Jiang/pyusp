"""compare_pyusp — regression matrix: run the pyusp (Uniscribe ScriptShapeOpenType)
final glyph run against uharfbuzz (HarfBuzz), and report per-case equality on
glyph ids AND advances.

Run with the babelsoft venv (has uharfbuzz):
  & 'D:\\Github\\babelsoft-py\\.venv\\Scripts\\python.exe' python/compare_pyusp.py
"""
import json
import os
import subprocess
import sys

import uharfbuzz as hb

EXE = r"D:\Github\pyusp\target\debug\pyusp.exe"
WF = r"C:\Windows\Fonts"

# (label, font, text, script)
CASES = [
    ("mongolian-golden", r"D:\Github\mongolian-utn\temp\hudum.otf",
     "\u1830\u1820\u1822\u182c\u1820\u1828", "mong"),  # ᠰᠠᠢᠬᠠᠨ
    ("latin", os.path.join(WF, "arial.ttf"), "AVATAR Office", ""),
    ("arabic", os.path.join(WF, "segoeui.ttf"), "\u0633\u0644\u0627\u0645", "arab"),
    ("devanagari", os.path.join(WF, "Nirmala.ttc"), "\u0915\u0930\u094d\u092e", "deva"),
    ("hebrew", os.path.join(WF, "segoeui.ttf"), "\u05e9\u05dc\u05d5\u05dd", "hebr"),
]

GOLDEN = [675, 281, 303, 471, 281, 351]


def run_pyusp(font, text):
    args = [EXE, "--font", font, "--text", text]
    r = subprocess.run(args, capture_output=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.decode("utf-8", "replace")[-600:])
    return json.loads(r.stdout.decode("utf-8"))


def harfbuzz(font_path, text, script):
    with open(font_path, "rb") as f:
        data = f.read()
    face = hb.Face(data)
    font = hb.Font(face)
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    if script:
        try:
            buf.script = script
        except Exception:
            pass
    hb.shape(font, buf)
    return (
        [g.codepoint for g in buf.glyph_infos],
        [g.cluster for g in buf.glyph_infos],
        [round(p.x_advance) for p in buf.glyph_positions],
    )


def main():
    print(f"{'case':<18} {'nGlyph':>6} {'==HB gid':>9} {'==HB ax':>8}  note")
    allok = True
    for label, font, text, script in CASES:
        if not os.path.exists(font):
            print(f"{label:<18}  (skip, font missing: {font})")
            continue
        try:
            d = run_pyusp(font, text)
        except Exception as e:
            print(f"{label:<18}  ERROR: {e}")
            allok = False
            continue
        gids = [g["g"] for g in d["final"]]
        axs = [g["ax"] for g in d["final"]]
        hb_gids, hb_cls, hb_axs = harfbuzz(font, text, script)
        same_gid = gids == hb_gids
        same_ax = axs == hb_axs
        note = ""
        if label == "mongolian-golden":
            note = "golden=%s" % (gids == GOLDEN)
            if gids != GOLDEN or not same_gid:
                allok = False
        elif label == "devanagari":
            # usp10 only knows legacy script tags ('deva'); Nirmala is 'dev2'
            # → usp10 cannot OT-shape it (expected structural limitation).
            note = "(expected: usp10 lacks dev2; nGlyph=%d)" % len(gids)
            allok = False  # counts as a finding, not a pass
        elif not same_gid or not same_ax:
            note = f"DIFF pyusp={gids[:8]} hb={hb_gids[:8]}"
            allok = False
        if not same_ax and same_gid and label not in ("devanagari",):
            note += f" ax-diff py={axs[:6]} hb={hb_axs[:6]}"
        print(f"{label:<18} {len(gids):>6} {'OK' if same_gid else 'DIFF':>9} "
              f"{'OK' if same_ax else 'DIFF':>8}  {note}")
    print("\nPASS (usp10 final == HarfBuzz on the legacy-tag corpus)"
          if allok else "\nSEE NOTES ABOVE (deva = structural usp10 limitation)")
    return 0 if allok else 1


if __name__ == "__main__":
    sys.exit(main())
