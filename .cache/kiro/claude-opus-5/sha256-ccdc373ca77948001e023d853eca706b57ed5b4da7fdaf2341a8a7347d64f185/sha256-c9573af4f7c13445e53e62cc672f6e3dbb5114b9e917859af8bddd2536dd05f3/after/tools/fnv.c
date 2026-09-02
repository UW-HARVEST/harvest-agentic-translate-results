/* FNV-1a over a file's bytes, printed as 16 hex digits.
 * Identical to the `hash` op in runner.c and to harness::fnv1a in the tests.
 * usage: fnv <file>
 */
#include <stdint.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    FILE *f = fopen(argv[1], "rb");
    if (!f) { perror("fopen"); return 3; }
    uint64_t h = 1469598103934665603ULL;
    int c;
    while ((c = fgetc(f)) != EOF) {
        h ^= (uint8_t)c;
        h *= 1099511628211ULL;
    }
    fclose(f);
    printf("%016llx\n", (unsigned long long)h);
    return 0;
}
