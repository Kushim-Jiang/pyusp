/* port/posix/winbase.h — POSIX shim: base (kernel32-ish) macros/decls for the port. */
#ifndef PORT_POSIX_WINBASE_H
#define PORT_POSIX_WINBASE_H

#include "windef.h"
#include "winerror.h"

#include <string.h>

/* memory helpers (map to C runtime) */
#define ZeroMemory(Dest,Len)        memset((Dest), 0, (Len))
#define FillMemory(Dest,Len,Fill)   memset((Dest), (Fill), (Len))
#define CopyMemory(Dest,Src,Len)    memmove((Dest), (Src), (Len))
#define MoveMemory(Dest,Src,Len)    memmove((Dest), (Src), (Len))
#define SecureZeroMemory(Dest,Len)  memset((Dest), 0, (Len))

/* Win32 lstr* string helpers — provided by posix_compat (see posix_compat.c) */
int WINAPI lstrlenW(LPCWSTR lpString);
int WINAPI lstrlenA(LPCSTR lpString);
LPWSTR WINAPI lstrcpyW(LPWSTR dst, LPCWSTR src);
LPWSTR WINAPI lstrcpynW(LPWSTR dst, LPCWSTR src, int max);
LPSTR  WINAPI lstrcpyA(LPSTR dst, LPCSTR src);
LPSTR  WINAPI lstrcpynA(LPSTR dst, LPCSTR src, int max);
int WINAPI lstrcmpW(LPCWSTR a, LPCWSTR b);
int WINAPI lstrcmpiW(LPCWSTR a, LPCWSTR b);

/* 16-bit-aware wcschr for the port sources (provided by posix_compat.c).
 * On POSIX wchar_t is normally 32-bit; the port build uses -fshort-wchar and
 * this implementation so Wine's L"" punctuation scans treat 16-bit units. */
wchar_t *wcschr(const wchar_t *s, wchar_t c);

#define CopyMemoryFence()

#ifndef min
#define min(a,b) (((a) < (b)) ? (a) : (b))
#endif
#ifndef max
#define max(a,b) (((a) > (b)) ? (a) : (b))
#endif

/* Interlocked* — POSIX atomics via GCC __sync builtins.
 * Windows signature: LONG (32-bit) operands; on LP64 `long` would be 64-bit
 * and mismatch callers passing volatile LONG* (int*). */
static __inline LONG InterlockedIncrement(volatile LONG *v){ return __sync_add_and_fetch(v, 1); }
static __inline LONG InterlockedDecrement(volatile LONG *v){ return __sync_sub_and_fetch(v, 1); }
static __inline LONG InterlockedExchange(volatile LONG *t, LONG v){ return __sync_lock_test_and_set(t, v); }
static __inline LONG InterlockedExchangeAdd(volatile LONG *t, LONG v){ return __sync_fetch_and_add(t, v); }
static __inline void *InterlockedExchangePointer(void *volatile *t, void *v){ return __sync_lock_test_and_set(t, v); }
static __inline LONG InterlockedCompareExchange(volatile LONG *d, LONG e, LONG c){ return __sync_val_compare_and_swap(d, c, e); }
static __inline void *InterlockedCompareExchangePointer(void *volatile *d, void *e, void *c){ return __sync_val_compare_and_swap(d, c, e); }

/* CRITICAL_SECTION — non-recursive spinlock (adequate for the port's cache lock) */
typedef struct _RTL_CRITICAL_SECTION {
    volatile LONG lock;
} RTL_CRITICAL_SECTION;
typedef RTL_CRITICAL_SECTION CRITICAL_SECTION;
static __inline void InitializeCriticalSection(CRITICAL_SECTION *cs){ cs->lock = 0; }
static __inline void EnterCriticalSection(CRITICAL_SECTION *cs){ while (__sync_lock_test_and_set(&cs->lock, 1)) { } }
static __inline void LeaveCriticalSection(CRITICAL_SECTION *cs){ __sync_lock_release(&cs->lock); }
static __inline void DeleteCriticalSection(CRITICAL_SECTION *cs){ cs->lock = 0; }

#define IsBadReadPtr(p,cb) 0
#define IsBadWritePtr(p,cb) 0

#define GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS 0x4

#define INVALID_FILE_SIZE ((DWORD)0xFFFFFFFF)
#define INVALID_SET_FILE_POINTER ((DWORD)-1)

#endif /* PORT_POSIX_WINBASE_H */
