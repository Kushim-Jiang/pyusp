/* port/posix/windows.h — POSIX shim aggregate (used by trace.c). */
#ifndef PORT_POSIX_WINDOWS_H
#define PORT_POSIX_WINDOWS_H

#include "windef.h"
#include "winbase.h"
#include "winerror.h"
#include "wingdi.h"
#include "winuser.h"
#include "winnls.h"
#include "winreg.h"

#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0600
#endif

#endif /* PORT_POSIX_WINDOWS_H */
