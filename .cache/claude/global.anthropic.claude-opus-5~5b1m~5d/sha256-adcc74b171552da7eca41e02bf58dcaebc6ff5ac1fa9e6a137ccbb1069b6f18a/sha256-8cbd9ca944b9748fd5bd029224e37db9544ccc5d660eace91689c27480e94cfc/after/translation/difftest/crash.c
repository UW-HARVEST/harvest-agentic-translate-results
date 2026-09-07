#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int cp_inflate(void*,int,void*,int);
extern const char* cp_error_reason;
int main(int argc,char**argv){
  FILE*f=fopen(argv[1],"rb"); static unsigned char in[1<<20]; int n=fread(in,1,sizeof in,f);
  int ob=atoi(argv[2]);
  unsigned char*out=malloc(1<<20); memset(out,0xAA,1<<20);
  int r=cp_inflate(in,n,out,ob);
  printf("ret=%d err=%s\n",r,cp_error_reason?cp_error_reason:"(null)");
  return 0;
}
