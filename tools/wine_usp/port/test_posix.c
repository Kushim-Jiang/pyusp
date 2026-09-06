/* test_posix.c — cross-platform bytes-mode parity driver (M1/M3).
 *
 * Compiles and runs natively on Linux/macOS/Windows against the port sources
 * (no GDI / no Rust): reads a font file, registers the raw bytes with
 * usp_set_font_bytes, drives ScriptItemizeOpenType -> ScriptShapeOpenType with
 * a NULL hdc, and prints the per-lookup trace (cmap -> GSUB lookups -> final)
 * as "name: g1 g2 ..." lines — the same canonical format used by the Windows
 * reference (see tools/wine_usp/testdata/noto-saikhan-windows-golden.json).
 *
 * Build (POSIX, from port/):
 *   cc -O2 -std=gnu11 -fPIC -fshort-wchar -I posix -I include -I src \
 *      test_posix.c <src/*.c + posix/posix_compat.c> -o build/test_posix
 *   (or: link against libwineusp.so + declare the Script* exports)
 * Run: ./test_posix <font.ttf>
 *
 * Default text (when argv[2] absent): Mongolian "saikhan"
 *   { 0x1830, 0x1820, 0x1822, 0x182C, 0x1820, 0x1828 }
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "windef.h"
#include "wingdi.h"
#include "usp10.h"
#include "fontbytes.h"

/* wineusp trace sink (trace.c) */
extern int usp_trace_begin(void);
extern void usp_trace_stop(void);
extern int usp_trace_count(void);
extern int usp_trace_stage(int i, char *name_out, int name_len,
                           int *glyph_count_out, const WORD **glyphs_out);

static WCHAR text[] = { 0x1830, 0x1820, 0x1822, 0x182C, 0x1820, 0x1828 };
#define NTEXT ((int)(sizeof(text) / sizeof(text[0])))
#define TAG_DFLT 0x64666C74u /* 'dflt' */
#define TAG_MONG 0x676E6F6Du /* 'mong' */

int main(int argc, char **argv)
{
    const char *fontpath = (argc > 1) ? argv[1] : NULL;
    FILE *f;
    unsigned char *blob;
    long len;
    HRESULT hr;
    SCRIPT_CONTROL ctrl;
    SCRIPT_STATE state;
    SCRIPT_ITEM items[16];
    OPENTYPE_TAG tags[16];
    int nItems = 0;
    SCRIPT_CACHE psc = NULL;
    SCRIPT_ANALYSIS psa;
    WORD logclust[64], glyphs[64];
    SCRIPT_CHARPROP cp[64];
    SCRIPT_GLYPHPROP gp[64];
    int ng = 0, i, j, nst;
    OPENTYPE_TAG tagScript, tagLangSys;

    if (!fontpath) { fprintf(stderr, "usage: test_posix <font.ttf> [text-file]\n"); return 2; }
    f = fopen(fontpath, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", fontpath); return 2; }
    fseek(f, 0, SEEK_END); len = ftell(f); fseek(f, 0, SEEK_SET);
    blob = malloc((size_t)len);
    if (fread(blob, 1, (size_t)len, f) != (size_t)len) { fprintf(stderr, "read fail\n"); return 2; }
    fclose(f);

    if (usp_set_font_bytes(blob, (size_t)len) != 0) { fprintf(stderr, "usp_set_font_bytes failed\n"); return 2; }

    memset(&ctrl, 0, sizeof(ctrl));
    memset(&state, 0, sizeof(state));
    hr = ScriptItemizeOpenType(text, NTEXT, 16, &ctrl, &state, items, tags, &nItems);
    if (hr != 0 || nItems < 1) { fprintf(stderr, "ScriptItemizeOpenType hr=%lx n=%d\n", (unsigned long)hr, nItems); return 2; }

    psa = items[0].a;
    psa.fLogicalOrder = 1;
    tagScript = (OPENTYPE_TAG)tags[0];
    if (!tagScript) tagScript = TAG_MONG;
    tagLangSys = TAG_DFLT;

    memset(logclust, 0, sizeof(logclust));
    memset(cp, 0, sizeof(cp));
    memset(gp, 0, sizeof(gp));

    usp_trace_begin();
    hr = ScriptShapeOpenType(NULL, &psc, &psa, tagScript, tagLangSys,
                             NULL, NULL, 0, text, NTEXT, 64,
                             logclust, cp, glyphs, gp, &ng);
    usp_trace_stop();
    if (hr != 0) { fprintf(stderr, "ScriptShapeOpenType hr=%lx\n", (unsigned long)hr); return 2; }

    /* Emit canonical stage lines (trace.c stage names already include the
     * final run). Stable, order-preserving, parseable by the parity checker. */
    nst = usp_trace_count();
    for (i = 0; i < nst; i++)
    {
        char name[64];
        int gc = 0;
        const WORD *gptr = NULL;
        if (usp_trace_stage(i, name, sizeof(name), &gc, &gptr) != 0) continue;
        printf("%s:", name);
        for (j = 0; j < gc; j++) printf(" %u", gptr[j]);
        printf("\n");
    }

    usp_clear_font_bytes();
    free(blob);
    return 0;
}
