/* Test-only helper: interposes libc `time()` so both the C and the Rust
 * library observe a caller-chosen epoch. This is NOT part of the translation;
 * it exists so the differential tests can reach timestamp-dependent code paths
 * (the >> 29 shift and the `% 100` modifier) at epochs where those operations
 * actually produce distinguishable values.
 *
 * Compiled at test time by tests/level6_faketime.rs; nothing under c_src/ is
 * involved.
 */
#include <stdlib.h>
#include <time.h>

static time_t fake_now(void) {
    const char *e = getenv("MATHOP_FAKE_TIME");
    if (!e) {
        return 0;
    }
    return (time_t)strtoll(e, NULL, 10);
}

time_t time(time_t *tloc) {
    time_t v = fake_now();
    if (tloc) {
        *tloc = v;
    }
    return v;
}
