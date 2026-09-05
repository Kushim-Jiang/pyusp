"""Download usp10.pdb from the Microsoft symbol server for the local usp10.dll.

Usage: python tools/get_usp10_pdb.py [dll_path] [out_dir]
  dll_path default: C:\\Windows\\System32\\usp10.dll
  out_dir  default: tools/pdb
"""
import os
import struct
import sys
import urllib.request

MSDL = "https://msdl.microsoft.com/download/symbols"


def pe_debug_guid(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:2] != b"MZ":
        raise SystemExit("not a PE")
    e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    if data[e_lfanew : e_lfanew + 4] != b"PE\x00\x00":
        raise SystemExit("no PE signature")
    opt = e_lfanew + 24
    magic = struct.unpack_from("<H", data, opt)[0]
    is_pe32 = magic == 0x10B
    # DataDirectory[6] = debug directory
    if is_pe32:
        dd_off = opt + 96 + 6 * 8
    else:
        dd_off = opt + 112 + 6 * 8
    dbg_rva, dbg_size = struct.unpack_from("<II", data, dd_off)
    if not dbg_rva:
        raise SystemExit("no debug directory")
    # map RVA -> file offset
    nsec = struct.unpack_from("<H", data, e_lfanew + 6)[0]
    sec_off = opt + struct.unpack_from("<H", data, e_lfanew + 20)[0]
    off = None
    for i in range(nsec):
        s = sec_off + i * 40
        va, vsize, raw, rsize = struct.unpack_from("<IIII", data, s + 12)
        if va <= dbg_rva < va + max(vsize, rsize):
            off = dbg_rva - va + raw
            break
    if off is None:
        raise SystemExit("debug dir not in a section")
    # IMAGE_DEBUG_DIRECTORY entries (28 bytes); Type==2 is CodeView
    for i in range(0, dbg_size, 28):
        e = off + i
        typ = struct.unpack_from("<I", data, e + 12)[0]
        if typ == 2:
            rawptr = struct.unpack_from("<I", data, e + 24)[0]
            if data[rawptr : rawptr + 4] == b"RSDS":
                guid = data[rawptr + 4 : rawptr + 20]
                age = struct.unpack_from("<I", data, rawptr + 20)[0]
                pdb = data[rawptr + 24 : rawptr + 24 + 300].split(b"\x00")[0].decode("latin1")
                return guid, age, pdb
    raise SystemExit("no RSDS entry")


def guid_key(guid, age):
    g = guid
    d1 = struct.unpack("<I", g[0:4])[0]
    d2 = struct.unpack("<H", g[4:6])[0]
    d3 = struct.unpack("<H", g[6:8])[0]
    rest = g[8:16]
    s = "%08X%04X%04X%s" % (d1, d2, d3, rest.hex().upper())
    return "%s%X" % (s, age)


def main():
    dll = sys.argv[1] if len(sys.argv) > 1 else r"C:\Windows\System32\usp10.dll"
    outdir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "pdb"
    )
    guid, age, pdbpath = pe_debug_guid(dll)
    key = guid_key(guid, age)
    name = os.path.basename(pdbpath)
    url = f"{MSDL}/{name}/{key}/{name}"
    os.makedirs(outdir, exist_ok=True)
    dest = os.path.join(outdir, name)
    print(f"dll={dll}")
    print(f"guid/age key={key}")
    print(f"pdb path in dll: {pdbpath}")
    print(f"downloading {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "pyusp-dev"})
    with urllib.request.urlopen(req) as r, open(dest, "wb") as f:
        f.write(r.read())
    print(f"saved {dest} ({os.path.getsize(dest)} bytes)")


if __name__ == "__main__":
    main()
