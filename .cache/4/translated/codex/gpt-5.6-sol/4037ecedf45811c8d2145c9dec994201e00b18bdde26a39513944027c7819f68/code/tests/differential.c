#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    float x;
    float y;
} c2v;

typedef struct {
    c2v p;
    float r;
} c2Circle;

typedef struct {
    c2v min;
    c2v max;
} c2AABB;

typedef struct {
    c2v a;
    c2v b;
    float r;
} c2Capsule;

typedef c2v (*c2V_fn)(float, float);
typedef c2v (*c2Mulvs_fn)(c2v, float);
typedef c2v (*c2Pair_fn)(c2v, c2v);
typedef c2v (*c2Clampv_fn)(c2v, c2v, c2v);
typedef float (*c2Dot_fn)(c2v, c2v);
typedef int (*circle_circle_fn)(c2Circle, c2Circle);
typedef int (*circle_aabb_fn)(c2Circle, c2AABB);
typedef int (*circle_capsule_fn)(c2Circle, c2Capsule);
typedef int (*collided_fn)(const void *, const void *, int);
typedef int (*circle_collide_fn)(float, float, float);

typedef struct {
    c2V_fn c2V;
    c2Mulvs_fn c2Mulvs;
    c2Pair_fn c2Maxv;
    c2Pair_fn c2Minv;
    c2Clampv_fn c2Clampv;
    c2Pair_fn c2Sub;
    c2Dot_fn c2Dot;
    circle_circle_fn c2CircletoCircle;
    circle_aabb_fn c2CircletoAABB;
    circle_capsule_fn c2CircletoCapsule;
    collided_fn c2Collided;
    circle_collide_fn circle_collide;
} api;

static uint32_t state = 0x6d2b79f5u;

static uint32_t random_u32(void) {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    return state;
}

static float random_float(void) {
    uint32_t bits = random_u32();
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static c2v random_v(void) {
    return (c2v){random_float(), random_float()};
}

static c2Circle random_circle(void) {
    return (c2Circle){random_v(), random_float()};
}

static c2AABB random_aabb(void) {
    return (c2AABB){random_v(), random_v()};
}

static c2Capsule random_capsule(void) {
    return (c2Capsule){random_v(), random_v(), random_float()};
}

static void load_symbol(void *handle, const char *name, void *destination) {
    void *symbol = dlsym(handle, name);
    if (!symbol) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    memcpy(destination, &symbol, sizeof(symbol));
}

static api load_api(const char *path) {
    void *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    api result;
    if (!handle) {
        fprintf(stderr, "cannot load %s: %s\n", path, dlerror());
        exit(2);
    }

#define LOAD(name) load_symbol(handle, #name, &result.name)
    LOAD(c2V);
    LOAD(c2Mulvs);
    LOAD(c2Maxv);
    LOAD(c2Minv);
    LOAD(c2Clampv);
    LOAD(c2Sub);
    LOAD(c2Dot);
    LOAD(c2CircletoCircle);
    LOAD(c2CircletoAABB);
    LOAD(c2CircletoCapsule);
    LOAD(c2Collided);
    LOAD(circle_collide);
#undef LOAD
    return result;
}

static void mismatch(const char *name, size_t iteration) {
    fprintf(stderr, "%s mismatch at iteration %zu\n", name, iteration);
    exit(1);
}

#define CHECK_VALUE(name, expression_a, expression_b)         \
    do {                                                       \
        __typeof__(expression_a) a_result = (expression_a);    \
        __typeof__(expression_b) b_result = (expression_b);    \
        if (memcmp(&a_result, &b_result, sizeof(a_result))) {   \
            mismatch((name), iteration);                       \
        }                                                      \
    } while (0)

int main(int argc, char **argv) {
    static const uint32_t edge_bits[] = {
        0x00000000u, 0x80000000u, 0x00000001u, 0x007fffffu,
        0x00800000u, 0x3f800000u, 0xbf800000u, 0x7f7fffffu,
        0xff7fffffu, 0x7f800000u, 0xff800000u, 0x7fc00000u,
        0x7fa00001u, 0xffc12345u,
    };
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_SO RUST_SO\n", argv[0]);
        return 2;
    }

    api c = load_api(argv[1]);
    api rust = load_api(argv[2]);

    for (size_t iteration = 0; iteration < 200000; ++iteration) {
        c2v a = random_v();
        c2v b = random_v();
        c2v lo = random_v();
        c2v hi = random_v();
        c2Circle circle_a = random_circle();
        c2Circle circle_b = random_circle();
        c2AABB aabb = random_aabb();
        c2Capsule capsule = random_capsule();
        float scalar = random_float();

        CHECK_VALUE("c2V", c.c2V(a.x, a.y), rust.c2V(a.x, a.y));
        CHECK_VALUE("c2Mulvs", c.c2Mulvs(a, scalar), rust.c2Mulvs(a, scalar));
        CHECK_VALUE("c2Maxv", c.c2Maxv(a, b), rust.c2Maxv(a, b));
        CHECK_VALUE("c2Minv", c.c2Minv(a, b), rust.c2Minv(a, b));
        CHECK_VALUE("c2Clampv", c.c2Clampv(a, lo, hi), rust.c2Clampv(a, lo, hi));
        CHECK_VALUE("c2Sub", c.c2Sub(a, b), rust.c2Sub(a, b));
        CHECK_VALUE("c2Dot", c.c2Dot(a, b), rust.c2Dot(a, b));
        CHECK_VALUE("c2CircletoCircle",
                    c.c2CircletoCircle(circle_a, circle_b),
                    rust.c2CircletoCircle(circle_a, circle_b));
        CHECK_VALUE("c2CircletoAABB",
                    c.c2CircletoAABB(circle_a, aabb),
                    rust.c2CircletoAABB(circle_a, aabb));
        CHECK_VALUE("c2CircletoCapsule",
                    c.c2CircletoCapsule(circle_a, capsule),
                    rust.c2CircletoCapsule(circle_a, capsule));
        CHECK_VALUE("c2Collided(circle)",
                    c.c2Collided(&circle_a, &circle_b, 0),
                    rust.c2Collided(&circle_a, &circle_b, 0));
        CHECK_VALUE("c2Collided(aabb)",
                    c.c2Collided(&circle_a, &aabb, 1),
                    rust.c2Collided(&circle_a, &aabb, 1));
        CHECK_VALUE("c2Collided(capsule)",
                    c.c2Collided(&circle_a, &capsule, 2),
                    rust.c2Collided(&circle_a, &capsule, 2));
        CHECK_VALUE("circle_collide",
                    c.circle_collide(a.x, a.y, scalar),
                    rust.circle_collide(a.x, a.y, scalar));
    }

    for (size_t iteration = 0;
         iteration < sizeof(edge_bits) / sizeof(edge_bits[0]);
         ++iteration) {
        float value;
        memcpy(&value, &edge_bits[iteration], sizeof(value));
        CHECK_VALUE("c2V(edge)", c.c2V(value, value), rust.c2V(value, value));
        CHECK_VALUE("c2Mulvs(edge)",
                    c.c2Mulvs((c2v){value, value}, value),
                    rust.c2Mulvs((c2v){value, value}, value));
        CHECK_VALUE("circle_collide(edge)",
                    c.circle_collide(value, value, value),
                    rust.circle_collide(value, value, value));
    }

    for (size_t iteration = 0; iteration < 100; ++iteration) {
        int type = iteration + 3;
        CHECK_VALUE("c2Collided(default)",
                    c.c2Collided(NULL, NULL, type),
                    rust.c2Collided(NULL, NULL, type));
    }

    puts("all 12 exports matched");
    return 0;
}
