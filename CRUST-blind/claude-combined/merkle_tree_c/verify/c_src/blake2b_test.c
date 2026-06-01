#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "dev-deps/blake2b.h"

void print_hex(const uint8_t *bytes, size_t n) {
    for (size_t i = 0; i < n; i++) {
        printf("%02x", bytes[i]);
    }
    printf("\n");
}

int main() {
    /* Test 1: blake2b("", outlen=64) */
    uint8_t out64[64] = {0};
    blake2b(out64, 64, "", 0, NULL, 0);
    printf("empty 64: ");
    print_hex(out64, 64);

    /* Test 2: blake2b("abc", outlen=32) */
    uint8_t out32[32] = {0};
    blake2b(out32, 32, "abc", 3, NULL, 0);
    printf("abc 32: ");
    print_hex(out32, 32);

    /* Test 3: blake2b("hello world", outlen=64) */
    uint8_t out64b[64] = {0};
    blake2b(out64b, 64, "hello world", 11, NULL, 0);
    printf("hello world 64: ");
    print_hex(out64b, 64);

    /* Test 4: streaming */
    blake2b_state s;
    blake2b_init(&s, 32);
    blake2b_update(&s, (const uint8_t*)"hello ", 6);
    blake2b_update(&s, (const uint8_t*)"world", 5);
    uint8_t out_stream[32];
    blake2b_final(&s, out_stream, 32);
    printf("streaming: ");
    print_hex(out_stream, 32);

    /* Test 5: keyed */
    uint8_t out_key[32];
    uint8_t key[16] = "1234567890123456";
    blake2b(out_key, 32, "abc", 3, key, 16);
    printf("keyed abc: ");
    print_hex(out_key, 32);

    /* Test 6: long input - 200 bytes of 'A' */
    uint8_t long_input[200];
    memset(long_input, 'A', 200);
    uint8_t out_long[64];
    blake2b(out_long, 64, long_input, 200, NULL, 0);
    printf("long 200 As: ");
    print_hex(out_long, 64);

    return 0;
}
