/*
 * Out-of-process driver harness (Phase B).
 *
 * The project itself builds no executable, so this tiny program plays the part
 * of one: it is linked against ONE shared object (chosen at link time) and
 * forwards each argv element to `driver()`. Running it once against the C `.so`
 * and once against the Rust `.so` and diffing stdout gives a byte-for-byte
 * comparison that is completely independent of the in-process libloading tests
 * (different process, real dynamic linker, real stdout).
 *
 * Not part of c_src/ and not used by the library; test scaffolding only.
 */
#include <stdio.h>

extern void driver(const char *in);

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        driver(argv[i]);
    }
    fflush(stdout);
    return 0;
}
