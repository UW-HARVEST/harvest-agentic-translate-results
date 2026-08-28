#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned int C2_TYPE;
enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE };

typedef struct { float x, y; } c2v;
typedef struct { float c, s; } c2r;
typedef struct { c2v p; c2r r; } c2x;
typedef struct { c2v p; float r; } c2Circle;
typedef struct { c2v min, max; } c2AABB;
typedef struct { c2v a, b; float r; } c2Capsule;
typedef struct {
    float metric;
    int count;
    int iA[3];
    int iB[3];
    float div;
} c2GJKCache;
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

typedef c2v (*fn_v_ff)(float, float);
typedef c2v (*fn_v_vf)(c2v, float);
typedef c2v (*fn_v_vv)(c2v, c2v);
typedef c2v (*fn_v_vvv)(c2v, c2v, c2v);
typedef float (*fn_f_vv)(c2v, c2v);
typedef c2r (*fn_r_void)(void);
typedef c2x (*fn_x_void)(void);
typedef void (*fn_void_v_bb)(c2v *, c2AABB *);
typedef void (*fn_void_shape_type_proxy)(const void *, C2_TYPE, c2Proxy *);
typedef float (*fn_f_v)(c2v);
typedef float (*fn_f_simplex)(c2Simplex *);
typedef c2v (*fn_v_rv)(c2r, c2v);
typedef c2v (*fn_v_xv)(c2x, c2v);
typedef void (*fn_void_simplex)(c2Simplex *);
typedef c2v (*fn_v_v)(c2v);
typedef c2v (*fn_v_simplex)(c2Simplex *);
typedef int (*fn_i_support)(const c2v *, int, c2v);
typedef void (*fn_void_witness)(c2Simplex *, c2v *, c2v *);
typedef float (*fn_gjk)(const void *, C2_TYPE, const c2x *,
                        const void *, C2_TYPE, const c2x *,
                        c2v *, c2v *, int, int *, c2GJKCache *);
typedef int (*fn_i_aabb_aabb)(c2AABB, c2AABB);
typedef int (*fn_i_aabb_capsule)(c2AABB, c2Capsule);
typedef int (*fn_i_capsule_capsule)(c2Capsule, c2Capsule);
typedef int (*fn_i_circle_circle)(c2Circle, c2Circle);
typedef int (*fn_i_circle_aabb)(c2Circle, c2AABB);
typedef int (*fn_i_circle_capsule)(c2Circle, c2Capsule);
typedef int (*fn_i_collided)(const void *, C2_TYPE, const void *, C2_TYPE);
typedef int (*fn_i_ffff)(float, float, float, float);

static uint32_t random_state;
static unsigned long comparisons;

static uint32_t random_u32(void) {
    uint32_t x = random_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    random_state = x;
    return x;
}

static float random_float(void) {
    int32_t n = (int32_t)(random_u32() % 200001U) - 100000;
    return (float)n / 137.0f;
}

static float float_bits(uint32_t bits) {
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static c2v random_v(void) {
    c2v value = { random_float(), random_float() };
    return value;
}

static c2r random_rot(void) {
    c2r value = { random_float(), random_float() };
    return value;
}

static c2x random_x(void) {
    c2x value = { random_v(), random_rot() };
    return value;
}

static c2Circle random_circle(void) {
    c2Circle value = { random_v(), random_float() };
    return value;
}

static c2AABB random_aabb(void) {
    c2v a = random_v();
    c2v b = random_v();
    c2AABB value = {
        { a.x < b.x ? a.x : b.x, a.y < b.y ? a.y : b.y },
        { a.x > b.x ? a.x : b.x, a.y > b.y ? a.y : b.y }
    };
    return value;
}

static c2Capsule random_capsule(void) {
    c2Capsule value = { random_v(), random_v(), random_float() };
    return value;
}

static c2sv random_sv(void) {
    c2sv value = {
        random_v(), random_v(), random_v(), random_float(),
        (int)(random_u32() % 8), (int)(random_u32() % 8)
    };
    return value;
}

static c2Simplex random_simplex(void) {
    c2Simplex value = {
        random_sv(), random_sv(), random_sv(), random_sv(),
        random_float(), (int)(random_u32() % 5)
    };
    if (value.div == 0.0f) value.div = 1.0f;
    return value;
}

static void *load_symbol(void *library, const char *name) {
    void *symbol;
    dlerror();
    symbol = dlsym(library, name);
    if (dlerror() != NULL) {
        fprintf(stderr, "missing symbol: %s\n", name);
        exit(2);
    }
    return symbol;
}

static void print_bytes(const char *label, const void *value, size_t size) {
    const unsigned char *bytes = value;
    fprintf(stderr, "%s:", label);
    for (size_t j = 0; j < size; j++) fprintf(stderr, " %02x", bytes[j]);
    fputc('\n', stderr);
}

#define LOAD(name, type) \
    type c_##name = (type)load_symbol(c_library, #name); \
    type r_##name = (type)load_symbol(r_library, #name)

#define SAME(name, c_value, r_value) do { \
    comparisons++; \
    if (memcmp(&(c_value), &(r_value), sizeof(c_value)) != 0) { \
        fprintf(stderr, "mismatch in %s at iteration %d\n", name, i); \
        print_bytes("C", &(c_value), sizeof(c_value)); \
        print_bytes("Rust", &(r_value), sizeof(r_value)); \
        return 1; \
    } \
} while (0)

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }
    void *c_library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *r_library = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_library == NULL || r_library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    LOAD(c2V, fn_v_ff);
    LOAD(c2Mulvs, fn_v_vf);
    LOAD(c2Maxv, fn_v_vv);
    LOAD(c2Minv, fn_v_vv);
    LOAD(c2Clampv, fn_v_vvv);
    LOAD(c2Sub, fn_v_vv);
    LOAD(c2Dot, fn_f_vv);
    LOAD(c2RotIdentity, fn_r_void);
    LOAD(c2xIdentity, fn_x_void);
    LOAD(c2BBVerts, fn_void_v_bb);
    LOAD(c2MakeProxy, fn_void_shape_type_proxy);
    LOAD(c2Len, fn_f_v);
    LOAD(c2Det2, fn_f_vv);
    LOAD(c2GJKSimplexMetric, fn_f_simplex);
    LOAD(c2Mulrv, fn_v_rv);
    LOAD(c2Add, fn_v_vv);
    LOAD(c2Mulxv, fn_v_xv);
    LOAD(c22, fn_void_simplex);
    LOAD(c23, fn_void_simplex);
    LOAD(c2Neg, fn_v_v);
    LOAD(c2Skew, fn_v_v);
    LOAD(c2CCW90, fn_v_v);
    LOAD(c2D, fn_v_simplex);
    LOAD(c2Support, fn_i_support);
    LOAD(c2Witness, fn_void_witness);
    LOAD(c2Div, fn_v_vf);
    LOAD(c2Norm, fn_v_v);
    LOAD(c2L, fn_v_simplex);
    LOAD(c2MulrvT, fn_v_rv);
    LOAD(c2GJK, fn_gjk);
    LOAD(c2AABBtoAABB, fn_i_aabb_aabb);
    LOAD(c2AABBtoCapsule, fn_i_aabb_capsule);
    LOAD(c2CapsuletoCapsule, fn_i_capsule_capsule);
    LOAD(c2CircletoCircle, fn_i_circle_circle);
    LOAD(c2CircletoAABB, fn_i_circle_aabb);
    LOAD(c2CircletoCapsule, fn_i_circle_capsule);
    LOAD(c2Collided, fn_i_collided);
    LOAD(aabb, fn_i_ffff);

    int i = 0;
    c2r cr = c_c2RotIdentity(), rr = r_c2RotIdentity();
    SAME("c2RotIdentity", cr, rr);
    c2x cx = c_c2xIdentity(), rx = r_c2xIdentity();
    SAME("c2xIdentity", cx, rx);

    random_state = 0x12345678U;
    for (i = 0; i < 1000000; i++) {
        c2v a = {
            float_bits(random_u32()),
            float_bits(random_u32())
        };
        c2v b = {
            float_bits(random_u32()),
            float_bits(random_u32())
        };
        c2v d = {
            float_bits(random_u32()),
            float_bits(random_u32())
        };
        float f = float_bits(random_u32());
        c2r rotation = random_rot();
        c2x transform = random_x();

#define COMPARE_CALL(name, call_args) do { \
    __typeof__(c_##name call_args) cv = c_##name call_args; \
    __typeof__(r_##name call_args) rv = r_##name call_args; \
    SAME(#name, cv, rv); \
} while (0)
        COMPARE_CALL(c2V, (a.x, a.y));
        COMPARE_CALL(c2Mulvs, (a, f));
        COMPARE_CALL(c2Maxv, (a, b));
        COMPARE_CALL(c2Minv, (a, b));
        COMPARE_CALL(c2Clampv, (a, b, d));
        COMPARE_CALL(c2Sub, (a, b));
        COMPARE_CALL(c2Dot, (a, b));
        COMPARE_CALL(c2Len, (a));
        float cdet = c_c2Det2(a, b);
        float rdet = r_c2Det2(a, b);
        if (memcmp(&cdet, &rdet, sizeof(cdet)) != 0) {
            print_bytes("a", &a, sizeof(a));
            print_bytes("b", &b, sizeof(b));
        }
        SAME("c2Det2", cdet, rdet);
        COMPARE_CALL(c2Mulrv, (rotation, a));
        COMPARE_CALL(c2Add, (a, b));
        COMPARE_CALL(c2Mulxv, (transform, a));
        COMPARE_CALL(c2Neg, (a));
        COMPARE_CALL(c2Skew, (a));
        COMPARE_CALL(c2CCW90, (a));
        COMPARE_CALL(c2Div, (a, f));
        COMPARE_CALL(c2Norm, (a));
        COMPARE_CALL(c2MulrvT, (rotation, a));
    }

    random_state = 0x9abcdef0U;
    for (i = 0; i < 5000; i++) {
        c2AABB cbb = random_aabb(), rbb = cbb;
        c2v cout[4], rout[4];
        c_c2BBVerts(cout, &cbb);
        r_c2BBVerts(rout, &rbb);
        SAME("c2BBVerts output", cout, rout);
        SAME("c2BBVerts input", cbb, rbb);

        for (C2_TYPE type = 0; type < 3; type++) {
            union {
                c2Circle circle;
                c2AABB aabb;
                c2Capsule capsule;
            } shape;
            memset(&shape, 0x5a, sizeof(shape));
            if (type == C2_TYPE_CIRCLE) shape.circle = random_circle();
            if (type == C2_TYPE_AABB) shape.aabb = random_aabb();
            if (type == C2_TYPE_CAPSULE) shape.capsule = random_capsule();
            c2Proxy cp, rp;
            memset(&cp, 0xa5, sizeof(cp));
            memset(&rp, 0xa5, sizeof(rp));
            c_c2MakeProxy(&shape, type, &cp);
            r_c2MakeProxy(&shape, type, &rp);
            SAME("c2MakeProxy", cp, rp);
        }

        c2Simplex cs = random_simplex(), rs = cs;
        float cm = c_c2GJKSimplexMetric(&cs);
        float rm = r_c2GJKSimplexMetric(&rs);
        SAME("c2GJKSimplexMetric", cm, rm);

        cs = random_simplex();
        rs = cs;
        cs.count = rs.count = 2;
        c_c22(&cs);
        r_c22(&rs);
        SAME("c22", cs, rs);

        cs = random_simplex();
        rs = cs;
        cs.count = rs.count = 3;
        c_c23(&cs);
        r_c23(&rs);
        SAME("c23", cs, rs);

        cs = random_simplex();
        rs = cs;
        c2v cv = c_c2D(&cs), rv = r_c2D(&rs);
        SAME("c2D", cv, rv);

        c2v verts[8];
        for (int j = 0; j < 8; j++) verts[j] = random_v();
        c2v direction = random_v();
        int ci = c_c2Support(verts, 8, direction);
        int ri = r_c2Support(verts, 8, direction);
        SAME("c2Support", ci, ri);

        cs = random_simplex();
        rs = cs;
        c2v ca, cb, ra, rb;
        c_c2Witness(&cs, &ca, &cb);
        r_c2Witness(&rs, &ra, &rb);
        SAME("c2Witness A", ca, ra);
        SAME("c2Witness B", cb, rb);

        cv = c_c2L(&cs);
        rv = r_c2L(&rs);
        SAME("c2L", cv, rv);
    }

    random_state = 0x31415926U;
    for (i = 0; i < 10000; i++) {
        c2Circle circle_a = random_circle();
        c2Circle circle_b = random_circle();
        c2AABB aabb_a = random_aabb();
        c2AABB aabb_b = random_aabb();
        c2Capsule capsule_a = random_capsule();
        c2Capsule capsule_b = random_capsule();

        int ci = c_c2AABBtoAABB(aabb_a, aabb_b);
        int ri = r_c2AABBtoAABB(aabb_a, aabb_b);
        SAME("c2AABBtoAABB", ci, ri);
        ci = c_c2AABBtoCapsule(aabb_a, capsule_a);
        ri = r_c2AABBtoCapsule(aabb_a, capsule_a);
        SAME("c2AABBtoCapsule", ci, ri);
        ci = c_c2CapsuletoCapsule(capsule_a, capsule_b);
        ri = r_c2CapsuletoCapsule(capsule_a, capsule_b);
        SAME("c2CapsuletoCapsule", ci, ri);
        ci = c_c2CircletoCircle(circle_a, circle_b);
        ri = r_c2CircletoCircle(circle_a, circle_b);
        SAME("c2CircletoCircle", ci, ri);
        ci = c_c2CircletoAABB(circle_a, aabb_a);
        ri = r_c2CircletoAABB(circle_a, aabb_a);
        SAME("c2CircletoAABB", ci, ri);
        ci = c_c2CircletoCapsule(circle_a, capsule_a);
        ri = r_c2CircletoCapsule(circle_a, capsule_a);
        SAME("c2CircletoCapsule", ci, ri);

        const void *shapes[3] = { &circle_a, &aabb_a, &capsule_a };
        const void *other[3] = { &circle_b, &aabb_b, &capsule_b };
        for (C2_TYPE ta = 0; ta < 3; ta++) {
            for (C2_TYPE tb = 0; tb < 3; tb++) {
                ci = c_c2Collided(shapes[ta], ta, other[tb], tb);
                ri = r_c2Collided(shapes[ta], ta, other[tb], tb);
                SAME("c2Collided", ci, ri);
            }
        }

        float min_x = random_float(), min_y = random_float();
        float max_x = random_float(), max_y = random_float();
        ci = c_aabb(min_x, min_y, max_x, max_y);
        ri = r_aabb(min_x, min_y, max_x, max_y);
        SAME("aabb", ci, ri);
    }

    random_state = 0x27182818U;
    for (i = 0; i < 10000; i++) {
        c2Circle circle = random_circle();
        c2AABB box = random_aabb();
        c2Capsule capsule = random_capsule();
        const void *shapes[3] = { &circle, &box, &capsule };
        C2_TYPE ta = random_u32() % 3;
        C2_TYPE tb = random_u32() % 3;
        c2x ax = random_x(), bx = random_x();
        c2v coa, cob, roa, rob;
        int cit = -1, rit = -1;
        c2GJKCache cc, rc;
        memset(&cc, 0, sizeof(cc));
        memset(&rc, 0, sizeof(rc));

        float cd = c_c2GJK(shapes[ta], ta, &ax, shapes[tb], tb, &bx,
                           &coa, &cob, i & 1, &cit, &cc);
        float rd = r_c2GJK(shapes[ta], ta, &ax, shapes[tb], tb, &bx,
                           &roa, &rob, i & 1, &rit, &rc);
        SAME("c2GJK distance", cd, rd);
        SAME("c2GJK outA", coa, roa);
        SAME("c2GJK outB", cob, rob);
        SAME("c2GJK iterations", cit, rit);
        SAME("c2GJK cache", cc, rc);

        cd = c_c2GJK(shapes[ta], ta, &ax, shapes[tb], tb, &bx,
                     &coa, &cob, 1, &cit, &cc);
        rd = r_c2GJK(shapes[ta], ta, &ax, shapes[tb], tb, &bx,
                     &roa, &rob, 1, &rit, &rc);
        SAME("c2GJK cached distance", cd, rd);
        SAME("c2GJK cached outA", coa, roa);
        SAME("c2GJK cached outB", cob, rob);
        SAME("c2GJK cached iterations", cit, rit);
        SAME("c2GJK cached cache", cc, rc);
    }

    printf("PASS: 38 symbols, %lu bitwise comparisons\n", comparisons);
    dlclose(r_library);
    dlclose(c_library);
    return 0;
}
