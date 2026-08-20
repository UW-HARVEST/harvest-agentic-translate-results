/* Reference harness for the ONE library call whose result is otherwise
 * unobservable through the program's output.
 *
 * c_src/src/main.c lines 73-74 are:
 *     int x = 0;
 *     scanf("%d", &x);
 * Both branches of the following `if (x)` print the same bytes ("0\n"), so the
 * value of `x` -- and therefore the fidelity of the scanf emulation in the Rust
 * translation -- cannot be observed from the program's stdout. This probe
 * performs the identical two statements and prints the scanf return value and
 * the resulting `x` so the two implementations can be compared directly.
 *
 * This file is NOT part of c_src/ and does not modify it; it only re-issues the
 * same glibc call with the same argument types.
 */
#include <stdio.h>

int main(void) {
    int x = 0;
    int r = scanf("%d", &x);
    printf("%d %d\n", r, x);
    return 0;
}
