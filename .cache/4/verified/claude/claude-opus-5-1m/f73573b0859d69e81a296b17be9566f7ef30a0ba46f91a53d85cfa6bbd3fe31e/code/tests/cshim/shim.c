/*
 * Test-only shim for the differential test-suite.
 *
 * c_src/src/lib.c is included verbatim (c_src itself is never modified) so that
 * its `static` helpers -- the real low-level entry points of the library -- can
 * be reached across a shared-library boundary and compared against the Rust
 * `itest_*` exports produced by feature `internal_test_api`.
 *
 * The wrappers below add no logic of their own: they forward arguments
 * unchanged so that every guard, cast and loop under test is the original C.
 */

#include <stddef.h>

#include "../../c_src/src/lib.c"

int itest_memchra(const char *str, int c, size_t n) {
    return memchra(str, c, n);
}

int itest_process_buffer(char *buffer, size_t len) {
    return process_buffer(buffer, len);
}

float itest_int_to_float_bits(int value) {
    return int_to_float_bits(value);
}

int itest_process_strings(char **strings, int count, const char *target) {
    return process_strings(strings, count, target);
}

int itest_safe_sum_array(int *arr, size_t size) {
    return safe_sum_array(arr, size);
}

int itest_interpret_as_int(unsigned char *bytes, size_t len) {
    return interpret_as_int(bytes, len);
}

int itest_count_occurrences(const char *text, char ch) {
    return count_occurrences(text, ch);
}

int itest_complex_iteration(int *data, size_t count) {
    return complex_iteration(data, count);
}

/*
 * Exposes the `snprintf` call site of memchra2 (c_src/src/lib.c:132) verbatim so
 * that the Rust emulation of `"test%d-%d-%d-%d"` can be compared byte-for-byte
 * against glibc's `%d` conversion for arbitrary arguments.  Only the formatted
 * string plus its NUL is copied out; the remainder of `buffer` is
 * uninitialised in the original too and is never read by the library.
 */
void itest_format_buffer(int a, int b, int c, int d, char *out, size_t outlen) {
    char buffer[64];
    snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
    size_t n = strlen(buffer) + 1;
    if (n > outlen) {
        n = outlen;
    }
    memcpy(out, buffer, n);
}
