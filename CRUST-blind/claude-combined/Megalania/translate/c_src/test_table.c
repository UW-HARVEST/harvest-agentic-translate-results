#include <stdio.h>
#include <stdint.h>
#include "src/perplexity_table.h"
int main() {
    printf("LOG2_LOOKUP[100]=%lu\n", LOG2_LOOKUP[100]);
    printf("LOG2_LOOKUP[1024]=%lu\n", LOG2_LOOKUP[1024]);
    printf("LOG2_LOOKUP[2000]=%lu\n", LOG2_LOOKUP[2000]);
    printf("LOG2_LOOKUP[2047]=%lu\n", LOG2_LOOKUP[2047]);
    printf("LOG2_LOOKUP[2]=%lu\n", LOG2_LOOKUP[2]);
    return 0;
}
