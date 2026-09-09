#include <stdio.h>
#include <stdlib.h>
extern size_t ZBUFFv04_freeDCtx(void*);
extern size_t ZBUFFv05_freeDCtx(void*);
extern size_t ZBUFFv06_freeDCtx(void*);
extern size_t ZBUFFv07_freeDCtx(void*);
extern size_t ZSTDv07_freeDDict(void*);
int main(int c, char** v){
  int k = atoi(v[1]);
  size_t r=0;
  switch(k){
    case 4: r=ZBUFFv04_freeDCtx(NULL); break;
    case 5: r=ZBUFFv05_freeDCtx(NULL); break;
    case 6: r=ZBUFFv06_freeDCtx(NULL); break;
    case 7: r=ZBUFFv07_freeDCtx(NULL); break;
    case 8: r=ZSTDv07_freeDDict(NULL); break;
  }
  printf("k=%d r=%zu\n",k,r); return 0;
}
