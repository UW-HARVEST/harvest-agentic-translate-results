#include <stdio.h>
#include <string.h>
#include "lib.c"

#define NSIG (8*8*27)
#define MAXW 200
static long  cnt[NSIG];
static int   w[NSIG][MAXW];
static int   nw[NSIG];

int main(void){
    /* pass 1: count */
    for(int r=0;r<256;r++)for(int g=0;g<256;g++)for(int b=0;b<256;b++){
        cb_rgb_255 in={(unsigned char)r,(unsigned char)g,(unsigned char)b};
        cb_rgb n=cbNorm(in);
        int rg=0; { float ch[3]={n.R,n.G,n.B};
            for(int i=0;i<3;i++) if((double)ch[i]>0.04045) rg|=1<<i; }
        cb_rgb m=cbRemoveGammaRGB(n);
        Tritanopia(&m.R,&m.G,&m.B);
        int ag=0; { float mm[3]={m.R,m.G,m.B};
            for(int i=0;i<3;i++) if((double)mm[i]>0.00313080495356037151702786377709) ag|=1<<i; }
        cb_rgb a=cbApplyGammaRGB(m);
        int den=0,p=1; { float aa[3]={a.R,a.G,a.B};
            for(int i=0;i<3;i++){ float v=aa[i]*255.f+0.5f;
                den += ((v<0.f)?0:((v>=256.f)?2:1))*p; p*=3; } }
        cnt[rg|(ag<<3)|(den<<6)]++;
    }
    /* pass 2: stride-sample witnesses so they spread over the whole space */
    long seen[NSIG]; memset(seen,0,sizeof seen);
    for(int r=0;r<256;r++)for(int g=0;g<256;g++)for(int b=0;b<256;b++){
        cb_rgb_255 in={(unsigned char)r,(unsigned char)g,(unsigned char)b};
        cb_rgb n=cbNorm(in);
        int rg=0; { float ch[3]={n.R,n.G,n.B};
            for(int i=0;i<3;i++) if((double)ch[i]>0.04045) rg|=1<<i; }
        cb_rgb m=cbRemoveGammaRGB(n);
        Tritanopia(&m.R,&m.G,&m.B);
        int ag=0; { float mm[3]={m.R,m.G,m.B};
            for(int i=0;i<3;i++) if((double)mm[i]>0.00313080495356037151702786377709) ag|=1<<i; }
        cb_rgb a=cbApplyGammaRGB(m);
        int den=0,p=1; { float aa[3]={a.R,a.G,a.B};
            for(int i=0;i<3;i++){ float v=aa[i]*255.f+0.5f;
                den += ((v<0.f)?0:((v>=256.f)?2:1))*p; p*=3; } }
        int k=rg|(ag<<3)|(den<<6);
        long stride = cnt[k]/MAXW; if(stride<1) stride=1;
        long idx = seen[k]++;
        /* always keep the very first witness, then every `stride`-th */
        if(nw[k]<MAXW && (idx==0 || idx%stride==0))
            w[k][nw[k]++] = (r<<16)|(g<<8)|b;
    }
    for(int k=0;k<NSIG;k++) if(cnt[k]){
        int rg=k&7, ag=(k>>3)&7, den=k>>6;
        printf("SIG %d%d%d %d%d%d %d%d%d %ld %d\n",
            (rg>>0)&1,(rg>>1)&1,(rg>>2)&1,
            (ag>>0)&1,(ag>>1)&1,(ag>>2)&1,
            den%3,(den/3)%3,(den/9)%3, cnt[k], nw[k]);
        for(int i=0;i<nw[k];i++)
            printf("W %d %d %d\n",(w[k][i]>>16)&255,(w[k][i]>>8)&255,w[k][i]&255);
    }
    return 0;
}
