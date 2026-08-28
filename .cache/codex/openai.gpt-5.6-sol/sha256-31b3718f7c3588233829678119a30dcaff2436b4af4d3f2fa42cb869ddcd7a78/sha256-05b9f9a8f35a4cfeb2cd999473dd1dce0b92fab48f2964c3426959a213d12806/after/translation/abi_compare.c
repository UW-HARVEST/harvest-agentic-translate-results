#include <dlfcn.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { float x, y; } c2v;
typedef struct { float t; c2v n; } c2Raycast;
typedef struct { c2v p; float r; } c2Circle;
typedef struct { c2v min, max; } c2AABB;
typedef struct { c2v a, b; float r; } c2Capsule;
typedef struct { c2v p, d; float t; } c2Ray;
typedef struct { c2v x, y; } c2m;

typedef c2v (*fn_v_ff)(float, float);
typedef float (*fn_f_v)(c2v);
typedef float (*fn_f_vv)(c2v, c2v);
typedef c2v (*fn_v_v)(c2v);
typedef c2v (*fn_v_vv)(c2v, c2v);
typedef c2v (*fn_v_vf)(c2v, float);
typedef c2v (*fn_v_mv)(c2m, c2v);
typedef int (*fn_i_aa)(c2AABB, c2AABB);
typedef int (*fn_i_av)(c2AABB, c2v);
typedef int (*fn_i_cv)(c2Circle, c2v);
typedef int (*fn_i_rc)(c2Ray, c2Circle, c2Raycast *);
typedef int (*fn_i_ra)(c2Ray, c2AABB, c2Raycast *);
typedef int (*fn_i_rk)(c2Ray, c2Capsule, c2Raycast *);
typedef int (*fn_i_cast)(c2Ray, const void *, int, c2Raycast *);
typedef int (*fn_spec)(c2Raycast *, float, float, float, float, float, float, float);

static void *symbol(void *handle, const char *name)
{
    void *value = dlsym(handle, name);
    if (!value) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    return value;
}

#define LOAD(type, name) \
    type c_##name = (type)symbol(c_lib, #name); \
    type r_##name = (type)symbol(r_lib, #name)

static uint32_t rng_state = 0x91e10da5u;

static uint32_t random_u32(void)
{
    uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    rng_state = x;
    return x;
}

static float random_float(void)
{
    return ((int32_t)(random_u32() % 200001u) - 100000) / 1024.0f;
}

static c2v random_v(void)
{
    c2v value = { random_float(), random_float() };
    return value;
}

static float from_bits(uint32_t bits)
{
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void fail(const char *name, size_t iteration)
{
    fprintf(stderr, "mismatch: %s at case %zu\n", name, iteration);
    exit(1);
}

#define SAME_VALUE(name, left, right, iteration) do { \
    __typeof__(left) lhs_ = (left); \
    __typeof__(right) rhs_ = (right); \
    if (memcmp(&lhs_, &rhs_, sizeof(lhs_)) != 0) fail((name), (iteration)); \
} while (0)

#define SAME_CALL(name, args, iteration) \
    SAME_VALUE(#name, c_##name args, r_##name args, iteration)

#define SAME_RAY_CALL(name, ray, shape, iteration) do { \
    c2Raycast c_out_ = { from_bits(0x7fc12345u), \
        { from_bits(0x80000000u), from_bits(0x7f812345u) } }; \
    c2Raycast r_out_ = c_out_; \
    int c_result_ = c_##name((ray), (shape), &c_out_); \
    int r_result_ = r_##name((ray), (shape), &r_out_); \
    if (c_result_ != r_result_ || memcmp(&c_out_, &r_out_, sizeof(c_out_)) != 0) \
        fail(#name, (iteration)); \
} while (0)

int main(int argc, char **argv)
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }
    void *c_lib = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *r_lib = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (!c_lib || !r_lib) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    LOAD(fn_v_ff, c2V);
    LOAD(fn_f_vv, c2Dot);
    LOAD(fn_f_v, c2Len);
    LOAD(fn_v_vv, c2Add);
    LOAD(fn_v_vv, c2Sub);
    LOAD(fn_v_vf, c2Mulvs);
    LOAD(fn_v_vf, c2Div);
    LOAD(fn_v_v, c2Norm);
    LOAD(fn_v_vv, c2Minv);
    LOAD(fn_v_vv, c2Maxv);
    LOAD(fn_v_v, c2Skew);
    LOAD(fn_v_v, c2Absv);
    LOAD(fn_i_rc, c2RaytoCircle);
    LOAD(fn_i_aa, c2AABBtoAABB);
    LOAD(fn_i_ra, c2RaytoAABB);
    LOAD(fn_v_v, c2CCW90);
    LOAD(fn_v_mv, c2MulmvT);
    LOAD(fn_i_av, c2AABBtoPoint);
    LOAD(fn_i_cv, c2CircleToPoint);
    LOAD(fn_i_rk, c2RaytoCapsule);
    LOAD(fn_i_cast, c2CastRay);
    LOAD(fn_spec, spec_ray);

    const uint32_t edge_bits[] = {
        0x00000000u, 0x80000000u, 0x3f800000u, 0xbf800000u,
        0x00000001u, 0x80000001u, 0x7f7fffffu, 0xff7fffffu,
        0x7f800000u, 0xff800000u, 0x7fc12345u
    };
    size_t edge_case = 0;
    for (size_t i = 0; i < sizeof(edge_bits) / sizeof(edge_bits[0]); ++i) {
        for (size_t j = 0; j < sizeof(edge_bits) / sizeof(edge_bits[0]); ++j) {
            c2v a = { from_bits(edge_bits[i]), from_bits(edge_bits[j]) };
            c2v b = { from_bits(edge_bits[j]), from_bits(edge_bits[i]) };
            float scalar = from_bits(edge_bits[j]);
            SAME_CALL(c2V, (a.x, a.y), edge_case);
            SAME_CALL(c2Dot, (a, b), edge_case);
            SAME_CALL(c2Len, (a), edge_case);
            SAME_CALL(c2Add, (a, b), edge_case);
            SAME_CALL(c2Sub, (a, b), edge_case);
            SAME_CALL(c2Mulvs, (a, scalar), edge_case);
            SAME_CALL(c2Div, (a, scalar), edge_case);
            SAME_CALL(c2Norm, (a), edge_case);
            SAME_CALL(c2Minv, (a, b), edge_case);
            SAME_CALL(c2Maxv, (a, b), edge_case);
            SAME_CALL(c2Skew, (a), edge_case);
            SAME_CALL(c2Absv, (a), edge_case);
            SAME_CALL(c2CCW90, (a), edge_case);
            ++edge_case;
        }
    }

    const size_t random_cases = 200000;
    for (size_t i = 0; i < random_cases; ++i) {
        c2v a = random_v();
        c2v b = random_v();
        float scalar = random_float();
        if (scalar == 0.0f) scalar = 1.0f;
        c2m matrix = { random_v(), random_v() };
        SAME_CALL(c2V, (a.x, a.y), i);
        SAME_CALL(c2Dot, (a, b), i);
        SAME_CALL(c2Len, (a), i);
        SAME_CALL(c2Add, (a, b), i);
        SAME_CALL(c2Sub, (a, b), i);
        SAME_CALL(c2Mulvs, (a, scalar), i);
        SAME_CALL(c2Div, (a, scalar), i);
        SAME_CALL(c2Norm, (a), i);
        SAME_CALL(c2Minv, (a, b), i);
        SAME_CALL(c2Maxv, (a, b), i);
        SAME_CALL(c2Skew, (a), i);
        SAME_CALL(c2Absv, (a), i);
        SAME_CALL(c2CCW90, (a), i);
        SAME_CALL(c2MulmvT, (matrix, b), i);

        c2AABB box_a = {
            { fminf(a.x, b.x), fminf(a.y, b.y) },
            { fmaxf(a.x, b.x), fmaxf(a.y, b.y) }
        };
        c2v c = random_v();
        c2v d = random_v();
        c2AABB box_b = {
            { fminf(c.x, d.x), fminf(c.y, d.y) },
            { fmaxf(c.x, d.x), fmaxf(c.y, d.y) }
        };
        c2Circle circle = { c, fabsf(random_float()) };
        SAME_CALL(c2AABBtoAABB, (box_a, box_b), i);
        SAME_CALL(c2AABBtoPoint, (box_a, d), i);
        SAME_CALL(c2CircleToPoint, (circle, d), i);

        c2Ray ray = { a, b, fabsf(random_float()) };
        c2Capsule capsule = { c, d, fabsf(random_float()) };
        if (capsule.a.x == capsule.b.x && capsule.a.y == capsule.b.y)
            capsule.b.x += 1.0f;
        SAME_RAY_CALL(c2RaytoCircle, ray, circle, i);
        SAME_RAY_CALL(c2RaytoAABB, ray, box_b, i);
        SAME_RAY_CALL(c2RaytoCapsule, ray, capsule, i);

        c2Raycast c_out = { from_bits(0x7fc12345u), { -0.0f, 1.0f } };
        c2Raycast r_out = c_out;
        int c_result = c_c2CastRay(ray, &circle, 0, &c_out);
        int r_result = r_c2CastRay(ray, &circle, 0, &r_out);
        if (c_result != r_result || memcmp(&c_out, &r_out, sizeof(c_out)) != 0)
            fail("c2CastRay(circle)", i);
        c_out = (c2Raycast){ from_bits(0x7fc12345u), { -0.0f, 1.0f } };
        r_out = c_out;
        c_result = c_c2CastRay(ray, &box_b, 1, &c_out);
        r_result = r_c2CastRay(ray, &box_b, 1, &r_out);
        if (c_result != r_result || memcmp(&c_out, &r_out, sizeof(c_out)) != 0)
            fail("c2CastRay(aabb)", i);
        c_out = (c2Raycast){ from_bits(0x7fc12345u), { -0.0f, 1.0f } };
        r_out = c_out;
        c_result = c_c2CastRay(ray, &capsule, 2, &c_out);
        r_result = r_c2CastRay(ray, &capsule, 2, &r_out);
        if (c_result != r_result || memcmp(&c_out, &r_out, sizeof(c_out)) != 0)
            fail("c2CastRay(capsule)", i);

        c_out = (c2Raycast){ from_bits(0x7fc12345u), { -0.0f, 1.0f } };
        r_out = c_out;
        c_result = c_spec_ray(&c_out, a.x, a.y, c.x, c.y, circle.r, d.x, d.y);
        r_result = r_spec_ray(&r_out, a.x, a.y, c.x, c.y, circle.r, d.x, d.y);
        if (c_result != r_result || memcmp(&c_out, &r_out, sizeof(c_out)) != 0)
            fail("spec_ray", i);
    }

    printf("ABI behavior matches: 22 symbols, %zu edge vector cases, "
           "%zu randomized full-library cases\n", edge_case, random_cases);
    dlclose(r_lib);
    dlclose(c_lib);
    return 0;
}
