/* port/posix/winuser.h — POSIX shim: USER32 surface used by the port. */
#ifndef PORT_POSIX_WINUSER_H
#define PORT_POSIX_WINUSER_H

#include "windef.h"
#include "winbase.h"
#include "wingdi.h"

/* sys color indices (subset) */
#define COLOR_SCROLLBAR 0
#define COLOR_BACKGROUND 1
#define COLOR_ACTIVECAPTION 2
#define COLOR_INACTIVECAPTION 3
#define COLOR_MENU 4
#define COLOR_WINDOW 5
#define COLOR_WINDOWFRAME 6
#define COLOR_MENUTEXT 7
#define COLOR_WINDOWTEXT 8
#define COLOR_CAPTIONTEXT 9
#define COLOR_ACTIVEBORDER 10
#define COLOR_INACTIVEBORDER 11
#define COLOR_APPWORKSPACE 12
#define COLOR_HIGHLIGHT 13
#define COLOR_HIGHLIGHTTEXT 14
#define COLOR_BTNFACE 15
#define COLOR_BTNSHADOW 16
#define COLOR_GRAYTEXT 17
#define COLOR_BTNTEXT 18
#define COLOR_INACTIVECAPTIONTEXT 19
#define COLOR_BTNHIGHLIGHT 20
#define COLOR_BTNHILIGHT COLOR_BTNHIGHLIGHT
#define COLOR_3DDKSHADOW 21
#define COLOR_3DLIGHT 22
#define COLOR_INFOTEXT 23
#define COLOR_INFOBK 24
#define COLOR_HOTLIGHT 26
#define COLOR_GRADIENTACTIVECAPTION 27
#define COLOR_GRADIENTINACTIVECAPTION 28
#define COLOR_MENUHILIGHT 29
#define COLOR_MENUBAR 30

/* message box / misc (not used but harmless) */
#define MB_OK 0x0

DWORD WINAPI GetSysColor(int nIndex);
int WINAPI GetSystemMetrics(int nIndex);
HMODULE WINAPI GetModuleHandleW(LPCWSTR name);
int WINAPI wsprintfW(LPWSTR out, LPCWSTR fmt, ...);

#endif /* PORT_POSIX_WINUSER_H */
