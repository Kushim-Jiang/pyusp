/* port/winternl.h — shim replacing Wine's include/winternl.h for the
 * standalone Uniscribe port. Only byte-swap helpers are needed. */
#ifndef PORT_WINTERNL_H
#define PORT_WINTERNL_H

#include <stdint.h>

static inline unsigned short RtlUshortByteSwap( unsigned short x )
{
    return (unsigned short)((x >> 8) | (x << 8));
}

static inline unsigned int RtlUlongByteSwap( unsigned int x )
{
    return ((x & 0xff000000u) >> 24) | ((x & 0x00ff0000u) >> 8)
         | ((x & 0x0000ff00u) << 8)  | ((x & 0x000000ffu) << 24);
}

#endif /* PORT_WINTERNL_H */
