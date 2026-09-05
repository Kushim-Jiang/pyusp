"""Enumerate loaded modules of a child that shapes once then sleeps, to see
whether the modern shaping engines (textshaping/dwrite) get loaded.

Usage: python tools/frida_modules.py
"""
import sys
import time

import frida

JS = r"""
send({ mods: Process.enumerateModules().filter(
  m => /text|dwrite|usp|gdi|shap/i.test(m.name)).map(m => m.name + ' @' + m.base) });
"""

CHILD = [
    r"D:\Github\pyusp\.venvtest\Scripts\python.exe",
    "-c",
    (
        "import pyusp,time;"
        "d=open(r'D:\\Github\\mongolian-utn\\temp\\hudum.otf','rb').read();"
        "pyusp.shape_with_uniscribe(d,'\\u1830\\u1820\\u1822\\u182c\\u1820\\u1828');"
        "print('shaped ok');time.sleep(30)"
    ),
]


def main():
    dev = frida.get_local_device()
    pid = dev.spawn(CHILD)
    session = dev.attach(pid)
    result = {}

    def on_message(msg, data):
        if msg.get("type") == "send":
            result.update(msg["payload"])

    script = session.create_script(JS)
    script.on("message", on_message)
    script.load()
    dev.resume(pid)
    deadline = time.time() + 20
    while "mods" not in result and time.time() < deadline:
        time.sleep(0.2)
    try:
        session.detach()
        dev.kill(pid)
    except Exception:
        pass
    for m in result.get("mods", []):
        print(m)
    return 0


if __name__ == "__main__":
    sys.exit(main())
