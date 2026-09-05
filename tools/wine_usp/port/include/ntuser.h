/* port/ntuser.h — shim replacing Wine's include/ntuser.h for the standalone
 * Uniscribe port. usp10.c only uses NtUserGetSysColor from USER32, which maps
 * to the native GetSysColor export. */
#ifndef PORT_NTUSER_H
#define PORT_NTUSER_H

#include <windows.h>

#define NtUserGetSysColor GetSysColor

#endif /* PORT_NTUSER_H */
