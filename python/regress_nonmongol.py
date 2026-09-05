# -*- coding: utf-8 -*-
import json, subprocess, os
EXE=r"D:\Github\pyusp\target\debug\pyusp.exe"
WINE=r"D:\Github\pyusp\tools\wine_usp\port\build\wineusp.dll"
OUT=r"D:\Github\pyusp\tools\wine_usp\regress.txt"
WF=r"C:\Windows\Fonts"
CASES=[
 ("mongolian-golden", r"D:\Github\mongolian-utn\temp\hudum.otf", "\u1830\u1820\u1822\u182c\u1820\u1828"),
 ("latin", os.path.join(WF,"arial.ttf"), "AVATAR Office"),
 ("arabic", os.path.join(WF,"segoeui.ttf"), "\u0633\u0644\u0627\u0645"),
 ("hebrew", os.path.join(WF,"segoeui.ttf"), "\u05e9\u05dc\u05d5\u05dd"),
 ("cyrillic", os.path.join(WF,"arial.ttf"), "\u041f\u0440\u0438\u0432\u0435\u0442"),
 ("greek", os.path.join(WF,"arial.ttf"), "\u039a\u03b1\u03bb\u03b7\u03bc\u03ad\u03c1\u03b1"),
]
def run(usp,font,text):
    r=subprocess.run(([] if not usp else [EXE,"--usp10",usp])+[EXE,"--font",font,"--text",text],capture_output=True,text=True,encoding="utf-8",errors="replace",timeout=20)
    if r.returncode!=0: return ("CRASH",None)
    d=json.loads(r.stdout)
    return ("ok",([g["g"] for g in d["final"]],[g["ax"] for g in d["final"]]))
lines=[]
for label,font,text in CASES:
    if not os.path.exists(font): lines.append(f"{label:<18} skip (font)"); continue
    s1,d1=run(None,font,text)  # None -> default system usp10? use explicit system dll path not needed; pass None means default
    s2,d2=run(WINE,font,text)
    # also explicit system: pyusp default loads system usp10 when --usp10 omitted; replicate by running with no --usp10? run() requires usp arg; pass "" 
    s3,d3=run("",font,text)
    lines.append(f"{label:<18} sys={d1} wine={d2}")
    if d1 and d2:
        lines.append(f"{'':<18} gid-match={d1[0]==d2[0]} ax-match={d1[1]==d2[1]}")
    else:
        lines.append(f"{'':<18} sys_status={s1} wine_status={s2} sys2_status={s3}")
open(OUT,"w",encoding="utf-8").write("\n".join(lines))
print("done")

