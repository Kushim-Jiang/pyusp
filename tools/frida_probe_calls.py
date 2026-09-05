"""frida dynamic probe — count internal gdi32full call-target executions during
a single ScriptShapeOpenType call (locate the per-lookup dispatcher).

Usage:
    python tools/frida_probe_calls.py <exe> --font <f> --text <t> --out <counts.json>
"""
import json
import sys
import time

import frida

JS = r"""
let counts = {};
let sites = {};
let tlo = 0;
let thi = 0;
let attached = false;
let dumpFirst = 0;
const followed = {};

function refreshRange() {
  const gdi = Process.findModuleByName('gdi32full.dll');
  if (gdi) {
    tlo = Number(gdi.base);
    thi = tlo + gdi.size;
  }
  return gdi;
}

const transformImpl = {
  transform(iterator) {
    let ins;
    while ((ins = iterator.next()) !== null) {
      if (ins.mnemonic === 'call') {
        const op0 = ins.operands[0];          if (op0 && dumpFirst < 30) {
            dumpFirst++;
            let o = {};
            for (const k in op0) { try { o[k] = String(op0[k]); } catch (e) {} }
            send({ opd: o, at: ins.address.toString(16), mn: ins.mnemonic });
          }        const siteKey = ins.address.toString(16);
        const immVal = (op0 && op0.type === 'imm') ? op0.value : null;
        const regName = (op0 && op0.type === 'reg') ? op0.value : null;
        const memRip = (op0 && op0.type === 'mem' && op0.base === 'rip');
        const memBase = (op0 && op0.type === 'mem' && op0.base !== 'rip') ? op0.base : null;
        const memAddr = memRip ? ins.address.add(ins.size).add(op0.disp) : null;
        iterator.putCallout(function (context) {
          if (tlo === 0) return;
          let dest = null;
          if (immVal !== null) dest = immVal;
          else if (regName !== null) dest = context[regName];
          else if (memBase !== null) dest = context[memBase];
          else if (memAddr !== null) {
            try { dest = memAddr.readPointer(); } catch (e) { dest = null; }
          }
          if (dest === null) return;
          const tv = Number(dest);
          if (tv >= tlo && tv < thi) {
            const dk = tv.toString(16);
            counts[dk] = (counts[dk] || 0) + 1;
            sites[dk] = sites[dk] || {};
            sites[dk][siteKey] = (sites[dk][siteKey] || 0) + 1;
          }
        });
      }
      iterator.keep();
    }
  }
};

function followAll() {
  const me = Process.getCurrentThreadId();
  for (const t of Process.enumerateThreads()) {
    if (t.id === me) continue;   // never follow the agent's own thread
    if (followed[t.id]) continue;
    try {
      Stalker.follow(t.id, transformImpl);
      followed[t.id] = true;
      send({ log: 'following tid ' + t.id });
    } catch (e) {
      send({ log: 'follow tid ' + t.id + ' failed: ' + e });
    }
  }
}

// after resume: follow every thread (main + workers) so the real engine is
// captured even if Uniscribe shapes on a worker thread.
setTimeout(followAll, 100);
setInterval(followAll, 20);

setInterval(function () {
  const gdi = refreshRange();
  if (!gdi || attached) return;
  attached = true;
  const shape = gdi.base.add(0x5a180);
  send({ log: 'gdi=' + gdi.base + ' shape=' + shape });
  Interceptor.attach(shape, {
    onEnter() {
      counts = {};
      sites = {};
    },
    onLeave() {
      send({ counts: counts, sites: sites, gdiBase: tlo });
    }
  });
  send({ log: 'attached' });
}, 2);
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
    pid = dev.spawn(args, env={"PYUSP_PRESLEEP_MS": "800"})
    session = dev.attach(pid)
    result = {}

    def on_message(msg, data):
        if msg.get("type") == "send":
            p = msg["payload"]
            if "log" in p:
                print("[probe]", p["log"])
            elif "err" in p:
                print("[probe err]", p["err"])
                if p.get("stack"):
                    print(p["stack"])
            else:
                result.update(p)
        elif msg.get("type") == "error":
            print("[probe error]", msg.get("description"))
            print("[probe error stack]", msg.get("stack"))

    script = session.create_script(JS)
    script.on("message", on_message)
    script.load()
    dev.resume(pid)
    deadline = time.time() + 60
    while "counts" not in result and time.time() < deadline:
        time.sleep(0.1)
    try:
        session.detach()
    except Exception:
        pass
    counts = result.get("counts")
    if counts is None:
        print("no counts captured; result keys:", list(result.keys()))
        return 2
    if out_path:
        with open(out_path, "w") as f:
            json.dump(result, f)
        print("saved", out_path, "targets:", len(counts))
    else:
        print(json.dumps(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
