#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    float x;
    float y;
} V;

typedef struct {
    V p;
    float r;
} Circle;

typedef struct {
    V min;
    V max;
} Aabb;

typedef V (*MakeVFn)(float, float);
typedef V (*BinaryVFn)(V, V);
typedef V (*ClampVFn)(V, V, V);
typedef float (*DotFn)(V, V);
typedef int (*CircleCircleFn)(Circle, Circle);
typedef int (*CircleAabbFn)(Circle, Aabb);
typedef int (*AabbAabbFn)(Aabb, Aabb);
typedef int (*CollidedFn)(const void *, int, const void *, int);

typedef struct {
    MakeVFn make_v;
    BinaryVFn max_v;
    BinaryVFn min_v;
    ClampVFn clamp_v;
    BinaryVFn sub;
    DotFn dot;
    CircleCircleFn circle_circle;
    CircleAabbFn circle_aabb;
    AabbAabbFn aabb_aabb;
    CollidedFn collided;
} Api;

static uint64_t rng_state = UINT64_C(0x4d595df4d0f33173);

static uint32_t next_u32(void) {
    rng_state ^= rng_state >> 12;
    rng_state ^= rng_state << 25;
    rng_state ^= rng_state >> 27;
    return (uint32_t)((rng_state * UINT64_C(2685821657736338717)) >> 32);
}

static float next_float(void) {
    uint32_t bits = next_u32();
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static V next_v(void) {
    V value = {next_float(), next_float()};
    return value;
}

static Circle next_circle(void) {
    Circle value = {next_v(), next_float()};
    return value;
}

static Aabb next_aabb(void) {
    Aabb value = {next_v(), next_v()};
    return value;
}

static uint32_t float_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void *load_symbol(void *handle, const char *name) {
    dlerror();
    void *symbol = dlsym(handle, name);
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", name, error);
        exit(2);
    }
    return symbol;
}

#define LOAD(api, handle, field, type, name) \
    do {                                      \
        void *symbol = load_symbol(handle, name); \
        memcpy(&(api).field, &symbol, sizeof(type)); \
    } while (0)

static Api load_api(void *handle) {
    Api api;
    LOAD(api, handle, make_v, MakeVFn, "c2V");
    LOAD(api, handle, max_v, BinaryVFn, "c2Maxv");
    LOAD(api, handle, min_v, BinaryVFn, "c2Minv");
    LOAD(api, handle, clamp_v, ClampVFn, "c2Clampv");
    LOAD(api, handle, sub, BinaryVFn, "c2Sub");
    LOAD(api, handle, dot, DotFn, "c2Dot");
    LOAD(api, handle, circle_circle, CircleCircleFn, "c2CircletoCircle");
    LOAD(api, handle, circle_aabb, CircleAabbFn, "c2CircletoAABB");
    LOAD(api, handle, aabb_aabb, AabbAabbFn, "c2AABBtoAABB");
    LOAD(api, handle, collided, CollidedFn, "collided");
    return api;
}

static void fail(const char *function, size_t iteration) {
    fprintf(stderr, "%s mismatch at iteration %zu\n", function, iteration);
    exit(1);
}

#define CHECK_BYTES(function, iteration, c_value, rust_value) \
    do {                                                       \
        if (memcmp(&(c_value), &(rust_value), sizeof(c_value)) != 0) { \
            fail(function, iteration);                         \
        }                                                      \
    } while (0)

#define CHECK_INT(function, iteration, c_value, rust_value) \
    do {                                                    \
        if ((c_value) != (rust_value)) {                    \
            fail(function, iteration);                      \
        }                                                   \
    } while (0)

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (c_handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", argv[1], dlerror());
        return 2;
    }
    void *rust_handle = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (rust_handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", argv[2], dlerror());
        return 2;
    }

    Api c = load_api(c_handle);
    Api rust = load_api(rust_handle);

    for (size_t i = 0; i < 500000; ++i) {
        V a = next_v();
        V b = next_v();
        V lo = next_v();
        V hi = next_v();
        Circle circle_a = next_circle();
        Circle circle_b = next_circle();
        Aabb aabb_a = next_aabb();
        Aabb aabb_b = next_aabb();

        V c_v = c.make_v(a.x, a.y);
        V rust_v = rust.make_v(a.x, a.y);
        CHECK_BYTES("c2V", i, c_v, rust_v);

        c_v = c.max_v(a, b);
        rust_v = rust.max_v(a, b);
        CHECK_BYTES("c2Maxv", i, c_v, rust_v);

        c_v = c.min_v(a, b);
        rust_v = rust.min_v(a, b);
        CHECK_BYTES("c2Minv", i, c_v, rust_v);

        c_v = c.clamp_v(a, lo, hi);
        rust_v = rust.clamp_v(a, lo, hi);
        CHECK_BYTES("c2Clampv", i, c_v, rust_v);

        c_v = c.sub(a, b);
        rust_v = rust.sub(a, b);
        CHECK_BYTES("c2Sub", i, c_v, rust_v);

        float c_float = c.dot(a, b);
        float rust_float = rust.dot(a, b);
        if (float_bits(c_float) != float_bits(rust_float)) {
            fail("c2Dot", i);
        }

        int c_int = c.circle_circle(circle_a, circle_b);
        int rust_int = rust.circle_circle(circle_a, circle_b);
        CHECK_INT("c2CircletoCircle", i, c_int, rust_int);

        c_int = c.circle_aabb(circle_a, aabb_a);
        rust_int = rust.circle_aabb(circle_a, aabb_a);
        CHECK_INT("c2CircletoAABB", i, c_int, rust_int);

        c_int = c.aabb_aabb(aabb_a, aabb_b);
        rust_int = rust.aabb_aabb(aabb_a, aabb_b);
        CHECK_INT("c2AABBtoAABB", i, c_int, rust_int);

        c_int = c.collided(&circle_a, 0, &circle_b, 0);
        rust_int = rust.collided(&circle_a, 0, &circle_b, 0);
        CHECK_INT("collided(circle,circle)", i, c_int, rust_int);

        c_int = c.collided(&circle_a, 0, &aabb_a, 1);
        rust_int = rust.collided(&circle_a, 0, &aabb_a, 1);
        CHECK_INT("collided(circle,aabb)", i, c_int, rust_int);

        c_int = c.collided(&aabb_a, 1, &circle_a, 0);
        rust_int = rust.collided(&aabb_a, 1, &circle_a, 0);
        CHECK_INT("collided(aabb,circle)", i, c_int, rust_int);

        c_int = c.collided(&aabb_a, 1, &aabb_b, 1);
        rust_int = rust.collided(&aabb_a, 1, &aabb_b, 1);
        CHECK_INT("collided(aabb,aabb)", i, c_int, rust_int);

        c_int = c.collided(NULL, -1, NULL, -1);
        rust_int = rust.collided(NULL, -1, NULL, -1);
        CHECK_INT("collided(unknown,unknown)", i, c_int, rust_int);

        c_int = c.collided(&circle_a, 0, NULL, 99);
        rust_int = rust.collided(&circle_a, 0, NULL, 99);
        CHECK_INT("collided(circle,unknown)", i, c_int, rust_int);
    }

    dlclose(rust_handle);
    dlclose(c_handle);
    puts("all 10 ABI functions matched for 500000 randomized inputs");
    return 0;
}
