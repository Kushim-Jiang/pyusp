#!/usr/bin/env python3
"""check_wheel_parity.py — python-level cross-platform parity check (M3).

Installed-wheel test: shape the Noto Sans Mongolian test string through
pyusp.shape_with_uniscribe(backend="wineusp", trace=True) and assert the
per-lookup trace (stage names + glyph runs) and the final run equal the
committed Windows reference golden — proving the wheel's native bytes-mode
shaping is identical on Linux/macOS/Windows.

Usage: check_wheel_parity.py [font.ttf] [golden.json]
Defaults resolve relative to the repo (tools/wine_usp/testdata).
"""
import json
import re
import sys
from pathlib import Path

import pyusp

ROOT = Path(__file__).resolve().parents[2]
FONT = ROOT / "tools" / "wine_usp" / "testdata" / "NotoSansMongolian-Regular.ttf"
GOLDEN = ROOT / "tools" / "wine_usp" / "testdata" / "noto-saikhan-windows-golden.json"
TEXT = "\u1830\u1820\u1822\u182C\u1820\u1828"  # Mongolian "saikhan"
NAME_RE = re.compile(r"^wineusp lookup (.*) \(item \d+\)$")


def main() -> int:
    font = Path(sys.argv[1]) if len(sys.argv) > 1 else FONT
    golden_path = Path(sys.argv[2]) if len(sys.argv) > 2 else GOLDEN
    if not font.exists() or not golden_path.exists():
        print(f"missing font={font} or golden={golden_path}", file=sys.stderr)
        return 2

    data = font.read_bytes()
    out = pyusp.shape_with_uniscribe(data, TEXT, backend="wineusp", trace=True)
    stages = []
    for s in out["stages"]:
        m = NAME_RE.match(s["m"])
        name = m.group(1) if m else s["m"]
        stages.append((name, [g["g"] for g in s["glyphs"]]))
    final = [g["g"] for g in out["final"]]

    golden = json.loads(golden_path.read_text(encoding="utf-8"))
    want = [(s["stage"], s["glyphs"]) for s in golden["stages"]]
    want_final = golden["final"]

    print(f"wheel stages={len(stages)} golden={len(want)}")
    print(f"final={final}")
    ok = stages == want and final == want_final
    print(f"PARITY_MATCH={ok}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
