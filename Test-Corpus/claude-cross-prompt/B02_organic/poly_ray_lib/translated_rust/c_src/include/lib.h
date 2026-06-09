typedef struct c2v {
        float x;
        float y;
} c2v;

typedef struct c2Raycast {
        float t;
        c2v n;
} c2Raycast;

int poly_ray(c2Raycast *cast1, c2Raycast *cast2);
