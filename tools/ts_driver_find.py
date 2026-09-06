"""Locate the TextShaping "apply one OT op" driver (the pyusp native-run hook
target) in an arbitrary TextShaping.dll file, so the native per-application
trace can be supported on more Windows builds.

Prints the file version, the driver RVA found by prologue signature, how many
times the signature matched (must be 1 to hook safely), and a ready-to-paste
table entry.

Usage:
  python ts_driver_find.py C:\\path\\to\\TextShaping.dll
"""
import io
import struct
import subprocess
import sys

# 6 pushes + lea rbp,[rsp-0x218] (start of the big-stack driver prologue),
# observed on Win11 10.0.26100 @ RVA 0x15650. First 13 bytes.
SIG = bytes.fromhex("40 55 56 41 54 41 55 41 56 41 57 48 8D")
# longer full 24-byte prologue for cross-check
SIG_FULL = bytes.fromhex(
    "40 55 56 41 54 41 55 41 56 41 57 48 8D AC 24 E8 FD FF FF 48 81 EC 18 03 00 00"
)


def main():
    path = sys.argv[1]
    data = io.open(path, "rb").read()
    try:
        vi = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "(Get-Item '%s').VersionInfo.FileVersion" % path,
            ],
            capture_output=True,
            text=True,
            timeout=20,
        ).stdout.strip()
    except Exception:
        vi = "?"
    print("file      :", path)
    print("filever   :", vi)

    assert data[:2] == b"MZ"
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    coff = pe + 4
    nsec = struct.unpack_from("<H", data, coff + 2)[0]
    opt = coff + 20
    magic = struct.unpack_from("<H", data, opt)[0]
    opt_size = struct.unpack_from("<H", data, coff + 16)[0]
    sec_off = opt + opt_size
    sections = []
    for i in range(nsec):
        base = sec_off + i * 40
        name = data[base : base + 8].rstrip(b"\0").decode("latin1")
        vsz, vaddr, rsize, raddr = struct.unpack_from("<IIII", data, base + 8)
        sections.append((name, vaddr, vsz, raddr, rsize))
    text = next(s for s in sections if s[0] == ".text")
    _, tvaddr, _, traddr, _ = text
    blob = data[traddr : traddr + text[2]]

    hits = []
    idx = 0
    while True:
        j = blob.find(SIG, idx)
        if j < 0:
            break
        hits.append(tvaddr + j)
        idx = j + 1
    print("sig(13B)  :", " ".join("%02x" % b for b in SIG))
    print("matches   :", len(hits), [hex(h) for h in hits])
    if len(hits) == 1:
        rva = hits[0]
        full_ok = SIG_FULL in blob
        print("driver RVA:", hex(rva), " full-prologue-ok:", full_ok)
        print("table entry: 0x%x" % rva)
    else:
        print("driver RVA: NOT UNIQUE — cannot hook safely")


if __name__ == "__main__":
    main()
