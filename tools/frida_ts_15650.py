"""frida: hook driver 0x15650 ENTRY (fires 26x, called by the outer per-glyph
loop). Dump arg regs + r13 at entry to find the glyph cursor/code that the
outer driver passes per call."""
import sys
import time
import frida

JS = r"""
'use strict';
let attached = false;
let n = 0;
function isR(p) { try { p.readU8(); return true; } catch (e) { return false; } }
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  try {
    Interceptor.attach(ts.base.add(0x15650), {
      onEnter(args) {
        n++;
        if (n > 60) return;
        const c = this.context;
        const rec = { n: n,
          ecx: (c.ecx >>> 0).toString(16), edx: (c.edx >>> 0).toString(16),
          r8: c.r8.toString(16), r9: c.r9.toString(16),
          r13: c.r13.toString(16), r14: c.r14.toString(16),
          rsi: c.rsi.toString(16), rdi: c.rdi.toString(16),
          rax: c.rax.toString(16) };
        // try interpret low regs as small numbers
        send(rec);
      }
    });
    send({ log: 'attached 0x15650 entry' });
  } catch (e) {
    send({ log: 'attach FAILED: ' + e.message });
  }
}
setInterval(tryAttach, 2);
"""


def main():
    args = sys.argv[1:]
    out_path = None
    for i, a in enumerate(args):
        if a == "--out" and i + 1 < len(args):
            out_path = args[i + 1]
            args = args[:i] + args[i + 2:]
            break
    dev = frida.get_local_device()
    pid = dev.spawn(args, env={"PYUSP_PRESLEEP_MS": "1500",
                               "PYUSP_PRELOAD_TS": "1",
                               "PYUSP_POSTLOAD_MS": "3000"})
    session = dev.attach(pid)

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

    script = session.create_script(JS)
    script.on("message", on_message)
    script.load()
    dev.resume(pid)
    time.sleep(14)
    try:
        session.detach()
    except Exception:
        pass
    try:
        dev.kill(pid)
    except Exception:
        pass


if __name__ == "__main__":
    main()
