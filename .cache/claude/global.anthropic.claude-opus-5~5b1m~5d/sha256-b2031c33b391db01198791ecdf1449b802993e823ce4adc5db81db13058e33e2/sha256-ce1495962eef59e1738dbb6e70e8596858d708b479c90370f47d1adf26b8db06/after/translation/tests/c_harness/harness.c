/*
 * Test-only harness for the ground-truth C library.
 *
 * `c_src/src/lib.c` declares all of its helpers `static`, so they have no
 * dynamic symbols and cannot be reached with dlsym(). This file textually
 * includes the UNMODIFIED C source and adds thin external-linkage wrappers so
 * that the error-surface rows of ERRORS.md can be driven directly across the
 * FFI boundary and compared against the Rust translation's `harness_*` exports.
 *
 * NOTHING in c_src/ is modified: this file lives entirely under translation/.
 */

#include <stddef.h>

/* The ground truth, verbatim. */
#include "../../../c_src/src/lib.c"

int harness_memchra(const char *str, int c, size_t n) {
    return memchra(str, c, n);
}

int harness_process_buffer(char *buffer, size_t len) {
    return process_buffer(buffer, len);
}

float harness_int_to_float_bits(int value) {
    return int_to_float_bits(value);
}

int harness_process_strings(const char **strings, int count, const char *target) {
    return process_strings((char **)strings, count, target);
}

int harness_safe_sum_array(int *arr, size_t size) {
    return safe_sum_array(arr, size);
}

int harness_interpret_as_int(unsigned char *bytes, size_t len) {
    return interpret_as_int(bytes, len);
}

int harness_count_occurrences(const char *text, char ch) {
    return count_occurrences(text, ch);
}

int harness_complex_iteration(int *data, size_t count) {
    return complex_iteration(data, count);
}

int harness_snprintf_fmt(char *buffer, size_t size, int a, int b, int c, int d) {
    return snprintf(buffer, size, "test%d-%d-%d-%d", a, b, c, d);
}
