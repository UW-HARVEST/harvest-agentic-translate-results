// Realistic c2GJKCache usage: start zeroed, then reuse the cache produced by
// the previous call for the same shape pair (the documented contract).
#define _GNU_SOURCE
#include <dlfcn.h>
#include <math.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

typedef struct c2v { float x, y; } c2v;
typedef struct c2r { float c, s; } c2r;
typedef struct c2x { c2v p; c2r r; } c2x;
typedef struct c2Circle { c2v p; float r; } c2Circle;
typedef struct c2AABB { c2v min, max; } c2AABB;
typedef struct c2Capsule { c2v a, b; float r; } c2Capsule;
typedef struct c2GJKCache { float metric; int count; int iA[3]; int iB[3]; float div; } c2GJKCache;

typedef float (*gjk_t)(const void *, int, const c2x *, const void *, int, const c2x *,
                       c2v *, c2v *, int, int *, c2GJKCache *);

static uint64_t rs = 0xfeedfacecafebeefULL;
static uint32_t nextu(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return (uint32_t)(rs >> 32); }
static float frand(float lo, float hi) { return lo + (hi - lo) * ((float)(nextu() % 1000001u) / 1000000.0f); }
static c2v vr(void) { c2v v; v.x = frand(-120, 120); v.y = frand(-120, 120); return v; }

int main(int argc, char **argv) {
	void *hc = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
	void *hr = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
	if (!hc || !hr) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }
	gjk_t gc = (gjk_t)dlsym(hc, "c2GJK");
	gjk_t gr = (gjk_t)dlsym(hr, "c2GJK");
	long checks = 0; int fails = 0;

	for (int trial = 0; trial < 40000; ++trial) {
		c2Circle ci = { vr(), frand(0, 40) };
		c2AABB   bb; bb.min = vr(); bb.max = vr();
		c2Capsule ca; ca.a = vr(); ca.b = vr(); ca.r = frand(0, 40);
		c2Circle ci2 = { vr(), frand(0, 40) };
		c2AABB   bb2; bb2.min = vr(); bb2.max = vr();
		c2Capsule ca2; ca2.a = vr(); ca2.b = vr(); ca2.r = frand(0, 40);
		const void *A[3] = { &ci, &bb, &ca };
		const void *B[3] = { &ci2, &bb2, &ca2 };
		int ta = (int)(nextu() % 3), tb = (int)(nextu() % 3);
		int use_radius = (int)(nextu() % 2);
		c2x ax, bx; float g = frand(-7, 7);
		ax.p = vr(); ax.r.c = cosf(g); ax.r.s = sinf(g);
		g = frand(-7, 7);
		bx.p = vr(); bx.r.c = cosf(g); bx.r.s = sinf(g);

		c2GJKCache k1, k2;
		memset(&k1, 0, sizeof k1);
		memset(&k2, 0, sizeof k2);
		// several successive calls, slowly translating the shapes, reusing cache
		for (int step = 0; step < 8; ++step) {
			ax.p.x += 0.75f; bx.p.y -= 0.5f;
			c2v oa1, ob1, oa2, ob2; int it1 = 0, it2 = 0;
			memset(&oa1, 0x11, sizeof oa1); memset(&ob1, 0x11, sizeof ob1);
			memset(&oa2, 0x11, sizeof oa2); memset(&ob2, 0x11, sizeof ob2);
			float r1 = gc(A[ta], ta, &ax, B[tb], tb, &bx, &oa1, &ob1, use_radius, &it1, &k1);
			float r2 = gr(A[ta], ta, &ax, B[tb], tb, &bx, &oa2, &ob2, use_radius, &it2, &k2);
			checks += 5;
			if (memcmp(&r1, &r2, 4) || memcmp(&oa1, &oa2, 8) || memcmp(&ob1, &ob2, 8) ||
			    it1 != it2 || memcmp(&k1, &k2, sizeof k1)) {
				if (fails < 20)
					fprintf(stderr, "MISMATCH trial=%d step=%d ta=%d tb=%d: dist %.9g vs %.9g "
					        "it %d vs %d cachecount %d vs %d\n",
					        trial, step, ta, tb, r1, r2, it1, it2, k1.count, k2.count);
				fails++;
			}
		}
	}
	printf("cache reuse: %ld checks, %d failures\n", checks, fails);
	return fails ? 1 : 0;
}
