"""End-to-end test of the Uniscribe per-lookup trace shaper (babelmap contract).

Run with the babelsoft venv (has pyusp + uharfbuzz):
  & 'D:\\Github\\babelsoft-py\\.venv\\Scripts\\python.exe' python/test_trace_shaper.py
"""
import json
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from uniscribe_trace_shaper import shape_with_uniscribe

WF = r"C:\Windows\Fonts"
CASES = [
    ("mongolian-golden", r"D:\Github\mongolian-utn\temp\hudum.otf",
     "\u1830\u1820\u1822\u182c\u1820\u1828"),
    ("latin", os.path.join(WF, "arial.ttf"), "AVATAR Office"),
    ("arabic", os.path.join(WF, "segoeui.ttf"), "\u0633\u0644\u0627\u0645"),
    ("hebrew", os.path.join(WF, "segoeui.ttf"), "\u05e9\u05dc\u05d5\u05dd"),
]
GOLDEN = [675, 281, 303, 471, 281, 351]
# Wine uniscribe port now matches system usp10 exactly for the Mongolian
# golden word (rclt enabled + GSUB engine deep fixes: T6 fmt1, empty-guard
# stop, T8 reverse parse fix, IgnoreMarks/backtrack mark filtering).
WINEUSP_MONG = GOLDEN


def main():
    ok_all = True
    for label, font, text in CASES:
        d = shape_with_uniscribe(font, text)
        gids = [g["g"] for g in d["final"]]
        nst = len(d["stages"])
        last = [g["g"] for g in d["stages"][-1]["glyphs"]] if d["stages"] else []
        consistent = last == gids
        note = ""
        if label == "mongolian-golden":
            note = "golden=%s" % (gids == GOLDEN)
            ok_all &= (gids == GOLDEN) and consistent
        else:
            ok_all &= consistent
        print(f"usp10  {label:<18} final={gids} stages={nst} lastStage==final={consistent}  {note}")

    # backend='wineusp': genuine per-lookup trace recorded inside Wine's port
    print()
    for label, font, text in CASES:
        d = shape_with_uniscribe(font, text, backend="wineusp")
        gids = [g["g"] for g in d["final"]]
        nst = len(d["stages"])
        last = [g["g"] for g in d["stages"][-1]["glyphs"]] if d["stages"] else []
        consistent = last == gids
        genuine = nst >= 2 and d["engine"] == "wineusp"
        note = ""
        if label == "mongolian-golden":
            note = "wineusp-final=%s" % (gids == WINEUSP_MONG)
            ok_all &= consistent and genuine and (gids == WINEUSP_MONG)
        else:
            ok_all &= consistent and genuine
        print(f"wineusp{label:<17} final={gids} stages={nst} lastStage==final={consistent} genuineTrace={genuine}  {note}")

    print("\nPASS" if ok_all else "\nFAIL")
    return 0 if ok_all else 1


if __name__ == "__main__":
    sys.exit(main())
