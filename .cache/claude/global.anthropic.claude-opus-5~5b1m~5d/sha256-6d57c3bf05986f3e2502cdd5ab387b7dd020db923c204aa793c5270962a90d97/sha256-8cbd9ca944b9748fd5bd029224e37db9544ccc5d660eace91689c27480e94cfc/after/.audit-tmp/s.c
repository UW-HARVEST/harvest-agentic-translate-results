#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
int main(int argc,char**argv){
  setvbuf(stdout,NULL,_IONBF,0);
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
  if(!h){fprintf(stderr,"%s\n",dlerror());return 1;}
  char*(*sa)(void*,char*)=dlsym(h,"stbds_stralloc");
  void(*sr)(void*)=dlsym(h,"stbds_strreset");
  unsigned char a[24]; memset(a,0,24);
  a[16]=(unsigned char)atoi(argv[2]);            /* block */
  printf("block=%u\n",a[16]);
  char*p=sa(a,"hello");
  printf("ok p=%s block_after=%u remaining=%zu\n",p,a[16],*(size_t*)(a+8));
  sr(a);
  return 0;
}
