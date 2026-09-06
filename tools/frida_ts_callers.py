"""frida: capture parser 0xc1c0's REAL return addresses (callers) via
this.returnAddress, to find which driver functions actually call the parser on
the Mongolian path."""
import sys
import time
import frida

JS = r"""
'use strict';
let attached = false;
let fires = 0;
const rets = {};
const tsBase = () => Process.findModuleByName('TextShaping.dll').base;
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  try {
    Interceptor.attach(ts.base.add(0xc1c0), {
      onEnter() {
        fires++;
        if (fires > 70) return;
        try {
          const off = this.returnAddress.sub(ts.base).toInt32() >>> 0;
          send({ site: 'parser', n: fires, ra: '0x' + off.toString(16) });
        } catch (e) {
          send({ site: 'parser', n: fires, ra: 'ERR ' + e.message });
        }
      }
    });
    Interceptor.attach(ts.base.add(0x25350), {
      onEnter() {
        fires++;
        if (fires < 60 || fires > 120) return; // cache region of the count
        try {
          const off = this.returnAddress.sub(ts.base).toInt32() >>> 0;
          send({ site: 'cache', n: fires, ra: '0x' + off.toString(16) });
        } catch (e) {
          send({ site: 'cache', n: fires, ra: 'ERR ' + e.message });
        }
      }
    });
    send({ log: 'attached parser+cache per-fire' });
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
