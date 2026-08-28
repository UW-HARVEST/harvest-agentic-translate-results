/* Differential tester: dlopens the C .so and the Rust .so, calls every
 * exported symbol with identical inputs and compares the raw bit patterns of
 * the return values and of the `out` structures. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct c2v { float x, y; } c2v;
typedef struct c2Raycast { float t; c2v n; } c2Raycast;
typedef struct c2Circle { c2v p; float r; } c2Circle;
typedef struct c2AABB { c2v min, max; } c2AABB;
typedef struct c2Capsule { c2v a, b; float r; } c2Capsule;
typedef struct c2Ray { c2v p, d; float t; } c2Ray;
typedef struct c2m { c2v x, y; } c2m;

typedef struct {
	c2v (*c2V)(float, float);
	float (*c2Dot)(c2v, c2v);
	float (*c2Len)(c2v);
	c2v (*c2Add)(c2v, c2v);
	c2v (*c2Sub)(c2v, c2v);
	c2v (*c2Mulvs)(c2v, float);
	c2v (*c2Div)(c2v, float);
	c2v (*c2Norm)(c2v);
	c2v (*c2Minv)(c2v, c2v);
	c2v (*c2Maxv)(c2v, c2v);
	c2v (*c2Skew)(c2v);
	c2v (*c2Absv)(c2v);
	c2v (*c2CCW90)(c2v);
	c2v (*c2MulmvT)(c2m, c2v);
	int (*c2AABBtoAABB)(c2AABB, c2AABB);
	int (*c2AABBtoPoint)(c2AABB, c2v);
	int (*c2CircleToPoint)(c2Circle, c2v);
	int (*c2RaytoCircle)(c2Ray, c2Circle, c2Raycast *);
	int (*c2RaytoAABB)(c2Ray, c2AABB, c2Raycast *);
	int (*c2RaytoCapsule)(c2Ray, c2Capsule, c2Raycast *);
	int (*c2CastRay)(c2Ray, const void *, int, c2Raycast *);
	int (*spec_ray)(c2Raycast *, float, float, float, float, float, float, float);
} api;

static void *H;
#define LOAD(f)                                                                \
	do {                                                                   \
		*(void **)(&a->f) = dlsym(H, #f);                               \
		if (!a->f) {                                                   \
			fprintf(stderr, "missing symbol %s\n", #f);            \
			return 1;                                              \
		}                                                              \
	} while (0)

static int load(api *a, const char *path)
{
	H = dlopen(path, RTLD_NOW | RTLD_LOCAL);
	if (!H) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); return 1; }
	LOAD(c2V); LOAD(c2Dot); LOAD(c2Len); LOAD(c2Add); LOAD(c2Sub);
	LOAD(c2Mulvs); LOAD(c2Div); LOAD(c2Norm); LOAD(c2Minv); LOAD(c2Maxv);
	LOAD(c2Skew); LOAD(c2Absv); LOAD(c2CCW90); LOAD(c2MulmvT);
	LOAD(c2AABBtoAABB); LOAD(c2AABBtoPoint); LOAD(c2CircleToPoint);
	LOAD(c2RaytoCircle); LOAD(c2RaytoAABB); LOAD(c2RaytoCapsule);
	LOAD(c2CastRay); LOAD(spec_ray);
	return 0;
}

static long fails, checks;

static void cmp(const char *what, const void *pa, const void *pb, size_t n)
{
	checks++;
	if (memcmp(pa, pb, n) != 0) {
		if (fails < 40) {
			const unsigned char *x = pa, *y = pb;
			printf("MISMATCH %s:  C=", what);
			for (size_t i = 0; i < n; i++) printf("%02x", x[i]);
			printf("  R=");
			for (size_t i = 0; i < n; i++) printf("%02x", y[i]);
			printf("\n");
		}
		fails++;
	}
}

/* xorshift RNG so both libraries see identical inputs */
static uint32_t rs = 0x12345678u;
static uint32_t nextu(void)
{
	rs ^= rs << 13; rs ^= rs >> 17; rs ^= rs << 5;
	return rs;
}

static const float specials[] = {
	0.0f, -0.0f, 1.0f, -1.0f, 0.5f, -0.5f, 2.0f, -2.0f, 3.0f, 1e-30f,
	-1e-30f, 1e30f, -1e30f, 1e-45f, 16777216.0f, 0.1f, -0.1f, 100.0f,
	-100.0f, 1.0f/0.0f, -1.0f/0.0f, 0.0f/0.0f, 1.17549435e-38f, 3.4e38f,
};
#define NSPEC ((int)(sizeof specials / sizeof specials[0]))

static float rnd(void)
{
	uint32_t u = nextu();
	switch (u % 8) {
	case 0: return specials[nextu() % NSPEC];
	case 1: { /* fully random bit pattern */
		uint32_t b = nextu(); float f; memcpy(&f, &b, 4); return f; }
	default: { /* nice-ish value in [-50, 50] */
		return ((float)(int32_t)nextu() / 2147483648.0f) * 50.0f; }
	}
}

static c2v rv(void) { c2v v; v.x = rnd(); v.y = rnd(); return v; }

int main(int argc, char **argv)
{
	api A, B;
	if (argc < 3) { fprintf(stderr, "usage: %s c.so rust.so\n", argv[0]); return 2; }
	if (load(&A, argv[1])) return 2;
	if (load(&B, argv[2])) return 2;

	long N = 400000;
	for (long i = 0; i < N; i++) {
		float f0 = rnd(), f1 = rnd(), f2 = rnd(), f3 = rnd();
		c2v v0 = rv(), v1 = rv();
		c2m m; m.x = rv(); m.y = rv();
		c2AABB bb0, bb1;
		bb0.min = rv(); bb0.max = rv(); bb1.min = rv(); bb1.max = rv();
		c2Circle ci; ci.p = rv(); ci.r = rnd();
		c2Capsule cap; cap.a = rv(); cap.b = rv(); cap.r = rnd();
		c2Ray ray; ray.p = rv(); ray.d = rv(); ray.t = rnd();
		c2v ra, rb;
		float fa, fb;
		int ia, ib;
		c2Raycast oa, ob;

		ra = A.c2V(f0, f1); rb = B.c2V(f0, f1); cmp("c2V", &ra, &rb, sizeof ra);
		fa = A.c2Dot(v0, v1); fb = B.c2Dot(v0, v1); cmp("c2Dot", &fa, &fb, 4);
		fa = A.c2Len(v0); fb = B.c2Len(v0); cmp("c2Len", &fa, &fb, 4);
		ra = A.c2Add(v0, v1); rb = B.c2Add(v0, v1); cmp("c2Add", &ra, &rb, sizeof ra);
		ra = A.c2Sub(v0, v1); rb = B.c2Sub(v0, v1); cmp("c2Sub", &ra, &rb, sizeof ra);
		ra = A.c2Mulvs(v0, f2); rb = B.c2Mulvs(v0, f2); cmp("c2Mulvs", &ra, &rb, sizeof ra);
		ra = A.c2Div(v0, f2); rb = B.c2Div(v0, f2); cmp("c2Div", &ra, &rb, sizeof ra);
		ra = A.c2Norm(v0); rb = B.c2Norm(v0); cmp("c2Norm", &ra, &rb, sizeof ra);
		ra = A.c2Minv(v0, v1); rb = B.c2Minv(v0, v1); cmp("c2Minv", &ra, &rb, sizeof ra);
		ra = A.c2Maxv(v0, v1); rb = B.c2Maxv(v0, v1); cmp("c2Maxv", &ra, &rb, sizeof ra);
		ra = A.c2Skew(v0); rb = B.c2Skew(v0); cmp("c2Skew", &ra, &rb, sizeof ra);
		ra = A.c2Absv(v0); rb = B.c2Absv(v0); cmp("c2Absv", &ra, &rb, sizeof ra);
		ra = A.c2CCW90(v0); rb = B.c2CCW90(v0); cmp("c2CCW90", &ra, &rb, sizeof ra);
		ra = A.c2MulmvT(m, v0); rb = B.c2MulmvT(m, v0); cmp("c2MulmvT", &ra, &rb, sizeof ra);
		ia = A.c2AABBtoAABB(bb0, bb1); ib = B.c2AABBtoAABB(bb0, bb1); cmp("c2AABBtoAABB", &ia, &ib, 4);
		ia = A.c2AABBtoPoint(bb0, v0); ib = B.c2AABBtoPoint(bb0, v0); cmp("c2AABBtoPoint", &ia, &ib, 4);
		ia = A.c2CircleToPoint(ci, v0); ib = B.c2CircleToPoint(ci, v0); cmp("c2CircleToPoint", &ia, &ib, 4);

		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2RaytoCircle(ray, ci, &oa); ib = B.c2RaytoCircle(ray, ci, &ob);
		cmp("c2RaytoCircle/ret", &ia, &ib, 4); cmp("c2RaytoCircle/out", &oa, &ob, sizeof oa);

		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2RaytoAABB(ray, bb1, &oa); ib = B.c2RaytoAABB(ray, bb1, &ob);
		cmp("c2RaytoAABB/ret", &ia, &ib, 4); cmp("c2RaytoAABB/out", &oa, &ob, sizeof oa);

		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2RaytoCapsule(ray, cap, &oa); ib = B.c2RaytoCapsule(ray, cap, &ob);
		cmp("c2RaytoCapsule/ret", &ia, &ib, 4); cmp("c2RaytoCapsule/out", &oa, &ob, sizeof oa);

		/* c2CastRay over each valid shape type */
		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2CastRay(ray, &ci, 0, &oa); ib = B.c2CastRay(ray, &ci, 0, &ob);
		cmp("c2CastRay0/ret", &ia, &ib, 4); cmp("c2CastRay0/out", &oa, &ob, sizeof oa);
		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2CastRay(ray, &bb1, 1, &oa); ib = B.c2CastRay(ray, &bb1, 1, &ob);
		cmp("c2CastRay1/ret", &ia, &ib, 4); cmp("c2CastRay1/out", &oa, &ob, sizeof oa);
		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.c2CastRay(ray, &cap, 2, &oa); ib = B.c2CastRay(ray, &cap, 2, &ob);
		cmp("c2CastRay2/ret", &ia, &ib, 4); cmp("c2CastRay2/out", &oa, &ob, sizeof oa);

		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.spec_ray(&oa, f0, f1, ci.p.x, ci.p.y, ci.r, ray.p.x, ray.p.y);
		ib = B.spec_ray(&ob, f0, f1, ci.p.x, ci.p.y, ci.r, ray.p.x, ray.p.y);
		cmp("spec_ray/ret", &ia, &ib, 4); cmp("spec_ray/out", &oa, &ob, sizeof oa);

		/* spec_ray with "realistic" small-magnitude inputs, hitting the circle */
		float mx = f2 * 0.3f, my = f3 * 0.3f;
		memset(&oa, 0xCC, sizeof oa); memset(&ob, 0xCC, sizeof ob);
		ia = A.spec_ray(&oa, mx, my, 0.0f, 0.0f, 1.0f, -3.0f, 0.25f);
		ib = B.spec_ray(&ob, mx, my, 0.0f, 0.0f, 1.0f, -3.0f, 0.25f);
		cmp("spec_ray2/ret", &ia, &ib, 4); cmp("spec_ray2/out", &oa, &ob, sizeof oa);
	}
	printf("checks=%ld mismatches=%ld\n", checks, fails);
	return fails != 0;
}
