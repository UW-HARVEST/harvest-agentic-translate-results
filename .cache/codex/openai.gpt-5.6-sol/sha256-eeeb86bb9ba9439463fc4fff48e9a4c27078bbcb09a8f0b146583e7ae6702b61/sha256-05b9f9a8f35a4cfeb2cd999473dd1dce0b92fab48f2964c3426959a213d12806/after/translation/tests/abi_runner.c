#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int C2_TYPE;
enum { C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY };

typedef struct { float x, y; } c2v;
typedef struct { c2v n; float d; } c2h;
typedef struct { float c, s; } c2r;
typedef struct { c2v p; c2r r; } c2x;
typedef struct { c2v p; float r; } c2Circle;
typedef struct { c2v min, max; } c2AABB;
typedef struct { c2v a, b; float r; } c2Capsule;
typedef struct { int count; c2v verts[8], norms[8]; } c2Poly;
typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;
typedef struct {
    c2v sA, sB, p;
    float u;
    int iA, iB;
} c2sv;
typedef struct {
    c2sv a, b, c, d;
    float div;
    int count;
} c2Simplex;
typedef struct {
    float metric;
    int count;
    int iA[3], iB[3];
    float div;
} c2GJKCache;
typedef struct {
    int count;
    float depths[2];
    c2v contact_points[2];
    c2v n;
} c2Manifold;

c2v c2V(float, float);
c2v c2Mulvs(c2v, float);
c2v c2Maxv(c2v, c2v);
c2v c2Minv(c2v, c2v);
c2v c2Clampv(c2v, c2v, c2v);
c2v c2Sub(c2v, c2v);
float c2Dot(c2v, c2v);
float c2Dist(c2h, c2v);
c2h c2PlaneAt(const c2Poly *, int);
c2r c2RotIdentity(void);
c2x c2xIdentity(void);
void c2BBVerts(c2v *, c2AABB *);
void c2MakeProxy(const void *, C2_TYPE, c2Proxy *);
float c2Len(c2v);
float c2Det2(c2v, c2v);
float c2GJKSimplexMetric(c2Simplex *);
c2v c2Mulrv(c2r, c2v);
c2v c2MulrvT(c2r, c2v);
c2v c2Add(c2v, c2v);
c2v c2Mulxv(c2x, c2v);
c2v c2MulxvT(c2x, c2v);
c2v c2Intersect(c2v, c2v, float, float);
c2v c2Div(c2v, float);
c2v c2Norm(c2v);
c2v c2Neg(c2v);
c2v c2CCW90(c2v);
void c22(c2Simplex *);
void c23(c2Simplex *);
c2v c2Skew(c2v);
c2v c2D(c2Simplex *);
int c2Support(const c2v *, int, c2v);
void c2Witness(c2Simplex *, c2v *, c2v *);
c2v c2L(c2Simplex *);
float c2GJK(const void *, C2_TYPE, const c2x *, const void *, C2_TYPE,
            const c2x *, c2v *, c2v *, int, int *, c2GJKCache *);
c2v c2Absv(c2v);
void c2CircletoCircleManifold(c2Circle, c2Circle, c2Manifold *);
void c2CircletoAABBManifold(c2Circle, c2AABB, c2Manifold *);
void c2CircletoCapsuleManifold(c2Circle, c2Capsule, c2Manifold *);
void c2AABBtoAABBManifold(c2AABB, c2AABB, c2Manifold *);
void c2CapsuletoPolyManifold(c2Capsule, const c2Poly *, const c2x *,
                             c2Manifold *);
void c2Norms(c2v *, c2v *, int);
void c2AABBtoCapsuleManifold(c2AABB, c2Capsule, c2Manifold *);
void c2CapsuletoCapsuleManifold(c2Capsule, c2Capsule, c2Manifold *);
void c2Collide(const void *, C2_TYPE, const void *, C2_TYPE, c2Manifold *);
void *ptr_from_parts(C2_TYPE, float, float, float, float, float);

static uint32_t state = 0x19c77d31u;
static FILE *output;

static uint32_t next_u32(void) {
    uint32_t x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    state = x;
    return x;
}

static float rf(float low, float high) {
    float unit = (float)(next_u32() & 0x00ffffffu) / 16777215.0f;
    return low + unit * (high - low);
}

static c2v rv(void) {
    c2v v = {rf(-20.0f, 20.0f), rf(-20.0f, 20.0f)};
    return v;
}

static void dump(const void *value, size_t size) {
    if (fwrite(value, size, 1, output) != 1) {
        perror("fwrite");
        exit(2);
    }
}

static void *must_sym(void *handle, const char *name) {
    void *result = dlsym(handle, name);
    if (!result) {
        fprintf(stderr, "dlsym(%s): %s\n", name, dlerror());
        exit(2);
    }
    return result;
}

#define LOAD(name) \
    __typeof__(&name) p_##name = (__typeof__(&name))must_sym(handle, #name)
#define DUMP(value) dump(&(value), sizeof(value))

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s LIBRARY OUTPUT\n", argv[0]);
        return 2;
    }
    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }
    output = fopen(argv[2], "wb");
    if (!output) {
        perror("fopen");
        return 2;
    }

    LOAD(c2V); LOAD(c2Mulvs); LOAD(c2Maxv); LOAD(c2Minv); LOAD(c2Clampv);
    LOAD(c2Sub); LOAD(c2Dot); LOAD(c2Dist); LOAD(c2PlaneAt);
    LOAD(c2RotIdentity); LOAD(c2xIdentity); LOAD(c2BBVerts);
    LOAD(c2MakeProxy); LOAD(c2Len); LOAD(c2Det2); LOAD(c2GJKSimplexMetric);
    LOAD(c2Mulrv); LOAD(c2MulrvT); LOAD(c2Add); LOAD(c2Mulxv);
    LOAD(c2MulxvT); LOAD(c2Intersect); LOAD(c2Div); LOAD(c2Norm);
    LOAD(c2Neg); LOAD(c2CCW90); LOAD(c22); LOAD(c23); LOAD(c2Skew);
    LOAD(c2D); LOAD(c2Support); LOAD(c2Witness); LOAD(c2L); LOAD(c2GJK);
    LOAD(c2Absv); LOAD(c2CircletoCircleManifold);
    LOAD(c2CircletoAABBManifold); LOAD(c2CircletoCapsuleManifold);
    LOAD(c2AABBtoAABBManifold); LOAD(c2CapsuletoPolyManifold);
    LOAD(c2Norms); LOAD(c2AABBtoCapsuleManifold);
    LOAD(c2CapsuletoCapsuleManifold); LOAD(c2Collide); LOAD(ptr_from_parts);

    for (int i = 0; i < 2000; ++i) {
        c2v a = rv(), b = rv(), lo = {-10.0f, -10.0f}, hi = {10.0f, 10.0f};
        float scalar = rf(0.1f, 10.0f), da = rf(-10.0f, -0.1f);
        float db = rf(0.1f, 10.0f);
        c2h h = {rv(), rf(-10.0f, 10.0f)};
        c2r r = {rf(-1.0f, 1.0f), rf(-1.0f, 1.0f)};
        c2x x = {rv(), r};
        c2v v;
        float f;

        v = p_c2V(a.x, a.y); DUMP(v);
        v = p_c2Mulvs(a, scalar); DUMP(v);
        v = p_c2Maxv(a, b); DUMP(v);
        v = p_c2Minv(a, b); DUMP(v);
        v = p_c2Clampv(a, lo, hi); DUMP(v);
        v = p_c2Sub(a, b); DUMP(v);
        f = p_c2Dot(a, b); DUMP(f);
        f = p_c2Dist(h, a); DUMP(f);
        f = p_c2Len(a); DUMP(f);
        f = p_c2Det2(a, b); DUMP(f);
        v = p_c2Mulrv(r, a); DUMP(v);
        v = p_c2MulrvT(r, a); DUMP(v);
        v = p_c2Add(a, b); DUMP(v);
        v = p_c2Mulxv(x, a); DUMP(v);
        v = p_c2MulxvT(x, a); DUMP(v);
        v = p_c2Intersect(a, b, da, db); DUMP(v);
        v = p_c2Div(a, scalar); DUMP(v);
        v = p_c2Norm(a); DUMP(v);
        v = p_c2Neg(a); DUMP(v);
        v = p_c2CCW90(a); DUMP(v);
        v = p_c2Skew(a); DUMP(v);
        v = p_c2Absv(a); DUMP(v);
    }

    {
        c2r r = p_c2RotIdentity();
        c2x x = p_c2xIdentity();
        DUMP(r); DUMP(x);
    }

    for (int i = 0; i < 1000; ++i) {
        c2AABB box = {rv(), rv()};
        if (box.min.x > box.max.x) {
            float t = box.min.x; box.min.x = box.max.x; box.max.x = t;
        }
        if (box.min.y > box.max.y) {
            float t = box.min.y; box.min.y = box.max.y; box.max.y = t;
        }
        c2v verts[8], norms[8];
        memset(verts, 0xa5, sizeof(verts));
        memset(norms, 0xa5, sizeof(norms));
        p_c2BBVerts(verts, &box);
        p_c2Norms(verts, norms, 4);
        dump(verts, sizeof(verts)); dump(norms, sizeof(norms));

        c2Poly poly;
        memset(&poly, 0xa5, sizeof(poly));
        poly.count = 4;
        memcpy(poly.verts, verts, 4 * sizeof(c2v));
        memcpy(poly.norms, norms, 4 * sizeof(c2v));
        int edge = (int)(next_u32() % 4);
        c2h plane = p_c2PlaneAt(&poly, edge);
        DUMP(plane);

        c2Circle circle = {rv(), rf(0.05f, 5.0f)};
        c2Capsule capsule = {rv(), rv(), rf(0.05f, 5.0f)};
        c2Proxy proxy;
        memset(&proxy, 0xa5, sizeof(proxy));
        p_c2MakeProxy(&circle, C2_TYPE_CIRCLE, &proxy); DUMP(proxy);
        memset(&proxy, 0xa5, sizeof(proxy));
        p_c2MakeProxy(&box, C2_TYPE_AABB, &proxy); DUMP(proxy);
        memset(&proxy, 0xa5, sizeof(proxy));
        p_c2MakeProxy(&capsule, C2_TYPE_CAPSULE, &proxy); DUMP(proxy);

        c2v support = rv();
        int index = p_c2Support(verts, 4, support);
        DUMP(index);

        c2Simplex simplex;
        memset(&simplex, 0xa5, sizeof(simplex));
        simplex.a.sA = rv(); simplex.a.sB = rv();
        simplex.b.sA = rv(); simplex.b.sB = rv();
        simplex.c.sA = rv(); simplex.c.sB = rv();
        simplex.a.p = rv(); simplex.b.p = rv(); simplex.c.p = rv();
        simplex.a.u = rf(0.1f, 5.0f);
        simplex.b.u = rf(0.1f, 5.0f);
        simplex.c.u = rf(0.1f, 5.0f);
        simplex.div = simplex.a.u + simplex.b.u + simplex.c.u;
        simplex.count = 1 + (int)(next_u32() % 3);
        float metric = p_c2GJKSimplexMetric(&simplex);
        c2v direction = p_c2D(&simplex);
        c2v witness_a, witness_b;
        p_c2Witness(&simplex, &witness_a, &witness_b);
        c2v location = p_c2L(&simplex);
        DUMP(metric); DUMP(direction); DUMP(witness_a); DUMP(witness_b);
        DUMP(location);

        c2Simplex segment = simplex;
        segment.count = 2;
        p_c22(&segment);
        DUMP(segment);
        c2Simplex triangle = simplex;
        triangle.count = 3;
        p_c23(&triangle);
        DUMP(triangle);

        c2Manifold m;
        memset(&m, 0xa5, sizeof(m));
        p_c2CircletoCircleManifold(circle,
            (c2Circle){rv(), rf(0.05f, 5.0f)}, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2CircletoAABBManifold(circle, box, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2CircletoCapsuleManifold(circle, capsule, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2AABBtoAABBManifold(box, (c2AABB){rv(), rv()}, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2AABBtoCapsuleManifold(box, capsule, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2CapsuletoCapsuleManifold(capsule,
            (c2Capsule){rv(), rv(), rf(0.05f, 5.0f)}, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2CapsuletoPolyManifold(capsule, &poly, NULL, &m); DUMP(m);
        memset(&m, 0xa5, sizeof(m));
        p_c2Collide(&circle, C2_TYPE_CIRCLE, &box, C2_TYPE_AABB, &m);
        DUMP(m);

        void *allocated = p_ptr_from_parts(C2_TYPE_CAPSULE,
            capsule.a.x, capsule.a.y, capsule.b.x, capsule.b.y, capsule.r);
        dump(allocated, sizeof(capsule));
        free(allocated);
    }

    for (int type_a = 0; type_a < 3; ++type_a) {
        for (int type_b = 0; type_b < 3; ++type_b) {
            c2Circle ca = {rv(), rf(0.05f, 5.0f)};
            c2AABB aa = {rv(), rv()};
            c2Capsule xa = {rv(), rv(), rf(0.05f, 5.0f)};
            c2Circle cb = {rv(), rf(0.05f, 5.0f)};
            c2AABB ab = {rv(), rv()};
            c2Capsule xb = {rv(), rv(), rf(0.05f, 5.0f)};
            void *a[] = {&xa, &ca, &aa};
            void *b[] = {&xb, &cb, &ab};
            c2v out_a, out_b;
            int iterations;
            c2GJKCache cache;
            memset(&cache, 0, sizeof(cache));
            float distance = p_c2GJK(a[type_a], type_a, NULL,
                b[type_b], type_b, NULL, &out_a, &out_b, 1, &iterations,
                &cache);
            DUMP(distance); DUMP(out_a); DUMP(out_b); DUMP(iterations);
            DUMP(cache);
            distance = p_c2GJK(a[type_a], type_a, NULL,
                b[type_b], type_b, NULL, &out_a, &out_b, 0, &iterations,
                &cache);
            DUMP(distance); DUMP(out_a); DUMP(out_b); DUMP(iterations);
            DUMP(cache);
        }
    }

    if (fclose(output) != 0) {
        perror("fclose");
        return 2;
    }
    dlclose(handle);
    return 0;
}
