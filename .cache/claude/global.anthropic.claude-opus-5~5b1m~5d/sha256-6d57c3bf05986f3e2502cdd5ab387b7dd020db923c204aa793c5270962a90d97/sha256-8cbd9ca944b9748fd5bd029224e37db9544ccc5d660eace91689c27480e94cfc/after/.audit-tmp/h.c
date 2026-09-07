#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef size_t (*fn_hb)(void*,size_t,size_t);
typedef size_t (*fn_hs)(char*,size_t);
typedef char*  (*fn_stralloc)(void*,char*);
typedef void   (*fn_strreset)(void*);
typedef struct { void*h; fn_hb hash_bytes; fn_hs hash_string;
  fn_stralloc stralloc; fn_strreset strreset; } L;
static void load(L*l,const char*p){ l->h=dlopen(p,RTLD_NOW|RTLD_LOCAL);
  if(!l->h){fprintf(stderr,"dlopen %s: %s\n",p,dlerror());exit(1);}
#define G(f,n) l->f=(void*)dlsym(l->h,n); if(!l->f){fprintf(stderr,"sym %s\n",n);exit(1);}
  G(hash_bytes,"stbds_hash_bytes") G(hash_string,"stbds_hash_string")
  G(stralloc,"stbds_stralloc") G(strreset,"stbds_strreset")
#undef G
}
static int fails=0;
static unsigned long long rs=88172645463325252ULL;
static unsigned long long xs(void){ rs^=rs<<13; rs^=rs>>7; rs^=rs<<17; return rs; }
int main(int argc,char**argv){
  L c,r; load(&c,argv[1]); load(&r,argv[2]);
  for(size_t len=0;len<=40;++len) for(int t=0;t<400;++t){
    unsigned char buf[64];
    for(size_t k=0;k<len;k++){ unsigned long long v=xs(); int m=v&3;
      buf[k]= m==0?0x00:m==1?0x80:m==2?0xff:(unsigned char)(v>>8); }
    size_t seed=(t&1)?0:(size_t)xs();
    size_t a=c.hash_bytes(buf,len,seed), b=r.hash_bytes(buf,len,seed);
    if(a!=b){ fails++; printf("hash_bytes MISMATCH len=%zu seed=0x%zx C=0x%zx R=0x%zx bytes=",len,seed,a,b);
      for(size_t k=0;k<len;k++)printf("%02x",buf[k]); printf("\n"); if(fails>6)return 1;}
  }
  printf("hash_bytes done fails=%d\n",fails);
  for(int t=0;t<20000;++t){ char s[40]; int n=xs()%38;
    for(int k=0;k<n;k++){unsigned long long v=xs();int m=v&3;
      s[k]=m==0?(char)0x80:m==1?(char)0xff:m==2?'a':(char)((v>>8)|1); if(!s[k])s[k]='z';}
    s[n]=0; size_t seed=(size_t)xs();
    size_t a=c.hash_string(s,seed), b=r.hash_string(s,seed);
    if(a!=b){fails++;printf("hash_string MISMATCH n=%d seed=0x%zx C=0x%zx R=0x%zx\n",n,seed,a,b);if(fails>6)return 1;}
  }
  printf("hash_string done fails=%d\n",fails);
  { unsigned char ac[24],ar[24]; memset(ac,0,24); memset(ar,0,24);
    for(int i=0;i<5000;i++){ char s[600]; size_t n=(i%7==0)?(xs()%550)+1:(xs()%30)+1;
      for(size_t k=0;k<n;k++) s[k]='a'+(k%26); s[n]=0;
      char*pc=c.stralloc(ac,s); char*pr=r.stralloc(ar,s);
      if(strcmp(pc,s)||strcmp(pr,s)){fails++;printf("stralloc content MISMATCH i=%d\n",i);break;}
      size_t remc=*(size_t*)(ac+8),remr=*(size_t*)(ar+8); unsigned bc=ac[16],br=ar[16];
      if(remc!=remr||bc!=br){fails++;printf("stralloc state MISMATCH i=%d rem %zu/%zu block %u/%u\n",i,remc,remr,bc,br);break;}
    }
    c.strreset(ac); r.strreset(ar);
    if(memcmp(ac,ar,24)){fails++;printf("strreset MISMATCH\n");}
  }
  printf("stralloc done fails=%d\nTOTAL=%d\n",fails,fails);
  return fails!=0;
}
