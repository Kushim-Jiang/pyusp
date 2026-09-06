"""frida: at 0x15650 entry, dump 64 u16 at the deref of r8/r9/rsi/rdi each of
the 26 fires to capture the FULL glyph-run progression per lookup application.
Also record the pointers so we can follow one buffer's evolution."""
import sys
import time
import frida

JS = r"""
'use strict';
let attached = false;
let n = 0;
function isR(p) { try { p.readU8(); return true; } catch (e) { return false; } }
function u16s(p, cnt) {
  try {
    const o = [];
    for (let i = 0; i < cnt; i++) o.push(p.add(i * 2).readU16());
    return o;
  } catch (e) { return null; }
}
function dump(p, name, out) {
  if (!p.isNull() && isR(p)) {
    const q = p.readPointer();
    if (!q.isNull() && isR(q)) {
      const v = u16s(q, 64);
      if (v) out.push({ name: name + '->', hex: q.toString(16), u16: v });
    }
  }
}
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  try {
    Interceptor.attach(ts.base.add(0x15650), {
      onEnter() {
        n++;
        if (n > 40) return;
        const c = this.context;
        const out = [];
        dump(c.r8, 'r8', out);
        dump(c.r9, 'r9', out);
        dump(c.rsi, 'rsi', out);
        dump(c.rdi, 'rdi', out);
        send({ n: n, dumps: out });
      }
    });
    send({ log: 'attached 0x15650 full-run dump' });
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
