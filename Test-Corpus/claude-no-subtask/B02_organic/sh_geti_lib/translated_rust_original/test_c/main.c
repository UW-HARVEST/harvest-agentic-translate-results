#include "lib.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    int num = argc > 1 ? atoi(argv[1]) : 5;
    sh_geti(num);
    return 0;
}
