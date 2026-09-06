/* port/posix_compat.c — POSIX implementations of the Win32 surface the
 * Wine Uniscribe port references.
 *
 * When the port is compiled natively for Linux/macOS (libwineusp.so/.dylib)
 * there is no gdi32/kernel32/user32. The shaping runs entirely in "bytes
 * mode" (usp_set_font_bytes + NULL hdc), so every GDI/DC/font function below
 * only needs to link — the real byte-backed implementations live in
 * fontbytes.c and are reached through the ntgdi.h/fontbytes.h dispatch
 * macros. Functions the shaping path genuinely calls (GetStringTypeW, the
 * lstr* helpers) are implemented functionally.
 *
 * WCHAR is 16-bit but POSIX wchar_t is 32-bit, so this file is compiled with
 * -fshort-wchar (making wchar_t 16-bit to match Wine's L"" literals) and MUST
 * NOT call the platform <wchar.h> wide functions (their ABI is 32-bit). All
 * string handling here is therefore done with explicit WCHAR loops, and the
 * only libc-wide function the port sources use (wcschr) is re-implemented
 * here.
 */
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "windef.h"
#include "winbase.h"
#include "wingdi.h"
#include "winuser.h"
#include "winnls.h"
#include "winreg.h"
#include "winerror.h"

/* 16-bit-aware wcschr (used by Wine's get_char_script / punctuation logic).
 * Declared in posix/winbase.h for the port TUs. */
wchar_t *wcschr(const wchar_t *s, wchar_t c)
{
    if (!s) return 0;
    while (*s) { if (*s == c) return (wchar_t *)s; s++; }
    return (c == 0) ? (wchar_t *)s : 0;
}

/* ------------------------------------------------------------------ */
/* kernel32-ish string helpers (functional)                            */
/* ------------------------------------------------------------------ */
int WINAPI lstrlenW(LPCWSTR s) { const WCHAR *p = s; if (!p) return 0; while (*p) p++; return (int)(p - s); }
int WINAPI lstrlenA(LPCSTR s) { return (int)strlen(s); }
LPWSTR WINAPI lstrcpyW(LPWSTR d, LPCWSTR s) { WCHAR *q = d; while ((*q++ = *s++)) { } return d; }
LPSTR WINAPI lstrcpyA(LPSTR d, LPCSTR s) { strcpy(d, s); return d; }
LPWSTR WINAPI lstrcpynW(LPWSTR d, LPCWSTR s, int n)
{ if (n <= 0) return d; { int i = 0; while (i < n - 1 && s[i]) { d[i] = s[i]; i++; } d[i] = 0; } return d; }
LPSTR WINAPI lstrcpynA(LPSTR d, LPCSTR s, int n)
{ if (n <= 0) return d; strncpy(d, s, (size_t)n - 1); d[n - 1] = 0; return d; }
int WINAPI lstrcmpW(LPCWSTR a, LPCWSTR b)
{ while (*a && *a == *b) { a++; b++; } return (*a < *b) ? -1 : (*a > *b ? 1 : 0); }
int WINAPI lstrcmpiW(LPCWSTR a, LPCWSTR b)
{
    for (;;) {
        WCHAR ca = *a, cb = *b;
        if (ca >= L'a' && ca <= L'z') ca = (WCHAR)(ca - (L'a' - L'A'));
        if (cb >= L'a' && cb <= L'z') cb = (WCHAR)(cb - (L'a' - L'A'));
        if (ca < cb) return -1;
        if (ca > cb) return 1;
        if (!ca) return 0;
        a++; b++;
    }
}

/* ------------------------------------------------------------------ */
/* GDI stubs — never reached in bytes mode; link-only.                 */
/* ------------------------------------------------------------------ */
int  WINAPI GetObjectW(HANDLE h, int c, void *pv) { (void)h; (void)c; (void)pv; return 0; }
HDC  WINAPI GetDC(HWND hwnd) { (void)hwnd; return 0; }
HDC  WINAPI CreateCompatibleDC(HDC hdc) { (void)hdc; return 0; }
HGDIOBJ WINAPI SelectObject(HDC hdc, HGDIOBJ h) { (void)hdc; (void)h; return 0; }
BOOL WINAPI DeleteDC(HDC hdc) { (void)hdc; return TRUE; }
BOOL WINAPI DeleteObject(HGDIOBJ h) { (void)h; return TRUE; }
HFONT WINAPI CreateFontIndirectW(const LOGFONTW *lf) { (void)lf; return 0; }
DWORD WINAPI GetFontData(HDC hdc, DWORD table, DWORD off, void *buf, DWORD len)
{ (void)hdc; (void)table; (void)off; (void)buf; (void)len; return (DWORD)GDI_ERROR; }
DWORD WINAPI GetGlyphIndicesW(HDC hdc, LPCWSTR s, int c, WORD *pgi, DWORD f)
{ (void)hdc; (void)s; (void)c; (void)pgi; (void)f; return (DWORD)GDI_ERROR; }
BOOL  WINAPI GetTextMetricsW(HDC hdc, LPTEXTMETRICW tm) { (void)hdc; (void)tm; return FALSE; }
UINT  WINAPI GetOutlineTextMetricsW(HDC hdc, UINT cb, LPOUTLINETEXTMETRICW otm)
{ (void)hdc; (void)cb; (void)otm; return 0; }
BOOL  WINAPI GetCharABCWidthsI(HDC hdc, UINT f, UINT n, LPWORD pgi, LPABC abc)
{ (void)hdc; (void)f; (void)n; (void)pgi; (void)abc; return FALSE; }
BOOL  WINAPI GetCharABCWidthsW(HDC hdc, UINT f, UINT l, LPABC abc)
{ (void)hdc; (void)f; (void)l; (void)abc; return FALSE; }
BOOL  WINAPI GetCharWidthI(HDC hdc, UINT f, UINT n, LPWORD pgi, LPINT out)
{ (void)hdc; (void)f; (void)n; (void)pgi; (void)out; return FALSE; }
BOOL  WINAPI GetCharWidth32W(HDC hdc, UINT f, UINT l, LPINT out)
{ (void)hdc; (void)f; (void)l; (void)out; return FALSE; }
UINT  WINAPI GetTextCharsetInfo(HDC hdc, LPFONTSIGNATURE sig, DWORD flags)
{ (void)hdc; (void)sig; (void)flags; return ANSI_CHARSET; }
HGDIOBJ WINAPI GetCurrentObject(HDC hdc, UINT type) { (void)hdc; (void)type; return 0; }
int  WINAPI GetDeviceCaps(HDC hdc, int index) { (void)hdc; (void)index; return 0; }
HGDIOBJ WINAPI GetStockObject(int i) { (void)i; return 0; }
BOOL WINAPI GetTextFaceW(HDC hdc, int c, LPWSTR name) { (void)hdc; (void)c; (void)name; return FALSE; }
BOOL WINAPI GetTextExtentPoint32W(HDC hdc, LPCWSTR s, int c, LPSIZE sz)
{ (void)hdc; (void)s; (void)c; (void)sz; return FALSE; }
int  WINAPI AddFontResourceExW(LPCWSTR f, DWORD fl, void *r) { (void)f; (void)fl; (void)r; return 0; }
BOOL WINAPI RemoveFontResourceExW(LPCWSTR f, DWORD fl, void *r) { (void)f; (void)fl; (void)r; return FALSE; }
int  WINAPI GetBkMode(HDC hdc) { (void)hdc; return TRANSPARENT; }
int  WINAPI SetBkMode(HDC hdc, int mode) { (void)hdc; (void)mode; return TRANSPARENT; }
COLORREF WINAPI GetBkColor(HDC hdc) { (void)hdc; return 0; }
COLORREF WINAPI SetBkColor(HDC hdc, COLORREF c) { (void)hdc; (void)c; return 0; }
COLORREF WINAPI GetTextColor(HDC hdc) { (void)hdc; return 0; }
COLORREF WINAPI SetTextColor(HDC hdc, COLORREF c) { (void)hdc; (void)c; return 0; }
BOOL  WINAPI ExtTextOutW(HDC hdc, int x, int y, UINT o, const RECT *r, LPCWSTR s, UINT c, const INT *dx)
{ (void)hdc; (void)x; (void)y; (void)o; (void)r; (void)s; (void)c; (void)dx; return TRUE; }
BOOL  WINAPI TextOutW(HDC hdc, int x, int y, LPCWSTR s, int c)
{ (void)hdc; (void)x; (void)y; (void)s; (void)c; return TRUE; }
BOOL  WINAPI ExtTextOutA(HDC hdc, int x, int y, UINT o, const RECT *r, LPCSTR s, UINT c, const INT *dx)
{ (void)hdc; (void)x; (void)y; (void)o; (void)r; (void)s; (void)c; (void)dx; return TRUE; }

/* ------------------------------------------------------------------ */
/* USER32 stubs                                                        */
/* ------------------------------------------------------------------ */
DWORD WINAPI GetSysColor(int nIndex) { (void)nIndex; return 0; }
int WINAPI GetSystemMetrics(int nIndex) { (void)nIndex; return 0; }
HMODULE WINAPI GetModuleHandleW(LPCWSTR name) { (void)name; return 0; }

/* Minimal wsprintfW: %s %c %d %i %u %x %X %o %lx/%lu (no width/floats). */
static int emit_ascii(WCHAR *d, const char *tmp)
{
    int n = 0;
    while (tmp[n]) { d[n] = (WCHAR)(unsigned char)tmp[n]; n++; }
    return n;
}
int WINAPI wsprintfW(LPWSTR out, LPCWSTR fmt, ...)
{
    va_list ap;
    WCHAR *d = out;
    const WCHAR *p = fmt;
    char tmp[64];
    va_start(ap, fmt);
    while (*p)
    {
        if (*p != L'%') { *d++ = *p++; continue; }
        p++;
        if (*p == L'%') { *d++ = L'%'; p++; continue; }
        if (*p == L'c') { *d++ = (WCHAR)va_arg(ap, int); p++; continue; }
        if (*p == L's') { const WCHAR *s = va_arg(ap, const WCHAR *); if (!s) s = L""; while (*s) *d++ = *s++; p++; continue; }
        if (*p == L'S') { const char *s = va_arg(ap, const char *); if (!s) s = ""; while (*s) *d++ = (WCHAR)(unsigned char)*s++; p++; continue; }
        if (*p == L'd' || *p == L'i') { long v = va_arg(ap, int); snprintf(tmp, sizeof(tmp), "%ld", v); d += emit_ascii(d, tmp); p++; continue; }
        if (*p == L'u') { unsigned long v = va_arg(ap, unsigned int); snprintf(tmp, sizeof(tmp), "%lu", v); d += emit_ascii(d, tmp); p++; continue; }
        if (*p == L'l' && (p[1] == L'd' || p[1] == L'u' || p[1] == L'x' || p[1] == L'X')) {
            unsigned long v = va_arg(ap, unsigned long);
            const char *f = (p[1]==L'd')?"%ld":(p[1]==L'u')?"%lu":(p[1]==L'X')?"%lX":"%lx";
            snprintf(tmp, sizeof(tmp), f, v); d += emit_ascii(d, tmp); p += 2; continue;
        }
        if (*p == L'x' || *p == L'X') { unsigned long v = va_arg(ap, unsigned int); snprintf(tmp, sizeof(tmp), (*p=='X')?"%X":"%x", v); d += emit_ascii(d, tmp); p++; continue; }
        if (*p == L'o') { unsigned long v = va_arg(ap, unsigned int); snprintf(tmp, sizeof(tmp), "%o", v); d += emit_ascii(d, tmp); p++; continue; }
        *d++ = L'%'; if (*p) *d++ = *p++;
    }
    *d = 0;
    va_end(ap);
    return (int)(d - out);
}

/* ------------------------------------------------------------------ */
/* NLS (functional where the shaping path uses them)                   */
/* ------------------------------------------------------------------ */
UINT WINAPI GetUserDefaultLCID(void) { return MAKELCID(MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT), 0); }
int  WINAPI GetUserDefaultLangID(void) { return (int)MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT); }
int  WINAPI GetSystemDefaultLangID(void) { return (int)MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT); }
int  WINAPI GetUserDefaultUILanguage(void) { return (int)MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT); }
int  WINAPI GetSystemDefaultUILanguage(void) { return (int)MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT); }
BOOL WINAPI IsValidLocale(DWORD id, DWORD flags) { (void)id; (void)flags; return TRUE; }
LCID WINAPI ConvertDefaultLocale(LCID locale) { return locale ? locale : MAKELCID(MAKELANGID(LANG_ENGLISH, SUBLANG_DEFAULT), 0); }
int WINAPI GetACP(void) { return CP_UTF8; }
int WINAPI GetOEMCP(void) { return CP_ACP; }

int WINAPI GetLocaleInfoW(LCID locale, LCTYPE lctype, LPWSTR data, int cch)
{
    (void)locale;
    if (lctype == (LOCALE_IDIGITSUBSTITUTION | LOCALE_RETURN_NUMBER) && data) {
        *(DWORD *)data = 1;
        return (int)sizeof(DWORD);
    }
    if (data && cch > 0) data[0] = 0;
    return 0;
}
int WINAPI GetLocaleInfoA(LCID locale, LCTYPE lctype, LPSTR data, int cch)
{
    (void)locale;
    if (lctype == (LOCALE_IDIGITSUBSTITUTION | LOCALE_RETURN_NUMBER) && data) {
        *(DWORD *)data = 1;
        return (int)sizeof(DWORD);
    }
    if (data && cch > 0) data[0] = 0;
    return 0;
}
int WINAPI CompareStringW(LCID locale, DWORD flags, LPCWSTR a, int na, LPCWSTR b, int nb)
{
    (void)locale; (void)flags;
    int n = na < nb ? na : nb, i;
    for (i = 0; i < n; i++) {
        if (a[i] < b[i]) return 1;   /* CSTR_LESS_THAN */
        if (a[i] > b[i]) return 3;   /* CSTR_GREATER_THAN */
    }
    if (na < nb) return 1;
    if (na > nb) return 3;
    return 2;                        /* CSTR_EQUAL */
}
BOOL WINAPI LCMapStringW(DWORD locale, DWORD flags, LPCWSTR src, int cchSrc, LPWSTR dst, int cchDest)
{
    (void)locale; (void)flags;
    int n = cchSrc < 0 ? lstrlenW(src) : cchSrc;
    if (!dst || cchDest <= n) return FALSE;
    { int i; for (i = 0; i < n; i++) dst[i] = src[i]; dst[n] = 0; }
    return TRUE;
}

/* GetStringTypeW — functional for the subset get_char_script() consults.
 * CTYPE1: control/digit/punct/alpha classes; CTYPE2: European vs Arabic
 * numbers plus basic directionality (used for C2_ARABICNUMBER). */
static int ctype2_class(WCHAR c)
{
    if (c >= 0x30 && c <= 0x39) return C2_EUROPENUMBER;
    if (c >= 0x0660 && c <= 0x0669) return C2_ARABICNUMBER;
    if (c >= 0x06F0 && c <= 0x06F9) return C2_ARABICNUMBER;
    if (c >= 0x0590 && c <= 0x08FF) return C2_RIGHTTOLEFT;
    if (c >= 0xFB1D && c <= 0xFDFF) return C2_RIGHTTOLEFT;
    if (c >= 0xFE70 && c <= 0xFEFC) return C2_RIGHTTOLEFT;
    if (c == 0x200F || c == 0x202B || c == 0x202E) return C2_RIGHTTOLEFT;
    if (c == 0x200E || c == 0x202A || c == 0x202D) return C2_LEFTTORIGHT;
    return C2_LEFTTORIGHT;
}

BOOL WINAPI GetStringTypeW(DWORD type, LPCWSTR src, int cchSrc, LPWORD chartype)
{
    int i;
    if (!src || cchSrc < 0 || !chartype) return FALSE;
    for (i = 0; i < cchSrc; i++) {
        WCHAR c = src[i];
        WORD t = 0;
        if (type == CT_CTYPE1) {
            if (c < 0x20 || c == 0x7f) t = C1_CNTRL;
            else if (c == 0x20) t = C1_SPACE | C1_BLANK;
            else if ((c >= '0' && c <= '9') || (c >= 0x0660 && c <= 0x0669) || (c >= 0x06F0 && c <= 0x06F9)) t = C1_DIGIT;
            else if (c >= 0x2000 && c <= 0x200B) t = C1_SPACE | C1_BLANK;
            else if (c < 0x80) {
                const char *p = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
                if (strchr(p, (char)c)) t = C1_PUNCT;
                else if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')) t = C1_ALPHA | C1_DEFINED;
                else t = C1_DEFINED;
            } else {
                if ((c >= 0x00C0 && c <= 0x024F) || (c >= 0x0370 && c <= 0x1FFF) ||
                    (c >= 0x2C00 && c <= 0xD7FF) || (c >= 0xF900 && c <= 0xFDCF) ||
                    (c >= 0xFDF0 && c <= 0xFFFD)) t = C1_ALPHA | C1_DEFINED;
                else t = C1_DEFINED;
            }
        } else if (type == CT_CTYPE2) {
            t = (WORD)ctype2_class(c);
        } else {
            t = C1_DEFINED;
        }
        chartype[i] = t;
    }
    return TRUE;
}
BOOL WINAPI GetStringTypeExW(LCID locale, DWORD type, LPCWSTR src, int cchSrc, LPWORD chartype)
{
    (void)locale;
    return GetStringTypeW(type, src, cchSrc, chartype);
}

/* ------------------------------------------------------------------ */
/* Registry stubs — the Wine fallback-font branch is not used in bytes */
/* mode; make every open fail so the fallback code path is skipped.    */
/* ------------------------------------------------------------------ */
LONG WINAPI RegOpenKeyExW(HKEY k, LPCWSTR s, DWORD o, DWORD a, HKEY *out)
{ (void)k; (void)s; (void)o; (void)a; (void)out; return ERROR_FILE_NOT_FOUND; }
LONG WINAPI RegOpenKeyA(HKEY k, LPCSTR s, HKEY *out)
{ (void)k; (void)s; (void)out; return ERROR_FILE_NOT_FOUND; }
LONG WINAPI RegQueryValueExW(HKEY k, LPCWSTR n, DWORD *r, DWORD *t, void *d, DWORD *s)
{ (void)k; (void)n; (void)r; (void)t; (void)d; (void)s; return ERROR_FILE_NOT_FOUND; }
LONG WINAPI RegCloseKey(HKEY k) { (void)k; return ERROR_SUCCESS; }
