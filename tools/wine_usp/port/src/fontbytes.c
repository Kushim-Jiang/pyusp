/* port/src/fontbytes.c — font-bytes provider for cross-platform shaping.
 *
 * When the engine registers a font's raw bytes via usp_set_font_bytes(), the
 * NtGdiGetFontData / NtGdiGetGlyphIndicesW / NtGdiGetTextCharsetInfo calls the
 * Wine Uniscribe port makes are answered directly from those bytes (OT table
 * fetch + cmap), instead of from a Windows GDI device context. This lets the
 * same GSUB/GPOS shaping + per-lookup trace run on platforms with no GDI/HDC.
 *
 * When no bytes are registered (Windows build, GDI path), each dispatch falls
 * back to the real gdi32 function — behaviour is byte-for-byte unchanged.
 *
 * LGPL-2.1-or-later (Wine Uniscribe port extension).
 */
#include <stdint.h>
#include <stddef.h>
#include <string.h>

#include "windef.h"
#include "wingdi.h"
#include "winnls.h"
#include "usp10.h"

/* Self include guard: inside this implementation file the GDI identifiers are
 * the REAL functions (used for fallback), so don't macro-remap them. */
#define PORT_FONTBYTES_IMPL
#include "fontbytes.h"

/* ---- helpers ---- */
static uint32_t be32(const unsigned char *p) { return ((uint32_t)p[0]<<24)|((uint32_t)p[1]<<16)|((uint32_t)p[2]<<8)|p[3]; }
static uint16_t be16(const unsigned char *p) { return (uint16_t)((p[0]<<8)|p[1]); }

/* ---- registered font-bytes context ---- */
static const unsigned char *g_data = NULL;
static size_t g_len = 0;
static int g_active = 0;

int usp_set_font_bytes(const void *data, size_t len)
{
    /* basic sanity: has sfnt version + table directory */
    if (!data || len < 12) { g_active = 0; return -1; }
    {
        const unsigned char *d = (const unsigned char *)data;
        unsigned ntables = be16(d + 4);
        if (ntables == 0 || (size_t)12 + (size_t)ntables * 16 > len) { g_active = 0; return -1; }
    }
    g_data = (const unsigned char *)data;
    g_len = len;
    g_active = 1;
    return 0;
}

void usp_clear_font_bytes(void) { g_active = 0; g_data = NULL; g_len = 0; }

int usp_font_bytes_active(void) { return g_active; }

/* Locate an OT table by 4-char tag. Returns (offset,length) or (0,0). */
static int find_table(uint32_t tag, uint32_t *off, uint32_t *length)
{
    const unsigned char *d = g_data;
    unsigned ntables = be16(d + 4);
    unsigned i;
    for (i = 0; i < ntables; i++)
    {
        const unsigned char *rec = d + 12 + (size_t)i * 16;
        if (be32(rec) == tag)
        {
            *off = be32(rec + 8);
            *length = be32(rec + 12);
            if ((size_t)*off + *length <= g_len) return 1;
            return 0;
        }
    }
    return 0;
}

uint32_t usp_fb_GetFontData(HDC hdc, uint32_t tag, uint32_t offset, void *buf, uint32_t len)
{
    if (g_active)
    {
        uint32_t off = 0, length = 0;
        if (!find_table(tag, &off, &length)) return (uint32_t)-1; /* GDI_ERROR */
        if (offset >= length) return (uint32_t)-1;
        if (!buf) return length - offset;
        {
            uint32_t avail = length - offset;
            uint32_t n = (len < avail) ? len : avail;
            memcpy(buf, g_data + off + offset, n);
            return n;
        }
    }
    return GetFontData(hdc, tag, offset, buf, len);
}

/* cmap glyph lookup: returns glyph or 0xFFFF if missing (GGI flag aware). */
static uint32_t cmap_lookup(uint32_t ch)
{
    uint32_t off = 0, length = 0;
    uint32_t best = 0, best_score = 0;
    const unsigned char *d;
    unsigned i;
    unsigned ntab;
    if (!find_table(0x636D6170 /*cmap*/, &off, &length)) return 0xFFFF;
    d = g_data + off;
    ntab = be16(d + 2);
    for (i = 0; i < ntab; i++)
    {
        const unsigned char *r = d + 4 + (size_t)i * 8;
        unsigned pid = be16(r), eid = be16(r + 2);
        uint32_t sub = be32(r + 4);
        unsigned score;
        if ((size_t)sub + 2 > length) continue;
        /* prefer Windows BMP(3,1), then Unicode full(0,4), BMP(0,3), then 3,10 */
        if (pid == 3 && eid == 1) score = 5;
        else if (pid == 0 && eid == 4) score = 4;
        else if (pid == 0 && eid == 3) score = 3;
        else if (pid == 3 && eid == 10) score = 2;
        else if (pid == 0 && eid == 2) score = 1;
        else continue;
        if (score > best_score) { best_score = score; best = sub; }
    }
    if (!best) return 0xFFFF;
    d = g_data + off + best;
    {
        uint16_t fmt = be16(d);
        if (fmt == 4 && (size_t)best + 16 <= length)
        {
            uint16_t segx2 = be16(d + 6);
            const unsigned char *endc  = d + 14;
            const unsigned char *startc = endc + segx2 + 2;
            const unsigned char *idelta = startc + segx2;
            const unsigned char *iro = idelta + segx2;
            unsigned nseg = segx2 / 2;
            unsigned s;
            for (s = 0; s < nseg; s++)
            {
                uint16_t e = be16(endc + s * 2);
                if (ch <= e)
                {
                    uint16_t st = be16(startc + s * 2);
                    if (ch < st) return 0xFFFF;
                    {
                        int16_t delta = (int16_t)be16(idelta + s * 2);
                        uint16_t ro = be16(iro + s * 2);
                        if (ro == 0) return (uint32_t)(((uint32_t)ch + (uint16_t)delta) & 0xFFFF);
                        {
                            const unsigned char *gp = iro + s * 2 + ro + (uint32_t)(ch - st) * 2;
                            if ((size_t)(gp - (g_data + off)) + 2 <= length)
                            {
                                uint16_t g = be16(gp);
                                if (g == 0) return 0xFFFF;
                                return (uint32_t)(((uint32_t)g + (uint16_t)delta) & 0xFFFF);
                            }
                            return 0xFFFF;
                        }
                    }
                }
            }
            return 0xFFFF;
        }
        else if (fmt == 12 && (size_t)best + 16 <= length)
        {
            uint32_t ngroups = be32(d + 12);
            const unsigned char *gr = d + 16;
            uint32_t lo = 0, hi = ngroups;
            while (lo < hi)
            {
                uint32_t mid = (lo + hi) / 2;
                const unsigned char *g = gr + (size_t)mid * 12;
                uint32_t sc = be32(g), ec = be32(g + 4), sg = be32(g + 8);
                if (ch < sc) hi = mid;
                else if (ch > ec) lo = mid + 1;
                else return sg + (ch - sc);
            }
            return 0xFFFF;
        }
        else if (fmt == 6 && (size_t)best + 10 <= length)
        {
            uint16_t first = be16(d + 6), n = be16(d + 8);
            if (ch >= first && ch - first < n) return be16(d + 10 + (ch - first) * 2);
            return 0xFFFF;
        }
        else if (fmt == 0 && (size_t)best + 262 <= length)
        {
            if (ch < 256) return d[6 + ch];
            return 0xFFFF;
        }
    }
    return 0xFFFF;
}

uint32_t usp_fb_GetGlyphIndicesW(HDC hdc, const WCHAR *chars, int count, WORD *out, uint32_t flags)
{
    int i;
    if (!g_active) return GetGlyphIndicesW(hdc, chars, count, out, flags);
    if (count <= 0) return 0;
    for (i = 0; i < count; i++)
    {
        uint32_t g = cmap_lookup((uint32_t)chars[i]);
        if (g == 0xFFFF && !(flags & 0x0040 /*GGI_MARK_NONEXISTING_GLYPHS*/)) g = 0;
        out[i] = (WORD)g;
    }
    return (uint32_t)count;
}

uint32_t usp_fb_GetTextCharsetInfo(HDC hdc, void *lpCs, uint32_t dwFlags)
{
    if (!g_active) return GetTextCharsetInfo(hdc, (LPFONTSIGNATURE)lpCs, dwFlags);
    return DEFAULT_CHARSET;
}

/* upem from the 'head' table (0 if not sfnt) */
uint32_t usp_fb_upem(void)
{
    uint32_t off = 0, length = 0;
    if (!g_active) return 0;
    if (!find_table(0x68656164 /*head*/, &off, &length)) return 0;
    if (length < 20) return 0;
    return be16(g_data + off + 18);
}

/* hmtx advance width for a glyph (0 if absent) */
uint32_t usp_fb_advance(uint32_t glyph)
{
    uint32_t off = 0, length = 0;
    uint32_t hhea = 0, hl = 0, maxp = 0, ml = 0, nh = 0, ng = 0;
    if (!g_active) return 0;
    if (find_table(0x68686561 /*hhea*/, &hhea, &hl) && hl >= 36)
        nh = be16(g_data + hhea + 34);
    if (find_table(0x6D617870 /*maxp*/, &maxp, &ml) && ml >= 6)
        ng = be16(g_data + maxp + 4);
    if (!nh) nh = ng;
    if (!find_table(0x686D7478 /*hmtx*/, &off, &length)) return 0;
    if (glyph >= ng) return 0;
    if (glyph < nh)
    {
        if ((size_t)off + (size_t)glyph * 4 + 2 <= g_len) return be16(g_data + off + glyph * 4);
    }
    else
    {
        if ((size_t)off + (size_t)(nh - 1) * 4 + 2 <= g_len) return be16(g_data + off + (nh - 1) * 4);
    }
    return 0;
}

/* ---- metrics + advance dispatch (bytes mode) ---- */

int usp_fb_GetTextMetricsW(HDC hdc, LPTEXTMETRICW tm)
{
    if (!g_active) return (GetTextMetricsW)(hdc, tm);
    memset(tm, 0, sizeof(TEXTMETRICW));
    tm->tmHeight = (LONG)usp_fb_upem();
    tm->tmAscent = (LONG)usp_fb_upem();
    tm->tmPitchAndFamily = TMPF_TRUETYPE;
    tm->tmCharSet = DEFAULT_CHARSET;
    return 1;
}

UINT usp_fb_GetOutlineTextMetricsW(HDC hdc, UINT cb, LPOUTLINETEXTMETRICW otm)
{
    if (!g_active) return (GetOutlineTextMetricsW)(hdc, cb, otm);
    return 0; /* no outline metrics in bytes mode; sc->otm stays NULL (unused) */
}

static BOOL abc_for_glyph(uint32_t glyph, LPABC abc)
{
    LONG w = (LONG)usp_fb_advance(glyph);
    abc->abcA = 0;
    abc->abcB = w;
    abc->abcC = 0;
    return TRUE;
}

BOOL usp_fb_GetCharABCWidthsI(HDC hdc, UINT first, UINT count, const WORD *pgi, LPABC abc)
{
    UINT i;
    if (!g_active) return (GetCharABCWidthsI)(hdc, first, count, (LPWORD)pgi, abc);
    for (i = 0; i < count; i++)
    {
        uint32_t g = pgi ? pgi[i] : (first + i);
        abc_for_glyph(g, &abc[i]);
    }
    return TRUE;
}

BOOL usp_fb_GetCharABCWidthsW(HDC hdc, UINT first, UINT last, LPABC abc)
{
    UINT i;
    if (!g_active) return (GetCharABCWidthsW)(hdc, first, last, abc);
    for (i = first; i <= last && i - first < 65536; i++)
    {
        uint32_t g = cmap_lookup(i);
        if (g == 0xFFFF) g = 0;
        abc_for_glyph(g, &abc[i - first]);
    }
    return TRUE;
}

BOOL usp_fb_GetCharWidthI(HDC hdc, UINT first, UINT count, const WORD *pgi, LPINT out)
{
    UINT i;
    if (!g_active) return (GetCharWidthI)(hdc, first, count, (LPWORD)pgi, out);
    for (i = 0; i < count; i++)
    {
        uint32_t g = pgi ? pgi[i] : (first + i);
        out[i] = (INT)usp_fb_advance(g);
    }
    return TRUE;
}

BOOL usp_fb_GetCharWidth32W(HDC hdc, UINT first, UINT last, LPINT out)
{
    UINT i;
    if (!g_active) return (GetCharWidth32W)(hdc, first, last, out);
    for (i = first; i <= last && i - first < 65536; i++)
    {
        uint32_t g = cmap_lookup(i);
        if (g == 0xFFFF) g = 0;
        out[i - first] = (INT)usp_fb_advance(g);
    }
    return TRUE;
}
