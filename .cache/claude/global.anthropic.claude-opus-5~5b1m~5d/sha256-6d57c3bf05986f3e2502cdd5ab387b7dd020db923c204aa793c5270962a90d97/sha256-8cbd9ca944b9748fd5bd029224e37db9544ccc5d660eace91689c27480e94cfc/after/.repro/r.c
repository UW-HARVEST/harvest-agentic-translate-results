#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef struct { void*storage; size_t remaining; unsigned char block, mode; } Arena;
int main(int argc,char**argv){
  void*h=dlopen(argv[1],RTLD_NOW); if(!h){fprintf(stderr,"%s\n",dlerror());return 2;}
  int which=atoi(argv[2]);
  if(which==1){ /* D1: stralloc with a caller-supplied arena whose block is preset */
    char*(*stralloc)(Arena*,char*)=dlsym(h,"stbds_stralloc");
    Arena a; memset(&a,0,sizeof a); a.block=(unsigned char)atoi(argv[3]);
    char s[]="hello";
    char*p=stralloc(&a,s);
    printf("ok p=%s block_after=%u remaining=%zu\n",p,a.block,a.remaining);
  } else { /* D2: hmdel_key with mode=2 removing a NON-last element */
    void*(*put)(void*,size_t,void*,size_t,int)=dlsym(h,"stbds_hmput_key");
    void*(*del)(void*,size_t,void*,size_t,size_t,int)=dlsym(h,"stbds_hmdel_key");
    void(*rs)(size_t)=dlsym(h,"stbds_rand_seed");
    rs(0x31415926);
    size_t es=16; void*t=NULL;
    char*keys[4]={"alpha","beta","gamma","delta"};
    for(int i=0;i<4;i++) t=put(t,es,keys[i],8,2);
    t=del(t,es,keys[0],8,0,2);
    printf("survived, len=%zu\n", *(size_t*)((char*)t-es-32));
  }
  return 0;
}
