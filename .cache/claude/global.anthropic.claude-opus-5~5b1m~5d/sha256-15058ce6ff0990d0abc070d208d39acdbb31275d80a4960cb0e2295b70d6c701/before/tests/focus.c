#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
typedef int (*fn_hc)(const char*, char*, int, int, int);
int main(int argc,char**argv){
  void*hC=dlopen("./cbuild/liblz4.so",RTLD_NOW), *hR=dlopen("./translation/target/release/liblz4.so",RTLD_NOW);
  fn_hc a=dlsym(hC,"LZ4_compress_HC"), b=dlsym(hR,"LZ4_compress_HC");
  int n = atoi(argv[1]); int lvl = atoi(argv[2]); int mode = argc>3?atoi(argv[3]):1;
  unsigned char src[400]; for(int i=0;i<n;i++) src[i] = mode==1?'a':(mode==2?('a'+i%7):(unsigned char)(i&0xff));
  char dc[600], dr[600]; memset(dc,0,600); memset(dr,0,600);
  int rc=a((char*)src,dc,n,600,lvl), rr=b((char*)src,dr,n,600,lvl);
  printf("n=%d lvl=%d mode=%d  C rc=%d:",n,lvl,mode,rc); for(int i=0;i<rc;i++)printf(" %02x",(unsigned char)dc[i]);
  printf("\n                  R rr=%d:",rr); for(int i=0;i<rr;i++)printf(" %02x",(unsigned char)dr[i]);
  printf("\n");
  return 0;
}
