#include <dlfcn.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { float x, y; } c2v;
typedef struct { c2v p; float r; } c2Circle;
typedef struct { c2v min, max; } c2AABB;
typedef struct { uint64_t state[2]; } cn_rnd_t;
typedef struct { float x, y; } lm_vec2;

typedef c2v (*c2V_fn)(float, float);
typedef c2v (*c2vv_fn)(c2v, c2v);
typedef c2v (*c2vvv_fn)(c2v, c2v, c2v);
typedef float (*c2Dot_fn)(c2v, c2v);
typedef int (*circle_circle_fn)(c2Circle, c2Circle);
typedef int (*circle_aabb_fn)(c2Circle, c2AABB);
typedef int (*aabb_aabb_fn)(c2AABB, c2AABB);
typedef int (*f2_fn)(const void *, int, const void *, int);
typedef int (*f3_fn)(int, int);
typedef double (*f4_fn)(cn_rnd_t *);
typedef uint32_t (*f5_fn)(uint32_t);
typedef uint32_t (*f7_fn)(uint32_t, uint32_t, uint32_t);
typedef lm_vec2 (*f9_fn)(lm_vec2, lm_vec2, lm_vec2, lm_vec2);
typedef float (*f10_fn)(uint16_t);
typedef void (*color_fn)(float *, const float *);
typedef double (*agglom_fn)(
    float, float, float, float, float, float, float,
    int, int, uint64_t, uint64_t, uint32_t, uint32_t, uint32_t, uint32_t,
    float, float, float, float, float, float, float, float, uint16_t,
    float, float, float, float, float, float, float, float, float);

typedef struct {
    c2V_fn c2V;
    c2vv_fn c2Maxv;
    c2vv_fn c2Minv;
    c2vvv_fn c2Clampv;
    c2vv_fn c2Sub;
    c2Dot_fn c2Dot;
    circle_circle_fn c2CircletoCircle;
    circle_aabb_fn c2CircletoAABB;
    aabb_aabb_fn c2AABBtoAABB;
    f2_fn f2;
    f3_fn f3;
    f4_fn f4;
    f5_fn f5;
    f7_fn f7;
    f9_fn f9;
    f10_fn f10;
    color_fn f11;
    color_fn f12;
    color_fn f13;
    agglom_fn agglom;
} api;

#define LOAD(api_value, handle, name) do { \
    *(void **)(&(api_value).name) = dlsym((handle), #name); \
    if (!(api_value).name) { fprintf(stderr, "missing symbol %s\n", #name); exit(2); } \
} while (0)

static api load_api(const char *path) {
    api value;
    void *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "dlopen %s: %s\n", path, dlerror());
        exit(2);
    }
    LOAD(value, handle, c2V);
    LOAD(value, handle, c2Maxv);
    LOAD(value, handle, c2Minv);
    LOAD(value, handle, c2Clampv);
    LOAD(value, handle, c2Sub);
    LOAD(value, handle, c2Dot);
    LOAD(value, handle, c2CircletoCircle);
    LOAD(value, handle, c2CircletoAABB);
    LOAD(value, handle, c2AABBtoAABB);
    LOAD(value, handle, f2);
    LOAD(value, handle, f3);
    LOAD(value, handle, f4);
    LOAD(value, handle, f5);
    LOAD(value, handle, f7);
    LOAD(value, handle, f9);
    LOAD(value, handle, f10);
    LOAD(value, handle, f11);
    LOAD(value, handle, f12);
    LOAD(value, handle, f13);
    LOAD(value, handle, agglom);
    return value;
}

static uint64_t random_state = UINT64_C(0xd1b54a32d192ed03);

static uint64_t random_u64(void) {
    uint64_t x = random_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    random_state = x;
    return x * UINT64_C(2685821657736338717);
}

static float random_float(void) {
    int32_t numerator = (int32_t)(random_u64() % 2000001) - 1000000;
    return (float)numerator / 1024.0f;
}

static uint32_t f32_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static uint64_t f64_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void fail(const char *name, uint64_t iteration, uint64_t c_bits, uint64_t r_bits) {
    fprintf(stderr,
            "%s mismatch at %" PRIu64 ": C=%016" PRIx64 " Rust=%016" PRIx64 "\n",
            name, iteration, c_bits, r_bits);
    exit(1);
}

static void check_vec(const char *name, uint64_t i, c2v c, c2v r) {
    if (f32_bits(c.x) != f32_bits(r.x)) fail(name, i, f32_bits(c.x), f32_bits(r.x));
    if (f32_bits(c.y) != f32_bits(r.y)) fail(name, i, f32_bits(c.y), f32_bits(r.y));
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }
    api c = load_api(argv[1]);
    api r = load_api(argv[2]);

    for (uint32_t h = 0; h <= UINT16_MAX; ++h) {
        uint32_t cb = f32_bits(c.f10((uint16_t)h));
        uint32_t rb = f32_bits(r.f10((uint16_t)h));
        if (cb != rb) fail("f10", h, cb, rb);
    }

    const int edge[] = {INT32_MIN, INT32_MIN + 1, -1001, -2, -1, 0, 1, 2, 1001, INT32_MAX};
    for (size_t i = 0; i < sizeof(edge) / sizeof(edge[0]); ++i) {
        for (size_t j = 0; j < sizeof(edge) / sizeof(edge[0]); ++j) {
            int cv = c.f3(edge[i], edge[j]);
            int rv = r.f3(edge[i], edge[j]);
            if (cv != rv) fail("f3-edge", i * 10 + j, (uint32_t)cv, (uint32_t)rv);
        }
    }

    for (uint64_t i = 0; i < 200000; ++i) {
        int a = (int)random_u64();
        int b = (int)random_u64();
        int c3 = c.f3(a, b);
        int r3 = r.f3(a, b);
        if (c3 != r3) fail("f3", i, (uint32_t)c3, (uint32_t)r3);

        uint32_t u = (uint32_t)random_u64();
        uint32_t c5 = c.f5(u);
        uint32_t r5 = r.f5(u);
        if (c5 != r5) fail("f5", i, c5, r5);

        uint32_t bsize = (uint32_t)random_u64();
        uint32_t channels = (uint32_t)random_u64();
        uint32_t depth = (uint32_t)random_u64();
        uint32_t c7 = c.f7(bsize, channels, depth);
        uint32_t r7 = r.f7(bsize, channels, depth);
        if (c7 != r7) fail("f7", i, c7, r7);

        cn_rnd_t cs = {{random_u64(), random_u64()}};
        cn_rnd_t rs = cs;
        double c4 = c.f4(&cs);
        double r4 = r.f4(&rs);
        if (f64_bits(c4) != f64_bits(r4)) fail("f4", i, f64_bits(c4), f64_bits(r4));
        if (memcmp(&cs, &rs, sizeof(cs)) != 0) fail("f4-state", i, cs.state[1], rs.state[1]);

        c2v a2 = {random_float(), random_float()};
        c2v b2 = {random_float(), random_float()};
        c2v lo = {random_float(), random_float()};
        c2v hi = {random_float(), random_float()};
        check_vec("c2V", i, c.c2V(a2.x, a2.y), r.c2V(a2.x, a2.y));
        check_vec("c2Maxv", i, c.c2Maxv(a2, b2), r.c2Maxv(a2, b2));
        check_vec("c2Minv", i, c.c2Minv(a2, b2), r.c2Minv(a2, b2));
        check_vec("c2Clampv", i, c.c2Clampv(a2, lo, hi), r.c2Clampv(a2, lo, hi));
        check_vec("c2Sub", i, c.c2Sub(a2, b2), r.c2Sub(a2, b2));
        if (f32_bits(c.c2Dot(a2, b2)) != f32_bits(r.c2Dot(a2, b2)))
            fail("c2Dot", i, f32_bits(c.c2Dot(a2, b2)), f32_bits(r.c2Dot(a2, b2)));

        c2Circle ca = {a2, random_float()};
        c2Circle cb = {b2, random_float()};
        c2AABB aa = {lo, hi};
        c2AABB ab = {a2, b2};
        if (c.c2CircletoCircle(ca, cb) != r.c2CircletoCircle(ca, cb))
            fail("c2CircletoCircle", i, c.c2CircletoCircle(ca, cb), r.c2CircletoCircle(ca, cb));
        if (c.c2CircletoAABB(ca, aa) != r.c2CircletoAABB(ca, aa))
            fail("c2CircletoAABB", i, c.c2CircletoAABB(ca, aa), r.c2CircletoAABB(ca, aa));
        if (c.c2AABBtoAABB(aa, ab) != r.c2AABBtoAABB(aa, ab))
            fail("c2AABBtoAABB", i, c.c2AABBtoAABB(aa, ab), r.c2AABBtoAABB(aa, ab));
        for (int ta = -1; ta <= 2; ++ta) {
            for (int tb = -1; tb <= 2; ++tb) {
                const void *pa = ta == 0 ? (const void *)&ca : (const void *)&aa;
                const void *pb = tb == 0 ? (const void *)&cb : (const void *)&ab;
                int c2 = c.f2(pa, ta, pb, tb);
                int r2 = r.f2(pa, ta, pb, tb);
                if (c2 != r2) fail("f2", i, (uint32_t)c2, (uint32_t)r2);
            }
        }

        lm_vec2 p1 = {random_float(), random_float()};
        lm_vec2 p2 = {random_float(), random_float()};
        lm_vec2 p3 = {random_float(), random_float()};
        lm_vec2 p4 = {random_float(), random_float()};
        lm_vec2 c9 = c.f9(p1, p2, p3, p4);
        lm_vec2 r9 = r.f9(p1, p2, p3, p4);
        if (f32_bits(c9.x) != f32_bits(r9.x)) fail("f9-x", i, f32_bits(c9.x), f32_bits(r9.x));
        if (f32_bits(c9.y) != f32_bits(r9.y)) fail("f9-y", i, f32_bits(c9.y), f32_bits(r9.y));

        float src[3] = {
            (float)((int)(random_u64() % 1441) - 720),
            random_float() / 512.0f,
            random_float() / 512.0f
        };
        float co[3], ro[3];
        c.f11(co, src);
        r.f11(ro, src);
        if (memcmp(co, ro, sizeof(co)) != 0) fail("f11", i, f32_bits(co[0]), f32_bits(ro[0]));
        c.f12(co, src);
        r.f12(ro, src);
        if (memcmp(co, ro, sizeof(co)) != 0) fail("f12", i, f32_bits(co[0]), f32_bits(ro[0]));
        src[0] = random_float() / 512.0f;
        c.f13(co, src);
        r.f13(ro, src);
        if (memcmp(co, ro, sizeof(co)) != 0) fail("f13", i, f32_bits(co[0]), f32_bits(ro[0]));
    }

    for (uint64_t i = 0; i < 25000; ++i) {
        float v[23];
        for (size_t j = 0; j < 23; ++j) v[j] = random_float() / 128.0f;
        int ag_f3_1 = (int)random_u64();
        int ag_f3_2 = (int)random_u64();
        if (ag_f3_2 == 0) ag_f3_2 = 1;
        uint64_t ag_f4_1 = random_u64();
        uint64_t ag_f4_2 = random_u64();
        uint32_t ag_f5_1 = (uint32_t)random_u64();
        uint32_t ag_f7_1 = (uint32_t)random_u64();
        uint32_t ag_f7_2 = (uint32_t)random_u64();
        uint32_t ag_f7_3 = (uint32_t)random_u64();
        uint16_t half = (uint16_t)random_u64();
        double cv = c.agglom(
            v[0], v[1], v[2], v[3], v[4], v[5], v[6],
            ag_f3_1, ag_f3_2, ag_f4_1, ag_f4_2, ag_f5_1,
            ag_f7_1, ag_f7_2, ag_f7_3,
            v[7], v[8], v[9], v[10], v[11], v[12], v[13], v[14], half,
            v[15], v[16], v[17], v[18], v[19], v[20], v[20], v[21], v[22]);
        double rv = r.agglom(
            v[0], v[1], v[2], v[3], v[4], v[5], v[6],
            ag_f3_1, ag_f3_2, ag_f4_1, ag_f4_2, ag_f5_1,
            ag_f7_1, ag_f7_2, ag_f7_3,
            v[7], v[8], v[9], v[10], v[11], v[12], v[13], v[14], half,
            v[15], v[16], v[17], v[18], v[19], v[20], v[20], v[21], v[22]);
        if (f64_bits(cv) != f64_bits(rv)) fail("agglom", i, f64_bits(cv), f64_bits(rv));
    }

    puts("differential checks passed");
    return 0;
}
