/* Differential harness for the LZ4 frame + file APIs. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *H;
#define LOAD(var, name) do { *(void**)&var = dlsym(H, name); if(!var){fprintf(stderr,"missing %s\n",name);exit(2);} } while(0)

static unsigned long long seed = 0x9E3779B97F4A7C15ULL;
static unsigned rnd(void) { seed ^= seed<<13; seed ^= seed>>7; seed ^= seed<<17; return (unsigned)(seed>>16); }

static void hp(const char *tag, const void *p, size_t n) {
    unsigned h = 2166136261u; const unsigned char *b = p; size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 16777619u; }
    printf("%s len=%zu hash=%08x\n", tag, n, h);
}

/* Mirror of LZ4F_preferences_t / frameInfo layout. */
typedef struct {
  int blockSizeID; int blockMode; int contentChecksumFlag; int frameType;
  unsigned long long contentSize; unsigned dictID; int blockChecksumFlag;
} FI;
typedef struct { FI frameInfo; int compressionLevel; unsigned autoFlush; unsigned favorDecSpeed; unsigned reserved[3]; } PREFS;
typedef struct { unsigned stableSrc; unsigned reserved[3]; } COPT;
typedef struct { unsigned stableDst; unsigned skipChecksums; unsigned r1, r0; } DOPT;

int main(int argc, char **argv) {
    unsigned (*isError)(size_t);
    const char* (*getErrorName)(size_t);
    int (*getErrorCode)(size_t);
    unsigned (*getVersion)(void);
    int (*compressionLevel_max)(void);
    size_t (*getBlockSize)(int);
    size_t (*compressFrameBound)(size_t, const PREFS*);
    size_t (*compressBound)(size_t, const PREFS*);
    size_t (*compressFrame)(void*, size_t, const void*, size_t, const PREFS*);
    size_t (*createCompressionContext)(void**, unsigned);
    size_t (*freeCompressionContext)(void*);
    size_t (*compressBegin)(void*, void*, size_t, const PREFS*);
    size_t (*compressUpdate)(void*, void*, size_t, const void*, size_t, const COPT*);
    size_t (*uncompressedUpdate)(void*, void*, size_t, const void*, size_t, const COPT*);
    size_t (*flush)(void*, void*, size_t, const COPT*);
    size_t (*compressEnd)(void*, void*, size_t, const COPT*);
    size_t (*createDecompressionContext)(void**, unsigned);
    size_t (*freeDecompressionContext)(void*);
    void (*resetDecompressionContext)(void*);
    size_t (*headerSize)(const void*, size_t);
    size_t (*getFrameInfo)(void*, FI*, const void*, size_t*);
    size_t (*decompress)(void*, void*, size_t*, const void*, size_t*, const DOPT*);
    size_t (*decompress_usingDict)(void*, void*, size_t*, const void*, size_t*, const void*, size_t, const DOPT*);
    void* (*createCDict)(const void*, size_t);
    void (*freeCDict)(void*);
    size_t (*compressFrame_usingCDict)(void*, void*, size_t, const void*, size_t, const void*, const PREFS*);
    size_t (*compressBegin_usingCDict)(void*, void*, size_t, const void*, const PREFS*);
    size_t (*compressBegin_usingDict)(void*, void*, size_t, const void*, size_t, const PREFS*);
    size_t (*writeOpen)(void**, FILE*, const PREFS*);
    size_t (*writeFn)(void*, const void*, size_t);
    size_t (*writeClose)(void*);
    size_t (*readOpen)(void**, FILE*);
    size_t (*readFn)(void*, void*, size_t);
    size_t (*readClose)(void*);

    if (argc < 3) { fprintf(stderr, "usage: %s <lib> <tmpdir>\n", argv[0]); return 1; }
    H = dlopen(argv[1], RTLD_NOW);
    if (!H) { fprintf(stderr, "%s\n", dlerror()); return 1; }

    LOAD(isError, "LZ4F_isError");
    LOAD(getErrorName, "LZ4F_getErrorName");
    LOAD(getErrorCode, "LZ4F_getErrorCode");
    LOAD(getVersion, "LZ4F_getVersion");
    LOAD(compressionLevel_max, "LZ4F_compressionLevel_max");
    LOAD(getBlockSize, "LZ4F_getBlockSize");
    LOAD(compressFrameBound, "LZ4F_compressFrameBound");
    LOAD(compressBound, "LZ4F_compressBound");
    LOAD(compressFrame, "LZ4F_compressFrame");
    LOAD(createCompressionContext, "LZ4F_createCompressionContext");
    LOAD(freeCompressionContext, "LZ4F_freeCompressionContext");
    LOAD(compressBegin, "LZ4F_compressBegin");
    LOAD(compressUpdate, "LZ4F_compressUpdate");
    LOAD(uncompressedUpdate, "LZ4F_uncompressedUpdate");
    LOAD(flush, "LZ4F_flush");
    LOAD(compressEnd, "LZ4F_compressEnd");
    LOAD(createDecompressionContext, "LZ4F_createDecompressionContext");
    LOAD(freeDecompressionContext, "LZ4F_freeDecompressionContext");
    LOAD(resetDecompressionContext, "LZ4F_resetDecompressionContext");
    LOAD(headerSize, "LZ4F_headerSize");
    LOAD(getFrameInfo, "LZ4F_getFrameInfo");
    LOAD(decompress, "LZ4F_decompress");
    LOAD(decompress_usingDict, "LZ4F_decompress_usingDict");
    LOAD(createCDict, "LZ4F_createCDict");
    LOAD(freeCDict, "LZ4F_freeCDict");
    LOAD(compressFrame_usingCDict, "LZ4F_compressFrame_usingCDict");
    LOAD(compressBegin_usingCDict, "LZ4F_compressBegin_usingCDict");
    LOAD(compressBegin_usingDict, "LZ4F_compressBegin_usingDict");
    LOAD(writeOpen, "LZ4F_writeOpen");
    LOAD(writeFn, "LZ4F_write");
    LOAD(writeClose, "LZ4F_writeClose");
    LOAD(readOpen, "LZ4F_readOpen");
    LOAD(readFn, "LZ4F_read");
    LOAD(readClose, "LZ4F_readClose");

    printf("ver=%u lvlmax=%d\n", getVersion(), compressionLevel_max());
    { int i; for (i = -2; i <= 9; i++) printf("bs %d -> %zu\n", i, getBlockSize(i)); }
    { size_t i; for (i = 0; i <= 26; i++) printf("err %zu isErr=%u name=%s code=%d\n",
        (size_t)(0-i), isError((size_t)(0-i)), getErrorName((size_t)(0-i)), getErrorCode((size_t)(0-i))); }

    /* single-shot frames over many preference combinations */
    {
        int sizes[] = {0,1,13,100,1000,65535,65536,65537,200000,700000};
        int ns = (int)(sizeof(sizes)/sizeof(sizes[0]));
        int si, bsid, bm, cc, bc, lvl, af, mode;
        for (mode = 0; mode < 2; mode++) {
        for (si = 0; si < ns; si++) {
            int n = sizes[si];
            char *src = malloc(n ? n : 1);
            int i;
            for (i = 0; i < n; i++)
                src[i] = mode ? (char)('A' + (i % 13)) : (char)((rnd() % 5) ? (i % 199) : rnd());
            for (bsid = 0; bsid <= 7; bsid++) {
                if (bsid && bsid < 4) continue;
                for (bm = 0; bm <= 1; bm++)
                for (cc = 0; cc <= 1; cc++)
                for (bc = 0; bc <= 1; bc++)
                for (af = 0; af <= 1; af++)
                for (lvl = -2; lvl <= 12; lvl += 5) {
                    PREFS p; size_t bound, csz;
                    char *cmp; 
                    memset(&p, 0, sizeof(p));
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.blockMode = bm;
                    p.frameInfo.contentChecksumFlag = cc;
                    p.frameInfo.blockChecksumFlag = bc;
                    p.frameInfo.contentSize = (n && (si & 1)) ? (unsigned long long)n : 0;
                    p.frameInfo.dictID = (si & 2) ? 0xC0FFEEu : 0;
                    p.compressionLevel = lvl;
                    p.autoFlush = af;
                    bound = compressFrameBound(n, &p);
                    cmp = malloc(bound);
                    csz = compressFrame(cmp, bound, src, n, &p);
                    printf("cf n=%d bs=%d bm=%d cc=%d bc=%d af=%d lvl=%d bound=%zu ",
                           n, bsid, bm, cc, bc, af, lvl, bound);
                    if (isError(csz)) printf("ERR %s\n", getErrorName(csz));
                    else {
                        hp("", cmp, csz);
                        /* round trip with varying chunk sizes */
                        {
                            void *dctx = NULL; size_t r;
                            char *dec = malloc((size_t)n + 16);
                            size_t spos = 0, dpos = 0;
                            createDecompressionContext(&dctx, 100);
                            r = 1;
                            while (spos < csz) {
                                size_t sin = csz - spos; size_t dout = (size_t)n + 16 - dpos;
                                if (sin > 700) sin = 700;
                                r = decompress(dctx, dec + dpos, &dout, cmp + spos, &sin, NULL);
                                if (isError(r)) break;
                                spos += sin; dpos += dout;
                                if (r == 0) break;
                            }
                            if (isError(r)) printf("  DERR %s\n", getErrorName(r));
                            else if (dpos != (size_t)n || (n && memcmp(src, dec, n)))
                                printf("  RTFAIL %zu\n", dpos);
                            else printf("  rt ok hint=%zu\n", r);
                            freeDecompressionContext(dctx);
                            free(dec);
                        }
                        /* header inspection */
                        {
                            FI info; size_t sz = csz > 19 ? 19 : csz; void *dctx = NULL;
                            memset(&info, 0, sizeof(info));
                            createDecompressionContext(&dctx, 100);
                            printf("  hs=%zu gfi=%zu bs=%d bm=%d cc=%d bc=%d cs=%llu did=%u\n",
                                   headerSize(cmp, sz), getFrameInfo(dctx, &info, cmp, &sz),
                                   info.blockSizeID, info.blockMode, info.contentChecksumFlag,
                                   info.blockChecksumFlag, info.contentSize, info.dictID);
                            freeDecompressionContext(dctx);
                        }
                    }
                    free(cmp);
                }
            }
            free(src);
        }
        }
    }

    /* streaming compression with explicit begin/update/flush/end */
    {
        int total = 400000;
        char *src = malloc(total);
        int i, lvl, bm, af, chunk;
        for (i = 0; i < total; i++) src[i] = (char)((rnd() % 8) ? (i % 131) : rnd());
        for (lvl = 0; lvl <= 12; lvl += 4)
        for (bm = 0; bm <= 1; bm++)
        for (af = 0; af <= 1; af++)
        for (chunk = 700; chunk <= 100000; chunk *= 13) {
            PREFS p; COPT o; void *cctx = NULL;
            size_t cap, tot = 0, r;
            char *out;
            memset(&p, 0, sizeof(p));
            memset(&o, 0, sizeof(o));
            p.compressionLevel = lvl; p.frameInfo.blockMode = bm; p.autoFlush = af;
            p.frameInfo.contentChecksumFlag = 1;
            cap = compressFrameBound(total, &p) + 1024;
            out = malloc(cap);
            createCompressionContext(&cctx, 100);
            r = compressBegin(cctx, out, cap, &p);
            if (isError(r)) { printf("BEGERR\n"); }
            tot += r;
            for (i = 0; i < total; i += chunk) {
                int n = (i + chunk > total) ? total - i : chunk;
                r = compressUpdate(cctx, out + tot, cap - tot, src + i, n, &o);
                if (isError(r)) { printf("UPDERR %s\n", getErrorName(r)); break; }
                tot += r;
                if ((i / chunk) % 3 == 2) {
                    r = flush(cctx, out + tot, cap - tot, &o);
                    if (isError(r)) { printf("FLUSHERR\n"); break; }
                    tot += r;
                }
            }
            r = compressEnd(cctx, out + tot, cap - tot, &o);
            if (isError(r)) printf("ENDERR %s\n", getErrorName(r));
            else tot += r;
            printf("stream lvl=%d bm=%d af=%d chunk=%d ", lvl, bm, af, chunk);
            hp("", out, tot);
            /* decompress in one shot */
            {
                void *dctx = NULL; size_t sin = tot, dout = (size_t)total + 64;
                char *dec = malloc(dout);
                size_t rr;
                createDecompressionContext(&dctx, 100);
                rr = decompress(dctx, dec, &dout, out, &sin, NULL);
                printf("  d=%zu sin=%zu dout=%zu ok=%d\n", rr, sin, dout,
                       (dout == (size_t)total && !memcmp(src, dec, total)));
                freeDecompressionContext(dctx);
                free(dec);
            }
            freeCompressionContext(cctx);
            free(out);
        }

        /* uncompressedUpdate */
        {
            PREFS p; void *cctx = NULL; size_t cap, tot = 0, r; char *out;
            memset(&p, 0, sizeof(p));
            p.frameInfo.blockMode = 1; p.frameInfo.blockChecksumFlag = 1;
            cap = compressFrameBound(total, &p) + 4096;
            out = malloc(cap);
            createCompressionContext(&cctx, 100);
            tot += compressBegin(cctx, out, cap, &p);
            for (i = 0; i < 100000; i += 20000) {
                r = uncompressedUpdate(cctx, out + tot, cap - tot, src + i, 20000, NULL);
                if (isError(r)) { printf("UCERR %s\n", getErrorName(r)); break; }
                tot += r;
                r = compressUpdate(cctx, out + tot, cap - tot, src + i + 10000, 5000, NULL);
                if (isError(r)) { printf("MIXERR %s\n", getErrorName(r)); break; }
                tot += r;
            }
            r = compressEnd(cctx, out + tot, cap - tot, NULL);
            tot += isError(r) ? 0 : r;
            printf("ucupd "); hp("", out, tot);
            freeCompressionContext(cctx);
            free(out);
        }

        /* CDict paths */
        {
            void *cd = createCDict(src, 70000);
            int l;
            for (l = 0; l <= 12; l += 4) {
                PREFS p; void *cctx = NULL; size_t cap, csz;
                char *out;
                memset(&p, 0, sizeof(p));
                p.compressionLevel = l;
                cap = compressFrameBound(60000, &p);
                out = malloc(cap);
                createCompressionContext(&cctx, 100);
                csz = compressFrame_usingCDict(cctx, out, cap, src + 70000, 60000, cd, &p);
                printf("cdict lvl=%d ", l);
                if (isError(csz)) printf("ERR %s\n", getErrorName(csz)); else hp("", out, csz);
                /* decompress using dict */
                if (!isError(csz)) {
                    void *dctx = NULL; size_t sin = csz, dout = 60000 + 64;
                    char *dec = malloc(dout);
                    size_t rr;
                    createDecompressionContext(&dctx, 100);
                    rr = decompress_usingDict(dctx, dec, &dout, out, &sin, src, 70000, NULL);
                    printf("  dd=%zu ok=%d\n", rr, (dout == 60000 && !memcmp(src + 70000, dec, 60000)));
                    freeDecompressionContext(dctx);
                    free(dec);
                }
                freeCompressionContext(cctx);
                free(out);
            }
            freeCDict(cd);
        }

        /* compressBegin_usingDict */
        for (lvl = 0; lvl <= 12; lvl += 6) {
            PREFS p; void *cctx = NULL; size_t cap, tot = 0, r; char *out;
            memset(&p, 0, sizeof(p));
            p.compressionLevel = lvl;
            cap = compressFrameBound(50000, &p) + 1024;
            out = malloc(cap);
            createCompressionContext(&cctx, 100);
            r = compressBegin_usingDict(cctx, out, cap, src, 60000, &p);
            if (isError(r)) printf("BUD ERR %s\n", getErrorName(r));
            tot += r;
            r = compressUpdate(cctx, out + tot, cap - tot, src + 60000, 50000, NULL);
            tot += isError(r) ? 0 : r;
            r = compressEnd(cctx, out + tot, cap - tot, NULL);
            tot += isError(r) ? 0 : r;
            printf("bud lvl=%d ", lvl); hp("", out, tot);
            freeCompressionContext(cctx);
            free(out);
        }

        /* skippable frame + truncated / corrupt input */
        {
            unsigned char sk[64];
            void *dctx = NULL;
            size_t sin, dout;
            char dec[128];
            memset(sk, 0xAB, sizeof(sk));
            sk[0]=0x50; sk[1]=0x2A; sk[2]=0x4D; sk[3]=0x18;
            sk[4]=20; sk[5]=0; sk[6]=0; sk[7]=0;
            createDecompressionContext(&dctx, 100);
            sin = sizeof(sk); dout = sizeof(dec);
            printf("skip r=%zu sin=%zu dout=%zu\n", decompress(dctx, dec, &dout, sk, &sin, NULL), sin, dout);
            freeDecompressionContext(dctx);

            /* corrupt frames */
            {
                PREFS p; char cmp[4096]; size_t csz; int j;
                memset(&p, 0, sizeof(p));
                p.frameInfo.contentChecksumFlag = 1;
                p.frameInfo.blockChecksumFlag = 1;
                csz = compressFrame(cmp, sizeof(cmp), src, 2000, &p);
                for (j = 0; j < 40; j++) {
                    char tmp[4096]; void *d2 = NULL; size_t si, dofs;
                    char o2[4096];
                    memcpy(tmp, cmp, csz);
                    tmp[(rnd() % csz)] ^= (char)(1 << (rnd() % 8));
                    createDecompressionContext(&d2, 100);
                    si = csz; dofs = sizeof(o2);
                    { size_t rr = decompress(d2, o2, &dofs, tmp, &si, NULL);
                      printf("corrupt %d isErr=%u code=%d dout=%zu\n", j, isError(rr), getErrorCode(rr), dofs); }
                    freeDecompressionContext(d2);
                }
                /* truncations */
                for (j = 1; j < (int)csz; j += 7) {
                    void *d2 = NULL; size_t si = (size_t)j, dofs = 4096;
                    char o2[4096];
                    createDecompressionContext(&d2, 100);
                    { size_t rr = decompress(d2, o2, &dofs, cmp, &si, NULL);
                      printf("trunc %d r=%zu isErr=%u dout=%zu si=%zu\n", j, rr, isError(rr), dofs, si); }
                    freeDecompressionContext(d2);
                }
            }
        }

        /* file API */
        {
            char path[512];
            int l;
            snprintf(path, sizeof(path), "%s/t.lz4", argv[2]);
            for (l = 0; l <= 12; l += 6) {
                PREFS p; void *wf = NULL, *rf = NULL;
                FILE *fp;
                size_t r;
                char *dec = malloc(total + 64);
                memset(&p, 0, sizeof(p));
                p.compressionLevel = l;
                p.frameInfo.blockSizeID = 5;
                p.frameInfo.contentChecksumFlag = 1;
                fp = fopen(path, "wb");
                r = writeOpen(&wf, fp, &p);
                printf("wopen=%zu\n", r);
                r = writeFn(wf, src, total);
                printf("write=%zu\n", r);
                r = writeClose(wf);
                printf("wclose=%zu\n", r);
                fclose(fp);
                fp = fopen(path, "rb");
                r = readOpen(&rf, fp);
                printf("ropen=%zu\n", r);
                r = readFn(rf, dec, total + 64);
                printf("read=%zu ok=%d\n", r, (r == (size_t)total && !memcmp(src, dec, total)));
                r = readClose(rf);
                printf("rclose=%zu\n", r);
                fclose(fp);
                free(dec);
                { FILE *f2 = fopen(path, "rb"); long sz;
                  fseek(f2, 0, SEEK_END); sz = ftell(f2); fclose(f2);
                  printf("filesize lvl=%d = %ld\n", l, sz); }
            }
            remove(path);
        }

        free(src);
    }

    printf("DONE\n");
    return 0;
}
