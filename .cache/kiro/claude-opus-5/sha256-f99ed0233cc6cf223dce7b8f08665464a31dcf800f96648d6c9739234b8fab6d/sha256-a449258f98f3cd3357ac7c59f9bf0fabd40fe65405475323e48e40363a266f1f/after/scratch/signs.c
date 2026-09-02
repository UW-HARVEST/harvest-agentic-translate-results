#include <stdio.h>
static inline int step(int x){ x=x*3+7; x=x^(x>>3); x=x-(x<<1); x=x/2+x%7; return x; }
static void scan(int entry,long len,const char*tag){
    long neg=0,pos=0,zero=0; int mn=2147483647,mx=-2147483648; int c=entry;
    for(long i=0;i<len;i++){ if(c<0)neg++; else if(c>0)pos++; else zero++;
        if(c<mn)mn=c; if(c>mx)mx=c; c=step(c);} 
    printf("%s len=%ld neg=%ld pos=%ld zero=%ld min=%d max=%d\n",tag,len,neg,pos,zero,mn,mx);
}
int main(void){ scan(-777334832,52330,"cycleA"); scan(-1030007402,11991,"cycleB");
    // how many distinct cycles exist? sample 200k random-ish starts
    return 0; }
