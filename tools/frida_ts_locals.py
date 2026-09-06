"""frida: at driver 0x15650 entry (26x = one GSUB lookup application over the
whole run), the outer-frame locals r8/r9/rsi/rdi/rax are adjacent stack
pointers. Dump u16 content AT each and one deref level each fire; the local
whose content transitions toward [675,281,303,471,281,351] across the 26
fires is the glyph run buffer."""
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
function u32(p) { try { return p.readU32(); } catch (e) { return -1; } }
function u64(p) { try { return p.readU64(); } catch (e) { return null; } }
function dump(p, name, out) {
  if (!isR(p)) return;
  const v16 = u16s(p, 12);
  const v32 = [];
  for (let i = 0; i < 6; i++) v32.push(u32(p.add(i * 4)));
  out.push({ name: name, hex: p.toString(16), u16: v16, u32: v32 });
  // one deref level
  const q = p.readPointer();
  if (!q.isNull() && isR(q)) {
    const q16 = u16s(q, 12);
    if (q16 && q16.some(x => x !== 0)) {
      out.push({ name: name + '->', hex: q.toString(16), u16: q16 });
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
        dump(c.rax, 'rax', out);
        send({ n: n, dumps: out });
      }
    });
    send({ log: 'attached 0x15650 dump-locals' });
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
