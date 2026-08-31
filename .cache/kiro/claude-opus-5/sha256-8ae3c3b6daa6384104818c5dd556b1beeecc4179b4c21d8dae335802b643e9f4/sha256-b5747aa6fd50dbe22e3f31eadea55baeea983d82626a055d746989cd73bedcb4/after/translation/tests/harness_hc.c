/* Differential harness for the LZ4 HC API. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *H;
#define LOAD(var, name) do { *(void**)&var = dlsym(H, name); if(!var){fprintf(stderr,"missing %s\n",name);exit(2);} } while(0)

static unsigned long long seed = 0x2545F4914F6CDD1DULL;
static unsigned rnd(void) { seed ^= seed<<13; seed ^= seed>>7; seed ^= seed<<17; return (unsigned)(seed>>16); }

static void hp(const char *tag, const void *p, int n) {
    unsigned h = 2166136261u; const unsigned char *b = p; int i;
    if (n < 0) { printf("%s len=%d\n", tag, n); return; }
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 16777619u; }
    printf("%s len=%d hash=%08x\n", tag, n, h);
}

int main(int argc, char **argv) {
    int (*compressBound)(int);
    int (*decompress_safe)(const char*,char*,int,int);
    int (*compress_HC)(const char*,char*,int,int,int);
    int (*sizeofStateHC)(void);
    int (*compress_HC_extStateHC)(void*,const char*,char*,int,int,int);
    int (*compress_HC_extStateHC_fastReset)(void*,const char*,char*,int,int,int);
    int (*compress_HC_destSize)(void*,const char*,char*,int*,int,int);
    void* (*createStreamHC)(void);
    int (*freeStreamHC)(void*);
    void* (*initStreamHC)(void*,size_t);
    void (*resetStreamHC)(void*,int);
    void (*resetStreamHC_fast)(void*,int);
    void (*setCompressionLevel)(void*,int);
    void (*favorDecompressionSpeed)(void*,int);
    int (*loadDictHC)(void*,const char*,int);
    void (*attach_HC_dictionary)(void*,const void*);
    int (*compress_HC_continue)(void*,const char*,char*,int,int);
    int (*compress_HC_continue_destSize)(void*,const char*,char*,int*,int);
    int (*saveDictHC)(void*,char*,int);
    int (*decompress_safe_usingDict)(const char*,char*,int,int,const char*,int);
    int (*compressHC)(const char*,char*,int);
    int (*compressHC2)(const char*,char*,int,int);
    int (*sizeofStreamStateHC)(void);
    int (*resetStreamStateHC)(void*,char*);
    void* (*createHC)(const char*);
    int (*freeHC)(void*);
    int (*compressHC2_continue)(void*,const char*,char*,int,int);
    char* (*slideInputBufferHC)(void*);

    if (argc < 2) { fprintf(stderr, "usage: %s <lib>\n", argv[0]); return 1; }
    H = dlopen(argv[1], RTLD_NOW);
    if (!H) { fprintf(stderr, "%s\n", dlerror()); return 1; }

    LOAD(compressBound, "LZ4_compressBound");
    LOAD(decompress_safe, "LZ4_decompress_safe");
    LOAD(compress_HC, "LZ4_compress_HC");
    LOAD(sizeofStateHC, "LZ4_sizeofStateHC");
    LOAD(compress_HC_extStateHC, "LZ4_compress_HC_extStateHC");
    LOAD(compress_HC_extStateHC_fastReset, "LZ4_compress_HC_extStateHC_fastReset");
    LOAD(compress_HC_destSize, "LZ4_compress_HC_destSize");
    LOAD(createStreamHC, "LZ4_createStreamHC");
    LOAD(freeStreamHC, "LZ4_freeStreamHC");
    LOAD(initStreamHC, "LZ4_initStreamHC");
    LOAD(resetStreamHC, "LZ4_resetStreamHC");
    LOAD(resetStreamHC_fast, "LZ4_resetStreamHC_fast");
    LOAD(setCompressionLevel, "LZ4_setCompressionLevel");
    LOAD(favorDecompressionSpeed, "LZ4_favorDecompressionSpeed");
    LOAD(loadDictHC, "LZ4_loadDictHC");
    LOAD(attach_HC_dictionary, "LZ4_attach_HC_dictionary");
    LOAD(compress_HC_continue, "LZ4_compress_HC_continue");
    LOAD(compress_HC_continue_destSize, "LZ4_compress_HC_continue_destSize");
    LOAD(saveDictHC, "LZ4_saveDictHC");
    LOAD(decompress_safe_usingDict, "LZ4_decompress_safe_usingDict");
    LOAD(compressHC, "LZ4_compressHC");
    LOAD(compressHC2, "LZ4_compressHC2");
    LOAD(sizeofStreamStateHC, "LZ4_sizeofStreamStateHC");
    LOAD(resetStreamStateHC, "LZ4_resetStreamStateHC");
    LOAD(createHC, "LZ4_createHC");
    LOAD(freeHC, "LZ4_freeHC");
    LOAD(compressHC2_continue, "LZ4_compressHC2_continue");
    LOAD(slideInputBufferHC, "LZ4_slideInputBufferHC");

    printf("sizeofStateHC=%d sizeofStreamStateHC=%d\n", sizeofStateHC(), sizeofStreamStateHC());

    {
        int sizes[] = {0,1,4,12,13,20,63,100,255,1000,4095,4096,5000,20000,65535,65536,70000,200000};
        int nsizes = (int)(sizeof(sizes)/sizeof(sizes[0]));
        int si, mode, lvl;
        for (mode = 0; mode < 3; mode++) {
            for (si = 0; si < nsizes; si++) {
                int n = sizes[si];
                char *src = malloc(n ? n : 1);
                int cb = compressBound(n);
                char *cmp = malloc(cb ? cb : 1);
                char *dec = malloc(n ? n : 1);
                int i;
                for (i = 0; i < n; i++) {
                    if (mode == 0) src[i] = (char)rnd();
                    else if (mode == 1) src[i] = (char)('a' + (i % 11));
                    else src[i] = (char)((rnd() % 6) ? (i % 233) : rnd());
                }
                for (lvl = -1; lvl <= 13; lvl++) {
                    int csz = compress_HC(src, cmp, n, cb, lvl);
                    printf("lvl=%d n=%d ", lvl, n);
                    hp("hc", cmp, csz);
                    if (csz > 0) {
                        int dsz = decompress_safe(cmp, dec, csz, n);
                        if (dsz != n || (n && memcmp(src, dec, n))) printf("  ROUNDTRIP FAIL %d\n", dsz);
                    }
                    /* limited output budget */
                    if (cb > 4) {
                        int budget = cb / 3 + 1;
                        csz = compress_HC(src, cmp, n, budget, lvl);
                        printf("  budget=%d -> %d\n", budget, csz);
                        if (csz > 0) hp("   b", cmp, csz);
                    }
                    /* extState + fastReset */
                    {
                        void *st = malloc((size_t)sizeofStateHC());
                        csz = compress_HC_extStateHC(st, src, cmp, n, cb, lvl);
                        hp("  es", cmp, csz);
                        csz = compress_HC_extStateHC_fastReset(st, src, cmp, n, cb, lvl);
                        hp("  fr", cmp, csz);
                        free(st);
                    }
                    /* destSize */
                    {
                        void *st = malloc((size_t)sizeofStateHC());
                        int sn = n;
                        int tgt = cb / 4 + 2;
                        csz = compress_HC_destSize(st, src, cmp, &sn, tgt, lvl);
                        printf("  ds tgt=%d consumed=%d out=%d\n", tgt, sn, csz);
                        if (csz > 0) hp("   dsh", cmp, csz);
                        free(st);
                    }
                }
                free(src); free(cmp); free(dec);
            }
        }
    }

    /* streaming HC */
    {
        int total = 200000;
        char *src = malloc(total);
        char *cmp = malloc(compressBound(50000) + 64);
        char *dec = malloc(total + 65536);
        int i, lvl, blk;
        for (i = 0; i < total; i++) src[i] = (char)((rnd() % 7) ? (i % 89) : rnd());

        for (lvl = 2; lvl <= 12; lvl += 2) {
            for (blk = 1234; blk <= 50000; blk *= 6) {
                void *s = createStreamHC();
                int off = 0, dpos = 0;
                resetStreamHC(s, lvl);
                while (off < total) {
                    int n = (off + blk > total) ? (total - off) : blk;
                    int csz = compress_HC_continue(s, src + off, cmp, n, compressBound(n));
                    printf("shc lvl=%d blk=%d ", lvl, blk);
                    hp("c", cmp, csz);
                    if (csz > 0) {
                        int dsz = decompress_safe_usingDict(cmp, dec + dpos, csz, n,
                                                           dpos ? dec : NULL, dpos > 65536 ? 65536 : dpos);
                        if (dsz != n || memcmp(src + off, dec + dpos, n)) printf("  SFAIL %d\n", dsz);
                        dpos += n;
                    }
                    off += n;
                }
                printf("  sv=%d\n", saveDictHC(s, dec + total, 65536));
                freeStreamHC(s);
            }
        }

        /* favorDecompressionSpeed */
        for (lvl = 10; lvl <= 12; lvl++) {
            void *s = createStreamHC();
            int csz;
            resetStreamHC(s, lvl);
            favorDecompressionSpeed(s, 1);
            csz = compress_HC_continue(s, src, cmp, 40000, compressBound(40000));
            printf("favor lvl=%d ", lvl); hp("c", cmp, csz);
            freeStreamHC(s);
        }

        /* dictionary attach */
        for (lvl = 2; lvl <= 12; lvl += 3) {
            void *d = createStreamHC();
            void *w = createStreamHC();
            int csz;
            setCompressionLevel(d, lvl);
            loadDictHC(d, src, 60000);
            resetStreamHC(w, lvl);
            attach_HC_dictionary(w, d);
            csz = compress_HC_continue(w, src + 60000, cmp, 3000, compressBound(3000));
            printf("attach lvl=%d small ", lvl); hp("c", cmp, csz);
            resetStreamHC(w, lvl);
            attach_HC_dictionary(w, d);
            csz = compress_HC_continue(w, src + 60000, cmp, 40000, compressBound(40000));
            printf("attach lvl=%d big ", lvl); hp("c", cmp, csz);
            freeStreamHC(d); freeStreamHC(w);
        }

        /* loadDictHC + continue + destSize */
        for (lvl = 2; lvl <= 12; lvl += 5) {
            void *s = createStreamHC();
            int sn = 30000, csz;
            setCompressionLevel(s, lvl);
            loadDictHC(s, src, 70000);
            csz = compress_HC_continue_destSize(s, src + 70000, cmp, &sn, 4000);
            printf("ldc lvl=%d consumed=%d ", lvl, sn); hp("c", cmp, csz);
            freeStreamHC(s);
        }

        /* obsolete entry points */
        {
            void *st = malloc((size_t)sizeofStreamStateHC());
            int r = resetStreamStateHC(st, src);
            int csz = compressHC2_continue(st, src, cmp, 20000, 6);
            printf("obs reset=%d ", r); hp("c", cmp, csz);
            printf("slide=%d\n", slideInputBufferHC(st) != NULL);
            free(st);
        }
        {
            void *h = createHC(src);
            int csz = compressHC2_continue(h, src, cmp, 15000, 4);
            hp("createHC", cmp, csz);
            freeHC(h);
        }
        hp("compressHC", cmp, compressHC(src, cmp, 10000));
        hp("compressHC2", cmp, compressHC2(src, cmp, 10000, 11));

        free(src); free(cmp); free(dec);
    }

    printf("DONE\n");
    return 0;
}
