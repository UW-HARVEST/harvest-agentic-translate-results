#include "castom_math.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: driver <fn> [args...]\n");
        return 1;
    }
    const char *fn = argv[1];
    if (strcmp(fn, "abs") == 0) {
        printf("%d\n", castom_abs(atoi(argv[2])));
    } else if (strcmp(fn, "fabs") == 0) {
        printf("%.20Lg\n", castom_fabs(atof(argv[2])));
    } else if (strcmp(fn, "floor") == 0) {
        printf("%.20Lg\n", castom_floor(atof(argv[2])));
    } else if (strcmp(fn, "ceil") == 0) {
        printf("%.20Lg\n", castom_ceil(atof(argv[2])));
    } else if (strcmp(fn, "trunc") == 0) {
        printf("%.20Lg\n", castom_trunc(atof(argv[2])));
    } else if (strcmp(fn, "sqrt") == 0) {
        printf("%.20Lg\n", castom_sqrt(atof(argv[2])));
    } else if (strcmp(fn, "exp") == 0) {
        printf("%.20Lg\n", castom_exp(atof(argv[2])));
    } else if (strcmp(fn, "log") == 0) {
        printf("%.20Lg\n", castom_log(atof(argv[2])));
    } else if (strcmp(fn, "sin") == 0) {
        printf("%.20Lg\n", castom_sin(atof(argv[2])));
    } else if (strcmp(fn, "cos") == 0) {
        printf("%.20Lg\n", castom_cos(atof(argv[2])));
    } else if (strcmp(fn, "tan") == 0) {
        printf("%.20Lg\n", castom_tan(atof(argv[2])));
    } else if (strcmp(fn, "asin") == 0) {
        printf("%.20Lg\n", castom_asin(atof(argv[2])));
    } else if (strcmp(fn, "acos") == 0) {
        printf("%.20Lg\n", castom_acos(atof(argv[2])));
    } else if (strcmp(fn, "atan") == 0) {
        printf("%.20Lg\n", castom_atan(atof(argv[2])));
    } else if (strcmp(fn, "fmod") == 0) {
        printf("%.20Lg\n", castom_fmod(atof(argv[2]), atof(argv[3])));
    } else if (strcmp(fn, "pow") == 0) {
        printf("%.20Lg\n", castom_pow(atof(argv[2]), atof(argv[3])));
    } else {
        fprintf(stderr, "unknown function: %s\n", fn);
        return 1;
    }
    return 0;
}
