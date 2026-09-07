#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <stdint.h>
typedef int (*inf_fn)(void*,int,void*,int);
static unsigned char in[4096] __attribute__((aligned(16)));
int main(int argc,char**argv){
  void*h=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
  inf_fn f=(inf_fn)dlsym(h,"cp_inflate");
  const char**err=(const char**)dlsym(h,"cp_error_reason");
  unsigned char out[8192];
  struct { const char*name; int n; int ob; unsigned char d[64]; } t[] = {
    {"len/nlen", 9, 100, {0x00,0x05,0x00,0x00,0x00,'h','e','l','l'}},
    {"stored-beyond", 40, 100, {0x00,0x01,0x00,0xFE,0xFF,'x'}},
    {"btype3", 6, 100, {0x07}},
    {"stored-ok", 9, 100, {0x01,0x04,0x00,0xFB,0xFF,'a','b','c','d'}},
  };
  for (unsigned i=0;i<sizeof(t)/sizeof(t[0]);i++){
    memset(in,0,sizeof in); memcpy(in,t[i].d,64); *err=NULL; memset(out,0,sizeof out);
    int r=f(in,t[i].n,out,t[i].ob);
    printf("%-14s ret=%d err=[%s] out=[%.8s]\n", t[i].name, r, *err?*err:"(null)", out);
  }
  // symbol/string/backwards errors from a real deflate stream
  FILE*fp=fopen("stream.bin","rb"); int n=fread(in,1,sizeof in,fp); fclose(fp);
  int obs[]={0,1,2,5,20,100};
  for (unsigned i=0;i<sizeof(obs)/sizeof(obs[0]);i++){
    *err=NULL; memset(out,0,sizeof out);
    int r=f(in,n,out,obs[i]);
    printf("stream ob=%-4d ret=%d err=[%s]\n", obs[i], r, *err?*err:"(null)");
  }
  return 0;
}
