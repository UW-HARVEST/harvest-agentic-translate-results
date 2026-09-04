#include <stdio.h>
#include <string.h>
int LZ4_decompress_safe(const char*,char*,int,int);
int LZ4_decompress_safe_partial(const char*,char*,int,int,int);
int main(void){
  char src[64]; memset(src,0xFF,sizeof src);
  char dst[2048];
  int negs[3] = {-1,-1000,-2147483647-1};
  for(int i=0;i<3;i++){ printf("neg %d ...", negs[i]); fflush(stdout);
    int r = LZ4_decompress_safe(src,dst,negs[i],1024); printf(" = %d\n", r); fflush(stdout); }
  int ts[6]={-1,0,1,50,100,101};
  for(int i=0;i<6;i++){ printf("partial t=%d ...", ts[i]); fflush(stdout);
    int r = LZ4_decompress_safe_partial(src,dst,10,ts[i],100); printf(" = %d\n", r); fflush(stdout); }
  return 0;
}
