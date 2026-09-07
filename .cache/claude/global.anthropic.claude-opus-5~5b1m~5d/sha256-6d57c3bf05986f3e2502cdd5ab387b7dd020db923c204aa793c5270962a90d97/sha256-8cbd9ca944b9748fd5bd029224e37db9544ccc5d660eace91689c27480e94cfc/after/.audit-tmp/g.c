#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef struct { size_t length,capacity; void*hash_table; ptrdiff_t temp; } HDR;
typedef void*(*fn)(void*,size_t,size_t,size_t);
typedef void (*ff)(void*);
int main(int argc,char**argv){
  setvbuf(stdout,NULL,_IONBF,0);
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL); if(!h){fprintf(stderr,"%s\n",dlerror());return 1;}
  fn g=dlsym(h,"stbds_arrgrowf"); ff f=dlsym(h,"stbds_arrfreef");
  size_t caps[]={0,1,2,3,4,5,7,8,100,1000};
  for(size_t es=1;es<=17;es+=4){
    void*a=NULL;
    for(int i=0;i<10;i++){
      size_t addlen=caps[i], mc=caps[(i*7)%10];
      a=g(a,es,addlen,mc);
      HDR*hd=((HDR*)a)-1;
      printf("es=%zu add=%zu mc=%zu -> len=%zu cap=%zu ht=%p temp=%td\n",es,addlen,mc,hd->length,hd->capacity,hd->hash_table,hd->temp);
      hd->length += addlen;
    }
    f(a);
  }
  /* wraparound: 2*cap overflow is unreachable; test min_cap<4 path and no-op path */
  { void*a=g(NULL,8,0,0); HDR*hd=((HDR*)a)-1; printf("nil,8,0,0 -> cap=%zu len=%zu\n",hd->capacity,hd->length);
    void*b=g(a,8,0,4); printf("same? %d\n", a==b);
    void*c2=g(a,8,0,5); HDR*h2=((HDR*)c2)-1; printf("grow to 5 -> cap=%zu\n",h2->capacity); f(c2); }
  return 0;
}
