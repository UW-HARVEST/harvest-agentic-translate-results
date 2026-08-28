#include <stdio.h>
#include "lib.c"

int main(void){
    long rg_pow=0, rg_lin=0, ag_pow=0, ag_lin=0;
    long den_neg=0, den_over=0, den_in=0;
    float dmin=1e30f, dmax=-1e30f;
    for(int r=0;r<256;r++)for(int g=0;g<256;g++)for(int b=0;b<256;b++){
        cb_rgb_255 in={(unsigned char)r,(unsigned char)g,(unsigned char)b};
        cb_rgb n=cbNorm(in);
        float ch[3]={n.R,n.G,n.B};
        for(int i=0;i<3;i++){ if((double)ch[i]>0.04045) rg_pow++; else rg_lin++; }
        cb_rgb m=cbRemoveGammaRGB(n);
        Tritanopia(&m.R,&m.G,&m.B);
        float mm[3]={m.R,m.G,m.B};
        for(int i=0;i<3;i++){ if((double)mm[i]>0.00313080495356037151702786377709) ag_pow++; else ag_lin++; }
        cb_rgb a=cbApplyGammaRGB(m);
        float aa[3]={a.R,a.G,a.B};
        for(int i=0;i<3;i++){
            float v=aa[i]*255.f+0.5f;
            if(v<dmin)dmin=v; if(v>dmax)dmax=v;
            if(v<0.f) den_neg++; else if(v>=256.f) den_over++; else den_in++;
        }
    }
    printf("removeGamma: pow=%ld lin=%ld\n", rg_pow, rg_lin);
    printf("applyGamma : pow=%ld lin=%ld\n", ag_pow, ag_lin);
    printf("cbDenorm arg range: [%.9g, %.9g]\n", dmin, dmax);
    printf("cbDenorm buckets: neg=%ld in[0,256)=%ld >=256=%ld\n", den_neg, den_in, den_over);
    return 0;
}
