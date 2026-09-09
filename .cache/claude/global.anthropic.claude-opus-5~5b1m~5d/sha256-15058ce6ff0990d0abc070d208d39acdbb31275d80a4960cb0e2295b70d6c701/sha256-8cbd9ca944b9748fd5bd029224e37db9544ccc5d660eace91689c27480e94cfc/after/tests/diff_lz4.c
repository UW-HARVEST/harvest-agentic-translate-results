/* Differential tester: dlopen C liblz4.so and Rust liblz4.so, compare outputs. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdint.h>

static void *hC, *hR;
static int fails = 0;
static int checks = 0;

#define SYM(h, name) dlsym(h, name)

static void *getsym(void *h, const char *n) {
    void *p = dlsym(h, n);
    if (!p) { fprintf(stderr, "missing symbol %s\n", n); exit(2); }
    return p;
}

static void cmp(const char *what, int rc, int rr, const void *bc, const void *br, size_t n) {
    checks++;
    if (rc != rr) { printf("MISMATCH %s: rc=%d rr=%d\n", what, rc, rr); fails++; return; }
    if (rc > 0 && n && memcmp(bc, br, n) != 0) {
        size_t i; for (i=0;i<n;i++) if (((unsigned char*)bc)[i]!=((unsigned char*)br)[i]) break;
        printf("MISMATCH %s: content differs at %zu (rc=%d)\n", what, i, rc); fails++;
    }
}

static uint64_t rng_state = 88172645463325252ULL;
static uint64_t rnd(void){ rng_state ^= rng_state<<13; rng_state ^= rng_state>>7; rng_state ^= rng_state<<17; return rng_state; }

static void fill(unsigned char *b, size_t n, int mode) {
    size_t i;
    switch (mode) {
    case 0: for (i=0;i<n;i++) b[i] = (unsigned char)rnd(); break;               /* incompressible */
    case 1: for (i=0;i<n;i++) b[i] = 'a'; break;                                /* all same */
    case 2: for (i=0;i<n;i++) b[i] = (unsigned char)('a' + (i%7)); break;        /* periodic */
    case 3: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%4) ? 'x' : (unsigned char)rnd()); break;
    case 4: for (i=0;i<n;i++) b[i] = (unsigned char)(i & 0xff); break;
    case 5: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%16)+'A'); break;
    default: memset(b, 0, n); break;
    }
}

int main(void) {
    hC = dlopen("./cbuild/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    if (!hC) { printf("dlopen C failed: %s\n", dlerror()); return 2; }
    hR = dlopen("./translation/target/release/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    if (!hR) { printf("dlopen R failed: %s\n", dlerror()); return 2; }

    typedef int (*fn_cd)(const char*, char*, int, int);
    typedef int (*fn_cf)(const char*, char*, int, int, int);
    typedef int (*fn_ds)(const char*, char*, int, int);
    typedef int (*fn_dsp)(const char*, char*, int, int, int);
    typedef int (*fn_cds)(const char*, char*, int*, int);
    typedef int (*fn_cbound)(int);
    typedef int (*fn_versionNumber)(void);
    typedef const char* (*fn_versionString)(void);
    typedef int (*fn_hc)(const char*, char*, int, int, int);

    fn_cd cdC = getsym(hC,"LZ4_compress_default"), cdR = getsym(hR,"LZ4_compress_default");
    fn_cf cfC = getsym(hC,"LZ4_compress_fast"),    cfR = getsym(hR,"LZ4_compress_fast");
    fn_ds dsC = getsym(hC,"LZ4_decompress_safe"),  dsR = getsym(hR,"LZ4_decompress_safe");
    fn_dsp dspC = getsym(hC,"LZ4_decompress_safe_partial"), dspR = getsym(hR,"LZ4_decompress_safe_partial");
    fn_cds cdsC = getsym(hC,"LZ4_compress_destSize"), cdsR = getsym(hR,"LZ4_compress_destSize");
    fn_cbound cbC = getsym(hC,"LZ4_compressBound"), cbR = getsym(hR,"LZ4_compressBound");
    fn_versionNumber vnC = getsym(hC,"LZ4_versionNumber"), vnR = getsym(hR,"LZ4_versionNumber");
    fn_versionString vsC = getsym(hC,"LZ4_versionString"), vsR = getsym(hR,"LZ4_versionString");

    if (vnC() != vnR()) { printf("versionNumber mismatch\n"); fails++; }
    if (strcmp(vsC(), vsR()) != 0) { printf("versionString mismatch: %s vs %s\n", vsC(), vsR()); fails++; }
    { int i; for (i=-5;i<70000;i+= (i<100?1:997)) if (cbC(i)!=cbR(i)) { printf("compressBound(%d) mismatch\n", i); fails++; } }

    size_t maxN = 400000;
    unsigned char *src = malloc(maxN);
    size_t cap = cbC((int)maxN) + 64;
    char *dC = malloc(cap), *dR = malloc(cap);
    unsigned char *oC = malloc(maxN+64), *oR = malloc(maxN+64);

    static const int sizes[] = {0,1,2,3,4,5,6,7,8,11,12,13,14,15,16,17,18,19,20,31,32,33,63,64,65,
        100,127,128,129,255,256,257,511,512,513,1000,1023,1024,1025,4095,4096,4097,
        16000,65530,65535,65536,65537,66000,70000,100000,200000,400000};
    int nsizes = (int)(sizeof(sizes)/sizeof(sizes[0]));
    int mode, si, acc;

    for (mode=0; mode<7; mode++) {
      for (si=0; si<nsizes; si++) {
        int n = sizes[si];
        if ((size_t)n > maxN) continue;
        fill(src, n, mode);
        /* compress_default with full capacity */
        int rc = cdC((const char*)src, dC, n, (int)cap);
        int rr = cdR((const char*)src, dR, n, (int)cap);
        cmp("compress_default", rc, rr, dC, dR, rc>0?rc:0);

        /* decompress the C output with both */
        if (rc > 0) {
            int uc = dsC(dC, (char*)oC, rc, n+64);
            int ur = dsR(dC, (char*)oR, rc, n+64);
            cmp("decompress_safe", uc, ur, oC, oR, uc>0?uc:0);
            if (uc != n) { printf("roundtrip size wrong: %d vs %d\n", uc, n); fails++; }
            if (uc>0 && memcmp(oC, src, uc)!=0) { printf("roundtrip content wrong mode=%d n=%d\n",mode,n); fails++; }
            /* exact dst capacity */
            uc = dsC(dC, (char*)oC, rc, n);
            ur = dsR(dC, (char*)oR, rc, n);
            cmp("decompress_safe_exact", uc, ur, oC, oR, uc>0?uc:0);
            /* undersized */
            if (n > 3) {
                uc = dsC(dC, (char*)oC, rc, n/2);
                ur = dsR(dC, (char*)oR, rc, n/2);
                cmp("decompress_safe_small", uc, ur, oC, oR, uc>0?uc:0);
            }
            /* truncated input */
            uc = dsC(dC, (char*)oC, rc/2, n+64);
            ur = dsR(dC, (char*)oR, rc/2, n+64);
            cmp("decompress_safe_trunc", uc, ur, oC, oR, uc>0?uc:0);
            /* partial */
            int t;
            for (t=0; t<4; t++) {
                int tos = (t==0)?0:(t==1)?1:(t==2)?n/3:n;
                uc = dspC(dC, (char*)oC, rc, tos, n+64);
                ur = dspR(dC, (char*)oR, rc, tos, n+64);
                cmp("decompress_safe_partial", uc, ur, oC, oR, uc>0?uc:0);
            }
        }

        /* limited output at various capacities */
        int k;
        for (k=0;k<6;k++) {
            int lim = (k==0)?0:(k==1)?1:(k==2)?2:(k==3)?(n/4+1):(k==4)?(n/2+1):(rc>0?rc:1);
            memset(dC,0xAA,cap); memset(dR,0xAA,cap);
            rc = cdC((const char*)src, dC, n, lim);
            rr = cdR((const char*)src, dR, n, lim);
            cmp("compress_limited", rc, rr, dC, dR, rc>0?rc:0);
        }

        /* acceleration variants */
        for (acc=-3; acc<=20; acc+=(acc<4?1:5)) {
            rc = cfC((const char*)src, dC, n, (int)cap, acc);
            rr = cfR((const char*)src, dR, n, (int)cap, acc);
            cmp("compress_fast", rc, rr, dC, dR, rc>0?rc:0);
        }
        { rc = cfC((const char*)src, dC, n, (int)cap, 70000);
          rr = cfR((const char*)src, dR, n, (int)cap, 70000);
          cmp("compress_fast_bigacc", rc, rr, dC, dR, rc>0?rc:0); }

        /* destSize */
        for (k=0;k<7;k++) {
            int tds = (k==0)?0:(k==1)?1:(k==2)?5:(k==3)?17:(k==4)?(n/4+3):(k==5)?(n/2+3):(n+20);
            int scC = n, scR = n;
            memset(dC,0xAA,cap); memset(dR,0xAA,cap);
            rc = cdsC((const char*)src, dC, &scC, tds);
            rr = cdsR((const char*)src, dR, &scR, tds);
            cmp("compress_destSize", rc, rr, dC, dR, rc>0?rc:0);
            if (scC != scR) { printf("destSize srcConsumed mismatch %d vs %d (mode=%d n=%d tds=%d)\n", scC, scR, mode, n, tds); fails++; }
        }
      }
    }

    printf("checks=%d fails=%d\n", checks, fails);
    return fails ? 1 : 0;
}
