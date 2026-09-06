/* test_bytesmode.c — bytes-mode A/B harness (cross-platform M0-2 check).
 *
 * Mirrors the pyusp engine's shape_item flow but drives ScriptShapeOpenType
 * with hdc == NULL and raw font bytes registered via usp_set_font_bytes().
 * Prints the per-lookup trace (cmap / <feat>#k/n / final) + the final glyph
 * run, so it can be compared byte-for-byte against the GDI (engine) reference.
 *
 * Build (mingw):
 *   gcc -O2 -I port\include -I port\src test_bytesmode.c <wineusp .o files> \
 *       -lgdi32 -luser32 -lkernel32 -o test_bytesmode.exe
 * Run: test_bytesmode.exe <font.otf>  (text hardcoded: Mongolian saikhan)
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

#include "windef.h"
#include "wingdi.h"
#include "usp10.h"
#include "fontbytes.h"

/* exported by wineusp (trace.c) */
extern int usp_trace_begin(void);
extern void usp_trace_stop(void);
extern int usp_trace_count(void);
extern int usp_trace_stage(int i, char *name_out, int name_len,
                           int *glyph_count_out, const WORD **glyphs_out);

static WCHAR text[] = { 0x1830, 0x1820, 0x1822, 0x182C, 0x1820, 0x1828 };
#define NTEXT ((int)(sizeof(text)/sizeof(text[0])))
#define TAG_MONG 0x676E6F6Du /* 'mong' */
#define TAG_DFLT 0x64666C74u /* 'dflt' */

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
    DWORD tags[16];
    int nItems = 0;
    SCRIPT_CACHE psc = NULL;
    SCRIPT_ANALYSIS psa;
    WORD logclust[64], glyphs[64];
    SCRIPT_CHARPROP cp[64];
    SCRIPT_GLYPHPROP gp[64];
    int ng = 0, i, j;
    int nst;
    OPENTYPE_TAG tagScript, tagLangSys;

    if (!fontpath) { fprintf(stderr, "usage: test_bytesmode <font.otf>\n"); return 2; }
    f = fopen(fontpath, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", fontpath); return 2; }
    fseek(f, 0, SEEK_END); len = ftell(f); fseek(f, 0, SEEK_SET);
    blob = malloc((size_t)len);
    if (fread(blob, 1, (size_t)len, f) != (size_t)len) { fprintf(stderr, "read fail\n"); return 2; }
    fclose(f);

    if (usp_set_font_bytes(blob, (size_t)len) != 0) { fprintf(stderr, "usp_set_font_bytes failed\n"); return 2; }

    /* byte-layer self-check (tags in Wine MS_MAKE_TAG / little-endian order) */
    {
        DWORD r0 = usp_fb_GetFontData(NULL, 0x64616568 /* 'head' le */, 0, NULL, 0);
        WORD g0 = 0xFFFF;
        DWORD gn = usp_fb_GetGlyphIndicesW(NULL, &text[0], 1, &g0, 0);
        printf("debug: head_len=%u upem=%u cmap_n=%u g[0x1830]=%u\n",
               r0, usp_fb_upem(), gn, g0);
    }

    memset(&ctrl, 0, sizeof(ctrl));
    memset(&state, 0, sizeof(state));
    hr = ScriptItemizeOpenType(text, NTEXT, 16, &ctrl, &state, items, tags, &nItems);
    if (hr != 0 || nItems < 1) { fprintf(stderr, "ScriptItemizeOpenType hr=%lx n=%d\n", (unsigned long)hr, nItems); return 2; }

    /* item 0 (whole run for this text) */
    psa = items[0].a;
    psa.fLogicalOrder = 1;          /* mirror engine A_FLOGICAL_ORDER */
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
    if (hr != 0) { fprintf(stderr, "ScriptShapeOpenType(bytes) hr=%lx\n", (unsigned long)hr); return 2; }

    printf("tagScript=%c%c%c%c ng=%d\n",
           (int)((tagScript>>24)&0xff), (int)((tagScript>>16)&0xff),
           (int)((tagScript>>8)&0xff), (int)(tagScript&0xff), ng);

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
    printf("final:");
    for (j = 0; j < ng; j++) printf(" %u", glyphs[j]);
    printf("\n");

    usp_clear_font_bytes();
    free(blob);
    return 0;
}
