/* Test harness: exercises the block-level LZ4 API through dlopen so the same
 * binary can be run against the C reference and the Rust translation. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *H;
#define LOAD(var, name) do { *(void**)&var = dlsym(H, name); if(!var){fprintf(stderr,"missing %s\n",name);exit(2);} } while(0)

static unsigned long long seed = 88172645463325252ULL;
static unsigned rnd(void) { seed ^= seed<<13; seed ^= seed>>7; seed ^= seed<<17; return (unsigned)(seed>>16); }

static void hash_print(const char *tag, const void *p, int n) {
    unsigned h = 2166136261u;
    const unsigned char *b = p;
    int i;
    if (n < 0) { printf("%s len=%d\n", tag, n); return; }
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 16777619u; }
    printf("%s len=%d hash=%08x\n", tag, n, h);
}

int main(int argc, char **argv) {
    int (*versionNumber)(void);
    const char* (*versionString)(void);
    int (*compressBound)(int);
    int (*compress_default)(const char*,char*,int,int);
    int (*compress_fast)(const char*,char*,int,int,int);
    int (*decompress_safe)(const char*,char*,int,int);
    int (*decompress_safe_partial)(const char*,char*,int,int,int);
    int (*compress_destSize)(const char*,char*,int*,int);
    void* (*createStream)(void);
    int (*freeStream)(void*);
    int (*loadDict)(void*,const char*,int);
    int (*loadDictSlow)(void*,const char*,int);
    int (*compress_fast_continue)(void*,const char*,char*,int,int,int);
    int (*saveDict)(void*,char*,int);
    void* (*createStreamDecode)(void);
    int (*freeStreamDecode)(void*);
    int (*setStreamDecode)(void*,const char*,int);
    int (*decompress_safe_continue)(void*,const char*,char*,int,int);
    int (*decompress_safe_usingDict)(const char*,char*,int,int,const char*,int);
    int (*sizeofState)(void);
    int (*compress_fast_extState)(void*,const char*,char*,int,int,int);
    unsigned (*XXH32)(const void*,size_t,unsigned);
    unsigned long long (*XXH64)(const void*,size_t,unsigned long long);
    void* (*XXH32_createState)(void);
    int (*XXH32_reset)(void*,unsigned);
    int (*XXH32_update)(void*,const void*,size_t);
    unsigned (*XXH32_digest)(const void*);
    int (*XXH32_freeState)(void*);
    void* (*XXH64_createState)(void);
    int (*XXH64_reset)(void*,unsigned long long);
    int (*XXH64_update)(void*,const void*,size_t);
    unsigned long long (*XXH64_digest)(const void*);
    int (*XXH64_freeState)(void*);
    int (*decoderRingBufferSize)(int);

    if (argc < 2) { fprintf(stderr, "usage: %s <lib>\n", argv[0]); return 1; }
    H = dlopen(argv[1], RTLD_NOW);
    if (!H) { fprintf(stderr, "%s\n", dlerror()); return 1; }

    LOAD(versionNumber, "LZ4_versionNumber");
    LOAD(versionString, "LZ4_versionString");
    LOAD(compressBound, "LZ4_compressBound");
    LOAD(compress_default, "LZ4_compress_default");
    LOAD(compress_fast, "LZ4_compress_fast");
    LOAD(decompress_safe, "LZ4_decompress_safe");
    LOAD(decompress_safe_partial, "LZ4_decompress_safe_partial");
    LOAD(compress_destSize, "LZ4_compress_destSize");
    LOAD(createStream, "LZ4_createStream");
    LOAD(freeStream, "LZ4_freeStream");
    LOAD(loadDict, "LZ4_loadDict");
    LOAD(loadDictSlow, "LZ4_loadDictSlow");
    LOAD(compress_fast_continue, "LZ4_compress_fast_continue");
    LOAD(saveDict, "LZ4_saveDict");
    LOAD(createStreamDecode, "LZ4_createStreamDecode");
    LOAD(freeStreamDecode, "LZ4_freeStreamDecode");
    LOAD(setStreamDecode, "LZ4_setStreamDecode");
    LOAD(decompress_safe_continue, "LZ4_decompress_safe_continue");
    LOAD(decompress_safe_usingDict, "LZ4_decompress_safe_usingDict");
    LOAD(sizeofState, "LZ4_sizeofState");
    LOAD(compress_fast_extState, "LZ4_compress_fast_extState");
    LOAD(XXH32, "LZ4_XXH32");
    LOAD(XXH64, "LZ4_XXH64");
    LOAD(XXH32_createState, "LZ4_XXH32_createState");
    LOAD(XXH32_reset, "LZ4_XXH32_reset");
    LOAD(XXH32_update, "LZ4_XXH32_update");
    LOAD(XXH32_digest, "LZ4_XXH32_digest");
    LOAD(XXH32_freeState, "LZ4_XXH32_freeState");
    LOAD(XXH64_createState, "LZ4_XXH64_createState");
    LOAD(XXH64_reset, "LZ4_XXH64_reset");
    LOAD(XXH64_update, "LZ4_XXH64_update");
    LOAD(XXH64_digest, "LZ4_XXH64_digest");
    LOAD(XXH64_freeState, "LZ4_XXH64_freeState");
    LOAD(decoderRingBufferSize, "LZ4_decoderRingBufferSize");

    printf("version=%d %s bound(1000)=%d sizeofState=%d ring=%d\n",
           versionNumber(), versionString(), compressBound(1000),
           sizeofState(), decoderRingBufferSize(4096));

    /* xxhash coverage over many lengths */
    {
        unsigned char buf[600];
        int i;
        for (i = 0; i < (int)sizeof(buf); i++) buf[i] = (unsigned char)rnd();
        for (i = 0; i <= 300; i++) {
            printf("xxh %d %08x %016llx\n", i, XXH32(buf, i, 0x9E3779B1u),
                   (unsigned long long)XXH64(buf, i, 0x123456789ABCDEF0ULL));
        }
        /* streaming, irregular chunk sizes */
        {
            void *s32 = XXH32_createState();
            void *s64 = XXH64_createState();
            int off = 0;
            XXH32_reset(s32, 7); XXH64_reset(s64, 7);
            while (off < (int)sizeof(buf)) {
                int n = (int)(rnd() % 37);
                if (off + n > (int)sizeof(buf)) n = (int)sizeof(buf) - off;
                XXH32_update(s32, buf+off, n);
                XXH64_update(s64, buf+off, n);
                off += n;
                if (n == 0 && off == 0) break;
                printf("stream %d %08x %016llx\n", off, XXH32_digest(s32),
                       (unsigned long long)XXH64_digest(s64));
            }
            XXH32_freeState(s32); XXH64_freeState(s64);
        }
    }

    /* block compression over a variety of inputs */
    {
        int sizes[] = {0,1,2,3,4,5,12,13,14,15,16,17,31,63,64,65,100,255,256,
                       1000,4095,4096,4097,20000,65535,65536,65537,100000,300000};
        int nsizes = (int)(sizeof(sizes)/sizeof(sizes[0]));
        int si, mode;
        for (mode = 0; mode < 3; mode++) {
            for (si = 0; si < nsizes; si++) {
                int n = sizes[si];
                char *src = malloc(n ? n : 1);
                int cb = compressBound(n);
                char *cmp = malloc(cb ? cb : 1);
                char *dec = malloc(n ? n : 1);
                int i, csz, dsz;
                for (i = 0; i < n; i++) {
                    if (mode == 0) src[i] = (char)rnd();                 /* random */
                    else if (mode == 1) src[i] = (char)('a' + (i % 7));  /* periodic */
                    else src[i] = (char)((rnd() % 4) ? (i % 251) : rnd());
                }
                csz = compress_default(src, cmp, n, cb);
                hash_print("cd", cmp, csz);
                if (csz > 0) {
                    dsz = decompress_safe(cmp, dec, csz, n);
                    printf("  dec=%d match=%d\n", dsz, (dsz == n && (n == 0 || !memcmp(src, dec, n))));
                    dsz = decompress_safe_partial(cmp, dec, csz, n/2, n);
                    printf("  part=%d\n", dsz);
                }
                /* acceleration variants */
                {
                    int acc;
                    for (acc = -1; acc < 40; acc += 7) {
                        csz = compress_fast(src, cmp, n, cb, acc);
                        hash_print("  cf", cmp, csz);
                    }
                }
                /* tight output budgets */
                {
                    int budget;
                    for (budget = 0; budget <= cb; budget += (cb/7 ? cb/7 : 1)) {
                        csz = compress_default(src, cmp, n, budget);
                        printf("  budget=%d -> %d\n", budget, csz);
                        if (csz > 0) hash_print("   b", cmp, csz);
                    }
                }
                /* destSize */
                {
                    int tgt = cb / 3 + 1;
                    int sn = n;
                    csz = compress_destSize(src, cmp, &sn, tgt);
                    printf("  destSize tgt=%d consumed=%d out=%d\n", tgt, sn, csz);
                    if (csz > 0) hash_print("   ds", cmp, csz);
                }
                /* extState */
                {
                    void *st = malloc((size_t)sizeofState());
                    csz = compress_fast_extState(st, src, cmp, n, cb, 1);
                    hash_print("  es", cmp, csz);
                    free(st);
                }
                free(src); free(cmp); free(dec);
            }
        }
    }

    /* streaming compression with dictionary */
    {
        int total = 200000;
        char *src = malloc(total);
        char *cmp = malloc(compressBound(70000) + 16);
        char *dec = malloc(total + 65536);
        int i, off, blk;
        for (i = 0; i < total; i++) src[i] = (char)((rnd() % 5) ? (i % 97) : rnd());

        for (blk = 1000; blk <= 70000; blk *= 7) {
            void *s = createStream();
            void *sd = createStreamDecode();
            int dpos = 0;
            off = 0;
            while (off < total) {
                int n = (off + blk > total) ? (total - off) : blk;
                int csz = compress_fast_continue(s, src + off, cmp, n, compressBound(n), 1);
                int dsz;
                hash_print("sc", cmp, csz);
                dsz = decompress_safe_continue(sd, cmp, dec + dpos, csz, n);
                printf("  sdec=%d ok=%d\n", dsz, (dsz == n && !memcmp(src + off, dec + dpos, n)));
                dpos += (dsz > 0 ? dsz : 0);
                off += n;
            }
            freeStream(s);
            freeStreamDecode(sd);
        }

        /* explicit dictionary */
        for (i = 0; i < 2; i++) {
            void *s = createStream();
            int dsize = 40000;
            int csz, dsz;
            if (i == 0) loadDict(s, src, dsize); else loadDictSlow(s, src, dsize);
            csz = compress_fast_continue(s, src + dsize, cmp, 30000, compressBound(30000), 1);
            hash_print("dict", cmp, csz);
            dsz = decompress_safe_usingDict(cmp, dec, csz, 30000, src, dsize);
            printf("  dictdec=%d ok=%d\n", dsz, (dsz == 30000 && !memcmp(src + dsize, dec, 30000)));
            {
                char save[65536];
                int sv = saveDict(s, save, 65536);
                printf("  saveDict=%d\n", sv);
            }
            freeStream(s);
        }

        /* setStreamDecode path */
        {
            void *sd = createStreamDecode();
            int csz, dsz;
            void *s = createStream();
            loadDict(s, src, 40000);
            csz = compress_fast_continue(s, src + 40000, cmp, 20000, compressBound(20000), 1);
            setStreamDecode(sd, src, 40000);
            dsz = decompress_safe_continue(sd, cmp, dec, csz, 20000);
            printf("ssd=%d ok=%d\n", dsz, (dsz == 20000 && !memcmp(src + 40000, dec, 20000)));
            freeStreamDecode(sd);
            freeStream(s);
        }

        /* malformed input handling */
        {
            char bad[64];
            int j;
            for (j = 0; j < (int)sizeof(bad); j++) bad[j] = (char)rnd();
            for (j = 1; j <= (int)sizeof(bad); j++) {
                printf("bad %d -> %d\n", j, decompress_safe(bad, dec, j, 100));
            }
        }

        free(src); free(cmp); free(dec);
    }

    printf("DONE\n");
    return 0;
}
