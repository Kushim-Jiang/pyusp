/* port/ntgdi.h — shim replacing Wine's include/ntgdi.h for the standalone
 * Uniscribe port. Wine's internal NtGdi* calls are the same underlying GDI
 * functions exported by native Windows gdi32, so we map them directly. */
#ifndef PORT_NTGDI_H
#define PORT_NTGDI_H

#include <stddef.h>
#include <wingdi.h>

/* Wine defines ARRAY_SIZE in its own winnt.h; native Windows headers only
 * provide ARRAYSIZE. Provide the Wine form for the ported sources. */
#ifndef ARRAY_SIZE
#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))
#endif

/* Wine calls these as NtGdiGetFontData(hdc, tag, 0, buf, len) etc.  Native
 * gdi32 exports GetFontData / GetGlyphIndicesW / GetTextCharsetInfo with the
 * same signatures/semantics.
 *
 * When the engine registers raw font bytes (usp_set_font_bytes), these
 * dispatch to the byte-backed provider in fontbytes.c so the same shaping can
 * run without a GDI device context (cross-platform). With no bytes registered
 * they fall back to the real gdi32 functions (Windows GDI path, unchanged). */
#include "fontbytes.h"
#define NtGdiGetFontData usp_fb_GetFontData
#define NtGdiGetGlyphIndicesW usp_fb_GetGlyphIndicesW
#define NtGdiGetTextCharsetInfo usp_fb_GetTextCharsetInfo

#endif /* PORT_NTGDI_H */
