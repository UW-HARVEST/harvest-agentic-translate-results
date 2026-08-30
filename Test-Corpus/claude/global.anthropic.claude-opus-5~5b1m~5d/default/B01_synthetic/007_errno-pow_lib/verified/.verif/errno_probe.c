/* Which errno values can glibc's pow() actually set?
 *
 * Establishes that after `pow`, errno is always one of {0, EDOM, ERANGE}.
 * That is what makes the `errno_any_nonzero` mutant in mutate.sh an EQUIVALENT
 * mutant rather than a blind spot: `err == EDOM` and
 * `err != 0 && err != ERANGE` agree on every reachable errno value.
 */
#include <errno.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static uint64_t s = 0x5EED1234ABCD0001ULL;
static uint64_t nx(void) {
  uint64_t x = s;
  x ^= x >> 12;
  x ^= x << 25;
  x ^= x >> 27;
  s = x;
  return x * 0x2545F4914F6CDD1DULL;
}

int main(void) {
  long seen[256] = {0};
  long other = 0;
  const long N = 8000000;
  for (long i = 0; i < N; i++) {
    double b, e;
    uint64_t u = nx(), v = nx();
    if (i % 3 == 0) { /* full 2^128 bit space */
      memcpy(&b, &u, 8);
      memcpy(&e, &v, 8);
    } else if (i % 3 == 1) { /* small integers */
      b = (double)(int64_t)(u % 2000) - 1000.0;
      e = (double)(int64_t)(v % 4000) - 2000.0;
    } else { /* random base, integral exponent spanning the over/underflow band */
      memcpy(&b, &u, 8);
      e = (double)(int64_t)(v % 2200) - 1100.0;
    }
    errno = 0;
    volatile double r = pow(b, e);
    (void)r;
    int en = errno;
    if (en >= 0 && en < 256)
      seen[en]++;
    else
      other++;
  }
  printf("after %ld pow() calls, distinct errno values:\n", N);
  int unexpected = 0;
  for (int i = 0; i < 256; i++) {
    if (!seen[i]) continue;
    const char *tag = i == 0 ? "(unset)"
                    : i == EDOM ? "(EDOM)"
                    : i == ERANGE ? "(ERANGE)"
                                  : "(*** UNEXPECTED ***)";
    if (i != 0 && i != EDOM && i != ERANGE) unexpected = 1;
    printf("  errno=%-3d %-22s count=%ld\n", i, tag, seen[i]);
  }
  if (other) {
    printf("  out-of-table errno values: %ld\n", other);
    unexpected = 1;
  }
  printf(unexpected ? "RESULT: UNEXPECTED errno value seen\n"
                    : "RESULT: only {0, EDOM, ERANGE} are reachable\n");
  return unexpected;
}
