#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef uint16_t (*float2half_fn)(float);

static float2half_fn load_function(void *handle) {
    void *symbol = dlsym(handle, "float2half");
    float2half_fn function = NULL;

    if (symbol != NULL) {
        memcpy(&function, &symbol, sizeof(function));
    }
    return function;
}

static int compare(float2half_fn c_function, float2half_fn rust_function,
                   uint32_t bits, uint64_t *checked) {
    union {
        uint32_t bits;
        float value;
    } input = {.bits = bits};
    uint16_t c_result = c_function(input.value);
    uint16_t rust_result = rust_function(input.value);

    ++*checked;
    if (c_result != rust_result) {
        fprintf(stderr,
                "mismatch for 0x%08x: C=0x%04x Rust=0x%04x\n",
                bits, c_result, rust_result);
        return 1;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *rust_handle = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_handle == NULL || rust_handle == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    float2half_fn c_function = load_function(c_handle);
    float2half_fn rust_function = load_function(rust_handle);
    if (c_function == NULL || rust_function == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    uint64_t checked = 0;
    for (uint32_t index = 0; index < 512; ++index) {
        int exponent = (int)(index & 0xff) - 127;
        uint32_t shift;

        if (exponent < -24) {
            shift = 24;
        } else if (exponent < -14) {
            shift = (uint32_t)(-exponent - 1);
        } else if (exponent <= 15) {
            shift = 13;
        } else if (exponent < 128) {
            shift = 24;
        } else {
            shift = 13;
        }

        uint32_t prefix = index << 23;
        uint32_t low_mask = ((UINT32_C(1) << shift) - 1) & 0x007fffff;
        uint32_t max_quotient = 0x007fffff >> shift;

        for (uint32_t quotient = 0; quotient <= max_quotient; ++quotient) {
            uint32_t mantissa = quotient << shift;
            if (compare(c_function, rust_function, prefix | mantissa,
                        &checked) ||
                compare(c_function, rust_function,
                        prefix | mantissa | low_mask, &checked)) {
                return 1;
            }
        }

        for (uint32_t bit = 0; bit < 23; ++bit) {
            if (compare(c_function, rust_function,
                        prefix | (UINT32_C(1) << bit), &checked)) {
                return 1;
            }
        }
    }

    dlclose(rust_handle);
    dlclose(c_handle);
    printf("matched %llu representative bit patterns\n",
           (unsigned long long)checked);
    return 0;
}
