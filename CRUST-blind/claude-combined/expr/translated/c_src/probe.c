#include <math.h>
#include <stdio.h>
#include "mapper_expr.h"

void run_int(const char *str, int x) {
    mapper_expr e = mapper_expr_new_from_string(str, 0, 0, 1);
    if (!e) { printf("FAIL parse: %s\n", str); return; }
    int outp;
    mapper_expr_evaluate(e, &x, &outp);
    printf("INT [%s] x=%d -> %d\n", str, x, outp);
    mapper_expr_free(e);
}

void run_float(const char *str, float x) {
    mapper_expr e = mapper_expr_new_from_string(str, 1, 1, 1);
    if (!e) { printf("FAIL parse: %s\n", str); return; }
    float outp;
    mapper_expr_evaluate(e, &x, &outp);
    printf("FLOAT [%s] x=%f -> %f\n", str, x, outp);
    mapper_expr_free(e);
}

void run_int_float(const char *str, int x) {
    mapper_expr e = mapper_expr_new_from_string(str, 0, 1, 1);
    if (!e) { printf("FAIL parse: %s\n", str); return; }
    float outp;
    mapper_expr_evaluate(e, &x, &outp);
    printf("INT_FLOAT [%s] x=%d -> %f\n", str, x, outp);
    mapper_expr_free(e);
}

void run_float_int(const char *str, float x) {
    mapper_expr e = mapper_expr_new_from_string(str, 1, 0, 1);
    if (!e) { printf("FAIL parse: %s\n", str); return; }
    int outp;
    mapper_expr_evaluate(e, &x, &outp);
    printf("FLOAT_INT [%s] x=%f -> %d\n", str, x, outp);
    mapper_expr_free(e);
}

int main() {
    run_int("y=x", 5);
    run_int("y=x+1", 10);
    run_int("y=x*2", 7);
    run_int("y=x-1", 0);
    run_int("y=10/x", 3);  // integer divide
    run_int("y=2+3*4", 0);
    run_int("y=(2+3)*4", 0);
    run_int("y=-x", 7);
    run_int("y=-5+x", 10);
    run_int("y=x*x", 6);
    run_float("y=x", 1.5);
    run_float("y=x+1.5", 2.0);
    run_float("y=2.5*x", 4.0);
    run_float("y=sin(x)", 0.0);
    run_float("y=cos(0.0)", 0.0);
    run_float("y=pow(2,3)", 0.0);
    run_float("y=sqrt(16.0)", 0.0);
    run_float("y=abs(-3.5)", 0.0);
    run_float("y=pi", 0.0);
    run_float("y=min(1.0,2.0)", 0.0);
    run_float("y=max(1.0,2.0)", 0.0);
    run_float("y=log10(100.0)", 0.0);
    run_float("y=log(2.718281828)", 0.0);
    run_float("y=exp(1.0)", 0.0);
    run_float("y=floor(3.7)", 0.0);
    run_float("y=ceil(3.2)", 0.0);
    run_float("y=round(3.5)", 0.0);
    run_float("y=tan(0.0)", 0.0);
    run_float("y=asin(1.0)", 0.0);
    run_float("y=acos(0.0)", 0.0);
    run_float("y=atan(1.0)", 0.0);
    run_float("y=atan2(1.0,1.0)", 0.0);
    run_float("y=hypot(3.0,4.0)", 0.0);
    run_float("y=cbrt(27.0)", 0.0);
    run_float("y=trunc(3.7)", 0.0);
    run_float("y=exp2(3.0)", 0.0);
    run_float("y=log2(8.0)", 0.0);
    run_int_float("y=x", 5);
    run_int_float("y=x+1", 5);
    run_float_int("y=x", 3.7);
    run_float_int("y=x*2.0", 1.5);
    return 0;
}
