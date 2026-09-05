"""End-to-end test of the Uniscribe per-lookup trace shaper (babelmap contract).

Genuine per-lookup trace now comes from Wine's Uniscribe port (``wineusp.dll``,
bundled in the pyusp wheel) — no HarfBuzz proxy. ``backend="usp10"`` returns the
authoritative Microsoft usp10 final and adopts the wineusp stages when the
finals match.

Run with the babelsoft venv (has pyusp):
  & 'D:\\Github\\babelsoft-py\\.venv\\Scripts\\python.exe' python/test_trace_shaper.py
"""
import json
import os
import sys

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
# Wine uniscribe port matches system usp10 exactly for the Mongolian golden
# word (rclt enabled + GSUB engine deep fixes: T6 fmt1, empty-guard stop,
# T8 reverse parse fix, IgnoreMarks/backtrack mark filtering).
WINEUSP_MONG = GOLDEN


def _gids(run):
    return [g["g"] for g in (run or [])]


def main():
    ok_all = True

    # 1) default backend='wineusp': genuine per-lookup trace (no HB).
    print("== backend=wineusp (default): genuine per-lookup trace ==")
    for label, font, text in CASES:
        d = shape_with_uniscribe(font, text)
        gids = _gids(d["final"])
        nst = len(d["stages"])
        last = _gids(d["stages"][-1]["glyphs"]) if d["stages"] else []
        consistent = last == gids
        genuine = nst >= 2 and d["engine"] == "wineusp"
        note = ""
        if label == "mongolian-golden":
            note = "golden=%s" % (gids == GOLDEN)
            ok_all &= (gids == GOLDEN) and consistent and genuine
        else:
            ok_all &= consistent and genuine
        print(f"wineusp {label:<17} final={gids} stages={nst} "
              f"lastStage==final={consistent} genuineTrace={genuine}  {note}")

    # 2) backend='usp10': authoritative usp10 final + genuine stages when equal.
    print()
    print("== backend=usp10: authoritative usp10 final + genuine stages ==")
    for label, font, text in CASES:
        d = shape_with_uniscribe(font, text, backend="usp10")
        gids = _gids(d["final"])
        nst = len(d["stages"])
        last = _gids(d["stages"][-1]["glyphs"]) if d["stages"] else []
        consistent = last == gids
        note = ""
        if label == "mongolian-golden":
            note = "usp10-final=%s" % (gids == GOLDEN)
            ok_all &= (gids == GOLDEN) and consistent
        else:
            ok_all &= consistent
        print(f"usp10   {label:<17} final={gids} stages={nst} "
              f"lastStage==final={consistent}  {note}")

    print("\nPASS" if ok_all else "\nFAIL")
    return 0 if ok_all else 1


if __name__ == "__main__":
    sys.exit(main())
