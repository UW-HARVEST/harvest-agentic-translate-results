#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef struct { size_t length,capacity; void*hash_table; ptrdiff_t temp; } HDR;
typedef struct { char*temp_key; size_t slot_count,used_count,uct,ucst,tomb,tct,seed,log2;
                 struct{void*st;size_t rem;unsigned char blk,mode;} string; void*storage; } HIDX;
typedef struct { char *key; int value; } SE;   /* string map entry, elemsize 16 */
typedef struct { int key; int value; } BE;     /* binary map entry, elemsize 8 */

typedef void*  (*fn_put)(void*,size_t,void*,size_t,int);
typedef void*  (*fn_get)(void*,size_t,void*,size_t,int);
typedef void*  (*fn_del)(void*,size_t,void*,size_t,size_t,int);
typedef void*  (*fn_shmode)(size_t,int);
typedef void   (*fn_hmfree)(void*,size_t);
typedef void   (*fn_rs)(size_t);
typedef void*  (*fn_putdef)(void*,size_t);
typedef void*  (*fn_getts)(void*,size_t,void*,size_t,ptrdiff_t*,int);
typedef struct { void*h; fn_put put; fn_get get; fn_del del; fn_shmode shmode;
                 fn_hmfree hmfree; fn_rs rand_seed; fn_putdef putdef; fn_getts getts; } L;
static void load(L*l,const char*p){ l->h=dlopen(p,RTLD_NOW|RTLD_LOCAL);
  if(!l->h){fprintf(stderr,"dlopen %s: %s\n",p,dlerror());exit(1);}
#define G(f,n) l->f=(void*)dlsym(l->h,n); if(!l->f){fprintf(stderr,"sym %s\n",n);exit(1);}
  G(put,"stbds_hmput_key") G(get,"stbds_hmget_key") G(del,"stbds_hmdel_key")
  G(shmode,"stbds_shmode_func") G(hmfree,"stbds_hmfree_func") G(rand_seed,"stbds_rand_seed")
  G(putdef,"stbds_hmput_default") G(getts,"stbds_hmget_key_ts")
#undef G
}
static int fails=0;
static int cmp_tk=0;
static const char*curkey=0;
static unsigned long long rs=12345678901ULL;
static unsigned long long xs(void){ rs^=rs<<13; rs^=rs>>7; rs^=rs<<17; return rs; }

/* dump observable state of a string map into a text buffer */
static void dumpS(char*out,size_t n,void*t,size_t elemsize){
  size_t o=0;
  if(!t){ snprintf(out,n,"NULL"); return; }
  HDR*h=((HDR*)((char*)t-elemsize))-1;
  o+=snprintf(out+o,n-o,"len=%zu temp=%td",h->length,h->temp);
  HIDX*x=(HIDX*)h->hash_table;
  if(x) o+=snprintf(out+o,n-o," sc=%zu uc=%zu tomb=%zu mode=%u blk=%u tk=%s",
                    x->slot_count,x->used_count,x->tomb,x->string.mode,x->string.blk,
                    cmp_tk?(x->temp_key&&curkey&&!strcmp(x->temp_key,curkey)?"SET":"UNSET"):"(skip)");
  SE*e=(SE*)t;
  for(size_t i=0;i+1<h->length && o<n-80;i++)
    o+=snprintf(out+o,n-o,"|%s=%d",e[i].key?e[i].key:"(nil)",e[i].value);
  out[o<n?o:n-1]=0;
}
static void dumpB(char*out,size_t n,void*t,size_t elemsize){
  size_t o=0;
  if(!t){ snprintf(out,n,"NULL"); return; }
  HDR*h=((HDR*)((char*)t-elemsize))-1;
  o+=snprintf(out+o,n-o,"len=%zu temp=%td",h->length,h->temp);
  HIDX*x=(HIDX*)h->hash_table;
  if(x) o+=snprintf(out+o,n-o," sc=%zu uc=%zu tomb=%zu",x->slot_count,x->used_count,x->tomb);
  BE*e=(BE*)t;
  for(size_t i=0;i+1<h->length && o<n-40;i++) o+=snprintf(out+o,n-o,"|%d=%d",e[i].key,e[i].value);
  out[o<n?o:n-1]=0;
}

#define NK 64
static char keys[NK][24];

static void run_string(L*c,L*r,int shmode_kind,const char*label){
  void*tc=NULL,*tr=NULL;
  size_t ES=sizeof(SE);
  c->rand_seed(0x31415926); r->rand_seed(0x31415926);
  if(shmode_kind>=0){ tc=c->shmode(ES,shmode_kind); tr=r->shmode(ES,shmode_kind); }
  char bc[4096],br[4096];
  for(int step=0;step<6000;step++){
    unsigned long long v=xs(); int op=v%10; int ki=(v>>8)%NK; cmp_tk=(op<5);
    char*k=keys[ki]; curkey=k;
    if(op<5){ /* shput */
      tc=c->put(tc,ES,k,sizeof(char*),1); tr=r->put(tr,ES,k,sizeof(char*),1);
      ptrdiff_t ic=(((HDR*)((char*)tc-ES))-1)->temp, ir=(((HDR*)((char*)tr-ES))-1)->temp;
      if(ic!=ir){fails++;printf("%s step %d put temp %td vs %td\n",label,step,ic,ir);return;}
      ((SE*)tc)[ic].value=step; ((SE*)tr)[ir].value=step;
    } else if(op<7){ /* shget */
      tc=c->get(tc,ES,k,sizeof(char*),1); tr=r->get(tr,ES,k,sizeof(char*),1);
    } else if(op<9){ /* shdel */
      tc=c->del(tc,ES,k,sizeof(char*),offsetof(SE,key),1);
      tr=r->del(tr,ES,k,sizeof(char*),offsetof(SE,key),1);
    } else { /* hmget_key_ts */
      ptrdiff_t a=-9,b=-9;
      tc=c->getts(tc,ES,k,sizeof(char*),&a,1); tr=r->getts(tr,ES,k,sizeof(char*),&b,1);
      if(a!=b){fails++;printf("%s step %d getts %td vs %td\n",label,step,a,b);return;}
    }
    dumpS(bc,sizeof bc,tc,ES); dumpS(br,sizeof br,tr,ES);
    if(strcmp(bc,br)){fails++;printf("%s step %d op=%d key=%s\n C=%s\n R=%s\n",label,step,op,k,bc,br);return;}
  }
  if(tc)c->hmfree((char*)tc-ES,ES); if(tr)r->hmfree((char*)tr-ES,ES);
  printf("%s ok\n",label);
}

static void run_binary(L*c,L*r){
  void*tc=NULL,*tr=NULL; size_t ES=sizeof(BE);
  c->rand_seed(0x31415926); r->rand_seed(0x31415926);
  char bc[4096],br[4096];
  for(int step=0;step<8000;step++){
    unsigned long long v=xs(); int op=v%10; int key=(int)((v>>8)%40); int kk=key;
    if(op<5){
      tc=c->put(tc,ES,&kk,sizeof(int),0); tr=r->put(tr,ES,&kk,sizeof(int),0);
      ptrdiff_t ic=(((HDR*)((char*)tc-ES))-1)->temp, ir=(((HDR*)((char*)tr-ES))-1)->temp;
      if(ic!=ir){fails++;printf("bin step %d put temp %td vs %td\n",step,ic,ir);return;}
      ((BE*)tc)[ic].key=kk; ((BE*)tc)[ic].value=step;
      ((BE*)tr)[ir].key=kk; ((BE*)tr)[ir].value=step;
    } else if(op<7){
      tc=c->get(tc,ES,&kk,sizeof(int),0); tr=r->get(tr,ES,&kk,sizeof(int),0);
    } else if(op<9){
      tc=c->del(tc,ES,&kk,sizeof(int),offsetof(BE,key),0);
      tr=r->del(tr,ES,&kk,sizeof(int),offsetof(BE,key),0);
    } else {
      tc=c->putdef(tc,ES); tr=r->putdef(tr,ES);
    }
    dumpB(bc,sizeof bc,tc,ES); dumpB(br,sizeof br,tr,ES);
    if(strcmp(bc,br)){fails++;printf("bin step %d op=%d key=%d\n C=%s\n R=%s\n",step,op,kk,bc,br);return;}
  }
  if(tc)c->hmfree((char*)tc-ES,ES); if(tr)r->hmfree((char*)tr-ES,ES);
  printf("binary ok\n");
}
int main(int argc,char**argv){
  setvbuf(stdout,NULL,_IONBF,0); L c,r; load(&c,argv[1]); load(&r,argv[2]);
  for(int i=0;i<NK;i++) snprintf(keys[i],sizeof keys[i],"key_%d_%c",i,'a'+(i%26));
  run_string(&c,&r,-1,"string/default");
  run_string(&c,&r,2,"string/strdup");
  run_string(&c,&r,3,"string/arena");
  run_string(&c,&r,1,"string/shdefault");
  
  run_binary(&c,&r);
  printf("TOTAL=%d\n",fails);
  return fails!=0;
}
