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
    /* Exactly 128 bytes input */
    uint8_t input128[128];
    for (int i = 0; i < 128; i++) input128[i] = (uint8_t)i;
    uint8_t out[64];
    blake2b(out, 64, input128, 128, NULL, 0);
    printf("128 bytes 0..127: ");
    print_hex(out, 64);

    /* 129 bytes */
    uint8_t input129[129];
    for (int i = 0; i < 129; i++) input129[i] = (uint8_t)i;
    blake2b(out, 64, input129, 129, NULL, 0);
    printf("129 bytes: ");
    print_hex(out, 64);

    /* outlen=1 */
    uint8_t out1[1];
    blake2b(out1, 1, "abc", 3, NULL, 0);
    printf("abc outlen=1: ");
    print_hex(out1, 1);

    /* outlen=64 with abc */
    uint8_t out64[64];
    blake2b(out64, 64, "abc", 3, NULL, 0);
    printf("abc outlen=64: ");
    print_hex(out64, 64);

    return 0;
}
