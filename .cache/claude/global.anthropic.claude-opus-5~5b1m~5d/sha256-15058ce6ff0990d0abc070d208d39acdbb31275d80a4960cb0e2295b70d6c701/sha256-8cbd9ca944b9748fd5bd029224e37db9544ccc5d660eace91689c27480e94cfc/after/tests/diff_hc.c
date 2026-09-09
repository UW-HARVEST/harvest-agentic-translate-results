/* Differential tester for HC + streaming APIs. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdint.h>

static void *hC, *hR;
static int fails = 0, checks = 0;

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
static uint64_t rs = 88172645463325252ULL;
static uint64_t rnd(void){ rs ^= rs<<13; rs ^= rs>>7; rs ^= rs<<17; return rs; }
static void fill(unsigned char *b, size_t n, int mode) {
    size_t i;
    switch (mode) {
    case 0: for (i=0;i<n;i++) b[i] = (unsigned char)rnd(); break;
    case 1: for (i=0;i<n;i++) b[i] = 'a'; break;
    case 2: for (i=0;i<n;i++) b[i] = (unsigned char)('a' + (i%7)); break;
    case 3: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%4) ? 'x' : (unsigned char)rnd()); break;
    case 4: for (i=0;i<n;i++) b[i] = (unsigned char)(i & 0xff); break;
    case 5: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%16)+'A'); break;
    case 6: for (i=0;i<n;i++) b[i] = (unsigned char)((i/64) % 3 ? 'q' : (unsigned char)rnd()); break;
    default: memset(b, 0, n); break;
    }
}

typedef int (*fn_hc)(const char*, char*, int, int, int);
typedef int (*fn_i0)(void);
typedef int (*fn_hcext)(void*, const char*, char*, int, int, int);
typedef int (*fn_hcds)(void*, const char*, char*, int*, int, int);
typedef void* (*fn_pv)(void);
typedef int (*fn_free)(void*);
typedef void (*fn_reset)(void*, int);
typedef int (*fn_loadd)(void*, const char*, int);
typedef int (*fn_cont)(void*, const char*, char*, int, int);
typedef int (*fn_contds)(void*, const char*, char*, int*, int);
typedef int (*fn_saved)(void*, char*, int);
typedef void (*fn_attach)(void*, const void*);
typedef int (*fn_ds)(const char*, char*, int, int);
typedef int (*fn_dsud)(const char*, char*, int, int, const char*, int);
typedef int (*fn_setsd)(void*, const char*, int);
typedef int (*fn_dsc)(void*, const char*, char*, int, int);
typedef int (*fn_cfc)(void*, const char*, char*, int, int, int);
typedef int (*fn_ld)(void*, const char*, int);
typedef int (*fn_sd)(void*, char*, int);

int main(void) {
    hC = dlopen("./cbuild/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    hR = dlopen("./translation/target/release/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    if (!hC || !hR) { printf("dlopen failed\n"); return 2; }

    fn_i0 sszC = getsym(hC,"LZ4_sizeofStateHC"), sszR = getsym(hR,"LZ4_sizeofStateHC");
    fn_i0 ssC = getsym(hC,"LZ4_sizeofState"), ssR = getsym(hR,"LZ4_sizeofState");
    if (sszC()!=sszR()) { printf("sizeofStateHC %d vs %d\n", sszC(), sszR()); fails++; }
    if (ssC()!=ssR())   { printf("sizeofState %d vs %d\n", ssC(), ssR()); fails++; }
    printf("sizeofStateHC=%d sizeofState=%d\n", sszC(), ssC());

    fn_hc hcC = getsym(hC,"LZ4_compress_HC"), hcR = getsym(hR,"LZ4_compress_HC");
    fn_hcext heC = getsym(hC,"LZ4_compress_HC_extStateHC"), heR = getsym(hR,"LZ4_compress_HC_extStateHC");
    fn_hcext hefC = getsym(hC,"LZ4_compress_HC_extStateHC_fastReset"), hefR = getsym(hR,"LZ4_compress_HC_extStateHC_fastReset");
    fn_hcds hdC = getsym(hC,"LZ4_compress_HC_destSize"), hdR = getsym(hR,"LZ4_compress_HC_destSize");
    fn_ds dsC = getsym(hC,"LZ4_decompress_safe"), dsR = getsym(hR,"LZ4_decompress_safe");
    fn_pv csC = getsym(hC,"LZ4_createStreamHC"), csR = getsym(hR,"LZ4_createStreamHC");
    fn_free fsC = getsym(hC,"LZ4_freeStreamHC"), fsR = getsym(hR,"LZ4_freeStreamHC");
    fn_reset rsC = getsym(hC,"LZ4_resetStreamHC_fast"), rsR = getsym(hR,"LZ4_resetStreamHC_fast");
    fn_reset rs2C = getsym(hC,"LZ4_resetStreamHC"), rs2R = getsym(hR,"LZ4_resetStreamHC");
    fn_loadd ldC = getsym(hC,"LZ4_loadDictHC"), ldR = getsym(hR,"LZ4_loadDictHC");
    fn_cont ctC = getsym(hC,"LZ4_compress_HC_continue"), ctR = getsym(hR,"LZ4_compress_HC_continue");
    fn_contds cdC = getsym(hC,"LZ4_compress_HC_continue_destSize"), cdR = getsym(hR,"LZ4_compress_HC_continue_destSize");
    fn_saved svC = getsym(hC,"LZ4_saveDictHC"), svR = getsym(hR,"LZ4_saveDictHC");
    fn_attach atC = getsym(hC,"LZ4_attach_HC_dictionary"), atR = getsym(hR,"LZ4_attach_HC_dictionary");
    fn_reset fdC = getsym(hC,"LZ4_favorDecompressionSpeed"), fdR = getsym(hR,"LZ4_favorDecompressionSpeed");
    fn_reset slC = getsym(hC,"LZ4_setCompressionLevel"), slR = getsym(hR,"LZ4_setCompressionLevel");

    size_t maxN = 300000;
    unsigned char *src = malloc(maxN);
    size_t cap = maxN + maxN/255 + 64;
    char *dC = malloc(cap), *dR = malloc(cap);
    unsigned char *oC = malloc(maxN+64), *oR = malloc(maxN+64);
    void *stC = malloc(sszC()), *stR = malloc(sszR());

    static const int sizes[] = {0,1,4,12,13,14,20,63,64,65,100,255,256,1000,1024,4095,4096,4097,
        16384,65535,65536,65537,100000,200000,300000};
    int nsizes = (int)(sizeof(sizes)/sizeof(sizes[0]));
    int mode, si, lvl;

    for (mode=0; mode<8; mode++) {
      for (si=0; si<nsizes; si++) {
        int n = sizes[si];
        fill(src, n, mode);
        for (lvl=-1; lvl<=14; lvl++) {
            int rc = hcC((const char*)src, dC, n, (int)cap, lvl);
            int rr = hcR((const char*)src, dR, n, (int)cap, lvl);
            cmp("compress_HC", rc, rr, dC, dR, rc>0?rc:0);
            if (rc>0) {
                int uc = dsC(dC, (char*)oC, rc, n+64);
                if (uc != n || (n && memcmp(oC, src, n))) { printf("HC roundtrip fail lvl=%d mode=%d n=%d uc=%d\n", lvl,mode,n,uc); fails++; }
            }
            /* extState */
            rc = heC(stC, (const char*)src, dC, n, (int)cap, lvl);
            rr = heR(stR, (const char*)src, dR, n, (int)cap, lvl);
            cmp("HC_extStateHC", rc, rr, dC, dR, rc>0?rc:0);
            /* fastReset (state already initialized by previous call) */
            rc = hefC(stC, (const char*)src, dC, n, (int)cap, lvl);
            rr = hefR(stR, (const char*)src, dR, n, (int)cap, lvl);
            cmp("HC_extStateHC_fastReset", rc, rr, dC, dR, rc>0?rc:0);
            /* limited output */
            int k;
            for (k=0;k<5;k++) {
                int lim = (k==0)?0:(k==1)?1:(k==2)?(n/4+1):(k==3)?(n/2+1):(rc>0?rc:1);
                memset(dC,0xAA,cap); memset(dR,0xAA,cap);
                int a = heC(stC,(const char*)src, dC, n, lim, lvl);
                int b = heR(stR,(const char*)src, dR, n, lim, lvl);
                cmp("HC_limited", a, b, dC, dR, a>0?a:0);
            }
            /* destSize */
            for (k=0;k<6;k++) {
                int tds = (k==0)?0:(k==1)?1:(k==2)?13:(k==3)?(n/4+3):(k==4)?(n/2+3):(n+20);
                int scC=n, scR=n;
                memset(dC,0xAA,cap); memset(dR,0xAA,cap);
                int a = hdC(stC,(const char*)src, dC, &scC, tds, lvl);
                int b = hdR(stR,(const char*)src, dR, &scR, tds, lvl);
                cmp("HC_destSize", a, b, dC, dR, a>0?a:0);
                if (scC!=scR) { printf("HC destSize consumed %d vs %d lvl=%d n=%d tds=%d\n", scC,scR,lvl,n,tds); fails++; }
            }
        }
      }
    }

    /* HC streaming */
    {
        void *sC = csC(), *sR = csR();
        int lvl;
        for (lvl=2; lvl<=12; lvl++) {
          for (mode=0; mode<8; mode++) {
            int blk;
            int n = 200000;
            fill(src, n, mode);
            rs2C(sC, lvl); rs2R(sR, lvl);
            /* load dict */
            ldC(sC, (const char*)src, 40000); ldR(sR, (const char*)src, 40000);
            int off = 40000;
            for (blk=0; blk<8 && off < n; blk++) {
                int bs = 5000 + blk*3000;
                if (off + bs > n) bs = n - off;
                int a = ctC(sC, (const char*)src+off, dC, bs, (int)cap);
                int b = ctR(sR, (const char*)src+off, dR, bs, (int)cap);
                cmp("HC_continue", a, b, dC, dR, a>0?a:0);
                off += bs;
            }
            /* destSize continue */
            rs2C(sC, lvl); rs2R(sR, lvl);
            off = 0;
            for (blk=0; blk<6 && off < n; blk++) {
                int sc1 = 20000, sc2 = 20000;
                if (off + 20000 > n) { sc1 = sc2 = n-off; }
                int a = cdC(sC, (const char*)src+off, dC, &sc1, 3000+blk*700);
                int b = cdR(sR, (const char*)src+off, dR, &sc2, 3000+blk*700);
                cmp("HC_continue_destSize", a, b, dC, dR, a>0?a:0);
                if (sc1!=sc2) { printf("HC cds consumed %d vs %d\n", sc1, sc2); fails++; }
                off += sc1 > 0 ? sc1 : 1;
            }
            /* saveDict */
            {
                char sbC[70000], sbR[70000];
                rs2C(sC, lvl); rs2R(sR, lvl);
                ctC(sC, (const char*)src, dC, 100000, (int)cap);
                ctR(sR, (const char*)src, dR, 100000, (int)cap);
                int a = svC(sC, sbC, 65536);
                int b = svR(sR, sbR, 65536);
                if (a!=b) { printf("saveDictHC %d vs %d\n", a, b); fails++; }
                else if (a>0 && memcmp(sbC, sbR, a)) { printf("saveDictHC content\n"); fails++; }
                int x = ctC(sC, (const char*)src+100000, dC, 50000, (int)cap);
                int y = ctR(sR, (const char*)src+100000, dR, 50000, (int)cap);
                cmp("HC_continue_afterSave", x, y, dC, dR, x>0?x:0);
            }
            /* favorDecSpeed */
            fdC(sC, 1); fdR(sR, 1);
            rs2C(sC, lvl); rs2R(sR, lvl);
            fdC(sC, 1); fdR(sR, 1);
            {
                int a = ctC(sC, (const char*)src, dC, 100000, (int)cap);
                int b = ctR(sR, (const char*)src, dR, 100000, (int)cap);
                cmp("HC_favorDec", a, b, dC, dR, a>0?a:0);
            }
            fdC(sC, 0); fdR(sR, 0);
          }
        }
        /* attach dictionary */
        {
            void *dictC = csC(), *dictR = csR();
            int lvl2;
            for (lvl2=2; lvl2<=12; lvl2++) {
              for (mode=0;mode<8;mode++) {
                int n = 120000; fill(src, n, mode);
                rs2C(dictC, lvl2); rs2R(dictR, lvl2);
                ldC(dictC, (const char*)src, 60000); ldR(dictR, (const char*)src, 60000);
                rs2C(sC, lvl2); rs2R(sR, lvl2);
                atC(sC, dictC); atR(sR, dictR);
                int a = ctC(sC, (const char*)src+60000, dC, 3000, (int)cap);
                int b = ctR(sR, (const char*)src+60000, dR, 3000, (int)cap);
                cmp("HC_attachDict_small", a, b, dC, dR, a>0?a:0);
                rs2C(sC, lvl2); rs2R(sR, lvl2);
                atC(sC, dictC); atR(sR, dictR);
                a = ctC(sC, (const char*)src+60000, dC, 60000, (int)cap);
                b = ctR(sR, (const char*)src+60000, dR, 60000, (int)cap);
                cmp("HC_attachDict_big", a, b, dC, dR, a>0?a:0);
              }
            }
            fsC(dictC); fsR(dictR);
        }
        fsC(sC); fsR(sR);
    }

    printf("checks=%d fails=%d\n", checks, fails);
    return fails ? 1 : 0;
}
