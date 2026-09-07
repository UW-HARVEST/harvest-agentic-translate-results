// Is the "uninitialised c2Proxy" region matchable at all?
//
// If the C's OWN result for an out-of-range cache index changes merely because
// an unrelated call (into the other .so) happened in between, then the value
// depends on the other library's stack layout -- i.e. it is unmatchable by
// construction, and no Rust implementation can be "correct" there.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>

typedef struct c2v { float x, y; } c2v;
typedef struct c2r { float c, s; } c2r;
typedef struct c2x { c2v p; c2r r; } c2x;
typedef struct c2Circle { c2v p; float r; } c2Circle;
typedef struct c2AABB { c2v min, max; } c2AABB;
typedef struct c2GJKCache { float metric; int count; int iA[3]; int iB[3]; float div; } c2GJKCache;
typedef float (*gjk_t)(const void *, int, const c2x *, const void *, int, const c2x *,
                       c2v *, c2v *, int, int *, c2GJKCache *);

static c2AABB bb1 = { { 100, 200 }, { 300, 400 } };
static c2AABB bb2 = { { -100, -200 }, { -300, -400 } };
static c2Circle c1 = { { 0, 0 }, 1 };
static c2Circle c2 = { { 5, 5 }, 1 };

// prime the proxy slot with the AABB pair, then read verts[idx] via the cache
static void run(gjk_t g, gjk_t interleave, int idx, const char *label) {
	c2v oa, ob; int it;
	c2GJKCache k;
	g(&bb1, 1, 0, &bb2, 1, 0, &oa, &ob, 0, &it, 0);
	if (interleave) {
		// exactly what diff.c does: the same call into the OTHER library
		c2v xa, xb; int xi;
		interleave(&bb1, 1, 0, &bb2, 1, 0, &xa, &xb, 0, &xi, 0);
	}
	memset(&k, 0, sizeof k);
	k.count = 1; k.div = 1; k.iA[0] = idx; k.iB[0] = 0;
	g(&c1, 0, 0, &c2, 0, 0, &oa, &ob, 0, &it, &k);
	printf("  %-28s idx=%d -> outA=(%12g,%12g)\n", label, idx, oa.x, oa.y);
}

int main(int argc, char **argv) {
	void *hc = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
	void *hr = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
	if (!hc || !hr) { fprintf(stderr, "%s\n", dlerror()); return 2; }
	gjk_t gc = (gjk_t)dlsym(hc, "c2GJK");
	gjk_t gr = (gjk_t)dlsym(hr, "c2GJK");

	for (int idx = 1; idx < 4; ++idx) {
		printf("index %d\n", idx);
		run(gc, 0,  idx, "C alone");
		run(gc, gr, idx, "C, Rust call interleaved");
		run(gr, 0,  idx, "Rust alone");
		run(gr, gc, idx, "Rust, C call interleaved");
	}
	return 0;
}
