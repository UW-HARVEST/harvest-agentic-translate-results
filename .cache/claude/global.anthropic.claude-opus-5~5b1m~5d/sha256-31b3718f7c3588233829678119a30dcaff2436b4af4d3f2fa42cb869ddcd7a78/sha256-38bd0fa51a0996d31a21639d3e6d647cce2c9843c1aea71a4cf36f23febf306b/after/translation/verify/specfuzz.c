/* Heavy fuzz of the only header-exposed entry point, spec_ray, including
 * NaN/inf/denormal/random-bit-pattern inputs. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct c2v { float x, y; } c2v;
typedef struct c2Raycast { float t; c2v n; } c2Raycast;
typedef int (*fn)(c2Raycast *, float, float, float, float, float, float, float);

static uint32_t rs = 0xdeadbeefu;
static uint32_t nextu(void) { rs ^= rs << 13; rs ^= rs >> 17; rs ^= rs << 5; return rs; }

static const float specials[] = {
	0.0f, -0.0f, 1.0f, -1.0f, 0.5f, -0.5f, 2.0f, -2.0f, 1e-30f, 1e30f,
	1e-45f, 0.1f, -0.1f, 3.0f, -3.0f, 100.0f, 1.0f/0.0f, -1.0f/0.0f,
	0.0f/0.0f, -(0.0f/0.0f), 1.17549435e-38f, 3.4e38f, 16777216.0f, 0.25f,
};
#define NSPEC ((int)(sizeof specials / sizeof specials[0]))

static float rnd(void)
{
	uint32_t u = nextu();
	switch (u % 6) {
	case 0: return specials[nextu() % NSPEC];
	case 1: { uint32_t b = nextu(); float f; memcpy(&f, &b, 4); return f; }
	case 2: { /* sNaN / qNaN with random payload */
		uint32_t b = (nextu() & 0x807fffffu) | 0x7f800000u;
		if ((b & 0x007fffffu) == 0) b |= 1;
		float f; memcpy(&f, &b, 4); return f; }
	default: return ((float)(int32_t)nextu() / 2147483648.0f) * 20.0f;
	}
}

int main(int argc, char **argv)
{
	void *h1, *h2; fn f1, f2;
	long fails = 0, n = 4000000;
	if (argc < 3) return 2;
	h1 = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
	h2 = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
	if (!h1 || !h2) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }
	f1 = (fn)dlsym(h1, "spec_ray");
	f2 = (fn)dlsym(h2, "spec_ray");
	if (!f1 || !f2) { fprintf(stderr, "no spec_ray\n"); return 2; }
	for (long i = 0; i < n; i++) {
		float a[7];
		for (int k = 0; k < 7; k++) a[k] = rnd();
		c2Raycast o1, o2;
		memset(&o1, 0x5A, sizeof o1);
		memset(&o2, 0x5A, sizeof o2);
		int r1 = f1(&o1, a[0], a[1], a[2], a[3], a[4], a[5], a[6]);
		int r2 = f2(&o2, a[0], a[1], a[2], a[3], a[4], a[5], a[6]);
		if (r1 != r2 || memcmp(&o1, &o2, sizeof o1) != 0) {
			if (fails < 10) {
				printf("MISMATCH args:");
				for (int k = 0; k < 7; k++) {
					uint32_t b; memcpy(&b, &a[k], 4);
					printf(" %08x", b);
				}
				printf("  ret %d/%d out ", r1, r2);
				const unsigned char *x = (void *)&o1, *y = (void *)&o2;
				for (size_t j = 0; j < sizeof o1; j++) printf("%02x", x[j]);
				printf("/");
				for (size_t j = 0; j < sizeof o1; j++) printf("%02x", y[j]);
				printf("\n");
			}
			fails++;
		}
	}
	printf("spec_ray: %ld cases, %ld mismatches\n", n, fails);
	return fails != 0;
}
