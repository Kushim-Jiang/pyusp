# Ghidra headless post-script: dump ScriptShapeOpenType's decompilation and its
# direct/indirect call tree, to locate the internal OT lookup machinery.
# Run: analyzeHeadless <proj> <name> -import gdi32full.dll -scriptPath <dir> \
#        -postScript dump_shape.py
# @category Analysis

import os

from ghidra.app.decompiler import DecompInterface
from ghidra.util.task import ConsoleTaskMonitor

TARGET = 0x18005a180
OUTDIR = os.environ.get("PYUSP_GHIDRA_OUT", r"D:\Github\pyusp\tools\ghidra\out")

prog = getCurrentProgram()
fm = prog.getFunctionManager()
listing = prog.getListing()
monitor = ConsoleTaskMonitor()

os.makedirs(OUTDIR, exist_ok=True)

fn = fm.getFunctionAt(prog.getAddressFactory().getDefaultAddressSpace().getAddress(TARGET))
if fn is None:
    print("no function at %#x" % TARGET)
else:
    print("target fn: %s @ %s size=%d" % (fn.getName(), fn.getEntryPoint(), fn.getBody().getNumAddresses()))

# --- decompile target & write to file ---
deco = DecompInterface()
deco.openProgram(prog)
out = None
if fn is not None:
    res = deco.decompileFunction(fn, 120, monitor)
    if res and res.decompileCompleted():
        with open(os.path.join(OUTDIR, "ScriptShapeOpenType.c"), "w") as f:
            f.write(res.getDecompiledFunction().getC())
        print("wrote ScriptShapeOpenType.c")

# --- walk call tree depth 3, collecting function addresses/names ---
visited = set()
results = []

def walk(addr, depth):
    if depth > 3 or addr in visited:
        return
    visited.add(addr)
    f = fm.getFunctionAt(prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr))
    if f is None:
        return
    results.append((depth, addr, f.getName(), f.getBody().getNumAddresses()))
    # collect call destinations
    callees = set()
    inst = listing.getInstructions(f.getBody(), True)
    while inst.hasNext() and not monitor.isCancelled():
        i = inst.next()
        refs = i.getReferencesFrom()
        for r in refs:
            if r.getReferenceType().isCall():
                callees.add(r.getToAddress().getOffset())
    for c in sorted(callees):
        walk(c, depth + 1)

if fn is not None:
    walk(TARGET, 0)

with open(os.path.join(OUTDIR, "calltree.txt"), "w") as f:
    for depth, addr, name, size in results:
        f.write("%d %#x %s size=%d\n" % (depth, addr, name, size))
print("wrote calltree.txt with %d nodes" % len(results))
