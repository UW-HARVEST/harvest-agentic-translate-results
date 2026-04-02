typedef struct c2v {
        float x;
        float y;
} c2v;

typedef struct c2Raycast {
        float t;
        c2v n;
} c2Raycast;

int gen_ray(c2Raycast *cast1, c2Raycast *cast2, c2Raycast *cast3,
                float mp_x, float mp_y, float r_p_x, float r_p_y,
                float c_p_x, float c_p_y, float c_r,
                float cap_a_x, float cap_a_y, float cap_b_x, float cap_b_y, float cap_r,
                float bb_min_x, float bb_min_y, float bb_max_x, float bb_max_y);
