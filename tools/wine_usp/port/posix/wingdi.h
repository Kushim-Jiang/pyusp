/* port/posix/wingdi.h — POSIX shim: GDI types/constants/decls used by the port. */
#ifndef PORT_POSIX_WINGDI_H
#define PORT_POSIX_WINGDI_H

#include "windef.h"

#define LF_FACESIZE 32
#define LF_FULLFACESIZE 64

typedef char *PSTR, *PCHAR_;

/* PANOSE (subset of real struct) */
typedef struct tagPANOSE {
    BYTE bFamilyType;
    BYTE bSerifStyle;
    BYTE bWeight;
    BYTE bProportion;
    BYTE bContrast;
    BYTE bStrokeVariation;
    BYTE bArmStyle;
    BYTE bLetterForm;
    BYTE bMidline;
    BYTE bXHeight;
} PANOSE, *LPPANOSE;

typedef struct _POINT {
    LONG x;
    LONG y;
} POINT, *PPOINT, *LPPOINT;

typedef struct _RECT {
    LONG left;
    LONG top;
    LONG right;
    LONG bottom;
} RECT, *PRECT, *LPRECT;

typedef struct tagSIZE {
    LONG cx;
    LONG cy;
} SIZE, *PSIZE, *LPSIZE;

typedef struct tagTEXTMETRICW {
    LONG  tmHeight;
    LONG  tmAscent;
    LONG  tmDescent;
    LONG  tmInternalLeading;
    LONG  tmExternalLeading;
    LONG  tmAveCharWidth;
    LONG  tmMaxCharWidth;
    LONG  tmWeight;
    LONG  tmOverhang;
    LONG  tmDigitizedAspectX;
    LONG  tmDigitizedAspectY;
    WCHAR tmFirstChar;
    WCHAR tmLastChar;
    WCHAR tmDefaultChar;
    WCHAR tmBreakChar;
    BYTE  tmItalic;
    BYTE  tmUnderlined;
    BYTE  tmStruckOut;
    BYTE  tmPitchAndFamily;
    BYTE  tmCharSet;
} TEXTMETRICW, *PTEXTMETRICW, *LPTEXTMETRICW;

typedef struct tagTEXTMETRICA {
    LONG  tmHeight;
    LONG  tmAscent;
    LONG  tmDescent;
    LONG  tmInternalLeading;
    LONG  tmExternalLeading;
    LONG  tmAveCharWidth;
    LONG  tmMaxCharWidth;
    LONG  tmWeight;
    LONG  tmOverhang;
    LONG  tmDigitizedAspectX;
    LONG  tmDigitizedAspectY;
    BYTE  tmFirstChar;
    BYTE  tmLastChar;
    BYTE  tmDefaultChar;
    BYTE  tmBreakChar;
    BYTE  tmItalic;
    BYTE  tmUnderlined;
    BYTE  tmStruckOut;
    BYTE  tmPitchAndFamily;
    BYTE  tmCharSet;
} TEXTMETRICA, *PTEXTMETRICA, *LPTEXTMETRICA;

typedef struct tagNEWTEXTMETRICW {
    LONG  tmHeight;
    LONG  tmAscent;
    LONG  tmDescent;
    LONG  tmInternalLeading;
    LONG  tmExternalLeading;
    LONG  tmAveCharWidth;
    LONG  tmMaxCharWidth;
    LONG  tmWeight;
    LONG  tmOverhang;
    LONG  tmDigitizedAspectX;
    LONG  tmDigitizedAspectY;
    WCHAR tmFirstChar;
    WCHAR tmLastChar;
    WCHAR tmDefaultChar;
    WCHAR tmBreakChar;
    BYTE  tmItalic;
    BYTE  tmUnderlined;
    BYTE  tmStruckOut;
    BYTE  tmPitchAndFamily;
    BYTE  tmCharSet;
    DWORD ntmFlags;
    UINT  ntmSizeEM;
    UINT  ntmCellHeight;
    UINT  ntmAvgWidth;
} NEWTEXTMETRICW, *PNEWTEXTMETRICW, *LPNEWTEXTMETRICW;

typedef struct tagABC {
    INT abcA;
    UINT abcB;
    INT abcC;
} ABC, *PABC, *LPABC;

typedef struct tagABCFLOAT {
    FLOAT abcfA;
    FLOAT abcfB;
    FLOAT abcfC;
} ABCFLOAT, *PABCFLOAT, *LPABCFLOAT;

typedef struct tagLOGFONTW {
    LONG  lfHeight;
    LONG  lfWidth;
    LONG  lfEscapement;
    LONG  lfOrientation;
    LONG  lfWeight;
    BYTE  lfItalic;
    BYTE  lfUnderline;
    BYTE  lfStrikeOut;
    BYTE  lfCharSet;
    BYTE  lfOutPrecision;
    BYTE  lfClipPrecision;
    BYTE  lfQuality;
    BYTE  lfPitchAndFamily;
    WCHAR lfFaceName[LF_FACESIZE];
} LOGFONTW, *PLOGFONTW, *LPLOGFONTW;

typedef struct tagLOGFONTA {
    LONG  lfHeight;
    LONG  lfWidth;
    LONG  lfEscapement;
    LONG  lfOrientation;
    LONG  lfWeight;
    BYTE  lfItalic;
    BYTE  lfUnderline;
    BYTE  lfStrikeOut;
    BYTE  lfCharSet;
    BYTE  lfOutPrecision;
    BYTE  lfClipPrecision;
    BYTE  lfQuality;
    BYTE  lfPitchAndFamily;
    CHAR  lfFaceName[LF_FACESIZE];
} LOGFONTA, *PLOGFONTA, *LPLOGFONTA;

typedef struct tagOUTLINETEXTMETRICW {
    UINT    otmSize;
    TEXTMETRICW otmTextMetrics;
    BYTE    otmFiller;
    PANOSE  otmPanoseNumber;
    UINT    otmfsSelection;
    UINT    otmfsType;
    INT     otmsCharSlopeRise;
    INT     otmsCharSlopeRun;
    INT     otmEMSquare;
    INT     otmAscent;
    INT     otmDescent;
    INT     otmLineGap;
    UINT    otmsCapEmHeight;
    UINT    otmsXHeight;
    RECT    otmrcFontBox;
    INT     otmMacAscent;
    INT     otmMacDescent;
    INT     otmMacLineGap;
    UINT    otmusMinimumPPEM;
    POINT   otmptSubscriptSize;
    POINT   otmptSubscriptOffset;
    POINT   otmptSuperscriptSize;
    POINT   otmptSuperscriptOffset;
    UINT    otmsStrikeoutSize;
    INT     otmsStrikeoutPosition;
    INT     otmsUnderscoreSize;
    INT     otmsUnderscorePosition;
    PSTR    otmpFamilyName;
    PSTR    otmpFaceName;
    PSTR    otmpStyleName;
    PSTR    otmpFullName;
} OUTLINETEXTMETRICW, *POUTLINETEXTMETRICW, *LPOUTLINETEXTMETRICW;

typedef struct tagTEXTMETRICW TEXTMETRIC;

typedef DWORD FONTSIGNATURE[4];
typedef FONTSIGNATURE *PFONTSIGNATURE, *LPFONTSIGNATURE;

typedef struct tagCHARSETINFO {
    UINT ciCharset;
    UINT ciFlags;
    UINT ciSize;
    FONTSIGNATURE ciFS;
} CHARSETINFO, *PCHARSETINFO, *LPCHARSETINFO;

typedef struct tagFONTSIGNATURE {
    DWORD fsUsb[4];
    DWORD fsCsb[2];
} FONTSIGNATURE2, *PFONTSIGNATURE2;

/* object types */
#define OBJ_PEN 1
#define OBJ_BRUSH 2
#define OBJ_DC 3
#define OBJ_METADC 4
#define OBJ_PAL 5
#define OBJ_FONT 6
#define OBJ_BITMAP 7
#define OBJ_REGION 8
#define OBJ_METAFILE 9
#define OBJ_MEMDC 10
#define OBJ_EXTPEN 11
#define OBJ_ENHMETADC 12
#define OBJ_ENHMETAFILE 13
#define OBJ_COLORSPACE 14

/* font charsets */
#define ANSI_CHARSET 0
#define DEFAULT_CHARSET 1
#define SYMBOL_CHARSET 2
#define SHIFTJIS_CHARSET 128
#define HANGEUL_CHARSET 129
#define HANGUL_CHARSET 129
#define GB2312_CHARSET 134
#define CHINESEBIG5_CHARSET 136
#define OEM_CHARSET 255
#define JOHAB_CHARSET 130
#define HEBREW_CHARSET 177
#define ARABIC_CHARSET 178
#define GREEK_CHARSET 161
#define TURKISH_CHARSET 162
#define VIETNAMESE_CHARSET 163
#define THAI_CHARSET 222
#define EASTEUROPE_CHARSET 238
#define RUSSIAN_CHARSET 204
#define MAC_CHARSET 77
#define BALTIC_CHARSET 186

/* font pitch/family */
#define DEFAULT_PITCH 0
#define FIXED_PITCH 1
#define VARIABLE_PITCH 2
#define MONO_FONT 8
#define FF_DONTCARE (0<<4)
#define FF_ROMAN (1<<4)
#define FF_SWISS (2<<4)
#define FF_MODERN (3<<4)
#define FF_SCRIPT (4<<4)
#define FF_DECORATIVE (5<<4)

/* TMPF flags */
#define TMPF_FIXED_PITCH 0x01
#define TMPF_VECTOR 0x02
#define TMPF_DEVICE 0x08
#define TMPF_TRUETYPE 0x04

/* weight */
#define FW_DONTCARE 0
#define FW_THIN 100
#define FW_EXTRALIGHT 200
#define FW_ULTRALIGHT FW_EXTRALIGHT
#define FW_LIGHT 300
#define FW_NORMAL 400
#define FW_REGULAR 400
#define FW_MEDIUM 500
#define FW_SEMIBOLD 600
#define FW_DEMIBOLD FW_SEMIBOLD
#define FW_BOLD 700
#define FW_EXTRABOLD 800
#define FW_ULTRABOLD FW_EXTRABOLD
#define FW_HEAVY 900
#define FW_BLACK FW_HEAVY

/* quality */
#define DEFAULT_QUALITY 0
#define DRAFT_QUALITY 1
#define PROOF_QUALITY 2
#define NONANTIALIASED_QUALITY 3
#define ANTIALIASED_QUALITY 4
#define CLEARTYPE_QUALITY 5
#define CLEARTYPE_NATURAL_QUALITY 6

/* out/clip precision */
#define OUT_DEFAULT_PRECIS 0
#define OUT_STRING_PRECIS 1
#define OUT_CHARACTER_PRECIS 2
#define OUT_STROKE_PRECIS 3
#define OUT_TT_PRECIS 4
#define OUT_DEVICE_PRECIS 5
#define OUT_RASTER_PRECIS 6
#define OUT_TT_ONLY_PRECIS 7
#define OUT_OUTLINE_PRECIS 8
#define OUT_SCREEN_OUTLINE_PRECIS 9
#define OUT_PS_ONLY_PRECIS 10
#define CLIP_DEFAULT_PRECIS 0
#define CLIP_CHARACTER_PRECIS 1
#define CLIP_STROKE_PRECIS 2
#define CLIP_MASK 0xf
#define CLIP_LH_ANGLES (1<<4)
#define CLIP_TT_ALWAYS (2<<4)
#define CLIP_DFA_DISABLE (4<<4)
#define CLIP_EMBEDDED (8<<4)

#define GDI_ERROR 0xFFFFFFFFu

#define NTDDI_VERSION 0x06000000
#define WINVER 0x0600

/* FONTSIGNATURE helper masks (subset used by the port) */
#define FS_LATIN1  0x00000001
#define FS_LATIN2  0x00000002
#define FS_CYRILLIC 0x00000004
#define FS_GREEK   0x00000008
#define FS_TURKISH 0x00000010
#define FS_HEBREW  0x00000020
#define FS_ARABIC  0x00000040
#define FS_BALTIC  0x00000080
#define FS_VIETNAMESE 0x00000100
#define FS_THAI    0x00010000
#define FS_JISJAPAN 0x00020000
#define FS_CHINESESIMP 0x00040000
#define FS_WANSUNG 0x00080000
#define FS_CHINESETRAD 0x00100000
#define FS_JOHAB   0x00200000
#define FS_SYMBOL  0x80000000

/* GDI function decls — provided by posix_compat.c on POSIX. On the Windows
 * build these come from gdi32 and are not remapped where listed below. */
int  WINAPI GetObjectW(HANDLE h, int c, void *pv);
HDC  WINAPI GetDC(HWND hwnd);
HDC  WINAPI CreateCompatibleDC(HDC hdc);
HGDIOBJ WINAPI SelectObject(HDC hdc, HGDIOBJ h);
BOOL WINAPI DeleteDC(HDC hdc);
BOOL WINAPI DeleteObject(HGDIOBJ h);
HFONT WINAPI CreateFontIndirectW(const LOGFONTW *lf);
DWORD WINAPI GetFontData(HDC hdc, DWORD table, DWORD offset, void *buf, DWORD len);
DWORD WINAPI GetGlyphIndicesW(HDC hdc, LPCWSTR lpStr, int c, WORD *pgi, DWORD flags);
BOOL  WINAPI GetTextMetricsW(HDC hdc, LPTEXTMETRICW tm);
UINT  WINAPI GetOutlineTextMetricsW(HDC hdc, UINT cb, LPOUTLINETEXTMETRICW otm);
BOOL  WINAPI GetCharABCWidthsI(HDC hdc, UINT first, UINT count, LPWORD pgi, LPABC abc);
BOOL  WINAPI GetCharABCWidthsW(HDC hdc, UINT first, UINT last, LPABC abc);
BOOL  WINAPI GetCharWidthI(HDC hdc, UINT first, UINT count, LPWORD pgi, LPINT out);
BOOL  WINAPI GetCharWidth32W(HDC hdc, UINT first, UINT last, LPINT out);
UINT  WINAPI GetTextCharsetInfo(HDC hdc, LPFONTSIGNATURE sig, DWORD flags);
HGDIOBJ WINAPI GetCurrentObject(HDC hdc, UINT type);
int  WINAPI GetDeviceCaps(HDC hdc, int index);
HGDIOBJ WINAPI GetStockObject(int i);
BOOL WINAPI GetTextFaceW(HDC hdc, int c, LPWSTR name);
BOOL WINAPI GetTextExtentPoint32W(HDC hdc, LPCWSTR str, int c, LPSIZE size);
int  WINAPI AddFontResourceExW(LPCWSTR file, DWORD flags, void *reserved);
BOOL WINAPI RemoveFontResourceExW(LPCWSTR file, DWORD flags, void *reserved);

#define FR_PRIVATE 0x10
#define FR_NOT_ENUM 0x20

/* background/text mode & color */
#define OPAQUE_ 2
int  WINAPI GetBkMode(HDC hdc);
int  WINAPI SetBkMode(HDC hdc, int mode);
COLORREF WINAPI GetBkColor(HDC hdc);
COLORREF WINAPI SetBkColor(HDC hdc, COLORREF color);
COLORREF WINAPI GetTextColor(HDC hdc);
COLORREF WINAPI SetTextColor(HDC hdc, COLORREF color);
BOOL  WINAPI ExtTextOutW(HDC hdc, int x, int y, UINT options, const RECT *clip, LPCWSTR str, UINT c, const INT *dx);
BOOL  WINAPI TextOutW(HDC hdc, int x, int y, LPCWSTR str, int c);
BOOL  WINAPI ExtTextOutA(HDC hdc, int x, int y, UINT options, const RECT *clip, LPCSTR str, UINT c, const INT *dx);

#define RASTER_FONTTYPE 0x0001
#define DEVICE_FONTTYPE 0x0002
#define TRUETYPE_FONTTYPE 0x0004

#define GGI_MARK_NONEXISTING_GLYPHS 0x0001

/* ExtTextOut options */
#define ETO_OPAQUE 0x0002
#define ETO_CLIPPED 0x0004
#define ETO_GLYPH_INDEX 0x0010
#define ETO_RTLREADING 0x0800
#define ETO_NUMERICSLOCAL 0x0400
#define ETO_NUMERICSLATIN 0x0800
#define ETO_IGNORELANGUAGE 0x1000
#define ETO_PDY 0x2000
#define OPAQUE 0x0002
#define TRANSPARENT 0x0001

#define TA_LEFT 0x0000
#define TA_RIGHT 0x0002
#define TA_CENTER 0x0006
#define TA_TOP 0x0000
#define TA_BOTTOM 0x0008
#define TA_BASELINE 0x0018
#define TA_NOUPDATECP 0x0000
#define TA_UPDATECP 0x0001

#endif /* PORT_POSIX_WINGDI_H */
