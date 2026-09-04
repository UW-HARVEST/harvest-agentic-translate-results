#include <stdio.h>
#include <string.h>
#include <stdlib.h>
extern size_t FSEv05_readNCount(short*, unsigned*, unsigned*, const void*, size_t);
extern size_t FSEv06_readNCount(short*, unsigned*, unsigned*, const void*, size_t);
extern size_t FSEv07_readNCount(short*, unsigned*, unsigned*, const void*, size_t);
static unsigned long long fnv(const void*p,size_t n){const unsigned char*b=p;unsigned long long h=1469598103934665603ULL;size_t i;for(i=0;i<n;i++){h^=b[i];h*=1099511628211ULL;}return h;}
static int hexval(char c){ if(c>='0'&&c<='9')return c-'0'; if(c>='a'&&c<='f')return c-'a'+10; if(c>='A'&&c<='F')return c-'A'+10; return -1; }
int main(int argc,char**argv){
  unsigned char in[4096]; size_t n=0; const char*h=argv[1];
  short nc[4096]; unsigned msv, tl; size_t r; int which = argc>2?atoi(argv[2]):5;
  size_t maxsv = argc>3?(size_t)atoi(argv[3]):255;
  size_t itl   = argc>4?(size_t)atoi(argv[4]):12;
  while(h[0]&&h[1]){ int a=hexval(h[0]),b=hexval(h[1]); if(a<0||b<0)break; in[n++]=(unsigned char)(a*16+b); h+=2; }
  memset(nc,0,sizeof(nc)); msv=(unsigned)maxsv; tl=(unsigned)itl;
  if(which==5) r=FSEv05_readNCount(nc,&msv,&tl,in,n);
  else if(which==6) r=FSEv06_readNCount(nc,&msv,&tl,in,n);
  else r=FSEv07_readNCount(nc,&msv,&tl,in,n);
  { long long s=(long long)r; if(s<0&&s>-1000) printf("v%d n=%zu rv=E%d msv=%u tl=%u nch=%016llx\n",which,n,(int)-s,msv,tl,fnv(nc,512));
    else printf("v%d n=%zu rv=%zu msv=%u tl=%u nch=%016llx\n",which,n,r,msv,tl,fnv(nc,512)); }
  { int i; printf("nc:"); for(i=0;i<40;i++) printf(" %d",nc[i]); printf("\n"); }
  return 0;
}
