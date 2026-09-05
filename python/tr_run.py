# -*- coding: utf-8 -*-
"""Run pyusp with USP_TR=1 for given words, print [usp] stage lines + final gids."""
import os, subprocess, sys, json

try:
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
except Exception:
    pass

EXE = r"D:\Github\pyusp\target\debug\pyusp.exe"
DLL = r"D:\Github\pyusp\tools\wine_usp\port\build\wineusp.dll"
FONT = r"D:\Github\mongolian-utn\temp\hudum.otf"

os.environ["USP_TR"] = "1"
os.environ["USP2"] = "1"
words = sys.argv[1:] or ["ᠰᠢᠭᠰᠢᠭᠠ", "ᠰᠠᠢᠬᠠᠨ"]
out = []

for w in words:
    try:
        r = subprocess.run([EXE, "--usp10", DLL, "--font", FONT, "--text", w],
                           capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=15)
    except subprocess.TimeoutExpired:
        out.append(f"\n########## {w} TIMEOUT ##########")
        continue
    out.append(f"\n########## {w} exit={r.returncode} ##########")
    for line in r.stderr.splitlines():
        if "[usp]" in line or "[usp2]" in line:
            out.append(line)
    if r.returncode == 0 and r.stdout:
        try:
            d = json.loads(r.stdout)
            out.append("final: " + repr([g["g"] for g in d["final"]]))
        except Exception as e:
            out.append("stdout parse err %s %s" % (e, r.stdout[:200]))
    else:
        out.append("NO STDOUT")

with open(r"D:\Github\pyusp\python\tr_out.txt", "w", encoding="utf-8") as fh:
    fh.write("\n".join(out))
print("saved", len(out), "lines")
