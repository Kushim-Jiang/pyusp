/* port/posix/windef.h — POSIX shim for the Wine Uniscribe port.
 *
 * Replaces MinGW's windef.h/winnt.h when the port is compiled natively for
 * Linux/macOS. Defines only the Windows base types the ported sources use.
 * On a Windows/MinGW build this file is NOT on the include path, so the real
 * Windows headers are used instead.
 */
#ifndef PORT_POSIX_WINDEF_H
#define PORT_POSIX_WINDEF_H

#include <stddef.h>
#include <stdint.h>

#ifndef _WINDEF_
#define _WINDEF_
#endif

/* ---- calling convention / decoration (no-op on POSIX) ---- */
#ifndef WINAPI
#define WINAPI
#endif
#ifndef APIENTRY
#define APIENTRY WINAPI
#endif
#ifndef CALLBACK
#define CALLBACK
#endif
#ifndef STDMETHODCALLTYPE
#define STDMETHODCALLTYPE
#endif
#ifndef STDAPICALLTYPE
#define STDAPICALLTYPE
#endif
#ifndef DECLSPEC_NORETURN
#define DECLSPEC_NORETURN
#endif
#ifndef __cdecl
#define __cdecl
#endif
#ifndef __stdcall
#define __stdcall
#endif
#ifndef __declspec
#define __declspec(x)
#endif
#ifndef __fastcall
#define __fastcall
#endif
#ifndef __forceinline
#define __forceinline inline
#endif
#ifndef __inline
#define __inline inline
#endif
#ifndef __unaligned
#define __unaligned
#endif
#ifndef __restrict
#define __restrict
#endif
#ifndef _inline
#define _inline inline
#endif
#ifndef __int8
#define __int8 int8_t
#endif
#ifndef __int16
#define __int16 int16_t
#endif
#ifndef __int32
#define __int32 int32_t
#endif
#ifndef __int64
#define __int64 int64_t
#endif
#ifndef __int3264
#define __int3264 int64_t
#endif

#define CDECL
#define PASCAL
#define WINAPIV
#define APIENTRYP WINAPI *
#define CALLBACKP CALLBACK *
#define STDMETHODCALLTYPE

#define NEAR
#define FAR
#define CONST const
#define VOID void

/* ---- base scalars ---- */
typedef char            CHAR;
typedef short           SHORT;
typedef long            LONG;
typedef int             INT;
typedef unsigned char   UCHAR;
typedef unsigned short  USHORT;
typedef unsigned long   ULONG;
typedef unsigned int    UINT;
typedef float           FLOAT;
typedef double          DOUBLE;
typedef uint8_t         BYTE;
typedef uint16_t        WORD;
typedef uint32_t        DWORD;
typedef uint64_t        DWORDLONG;
typedef int64_t         LONGLONG;
typedef uint64_t        ULONGLONG;
typedef unsigned char   BOOLEAN;
typedef int             BOOL;

#ifndef TRUE
#define TRUE 1
#endif
#ifndef FALSE
#define FALSE 0
#endif

#ifndef WCHAR
typedef uint16_t        WCHAR;
#endif
typedef uint16_t        WCHAR16;
typedef uint32_t        WCHAR32;
typedef char            CHAR8;

typedef BYTE            *PBYTE, *LPBYTE;
typedef CHAR            *PCHAR, *LPSTR, *LPCSTR;
typedef const CHAR      *PCSTR;
typedef WCHAR           *PWCHAR, *LPWSTR, *LPOLESTR;
typedef const WCHAR     *PCWSTR, *LPCWSTR, *LPCOLESTR;
typedef WORD            *PWORD, *LPWORD;
typedef DWORD           *PDWORD, *LPDWORD;
typedef LONG            *PLONG, *LPLONG;
typedef ULONG           *PULONG, *LPULONG;
typedef void            *PVOID, *LPVOID;
typedef const void      *LPCVOID;
typedef int             *LPINT, *PINT;
typedef UINT            *PUINT, *LPUINT;

typedef int64_t         INT_PTR, *PINT_PTR;
typedef uint64_t        UINT_PTR, *PUINT_PTR;
typedef int64_t         LONG_PTR, *PLONG_PTR;
typedef uint64_t        ULONG_PTR, *PULONG_PTR;
typedef uintptr_t       SIZE_T, *PSIZE_T;
typedef intptr_t        SSIZE_T, *PSSIZE_T;
typedef uintptr_t       DWORD_PTR, *PDWORD_PTR;


/* ---- handles ---- */
typedef void *HANDLE;
typedef HANDLE *PHANDLE;
typedef HANDLE HINSTANCE;
typedef HANDLE HMODULE;
typedef HANDLE HWND;
typedef HANDLE HDC;
typedef HANDLE HGDIOBJ;
typedef HANDLE HFONT;
typedef HANDLE HBRUSH;
typedef HANDLE HPEN;
typedef HANDLE HBITMAP;
typedef HANDLE HICON;
typedef HANDLE HCURSOR;
typedef HANDLE HMENU;
typedef HANDLE HRGN;
typedef HANDLE HPALETTE;
typedef HANDLE HKEY;
typedef HANDLE HWINEVENTHOOK;
typedef HANDLE HPOWERNOTIFY;

#define INVALID_HANDLE_VALUE ((HANDLE)(LONG_PTR)-1)
#define NULL_HANDLE ((HANDLE)0)

typedef void           *HANDLE_PTR;

/* ---- common structs ---- */
typedef struct { int unused; } SECURITY_ATTRIBUTES;

/* ---- HRESULT / SCODE / NTSTATUS / LRESULT ---- */
typedef LONG            HRESULT;
typedef LONG            SCODE;
typedef LONG            LRESULT;
typedef ULONG_PTR       WPARAM;
typedef LONG_PTR        LPARAM;
typedef SHORT           ATOM;

#define S_OK            ((HRESULT)0L)
#define S_FALSE         ((HRESULT)1L)
#define E_NOTIMPL       ((HRESULT)0x80004001L)
#define E_NOINTERFACE   ((HRESULT)0x80004002L)
#define E_POINTER       ((HRESULT)0x80004003L)
#define E_ABORT         ((HRESULT)0x80004004L)
#define E_FAIL          ((HRESULT)0x80004005L)
#define E_UNEXPECTED    ((HRESULT)0x8000FFFFL)
#define E_OUTOFMEMORY   ((HRESULT)0x8007000EL)
#define E_INVALIDARG    ((HRESULT)0x80070057L)
#define E_ACCESSDENIED  ((HRESULT)0x80070005L)
#define E_HANDLE        ((HRESULT)0x80070006L)
#define E_PENDING       ((HRESULT)0x8000000AL)

#define MAKE_HRESULT(sev,fac,code) ((HRESULT)(((ULONG)(sev)<<31) | ((ULONG)(fac)<<16) | (ULONG)(code)))
#define MAKE_SCODE(sev,fac,code)  ((SCODE)(((ULONG)(sev)<<31) | ((ULONG)(fac)<<16) | (ULONG)(code)))
#define SUCCEEDED(hr)   (((HRESULT)(hr)) >= 0)
#define FAILED(hr)      (((HRESULT)(hr)) < 0)

#define SEVERITY_SUCCESS 0
#define SEVERITY_ERROR 1
#define FACILITY_NULL 0
#define FACILITY_RPC 1
#define FACILITY_DISPATCH 2
#define FACILITY_STORAGE 3
#define FACILITY_ITF 4
#define FACILITY_WIN32 7
#define FACILITY_WINDOWS 8
#define FACILITY_SSPI 9
#define FACILITY_SECURITY 9
#define FACILITY_CONTROL 10
#define FACILITY_CERT 11
#define FACILITY_INTERNET 12
#define FACILITY_MEDIASERVER 13
#define FACILITY_MSMQ 14
#define FACILITY_SETUPAPI 15
#define FACILITY_SCARD 16
#define FACILITY_COMPLUS 17
#define FACILITY_AAF 18
#define FACILITY_URT 19
#define FACILITY_ACS 20
#define FACILITY_DPLAY 21
#define FACILITY_UMI 22
#define FACILITY_SXS 23
#define FACILITY_WINDOWS_CE 24
#define FACILITY_HTTP 25
#define FACILITY_USERMODE_COMMONLOG 26
#define FACILITY_WER 27
#define FACILITY_USERMODE_FILTER_MANAGER 31
#define FACILITY_BACKGROUNDCOPY 32
#define FACILITY_CONFIGURATION 33
#define FACILITY_WIA 33
#define FACILITY_STATE_MANAGEMENT 34
#define FACILITY_METADIRECTORY 35
#define FACILITY_WINDOWSUPDATE 36
#define FACILITY_DIRECTORYSERVICE 37
#define FACILITY_GRAPHICS 38
#define FACILITY_SHELL 39
#define FACILITY_NAP 39
#define FACILITY_TPM_SERVICES 40
#define FACILITY_TPM_SOFTWARE 41
#define FACILITY_UI 42
#define FACILITY_XAML 43
#define FACILITY_ACTION_QUEUE 44
#define FACILITY_PLA 48
#define FACILITY_WINDOWS_SETUP 48
#define FACILITY_FIREWALL 49
#define FACILITY_WINRM 51

#define HRESULT_CODE(hr)      ((hr) & 0xFFFF)
#define HRESULT_FACILITY(hr)  (((hr) >> 16) & 0x1FFF)
#define HRESULT_SEVERITY(hr)  (((hr) >> 31) & 1)

/* ---- common constants ---- */
#define MAX_PATH 260
#define MAX_LONG_PATH 32767
#define INFINITE 0xFFFFFFFFu
#define MAXULONG 0xffffffff
#define MAXLONG  0x7fffffff
#define MAXWORD  0xffff
#define MAXBYTE  0xff
#define MAXCHAR  0x7f
#define MINCHAR  0x80
#define MINSHORT 0x8000
#define MAXSHORT 0x7fff
#define MINLONG  0x80000000
#define MAXLONG_ 0x7fffffff

#define ANSI_NULL 0
#define UNICODE_NULL 0

/* ---- byte/word packing helpers (byte order independent: caller supplies) ---- */
#define MAKEWORD(a,b)      ((WORD)(((BYTE)((DWORD_PTR)(a))) | ((WORD)((BYTE)((DWORD_PTR)(b)))) << 8))
#define MAKELONG(a,b)      ((LONG)(((WORD)((DWORD_PTR)(a))) | ((DWORD)((WORD)((DWORD_PTR)(b)))) << 16))
#define LOWORD(l)          ((WORD)((DWORD_PTR)(l) & 0xffff))
#define HIWORD(l)          ((WORD)((DWORD_PTR)(l) >> 16))
#define LOBYTE(w)          ((BYTE)((DWORD_PTR)(w) & 0xff))
#define HIBYTE(w)          ((BYTE)((DWORD_PTR)(w) >> 8))
#define MAKELPARAM(l,h)    ((LPARAM)(DWORD)MAKELONG(l,h))
#define MAKELRESULT(l,h)   ((LRESULT)(DWORD)MAKELONG(l,h))

#define RGB(r,g,b) ((COLORREF)(((BYTE)(r)|((WORD)((BYTE)(g))<<8))|(((DWORD)(BYTE)(b))<<16)))
typedef DWORD COLORREF;

#define GET_X_LPARAM(lp)  ((int)(short)LOWORD(lp))
#define GET_Y_LPARAM(lp)  ((int)(short)HIWORD(lp))

/* ---- misc macros Wine code relies on ---- */
#ifndef FIELD_OFFSET
#define FIELD_OFFSET(type, field) ((LONG)offsetof(type, field))
#endif
#define FIELD_SIZE(type, field) (sizeof(((type *)0)->field))
#ifndef ARRAYSIZE
#define ARRAYSIZE(a) (sizeof(a)/sizeof((a)[0]))
#endif
#ifndef ARRAY_SIZE
#define ARRAY_SIZE(a) (sizeof(a)/sizeof((a)[0]))
#endif

#define OPTIONAL
#define _Success_(x)
#define _Ret_maybenull_
#define _Check_return_
#define __out_ecount(x)
#define __in_ecount(x)
#define __out
#define __in

/* ---- user/gdi base types that live here for convenience ---- */
typedef LONG_PTR  LRESULT_PTR;
typedef DWORD     TCHAR_T;

/* surrogate helpers */
#define IS_HIGH_SURROGATE(w)  (((w) >= 0xD800) && ((w) <= 0xDBFF))
#define IS_LOW_SURROGATE(w)   (((w) >= 0xDC00) && ((w) <= 0xDFFF))
#define IS_SURROGATE_PAIR(h,l) (IS_HIGH_SURROGATE(h) && IS_LOW_SURROGATE(l))
#define MAKE_SURROGATE_PAIR(h,l) (0x10000 + (((DWORD)(h) - 0xD800) << 10) + ((DWORD)(l) - 0xDC00))

#endif /* PORT_POSIX_WINDEF_H */
