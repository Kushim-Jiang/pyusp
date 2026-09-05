# -*- coding: utf-8 -*-
"""Broad sys-vs-wine matrix on Mongolian words, writes report file."""
import json
import subprocess

EXE = r"D:\Github\pyusp\target\debug\pyusp.exe"
FONT = r"D:\Github\mongolian-utn\temp\hudum.otf"
WINE = r"D:\Github\pyusp\tools\wine_usp\port\build\wineusp.dll"

WORDS = {
    "saihan": [0x1830, 0x1820, 0x1822, 0x182C, 0x1820, 0x1828],
    "moŋɣol": [0x182E, 0x1823, 0x1829, 0x182D, 0x1823, 0x182F],
    "ulus": [0x1824, 0x182F, 0x1824, 0x1830],
    "kele": [0x182C, 0x1821, 0x182F, 0x1821],
    "nigen": [0x1828, 0x1822, 0x182D, 0x1821, 0x1828],
    "tenger": [0x182F, 0x1821, 0x1828, 0x1822, 0x182F],
    "sigsig": [0x1830, 0x1822, 0x182D, 0x1830, 0x1822, 0x182D],
    "sigsiga": [0x1830, 0x1822, 0x182D, 0x1830, 0x1822, 0x182D, 0x1820],
    "sigsigsigsiga": [0x1830, 0x1822, 0x182D] * 4 + [0x1820],
}


def run(usp, text):
    try:
        r = subprocess.run([EXE, "--usp10", usp, "--font", FONT, "--text", text],
                           capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=20)
    except subprocess.TimeoutExpired:
        return "TIMEOUT", None
    if r.returncode != 0:
        return "CRASH", None
    try:
        return "ok", json.loads(r.stdout)
    except Exception:
        return "BADJSON", None


lines = []
for name, cps in WORDS.items():
    txt = "".join(chr(c) for c in cps)
    ss, s = run("usp10.dll", txt)
    ws, w = run(WINE, txt)
    if s is None or w is None:
        lines.append(f"{name:<14} sys={ss} wine={ws}")
        continue
    sg = [g["g"] for g in s["final"]]
    wg = [g["g"] for g in w["final"]]
    sax = [g["ax"] for g in s["final"]]
    wax = [g["ax"] for g in w["final"]]
    eq = sg == wg and sax == wax
    lines.append(f"{name:<14} match={eq}")
    lines.append(f"    sys ={sg}")
    lines.append(f"    wine={wg}")

with open(r"D:\Github\pyusp\tools\wine_usp\matrix.txt", "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines))
print("done")
