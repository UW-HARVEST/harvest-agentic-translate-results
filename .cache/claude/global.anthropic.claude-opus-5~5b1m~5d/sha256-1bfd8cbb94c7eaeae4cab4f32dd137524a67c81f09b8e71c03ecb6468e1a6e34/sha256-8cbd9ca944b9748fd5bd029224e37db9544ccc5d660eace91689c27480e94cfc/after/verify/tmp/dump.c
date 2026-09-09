#include <stdio.h>
#include <string.h>
static unsigned long long g_st;
static void rs(unsigned long long s){ g_st = s?s:1; }
static unsigned long long r64(void){ g_st^=g_st<<13; g_st^=g_st>>7; g_st^=g_st<<17; return g_st; }
static unsigned r32(void){ return (unsigned)(r64()>>32); }
int main(int argc,char**argv){
  int target = atoi(argv[1]);
  unsigned char b[4096]; size_t n; int i,k;
  rs(0x7001);
  for(k=0;k<=target;k++){
    memset(b,0,sizeof(b));
    n = 4 + (size_t)(r32()%200);
    for(i=0;i<(int)n;i++) b[i]=(unsigned char)r32();
  }
  printf("n=%zu\n",n);
  for(i=0;i<(int)n;i++) printf("%02x", b[i]);
  printf("\n");
  return 0;
}
