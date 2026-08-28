#include "../../../c_src/src/lib.c"

int verify_process_buffer(char *buffer, size_t len) {
    return process_buffer(buffer, len);
}

int verify_process_strings(char **strings, int count, const char *target) {
    return process_strings(strings, count, target);
}

int verify_safe_sum_array(int *arr, size_t size) {
    return safe_sum_array(arr, size);
}

int verify_interpret_as_int(unsigned char *bytes, size_t len) {
    return interpret_as_int(bytes, len);
}

int verify_count_occurrences(const char *text, char ch) {
    return count_occurrences(text, ch);
}

int verify_complex_iteration(int *data, size_t count) {
    return complex_iteration(data, count);
}
