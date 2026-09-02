// strtoul-level oracle.  Reads hex-encoded candidate argv[1] values, one per
// line, from stdin, and reproduces main.c's parse+validate decision exactly.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include <errno.h>

static int unhex(int c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

int main(void) {
    static char line[70000];
    static char buf[35000];
    while (fgets(line, sizeof line, stdin)) {
        size_t n = strlen(line);
        while (n && (line[n-1] == '\n' || line[n-1] == '\r')) line[--n] = '\0';
        size_t m = 0;
        for (size_t i = 0; i + 1 < n; i += 2) {
            buf[m++] = (char)((unhex(line[i]) << 4) | unhex(line[i+1]));
        }
        buf[m] = '\0';

        errno = 0;
        char *endptr;
        unsigned long temp_seed = strtoul(buf, &endptr, 10);
        int err = errno;
        long off = endptr - buf;

        printf("val=%lu off=%ld erange=%d", temp_seed, off, err != 0 ? 1 : 0);
        if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX) {
            printf(" decision=err\n");
        } else {
            printf(" decision=ok seed=%u\n", (unsigned int)temp_seed);
        }
    }
    return 0;
}
