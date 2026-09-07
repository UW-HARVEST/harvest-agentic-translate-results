#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef struct { char*key; int value; } SE;
int main(int argc,char**argv){
  setvbuf(stdout,NULL,_IONBF,0);
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
  if(!h){fprintf(stderr,"%s\n",dlerror());return 1;}
  void*(*put)(void*,size_t,void*,size_t,int)=dlsym(h,"stbds_hmput_key");
  void*(*del)(void*,size_t,void*,size_t,size_t,int)=dlsym(h,"stbds_hmdel_key");
  void(*rsd)(size_t)=dlsym(h,"stbds_rand_seed"); rsd(0x31415926);
  int mode=atoi(argv[2]);
  void*t=NULL; size_t ES=sizeof(SE);
  static char *ks[]={"alpha","beta","gamma","delta"};
  for(int i=0;i<4;i++){ t=put(t,ES,ks[i],sizeof(char*),mode);
    ptrdiff_t idx=*(ptrdiff_t*)((char*)t-ES-8); ((SE*)t)[idx].value=100+i;
    printf("put %s -> idx %td\n",ks[i],idx); }
  printf("deleting alpha with mode=%d\n",mode);
  t=del(t,ES,"alpha",sizeof(char*),offsetof(SE,key),mode);
  printf("survived delete, t=%p\n",t);
  return 0;
}
