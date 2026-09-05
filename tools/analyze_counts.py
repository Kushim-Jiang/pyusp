"""Analyze a frida call-count dump: bucket call targets by count and print
the interesting mid-range candidates plus their call sites.

Usage: python tools/analyze_counts.py <counts.json> [hex]
"""
import collections
import json
import sys


def main():
    path = sys.argv[1]
    prefix = sys.argv[2] if len(sys.argv) > 2 else "hex"
    d = json.load(open(path))
    base = int(d.get("gdiBase", 0x180000000))
    c = collections.Counter({int(k, 16): v for k, v in d["counts"].items()})
    sites = d.get("sites", {})
    fmt = (lambda rva: "0x%06x" % rva) if prefix == "hex" else (lambda rva: "%d" % rva)
    print("distinct internal call targets:", len(c))
    if not c:
        return
    b = collections.Counter(c.values())
    print("count buckets (count -> #targets):")
    for cnt in sorted(b):
        print("  count=%-6d : %d" % (cnt, b[cnt]))
    print("\ncandidates sorted by count (low->high):")
    for addr, cnt in sorted(c.items(), key=lambda kv: kv[1])[:200]:
        rva = addr - base
        nsrc = len(sites.get(addr, {}))
        print("  rva=%s count=%d sites=%d" % (fmt(rva), cnt, nsrc))


if __name__ == "__main__":
    main()
