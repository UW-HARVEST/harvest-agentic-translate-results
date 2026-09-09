/* Differential harness for the dictBuilder DISPLAY / DISPLAYLEVEL / DISPLAYUPDATE
 * stderr diagnostics.
 *
 * Every dictBuilder entry point that takes a notificationLevel is called with
 * notificationLevel = 4 on a small deterministic corpus.  stdout carries the
 * return values (so a mismatch there is caught too); stderr carries the
 * diagnostics we actually want to diff.
 *
 * Build twice (once per libzstd.so) and diff the captured stderr. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#define ZDICT_STATIC_LINKING_ONLY
#include "zdict.h"

/* ---- deterministic PRNG (xorshift64) ---- */
static unsigned long long g_state = 88172645463325252ULL;
static void rs(unsigned long long s) { g_state = s ? s : 1; }
static unsigned long long r64(void) {
    g_state ^= g_state << 13; g_state ^= g_state >> 7; g_state ^= g_state << 17;
    return g_state;
}
static unsigned r32(void) { return (unsigned)(r64() >> 32); }

/* ---- deterministic text-ish corpus, split into fixed-size samples ---- */
#define NB_SAMPLES   96
#define SAMPLE_SIZE  1024
#define CORPUS_SIZE  (NB_SAMPLES * SAMPLE_SIZE)

static unsigned char  g_corpus[CORPUS_SIZE];
static size_t         g_sizes[NB_SAMPLES];

static void build_corpus(void) {
    static const char* words[] = {
        "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ",
        "zstandard ", "compression ", "library ", "test ", "data ", "abc ", "xyz ",
        "dictionary ", "builder ", "cover ", "fastcover ", "legacy "
    };
    size_t i = 0, k;
    rs(0x12345678ULL);
    while (i < CORPUS_SIZE) {
        const char* w = words[r32() % (sizeof(words)/sizeof(words[0]))];
        size_t l = strlen(w);
        if (i + l > CORPUS_SIZE) break;
        memcpy(g_corpus + i, w, l);
        i += l;
    }
    while (i < CORPUS_SIZE) g_corpus[i++] = ' ';
    for (k = 0; k < NB_SAMPLES; k++) g_sizes[k] = SAMPLE_SIZE;
}

#define DICT_CAP 4096
static unsigned char g_dict[DICT_CAP];

static void banner(const char* s) {
    fflush(stdout);
    fprintf(stderr, "\n===== %s =====\n", s);
    fflush(stderr);
}

static void report(const char* s, size_t r) {
    fflush(stderr);
    if (ZDICT_isError(r)) printf("%-44s ERROR %s\n", s, ZDICT_getErrorName(r));
    else                  printf("%-44s size=%zu\n", s, r);
    fflush(stdout);
}

static unsigned g_level = 4;

int main(int argc, char** argv) {
    if (argc > 1) g_level = (unsigned)atoi(argv[1]);
    build_corpus();

    /* ---- 1. ZDICT_trainFromBuffer_legacy ---- */
    {
        ZDICT_legacy_params_t p;
        size_t r;
        memset(&p, 0, sizeof(p));
        p.selectivityLevel = 0;
        p.zParams.compressionLevel = 3;
        p.zParams.notificationLevel = g_level;
        p.zParams.dictID = 1;
        banner("ZDICT_trainFromBuffer_legacy");
        r = ZDICT_trainFromBuffer_legacy(g_dict, DICT_CAP, g_corpus, g_sizes,
                                         NB_SAMPLES, p);
        report("ZDICT_trainFromBuffer_legacy", r);
    }

    /* ---- 2. ZDICT_trainFromBuffer_cover ---- */
    {
        ZDICT_cover_params_t p;
        size_t r;
        memset(&p, 0, sizeof(p));
        p.k = 200; p.d = 8; p.steps = 4; p.nbThreads = 1; p.splitPoint = 1.0;
        p.zParams.compressionLevel = 3;
        p.zParams.notificationLevel = g_level;
        p.zParams.dictID = 2;
        banner("ZDICT_trainFromBuffer_cover");
        r = ZDICT_trainFromBuffer_cover(g_dict, DICT_CAP, g_corpus, g_sizes,
                                        NB_SAMPLES, p);
        report("ZDICT_trainFromBuffer_cover", r);
    }

    /* ---- 3. ZDICT_optimizeTrainFromBuffer_cover ---- */
    {
        ZDICT_cover_params_t p;
        size_t r;
        memset(&p, 0, sizeof(p));
        p.steps = 2; p.nbThreads = 1; p.splitPoint = 0.75;
        p.zParams.compressionLevel = 3;
        p.zParams.notificationLevel = g_level;
        p.zParams.dictID = 3;
        banner("ZDICT_optimizeTrainFromBuffer_cover");
        r = ZDICT_optimizeTrainFromBuffer_cover(g_dict, DICT_CAP, g_corpus, g_sizes,
                                                NB_SAMPLES, &p);
        report("ZDICT_optimizeTrainFromBuffer_cover", r);
        printf("  -> chosen k=%u d=%u steps=%u\n", p.k, p.d, p.steps);
        fflush(stdout);
    }

    /* ---- 4. ZDICT_trainFromBuffer_fastCover ---- */
    {
        ZDICT_fastCover_params_t p;
        size_t r;
        memset(&p, 0, sizeof(p));
        p.k = 200; p.d = 8; p.f = 20; p.steps = 4; p.nbThreads = 1;
        p.splitPoint = 1.0; p.accel = 1;
        p.zParams.compressionLevel = 3;
        p.zParams.notificationLevel = g_level;
        p.zParams.dictID = 4;
        banner("ZDICT_trainFromBuffer_fastCover");
        r = ZDICT_trainFromBuffer_fastCover(g_dict, DICT_CAP, g_corpus, g_sizes,
                                            NB_SAMPLES, p);
        report("ZDICT_trainFromBuffer_fastCover", r);
    }

    /* ---- 5. ZDICT_optimizeTrainFromBuffer_fastCover ---- */
    {
        ZDICT_fastCover_params_t p;
        size_t r;
        memset(&p, 0, sizeof(p));
        p.steps = 2; p.nbThreads = 1; p.splitPoint = 0.75; p.accel = 1; p.f = 20;
        p.zParams.compressionLevel = 3;
        p.zParams.notificationLevel = g_level;
        p.zParams.dictID = 5;
        banner("ZDICT_optimizeTrainFromBuffer_fastCover");
        r = ZDICT_optimizeTrainFromBuffer_fastCover(g_dict, DICT_CAP, g_corpus,
                                                    g_sizes, NB_SAMPLES, &p);
        report("ZDICT_optimizeTrainFromBuffer_fastCover", r);
        printf("  -> chosen k=%u d=%u f=%u steps=%u accel=%u\n",
               p.k, p.d, p.f, p.steps, p.accel);
        fflush(stdout);
    }

    /* ---- 6. ZDICT_finalizeDictionary ---- */
    {
        ZDICT_params_t p;
        size_t r;
        unsigned char content[1024];
        unsigned char out[DICT_CAP];
        memcpy(content, g_corpus + 2048, sizeof(content));
        memset(&p, 0, sizeof(p));
        p.compressionLevel = 3;
        p.notificationLevel = g_level;
        p.dictID = 6;
        banner("ZDICT_finalizeDictionary");
        r = ZDICT_finalizeDictionary(out, DICT_CAP, content, sizeof(content),
                                     g_corpus, g_sizes, NB_SAMPLES, p);
        report("ZDICT_finalizeDictionary", r);
    }

    /* ---- 7. error paths that only produce diagnostics ---- */
    {
        ZDICT_cover_params_t cp;
        ZDICT_fastCover_params_t fp;
        size_t r;

        memset(&cp, 0, sizeof(cp));
        cp.k = 200; cp.d = 8; cp.steps = 4; cp.nbThreads = 1; cp.splitPoint = 1.0;
        cp.zParams.notificationLevel = g_level;
        banner("cover: dictBufferCapacity too small");
        r = ZDICT_trainFromBuffer_cover(g_dict, 16, g_corpus, g_sizes, NB_SAMPLES, cp);
        report("cover tiny dictBufferCapacity", r);

        banner("cover: nbSamples == 0");
        r = ZDICT_trainFromBuffer_cover(g_dict, DICT_CAP, g_corpus, g_sizes, 0, cp);
        report("cover nbSamples=0", r);

        banner("cover: too few training samples");
        r = ZDICT_trainFromBuffer_cover(g_dict, DICT_CAP, g_corpus, g_sizes, 3, cp);
        report("cover nbSamples=3", r);

        memset(&fp, 0, sizeof(fp));
        fp.k = 200; fp.d = 8; fp.f = 20; fp.steps = 4; fp.nbThreads = 1;
        fp.splitPoint = 1.0; fp.accel = 1;
        fp.zParams.notificationLevel = g_level;
        banner("fastCover: dictBufferCapacity too small");
        r = ZDICT_trainFromBuffer_fastCover(g_dict, 16, g_corpus, g_sizes, NB_SAMPLES, fp);
        report("fastCover tiny dictBufferCapacity", r);

        banner("fastCover: nbSamples == 0");
        r = ZDICT_trainFromBuffer_fastCover(g_dict, DICT_CAP, g_corpus, g_sizes, 0, fp);
        report("fastCover nbSamples=0", r);

        banner("fastCover: too few training samples");
        r = ZDICT_trainFromBuffer_fastCover(g_dict, DICT_CAP, g_corpus, g_sizes, 3, fp);
        report("fastCover nbSamples=3", r);

        /* COVER_warnOnSmallCorpus path: huge dict vs tiny corpus */
        memset(&cp, 0, sizeof(cp));
        cp.k = 50; cp.d = 8; cp.steps = 4; cp.nbThreads = 1; cp.splitPoint = 1.0;
        cp.zParams.notificationLevel = g_level;
        banner("cover: warnOnSmallCorpus");
        r = ZDICT_trainFromBuffer_cover(g_dict, DICT_CAP, g_corpus, g_sizes, 6, cp);
        report("cover small corpus", r);

        memset(&fp, 0, sizeof(fp));
        fp.k = 50; fp.d = 8; fp.f = 20; fp.steps = 4; fp.nbThreads = 1;
        fp.splitPoint = 1.0; fp.accel = 1;
        fp.zParams.notificationLevel = g_level;
        banner("fastCover: warnOnSmallCorpus");
        r = ZDICT_trainFromBuffer_fastCover(g_dict, DICT_CAP, g_corpus, g_sizes, 6, fp);
        report("fastCover small corpus", r);
    }

    fflush(stderr);
    fflush(stdout);
    return 0;
}
