typedef enum {
        C2_TYPE_CAPSULE,
        C2_TYPE_CIRCLE,
        C2_TYPE_AABB,
} C2_TYPE;

int omni_collide(C2_TYPE type_a, float a1, float a2, float a3, float a4, float a5,
		C2_TYPE type_b, float b1, float b2, float b3, float b4, float b5);
