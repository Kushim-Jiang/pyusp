/* port/src/fontbytes.h — font-bytes provider (cross-platform shaping).
 * Declares the byte-backed font operations and the dispatch entry points the
 * port's ntgdi.h maps NtGdi* calls to. Also the additive public extension:
 *   int usp_set_font_bytes(const void *data, size_t len)
 *   void usp_clear_font_bytes(void)
 * LGPL-2.1-or-later.
 */
#ifndef PORT_FONTBYTES_H
#define PORT_FONTBYTES_H

#include <stddef.h>
#include <stdint.h>
#include "windef.h"

/* Register raw font file bytes for byte-backed font access (0 ok, -1 invalid).
 * Until registered (or after usp_clear_font_bytes), every dispatch falls back
 * to the real gdi32 function, so the Windows GDI path is unchanged. */
int usp_set_font_bytes(const void *data, size_t len);
void usp_clear_font_bytes(void);
int usp_font_bytes_active(void);

/* Dispatch wrappers (mirror gdi32 signatures). */
uint32_t usp_fb_GetFontData(HDC hdc, uint32_t tag, uint32_t offset, void *buf, uint32_t len);
uint32_t usp_fb_GetGlyphIndicesW(HDC hdc, const WCHAR *chars, int count, WORD *out, uint32_t flags);
uint32_t usp_fb_GetTextCharsetInfo(HDC hdc, void *lpCs, uint32_t dwFlags);

/* Direct byte queries (convenience for the engine / tests). */
uint32_t usp_fb_upem(void);
uint32_t usp_fb_advance(uint32_t glyph);

/* Metrics + advance dispatch (bytes mode). */
int usp_fb_GetTextMetricsW(HDC hdc, LPTEXTMETRICW tm);
UINT usp_fb_GetOutlineTextMetricsW(HDC hdc, UINT cb, LPOUTLINETEXTMETRICW otm);
BOOL usp_fb_GetCharABCWidthsI(HDC hdc, UINT first, UINT count, const WORD *pgi, LPABC abc);
BOOL usp_fb_GetCharABCWidthsW(HDC hdc, UINT first, UINT last, LPABC abc);
BOOL usp_fb_GetCharWidthI(HDC hdc, UINT first, UINT count, const WORD *pgi, LPINT out);
BOOL usp_fb_GetCharWidth32W(HDC hdc, UINT first, UINT last, LPINT out);

/* In the implementation file (PORT_FONTBYTES_IMPL) keep the real gdi32 names so
 * fallback calls reach gdi32; everywhere else remap the font GDI calls to the
 * dispatchers so bytes mode is honoured without touching the Wine call sites. */
#ifndef PORT_FONTBYTES_IMPL
#define GetTextMetricsW usp_fb_GetTextMetricsW
#define GetOutlineTextMetricsW usp_fb_GetOutlineTextMetricsW
#define GetCharABCWidthsI usp_fb_GetCharABCWidthsI
#define GetCharABCWidthsW usp_fb_GetCharABCWidthsW
#define GetCharWidthI usp_fb_GetCharWidthI
#define GetCharWidth32W usp_fb_GetCharWidth32W
#endif

#endif /* PORT_FONTBYTES_H */
