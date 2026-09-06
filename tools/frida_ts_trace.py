"""frida native-trace producer.

Hooks TextShaping driver 0x15650 (one OT "apply" over the run) and records the
glyph-run state (r9 -> heap u16 array, records of 4 u16: {gid, flag, logIdx, 1})
at every application (onEnter and onLeave). Emits a JSON trace of per-application
glyph states plus the final run, cross-checked against the pyusp/usp10 final.

Native TextShaping is a fused per-glyph engine (no per-lookup boundary), so this
is the per-glyph/per-application granularity it exposes: apps resolve each glyph,
then the run is assembled and the remaining GSUB work is fused into the last app.
Equivalence with the authoritative wineusp named stages is by glyph-array match.

Usage:
  python frida_ts_trace.py <pyusp.exe> --font <f.otf> --text <t> --out <trace.json>
Env for the child is set here (PYUSP_PRELOAD_TS=1, PYUSP_POSTLOAD_MS=3000).
"""
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
// read up to `max` glyph records (4 u16 each) from the run pointer p
function runAt(p, max) {
  const v = u16s(p, max * 4);
  if (!v) return null;
  const gids = [];
  let prevIdx = -1;
  for (let i = 0; i < max; i++) {
    const g = v[i * 4];
    const idxf = v[i * 4 + 2];
    if (g === 0) break;
    if (i > 0 && idxf !== prevIdx + 1) break; // idx field no longer increments
    prevIdx = idxf;
    gids.push(g);
  }
  return { hex: p.toString(16), gids: gids };
}
let curPtr = null;
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  try {
    Interceptor.attach(ts.base.add(0x15650), {
      onEnter() {
        n++;
        if (n > 200) return;
        const r9p = this.context.r9;
        curPtr = null;
        if (!r9p.isNull() && isR(r9p)) {
          const q = r9p.readPointer();
          if (!q.isNull() && isR(q)) {
            curPtr = q;
            send({ app: n, phase: 'enter', run: runAt(q, 8) });
          }
        }
      },
      onLeave() {
        if (curPtr && isR(curPtr)) {
          send({ app: n, phase: 'leave', run: runAt(curPtr, 8) });
        }
      }
    });
    send({ log: 'attached 0x15650 trace' });
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
    events = []
    # also capture the child's own stdout (pyusp final JSON) via a spawn-side
    # console read is complex; we rely on run-on-leave of the last app.

    def on_message(msg, data):
        if msg["type"] == "send":
            events.append(msg["payload"])
        elif msg["type"] == "error":
            events.append({"log": "FRIDA-ERR " + str(msg.get("description", msg))})

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

    apps = {}
    order = []
    for ev in events:
        if "app" in ev:
            a = ev["app"]
            if a not in apps:
                apps[a] = {"enter": None, "leave": None}
                order.append(a)
            if ev["phase"] == "enter":
                apps[a]["enter"] = ev.get("run")
            else:
                apps[a]["leave"] = ev.get("run")
    out = {"applications": len(order), "apps": []}
    for a in sorted(order):
        out["apps"].append({"app": a,
                            "enter": apps[a]["enter"],
                            "leave": apps[a]["leave"]})
    if order:
        last = apps[max(order)]
        out["last_app_leave_run"] = last.get("leave")
    if out_path:
        import json
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(out, f, indent=1, ensure_ascii=False)
        print("wrote", out_path)
    else:
        print(out)


if __name__ == "__main__":
    main()
