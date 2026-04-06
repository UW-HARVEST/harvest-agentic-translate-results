/* Helper: calls C omni_manifold and prints result as space-separated floats.
   Usage: c_helper type_a a1 a2 a3 a4 a5 type_b b1 b2 b3 b4 b5
   Output: count depths0 depths1 cpx0 cpy0 cpx1 cpy1 nx ny
   Floats printed with enough precision for exact round-trip. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef enum { C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY } C2_TYPE;
typedef struct { float x, y; } c2v;
typedef struct { int count; float depths[2]; c2v contact_points[2]; c2v n; } c2Manifold;

extern void omni_manifold(c2Manifold*, C2_TYPE, float, float, float, float, float,
                          C2_TYPE, float, float, float, float, float);

static void print_float_hex(float f) {
    /* Print as hex to ensure exact round-trip */
    unsigned char *p = (unsigned char *)&f;
    printf("%02x%02x%02x%02x", p[0], p[1], p[2], p[3]);
}

int main(int argc, char **argv) {
    if (argc != 13) { fprintf(stderr, "need 12 args\n"); return 1; }
    int type_a = atoi(argv[1]);
    float a1 = strtof(argv[2], NULL), a2 = strtof(argv[3], NULL),
          a3 = strtof(argv[4], NULL), a4 = strtof(argv[5], NULL), a5 = strtof(argv[6], NULL);
    int type_b = atoi(argv[7]);
    float b1 = strtof(argv[8], NULL), b2 = strtof(argv[9], NULL),
          b3 = strtof(argv[10], NULL), b4 = strtof(argv[11], NULL), b5 = strtof(argv[12], NULL);

    c2Manifold m;
    memset(&m, 0, sizeof(m));
    omni_manifold(&m, (C2_TYPE)type_a, a1, a2, a3, a4, a5, (C2_TYPE)type_b, b1, b2, b3, b4, b5);

    /* Print as hex bytes for exact comparison */
    unsigned char *bytes = (unsigned char *)&m;
    for (int i = 0; i < (int)sizeof(m); i++) {
        if (i > 0) printf(" ");
        printf("%d", bytes[i]);
    }
    printf("\n");
    return 0;
}
