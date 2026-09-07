typedef struct c2v {
        float x;
        float y;
} c2v;

typedef struct c2Raycast {
        float t;
        c2v n;
} c2Raycast;

int spec_ray(c2Raycast *cast, float mp_x, float mp_y, float c_p_x, float c_p_y, float c_r,
                float r_p_x, float r_p_y);
