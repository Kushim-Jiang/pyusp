"""frida: dump the run buffer (r9->) in onLeave of 0x15650 so we see the state
right AFTER each driver application returns. If the true final
[675,281,303,471,281,351] is reached by the last 0x15650 return, this confirms
the run buffer + lets us snapshot per-application states."""
import sys
import time
import frida

JS = r"""
'use strict';
let attached = false;
let n = 0;
let curRun = null;
function isR(p) { try { p.readU8(); return true; } catch (e) { return false; } }
function u16s(p, cnt) {
  try {
    const o = [];
    for (let i = 0; i < cnt; i++) o.push(p.add(i * 2).readU16());
    return o;
  } catch (e) { return null; }
}
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  try {
    Interceptor.attach(ts.base.add(0x15650), {
      onEnter(args) {
        n++;
        if (n > 40) return;
        const r9p = this.context.r9;
        curRun = null;
        if (!r9p.isNull() && isR(r9p)) {
          const q = r9p.readPointer();
          if (!q.isNull() && isR(q)) curRun = q;
        }
      },
      onLeave() {
        if (curRun) {
          const v = u16s(curRun, 32);
          if (v) {
            // extract 6 gids at stride 4
            const gids = [v[0], v[4], v[8], v[12], v[16], v[20]];
            send({ n: n, gids: gids, hex: curRun.toString(16),
                   raw: v.slice(0, 24) });
          }
        }
      }
    });
    send({ log: 'attached 0x15650 onLeave dump' });
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
