#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { float x, y; } c2v;
typedef struct { float c, s; } c2r;
typedef struct { c2v p; c2r r; } c2x;
typedef struct { c2v p; float r; } c2Circle;
typedef struct { c2v min, max; } c2AABB;
typedef struct { c2v a, b; float r; } c2Capsule;
typedef struct {
    float metric;
    int count;
    int iA[3], iB[3];
    float div;
} c2GJKCache;
typedef struct {
    float radius;
    int count;
    c2v verts[8];
} c2Proxy;
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

enum { C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB };

typedef struct {
    void *handle;
    c2v (*c2V)(float, float);
    c2v (*c2Mulvs)(c2v, float);
    c2v (*c2Maxv)(c2v, c2v);
    c2v (*c2Minv)(c2v, c2v);
    c2v (*c2Clampv)(c2v, c2v, c2v);
    c2v (*c2Sub)(c2v, c2v);
    float (*c2Dot)(c2v, c2v);
    c2r (*c2RotIdentity)(void);
    c2x (*c2xIdentity)(void);
    void (*c2BBVerts)(c2v *, c2AABB *);
    void (*c2MakeProxy)(const void *, int, c2Proxy *);
    float (*c2Len)(c2v);
    float (*c2Det2)(c2v, c2v);
    float (*c2GJKSimplexMetric)(c2Simplex *);
    c2v (*c2Mulrv)(c2r, c2v);
    c2v (*c2Add)(c2v, c2v);
    c2v (*c2Mulxv)(c2x, c2v);
    void (*c22)(c2Simplex *);
    void (*c23)(c2Simplex *);
    c2v (*c2Neg)(c2v);
    c2v (*c2Skew)(c2v);
    c2v (*c2CCW90)(c2v);
    c2v (*c2D)(c2Simplex *);
    int (*c2Support)(const c2v *, int, c2v);
    void (*c2Witness)(c2Simplex *, c2v *, c2v *);
    c2v (*c2Div)(c2v, float);
    c2v (*c2Norm)(c2v);
    c2v (*c2L)(c2Simplex *);
    c2v (*c2MulrvT)(c2r, c2v);
    float (*c2GJK)(const void *, int, const c2x *, const void *, int,
                   const c2x *, c2v *, c2v *, int, int *, c2GJKCache *);
    int (*c2AABBtoAABB)(c2AABB, c2AABB);
    int (*c2AABBtoCapsule)(c2AABB, c2Capsule);
    int (*c2CapsuletoCapsule)(c2Capsule, c2Capsule);
    int (*c2CircletoCircle)(c2Circle, c2Circle);
    int (*c2CircletoAABB)(c2Circle, c2AABB);
    int (*c2CircletoCapsule)(c2Circle, c2Capsule);
    int (*c2Collided)(const void *, int, const void *, int);
    void *(*ptr_from_parts)(int, float, float, float, float, float);
    int (*omni_collide)(int, float, float, float, float, float,
                        int, float, float, float, float, float);
} Api;

#define LOAD_ONE(api, name) do { \
    *(void **)(&(api)->name) = dlsym((api)->handle, #name); \
    if (!(api)->name) { \
        fprintf(stderr, "missing symbol %s: %s\n", #name, dlerror()); \
        exit(2); \
    } \
} while (0)

static void load_api(Api *api, const char *path)
{
    memset(api, 0, sizeof(*api));
    api->handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!api->handle) {
        fprintf(stderr, "dlopen %s: %s\n", path, dlerror());
        exit(2);
    }
    LOAD_ONE(api, c2V);
    LOAD_ONE(api, c2Mulvs);
    LOAD_ONE(api, c2Maxv);
    LOAD_ONE(api, c2Minv);
    LOAD_ONE(api, c2Clampv);
    LOAD_ONE(api, c2Sub);
    LOAD_ONE(api, c2Dot);
    LOAD_ONE(api, c2RotIdentity);
    LOAD_ONE(api, c2xIdentity);
    LOAD_ONE(api, c2BBVerts);
    LOAD_ONE(api, c2MakeProxy);
    LOAD_ONE(api, c2Len);
    LOAD_ONE(api, c2Det2);
    LOAD_ONE(api, c2GJKSimplexMetric);
    LOAD_ONE(api, c2Mulrv);
    LOAD_ONE(api, c2Add);
    LOAD_ONE(api, c2Mulxv);
    LOAD_ONE(api, c22);
    LOAD_ONE(api, c23);
    LOAD_ONE(api, c2Neg);
    LOAD_ONE(api, c2Skew);
    LOAD_ONE(api, c2CCW90);
    LOAD_ONE(api, c2D);
    LOAD_ONE(api, c2Support);
    LOAD_ONE(api, c2Witness);
    LOAD_ONE(api, c2Div);
    LOAD_ONE(api, c2Norm);
    LOAD_ONE(api, c2L);
    LOAD_ONE(api, c2MulrvT);
    LOAD_ONE(api, c2GJK);
    LOAD_ONE(api, c2AABBtoAABB);
    LOAD_ONE(api, c2AABBtoCapsule);
    LOAD_ONE(api, c2CapsuletoCapsule);
    LOAD_ONE(api, c2CircletoCircle);
    LOAD_ONE(api, c2CircletoAABB);
    LOAD_ONE(api, c2CircletoCapsule);
    LOAD_ONE(api, c2Collided);
    LOAD_ONE(api, ptr_from_parts);
    LOAD_ONE(api, omni_collide);
}

static uint32_t state = 0x51f15e5du;

static uint32_t rnd(void)
{
    state = state * 1664525u + 1013904223u;
    return state;
}

static float rf(void)
{
    return (float)((int)(rnd() % 4001u) - 2000) / 16.0f;
}

static c2v rv(void)
{
    c2v v = { rf(), rf() };
    return v;
}

static void fail(const char *name, int iteration)
{
    fprintf(stderr, "mismatch: %s at iteration %d\n", name, iteration);
    exit(1);
}

#define SAME_OBJ(name, lhs, rhs, iteration) do { \
    if (memcmp(&(lhs), &(rhs), sizeof(lhs)) != 0) fail(name, iteration); \
} while (0)

#define SAME_CALL(name, type, call_c, call_r, iteration) do { \
    type lhs_ = (call_c); \
    type rhs_ = (call_r); \
    SAME_OBJ(name, lhs_, rhs_, iteration); \
} while (0)

static const void *shape_ptr(int type, const c2Circle *circle,
                             const c2AABB *aabb, const c2Capsule *capsule)
{
    if (type == C2_TYPE_CIRCLE) return circle;
    if (type == C2_TYPE_AABB) return aabb;
    return capsule;
}

static size_t shape_size(int type)
{
    if (type == C2_TYPE_CIRCLE) return sizeof(c2Circle);
    if (type == C2_TYPE_AABB) return sizeof(c2AABB);
    return sizeof(c2Capsule);
}

int main(int argc, char **argv)
{
    Api c, r;
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIB RUST_LIB\n", argv[0]);
        return 2;
    }
    load_api(&c, argv[1]);
    load_api(&r, argv[2]);

    for (int n = 0; n < 20000; ++n) {
        c2v a = rv(), b = rv(), lo = rv(), hi = rv();
        float x = rf(), y = rf();
        float scalar = (float)(rnd() % 255u + 1u) / 32.0f;
        c2r rot = { (float)((int)(rnd() % 33u) - 16) / 16.0f,
                    (float)((int)(rnd() % 33u) - 16) / 16.0f };
        c2x transform = { rv(), rot };

        SAME_CALL("c2V", c2v, c.c2V(x, y), r.c2V(x, y), n);
        SAME_CALL("c2Mulvs", c2v, c.c2Mulvs(a, scalar),
                  r.c2Mulvs(a, scalar), n);
        SAME_CALL("c2Maxv", c2v, c.c2Maxv(a, b), r.c2Maxv(a, b), n);
        SAME_CALL("c2Minv", c2v, c.c2Minv(a, b), r.c2Minv(a, b), n);
        SAME_CALL("c2Clampv", c2v, c.c2Clampv(a, lo, hi),
                  r.c2Clampv(a, lo, hi), n);
        SAME_CALL("c2Sub", c2v, c.c2Sub(a, b), r.c2Sub(a, b), n);
        SAME_CALL("c2Dot", float, c.c2Dot(a, b), r.c2Dot(a, b), n);
        SAME_CALL("c2RotIdentity", c2r, c.c2RotIdentity(),
                  r.c2RotIdentity(), n);
        SAME_CALL("c2xIdentity", c2x, c.c2xIdentity(), r.c2xIdentity(), n);
        SAME_CALL("c2Len", float, c.c2Len(a), r.c2Len(a), n);
        SAME_CALL("c2Det2", float, c.c2Det2(a, b), r.c2Det2(a, b), n);
        SAME_CALL("c2Mulrv", c2v, c.c2Mulrv(rot, a),
                  r.c2Mulrv(rot, a), n);
        SAME_CALL("c2Add", c2v, c.c2Add(a, b), r.c2Add(a, b), n);
        SAME_CALL("c2Mulxv", c2v, c.c2Mulxv(transform, a),
                  r.c2Mulxv(transform, a), n);
        SAME_CALL("c2Neg", c2v, c.c2Neg(a), r.c2Neg(a), n);
        SAME_CALL("c2Skew", c2v, c.c2Skew(a), r.c2Skew(a), n);
        SAME_CALL("c2CCW90", c2v, c.c2CCW90(a), r.c2CCW90(a), n);
        SAME_CALL("c2Div", c2v, c.c2Div(a, scalar),
                  r.c2Div(a, scalar), n);
        if (a.x != 0.0f || a.y != 0.0f) {
            SAME_CALL("c2Norm", c2v, c.c2Norm(a), r.c2Norm(a), n);
        }
        SAME_CALL("c2MulrvT", c2v, c.c2MulrvT(rot, a),
                  r.c2MulrvT(rot, a), n);

        c2AABB box = { rv(), rv() };
        c2v cv[4], rvv[4];
        c.c2BBVerts(cv, &box);
        r.c2BBVerts(rvv, &box);
        SAME_OBJ("c2BBVerts", cv, rvv, n);

        c2Circle circle = { rv(), (float)(rnd() % 200u) / 16.0f };
        c2Capsule capsule = { rv(), rv(), (float)(rnd() % 200u) / 16.0f };
        for (int type = 0; type < 3; ++type) {
            c2Proxy cp, rp;
            memset(&cp, 0xa5, sizeof(cp));
            memset(&rp, 0xa5, sizeof(rp));
            const void *shape = shape_ptr(type, &circle, &box, &capsule);
            c.c2MakeProxy(shape, type, &cp);
            r.c2MakeProxy(shape, type, &rp);
            SAME_OBJ("c2MakeProxy", cp, rp, n);
        }

        c2Simplex cs, rs;
        memset(&cs, 0, sizeof(cs));
        c2sv *verts = &cs.a;
        for (int i = 0; i < 4; ++i) {
            verts[i].sA = rv();
            verts[i].sB = rv();
            verts[i].p = rv();
            verts[i].u = (float)(rnd() % 100u + 1u) / 16.0f;
            verts[i].iA = (int)(rnd() % 4u);
            verts[i].iB = (int)(rnd() % 4u);
        }
        cs.div = (float)(rnd() % 100u + 1u) / 16.0f;
        cs.count = (int)(rnd() % 3u) + 1;
        rs = cs;

        SAME_CALL("c2GJKSimplexMetric", float,
                  c.c2GJKSimplexMetric(&cs), r.c2GJKSimplexMetric(&rs), n);
        SAME_CALL("c2D", c2v, c.c2D(&cs), r.c2D(&rs), n);
        SAME_CALL("c2L", c2v, c.c2L(&cs), r.c2L(&rs), n);
        c2v cwa, cwb, rwa, rwb;
        c.c2Witness(&cs, &cwa, &cwb);
        r.c2Witness(&rs, &rwa, &rwb);
        SAME_OBJ("c2Witness-a", cwa, rwa, n);
        SAME_OBJ("c2Witness-b", cwb, rwb, n);
        SAME_CALL("c2Support", int, c.c2Support(&cs.a.p, 1, a),
                  r.c2Support(&rs.a.p, 1, a), n);

        cs.count = 2;
        rs = cs;
        c.c22(&cs);
        r.c22(&rs);
        SAME_OBJ("c22", cs, rs, n);
        cs.count = 3;
        rs = cs;
        c.c23(&cs);
        r.c23(&rs);
        SAME_OBJ("c23", cs, rs, n);

        c2AABB box2 = { rv(), rv() };
        c2Circle circle2 = { rv(), (float)(rnd() % 200u) / 16.0f };
        c2Capsule capsule2 = { rv(), rv(),
                               (float)(rnd() % 200u) / 16.0f };
        SAME_CALL("c2AABBtoAABB", int, c.c2AABBtoAABB(box, box2),
                  r.c2AABBtoAABB(box, box2), n);
        SAME_CALL("c2AABBtoCapsule", int, c.c2AABBtoCapsule(box, capsule),
                  r.c2AABBtoCapsule(box, capsule), n);
        SAME_CALL("c2CapsuletoCapsule", int,
                  c.c2CapsuletoCapsule(capsule, capsule2),
                  r.c2CapsuletoCapsule(capsule, capsule2), n);
        SAME_CALL("c2CircletoCircle", int,
                  c.c2CircletoCircle(circle, circle2),
                  r.c2CircletoCircle(circle, circle2), n);
        SAME_CALL("c2CircletoAABB", int, c.c2CircletoAABB(circle, box),
                  r.c2CircletoAABB(circle, box), n);
        SAME_CALL("c2CircletoCapsule", int,
                  c.c2CircletoCapsule(circle, capsule),
                  r.c2CircletoCapsule(circle, capsule), n);

        for (int ta = 0; ta < 3; ++ta) {
            for (int tb = 0; tb < 3; ++tb) {
                const void *sa = shape_ptr(ta, &circle, &box, &capsule);
                const void *sb = shape_ptr(tb, &circle2, &box2, &capsule2);
                SAME_CALL("c2Collided", int,
                          c.c2Collided(sa, ta, sb, tb),
                          r.c2Collided(sa, ta, sb, tb), n);

                c2v coa, cob, roa, rob;
                int ci = -1, ri = -1;
                c2GJKCache cc, rc;
                memset(&cc, 0, sizeof(cc));
                memset(&rc, 0, sizeof(rc));
                float cd = c.c2GJK(sa, ta, NULL, sb, tb, NULL,
                                   &coa, &cob, 1, &ci, &cc);
                float rd = r.c2GJK(sa, ta, NULL, sb, tb, NULL,
                                   &roa, &rob, 1, &ri, &rc);
                SAME_OBJ("c2GJK-distance", cd, rd, n);
                SAME_OBJ("c2GJK-outA", coa, roa, n);
                SAME_OBJ("c2GJK-outB", cob, rob, n);
                SAME_OBJ("c2GJK-iterations", ci, ri, n);
                SAME_OBJ("c2GJK-cache", cc, rc, n);

                cd = c.c2GJK(sa, ta, NULL, sb, tb, NULL,
                             &coa, &cob, 0, &ci, &cc);
                rd = r.c2GJK(sa, ta, NULL, sb, tb, NULL,
                             &roa, &rob, 0, &ri, &rc);
                SAME_OBJ("c2GJK-cached-distance", cd, rd, n);
                SAME_OBJ("c2GJK-cached-outA", coa, roa, n);
                SAME_OBJ("c2GJK-cached-outB", cob, rob, n);
                SAME_OBJ("c2GJK-cached-iterations", ci, ri, n);
                SAME_OBJ("c2GJK-cached-cache", cc, rc, n);

                int co = c.omni_collide(
                    ta, circle.p.x, circle.p.y, capsule.b.x,
                    capsule.b.y, capsule.r, tb, circle2.p.x,
                    circle2.p.y, capsule2.b.x, capsule2.b.y, capsule2.r);
                int ro = r.omni_collide(
                    ta, circle.p.x, circle.p.y, capsule.b.x,
                    capsule.b.y, capsule.r, tb, circle2.p.x,
                    circle2.p.y, capsule2.b.x, capsule2.b.y, capsule2.r);
                SAME_OBJ("omni_collide", co, ro, n);
            }
        }

        for (int type = 0; type < 3; ++type) {
            void *cp = c.ptr_from_parts(type, x, y, a.x, a.y, scalar);
            void *rp = r.ptr_from_parts(type, x, y, a.x, a.y, scalar);
            if (!cp || !rp) fail("ptr_from_parts-null", n);
            if (memcmp(cp, rp, shape_size(type)) != 0) {
                fail("ptr_from_parts", n);
            }
            free(cp);
            free(rp);
        }
    }

    dlclose(c.handle);
    dlclose(r.handle);
    puts("all ABI differential checks passed (20000 iterations)");
    return 0;
}
