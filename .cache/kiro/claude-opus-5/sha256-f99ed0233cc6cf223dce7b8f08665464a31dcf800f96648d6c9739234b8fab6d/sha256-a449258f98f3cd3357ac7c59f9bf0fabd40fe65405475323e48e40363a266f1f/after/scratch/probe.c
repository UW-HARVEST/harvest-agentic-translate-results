#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static inline int step(int x){
    x = x*3+7; x = x^(x>>3); x = x-(x<<1); x = x/2 + x%7; return x;
}
int main(void){
    // Floyd cycle detection from a few starts
    int starts[]={0,1,5,42,123456789,-5,2147483647,-2147483648};
    for(size_t i=0;i<sizeof(starts)/sizeof(starts[0]);i++){
        int t=starts[i], h=starts[i];
        long mu=0;
        do { t=step(t); h=step(step(h)); mu++; } while(t!=h && mu<100000000);
        // cycle length
        long lam=0; int p=t; do { p=step(p); lam++; } while(p!=t && lam<100000000);
        // tail length
        long tail=0; int a=starts[i], b=t;
        while(a!=b && tail<100000000){ a=step(a); b=step(b); tail++; }
        printf("start=%d tail=%ld cyclen=%ld cycle_entry=%d\n", starts[i], tail, lam, a);
    }
    // enumerate the cycle from 0
    int x=0; for(int i=0;i<400;i++) x=step(x);
    printf("cycle elems from 0 after 400 steps: ");
    int c=x; for(int i=0;i<12;i++){ printf("%d ", c); c=step(c);} printf("\n");
    return 0;
}
