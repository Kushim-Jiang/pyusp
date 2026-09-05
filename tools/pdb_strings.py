"""Dump candidate identifier strings from a PDB (ascii + utf-16le) to stdout."""
import re
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "tools/pdb/usp10.pdb"
want = sys.argv[2] if len(sys.argv) > 2 else ""

d = open(path, "rb").read()

ascii_set = set()
for m in re.finditer(rb"[\x20-\x7e]{4,}", d):
    s = m.group().decode("latin1")
    ascii_set.add(s)

utf16 = set()
for m in re.finditer(rb"(?:[\x20-\x7e]\x00){4,}", d):
    s = m.group().decode("utf-16le", "ignore")
    utf16.add(s)

allnames = sorted(ascii_set | utf16)
if want:
    allnames = [s for s in allnames if want.lower() in s.lower()]
for s in allnames:
    print(s)
