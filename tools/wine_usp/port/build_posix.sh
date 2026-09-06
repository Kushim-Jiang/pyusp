#!/bin/sh
# build_posix.sh — compile the Wine Uniscribe port natively for Linux/macOS.
#
# Produces libwineusp.so (Linux) / libwineusp.dylib (macOS) in this directory
# from port/src + port/posix (POSIX shims). No Windows SDK / gdi32 needed: the
# shaping runs in "bytes mode" (usp_set_font_bytes + NULL hdc), see the
# README/SPIKE notes and posix/posix_compat.c.
#
# Flags:
#   -fshort-wchar  wchar_t is 16-bit so Wine's L"" literals match WCHAR.
#   -fPIC          required for shared libraries.
#
# The same sources compile on Windows/MinGW with the *real* Windows headers
# (port/include + MinGW) — port/posix is only added to the include path for
# the POSIX build.
set -e
cd "$(dirname "$0")"

CC="${CC:-cc}"
SRCS="
  src/bidi.c
  src/bracket.c
  src/breaking.c
  src/direction.c
  src/fontbytes.c
  src/indic.c
  src/indicsyllable.c
  src/linebreak.c
  src/mirror.c
  src/opentype.c
  src/shape.c
  src/shaping.c
  src/trace.c
  src/usp10.c
  posix/posix_compat.c
"
CFLAGS="-O2 -std=gnu11 -fPIC -fshort-wchar -I posix -I include -I src"
mkdir -p build

objs=""
for f in $SRCS; do
    o="build/$(basename "$f" .c).posix.o"
    $CC $CFLAGS -c "$f" -o "$o"
    objs="$objs $o"
done

case "$(uname -s)" in
    Darwin)
        $CC -dynamiclib -o libwineusp.dylib $objs
        echo "built libwineusp.dylib"
        ;;
    *)
        $CC -shared -Wl,-z,defs -o libwineusp.so $objs -lm
        echo "built libwineusp.so"
        ;;
esac
