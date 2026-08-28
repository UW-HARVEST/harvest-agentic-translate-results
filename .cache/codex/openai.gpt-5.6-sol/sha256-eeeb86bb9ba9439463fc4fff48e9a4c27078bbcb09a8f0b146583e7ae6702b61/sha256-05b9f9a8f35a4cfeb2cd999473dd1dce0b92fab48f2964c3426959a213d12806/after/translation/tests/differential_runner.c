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
    int count;
    float depths[2];
    c2v contact_points[2];
    c2v n;
} c2Manifold;

typedef void (*omni_fn)(c2Manifold *, int,
                        float, float, float, float, float,
                        int, float, float, float, float, float);

static uint32_t state = 0x739a4f21u;

static uint32_t next_u32(void) {
    uint32_t x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    state = x;
    return x;
}

static float random_float(float low, float high) {
    float unit = (float)(next_u32() & 0x00ffffffu) / 16777215.0f;
    return low + unit * (high - low);
}

static void make_shape(int type, float out[5]) {
    float x = random_float(-20.0f, 20.0f);
    float y = random_float(-20.0f, 20.0f);
    switch (type) {
    case 0:
        out[0] = x;
        out[1] = y;
        out[2] = x + random_float(0.25f, 8.0f);
        out[3] = y + random_float(-8.0f, 8.0f);
        out[4] = random_float(0.05f, 5.0f);
        break;
    case 1:
        out[0] = x;
        out[1] = y;
        out[2] = random_float(0.05f, 5.0f);
        out[3] = random_float(-10.0f, 10.0f);
        out[4] = random_float(-10.0f, 10.0f);
        break;
    case 2:
        out[0] = x;
        out[1] = y;
        out[2] = x + random_float(0.05f, 8.0f);
        out[3] = y + random_float(0.05f, 8.0f);
        out[4] = random_float(-10.0f, 10.0f);
        break;
    default:
        abort();
    }
}

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
    omni_fn omni = (omni_fn)dlsym(handle, "omni_manifold");
    if (!omni) {
        fprintf(stderr, "dlsym: %s\n", dlerror());
        return 2;
    }
    FILE *output = fopen(argv[2], "wb");
    if (!output) {
        perror("fopen");
        return 2;
    }

    for (int type_a = 0; type_a < 3; ++type_a) {
        for (int type_b = 0; type_b < 3; ++type_b) {
            for (int i = 0; i < 5000; ++i) {
                float a[5], b[5];
                c2Manifold manifold;
                make_shape(type_a, a);
                make_shape(type_b, b);
                memset(&manifold, 0xa5, sizeof(manifold));
                omni(&manifold,
                     type_a, a[0], a[1], a[2], a[3], a[4],
                     type_b, b[0], b[1], b[2], b[3], b[4]);
                if (fwrite(&manifold, sizeof(manifold), 1, output) != 1) {
                    perror("fwrite");
                    return 2;
                }
            }
        }
    }

    if (fclose(output) != 0) {
        perror("fclose");
        return 2;
    }
    dlclose(handle);
    return 0;
}
