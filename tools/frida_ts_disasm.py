"""frida: reliably disassemble TextShaping functions (capstone via frida) to
find how args are dereferenced. We want to know which arg/offset of 0x25350
(the GSUB-cache wrapper) is the glyph-array pointer + count."""
import sys
import time
import frida

# RVAs to disassemble
RVAS = [0x25350, 0xc1c0, 0x15712, 0x1223b, 0xf8cd, 0x636f]

JS_TMPL = r"""
'use strict';
let attached = false;
const rvas = %s;
const want = %s; // 0x25350 wanted for full dump
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  for (const rva of rvas) {
    try {
      const p = ts.base.add(rva);
      const insns = [];
      const max = want === rva ? 0x200 : 0x60;
      let cur = p;
      const end = p.add(max);
      // simple linear walk via Instruction.parse (capstone, reliable)
      for (let i = 0; i < 400 && cur.compare(end) < 0; i++) {
        const ins = Instruction.parse(cur);
        insns.push(ins.address.sub(ts.base).toString(16) + ': ' + ins.mnemonic + ' ' + ins.opStr);
        cur = ins.next;
      }
      send({ rva: rva, insns: insns });
    } catch (e) {
      send({ rva: rva, err: e.message });
    }
  }
}
setInterval(tryAttach, 2);
"""


def main():
    args = sys.argv[1:]
    out_path = None
    want = 0x25350
    for i, a in enumerate(args):
        if a == "--out" and i + 1 < len(args):
            out_path = args[i + 1]
            args = args[:i] + args[i + 2:]
            break
        if a == "--want" and i + 1 < len(args):
            want = int(args[i + 1], 0)
            args = args[:i] + args[i + 2:]
            break
    dev = frida.get_local_device()
    pid = dev.spawn(args, env={"PYUSP_PRESLEEP_MS": "1500",
                               "PYUSP_PRELOAD_TS": "1",
                               "PYUSP_POSTLOAD_MS": "3000"})
    session = dev.attach(pid)
    js = JS_TMPL % (str(RVAS).replace(" ", ""), hex(want))

    def on_message(msg, data):
        if msg["type"] == "send":
            line = str(msg["payload"])
        elif msg["type"] == "error":
            line = "FRIDA-ERR: " + str(msg.get("description", msg))
        else:
            return
        if out_path:
            with open(out_path, "a", encoding="utf-8") as f:
                f.write(line + "\n")
        else:
            print(line, flush=True)

    script = session.create_script(js)
    script.on("message", on_message)
    script.load()
    dev.resume(pid)
    time.sleep(8)
    try:
        session.detach()
    except Exception:
        pass


if __name__ == "__main__":
    main()
