/*
 * TEST FIXTURE — not part of the library under test, and not part of c_src/.
 *
 * An LD_PRELOAD shim that makes libc's time() return a fixed value taken from
 * $FAKE_TIME. Both the C reference .so and the Rust .so import time@GLIBC, so
 * preloading this makes them observe the *same* clock and lets the tests drive
 * `get_computation_timestamp` (time() >> 29) and mathop's `% 100` time modifier
 * across values that wall-clock time cannot reach - including negative ones,
 * which exercise the arithmetic right shift.
 */
#include <stdlib.h>

long time(long *tloc) {
    const char *s = getenv("FAKE_TIME");
    long v = 0;
    if (s != NULL) {
        v = strtol(s, NULL, 10);
    }
    if (tloc != NULL) {
        *tloc = v;
    }
    return v;
}
