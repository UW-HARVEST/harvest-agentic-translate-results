#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stddef.h>
static int fails=0, checks=0;
static void*gs(void*h,const char*n){void*p=dlsym(h,n); if(!p){fprintf(stderr,"miss %s\n",n);exit(2);} return p;}
typedef struct { unsigned bsid,bmode,ccflag,ftype; unsigned long long csize; unsigned dictID; unsigned bcflag; } FI;
typedef struct { FI fi; int level; unsigned autoFlush,favorDecSpeed; unsigned reserved[3]; } PREFS;
typedef struct { unsigned stableSrc; unsigned r[3]; } COPT;
typedef struct { unsigned stableDst,skipChecksums,r1,r0; } DOPT;
typedef size_t(*fn_cf)(void*,size_t,const void*,size_t,const PREFS*);
typedef size_t(*fn_cfb)(size_t,const PREFS*);
typedef unsigned(*fn_ie)(size_t);
typedef size_t(*fn_cdc)(void**,unsigned);
typedef size_t(*fn_g)(void*);
typedef size_t(*fn_dec)(void*,void*,size_t*,const void*,size_t*,const DOPT*);
static uint64_t rs=0xdeadbeefcafeULL;
static uint64_t rnd(void){rs^=rs<<13;rs^=rs>>7;rs^=rs<<17;return rs;}
static void fill(unsigned char*b,size_t n,int m){size_t i;for(i=0;i<n;i++){switch(m){
case 0: b[i]=(unsigned char)rnd();break; case 1: b[i]='a';break;
case 2: b[i]=(unsigned char)('a'+(i%7));break; case 3: b[i]=(unsigned char)((rnd()%4)?'x':(unsigned char)rnd());break;
case 4: b[i]=(unsigned char)(i&0xff);break; default: b[i]=(unsigned char)((rnd()%16)+'A');}}}
struct A{fn_cf cf;fn_cfb cfb;fn_ie ie;fn_cdc cd;fn_g fd;fn_dec dec;};
static void load(struct A*a,void*h){a->cf=gs(h,"LZ4F_compressFrame");a->cfb=gs(h,"LZ4F_compressFrameBound");
 a->ie=gs(h,"LZ4F_isError");a->cd=gs(h,"LZ4F_createDecompressionContext");a->fd=gs(h,"LZ4F_freeDecompressionContext");
 a->dec=gs(h,"LZ4F_decompress");}
static size_t dfr(struct A*a,const unsigned char*cb,size_t cs,unsigned char*o,size_t oc,size_t sch,size_t dch,size_t*ne){
 void*d=NULL; size_t r=a->cd(&d,100); size_t si=0,di=0; *ne=0;
 if(a->ie(r)){*ne=r;return (size_t)-1;}
 while(si<cs){size_t sc=cs-si;if(sc>sch)sc=sch;size_t dc=oc-di;if(dc>dch)dc=dch;
  size_t h=a->dec(d,o+di,&dc,cb+si,&sc,NULL);
  if(a->ie(h)){*ne=h;a->fd(d);return (size_t)-1;}
  si+=sc;di+=dc; if(h==0)break; if(sc==0&&dc==0)break;}
 a->fd(d);return di;}
static struct A C,R;
int main(void){
 void*hC=dlopen("./cbuild/liblz4.so",RTLD_NOW),*hR=dlopen("./translation/target/release/liblz4.so",RTLD_NOW);
 if(!hC||!hR){printf("dlopen fail\n");return 2;} load(&C,hC);load(&R,hR);
 size_t maxN=300000; unsigned char*src=malloc(maxN); size_t cap=maxN+maxN/100+4096;
 unsigned char*bC=malloc(cap),*bR=malloc(cap),*oC=malloc(maxN+8192),*oR=malloc(maxN+8192);
 size_t sizes[]={0,1,13,100,65536,65537,300000};
 int levels[]={-3,0,1,2,9,10,12};
 int bsids[]={0,4,5,7};
 for(int mode=0;mode<6;mode++)for(int si=0;si<7;si++){
  size_t n=sizes[si]; fill(src,n,mode);
  for(int il=0;il<7;il++)for(int ib=0;ib<4;ib++)for(int f=0;f<4;f++){
   PREFS p;memset(&p,0,sizeof p);
   p.fi.bsid=bsids[ib]; p.level=levels[il];
   p.fi.ccflag=f&1; p.fi.bcflag=(f>>1)&1; p.fi.bmode=(il+ib)&1;
   p.fi.csize=(f&1)?1:0; p.fi.dictID=(f&2)?99:0; p.favorDecSpeed=il&1;
   size_t bd=C.cfb(n,&p); if(bd>cap)continue;
   memset(bC,0xAA,bd);memset(bR,0xAA,bd);
   size_t rc=C.cf(bC,bd,src,n,&p), rr=R.cf(bR,bd,src,n,&p);
   checks++;
   if(rc!=rr||(!C.ie(rc)&&memcmp(bC,bR,rc))){printf("MISMATCH cf n=%zu lvl=%d bsid=%u f=%d rc=%zd rr=%zd\n",n,p.level,p.fi.bsid,f,(ptrdiff_t)rc,(ptrdiff_t)rr);fails++;continue;}
   if(C.ie(rc))continue;
   size_t chunks[3]={1,997,(size_t)-1};
   for(int k=0;k<3;k++){size_t e1,e2;
    size_t a=dfr(&C,bC,rc,oC,maxN+8192,chunks[k],chunks[k],&e1);
    size_t b=dfr(&R,bR,rc,oR,maxN+8192,chunks[k],chunks[k],&e2);
    checks++;
    if(a!=b||e1!=e2){printf("MISMATCH dec n=%zu lvl=%d bsid=%u f=%d ck=%zu %zu/%zd vs %zu/%zd\n",n,p.level,p.fi.bsid,f,chunks[k],a,(ptrdiff_t)e1,b,(ptrdiff_t)e2);fails++;}
    else if(a!=(size_t)-1&&(a!=n||memcmp(oC,src,n)||memcmp(oC,oR,a))){printf("MISMATCH decdata n=%zu lvl=%d\n",n,p.level);fails++;}
   }
  }
 }
 printf("checks=%d fails=%d\n",checks,fails);
 return fails?1:0;}
