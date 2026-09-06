/* port/posix/winreg.h — POSIX shim: registry surface used by the port. */
#ifndef PORT_POSIX_WINREG_H
#define PORT_POSIX_WINREG_H

#include "windef.h"
#include "winbase.h"
#include "winerror.h"

#define HKEY_CLASSES_ROOT  ((HKEY)(LONG_PTR)(int)0x80000000)
#define HKEY_CURRENT_USER  ((HKEY)(LONG_PTR)(int)0x80000001)
#define HKEY_LOCAL_MACHINE ((HKEY)(LONG_PTR)(int)0x80000002)
#define HKEY_USERS         ((HKEY)(LONG_PTR)(int)0x80000003)
#define HKEY_PERFORMANCE_DATA ((HKEY)(LONG_PTR)(int)0x80000004)
#define HKEY_CURRENT_CONFIG   ((HKEY)(LONG_PTR)(int)0x80000005)
#define HKEY_DYN_DATA         ((HKEY)(LONG_PTR)(int)0x80000006)

#define KEY_QUERY_VALUE 0x0001
#define KEY_SET_VALUE 0x0002
#define KEY_READ 0x20019
#define KEY_WRITE 0x20006
#define KEY_ALL_ACCESS 0xF003F

#define REG_NONE 0
#define REG_SZ 1
#define REG_EXPAND_SZ 2
#define REG_BINARY 3
#define REG_DWORD 4
#define REG_DWORD_LITTLE_ENDIAN 4
#define REG_MULTI_SZ 7

LONG WINAPI RegQueryValueExW(HKEY key, LPCWSTR name, DWORD *reserved, DWORD *type, void *data, DWORD *size);
LONG WINAPI RegOpenKeyExW(HKEY key, LPCWSTR sub, DWORD options, DWORD access, HKEY *out);
LONG WINAPI RegOpenKeyA(HKEY key, LPCSTR sub, HKEY *out);
LONG WINAPI RegCloseKey(HKEY key);

#endif /* PORT_POSIX_WINREG_H */
