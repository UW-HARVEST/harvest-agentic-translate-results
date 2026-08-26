typedef enum {
        C2_TYPE_CAPSULE,
        C2_TYPE_CIRCLE,
        C2_TYPE_AABB,
	C2_TYPE_POLY,
} C2_TYPE;

typedef struct c2v {
        float x;
        float y;
} c2v;

typedef struct c2Manifold {
    int count;
    float depths[2];
    c2v contact_points[2];
    c2v n;
} c2Manifold;

void omni_manifold(c2Manifold *m,
		C2_TYPE type_a, float a1, float a2, float a3, float a4, float a5,
		C2_TYPE type_b, float b1, float b2, float b3, float b4, float b5);
