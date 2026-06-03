#include "q_shared.h"
#include <stdio.h>


int main(int argc, char** argv) {
    vec3_t Inputs;
    if(argc != 4) {
        fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
        exit(1);
    }

    Inputs[0] = atof(argv[1]);
    Inputs[1] = atof(argv[2]);
    Inputs[2] = atof(argv[3]);

    VectorNormalizeFast(Inputs);

    printf("%f %f %f\n", Inputs[0], Inputs[1], Inputs[2]);
    return 0;
}
