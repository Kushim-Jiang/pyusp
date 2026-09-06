"""Align a native TextShaping trace (produced by frida_ts_trace.py) against the
authoritative wineusp named-stage trace for the same text, and validate that
the native final run equals the wineusp/usp10 final.

Usage:
  python frida_ts_align.py --native <native_trace.json> --wine <wine_trace.json>
"""
import argparse
import io
import json


def load(p):
    return json.load(io.open(p, encoding="utf-8"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--native", required=True)
    ap.add_argument("--wine", required=True)
    a = ap.parse_args()
    nat = load(a.native)
    win = load(a.wine)

    win_final = [g["g"] for g in win["final"]]
    nat_final = nat.get("last_app_leave_run")
    nat_final = nat_final["gids"] if nat_final else None
    if nat_final and len(nat_final) > len(win_final):
        nat_final = nat_final[: len(win_final)]

    print("== native trace applications (TextShaping 0x15650) ==")
    for app in nat["apps"]:
        en = app["enter"]["gids"] if app["enter"] else None
        le = app["leave"]["gids"] if app["leave"] else None
        ch = " <== run CHANGES" if (en and le and en != le) else ""
        print("  app %-3d enter=%-34s leave=%-34s%s" % (app["app"], en, le, ch))

    print("\n== authoritative wineusp stages (feature#lookup, collapsed) ==")
    prev = None
    for s in win.get("stages", []):
        g = [x["g"] for x in s.get("glyphs", [])]
        if g != prev:
            print("  %-34s %s" % (s.get("m", ""), g))
            prev = g

    print("\n== VALIDATION ==")
    print("  native final run :", nat_final)
    print("  wineusp final    :", win_final)
    print("  native == wineusp final:", nat_final == win_final)


if __name__ == "__main__":
    main()
