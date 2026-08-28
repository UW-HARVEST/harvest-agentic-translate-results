/* Standalone differential driver: dlopen two objects exporting `pow43` and
 * compare them bit-for-bit over the whole well-defined domain (-16..=8223).
 *
 * Used to confirm that the C *reference* itself is stable across compilers and
 * optimisation levels, i.e. that "byte-identical to the C" is a well-defined
 * target and not an artefact of one particular build.
 *
 *   cc -O1 -o cmp_so cmp_so.c -ldl
 *   ./cmp_so <a.so> <b.so>
 */
#include <stdio.h>
#include <string.h>
#include <dlfcn.h>

typedef float (*pow43_fn)(int);

static unsigned bits(float f) {
    unsigned u;
    memcpy(&u, &f, sizeof u);
    return u;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <a.so> <b.so>\n", argv[0]);
        return 2;
    }
    void *ha = dlopen(argv[1], RTLD_NOW);
    if (!ha) { fprintf(stderr, "dlopen %s: %s\n", argv[1], dlerror()); return 2; }
    void *hb = dlopen(argv[2], RTLD_NOW);
    if (!hb) { fprintf(stderr, "dlopen %s: %s\n", argv[2], dlerror()); return 2; }

    pow43_fn a = (pow43_fn)dlsym(ha, "pow43");
    pow43_fn b = (pow43_fn)dlsym(hb, "pow43");
    if (!a || !b) { fprintf(stderr, "dlsym pow43 failed\n"); return 2; }

    long mismatches = 0, shown = 0;
    for (int x = -16; x <= 8223; x++) {
        unsigned ba = bits(a(x)), bb = bits(b(x));
        if (ba != bb) {
            if (shown++ < 8)
                printf("  diff x=%d a=%08x b=%08x\n", x, ba, bb);
            mismatches++;
        }
    }
    printf("mismatches over -16..8223: %ld\n", mismatches);
    return mismatches != 0;
}
