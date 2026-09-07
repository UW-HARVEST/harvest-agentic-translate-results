#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
int main(int argc,char**argv){
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
  if(!h){fprintf(stderr,"%s\n",dlerror());return 1;}
  void(*hx)(char)=dlsym(h,"helxo");
  char*(*sk)(int)=dlsym(h,"strkey");
  void(*rsd)(size_t)=dlsym(h,"stbds_rand_seed");
  rsd(0x31415926);
  for(int c=0;c<260;c+=7){ printf("--- letter=%d\n", (int)(char)c); hx((char)c); }
  for(int n=-3;n<5;n++) printf("strkey(%d)=%s\n",n,sk(n));
  printf("strkey(%d)=%s\n",-2147483647-1,sk(-2147483647-1));
  fflush(stdout);
  return 0;
}
