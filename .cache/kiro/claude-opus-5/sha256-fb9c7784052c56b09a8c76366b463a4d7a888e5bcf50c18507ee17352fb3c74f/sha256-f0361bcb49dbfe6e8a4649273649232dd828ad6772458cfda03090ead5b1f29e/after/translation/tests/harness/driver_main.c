/* Real C consumer of the library, compiled by gcc against the unmodified
 * public header (c_src/include/driver.h) and linked against EITHER .so.
 *
 * Purpose: the Rust export takes its argument as `int` and truncates, because
 * that is what gcc's `char`-taking callee does with the argument register.
 * This harness proves the export is still ABI-compatible with a caller that
 * uses the *declared* `void driver(char)` prototype and lets a real gcc call
 * site (not libloading) drive both libraries.
 *
 * Usage: driver_main <int-value>   -- value is converted to char by the C
 *        compiler, exactly as the header prescribes.
 */
#include <stdio.h>
#include <stdlib.h>

#include "driver.h"

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <value>\n", argv[0]);
        return 2;
    }
    long v = strtol(argv[1], NULL, 0);
    char c = (char) v;
    driver(c);
    return 0;
}
