// Adversarial fuzz of the int-returning public surface using fully random
// 32-bit patterns reinterpreted as floats (NaNs, infinities, denormals...).
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

typedef struct c2v { float x, y; } c2v;
typedef struct c2Circle { c2v p; float r; } c2Circle;
typedef struct c2AABB { c2v min, max; } c2AABB;
typedef struct c2Capsule { c2v a, b; float r; } c2Capsule;

typedef int (*cap_t)(float, float, float, float, float);
typedef int (*col_t)(const void *, int, const void *, int);
typedef int (*cc_t)(c2Circle, c2Circle);
typedef int (*ca_t)(c2Circle, c2AABB);
typedef int (*ck_t)(c2Circle, c2Capsule);
typedef int (*aa_t)(c2AABB, c2AABB);
typedef int (*ak_t)(c2AABB, c2Capsule);
typedef int (*kk_t)(c2Capsule, c2Capsule);

static uint64_t rs = 0xdeadbeef12345678ULL;
static uint32_t nextu(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return (uint32_t)(rs >> 32); }
static float fbits(void) {
	uint32_t u;
	switch (nextu() % 6) {
	case 0: u = nextu(); break;                                  // anything
	case 1: u = (nextu() & 0x807fffffu) | 0x7f800000u; break;     // inf / nan
	case 2: u = nextu() & 0x807fffffu; break;                     // zero / denormal
	case 3: u = (nextu() & 0x80000000u) | 0x3f800000u; break;     // +-1
	case 4: u = (nextu() & 0x80ffffffu) | 0x42000000u; break;     // moderate magnitude
	default: u = (nextu() & 0x80ffffffu) | 0x40000000u; break;
	}
	float f; memcpy(&f, &u, 4); return f;
}

int main(int argc, char **argv) {
	void *hc = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
	void *hr = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
	if (!hc || !hr) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }
#define G(T, n) T n##_c = (T)dlsym(hc, #n), n##_r = (T)dlsym(hr, #n)
	G(cap_t, capsule);
	G(col_t, c2Collided);
	G(cc_t, c2CircletoCircle);
	G(ca_t, c2CircletoAABB);
	G(ck_t, c2CircletoCapsule);
	G(aa_t, c2AABBtoAABB);
	G(ak_t, c2AABBtoCapsule);
	G(kk_t, c2CapsuletoCapsule);
	long checks = 0; int fails = 0;

	for (int i = 0; i < 2000000; ++i) {
		float a = fbits(), b = fbits(), c = fbits(), d = fbits(), e = fbits();
		int r1 = capsule_c(a, b, c, d, e), r2 = capsule_r(a, b, c, d, e);
		checks++;
		if (r1 != r2) {
			if (fails < 20) {
				uint32_t ua, ub, uc, ud, ue;
				memcpy(&ua,&a,4); memcpy(&ub,&b,4); memcpy(&uc,&c,4); memcpy(&ud,&d,4); memcpy(&ue,&e,4);
				fprintf(stderr, "MISMATCH capsule(0x%08x,0x%08x,0x%08x,0x%08x,0x%08x) = %d vs %d\n",
				        ua, ub, uc, ud, ue, r1, r2);
			}
			fails++;
		}
	}
	printf("capsule random-bits: %ld checks, %d failures\n", checks, fails);

	for (int i = 0; i < 400000; ++i) {
		c2Circle c1 = { { fbits(), fbits() }, fbits() };
		c2Circle c2 = { { fbits(), fbits() }, fbits() };
		c2AABB b1; b1.min.x = fbits(); b1.min.y = fbits(); b1.max.x = fbits(); b1.max.y = fbits();
		c2AABB b2; b2.min.x = fbits(); b2.min.y = fbits(); b2.max.x = fbits(); b2.max.y = fbits();
		c2Capsule p1; p1.a.x = fbits(); p1.a.y = fbits(); p1.b.x = fbits(); p1.b.y = fbits(); p1.r = fbits();
		c2Capsule p2; p2.a.x = fbits(); p2.a.y = fbits(); p2.b.x = fbits(); p2.b.y = fbits(); p2.r = fbits();
#define CHK(name, expr_c, expr_r) do { checks++; int x = (expr_c), y = (expr_r); \
	if (x != y) { if (fails < 40) fprintf(stderr, "MISMATCH %s: %d vs %d\n", name, x, y); fails++; } } while (0)
		CHK("c2CircletoCircle", c2CircletoCircle_c(c1, c2), c2CircletoCircle_r(c1, c2));
		CHK("c2CircletoAABB", c2CircletoAABB_c(c1, b1), c2CircletoAABB_r(c1, b1));
		CHK("c2CircletoCapsule", c2CircletoCapsule_c(c1, p1), c2CircletoCapsule_r(c1, p1));
		CHK("c2AABBtoAABB", c2AABBtoAABB_c(b1, b2), c2AABBtoAABB_r(b1, b2));
		CHK("c2AABBtoCapsule", c2AABBtoCapsule_c(b1, p1), c2AABBtoCapsule_r(b1, p1));
		CHK("c2CapsuletoCapsule", c2CapsuletoCapsule_c(p1, p2), c2CapsuletoCapsule_r(p1, p2));
		const void *SA[3] = { &c1, &b1, &p1 };
		const void *SB[3] = { &c2, &b2, &p2 };
		for (int ta = 0; ta < 3; ++ta) for (int tb = 0; tb < 3; ++tb)
			CHK("c2Collided", c2Collided_c(SA[ta], ta, SB[tb], tb), c2Collided_r(SA[ta], ta, SB[tb], tb));
	}
	printf("booleans random-bits: %ld checks, %d failures\n", checks, fails);
	return fails ? 1 : 0;
}
