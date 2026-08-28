#include "../../c_src/include/lib.h"

#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef cJSON_bool (*parse_number_fn)(cJSON *, parse_buffer *);

static size_t cases_run = 0;

static parse_number_fn load_function(void *handle)
{
    void *symbol = dlsym(handle, "parse_number");
    parse_number_fn function = NULL;

    if (symbol == NULL)
    {
        fprintf(stderr, "dlsym(parse_number): %s\n", dlerror());
        exit(2);
    }

    memcpy(&function, &symbol, sizeof(function));
    return function;
}

static void report_bytes(const char *label, const void *value, size_t size)
{
    const unsigned char *bytes = value;
    size_t i;

    fprintf(stderr, "%s=", label);
    for (i = 0; i < size; i++)
    {
        fprintf(stderr, "%02x", bytes[i]);
    }
    fputc('\n', stderr);
}

static void compare_case(
    parse_number_fn c_function,
    parse_number_fn rust_function,
    const unsigned char *content,
    size_t length,
    size_t offset)
{
    cJSON c_item;
    cJSON rust_item;
    parse_buffer c_buffer = {content, length, offset, (size_t)0x12345678};
    parse_buffer rust_buffer = c_buffer;
    cJSON_bool c_result;
    cJSON_bool rust_result;

    memset(&c_item, 0xa5, sizeof(c_item));
    rust_item = c_item;

    c_result = c_function(&c_item, &c_buffer);
    rust_result = rust_function(&rust_item, &rust_buffer);
    cases_run++;

    if ((c_result != rust_result)
        || (memcmp(&c_item, &rust_item, sizeof(c_item)) != 0)
        || (memcmp(&c_buffer, &rust_buffer, sizeof(c_buffer)) != 0))
    {
        fprintf(
            stderr,
            "mismatch at case %zu (length=%zu offset=%zu): C=%d Rust=%d\n",
            cases_run,
            length,
            offset,
            c_result,
            rust_result);
        report_bytes("input", content, length);
        report_bytes("c_item", &c_item, sizeof(c_item));
        report_bytes("rust_item", &rust_item, sizeof(rust_item));
        report_bytes("c_buffer", &c_buffer, sizeof(c_buffer));
        report_bytes("rust_buffer", &rust_buffer, sizeof(rust_buffer));
        exit(1);
    }
}

static void compare_null_cases(
    parse_number_fn c_function,
    parse_number_fn rust_function)
{
    cJSON c_item;
    cJSON rust_item;
    parse_buffer c_buffer = {NULL, 12, 3, 7};
    parse_buffer rust_buffer = c_buffer;
    cJSON_bool c_result;
    cJSON_bool rust_result;

    memset(&c_item, 0xa5, sizeof(c_item));
    rust_item = c_item;
    c_result = c_function(&c_item, NULL);
    rust_result = rust_function(&rust_item, NULL);
    cases_run++;
    if ((c_result != rust_result)
        || (memcmp(&c_item, &rust_item, sizeof(c_item)) != 0))
    {
        fprintf(stderr, "null-buffer mismatch\n");
        exit(1);
    }

    c_result = c_function(&c_item, &c_buffer);
    rust_result = rust_function(&rust_item, &rust_buffer);
    cases_run++;
    if ((c_result != rust_result)
        || (memcmp(&c_item, &rust_item, sizeof(c_item)) != 0)
        || (memcmp(&c_buffer, &rust_buffer, sizeof(c_buffer)) != 0))
    {
        fprintf(stderr, "null-content mismatch\n");
        exit(1);
    }
}

static void compare_known_cases(
    parse_number_fn c_function,
    parse_number_fn rust_function)
{
    static const char *const inputs[] = {
        "",
        "0",
        "-0",
        "+1",
        "1.25",
        "-.5",
        "1e3",
        "1E-3",
        "1e",
        "1e+",
        "1e-",
        "1+2",
        "01",
        ".",
        "+.",
        "--1",
        "e1",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "1e309",
        "-1e309",
        "1e-4000",
        "00.10E+02x",
        "12abc",
        " 1",
        "nan",
        "inf",
    };
    static const unsigned char embedded_nul[] = {'1', '2', '\0', '3', '4'};
    static const unsigned char offset_input[] = {'x', 'x', '-', '1', '2', '.', '5', 'z'};
    size_t i;

    for (i = 0; i < sizeof(inputs) / sizeof(inputs[0]); i++)
    {
        compare_case(
            c_function,
            rust_function,
            (const unsigned char *)inputs[i],
            strlen(inputs[i]),
            0);
    }

    compare_case(
        c_function,
        rust_function,
        embedded_nul,
        sizeof(embedded_nul),
        0);
    compare_case(
        c_function,
        rust_function,
        offset_input,
        sizeof(offset_input),
        2);
    compare_case(
        c_function,
        rust_function,
        offset_input,
        sizeof(offset_input) - 1,
        2);
    compare_case(
        c_function,
        rust_function,
        offset_input,
        sizeof(offset_input),
        sizeof(offset_input));
}

static void compare_exhaustive_cases(
    parse_number_fn c_function,
    parse_number_fn rust_function)
{
    static const unsigned char alphabet[] = {
        '0', '1', '9', '+', '-', '.', 'e', 'E', 'x', ' ', '\0', 0xff
    };
    unsigned char input[6];
    size_t length;
    size_t combinations = 1;

    for (length = 0; length <= sizeof(input); length++)
    {
        size_t code;

        if (length != 0)
        {
            combinations *= sizeof(alphabet);
        }

        for (code = 0; code < combinations; code++)
        {
            size_t remaining = code;
            size_t i;

            for (i = 0; i < length; i++)
            {
                input[i] = alphabet[remaining % sizeof(alphabet)];
                remaining /= sizeof(alphabet);
            }
            compare_case(c_function, rust_function, input, length, 0);
        }
    }
}

static uint64_t next_random(uint64_t *state)
{
    uint64_t value = *state;

    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    return value;
}

static void compare_random_cases(
    parse_number_fn c_function,
    parse_number_fn rust_function)
{
    static const unsigned char likely_bytes[] = {
        '0', '1', '2', '8', '9', '+', '-', '.', 'e', 'E', 'x'
    };
    unsigned char input[96];
    uint64_t state = UINT64_C(0x8c3c010cb4754c91);
    size_t iteration;

    for (iteration = 0; iteration < 100000; iteration++)
    {
        size_t length = (size_t)(next_random(&state) % (sizeof(input) + 1));
        size_t offset = (size_t)(next_random(&state) % (length + 1));
        size_t i;

        for (i = 0; i < length; i++)
        {
            uint64_t value = next_random(&state);

            input[i] = (value % 5 == 0)
                ? (unsigned char)value
                : likely_bytes[value % sizeof(likely_bytes)];
        }
        compare_case(c_function, rust_function, input, length, offset);
    }
}

int main(int argc, char **argv)
{
    void *c_handle;
    void *rust_handle;
    parse_number_fn c_function;
    parse_number_fn rust_function;

    if (argc != 3)
    {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    c_handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (c_handle == NULL)
    {
        fprintf(stderr, "dlopen(%s): %s\n", argv[1], dlerror());
        return 2;
    }
    rust_handle = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (rust_handle == NULL)
    {
        fprintf(stderr, "dlopen(%s): %s\n", argv[2], dlerror());
        return 2;
    }

    c_function = load_function(c_handle);
    rust_function = load_function(rust_handle);

    compare_null_cases(c_function, rust_function);
    compare_known_cases(c_function, rust_function);
    compare_exhaustive_cases(c_function, rust_function);
    compare_random_cases(c_function, rust_function);

    printf("differential cases passed: %zu\n", cases_run);

    dlclose(rust_handle);
    dlclose(c_handle);
    return 0;
}
