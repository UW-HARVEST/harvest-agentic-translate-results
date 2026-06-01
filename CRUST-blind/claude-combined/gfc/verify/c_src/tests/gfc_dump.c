#include <gfc/gfc.h>
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>

void dump_range(uint64_t range, uint64_t rounds, uint64_t seed) {
    GFC *gfc = gfc_init(range, rounds, seed);
    printf("RANGE %" PRIu64 " ROUNDS %" PRIu64 " SEED %" PRIu64 "\n", range, rounds, seed);
    for (uint64_t i = 0; i < range; i++) {
        uint64_t enc = gfc_encrypt(gfc, i);
        uint64_t dec = gfc_decrypt(gfc, enc);
        printf("  enc[%" PRIu64 "] = %" PRIu64 ", dec[%" PRIu64 "] = %" PRIu64 "\n", i, enc, enc, dec);
    }
    gfc_destroy(gfc);
}

int main() {
    dump_range(1, 1, 42);
    dump_range(10, 1, 42);
    dump_range(10, 6, 42);
    dump_range(16, 4, 7);
    dump_range(100, 6, 12345);
    return 0;
}
