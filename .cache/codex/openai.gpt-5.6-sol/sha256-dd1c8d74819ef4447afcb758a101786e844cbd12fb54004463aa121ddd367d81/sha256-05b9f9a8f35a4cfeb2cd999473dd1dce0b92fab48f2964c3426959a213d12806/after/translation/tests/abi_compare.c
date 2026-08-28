#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef const char **(*create_line_pointers_fn)(char *, size_t, size_t);

static create_line_pointers_fn load_function(void *library)
{
    void *symbol = dlsym(library, "UTIL_createLinePointers");
    create_line_pointers_fn function;
    memcpy(&function, &symbol, sizeof(function));
    return function;
}

static int compare_case(create_line_pointers_fn c_function,
                        create_line_pointers_fn rust_function,
                        char *buffer,
                        size_t num_lines,
                        size_t buffer_size)
{
    const char **c_result = c_function(buffer, num_lines, buffer_size);
    const char **rust_result = rust_function(buffer, num_lines, buffer_size);
    int mismatch = (c_result == NULL) != (rust_result == NULL);

    if (!mismatch && c_result != NULL) {
        mismatch = memcmp(c_result, rust_result,
                          num_lines * sizeof(*c_result)) != 0;
    }

    free(c_result);
    free(rust_result);
    return mismatch;
}

int main(int argc, char **argv)
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *rust_library = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_library == NULL || rust_library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    create_line_pointers_fn c_function = load_function(c_library);
    create_line_pointers_fn rust_function = load_function(rust_library);
    if (c_function == NULL || rust_function == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    uint64_t state = UINT64_C(0x4d595df4d0f33173);
    size_t cases = 0;
    for (size_t buffer_size = 0; buffer_size <= 128; ++buffer_size) {
        char buffer[128];
        for (size_t pattern = 0; pattern < 257; ++pattern) {
            for (size_t i = 0; i < buffer_size; ++i) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                buffer[i] = (pattern == 0) ? '\0' : (char)state;
            }

            for (size_t num_lines = 0; num_lines <= 130; ++num_lines) {
                ++cases;
                if (compare_case(c_function, rust_function, buffer,
                                 num_lines, buffer_size)) {
                    fprintf(stderr,
                            "mismatch: buffer_size=%zu pattern=%zu num_lines=%zu\n",
                            buffer_size, pattern, num_lines);
                    return 1;
                }
            }
        }
    }

    dlclose(rust_library);
    dlclose(c_library);
    printf("matched %zu cases\n", cases);
    return 0;
}
