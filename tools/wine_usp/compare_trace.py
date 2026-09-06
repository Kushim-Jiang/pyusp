#!/usr/bin/env python3
"""compare_trace.py — cross-platform parity checker (M1/M3).

Compares a test_posix "name: g1 g2 ..." trace file against the committed
Windows reference (tools/wine_usp/testdata/noto-saikhan-windows-golden.json)
stage-for-stage. Exit 0 on exact match, 1 otherwise.

Usage: compare_trace.py <trace.txt> <golden.json>
"""
import json
import re
import sys

NAME_RE = re.compile(r"^([a-z]+#?\d*/\d*|cmap|final): (.*)$")


def parse_trace(path):
    seq = []
    for line in open(path, encoding="utf-8"):
        m = NAME_RE.match(line.strip())
        if m:
            gids = [int(x) for x in m.group(2).split()]
            seq.append((m.group(1), gids))
    return seq


def main():
    if len(sys.argv) != 3:
        print("usage: compare_trace.py <trace.txt> <golden.json>", file=sys.stderr)
        return 2
    trace = parse_trace(sys.argv[1])
    golden = json.load(open(sys.argv[2], encoding="utf-8"))
    want = [(s["stage"], s["glyphs"]) for s in golden["stages"]]

    print(f"driver stages={len(trace)} golden={len(want)}")
    if len(trace) != len(want):
        print(f"LENGTH MISMATCH: {len(trace)} vs {len(want)}")
        return 1
    for i, (a, b) in enumerate(zip(trace, want)):
        if a != b:
            print(f"stage {i} mismatch:\n  driver={a}\n  golden={b}")
            return 1
    print("PARITY_MATCH=True")
    return 0


if __name__ == "__main__":
    sys.exit(main())
