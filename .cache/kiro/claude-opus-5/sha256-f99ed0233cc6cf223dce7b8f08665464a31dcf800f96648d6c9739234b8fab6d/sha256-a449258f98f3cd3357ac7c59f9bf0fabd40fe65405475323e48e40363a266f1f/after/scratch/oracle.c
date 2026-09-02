// Reduced-scale oracle: dumps the glibc rand() stream for many seeds and the
// result of the 100-step arithmetic kernel for tricky int32 inputs.
// Lives outside c_src/ -- c_src is never modified.
#include <stdio.h>
#include <stdlib.h>
#include <limits.h>

static int kernel(int x) {
    for (int j = 0; j < 100; j++) {
        x = x * 3 + 7;
        x = x ^ (x >> 3);
        x = x - (x << 1);
        x = x / 2 + x % 7;
    }
    return x;
}

int main(void) {
    unsigned int seeds[] = {
        0u, 1u, 2u, 3u, 5u, 42u, 12345u, 65535u, 127773u, 2147483646u,
        2147483647u, 2147483648u, 2147483649u, 3000000000u, 4294967294u,
        4294967295u, 16807u, 999999999u, 1000000000u, 2000000000u
    };
    for (size_t s = 0; s < sizeof(seeds)/sizeof(seeds[0]); s++) {
        srand(seeds[s]);
        printf("SEED %u:", seeds[s]);
        for (int i = 0; i < 12; i++) printf(" %d", rand());
        printf("\n");
    }

    int vals[] = {
        0, 1, -1, 2, -2, 3, -3, 7, -7, 6, -6, 8, -8,
        INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1,
        1073741824, -1073741824, 127773, -127773,
        16807, 2147483646, -2147483647, 100, -100, 12345, -12345
    };
    for (size_t i = 0; i < sizeof(vals)/sizeof(vals[0]); i++) {
        printf("K %d -> %d\n", vals[i], kernel(vals[i]));
    }

    // Also: one step at a time for a few values, to localise any mismatch.
    int probe[] = {0, -1, INT_MIN, INT_MAX, -3, 5};
    for (size_t i = 0; i < sizeof(probe)/sizeof(probe[0]); i++) {
        int x = probe[i];
        printf("TRACE %d:", probe[i]);
        for (int j = 0; j < 8; j++) {
            x = x * 3 + 7;      printf(" a=%d", x);
            x = x ^ (x >> 3);   printf(" b=%d", x);
            x = x - (x << 1);   printf(" c=%d", x);
            x = x / 2 + x % 7;  printf(" d=%d", x);
        }
        printf("\n");
    }
    return 0;
}
