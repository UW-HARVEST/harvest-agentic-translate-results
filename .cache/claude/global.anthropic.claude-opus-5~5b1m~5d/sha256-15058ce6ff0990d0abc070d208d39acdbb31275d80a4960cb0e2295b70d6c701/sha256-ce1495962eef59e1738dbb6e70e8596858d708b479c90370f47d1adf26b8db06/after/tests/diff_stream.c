/*
 * diff_stream.c - Differential tester for the LZ4 (v1.10.0) BLOCK streaming +
 * dictionary API, comparing a reference C liblz4.so against a Rust port
 * liblz4.so. Both libraries are dlopen()'d; nothing is linked against any
 * lz4 header. All symbols are resolved via dlsym() and called through
 * hand-written function pointer typedefs.
 *
 * Only the plain LZ4 block API is covered (no LZ4F_* frame API, no HC).
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <dlfcn.h>

/* ------------------------------------------------------------------ */
/* Globals / bookkeeping                                              */
/* ------------------------------------------------------------------ */

static void *hC, *hR;
static long checks = 0;
static long fails = 0;
static char CTX[512];

#define SETCTX(...) snprintf(CTX, sizeof(CTX), __VA_ARGS__)

static void *getsym(void *h, const char *n) {
    void *p = dlsym(h, n);
    if (!p) { fprintf(stderr, "FATAL: missing symbol %s\n", n); exit(2); }
    return p;
}

static void chkInt(const char *what, long long c, long long r) {
    checks++;
    if (c != r) {
        printf("MISMATCH %s [%s]: c=%lld r=%lld\n", what, CTX, c, r);
        fails++;
    }
}

/* boolean-style comparisons (NULL-ness, pointer-equality semantics) */
static void chkBool(const char *what, int c, int r) {
    checks++;
    if ((c != 0) != (r != 0)) {
        printf("MISMATCH %s [%s]: c=%d r=%d\n", what, CTX, c, r);
        fails++;
    }
}

static void chkBuf(const char *what, const void *a, const void *b, size_t n) {
    checks++;
    if (n == 0) return;
    size_t i;
    const unsigned char *pa = (const unsigned char *)a, *pb = (const unsigned char *)b;
    for (i = 0; i < n; i++) if (pa[i] != pb[i]) break;
    if (i < n) {
        printf("MISMATCH %s [%s]: content differs at byte %zu (compared %zu bytes)\n",
               what, CTX, i, n);
        fails++;
    }
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG                                                 */
/* ------------------------------------------------------------------ */

static uint64_t rng_state = 88172645463325252ULL;
static void rng_seed(uint64_t s) { rng_state = s ? s : 1; }
static uint64_t rnd(void) {
    rng_state ^= rng_state << 13;
    rng_state ^= rng_state >> 7;
    rng_state ^= rng_state << 17;
    return rng_state;
}

#define NMODES 6
static const char *modename(int m) {
    static const char *names[] = {
        "random", "identical", "periodic5", "noisy", "counter", "alpha16"
    };
    return names[m];
}

static void fill(unsigned char *b, size_t n, int mode) {
    size_t i;
    switch (mode) {
    case 0: for (i = 0; i < n; i++) b[i] = (unsigned char)rnd(); break; /* incompressible */
    case 1: memset(b, 0x5A, n); break;                                  /* all identical */
    case 2: for (i = 0; i < n; i++) b[i] = (unsigned char)('A' + (i % 5)); break; /* short period */
    case 3: for (i = 0; i < n; i++)                                     /* mostly-repeat + noise */
                b[i] = ((rnd() & 0xFF) < 25) ? (unsigned char)rnd() : (unsigned char)'z';
            break;
    case 4: for (i = 0; i < n; i++) b[i] = (unsigned char)(i & 0xff); break; /* counter */
    case 5: for (i = 0; i < n; i++) b[i] = (unsigned char)('a' + (rnd() % 16)); break; /* 16-sym alphabet */
    default: memset(b, 0, n); break;
    }
}

/* ------------------------------------------------------------------ */
/* Function pointer typedefs (all pointer args are void* or const void*) */
/* since the ABI representation of any pointer type is identical; we   */
/* never dereference these types ourselves, only pass them through.)   */
/* ------------------------------------------------------------------ */

typedef void *(*fn_v_v)(void);                                   /* createStream/createStreamDecode */
typedef int   (*fn_i_p)(void *);                                  /* freeStream/freeStreamDecode */
typedef void  (*fn_void_p)(void *);                               /* resetStream / resetStream_fast */
typedef void *(*fn_p_pz)(void *, size_t);                         /* initStream */
typedef int   (*fn_i_pcpi)(void *, const char *, int);            /* loadDict/loadDictSlow/setStreamDecode */
typedef void  (*fn_void_pp)(void *, const void *);                /* attach_dictionary */
typedef int   (*fn_i_pcci)(void *, char *, int);                  /* saveDict */
typedef int   (*fn_i_p_cci)(void *, const char *, char *, int);   /* forceExtDict/compress_continue/withState */
typedef int   (*fn_i_p_cciii)(void *, const char *, char *, int, int, int); /* compress_fast_continue / extState[_fastReset] */
typedef int   (*fn_i_p_ccpii)(void *, const char *, char *, int *, int, int); /* compress_destSize_extState */
typedef int   (*fn_i_v0)(void);                                   /* sizeofState/sizeofStreamState/versionNumber */
typedef int   (*fn_i_pc)(void *, char *);                         /* resetStreamState */
typedef void *(*fn_p_c)(char *);                                  /* create */
typedef void *(*fn_p_p)(void *);                                  /* slideInputBuffer */
typedef int   (*fn_i_cci)(const char *, char *, int);             /* compress/decompress_fast/uncompress/withPrefix64k(fast) */
typedef int   (*fn_i_cciii)(const char *, char *, int, int, int);      /* compress_fast / decompress_safe_partial */
typedef int   (*fn_i_p_ccii)(void *, const char *, char *, int, int); /* limitedOutput_withState/_continue, decompress_safe_continue */
typedef int   (*fn_i_ccii)(const char *, char *, int, int);        /* compress_default/limitedOutput/decompress_safe/uncompress_unknownOutputSize/withPrefix64k(safe) */
typedef int   (*fn_i_ccpi)(const char *, char *, int *, int);       /* compress_destSize */
typedef int   (*fn_i_i)(int);                                       /* compressBound / decoderRingBufferSize */
typedef const char *(*fn_str_v)(void);                              /* versionString */
typedef int   (*fn_i_p_cci4)(void *, const char *, char *, int);     /* decompress_fast_continue (ptr,src,dst,origSize) */
typedef int   (*fn_usingDictSafe)(const char *, char *, int, int, const char *, int); /* decompress_safe_usingDict */
typedef int   (*fn_usingDictFast)(const char *, char *, int, const char *, int);      /* decompress_fast_usingDict */
typedef int   (*fn_partialUsingDict)(const char *, char *, int, int, int, const char *, int); /* decompress_safe_partial_usingDict */
typedef int   (*fn_forceExtDict)(const char *, char *, int, int, const void *, size_t);       /* decompress_safe_forceExtDict */
typedef int   (*fn_partialForceExtDict)(const char *, char *, int, int, int, const void *, size_t); /* decompress_safe_partial_forceExtDict */

/* ------------------------------------------------------------------ */
/* Resolved symbol pairs                                              */
/* ------------------------------------------------------------------ */

#define PAIR(ty, name) static ty name##C, name##R
PAIR(fn_v_v, createStream);
PAIR(fn_i_p, freeStream);
PAIR(fn_void_p, resetStream);
PAIR(fn_void_p, resetStreamFast);
PAIR(fn_p_pz, initStream);
PAIR(fn_i_pcpi, loadDict);
PAIR(fn_i_pcpi, loadDictSlow);
PAIR(fn_void_pp, attachDictionary);
PAIR(fn_i_p_cciii, compressFastContinue);
PAIR(fn_i_pcci, saveDict);
PAIR(fn_i_p_cci, compressForceExtDict);
PAIR(fn_i_p_cciii, compressFastExtState);
PAIR(fn_i_p_cciii, compressFastExtStateFastReset);
PAIR(fn_i_p_ccpii, compressDestSizeExtState);
PAIR(fn_i_v0, sizeofState);
PAIR(fn_i_v0, sizeofStreamState);
PAIR(fn_i_v0, versionNumber);
PAIR(fn_i_pc, resetStreamState);
PAIR(fn_p_c, createObsolete);
PAIR(fn_p_p, slideInputBuffer);
PAIR(fn_i_cci, compressObsolete);
PAIR(fn_i_ccii, compressLimitedOutput);
PAIR(fn_i_p_cci, compressWithState);
PAIR(fn_i_p_ccii, compressLimitedOutputWithState);
PAIR(fn_i_p_cci, compressContinue);
PAIR(fn_i_p_ccii, compressLimitedOutputContinue);
PAIR(fn_i_i, compressBound);
PAIR(fn_str_v, versionString);
PAIR(fn_i_cciii, compressFast);
PAIR(fn_i_ccii, compressDefault);
PAIR(fn_i_ccpi, compressDestSize);

PAIR(fn_v_v, createStreamDecode);
PAIR(fn_i_p, freeStreamDecode);
PAIR(fn_i_pcpi, setStreamDecode);
PAIR(fn_i_i, decoderRingBufferSize);
PAIR(fn_i_p_ccii, decompressSafeContinue);
PAIR(fn_usingDictSafe, decompressSafeUsingDict);
PAIR(fn_partialUsingDict, decompressSafePartialUsingDict);
PAIR(fn_i_ccii, decompressSafeWithPrefix64k);
PAIR(fn_forceExtDict, decompressSafeForceExtDict);
PAIR(fn_partialForceExtDict, decompressSafePartialForceExtDict);
PAIR(fn_i_cci, decompressFast);
PAIR(fn_i_p_cci4, decompressFastContinue);
PAIR(fn_usingDictFast, decompressFastUsingDict);
PAIR(fn_i_cci, decompressFastWithPrefix64k);
PAIR(fn_i_ccii, decompressSafe);
PAIR(fn_i_cciii, decompressSafePartial);
PAIR(fn_i_cci, uncompress);
PAIR(fn_i_ccii, uncompressUnknownOutputSize);

static void resolve_all(void) {
#define RS(field, symname) field##C = (void *)getsym(hC, symname); field##R = (void *)getsym(hR, symname)

    RS(createStream, "LZ4_createStream");
    RS(freeStream, "LZ4_freeStream");
    RS(resetStream, "LZ4_resetStream");
    RS(resetStreamFast, "LZ4_resetStream_fast");
    RS(initStream, "LZ4_initStream");
    RS(loadDict, "LZ4_loadDict");
    RS(loadDictSlow, "LZ4_loadDictSlow");
    RS(attachDictionary, "LZ4_attach_dictionary");
    RS(compressFastContinue, "LZ4_compress_fast_continue");
    RS(saveDict, "LZ4_saveDict");
    RS(compressForceExtDict, "LZ4_compress_forceExtDict");
    RS(compressFastExtState, "LZ4_compress_fast_extState");
    RS(compressFastExtStateFastReset, "LZ4_compress_fast_extState_fastReset");
    RS(compressDestSizeExtState, "LZ4_compress_destSize_extState");
    RS(sizeofState, "LZ4_sizeofState");
    RS(sizeofStreamState, "LZ4_sizeofStreamState");
    RS(versionNumber, "LZ4_versionNumber");
    RS(resetStreamState, "LZ4_resetStreamState");
    RS(createObsolete, "LZ4_create");
    RS(slideInputBuffer, "LZ4_slideInputBuffer");
    RS(compressObsolete, "LZ4_compress");
    RS(compressLimitedOutput, "LZ4_compress_limitedOutput");
    RS(compressWithState, "LZ4_compress_withState");
    RS(compressLimitedOutputWithState, "LZ4_compress_limitedOutput_withState");
    RS(compressContinue, "LZ4_compress_continue");
    RS(compressLimitedOutputContinue, "LZ4_compress_limitedOutput_continue");
    RS(compressBound, "LZ4_compressBound");
    RS(versionString, "LZ4_versionString");
    RS(compressFast, "LZ4_compress_fast");
    RS(compressDefault, "LZ4_compress_default");
    RS(compressDestSize, "LZ4_compress_destSize");

    RS(createStreamDecode, "LZ4_createStreamDecode");
    RS(freeStreamDecode, "LZ4_freeStreamDecode");
    RS(setStreamDecode, "LZ4_setStreamDecode");
    RS(decoderRingBufferSize, "LZ4_decoderRingBufferSize");
    RS(decompressSafeContinue, "LZ4_decompress_safe_continue");
    RS(decompressSafeUsingDict, "LZ4_decompress_safe_usingDict");
    RS(decompressSafePartialUsingDict, "LZ4_decompress_safe_partial_usingDict");
    RS(decompressSafeWithPrefix64k, "LZ4_decompress_safe_withPrefix64k");
    RS(decompressSafeForceExtDict, "LZ4_decompress_safe_forceExtDict");
    RS(decompressSafePartialForceExtDict, "LZ4_decompress_safe_partial_forceExtDict");
    RS(decompressFast, "LZ4_decompress_fast");
    RS(decompressFastContinue, "LZ4_decompress_fast_continue");
    RS(decompressFastUsingDict, "LZ4_decompress_fast_usingDict");
    RS(decompressFastWithPrefix64k, "LZ4_decompress_fast_withPrefix64k");
    RS(decompressSafe, "LZ4_decompress_safe");
    RS(decompressSafePartial, "LZ4_decompress_safe_partial");
    RS(uncompress, "LZ4_uncompress");
    RS(uncompressUnknownOutputSize, "LZ4_uncompress_unknownOutputSize");
#undef RS
}

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

static size_t g_cboundMax; /* compressBound(maxBlock) as computed by C lib, plus slack */

static void *newStreamC(void) { return createStreamC(); }
static void *newStreamR(void) { return createStreamR(); }

/* ------------------------------------------------------------------ */
/* 1. versionNumber / versionString / compressBound / decoderRingBufferSize */
/* ------------------------------------------------------------------ */

static void test_versions_and_bounds(void) {
    SETCTX("versionNumber");
    chkInt("versionNumber", versionNumberC(), versionNumberR());

    SETCTX("versionString");
    {
        const char *vc = versionStringC();
        const char *vr = versionStringR();
        checks++;
        if (strcmp(vc, vr) != 0) {
            printf("MISMATCH versionString [%s]: c=%s r=%s\n", CTX, vc, vr);
            fails++;
        }
    }

    /* compressBound over many values, incl. negative and > LZ4_MAX_INPUT_SIZE */
    {
        int vals[] = {
            -1000000, -65536, -1000, -1, 0, 1, 2, 3, 4, 5, 16, 17, 100, 1000,
            65535, 65536, 65537, 100000, 1000000, 0x7DFFFFFF, 0x7E000000,
            0x7E000001, 0x7E000010, 0x7FFFFFFF, (int)0x80000000u, INT32_MIN
        };
        size_t i, n = sizeof(vals) / sizeof(vals[0]);
        for (i = 0; i < n; i++) {
            SETCTX("compressBound(%d)", vals[i]);
            chkInt("compressBound", compressBoundC(vals[i]), compressBoundR(vals[i]));
        }
    }

    /* decoderRingBufferSize over many values incl. negatives and huge */
    {
        int vals[] = {
            -1000000, -1, 0, 1, 15, 16, 17, 1000, 65536, 100000,
            0x7DFFFFFF, 0x7E000000, 0x7E000001, 0x7FFFFFFF, INT32_MIN
        };
        size_t i, n = sizeof(vals) / sizeof(vals[0]);
        for (i = 0; i < n; i++) {
            SETCTX("decoderRingBufferSize(%d)", vals[i]);
            chkInt("decoderRingBufferSize", decoderRingBufferSizeC(vals[i]), decoderRingBufferSizeR(vals[i]));
        }
    }
}

/* ------------------------------------------------------------------ */
/* 2. LZ4_initStream NULL-ness edge cases                              */
/* ------------------------------------------------------------------ */

#define LZ4_STREAM_MINSIZE 16416u

static void test_initstream_nullness(void) {
    unsigned char *buf = malloc(LZ4_STREAM_MINSIZE + 64);
    void *rc, *rr;

    /* NULL buffer */
    SETCTX("initStream(NULL, minsize)");
    rc = initStreamC(NULL, LZ4_STREAM_MINSIZE);
    rr = initStreamR(NULL, LZ4_STREAM_MINSIZE);
    chkBool("initStream.nullbuf", rc != NULL, rr != NULL);

    /* Too-small size */
    SETCTX("initStream(buf, 0)");
    rc = initStreamC(buf, 0);
    rr = initStreamR(buf, 0);
    chkBool("initStream.size0", rc != NULL, rr != NULL);

    SETCTX("initStream(buf, minsize-1)");
    rc = initStreamC(buf, LZ4_STREAM_MINSIZE - 1);
    rr = initStreamR(buf, LZ4_STREAM_MINSIZE - 1);
    chkBool("initStream.toosmall", rc != NULL, rr != NULL);

    /* Exact size, aligned buffer -> should succeed */
    SETCTX("initStream(buf, minsize) aligned");
    rc = initStreamC(buf, LZ4_STREAM_MINSIZE);
    rr = initStreamR(buf, LZ4_STREAM_MINSIZE);
    chkBool("initStream.aligned_exact", rc != NULL, rr != NULL);

    /* Larger size, aligned -> should succeed */
    SETCTX("initStream(buf, minsize+64) aligned");
    rc = initStreamC(buf, LZ4_STREAM_MINSIZE + 64);
    rr = initStreamR(buf, LZ4_STREAM_MINSIZE + 64);
    chkBool("initStream.aligned_larger", rc != NULL, rr != NULL);

    /* Misaligned buffer (offset by 1 byte), exact size */
    SETCTX("initStream(buf+1, minsize) misaligned");
    rc = initStreamC(buf + 1, LZ4_STREAM_MINSIZE);
    rr = initStreamR(buf + 1, LZ4_STREAM_MINSIZE);
    chkBool("initStream.misaligned", rc != NULL, rr != NULL);

    free(buf);
}

/* ------------------------------------------------------------------ */
/* 3. LZ4_loadDict / LZ4_loadDictSlow over many dict sizes             */
/* ------------------------------------------------------------------ */

static void test_loaddict_sizes(void) {
    static const int sizes[] = {0, 1, 3, 4, 5, 64, 1000, 65535, 65536, 70000, 200000};
    size_t i, n = sizeof(sizes) / sizeof(sizes[0]);
    unsigned char *dict = malloc(200000);
    fill(dict, 200000, 2);

    for (i = 0; i < n; i++) {
        int ds = sizes[i];
        void *sC = newStreamC(), *sR = newStreamR();

        SETCTX("loadDict size=%d", ds);
        {
            int rc = loadDictC(sC, (const char *)dict, ds);
            int rr = loadDictR(sR, (const char *)dict, ds);
            chkInt("loadDict.size", rc, rr);
        }
        freeStreamC(sC); freeStreamR(sR);

        sC = newStreamC(); sR = newStreamR();
        SETCTX("loadDictSlow size=%d", ds);
        {
            int rc = loadDictSlowC(sC, (const char *)dict, ds);
            int rr = loadDictSlowR(sR, (const char *)dict, ds);
            chkInt("loadDictSlow.size", rc, rr);
        }
        freeStreamC(sC); freeStreamR(sR);
    }
    free(dict);
}

/* ------------------------------------------------------------------ */
/* 4. compress_fast_continue: prefix-mode chain, many blocks           */
/* ------------------------------------------------------------------ */

static const int block_sizes[] = {
    0, 1, 5, 7, 64, 999, 1000, 3, 4095, 4096, 4097, 8000, 16000,
    20000, 30000, 65535, 65536, 70000, 4, 17, 100
};
#define NBLOCKS (int)(sizeof(block_sizes) / sizeof(block_sizes[0]))

static void test_prefix_chain(void) {
    const size_t BIGN = 400000;
    unsigned char *big = malloc(BIGN);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int accs[] = {-1, 1, 2, 5, 100};
    int mode, ai;

    for (mode = 0; mode < NMODES; mode++) {
        fill(big, BIGN, mode);
        for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
            int acc = accs[ai];
            void *sC = newStreamC(), *sR = newStreamR();
            resetStreamFastC(sC); resetStreamFastR(sR);
            size_t off = 0;
            int bi;
            for (bi = 0; bi < NBLOCKS; bi++) {
                int nsz = block_sizes[bi];
                if (off + (size_t)nsz > BIGN) break;
                const unsigned char *src = big + off;
                int cap = compressBoundC(nsz) + 64;

                SETCTX("prefix_chain mode=%s acc=%d block=%d n=%d off=%zu",
                       modename(mode), acc, bi, nsz, off);
                int rc = compressFastContinueC(sC, (const char *)src, dstC, nsz, cap, acc);
                int rr = compressFastContinueR(sR, (const char *)src, dstR, nsz, cap, acc);
                chkInt("compress_fast_continue.prefix.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("compress_fast_continue.prefix.bytes", dstC, dstR, cl);
                }
                if (rc <= 0 || rr <= 0) {
                    /* stream state undefined after failure per docs: reset both */
                    resetStreamFastC(sC); resetStreamFastR(sR);
                }
                off += (size_t)nsz;
            }
            freeStreamC(sC); freeStreamR(sR);
        }
    }
    free(big); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 5. compress_fast_continue: tight dstCapacity variants               */
/* ------------------------------------------------------------------ */

static void test_tight_capacity(void) {
    static const int sizes[] = {0, 1, 5, 100, 1000, 4096, 8000, 20000, 65536, 70000};
    static const int accs[] = {-1, 1, 2, 5, 100};
    unsigned char *big = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    int mode, si, ai, k;

    for (mode = 0; mode < NMODES; mode++) {
        fill(big, 200000, mode);
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int n = sizes[si];
            if (n + 4096 > 200000) continue;
            for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
                int acc = accs[ai];
                int cbnd = compressBoundC(n);
                int caps[] = {0, 1, cbnd / 4, cbnd / 2, cbnd, cbnd + 64};
                for (k = 0; k < (int)(sizeof(caps) / sizeof(caps[0])); k++) {
                    int cap = caps[k];
                    void *sC = newStreamC(), *sR = newStreamR();
                    resetStreamFastC(sC); resetStreamFastR(sR);
                    /* establish some history with a generous first block */
                    compressFastContinueC(sC, (const char *)big, dstC, 500, g_cboundMax, 1);
                    compressFastContinueR(sR, (const char *)big, dstR, 500, g_cboundMax, 1);

                    SETCTX("tight_capacity mode=%s n=%d acc=%d cap=%d", modename(mode), n, acc, cap);
                    const unsigned char *src = big + 500;
                    int rc = compressFastContinueC(sC, (const char *)src, dstC, n, cap, acc);
                    int rr = compressFastContinueR(sR, (const char *)src, dstR, n, cap, acc);
                    chkInt("compress_fast_continue.tight.rc", rc, rr);
                    if (rc > 0 && rr > 0) {
                        size_t cl = (size_t)(rc < rr ? rc : rr);
                        chkBuf("compress_fast_continue.tight.bytes", dstC, dstR, cl);
                    }
                    freeStreamC(sC); freeStreamR(sR);
                }
            }
        }
    }
    free(big); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 6. Double-buffer mode                                               */
/* ------------------------------------------------------------------ */

static void test_double_buffer(void) {
    static const int accs[] = {-1, 1, 5};
    unsigned char *bufA = malloc(200000), *bufB = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    int mode, ai;

    for (mode = 0; mode < NMODES; mode++) {
        fill(bufA, 200000, mode);
        fill(bufB, 200000, (mode + 1) % NMODES);
        for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
            int acc = accs[ai];
            void *sC = newStreamC(), *sR = newStreamR();
            resetStreamFastC(sC); resetStreamFastR(sR);
            size_t offA = 0, offB = 0;
            int bi;
            for (bi = 0; bi < NBLOCKS; bi++) {
                int nsz = block_sizes[bi];
                unsigned char *buf = (bi & 1) ? bufB : bufA;
                size_t *off = (bi & 1) ? &offB : &offA;
                if (*off + (size_t)nsz + 1 > 200000) break;
                const unsigned char *src = buf + *off;
                int cap = compressBoundC(nsz) + 64;

                SETCTX("double_buffer mode=%s acc=%d block=%d n=%d which=%s",
                       modename(mode), acc, bi, nsz, (bi & 1) ? "B" : "A");
                int rc = compressFastContinueC(sC, (const char *)src, dstC, nsz, cap, acc);
                int rr = compressFastContinueR(sR, (const char *)src, dstR, nsz, cap, acc);
                chkInt("compress_fast_continue.double.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("compress_fast_continue.double.bytes", dstC, dstR, cl);
                }
                if (rc <= 0 || rr <= 0) { resetStreamFastC(sC); resetStreamFastR(sR); }
                *off += (size_t)nsz + 1; /* keep buffers separated by >=1 byte */
            }
            freeStreamC(sC); freeStreamR(sR);
        }
    }
    free(bufA); free(bufB); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 7. Ring-buffer compression (< 64KB, reused circularly)              */
/* ------------------------------------------------------------------ */

static void test_ring_buffer_compress(void) {
    const int maxBlock = 4000;
    const size_t ringSize = (size_t)maxBlock * 8; /* 32000, < 64KB */
    unsigned char *ring = malloc(ringSize);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int accs[] = {-1, 1, 5};
    static const int sizes[] = {1, 5, 100, 999, 1000, 3999, 4000};
    int mode, ai;

    for (mode = 0; mode < NMODES; mode++) {
        for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
            int acc = accs[ai];
            void *sC = newStreamC(), *sR = newStreamR();
            resetStreamFastC(sC); resetStreamFastR(sR);
            size_t offset = 0;
            int bi;
            for (bi = 0; bi < 40; bi++) {
                int nsz = sizes[bi % (int)(sizeof(sizes) / sizeof(sizes[0]))];
                if (offset + (size_t)nsz > ringSize) offset = 0;
                unsigned char *dst = ring + offset;
                /* new data written at same address+content for both libs */
                fill(dst, (size_t)nsz, mode);

                int cap = compressBoundC(nsz) + 64;
                SETCTX("ring_buffer mode=%s acc=%d block=%d n=%d off=%zu",
                       modename(mode), acc, bi, nsz, offset);
                int rc = compressFastContinueC(sC, (const char *)dst, dstC, nsz, cap, acc);
                int rr = compressFastContinueR(sR, (const char *)dst, dstR, nsz, cap, acc);
                chkInt("compress_fast_continue.ring.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("compress_fast_continue.ring.bytes", dstC, dstR, cl);
                }
                if (rc <= 0 || rr <= 0) { resetStreamFastC(sC); resetStreamFastR(sR); }
                offset += (size_t)nsz;
            }
            freeStreamC(sC); freeStreamR(sR);
        }
    }
    free(ring); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 8. LZ4_loadDict followed by compress_fast_continue on separate      */
/*    (extDict) buffer                                                 */
/* ------------------------------------------------------------------ */

static void test_loaddict_then_extdict_continue(void) {
    unsigned char *dict = malloc(70000);
    unsigned char *src = malloc(70000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int dictSizes[] = {0, 1, 100, 1000, 65535, 65536, 70000};
    static const int srcSizes[] = {1, 100, 4000, 8000, 20000};
    int mode, di, si;

    for (mode = 0; mode < NMODES; mode++) {
        fill(dict, 70000, mode);
        fill(src, 70000, (mode + 2) % NMODES);
        for (di = 0; di < (int)(sizeof(dictSizes) / sizeof(dictSizes[0])); di++) {
            for (si = 0; si < (int)(sizeof(srcSizes) / sizeof(srcSizes[0])); si++) {
                int ds = dictSizes[di], ns = srcSizes[si];
                void *sC = newStreamC(), *sR = newStreamR();

                loadDictC(sC, (const char *)dict, ds);
                loadDictR(sR, (const char *)dict, ds);

                int cap = compressBoundC(ns) + 64;
                SETCTX("loaddict_extdict mode=%s dictSize=%d n=%d", modename(mode), ds, ns);
                int rc = compressFastContinueC(sC, (const char *)src, dstC, ns, cap, 1);
                int rr = compressFastContinueR(sR, (const char *)src, dstR, ns, cap, 1);
                chkInt("loaddict_extdict.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("loaddict_extdict.bytes", dstC, dstR, cl);
                }
                freeStreamC(sC); freeStreamR(sR);
            }
        }
    }
    free(dict); free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 9. LZ4_attach_dictionary                                             */
/* ------------------------------------------------------------------ */

static void test_attach_dictionary(void) {
    unsigned char *dict = malloc(70000);
    unsigned char *src = malloc(20000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int srcSizes[] = {1, 100, 4000, 4096, 4097, 8000, 16000};
    int mode, slow, si;

    for (mode = 0; mode < NMODES; mode++) {
        fill(dict, 70000, mode);
        fill(src, 20000, (mode + 3) % NMODES);
        for (slow = 0; slow < 2; slow++) {
            void *dictStreamC = newStreamC(), *dictStreamR = newStreamR();
            if (slow) {
                loadDictSlowC(dictStreamC, (const char *)dict, 50000);
                loadDictSlowR(dictStreamR, (const char *)dict, 50000);
            } else {
                loadDictC(dictStreamC, (const char *)dict, 50000);
                loadDictR(dictStreamR, (const char *)dict, 50000);
            }

            for (si = 0; si < (int)(sizeof(srcSizes) / sizeof(srcSizes[0])); si++) {
                int ns = srcSizes[si];
                void *wC = newStreamC(), *wR = newStreamR();
                resetStreamFastC(wC); resetStreamFastR(wR);
                attachDictionaryC(wC, dictStreamC);
                attachDictionaryR(wR, dictStreamR);

                int cap = compressBoundC(ns) + 64;
                SETCTX("attach_dictionary mode=%s slow=%d n=%d", modename(mode), slow, ns);
                int rc = compressFastContinueC(wC, (const char *)src, dstC, ns, cap, 1);
                int rr = compressFastContinueR(wR, (const char *)src, dstR, ns, cap, 1);
                chkInt("attach_dictionary.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("attach_dictionary.bytes", dstC, dstR, cl);
                }
                freeStreamC(wC); freeStreamR(wR);
            }

            /* attach NULL dictionary */
            {
                void *wC = newStreamC(), *wR = newStreamR();
                resetStreamFastC(wC); resetStreamFastR(wR);
                attachDictionaryC(wC, NULL);
                attachDictionaryR(wR, NULL);
                int ns = 5000;
                int cap = compressBoundC(ns) + 64;
                SETCTX("attach_dictionary_NULL mode=%s slow=%d n=%d", modename(mode), slow, ns);
                int rc = compressFastContinueC(wC, (const char *)src, dstC, ns, cap, 1);
                int rr = compressFastContinueR(wR, (const char *)src, dstR, ns, cap, 1);
                chkInt("attach_dictionary_NULL.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("attach_dictionary_NULL.bytes", dstC, dstR, cl);
                }
                freeStreamC(wC); freeStreamR(wR);
            }

            freeStreamC(dictStreamC); freeStreamR(dictStreamR);
        }
    }
    free(dict); free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 10. LZ4_saveDict + continue, LZ4_slideInputBuffer pointer semantics */
/* ------------------------------------------------------------------ */

static void test_savedict_and_continue(void) {
    unsigned char *src = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int maxDictSizes[] = {0, 4, 1000, 65536, 200000 /* > available */};
    int mode, k;

    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        for (k = 0; k < (int)(sizeof(maxDictSizes) / sizeof(maxDictSizes[0])); k++) {
            int maxDictSize = maxDictSizes[k];
            void *sC = newStreamC(), *sR = newStreamR();
            resetStreamFastC(sC); resetStreamFastR(sR);

            /* establish a small history block */
            int firstN = 5000;
            compressFastContinueC(sC, (const char *)src, dstC, firstN, g_cboundMax, 1);
            compressFastContinueR(sR, (const char *)src, dstR, firstN, g_cboundMax, 1);

            unsigned char *safeC = malloc(70000), *safeR = malloc(70000);
            memset(safeC, 0xAA, 70000); memset(safeR, 0xAA, 70000);

            SETCTX("saveDict mode=%s maxDictSize=%d", modename(mode), maxDictSize);
            int rc = saveDictC(sC, (char *)safeC, maxDictSize);
            int rr = saveDictR(sR, (char *)safeR, maxDictSize);
            chkInt("saveDict.rc", rc, rr);
            if (rc > 0 && rr > 0) {
                size_t cl = (size_t)(rc < rr ? rc : rr);
                chkBuf("saveDict.bytes", safeC, safeR, cl);
            }

            /* slideInputBuffer: compare pointer-equality semantics against safeBuffer */
            if (rc > 0) {
                void *pC = slideInputBufferC(sC);
                void *pR = slideInputBufferR(sR);
                SETCTX("slideInputBuffer mode=%s maxDictSize=%d", modename(mode), maxDictSize);
                chkBool("slideInputBuffer.eq_safebuf", pC == (void *)safeC, pR == (void *)safeR);
            }

            /* continue compressing after saveDict */
            int nextN = 3000;
            const unsigned char *src2 = src + firstN + 1;
            SETCTX("saveDict_then_continue mode=%s maxDictSize=%d", modename(mode), maxDictSize);
            int rc2 = compressFastContinueC(sC, (const char *)src2, dstC, nextN, g_cboundMax, 1);
            int rr2 = compressFastContinueR(sR, (const char *)src2, dstR, nextN, g_cboundMax, 1);
            chkInt("saveDict_then_continue.rc", rc2, rr2);
            if (rc2 > 0 && rr2 > 0) {
                size_t cl = (size_t)(rc2 < rr2 ? rc2 : rr2);
                chkBuf("saveDict_then_continue.bytes", dstC, dstR, cl);
            }

            free(safeC); free(safeR);
            freeStreamC(sC); freeStreamR(sR);
        }
    }
    free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 11. LZ4_compress_forceExtDict                                       */
/* ------------------------------------------------------------------ */

static void test_force_extdict(void) {
    unsigned char *src = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int sizes[] = {1, 100, 1000, 4096, 8000, 20000};
    int mode, si;

    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            void *sC = newStreamC(), *sR = newStreamR();
            resetStreamFastC(sC); resetStreamFastR(sR);
            /* establish history */
            compressFastContinueC(sC, (const char *)src, dstC, 5000, g_cboundMax, 1);
            compressFastContinueR(sR, (const char *)src, dstR, 5000, g_cboundMax, 1);

            const unsigned char *src2 = src + 5000 + 1;
            SETCTX("compress_forceExtDict mode=%s n=%d", modename(mode), ns);
            int rc = compressForceExtDictC(sC, (const char *)src2, dstC, ns);
            int rr = compressForceExtDictR(sR, (const char *)src2, dstR, ns);
            chkInt("compress_forceExtDict.rc", rc, rr);
            if (rc > 0 && rr > 0) {
                size_t cl = (size_t)(rc < rr ? rc : rr);
                chkBuf("compress_forceExtDict.bytes", dstC, dstR, cl);
            }
            freeStreamC(sC); freeStreamR(sR);
        }
    }
    free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 12. extState / extState_fastReset / destSize_extState / misc sizes  */
/* ------------------------------------------------------------------ */

static void test_extstate_and_misc(void) {
    SETCTX("sizeofState");
    chkInt("sizeofState", sizeofStateC(), sizeofStateR());
    SETCTX("sizeofStreamState");
    chkInt("sizeofStreamState", sizeofStreamStateC(), sizeofStreamStateR());

    unsigned char *src = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int sizes[] = {0, 1, 100, 1000, 4096, 8000, 65536, 70000};
    static const int accs[] = {-1, 1, 2, 100};
    int mode, si, ai;

    /* extState (fresh state each call) */
    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
                int acc = accs[ai];
                void *stC = newStreamC(), *stR = newStreamR(); /* zero-initialized via createStream */
                int cap = compressBoundC(ns) + 64;
                SETCTX("compress_fast_extState mode=%s n=%d acc=%d", modename(mode), ns, acc);
                int rc = compressFastExtStateC(stC, (const char *)src, dstC, ns, cap, acc);
                int rr = compressFastExtStateR(stR, (const char *)src, dstR, ns, cap, acc);
                chkInt("compress_fast_extState.rc", rc, rr);
                if (rc > 0 && rr > 0) {
                    size_t cl = (size_t)(rc < rr ? rc : rr);
                    chkBuf("compress_fast_extState.bytes", dstC, dstR, cl);
                }
                freeStreamC(stC); freeStreamR(stR);
            }
        }
    }

    /* extState_fastReset: repeatedly reuse same (correctly-initialized) state buffer */
    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        void *stC = newStreamC(), *stR = newStreamR();
        int bi;
        for (bi = 0; bi < NBLOCKS; bi++) {
            int ns = block_sizes[bi];
            if (ns > 190000) continue;
            int cap = compressBoundC(ns) + 64;
            SETCTX("compress_fast_extState_fastReset mode=%s iter=%d n=%d", modename(mode), bi, ns);
            int rc = compressFastExtStateFastResetC(stC, (const char *)src, dstC, ns, cap, 1);
            int rr = compressFastExtStateFastResetR(stR, (const char *)src, dstR, ns, cap, 1);
            chkInt("compress_fast_extState_fastReset.rc", rc, rr);
            if (rc > 0 && rr > 0) {
                size_t cl = (size_t)(rc < rr ? rc : rr);
                chkBuf("compress_fast_extState_fastReset.bytes", dstC, dstR, cl);
            }
        }
        freeStreamC(stC); freeStreamR(stR);
    }

    /* compress_destSize_extState with accelerations */
    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            for (ai = 0; ai < (int)(sizeof(accs) / sizeof(accs[0])); ai++) {
                int acc = accs[ai];
                int tdsList[] = {0, 1, 5, ns / 4 + 1, ns / 2 + 1, compressBoundC(ns) + 20};
                int t;
                for (t = 0; t < (int)(sizeof(tdsList) / sizeof(tdsList[0])); t++) {
                    int tds = tdsList[t];
                    void *stC = newStreamC(), *stR = newStreamR();
                    int scC = ns, scR = ns;
                    memset(dstC, 0xAA, (size_t)tds < g_cboundMax ? (size_t)tds + 1 : g_cboundMax);
                    memset(dstR, 0xAA, (size_t)tds < g_cboundMax ? (size_t)tds + 1 : g_cboundMax);
                    SETCTX("compress_destSize_extState mode=%s n=%d acc=%d tds=%d", modename(mode), ns, acc, tds);
                    int rc = compressDestSizeExtStateC(stC, (const char *)src, dstC, &scC, tds, acc);
                    int rr = compressDestSizeExtStateR(stR, (const char *)src, dstR, &scR, tds, acc);
                    chkInt("compress_destSize_extState.rc", rc, rr);
                    chkInt("compress_destSize_extState.srcConsumed", scC, scR);
                    if (rc > 0 && rr > 0) {
                        size_t cl = (size_t)(rc < rr ? rc : rr);
                        chkBuf("compress_destSize_extState.bytes", dstC, dstR, cl);
                    }
                    freeStreamC(stC); freeStreamR(stR);
                }
            }
        }
    }

    /* LZ4_resetStreamState / LZ4_create / LZ4_slideInputBuffer basics */
    {
        int stateSize = sizeofStreamStateC();
        unsigned char *stateBufC = malloc((size_t)stateSize + 8);
        unsigned char *stateBufR = malloc((size_t)stateSize + 8);
        SETCTX("resetStreamState");
        int rc = resetStreamStateC(stateBufC, (char *)src);
        int rr = resetStreamStateR(stateBufR, (char *)src);
        chkInt("resetStreamState.rc", rc, rr);
        free(stateBufC); free(stateBufR);

        SETCTX("LZ4_create");
        void *ncC = createObsoleteC(NULL);
        void *ncR = createObsoleteR(NULL);
        chkBool("LZ4_create.nonnull", ncC != NULL, ncR != NULL);
        if (ncC) freeStreamC(ncC);
        if (ncR) freeStreamR(ncR);
    }

    free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* 13. Obsolete compression wrappers                                   */
/* ------------------------------------------------------------------ */

static void test_obsolete_wrappers(void) {
    unsigned char *src = malloc(200000);
    char *dstC = malloc(g_cboundMax), *dstR = malloc(g_cboundMax);
    static const int sizes[] = {0, 1, 100, 1000, 4096, 20000, 65536, 70000};
    int mode, si;

    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 200000, mode);
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];

            SETCTX("LZ4_compress mode=%s n=%d", modename(mode), ns);
            int rc = compressObsoleteC((const char *)src, dstC, ns);
            int rr = compressObsoleteR((const char *)src, dstR, ns);
            chkInt("LZ4_compress.rc", rc, rr);
            if (rc > 0 && rr > 0) chkBuf("LZ4_compress.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));

            int lims[] = {0, 1, ns / 4 + 1, compressBoundC(ns)};
            int k;
            for (k = 0; k < (int)(sizeof(lims) / sizeof(lims[0])); k++) {
                int lim = lims[k];
                SETCTX("LZ4_compress_limitedOutput mode=%s n=%d lim=%d", modename(mode), ns, lim);
                rc = compressLimitedOutputC((const char *)src, dstC, ns, lim);
                rr = compressLimitedOutputR((const char *)src, dstR, ns, lim);
                chkInt("LZ4_compress_limitedOutput.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("LZ4_compress_limitedOutput.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));
            }

            void *stC = newStreamC(), *stR = newStreamR();
            SETCTX("LZ4_compress_withState mode=%s n=%d", modename(mode), ns);
            rc = compressWithStateC(stC, (const char *)src, dstC, ns);
            rr = compressWithStateR(stR, (const char *)src, dstR, ns);
            chkInt("LZ4_compress_withState.rc", rc, rr);
            if (rc > 0 && rr > 0) chkBuf("LZ4_compress_withState.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));
            freeStreamC(stC); freeStreamR(stR);

            stC = newStreamC(); stR = newStreamR();
            SETCTX("LZ4_compress_limitedOutput_withState mode=%s n=%d", modename(mode), ns);
            rc = compressLimitedOutputWithStateC(stC, (const char *)src, dstC, ns, compressBoundC(ns));
            rr = compressLimitedOutputWithStateR(stR, (const char *)src, dstR, ns, compressBoundC(ns));
            chkInt("LZ4_compress_limitedOutput_withState.rc", rc, rr);
            if (rc > 0 && rr > 0) chkBuf("LZ4_compress_limitedOutput_withState.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));
            freeStreamC(stC); freeStreamR(stR);

            void *scC = newStreamC(), *scR = newStreamR();
            resetStreamFastC(scC); resetStreamFastR(scR);
            SETCTX("LZ4_compress_continue mode=%s n=%d", modename(mode), ns);
            rc = compressContinueC(scC, (const char *)src, dstC, ns);
            rr = compressContinueR(scR, (const char *)src, dstR, ns);
            chkInt("LZ4_compress_continue.rc", rc, rr);
            if (rc > 0 && rr > 0) chkBuf("LZ4_compress_continue.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));
            freeStreamC(scC); freeStreamR(scR);

            scC = newStreamC(); scR = newStreamR();
            resetStreamFastC(scC); resetStreamFastR(scR);
            SETCTX("LZ4_compress_limitedOutput_continue mode=%s n=%d", modename(mode), ns);
            rc = compressLimitedOutputContinueC(scC, (const char *)src, dstC, ns, compressBoundC(ns));
            rr = compressLimitedOutputContinueR(scR, (const char *)src, dstR, ns, compressBoundC(ns));
            chkInt("LZ4_compress_limitedOutput_continue.rc", rc, rr);
            if (rc > 0 && rr > 0) chkBuf("LZ4_compress_limitedOutput_continue.bytes", dstC, dstR, (size_t)(rc < rr ? rc : rr));
            freeStreamC(scC); freeStreamR(scR);
        }
    }
    free(src); free(dstC); free(dstR);
}

/* ------------------------------------------------------------------ */
/* Utility: produce a chain of blocks with the reference C compressor  */
/* ------------------------------------------------------------------ */

typedef struct {
    int srcSize;
    int cmpSize;
    unsigned char *cmp; /* owned copy of the compressed block */
} Block;

static Block *make_reference_chain(const unsigned char *big, const int *sizes, int nblocks, int *outCount) {
    Block *blocks = malloc(sizeof(Block) * (size_t)nblocks);
    void *s = newStreamC();
    resetStreamFastC(s);
    size_t off = 0;
    int count = 0, bi;
    char *tmp = malloc(g_cboundMax);
    for (bi = 0; bi < nblocks; bi++) {
        int n = sizes[bi];
        int cap = compressBoundC(n) + 64;
        int rc = compressFastContinueC(s, (const char *)(big + off), tmp, n, cap, 1);
        if (rc <= 0 && n > 0) { resetStreamFastC(s); continue; }
        blocks[count].srcSize = n;
        blocks[count].cmpSize = rc;
        blocks[count].cmp = malloc((size_t)(rc > 0 ? rc : 1));
        if (rc > 0) memcpy(blocks[count].cmp, tmp, (size_t)rc);
        count++;
        off += (size_t)n;
    }
    free(tmp);
    freeStreamC(s);
    *outCount = count;
    return blocks;
}

static void free_reference_chain(Block *blocks, int count) {
    int i;
    for (i = 0; i < count; i++) free(blocks[i].cmp);
    free(blocks);
}

/* ------------------------------------------------------------------ */
/* 14. LZ4_createStreamDecode / freeStreamDecode / setStreamDecode     */
/* ------------------------------------------------------------------ */

static void test_decode_basic(void) {
    void *dC = createStreamDecodeC();
    void *dR = createStreamDecodeR();
    SETCTX("createStreamDecode");
    chkBool("createStreamDecode.nonnull", dC != NULL, dR != NULL);

    SETCTX("setStreamDecode(NULL,0)");
    {
        int rc = setStreamDecodeC(dC, NULL, 0);
        int rr = setStreamDecodeR(dR, NULL, 0);
        chkInt("setStreamDecode.reset", rc, rr);
    }

    unsigned char dict[1000];
    fill(dict, 1000, 0);
    SETCTX("setStreamDecode(dict,1000)");
    {
        int rc = setStreamDecodeC(dC, (const char *)dict, 1000);
        int rr = setStreamDecodeR(dR, (const char *)dict, 1000);
        chkInt("setStreamDecode.withdict", rc, rr);
    }

    SETCTX("freeStreamDecode");
    {
        int rc = freeStreamDecodeC(dC);
        int rr = freeStreamDecodeR(dR);
        chkInt("freeStreamDecode.rc", rc, rr);
    }
}

/* ------------------------------------------------------------------ */
/* 15. decompress_safe_continue: contiguous / double-buffer / ring     */
/*     + corrupted/truncated block error-code comparison               */
/* ------------------------------------------------------------------ */

static void test_decompress_safe_continue_modes(void) {
    const size_t BIGN = 300000;
    unsigned char *big = malloc(BIGN);
    int mode;

    for (mode = 0; mode < NMODES; mode++) {
        fill(big, BIGN, mode);
        int count;
        Block *blocks = make_reference_chain(big, block_sizes, NBLOCKS, &count);

        /* --- contiguous decode --- */
        {
            unsigned char *outC = malloc(BIGN + 4096), *outR = malloc(BIGN + 4096);
            void *dC = createStreamDecodeC(), *dR = createStreamDecodeR();
            size_t off = 0;
            int i;
            for (i = 0; i < count; i++) {
                SETCTX("decompress_safe_continue contiguous mode=%s block=%d srcSize=%d",
                       modename(mode), i, blocks[i].srcSize);
                int rc = decompressSafeContinueC(dC, (const char *)blocks[i].cmp, (char *)(outC + off),
                                                  blocks[i].cmpSize, blocks[i].srcSize);
                int rr = decompressSafeContinueR(dR, (const char *)blocks[i].cmp, (char *)(outR + off),
                                                  blocks[i].cmpSize, blocks[i].srcSize);
                chkInt("decompress_safe_continue.contiguous.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_safe_continue.contiguous.bytes",
                                              outC + off, outR + off, (size_t)(rc < rr ? rc : rr));
                off += (size_t)blocks[i].srcSize;
            }
            freeStreamDecodeC(dC); freeStreamDecodeR(dR);
            free(outC); free(outR);
        }

        /* --- double-buffer decode --- */
        {
            unsigned char *outA_C = malloc(400000), *outB_C = malloc(400000);
            unsigned char *outA_R = malloc(400000), *outB_R = malloc(400000);
            void *dC = createStreamDecodeC(), *dR = createStreamDecodeR();
            size_t offA = 0, offB = 0;
            int i;
            for (i = 0; i < count; i++) {
                int which = i & 1;
                unsigned char *dstC_ = which ? outB_C : outA_C;
                unsigned char *dstR_ = which ? outB_R : outA_R;
                size_t *off = which ? &offB : &offA;
                if (*off + (size_t)blocks[i].srcSize + 1 > 400000) break;

                SETCTX("decompress_safe_continue double mode=%s block=%d srcSize=%d",
                       modename(mode), i, blocks[i].srcSize);
                int rc = decompressSafeContinueC(dC, (const char *)blocks[i].cmp, (char *)(dstC_ + *off),
                                                  blocks[i].cmpSize, blocks[i].srcSize);
                int rr = decompressSafeContinueR(dR, (const char *)blocks[i].cmp, (char *)(dstR_ + *off),
                                                  blocks[i].cmpSize, blocks[i].srcSize);
                chkInt("decompress_safe_continue.double.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_safe_continue.double.bytes",
                                              dstC_ + *off, dstR_ + *off, (size_t)(rc < rr ? rc : rr));
                *off += (size_t)blocks[i].srcSize + 1;
            }
            freeStreamDecodeC(dC); freeStreamDecodeR(dR);
            free(outA_C); free(outB_C); free(outA_R); free(outB_R);
        }

        /* --- ring-buffer decode (size = decoderRingBufferSize(maxBlockSize)) --- */
        {
            int maxBlockSize = 70000; /* covers all block_sizes */
            int ringSz = decoderRingBufferSizeC(maxBlockSize);
            unsigned char *ringC = malloc((size_t)ringSz), *ringR = malloc((size_t)ringSz);
            void *dC = createStreamDecodeC(), *dR = createStreamDecodeR();
            int offset = 0;
            int i;
            for (i = 0; i < count; i++) {
                if (offset + blocks[i].srcSize > ringSz) offset = 0;
                SETCTX("decompress_safe_continue ring mode=%s block=%d srcSize=%d off=%d",
                       modename(mode), i, blocks[i].srcSize, offset);
                int rc = decompressSafeContinueC(dC, (const char *)blocks[i].cmp, (char *)(ringC + offset),
                                                  blocks[i].cmpSize, ringSz - offset);
                int rr = decompressSafeContinueR(dR, (const char *)blocks[i].cmp, (char *)(ringR + offset),
                                                  blocks[i].cmpSize, ringSz - offset);
                chkInt("decompress_safe_continue.ring.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_safe_continue.ring.bytes",
                                              ringC + offset, ringR + offset, (size_t)(rc < rr ? rc : rr));
                offset += blocks[i].srcSize;
            }
            freeStreamDecodeC(dC); freeStreamDecodeR(dR);
            free(ringC); free(ringR);
        }

        /* --- corrupted / truncated blocks --- */
        if (count > 2) {
            unsigned char *outC = malloc(BIGN + 4096), *outR = malloc(BIGN + 4096);
            void *dC = createStreamDecodeC(), *dR = createStreamDecodeR();
            size_t off = 0;
            int i;
            for (i = 0; i < count; i++) {
                int useCorrupt = (i % 5 == 3) && blocks[i].cmpSize > 4;
                int useTrunc = (i % 5 == 4) && blocks[i].cmpSize > 2;
                unsigned char *cmpBuf = blocks[i].cmp;
                int cmpSize = blocks[i].cmpSize;
                unsigned char *tmpCorrupt = NULL;

                if (useCorrupt) {
                    tmpCorrupt = malloc((size_t)cmpSize);
                    memcpy(tmpCorrupt, cmpBuf, (size_t)cmpSize);
                    tmpCorrupt[cmpSize / 2] ^= 0xFF;
                    tmpCorrupt[cmpSize - 1] ^= 0x7F;
                    cmpBuf = tmpCorrupt;
                } else if (useTrunc) {
                    cmpSize = cmpSize / 2;
                }

                SETCTX("decompress_safe_continue error-path mode=%s block=%d corrupt=%d trunc=%d",
                       modename(mode), i, useCorrupt, useTrunc);
                int rc = decompressSafeContinueC(dC, (const char *)cmpBuf, (char *)(outC + off),
                                                  cmpSize, blocks[i].srcSize);
                int rr = decompressSafeContinueR(dR, (const char *)cmpBuf, (char *)(outR + off),
                                                  cmpSize, blocks[i].srcSize);
                /* only compare sign (error vs success); exact negative error codes need not match,
                   but success/fail classification and, on success, content must match */
                chkBool("decompress_safe_continue.errorpath.sign", rc < 0, rr < 0);
                if (rc >= 0 && rr >= 0) {
                    checks++;
                    if (rc != rr) {
                        printf("MISMATCH decompress_safe_continue.errorpath.rc [%s]: c=%d r=%d\n", CTX, rc, rr);
                        fails++;
                    } else if (rc > 0) {
                        chkBuf("decompress_safe_continue.errorpath.bytes", outC + off, outR + off, (size_t)rc);
                    }
                }
                free(tmpCorrupt);

                if (useCorrupt || useTrunc) {
                    /* stream state undefined after malformed input: reset both before continuing */
                    setStreamDecodeC(dC, NULL, 0);
                    setStreamDecodeR(dR, NULL, 0);
                    off = 0; /* also restart contiguous output offset since history is reset */
                } else {
                    off += (size_t)blocks[i].srcSize;
                }
            }
            freeStreamDecodeC(dC); freeStreamDecodeR(dR);
            free(outC); free(outR);
        }

        free_reference_chain(blocks, count);
    }
    free(big);
}

/* ------------------------------------------------------------------ */
/* 16. decompress_safe_usingDict / decompress_safe_partial_usingDict   */
/* ------------------------------------------------------------------ */

static void test_decompress_usingdict(void) {
    unsigned char *dictBuf = malloc(70000);
    unsigned char *srcBuf = malloc(20000);
    int mode;

    for (mode = 0; mode < NMODES; mode++) {
        fill(dictBuf, 70000, mode);
        fill(srcBuf, 20000, (mode + 1) % NMODES);

        static const int dictSizes[] = {0, 1, 100, 65535, 65536, 70000};
        static const int srcSizes[] = {1, 100, 4000, 16000};
        int di, si;
        for (di = 0; di < (int)(sizeof(dictSizes) / sizeof(dictSizes[0])); di++) {
            int ds = dictSizes[di];
            for (si = 0; si < (int)(sizeof(srcSizes) / sizeof(srcSizes[0])); si++) {
                int ns = srcSizes[si];

                /* compress a one-shot block using the C library's normal compressor,
                   feeding it through the streaming dict-based path for reference */
                void *sC = newStreamC();
                loadDictC(sC, (const char *)dictBuf, ds);
                char *cmp = malloc((size_t)compressBoundC(ns) + 64);
                int cmpSize = compressFastContinueC(sC, (const char *)srcBuf, cmp, ns, compressBoundC(ns) + 64, 1);
                freeStreamC(sC);
                if (cmpSize <= 0) { free(cmp); continue; }

                /* dict adjacent to dst: allocate dst right after a copy of dict */
                {
                    unsigned char *adjC = malloc((size_t)ds + (size_t)ns + 64);
                    unsigned char *adjR = malloc((size_t)ds + (size_t)ns + 64);
                    memcpy(adjC, dictBuf, (size_t)ds);
                    memcpy(adjR, dictBuf, (size_t)ds);

                    SETCTX("decompress_safe_usingDict adjacent mode=%s dictSize=%d n=%d", modename(mode), ds, ns);
                    int rc = decompressSafeUsingDictC((const char *)cmp, (char *)(adjC + ds), cmpSize, ns + 64,
                                                       (const char *)adjC, ds);
                    int rr = decompressSafeUsingDictR((const char *)cmp, (char *)(adjR + ds), cmpSize, ns + 64,
                                                       (const char *)adjR, ds);
                    chkInt("decompress_safe_usingDict.adjacent.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_safe_usingDict.adjacent.bytes",
                                                  adjC + ds, adjR + ds, (size_t)(rc < rr ? rc : rr));
                    free(adjC); free(adjR);
                }

                /* dict in separate buffer */
                {
                    unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                    SETCTX("decompress_safe_usingDict separate mode=%s dictSize=%d n=%d", modename(mode), ds, ns);
                    int rc = decompressSafeUsingDictC((const char *)cmp, (char *)dstC_, cmpSize, ns + 64,
                                                       (const char *)dictBuf, ds);
                    int rr = decompressSafeUsingDictR((const char *)cmp, (char *)dstR_, cmpSize, ns + 64,
                                                       (const char *)dictBuf, ds);
                    chkInt("decompress_safe_usingDict.separate.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_safe_usingDict.separate.bytes",
                                                  dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                    free(dstC_); free(dstR_);
                }

                /* partial_usingDict: many targetOutputSize / dstCapacity combos */
                {
                    int tosList[] = {0, 1, ns / 3, ns, ns + 32};
                    int t;
                    for (t = 0; t < (int)(sizeof(tosList) / sizeof(tosList[0])); t++) {
                        int tos = tosList[t];
                        int dcap = ns + 32;
                        unsigned char *dstC_ = malloc((size_t)dcap), *dstR_ = malloc((size_t)dcap);
                        SETCTX("decompress_safe_partial_usingDict mode=%s dictSize=%d n=%d tos=%d",
                               modename(mode), ds, ns, tos);
                        int rc = decompressSafePartialUsingDictC((const char *)cmp, (char *)dstC_, cmpSize, tos, dcap,
                                                                  (const char *)dictBuf, ds);
                        int rr = decompressSafePartialUsingDictR((const char *)cmp, (char *)dstR_, cmpSize, tos, dcap,
                                                                  (const char *)dictBuf, ds);
                        chkInt("decompress_safe_partial_usingDict.rc", rc, rr);
                        if (rc > 0 && rr > 0) chkBuf("decompress_safe_partial_usingDict.bytes",
                                                      dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                        free(dstC_); free(dstR_);
                    }
                }

                free(cmp);
            }
        }
    }
    free(dictBuf); free(srcBuf);
}

/* ------------------------------------------------------------------ */
/* 17. withPrefix64k / forceExtDict / partial_forceExtDict variants    */
/* ------------------------------------------------------------------ */

static void test_prefix64k_and_forceextdict_decode(void) {
    unsigned char *src = malloc(70000);
    int mode;

    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 70000, mode);
        static const int sizes[] = {1, 100, 4000, 65535, 65536, 70000};
        int si;
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            /* One-shot compress (prefix64k semantics require decompression starting at
               offset 0 of a stream w/ <=64KB history, which matches a plain one-shot block). */
            char *cmp = malloc((size_t)compressBoundC(ns) + 64);
            int cmpSize = compressDefaultC((const char *)src, cmp, ns, compressBoundC(ns) + 64);
            if (cmpSize <= 0) { free(cmp); continue; }

            /* decompress_safe_withPrefix64k */
            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("decompress_safe_withPrefix64k mode=%s n=%d", modename(mode), ns);
                int rc = decompressSafeWithPrefix64kC((const char *)cmp, (char *)dstC_, cmpSize, ns + 64);
                int rr = decompressSafeWithPrefix64kR((const char *)cmp, (char *)dstR_, cmpSize, ns + 64);
                chkInt("decompress_safe_withPrefix64k.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_safe_withPrefix64k.bytes", dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                free(dstC_); free(dstR_);
            }

            /* decompress_safe_forceExtDict with empty dict (dictSize 0) -- behaves like noDict */
            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("decompress_safe_forceExtDict mode=%s n=%d", modename(mode), ns);
                int rc = decompressSafeForceExtDictC((const char *)cmp, (char *)dstC_, cmpSize, ns + 64, src, 0);
                int rr = decompressSafeForceExtDictR((const char *)cmp, (char *)dstR_, cmpSize, ns + 64, src, 0);
                chkInt("decompress_safe_forceExtDict.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_safe_forceExtDict.bytes", dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                free(dstC_); free(dstR_);
            }

            /* decompress_safe_partial_forceExtDict */
            {
                int tosList[] = {0, 1, ns / 3, ns};
                int t;
                for (t = 0; t < (int)(sizeof(tosList) / sizeof(tosList[0])); t++) {
                    int tos = tosList[t];
                    unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                    SETCTX("decompress_safe_partial_forceExtDict mode=%s n=%d tos=%d", modename(mode), ns, tos);
                    int rc = decompressSafePartialForceExtDictC((const char *)cmp, (char *)dstC_, cmpSize, tos, ns + 64, src, 0);
                    int rr = decompressSafePartialForceExtDictR((const char *)cmp, (char *)dstR_, cmpSize, tos, ns + 64, src, 0);
                    chkInt("decompress_safe_partial_forceExtDict.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_safe_partial_forceExtDict.bytes", dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                    free(dstC_); free(dstR_);
                }
            }

            /* also with a real (non-empty) ext dictionary, using forceExtDict compressor for a second block */
            {
                void *sC = newStreamC();
                resetStreamFastC(sC);
                char *cmp0 = malloc((size_t)compressBoundC(2000) + 64);
                compressFastContinueC(sC, (const char *)src, cmp0, 2000, compressBoundC(2000) + 64, 1);
                char *cmp1c = malloc((size_t)compressBoundC(ns) + 64);
                int cmp1Size = compressForceExtDictC(sC, (const char *)(src + 2001), cmp1c, ns > 60000 ? 60000 : ns);
                freeStreamC(sC);
                int usedNs = ns > 60000 ? 60000 : ns;
                if (cmp1Size > 0) {
                    unsigned char *dstC_ = malloc((size_t)usedNs + 64), *dstR_ = malloc((size_t)usedNs + 64);
                    SETCTX("decompress_safe_forceExtDict realdict mode=%s n=%d", modename(mode), usedNs);
                    int rc = decompressSafeForceExtDictC((const char *)cmp1c, (char *)dstC_, cmp1Size, usedNs + 64, src, 2000);
                    int rr = decompressSafeForceExtDictR((const char *)cmp1c, (char *)dstR_, cmp1Size, usedNs + 64, src, 2000);
                    chkInt("decompress_safe_forceExtDict.realdict.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_safe_forceExtDict.realdict.bytes", dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                    free(dstC_); free(dstR_);
                }
                free(cmp0); free(cmp1c);
            }

            free(cmp);
        }
    }
    free(src);
}

/* ------------------------------------------------------------------ */
/* 18. decompress_fast / decompress_fast_continue / decompress_fast_usingDict / */
/*     decompress_fast_withPrefix64k -- valid input only               */
/* ------------------------------------------------------------------ */

static void test_decompress_fast_variants(void) {
    unsigned char *src = malloc(70000);
    int mode;

    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 70000, mode);
        static const int sizes[] = {1, 100, 4000, 65535, 65536, 70000};
        int si;
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            char *cmp = malloc((size_t)compressBoundC(ns) + 64);
            int cmpSize = compressDefaultC((const char *)src, cmp, ns, compressBoundC(ns) + 64);
            if (cmpSize <= 0) { free(cmp); continue; }

            /* LZ4_decompress_fast(src, dst, originalSize) - exact size */
            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("decompress_fast mode=%s n=%d", modename(mode), ns);
                int rc = decompressFastC((const char *)cmp, (char *)dstC_, ns);
                int rr = decompressFastR((const char *)cmp, (char *)dstR_, ns);
                chkInt("decompress_fast.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_fast.bytes", dstC_, dstR_, (size_t)ns);
                free(dstC_); free(dstR_);
            }

            /* LZ4_decompress_fast_withPrefix64k */
            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("decompress_fast_withPrefix64k mode=%s n=%d", modename(mode), ns);
                int rc = decompressFastWithPrefix64kC((const char *)cmp, (char *)dstC_, ns);
                int rr = decompressFastWithPrefix64kR((const char *)cmp, (char *)dstR_, ns);
                chkInt("decompress_fast_withPrefix64k.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("decompress_fast_withPrefix64k.bytes", dstC_, dstR_, (size_t)ns);
                free(dstC_); free(dstR_);
            }

            /* LZ4_decompress_fast_usingDict, with a real dictionary */
            {
                unsigned char dict[2000];
                fill(dict, 2000, (mode + 1) % NMODES);
                void *sC = newStreamC();
                resetStreamFastC(sC);
                loadDictC(sC, (const char *)dict, 2000);
                char *cmp2 = malloc((size_t)compressBoundC(ns) + 64);
                int cmp2Size = compressFastContinueC(sC, (const char *)src, cmp2, ns, compressBoundC(ns) + 64, 1);
                freeStreamC(sC);
                if (cmp2Size > 0) {
                    unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                    SETCTX("decompress_fast_usingDict mode=%s n=%d", modename(mode), ns);
                    int rc = decompressFastUsingDictC((const char *)cmp2, (char *)dstC_, ns, (const char *)dict, 2000);
                    int rr = decompressFastUsingDictR((const char *)cmp2, (char *)dstR_, ns, (const char *)dict, 2000);
                    chkInt("decompress_fast_usingDict.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_fast_usingDict.bytes", dstC_, dstR_, (size_t)ns);
                    free(dstC_); free(dstR_);
                }
                free(cmp2);
            }

            /* LZ4_decompress_fast_continue: sequential valid blocks via reference chain */
            {
                int count;
                Block *blocks = make_reference_chain(src, block_sizes, NBLOCKS, &count);
                unsigned char *outC = malloc(300000), *outR = malloc(300000);
                void *dC = createStreamDecodeC(), *dR = createStreamDecodeR();
                size_t off = 0;
                int i;
                for (i = 0; i < count; i++) {
                    if (blocks[i].srcSize == 0) continue; /* fast_continue needs known valid originalSize>0 typically */
                    SETCTX("decompress_fast_continue mode=%s block=%d srcSize=%d", modename(mode), i, blocks[i].srcSize);
                    int rc = decompressFastContinueC(dC, (const char *)blocks[i].cmp, (char *)(outC + off), blocks[i].srcSize);
                    int rr = decompressFastContinueR(dR, (const char *)blocks[i].cmp, (char *)(outR + off), blocks[i].srcSize);
                    chkInt("decompress_fast_continue.rc", rc, rr);
                    if (rc > 0 && rr > 0) chkBuf("decompress_fast_continue.bytes", outC + off, outR + off, (size_t)blocks[i].srcSize);
                    off += (size_t)blocks[i].srcSize;
                }
                freeStreamDecodeC(dC); freeStreamDecodeR(dR);
                free(outC); free(outR);
                free_reference_chain(blocks, count);
            }

            free(cmp);
        }
    }
    free(src);
}

/* ------------------------------------------------------------------ */
/* 19. LZ4_uncompress / LZ4_uncompress_unknownOutputSize                */
/* ------------------------------------------------------------------ */

static void test_uncompress_wrappers(void) {
    unsigned char *src = malloc(70000);
    int mode;
    for (mode = 0; mode < NMODES; mode++) {
        fill(src, 70000, mode);
        static const int sizes[] = {0, 1, 100, 4000, 65536, 70000};
        int si;
        for (si = 0; si < (int)(sizeof(sizes) / sizeof(sizes[0])); si++) {
            int ns = sizes[si];
            char *cmp = malloc((size_t)compressBoundC(ns) + 64);
            int cmpSize = compressDefaultC((const char *)src, cmp, ns, compressBoundC(ns) + 64);
            if (cmpSize <= 0) { free(cmp); continue; }

            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("LZ4_uncompress mode=%s n=%d", modename(mode), ns);
                int rc = uncompressC((const char *)cmp, (char *)dstC_, ns);
                int rr = uncompressR((const char *)cmp, (char *)dstR_, ns);
                chkInt("LZ4_uncompress.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("LZ4_uncompress.bytes", dstC_, dstR_, (size_t)ns);
                free(dstC_); free(dstR_);
            }
            {
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("LZ4_uncompress_unknownOutputSize mode=%s n=%d", modename(mode), ns);
                int rc = uncompressUnknownOutputSizeC((const char *)cmp, (char *)dstC_, cmpSize, ns + 64);
                int rr = uncompressUnknownOutputSizeR((const char *)cmp, (char *)dstR_, cmpSize, ns + 64);
                chkInt("LZ4_uncompress_unknownOutputSize.rc", rc, rr);
                if (rc > 0 && rr > 0) chkBuf("LZ4_uncompress_unknownOutputSize.bytes", dstC_, dstR_, (size_t)(rc < rr ? rc : rr));
                free(dstC_); free(dstR_);
            }
            /* truncated / corrupted input on the safe wrapper */
            if (cmpSize > 4) {
                unsigned char *corrupt = malloc((size_t)cmpSize);
                memcpy(corrupt, cmp, (size_t)cmpSize);
                corrupt[cmpSize / 2] ^= 0xFF;
                unsigned char *dstC_ = malloc((size_t)ns + 64), *dstR_ = malloc((size_t)ns + 64);
                SETCTX("LZ4_uncompress_unknownOutputSize corrupt mode=%s n=%d", modename(mode), ns);
                int rc = uncompressUnknownOutputSizeC((const char *)corrupt, (char *)dstC_, cmpSize, ns + 64);
                int rr = uncompressUnknownOutputSizeR((const char *)corrupt, (char *)dstR_, cmpSize, ns + 64);
                chkBool("LZ4_uncompress_unknownOutputSize.corrupt.sign", rc < 0, rr < 0);
                free(corrupt); free(dstC_); free(dstR_);
            }
            free(cmp);
        }
    }
    free(src);
}

/* ------------------------------------------------------------------ */
/* main                                                                */
/* ------------------------------------------------------------------ */

int main(void) {
    hC = dlopen("./cbuild/liblz4.so", RTLD_NOW | RTLD_LOCAL);
    if (!hC) { printf("dlopen C failed: %s\n", dlerror()); return 2; }
    hR = dlopen("./translation/target/release/liblz4.so", RTLD_NOW | RTLD_LOCAL);
    if (!hR) { printf("dlopen R failed: %s\n", dlerror()); return 2; }

    resolve_all();

    rng_seed(0xC0FFEE1234ULL);

    /* max compressBound over our largest test buffer, plus generous slack */
    g_cboundMax = (size_t)compressBoundC(400000) + 4096;

    test_versions_and_bounds();
    test_initstream_nullness();
    test_loaddict_sizes();
    test_prefix_chain();
    test_tight_capacity();
    test_double_buffer();
    test_ring_buffer_compress();
    test_loaddict_then_extdict_continue();
    test_attach_dictionary();
    test_savedict_and_continue();
    test_force_extdict();
    test_extstate_and_misc();
    test_obsolete_wrappers();

    test_decode_basic();
    test_decompress_safe_continue_modes();
    test_decompress_usingdict();
    test_prefix64k_and_forceextdict_decode();
    test_decompress_fast_variants();
    test_uncompress_wrappers();

    printf("checks=%ld fails=%ld\n", checks, fails);
    return fails ? 1 : 0;
}
