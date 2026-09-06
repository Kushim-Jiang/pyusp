"""frida B-plan v8: attach at confirmed-hot substitution driver continuation
0x15712 (parser returns here every glyph step). Per fire (always send):
  - [rbp+0x278] u32 = the glyph id being matched at this step
  - scan rbp+-0x600 for a window of >=4 consecutive u16 all < 907 (glyph run)
  - rbx span object dump [rbx+0x10]..[rbx+0x48]
Also hook 0xaddd / 0x2bd85 (early parser callers) minimally."""
import sys
import time
import frida

JS = r"""
'use strict';
let attached = false;
const counts = {};
const NGLYPH = 907;
function u16s(p, n) {
  try {
    const out = [];
    for (let i = 0; i < n; i++) out.push(p.add(i * 2).readU16());
    return out;
  } catch (e) { return null; }
}
function isReadable(p) {
  try { p.readU8(); return true; } catch (e) { return false; }
}
function findRun(frame, rbp) {
  // find a window of 4+ consecutive u16 all in [0,NGLYPH) near rbp
  try {
    const base = frame;
    for (let off = 0; off < 0xc00; off += 2) {
      const p = base.add(off);
      if (!isReadable(p)) return null;
      let ok = 0;
      const vals = [];
      for (let i = 0; i < 8; i++) {
        let v;
        try { v = p.add(i * 2).readU16(); } catch (e) { break; }
        if (v >= NGLYPH) break;
        vals.push(v);
        ok++;
      }
      if (ok >= 4) {
        const disp = (p.compare(rbp) >= 0 ? '+' : '-') +
                     '0x' + Math.abs(p.sub(rbp)).toString(16);
        return { disp: disp, run: JSON.stringify(vals) };
      }
    }
  } catch (e) {}
  return null;
}
function dumpSpan(rbx) {
  try {
    const start = rbx.add(0x10).readPointer();
    const end = rbx.add(0x48).readPointer();
    if (start.isNull() || end.isNull()) return null;
    const n = Math.floor(end.sub(start) / 2);
    if (n < 1 || n > 64) return null;
    const v = u16s(start, n);
    if (!v) return null;
    return JSON.stringify(v);
  } catch (e) { return null; }
}
function tryAttach() {
  const ts = Process.findModuleByName('TextShaping.dll');
  if (!ts || attached) return;
  attached = true;
  send({ log: 'ts=' + ts.base });
  const sites = [
    { rva: 0x15712, tag: '15712', readKey: true },
    { rva: 0xaddd, tag: 'addd', readKey: false },
    { rva: 0x2bd85, tag: '2bd85', readKey: false },
    { rva: 0x2bdde, tag: '2bdde', readKey: false }
  ];
  for (const s of sites) {
    try {
      counts[s.tag] = 0;
      Interceptor.attach(ts.base.add(s.rva), {
        onEnter() {
          counts[s.tag]++;
          const n = counts[s.tag];
          if (n > 200) return;
          const ctx = this.context;
          const rbp = ctx.rbp;
          const rec = { site: s.tag, n: n };
          if (s.readKey && !rbp.isNull() && isReadable(rbp.add(0x278))) {
            rec.key = rbp.add(0x278).readU32();
          }
          if (!rbp.isNull()) {
            const run = findRun(rbp.sub(0x600), rbp);
            if (run) rec.run = run;
          }
          const rbx = ctx.rbx;
          if (!rbx.isNull()) {
            const sp = dumpSpan(rbx);
            if (sp) rec.rbxSpan = sp;
          }
          send(rec);
        }
      });
    } catch (e) {
      send({ log: s.tag + ' FAILED: ' + e.message });
    }
  }
  send({ log: 'attached all, per-fire always send' });
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
    time.sleep(16)
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
