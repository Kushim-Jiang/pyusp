/* port/wine/debug.h — no-op stub for the standalone Uniscribe port.
 * The Wine core files reference WINE_* debug macros but never actually use
 * them (verified: 0 WINE_TRACE/WARN/FIXME/ERR call sites), so empty stubs are
 * sufficient. */
#ifndef __WINE_WINE_DEBUG_H
#define __WINE_WINE_DEBUG_H

#define WINE_DEFAULT_DEBUG_CHANNEL(x)
#define WINE_DECLARE_DEBUG_CHANNEL(x)
#define WINE_TRACE(...)
#define WINE_WARN(...)
#define WINE_FIXME(...)
#define WINE_ERR(...)
#define TRACE_ON(x) 0
#define WARN_ON(x)  0
#define FIXME_ON(x) 0
#define ERR_ON(x)   0
#define TRACE(...)
#define WARN(...)
#define FIXME(...)
#define ERR(...)
#define MESSAGE(...)
#define wine_dbgstr_an(x) (x)
#define wine_dbgstr_w(x) L""
#define wine_dbgstr_wn(x,n) L""
#define wine_dbgstr_guid(x) L""

#endif /* __WINE_WINE_DEBUG_H */
