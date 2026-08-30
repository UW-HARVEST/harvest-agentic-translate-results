#include <stdio.h>
#include <stdlib.h>
int main(int argc, char**argv){
    if (argc < 3) return 2;
    int x = atoi(argv[1]), y = atoi(argv[2]);
    div_t r = div(x,y);
    printf("quotient: %d, remainder: %d\n", r.quot, r.rem);
    return 0;
}
