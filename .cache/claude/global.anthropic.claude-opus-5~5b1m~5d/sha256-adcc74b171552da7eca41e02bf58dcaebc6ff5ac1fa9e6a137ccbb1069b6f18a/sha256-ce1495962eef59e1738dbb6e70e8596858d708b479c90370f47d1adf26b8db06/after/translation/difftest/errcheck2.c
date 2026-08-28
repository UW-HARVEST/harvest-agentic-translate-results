#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
typedef int (*inf_fn)(void*,int,void*,int);
static unsigned char in[4096] __attribute__((aligned(16)));
int main(int argc,char**argv){
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
  inf_fn f=(inf_fn)dlsym(h,"cp_inflate");
  const char**err=(const char**)dlsym(h,"cp_error_reason");
  FILE*fp=fopen("backdist.bin","rb"); int n=fread(in,1,sizeof in,fp); fclose(fp);
  unsigned char out[256];
  for(int ob=0;ob<=64;ob+=16){
    *err=NULL; memset(out,0,sizeof out);
    int r=f(in,n,out,ob);
    printf("ob=%-3d ret=%d err=[%s] out=[%.16s]\n",ob,r,*err?*err:"(null)",out);
  }
  return 0;
}
