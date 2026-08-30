/* Interposition probe: a replacement definition of the library's low-level
 * entry point.  When LD_PRELOAD'ed, an *interposable* call from `driver` to
 * `printHexCharLine` must land here instead of in the library's own copy. */
#include <stdio.h>
void printHexCharLine(char c) { printf("SHIM(%d)\n", (int)c); }
