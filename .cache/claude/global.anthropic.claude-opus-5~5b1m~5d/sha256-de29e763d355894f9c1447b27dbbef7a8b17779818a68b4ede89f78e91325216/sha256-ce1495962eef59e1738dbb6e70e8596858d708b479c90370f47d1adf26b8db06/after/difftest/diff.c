// Differential tester: dlopen the C .so and the Rust .so, call every exported
// symbol with identical inputs, compare results bit-for-bit.
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
typedef struct c2Proxy { float radius; int count; c2v verts[8]; } c2Proxy;
typedef struct c2sv { c2v sA, sB, p; float u; int iA, iB; } c2sv;
typedef struct c2Simplex { c2sv a, b, c, d; float div; int count; } c2Simplex;

typedef struct {
	void *h;
	c2v   (*c2V)(float, float);
	c2v   (*c2Mulvs)(c2v, float);
	c2v   (*c2Maxv)(c2v, c2v);
	c2v   (*c2Minv)(c2v, c2v);
	c2v   (*c2Clampv)(c2v, c2v, c2v);
	c2v   (*c2Sub)(c2v, c2v);
	float (*c2Dot)(c2v, c2v);
	c2r   (*c2RotIdentity)(void);
	c2x   (*c2xIdentity)(void);
	void  (*c2BBVerts)(c2v *, c2AABB *);
	void  (*c2MakeProxy)(const void *, int, c2Proxy *);
	float (*c2Len)(c2v);
	float (*c2Det2)(c2v, c2v);
	float (*c2GJKSimplexMetric)(c2Simplex *);
	c2v   (*c2Mulrv)(c2r, c2v);
	c2v   (*c2Add)(c2v, c2v);
	c2v   (*c2Mulxv)(c2x, c2v);
	void  (*c22)(c2Simplex *);
	void  (*c23)(c2Simplex *);
	c2v   (*c2Neg)(c2v);
	c2v   (*c2Skew)(c2v);
	c2v   (*c2CCW90)(c2v);
	c2v   (*c2D)(c2Simplex *);
	int   (*c2Support)(const c2v *, int, c2v);
	void  (*c2Witness)(c2Simplex *, c2v *, c2v *);
	c2v   (*c2Div)(c2v, float);
	c2v   (*c2Norm)(c2v);
	c2v   (*c2L)(c2Simplex *);
	c2v   (*c2MulrvT)(c2r, c2v);
	float (*c2GJK)(const void *, int, const c2x *, const void *, int, const c2x *,
	               c2v *, c2v *, int, int *, c2GJKCache *);
	int   (*c2AABBtoAABB)(c2AABB, c2AABB);
	int   (*c2AABBtoCapsule)(c2AABB, c2Capsule);
	int   (*c2CapsuletoCapsule)(c2Capsule, c2Capsule);
	int   (*c2CircletoCircle)(c2Circle, c2Circle);
	int   (*c2CircletoAABB)(c2Circle, c2AABB);
	int   (*c2CircletoCapsule)(c2Circle, c2Capsule);
	int   (*c2Collided)(const void *, int, const void *, int);
	int   (*capsule)(float, float, float, float, float);
} Lib;

static int no_cache = 0;
static int cache_idx0 = 0;
static int failures = 0;
static long checks = 0;

#define LOAD(L, name) do { \
	*(void **)(&(L)->name) = dlsym((L)->h, #name); \
	if (!(L)->name) { fprintf(stderr, "missing symbol %s in %s\n", #name, path); exit(2); } \
} while (0)

static void load(Lib *L, const char *path) {
	L->h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
	if (!L->h) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); exit(2); }
	LOAD(L, c2V); LOAD(L, c2Mulvs); LOAD(L, c2Maxv); LOAD(L, c2Minv);
	LOAD(L, c2Clampv); LOAD(L, c2Sub); LOAD(L, c2Dot); LOAD(L, c2RotIdentity);
	LOAD(L, c2xIdentity); LOAD(L, c2BBVerts); LOAD(L, c2MakeProxy); LOAD(L, c2Len);
	LOAD(L, c2Det2); LOAD(L, c2GJKSimplexMetric); LOAD(L, c2Mulrv); LOAD(L, c2Add);
	LOAD(L, c2Mulxv); LOAD(L, c22); LOAD(L, c23); LOAD(L, c2Neg); LOAD(L, c2Skew);
	LOAD(L, c2CCW90); LOAD(L, c2D); LOAD(L, c2Support); LOAD(L, c2Witness);
	LOAD(L, c2Div); LOAD(L, c2Norm); LOAD(L, c2L); LOAD(L, c2MulrvT); LOAD(L, c2GJK);
	LOAD(L, c2AABBtoAABB); LOAD(L, c2AABBtoCapsule); LOAD(L, c2CapsuletoCapsule);
	LOAD(L, c2CircletoCircle); LOAD(L, c2CircletoAABB); LOAD(L, c2CircletoCapsule);
	LOAD(L, c2Collided); LOAD(L, capsule);
}

static void bad(const char *what, const char *detail) {
	if (failures < 40) fprintf(stderr, "MISMATCH %s: %s\n", what, detail);
	failures++;
}

static void cmp_mem(const char *what, const void *a, const void *b, size_t n) {
	checks++;
	if (memcmp(a, b, n) != 0) {
		char buf[512]; size_t o = 0;
		const unsigned char *pa = a, *pb = b;
		o += snprintf(buf + o, sizeof buf - o, "bytes ");
		for (size_t i = 0; i < n && o < 400; ++i)
			o += snprintf(buf + o, sizeof buf - o, "%02x/%02x ", pa[i], pb[i]);
		bad(what, buf);
	}
}

static void cmp_f(const char *what, float a, float b) {
	checks++;
	uint32_t ua, ub; memcpy(&ua, &a, 4); memcpy(&ub, &b, 4);
	if (ua != ub) {
		char buf[160];
		snprintf(buf, sizeof buf, "%.9g (0x%08x) vs %.9g (0x%08x)", a, ua, b, ub);
		bad(what, buf);
	}
}

static void cmp_i(const char *what, int a, int b) {
	checks++;
	if (a != b) { char buf[64]; snprintf(buf, sizeof buf, "%d vs %d", a, b); bad(what, buf); }
}

static void cmp_v(const char *what, c2v a, c2v b) { cmp_mem(what, &a, &b, sizeof a); }

// ---- deterministic PRNG -----------------------------------------------------
static uint64_t rs = 0x123456789abcdefULL;
static uint32_t nextu(void) {
	rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17;
	return (uint32_t)(rs >> 32);
}
static float frand(float lo, float hi) {
	return lo + (hi - lo) * ((float)(nextu() % 1000001u) / 1000000.0f);
}
// occasionally return nasty values
static int finite_only = 0;
static int nan_heavy = 0;
static float fnasty(void) {
	if (nan_heavy && (nextu() % 2)) {
		static const uint32_t nans[] = {
			0x7fc00000u, 0xffc00000u, 0x7fc00001u, 0xffc12345u,
			0x7f800001u, 0xff800001u, 0x7fbfffffu, 0xffffffffu,
			0x7fd55555u, 0xffaaaaaau, 0x7f800000u, 0xff800000u };
		uint32_t u = nans[nextu() % 12];
		float f; memcpy(&f, &u, 4); return f;
	}
	if (finite_only) {
		switch (nextu() % 16) {
		case 0: return 0.0f;
		case 1: return -0.0f;
		case 2: return 1.0f;
		case 3: return -1.0f;
		case 4: return 1.19209289550781250e-7f;
		case 5: return 1e-30f;
		case 6: return -1e-30f;
		case 7: return frand(-1e6f, 1e6f);
		default: return frand(-150.0f, 150.0f);
		}
	}
	switch (nextu() % 24) {
	case 0: return 0.0f;
	case 1: return -0.0f;
	case 2: return 1.0f;
	case 3: return -1.0f;
	case 4: return INFINITY;
	case 5: return -INFINITY;
	case 6: {   // assorted NaN payloads, quiet and signalling, both signs
		static const uint32_t nans[] = {
			0x7fc00000u, 0xffc00000u, 0x7fc00001u, 0xffc12345u,
			0x7f800001u, 0xff800001u, 0x7fbfffffu, 0xffffffffu,
			0x7fd55555u, 0xffaaaaaau };
		uint32_t u = nans[nextu() % 10];
		float f; memcpy(&f, &u, 4); return f;
	}
	case 7: return 1.19209289550781250e-7f;
	case 8: return 3.4028235e38f;
	case 9: return -3.4028235e38f;
	case 10: return 1e-30f;
	case 11: return -1e-30f;
	case 12: return frand(-1e6f, 1e6f);
	default: return frand(-150.0f, 150.0f);
	}
}
static c2v vrand(void) { c2v v; v.x = fnasty(); v.y = fnasty(); return v; }
static c2v vtame(void) { c2v v; v.x = frand(-150, 150); v.y = frand(-150, 150); return v; }

int main(int argc, char **argv) {
	if (argc != 3) { fprintf(stderr, "usage: %s libC.so libRust.so\n", argv[0]); return 2; }
	Lib C, R;
	if (getenv("FINITE_ONLY")) finite_only = 1;
	if (getenv("NAN_HEAVY")) nan_heavy = 1;
	if (getenv("NO_CACHE")) no_cache = 1;
	if (getenv("CACHE_IDX0")) cache_idx0 = 1;
	load(&C, argv[1]);
	load(&R, argv[2]);

	// ---- capsule(): the public entry point, exhaustive-ish sweep ----------
	for (int i = 0; i < 400000; ++i) {
		float a, b, c, d, e;
		if (i < 2000) {
			// grid over interesting region
			a = (float)((i % 13) * 20 - 120);
			b = (float)(((i / 13) % 13) * 20 - 120);
			c = (float)(((i / 169) % 13) * 20 - 120);
			d = (float)(((i / 2197) % 13) * 20 - 120);
			e = (float)((i % 7) * 10);
		} else if (i < 3000) {
			a = fnasty(); b = fnasty(); c = fnasty(); d = fnasty(); e = fnasty();
		} else {
			a = frand(-200, 200); b = frand(-200, 200);
			c = frand(-200, 200); d = frand(-200, 200); e = frand(-5, 60);
		}
		cmp_i("capsule", C.capsule(a, b, c, d, e), R.capsule(a, b, c, d, e));
	}
	printf("capsule: %ld checks, %d failures\n", checks, failures);

	// ---- scalar / vector helpers ------------------------------------------
	for (int i = 0; i < 200000; ++i) {
		c2v u = vrand(), v = vrand(), w = vrand();
		float f = fnasty();
		c2r rr; rr.c = fnasty(); rr.s = fnasty();
		c2x xx; xx.p = vrand(); xx.r = rr;

		cmp_v("c2V", C.c2V(u.x, u.y), R.c2V(u.x, u.y));
		cmp_v("c2Mulvs", C.c2Mulvs(u, f), R.c2Mulvs(u, f));
		cmp_v("c2Maxv", C.c2Maxv(u, v), R.c2Maxv(u, v));
		cmp_v("c2Minv", C.c2Minv(u, v), R.c2Minv(u, v));
		cmp_v("c2Clampv", C.c2Clampv(u, v, w), R.c2Clampv(u, v, w));
		cmp_v("c2Sub", C.c2Sub(u, v), R.c2Sub(u, v));
		cmp_f("c2Dot", C.c2Dot(u, v), R.c2Dot(u, v));
		cmp_f("c2Len", C.c2Len(u), R.c2Len(u));
		cmp_f("c2Det2", C.c2Det2(u, v), R.c2Det2(u, v));
		cmp_v("c2Mulrv", C.c2Mulrv(rr, u), R.c2Mulrv(rr, u));
		cmp_v("c2MulrvT", C.c2MulrvT(rr, u), R.c2MulrvT(rr, u));
		cmp_v("c2Add", C.c2Add(u, v), R.c2Add(u, v));
		cmp_v("c2Mulxv", C.c2Mulxv(xx, u), R.c2Mulxv(xx, u));
		cmp_v("c2Neg", C.c2Neg(u), R.c2Neg(u));
		cmp_v("c2Skew", C.c2Skew(u), R.c2Skew(u));
		cmp_v("c2CCW90", C.c2CCW90(u), R.c2CCW90(u));
		cmp_v("c2Div", C.c2Div(u, f), R.c2Div(u, f));
		cmp_v("c2Norm", C.c2Norm(u), R.c2Norm(u));
	}
	{
		c2r r1 = C.c2RotIdentity(), r2 = R.c2RotIdentity();
		cmp_mem("c2RotIdentity", &r1, &r2, sizeof r1);
		c2x x1 = C.c2xIdentity(), x2 = R.c2xIdentity();
		cmp_mem("c2xIdentity", &x1, &x2, sizeof x1);
	}
	printf("helpers: %ld checks, %d failures\n", checks, failures);

	// ---- c2BBVerts / c2MakeProxy / c2Support ------------------------------
	for (int i = 0; i < 100000; ++i) {
		c2AABB bb; bb.min = vrand(); bb.max = vrand();
		c2v o1[4], o2[4];
		memset(o1, 0xAA, sizeof o1); memset(o2, 0xAA, sizeof o2);
		C.c2BBVerts(o1, &bb); R.c2BBVerts(o2, &bb);
		cmp_mem("c2BBVerts", o1, o2, sizeof o1);

		c2Circle ci; ci.p = vrand(); ci.r = fnasty();
		c2Capsule ca; ca.a = vrand(); ca.b = vrand(); ca.r = fnasty();
		c2Proxy p1, p2;
		for (int t = 0; t < 3; ++t) {
			const void *shape = t == 0 ? (void *)&ci : t == 1 ? (void *)&bb : (void *)&ca;
			memset(&p1, 0, sizeof p1); memset(&p2, 0, sizeof p2);
			C.c2MakeProxy(shape, t, &p1); R.c2MakeProxy(shape, t, &p2);
			cmp_mem("c2MakeProxy", &p1, &p2, sizeof p1);
		}
		c2v verts[8];
		for (int k = 0; k < 8; ++k) verts[k] = vrand();
		int n = 1 + (int)(nextu() % 8);
		c2v dd = vrand();
		cmp_i("c2Support", C.c2Support(verts, n, dd), R.c2Support(verts, n, dd));
	}
	printf("proxy/support: %ld checks, %d failures\n", checks, failures);

	// ---- simplex routines -------------------------------------------------
	for (int i = 0; i < 200000; ++i) {
		c2Simplex s;
		memset(&s, 0, sizeof s);
		c2sv *vs = &s.a;
		for (int k = 0; k < 4; ++k) {
			vs[k].sA = vrand(); vs[k].sB = vrand(); vs[k].p = vrand();
			vs[k].u = fnasty();
			vs[k].iA = (int)(nextu() % 8); vs[k].iB = (int)(nextu() % 8);
		}
		s.div = fnasty();
		s.count = (int)(nextu() % 5);

		c2Simplex s1 = s, s2 = s;
		cmp_f("c2GJKSimplexMetric", C.c2GJKSimplexMetric(&s1), R.c2GJKSimplexMetric(&s2));
		cmp_mem("c2GJKSimplexMetric.state", &s1, &s2, sizeof s1);

		s1 = s; s2 = s;
		cmp_v("c2D", C.c2D(&s1), R.c2D(&s2));
		cmp_mem("c2D.state", &s1, &s2, sizeof s1);

		s1 = s; s2 = s;
		cmp_v("c2L", C.c2L(&s1), R.c2L(&s2));
		cmp_mem("c2L.state", &s1, &s2, sizeof s1);

		s1 = s; s2 = s;
		C.c22(&s1); R.c22(&s2);
		cmp_mem("c22", &s1, &s2, sizeof s1);

		s1 = s; s2 = s;
		C.c23(&s1); R.c23(&s2);
		cmp_mem("c23", &s1, &s2, sizeof s1);

		s1 = s; s2 = s;
		c2v wa1, wb1, wa2, wb2;
		memset(&wa1, 0x5A, sizeof wa1); memset(&wb1, 0x5A, sizeof wb1);
		memset(&wa2, 0x5A, sizeof wa2); memset(&wb2, 0x5A, sizeof wb2);
		C.c2Witness(&s1, &wa1, &wb1); R.c2Witness(&s2, &wa2, &wb2);
		cmp_mem("c2Witness.a", &wa1, &wa2, sizeof wa1);
		cmp_mem("c2Witness.b", &wb1, &wb2, sizeof wb1);
		cmp_mem("c2Witness.state", &s1, &s2, sizeof s1);
	}
	printf("simplex: %ld checks, %d failures\n", checks, failures);

	// ---- shape-vs-shape booleans ------------------------------------------
	for (int i = 0; i < 200000; ++i) {
		int tame = (i % 4) != 0;
		c2Circle c1, c2_;
		c2AABB b1, b2;
		c2Capsule p1, p2;
		c1.p = tame ? vtame() : vrand(); c1.r = tame ? frand(0, 60) : fnasty();
		c2_.p = tame ? vtame() : vrand(); c2_.r = tame ? frand(0, 60) : fnasty();
		b1.min = tame ? vtame() : vrand(); b1.max = tame ? vtame() : vrand();
		b2.min = tame ? vtame() : vrand(); b2.max = tame ? vtame() : vrand();
		p1.a = tame ? vtame() : vrand(); p1.b = tame ? vtame() : vrand();
		p1.r = tame ? frand(0, 60) : fnasty();
		p2.a = tame ? vtame() : vrand(); p2.b = tame ? vtame() : vrand();
		p2.r = tame ? frand(0, 60) : fnasty();

		cmp_i("c2CircletoCircle", C.c2CircletoCircle(c1, c2_), R.c2CircletoCircle(c1, c2_));
		cmp_i("c2CircletoAABB", C.c2CircletoAABB(c1, b1), R.c2CircletoAABB(c1, b1));
		cmp_i("c2CircletoCapsule", C.c2CircletoCapsule(c1, p1), R.c2CircletoCapsule(c1, p1));
		cmp_i("c2AABBtoAABB", C.c2AABBtoAABB(b1, b2), R.c2AABBtoAABB(b1, b2));
		cmp_i("c2AABBtoCapsule", C.c2AABBtoCapsule(b1, p1), R.c2AABBtoCapsule(b1, p1));
		cmp_i("c2CapsuletoCapsule", C.c2CapsuletoCapsule(p1, p2), R.c2CapsuletoCapsule(p1, p2));

		const void *shapes[3][2] = { { &c1, &c2_ }, { &b1, &b2 }, { &p1, &p2 } };
		for (int ta = 0; ta < 3; ++ta)
			for (int tb = 0; tb < 3; ++tb)
				cmp_i("c2Collided",
				      C.c2Collided(shapes[ta][0], ta, shapes[tb][1], tb),
				      R.c2Collided(shapes[ta][0], ta, shapes[tb][1], tb));
		// invalid type ids take the `default:` branches
		cmp_i("c2Collided.bad", C.c2Collided(&c1, 7, &c2_, 0), R.c2Collided(&c1, 7, &c2_, 0));
		cmp_i("c2Collided.bad2", C.c2Collided(&c1, 0, &c2_, 9), R.c2Collided(&c1, 0, &c2_, 9));
		cmp_i("c2Collided.bad3", C.c2Collided(&b1, 1, &c2_, 9), R.c2Collided(&b1, 1, &c2_, 9));
		cmp_i("c2Collided.bad4", C.c2Collided(&p1, 2, &c2_, 9), R.c2Collided(&p1, 2, &c2_, 9));
	}
	printf("booleans: %ld checks, %d failures\n", checks, failures);

	// ---- c2GJK full surface ----------------------------------------------
	for (int i = 0; i < 200000; ++i) {
		int tame = (i % 4) != 0;
		c2Circle ci; ci.p = tame ? vtame() : vrand(); ci.r = tame ? frand(0, 60) : fnasty();
		c2AABB bb; bb.min = tame ? vtame() : vrand(); bb.max = tame ? vtame() : vrand();
		c2Capsule ca; ca.a = tame ? vtame() : vrand(); ca.b = tame ? vtame() : vrand();
		ca.r = tame ? frand(0, 60) : fnasty();
		c2Circle ci2; ci2.p = tame ? vtame() : vrand(); ci2.r = tame ? frand(0, 60) : fnasty();
		c2AABB bb2; bb2.min = tame ? vtame() : vrand(); bb2.max = tame ? vtame() : vrand();
		c2Capsule ca2; ca2.a = tame ? vtame() : vrand(); ca2.b = tame ? vtame() : vrand();
		ca2.r = tame ? frand(0, 60) : fnasty();

		const void *A[3] = { &ci, &bb, &ca };
		const void *B[3] = { &ci2, &bb2, &ca2 };

		c2x ax, bx;
		float ang = frand(-7, 7);
		ax.p = tame ? vtame() : vrand(); ax.r.c = cosf(ang); ax.r.s = sinf(ang);
		ang = frand(-7, 7);
		bx.p = tame ? vtame() : vrand(); bx.r.c = cosf(ang); bx.r.s = sinf(ang);

		int use_ax = (int)(nextu() % 2), use_bx = (int)(nextu() % 2);
		int use_radius = (int)(nextu() % 2);
		int use_cache = no_cache ? 0 : (int)(nextu() % 2);

		for (int ta = 0; ta < 3; ++ta) for (int tb = 0; tb < 3; ++tb) {
			c2GJKCache k1, k2;
			memset(&k1, 0, sizeof k1); memset(&k2, 0, sizeof k2);
			if (use_cache && (nextu() % 2)) {
				k1.metric = frand(-200, 200);
				k1.count = (int)(nextu() % 4);
				for (int q = 0; q < 3; ++q) {
					/* index 0 always refers to an initialised proxy vertex;
					   larger indices make the C read uninitialised stack. */
					k1.iA[q] = cache_idx0 ? 0 : (int)(nextu() % 4);
					k1.iB[q] = cache_idx0 ? 0 : (int)(nextu() % 4);
				}
				k1.div = frand(-10, 10);
				k2 = k1;
			}
			c2v oa1, ob1, oa2, ob2;
			int it1 = -12345, it2 = -12345;
			memset(&oa1, 0x33, sizeof oa1); memset(&ob1, 0x33, sizeof ob1);
			memset(&oa2, 0x33, sizeof oa2); memset(&ob2, 0x33, sizeof ob2);
			float r1 = C.c2GJK(A[ta], ta, use_ax ? &ax : 0, B[tb], tb, use_bx ? &bx : 0,
			                   &oa1, &ob1, use_radius, &it1, use_cache ? &k1 : 0);
			float r2 = R.c2GJK(A[ta], ta, use_ax ? &ax : 0, B[tb], tb, use_bx ? &bx : 0,
			                   &oa2, &ob2, use_radius, &it2, use_cache ? &k2 : 0);
			cmp_f("c2GJK.dist", r1, r2);
			cmp_mem("c2GJK.outA", &oa1, &oa2, sizeof oa1);
			cmp_mem("c2GJK.outB", &ob1, &ob2, sizeof ob1);
			cmp_i("c2GJK.iters", it1, it2);
			cmp_mem("c2GJK.cache", &k1, &k2, sizeof k1);
			// null-out variants
			float n1 = C.c2GJK(A[ta], ta, 0, B[tb], tb, 0, 0, 0, use_radius, 0, 0);
			float n2 = R.c2GJK(A[ta], ta, 0, B[tb], tb, 0, 0, 0, use_radius, 0, 0);
			cmp_f("c2GJK.null", n1, n2);
		}
	}
	printf("c2GJK: %ld checks, %d failures\n", checks, failures);

	printf("\nTOTAL: %ld checks, %d failures\n", checks, failures);
	return failures ? 1 : 0;
}
