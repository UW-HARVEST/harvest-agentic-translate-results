/* Differential tester for the LZ4F frame + file APIs. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stddef.h>

static void *hC, *hR;
static int fails = 0, checks = 0;

static void *getsym(void *h, const char *n) {
    void *p = dlsym(h, n);
    if (!p) { fprintf(stderr, "missing symbol %s\n", n); exit(2); }
    return p;
}
static void cmpz(const char *what, size_t rc, size_t rr, const void *bc, const void *br, size_t n) {
    checks++;
    if (rc != rr) { printf("MISMATCH %s: rc=%zd rr=%zd\n", what, (ptrdiff_t)rc, (ptrdiff_t)rr); fails++; return; }
    if (n && memcmp(bc, br, n) != 0) {
        size_t i; for (i=0;i<n;i++) if (((unsigned char*)bc)[i]!=((unsigned char*)br)[i]) break;
        printf("MISMATCH %s: content differs at %zu (rc=%zu)\n", what, i, rc); fails++;
    }
}
static uint64_t rs = 0x1234567890abcdefULL;
static uint64_t rnd(void){ rs ^= rs<<13; rs ^= rs>>7; rs ^= rs<<17; return rs; }
static void fill(unsigned char *b, size_t n, int mode) {
    size_t i;
    switch (mode) {
    case 0: for (i=0;i<n;i++) b[i] = (unsigned char)rnd(); break;
    case 1: for (i=0;i<n;i++) b[i] = 'a'; break;
    case 2: for (i=0;i<n;i++) b[i] = (unsigned char)('a' + (i%7)); break;
    case 3: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%4) ? 'x' : (unsigned char)rnd()); break;
    case 4: for (i=0;i<n;i++) b[i] = (unsigned char)(i & 0xff); break;
    default: for (i=0;i<n;i++) b[i] = (unsigned char)((rnd()%16)+'A'); break;
    }
}

typedef struct { unsigned bsid, bmode, ccflag, ftype; unsigned long long csize; unsigned dictID; unsigned bcflag; } FI;
typedef struct { FI fi; int level; unsigned autoFlush, favorDecSpeed; unsigned reserved[3]; } PREFS;
typedef struct { unsigned stableSrc; unsigned reserved[3]; } COPT;
typedef struct { unsigned stableDst, skipChecksums, r1, r0; } DOPT;

typedef size_t (*fn_cf)(void*, size_t, const void*, size_t, const PREFS*);
typedef size_t (*fn_cfb)(size_t, const PREFS*);
typedef size_t (*fn_cb)(size_t, const PREFS*);
typedef int    (*fn_i0)(void);
typedef unsigned (*fn_u0)(void);
typedef unsigned (*fn_iserr)(size_t);
typedef const char* (*fn_errname)(size_t);
typedef int (*fn_errcode)(size_t);
typedef size_t (*fn_gbs)(unsigned);
typedef size_t (*fn_ccc)(void**, unsigned);
typedef size_t (*fn_fcc)(void*);
typedef size_t (*fn_cbegin)(void*, void*, size_t, const PREFS*);
typedef size_t (*fn_cbeginD)(void*, void*, size_t, const void*, size_t, const PREFS*);
typedef size_t (*fn_cbeginCD)(void*, void*, size_t, const void*, const PREFS*);
typedef size_t (*fn_cupd)(void*, void*, size_t, const void*, size_t, const COPT*);
typedef size_t (*fn_flush)(void*, void*, size_t, const COPT*);
typedef size_t (*fn_cend)(void*, void*, size_t, const COPT*);
typedef size_t (*fn_cdc)(void**, unsigned);
typedef size_t (*fn_fdc)(void*);
typedef void   (*fn_rdc)(void*);
typedef size_t (*fn_hsz)(const void*, size_t);
typedef size_t (*fn_gfi)(void*, FI*, const void*, size_t*);
typedef size_t (*fn_dec)(void*, void*, size_t*, const void*, size_t*, const DOPT*);
typedef size_t (*fn_decD)(void*, void*, size_t*, const void*, size_t*, const void*, size_t, const DOPT*);
typedef void*  (*fn_ccd)(const void*, size_t);
typedef void   (*fn_fcd)(void*);
typedef size_t (*fn_cfcd)(void*, void*, size_t, const void*, size_t, const void*, const PREFS*);
typedef size_t (*fn_uupd)(void*, void*, size_t, const void*, size_t, const COPT*);

struct api {
    fn_cf compressFrame; fn_cfb compressFrameBound; fn_cb compressBound;
    fn_i0 clmax; fn_u0 getVersion; fn_iserr isError; fn_errname getErrorName; fn_errcode getErrorCode;
    fn_gbs getBlockSize;
    fn_ccc createC; fn_fcc freeC;
    fn_cbegin cbegin; fn_cbeginD cbeginDict; fn_cbeginD cbeginDictOnce; fn_cbeginCD cbeginCDict;
    fn_cupd cupdate; fn_flush flush; fn_cend cend; fn_uupd uupdate;
    fn_cdc createD; fn_fdc freeD; fn_rdc resetD;
    fn_hsz headerSize; fn_gfi getFrameInfo; fn_dec decompress; fn_decD decompressUsingDict;
    fn_ccd createCDict; fn_fcd freeCDict; fn_cfcd compressFrameUsingCDict;
};

static void load(struct api *a, void *h) {
    a->compressFrame = getsym(h,"LZ4F_compressFrame");
    a->compressFrameBound = getsym(h,"LZ4F_compressFrameBound");
    a->compressBound = getsym(h,"LZ4F_compressBound");
    a->clmax = getsym(h,"LZ4F_compressionLevel_max");
    a->getVersion = getsym(h,"LZ4F_getVersion");
    a->isError = getsym(h,"LZ4F_isError");
    a->getErrorName = getsym(h,"LZ4F_getErrorName");
    a->getErrorCode = getsym(h,"LZ4F_getErrorCode");
    a->getBlockSize = getsym(h,"LZ4F_getBlockSize");
    a->createC = getsym(h,"LZ4F_createCompressionContext");
    a->freeC = getsym(h,"LZ4F_freeCompressionContext");
    a->cbegin = getsym(h,"LZ4F_compressBegin");
    a->cbeginDict = getsym(h,"LZ4F_compressBegin_usingDict");
    a->cbeginDictOnce = getsym(h,"LZ4F_compressBegin_usingDictOnce");
    a->cbeginCDict = getsym(h,"LZ4F_compressBegin_usingCDict");
    a->cupdate = getsym(h,"LZ4F_compressUpdate");
    a->flush = getsym(h,"LZ4F_flush");
    a->cend = getsym(h,"LZ4F_compressEnd");
    a->uupdate = getsym(h,"LZ4F_uncompressedUpdate");
    a->createD = getsym(h,"LZ4F_createDecompressionContext");
    a->freeD = getsym(h,"LZ4F_freeDecompressionContext");
    a->resetD = getsym(h,"LZ4F_resetDecompressionContext");
    a->headerSize = getsym(h,"LZ4F_headerSize");
    a->getFrameInfo = getsym(h,"LZ4F_getFrameInfo");
    a->decompress = getsym(h,"LZ4F_decompress");
    a->decompressUsingDict = getsym(h,"LZ4F_decompress_usingDict");
    a->createCDict = getsym(h,"LZ4F_createCDict");
    a->freeCDict = getsym(h,"LZ4F_freeCDict");
    a->compressFrameUsingCDict = getsym(h,"LZ4F_compressFrame_usingCDict");
}

static struct api C, R;

static unsigned char *src, *cbufC, *cbufR, *dbufC, *dbufR;
static size_t capacity;

/* full frame decompress loop; returns total decoded or (size_t)-1..-24 error */
static size_t decode_frame(struct api *a, const unsigned char *cbuf, size_t csize,
                           unsigned char *out, size_t outCap, size_t srcChunk, size_t dstChunk,
                           const DOPT *dopt, size_t *nerr)
{
    void *dctx = NULL;
    size_t r = a->createD(&dctx, 100);
    size_t si = 0, di = 0;
    if (a->isError(r)) { *nerr = r; return (size_t)-1; }
    *nerr = 0;
    while (si < csize) {
        size_t sc = csize - si; if (sc > srcChunk) sc = srcChunk;
        size_t dc = outCap - di; if (dc > dstChunk) dc = dstChunk;
        size_t hint = a->decompress(dctx, out+di, &dc, cbuf+si, &sc, dopt);
        if (a->isError(hint)) { *nerr = hint; a->freeD(dctx); return (size_t)-1; }
        si += sc; di += dc;
        if (hint == 0) break;
        if (sc == 0 && dc == 0) break;
    }
    a->freeD(dctx);
    return di;
}

int main(void) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    hC = dlopen("./cbuild/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    hR = dlopen("./translation/target/release/liblz4.so", RTLD_NOW|RTLD_LOCAL);
    if (!hC || !hR) { printf("dlopen failed: %s\n", dlerror()); return 2; }
    load(&C, hC); load(&R, hR);

    if (C.getVersion() != R.getVersion()) { printf("getVersion mismatch\n"); fails++; }
    if (C.clmax() != R.clmax()) { printf("clmax mismatch\n"); fails++; }
    { unsigned b; for (b=0;b<12;b++) if (C.getBlockSize(b)!=R.getBlockSize(b)) { printf("getBlockSize(%u) %zd vs %zd\n", b, (ptrdiff_t)C.getBlockSize(b), (ptrdiff_t)R.getBlockSize(b)); fails++; } }
    { int i; for (i=-30;i<30;i++) {
        size_t code = (size_t)(ptrdiff_t)i;
        if (C.isError(code)!=R.isError(code)) { printf("isError(%d) mismatch\n", i); fails++; }
        if (C.getErrorCode(code)!=R.getErrorCode(code)) { printf("getErrorCode(%d) %d vs %d\n", i, C.getErrorCode(code), R.getErrorCode(code)); fails++; }
        { const char*a=C.getErrorName(code), *b=R.getErrorName(code);
          if ((a==NULL)!=(b==NULL) || (a && b && strcmp(a,b))) { printf("getErrorName(%d) '%s' vs '%s'\n", i, a?a:"(null)", b?b:"(null)"); fails++; } }
      } }

    size_t maxN = 700000;
    src = malloc(maxN);
    capacity = maxN + maxN/200 + 4096;
    cbufC = malloc(capacity); cbufR = malloc(capacity);
    dbufC = malloc(maxN + 8192); dbufR = malloc(maxN + 8192);

    static const size_t sizes[] = {0,1,2,7,19,64,100,1000,65535,65536,65537,70000,
        262143,262144,262145,300000,700000};
    int nsizes = (int)(sizeof(sizes)/sizeof(sizes[0]));

    /* preference matrix */
    int bsids[] = {0,4,5,6,7};
    int bmodes[] = {0,1};
    int ccflags[] = {0,1};
    int bcflags[] = {0,1};
    int levels[] = {-5,-1,0,1,2,3,6,9,10,11,12,15};
    int afs[] = {0,1};

    int mode, si, ib, im, ic, ibc, il, ia;

    /* ---- compressFrameBound / compressBound ---- */
    for (ib=0; ib<5; ib++) for (im=0; im<2; im++) for (ic=0; ic<2; ic++) for (ibc=0; ibc<2; ibc++) for (ia=0; ia<2; ia++) {
        PREFS p; memset(&p,0,sizeof(p));
        p.fi.bsid=bsids[ib]; p.fi.bmode=bmodes[im]; p.fi.ccflag=ccflags[ic]; p.fi.bcflag=bcflags[ibc];
        p.autoFlush=afs[ia];
        for (si=0; si<nsizes; si++) {
            size_t n = sizes[si];
            if (C.compressFrameBound(n,&p)!=R.compressFrameBound(n,&p)) { printf("cfBound mismatch n=%zu\n", n); fails++; }
            if (C.compressBound(n,&p)!=R.compressBound(n,&p)) { printf("cBound mismatch n=%zu\n", n); fails++; }
        }
        if (C.compressFrameBound(1000,NULL)!=R.compressFrameBound(1000,NULL)) { printf("cfBound NULL mismatch\n"); fails++; }
        if (C.compressBound(1000,NULL)!=R.compressBound(1000,NULL)) { printf("cBound NULL mismatch\n"); fails++; }
    }

    /* ---- compressFrame single-shot ---- */
    for (mode=0; mode<6; mode++) {
      for (si=0; si<nsizes; si++) {
        size_t n = sizes[si];
        if (n > maxN) continue;
        fill(src, n, mode);
        for (il=0; il<12; il+=3) for (ib=0; ib<5; ib+=2) for (ic=0; ic<2; ic++) {
            PREFS p; memset(&p,0,sizeof(p));
            p.fi.bsid = bsids[ib]; p.fi.ccflag = ccflags[ic];
            p.fi.bcflag = (il+ib) & 1;
            p.fi.bmode = (il+ic) & 1;
            p.fi.csize = ((il+ib+ic)&1) ? 1 : 0;
            p.fi.dictID = ((il)&1) ? 0xABCD1234u : 0;
            p.level = levels[il];
            p.favorDecSpeed = (il & 1);
            size_t bound = C.compressFrameBound(n, &p);
            if (bound > capacity) continue;
            memset(cbufC,0xAA,bound); memset(cbufR,0xAA,bound);
            size_t rc = C.compressFrame(cbufC, bound, src, n, &p);
            size_t rr = R.compressFrame(cbufR, bound, src, n, &p);
            cmpz("compressFrame", rc, rr, cbufC, cbufR, C.isError(rc)?0:rc);
            /* undersized dst */
            if (bound > 4) {
                size_t a = C.compressFrame(cbufC, bound-1, src, n, &p);
                size_t b = R.compressFrame(cbufR, bound-1, src, n, &p);
                cmpz("compressFrame_tight", a, b, cbufC, cbufR, C.isError(a)?0:a);
            }
            /* decode with several chunk sizes */
            if (!C.isError(rc)) {
                size_t chunks[4] = {7, 4096, (size_t)-1, (size_t)-1};
                int k;
                for (k=0;k<3;k++) {
                    size_t neC, neR;
                    size_t oc = decode_frame(&C, cbufC, rc, dbufC, maxN+8192, chunks[k], chunks[k]==1?1:chunks[k], NULL, &neC);
                    size_t orr = decode_frame(&R, cbufR, rc, dbufR, maxN+8192, chunks[k], chunks[k]==1?1:chunks[k], NULL, &neR);
                    checks++;
                    if (oc != orr || neC != neR) { printf("MISMATCH decode: %zu/%zd vs %zu/%zd (n=%zu lvl=%d bsid=%d chunk=%zu)\n", oc,(ptrdiff_t)neC, orr,(ptrdiff_t)neR, n, p.level, p.fi.bsid, chunks[k]); fails++; }
                    else if (oc != (size_t)-1 && oc && memcmp(dbufC, dbufR, oc)) { printf("MISMATCH decode content\n"); fails++; }
                    else if (oc != (size_t)-1 && oc != n) { printf("decode size %zu != %zu\n", oc, n); fails++; }
                    else if (oc != (size_t)-1 && oc && memcmp(dbufC, src, oc)) { printf("decode wrong data\n"); fails++; }
                }
            }
        }
      }
    }

    /* ---- streaming compression ---- */
    for (mode=0; mode<6; mode++) {
      size_t n = 400000;
      fill(src, n, mode);
      for (il=0; il<12; il+=2) for (ib=0; ib<5; ib+=2) for (ia=0; ia<2; ia++) {
        PREFS p; memset(&p,0,sizeof(p));
        p.fi.bsid=bsids[ib]; p.fi.bmode=(il+ib)&1; p.fi.ccflag=(il&1); p.fi.bcflag=(ib&1);
        p.level = levels[il]; p.autoFlush = afs[ia];
        void *cc=NULL, *cr=NULL;
        if (C.isError(C.createC(&cc,100)) || R.isError(R.createC(&cr,100))) { printf("createC failed\n"); fails++; break; }
        size_t oc=0, orr=0;
        size_t hc = C.cbegin(cc, cbufC, capacity, &p);
        size_t hr = R.cbegin(cr, cbufR, capacity, &p);
        cmpz("compressBegin", hc, hr, cbufC, cbufR, C.isError(hc)?0:hc);
        oc = C.isError(hc)?0:hc; orr = R.isError(hr)?0:hr;
        size_t off = 0; int blk = 0;
        while (off < n) {
            size_t chunk = (size_t)(1000 + blk*17000);
            if (off + chunk > n) chunk = n - off;
            COPT co; memset(&co,0,sizeof(co)); co.stableSrc = (blk&1);
            size_t a = C.cupdate(cc, cbufC+oc, capacity-oc, src+off, chunk, &co);
            size_t b = R.cupdate(cr, cbufR+orr, capacity-orr, src+off, chunk, &co);
            cmpz("compressUpdate", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
            if (C.isError(a)||R.isError(b)) break;
            oc += a; orr += b;
            if (blk % 3 == 2) {
                a = C.flush(cc, cbufC+oc, capacity-oc, NULL);
                b = R.flush(cr, cbufR+orr, capacity-orr, NULL);
                cmpz("flush", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
                if (C.isError(a)||R.isError(b)) break;
                oc += a; orr += b;
            }
            off += chunk; blk++;
        }
        { size_t a = C.cend(cc, cbufC+oc, capacity-oc, NULL);
          size_t b = R.cend(cr, cbufR+orr, capacity-orr, NULL);
          cmpz("compressEnd", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
          if (!C.isError(a)) oc += a; if (!R.isError(b)) orr += b; }
        checks++;
        if (oc != orr || memcmp(cbufC, cbufR, oc<orr?oc:orr)) { printf("MISMATCH stream total (%zu vs %zu) mode=%d lvl=%d bsid=%d af=%d\n", oc, orr, mode, p.level, p.fi.bsid, p.autoFlush); fails++; }
        else {
            size_t neC, neR;
            size_t dc = decode_frame(&C, cbufC, oc, dbufC, maxN+8192, 65536, 65536, NULL, &neC);
            size_t dr = decode_frame(&R, cbufR, orr, dbufR, maxN+8192, 65536, 65536, NULL, &neR);
            if (dc!=dr||neC!=neR) { printf("MISMATCH stream decode\n"); fails++; }
            else if (dc!=(size_t)-1 && (dc!=n || memcmp(dbufC,src,n))) { printf("stream roundtrip fail\n"); fails++; }
        }
        C.freeC(cc); R.freeC(cr);
      }
    }

    /* ---- uncompressedUpdate ---- */
    for (mode=0; mode<3; mode++) {
      size_t n = 200000; fill(src, n, mode);
      for (ib=0; ib<5; ib++) for (ic=0;ic<2;ic++) {
        PREFS p; memset(&p,0,sizeof(p));
        p.fi.bsid=bsids[ib]; p.fi.bmode=1; p.fi.ccflag=ic; p.fi.bcflag=(ib&1);
        void *cc=NULL, *cr=NULL;
        C.createC(&cc,100); R.createC(&cr,100);
        size_t oc = C.cbegin(cc, cbufC, capacity, &p);
        size_t orr = R.cbegin(cr, cbufR, capacity, &p);
        cmpz("ub_begin", oc, orr, cbufC, cbufR, C.isError(oc)?0:oc);
        size_t off=0; int blk=0;
        while (off < n) {
            size_t chunk = 30000; if (off+chunk>n) chunk = n-off;
            size_t a,b;
            if (blk & 1) {
                a = C.uupdate(cc, cbufC+oc, capacity-oc, src+off, chunk, NULL);
                b = R.uupdate(cr, cbufR+orr, capacity-orr, src+off, chunk, NULL);
                cmpz("uncompressedUpdate", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
            } else {
                a = C.cupdate(cc, cbufC+oc, capacity-oc, src+off, chunk, NULL);
                b = R.cupdate(cr, cbufR+orr, capacity-orr, src+off, chunk, NULL);
                cmpz("mixedUpdate", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
            }
            if (C.isError(a)||R.isError(b)) break;
            oc+=a; orr+=b; off+=chunk; blk++;
        }
        { size_t a=C.cend(cc,cbufC+oc,capacity-oc,NULL), b=R.cend(cr,cbufR+orr,capacity-orr,NULL);
          cmpz("ub_end", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
          if(!C.isError(a)) oc+=a; if(!R.isError(b)) orr+=b; }
        if (oc==orr) {
            size_t neC,neR;
            size_t dc=decode_frame(&C,cbufC,oc,dbufC,maxN+8192,65536,65536,NULL,&neC);
            size_t dr=decode_frame(&R,cbufR,orr,dbufR,maxN+8192,65536,65536,NULL,&neR);
            if (dc!=dr||neC!=neR){printf("MISMATCH ub decode\n");fails++;}
            else if (dc!=(size_t)-1 && (dc!=n||memcmp(dbufC,src,n))){printf("ub roundtrip fail\n");fails++;}
        }
        C.freeC(cc); R.freeC(cr);
      }
    }

    /* ---- dictionary compression ---- */
    for (mode=0; mode<4; mode++) {
      size_t n = 150000; fill(src, n, mode);
      size_t dictSizes[] = {0, 4, 100, 65535, 65536, 70000};
      int idx;
      for (idx=0; idx<6; idx++) for (il=0; il<12; il+=2) {
        size_t ds = dictSizes[idx];
        PREFS p; memset(&p,0,sizeof(p));
        p.fi.bsid = 4 + (il % 4); p.fi.bmode = il & 1; p.fi.ccflag = (il>>1)&1;
        p.level = levels[il];
        /* usingDict */
        {
          void *cc=NULL, *cr=NULL; C.createC(&cc,100); R.createC(&cr,100);
          size_t oc = C.cbeginDict(cc, cbufC, capacity, src, ds, &p);
          size_t orr = R.cbeginDict(cr, cbufR, capacity, src, ds, &p);
          cmpz("cbeginDict", oc, orr, cbufC, cbufR, C.isError(oc)?0:oc);
          if (!C.isError(oc)) {
            size_t a = C.cupdate(cc, cbufC+oc, capacity-oc, src+ds, n-ds, NULL);
            size_t b = R.cupdate(cr, cbufR+orr, capacity-orr, src+ds, n-ds, NULL);
            cmpz("cupdate_dict", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
            if (!C.isError(a)) { oc+=a; orr+=b; }
            a = C.cend(cc, cbufC+oc, capacity-oc, NULL);
            b = R.cend(cr, cbufR+orr, capacity-orr, NULL);
            cmpz("cend_dict", a, b, cbufC+oc, cbufR+orr, C.isError(a)?0:a);
            if (!C.isError(a)) { oc+=a; orr+=b; }
            /* decompress_usingDict */
            if (oc == orr) {
                DOPT dopt; memset(&dopt,0,sizeof(dopt));
                void *dc=NULL, *dr=NULL; C.createD(&dc,100); R.createD(&dr,100);
                size_t si1=oc, di1=maxN+8192, si2=oc, di2=maxN+8192;
                size_t h1 = C.decompressUsingDict(dc, dbufC, &di1, cbufC, &si1, src, ds, &dopt);
                size_t h2 = R.decompressUsingDict(dr, dbufR, &di2, cbufR, &si2, src, ds, &dopt);
                checks++;
                if (h1!=h2 || si1!=si2 || di1!=di2 || (di1 && memcmp(dbufC,dbufR,di1))) {
                    printf("MISMATCH decompress_usingDict h=%zd/%zd si=%zu/%zu di=%zu/%zu\n", (ptrdiff_t)h1,(ptrdiff_t)h2,si1,si2,di1,di2); fails++;
                }
                C.freeD(dc); R.freeD(dr);
            }
          }
          C.freeC(cc); R.freeC(cr);
        }
        /* CDict */
        if (ds > 0) {
          void *cdC = C.createCDict(src, ds), *cdR = R.createCDict(src, ds);
          if (!cdC || !cdR) { printf("createCDict failed\n"); fails++; }
          else {
            void *cc=NULL, *cr=NULL; C.createC(&cc,100); R.createC(&cr,100);
            size_t bound = C.compressFrameBound(n-ds, &p);
            if (bound <= capacity) {
              size_t a = C.compressFrameUsingCDict(cc, cbufC, bound, src+ds, n-ds, cdC, &p);
              size_t b = R.compressFrameUsingCDict(cr, cbufR, bound, src+ds, n-ds, cdR, &p);
              cmpz("compressFrame_usingCDict", a, b, cbufC, cbufR, C.isError(a)?0:a);
            }
            /* compressBegin_usingCDict streaming */
            size_t oc = C.cbeginCDict(cc, cbufC, capacity, cdC, &p);
            size_t orr = R.cbeginCDict(cr, cbufR, capacity, cdR, &p);
            cmpz("cbeginCDict", oc, orr, cbufC, cbufR, C.isError(oc)?0:oc);
            if (!C.isError(oc)) {
                size_t x = C.cupdate(cc, cbufC+oc, capacity-oc, src+ds, 3000, NULL);
                size_t y = R.cupdate(cr, cbufR+orr, capacity-orr, src+ds, 3000, NULL);
                cmpz("cupdate_cdict", x, y, cbufC+oc, cbufR+orr, C.isError(x)?0:x);
                if (!C.isError(x)) { oc+=x; orr+=y; }
                x = C.cupdate(cc, cbufC+oc, capacity-oc, src+ds+3000, 90000, NULL);
                y = R.cupdate(cr, cbufR+orr, capacity-orr, src+ds+3000, 90000, NULL);
                cmpz("cupdate_cdict2", x, y, cbufC+oc, cbufR+orr, C.isError(x)?0:x);
                if (!C.isError(x)) { oc+=x; orr+=y; }
                x = C.cend(cc, cbufC+oc, capacity-oc, NULL);
                y = R.cend(cr, cbufR+orr, capacity-orr, NULL);
                cmpz("cend_cdict", x, y, cbufC+oc, cbufR+orr, C.isError(x)?0:x);
            }
            C.freeC(cc); R.freeC(cr);
            C.freeCDict(cdC); R.freeCDict(cdR);
          }
        }
      }
    }

    /* ---- headerSize / getFrameInfo / malformed input ---- */
    {
        PREFS p; memset(&p,0,sizeof(p));
        p.fi.csize = 1; p.fi.dictID = 7; p.fi.ccflag = 1; p.fi.bcflag = 1; p.fi.bsid = 6;
        fill(src, 100000, 2);
        size_t rc = C.compressFrame(cbufC, capacity, src, 100000, &p);
        memcpy(cbufR, cbufC, rc);
        size_t k;
        for (k=0; k<=24 && k<=rc; k++) {
            size_t a = C.headerSize(cbufC, k), b = R.headerSize(cbufC, k);
            if (a!=b) { printf("headerSize(%zu) %zd vs %zd\n", k, (ptrdiff_t)a,(ptrdiff_t)b); fails++; }
            void *dc=NULL,*dr=NULL; C.createD(&dc,100); R.createD(&dr,100);
            size_t s1=k, s2=k; FI f1, f2; memset(&f1,0,sizeof(f1)); memset(&f2,0,sizeof(f2));
            size_t g1 = C.getFrameInfo(dc,&f1,cbufC,&s1);
            size_t g2 = R.getFrameInfo(dr,&f2,cbufC,&s2);
            checks++;
            if (g1!=g2||s1!=s2||memcmp(&f1,&f2,sizeof(f1))) { printf("MISMATCH getFrameInfo(%zu) g=%zd/%zd s=%zu/%zu\n", k,(ptrdiff_t)g1,(ptrdiff_t)g2,s1,s2); fails++; }
            C.freeD(dc); R.freeD(dr);
        }
        if (C.headerSize(NULL,10)!=R.headerSize(NULL,10)) { printf("headerSize NULL mismatch\n"); fails++; }
        /* corrupt bytes */
        for (k=0; k<40 && k<rc; k++) {
            unsigned char save = cbufC[k];
            cbufC[k] ^= 0x55;
            size_t neC, neR;
            size_t oc = decode_frame(&C, cbufC, rc, dbufC, maxN+8192, 100000, 100000, NULL, &neC);
            size_t orr = decode_frame(&R, cbufC, rc, dbufR, maxN+8192, 100000, 100000, NULL, &neR);
            checks++;
            if (oc!=orr||neC!=neR) { printf("MISMATCH corrupt@%zu %zu/%zd vs %zu/%zd\n", k, oc,(ptrdiff_t)neC, orr,(ptrdiff_t)neR); fails++; }
            cbufC[k] = save;
        }
        /* truncated */
        for (k=1; k<rc; k += 1 + rc/40) {
            size_t neC, neR;
            size_t oc = decode_frame(&C, cbufC, k, dbufC, maxN+8192, 100000, 100000, NULL, &neC);
            size_t orr = decode_frame(&R, cbufC, k, dbufR, maxN+8192, 100000, 100000, NULL, &neR);
            checks++;
            if (oc!=orr||neC!=neR) { printf("MISMATCH trunc@%zu\n", k); fails++; }
            else if (oc!=(size_t)-1 && oc && memcmp(dbufC,dbufR,oc)) { printf("MISMATCH trunc content@%zu\n",k); fails++; }
        }
        /* skipChecksums / stableDst options */
        { DOPT d1; memset(&d1,0,sizeof(d1)); d1.skipChecksums = 1;
          size_t neC,neR;
          size_t oc=decode_frame(&C,cbufC,rc,dbufC,maxN+8192,3000,3000,&d1,&neC);
          size_t orr=decode_frame(&R,cbufC,rc,dbufR,maxN+8192,3000,3000,&d1,&neR);
          checks++; if(oc!=orr||neC!=neR||(oc!=(size_t)-1&&memcmp(dbufC,dbufR,oc))){printf("MISMATCH skipChecksums\n");fails++;}
          memset(&d1,0,sizeof(d1)); d1.stableDst=1;
          oc=decode_frame(&C,cbufC,rc,dbufC,maxN+8192,3000,3000,&d1,&neC);
          orr=decode_frame(&R,cbufC,rc,dbufR,maxN+8192,3000,3000,&d1,&neR);
          checks++; if(oc!=orr||neC!=neR||(oc!=(size_t)-1&&memcmp(dbufC,dbufR,oc))){printf("MISMATCH stableDst\n");fails++;}
        }
    }

    /* ---- skippable frames ---- */
    {
        unsigned char sf[64];
        memset(sf,0,sizeof(sf));
        sf[0]=0x50; sf[1]=0x2A; sf[2]=0x4D; sf[3]=0x18;
        sf[4]=20; sf[5]=0; sf[6]=0; sf[7]=0;
        size_t total = 8+20;
        size_t k;
        for (k=0;k<20;k++) sf[8+k] = (unsigned char)k;
        size_t neC,neR;
        size_t oc = decode_frame(&C, sf, total, dbufC, 1000, 100, 100, NULL, &neC);
        size_t orr = decode_frame(&R, sf, total, dbufR, 1000, 100, 100, NULL, &neR);
        checks++; if (oc!=orr||neC!=neR) { printf("MISMATCH skippable %zu/%zd vs %zu/%zd\n", oc,(ptrdiff_t)neC,orr,(ptrdiff_t)neR); fails++; }
        oc = decode_frame(&C, sf, total, dbufC, 1000, 3, 3, NULL, &neC);
        orr = decode_frame(&R, sf, total, dbufR, 1000, 3, 3, NULL, &neR);
        checks++; if (oc!=orr||neC!=neR) { printf("MISMATCH skippable small chunks\n"); fails++; }
    }

    /* ---- file API ---- */
    {
        typedef size_t (*fn_wopen)(void**, void*, const PREFS*);
        typedef size_t (*fn_wr)(void*, const void*, size_t);
        typedef size_t (*fn_wclose)(void*);
        typedef size_t (*fn_ropen)(void**, void*);
        typedef size_t (*fn_rd)(void*, void*, size_t);
        typedef size_t (*fn_rclose)(void*);
        fn_wopen woC=getsym(hC,"LZ4F_writeOpen"), woR=getsym(hR,"LZ4F_writeOpen");
        fn_wr wC=getsym(hC,"LZ4F_write"), wR=getsym(hR,"LZ4F_write");
        fn_wclose wcC=getsym(hC,"LZ4F_writeClose"), wcR=getsym(hR,"LZ4F_writeClose");
        fn_ropen roC=getsym(hC,"LZ4F_readOpen"), roR=getsym(hR,"LZ4F_readOpen");
        fn_rd rdC=getsym(hC,"LZ4F_read"), rdR=getsym(hR,"LZ4F_read");
        fn_rclose rcC=getsym(hC,"LZ4F_readClose"), rcR=getsym(hR,"LZ4F_readClose");

        int ii;
        for (ii=0; ii<8; ii++) {
            size_t n = 250000; fill(src, n, ii%6);
            PREFS p; memset(&p,0,sizeof(p));
            p.fi.bsid = 4 + (ii%4); p.fi.bmode = ii&1; p.fi.ccflag = (ii>>1)&1; p.fi.bcflag = (ii>>2)&1;
            p.level = (ii&1)?9:0;
            const char *fnC = "./tmp/fileC.lz4", *fnR = "./tmp/fileR.lz4";
            /* write via both */
            void *ws=NULL; FILE *f = fopen(fnC,"wb");
            size_t r = woC(&ws, f, &p);
            if (C.isError(r)) { printf("writeOpen C err\n"); fails++; }
            size_t off=0; while (off<n) { size_t c = 40000; if(off+c>n)c=n-off; if (wC(ws, src+off, c)!=c){printf("write C err\n");fails++;break;} off+=c; }
            wcC(ws); fclose(f);
            ws=NULL; f = fopen(fnR,"wb");
            r = woR(&ws, f, &p);
            if (R.isError(r)) { printf("writeOpen R err\n"); fails++; }
            off=0; while (off<n) { size_t c = 40000; if(off+c>n)c=n-off; if (wR(ws, src+off, c)!=c){printf("write R err\n");fails++;break;} off+=c; }
            wcR(ws); fclose(f);
            /* compare files */
            {
                FILE *a=fopen(fnC,"rb"), *b=fopen(fnR,"rb");
                fseek(a,0,SEEK_END); long la=ftell(a); fseek(b,0,SEEK_END); long lb=ftell(b);
                checks++;
                if (la!=lb) { printf("MISMATCH file size %ld vs %ld (ii=%d)\n", la, lb, ii); fails++; }
                else {
                    fseek(a,0,SEEK_SET); fseek(b,0,SEEK_SET);
                    unsigned char *ba=malloc(la), *bb=malloc(lb);
                    fread(ba,1,la,a); fread(bb,1,lb,b);
                    if (memcmp(ba,bb,la)) { printf("MISMATCH file content ii=%d\n", ii); fails++; }
                    free(ba); free(bb);
                }
                fclose(a); fclose(b);
            }
            /* read back with both */
            {
                void *rsC=NULL, *rsR=NULL;
                FILE *fa=fopen(fnC,"rb"), *fb=fopen(fnC,"rb");
                size_t x = roC(&rsC, fa), y = roR(&rsR, fb);
                checks++;
                if ((C.isError(x)!=0)!=(R.isError(y)!=0)) { printf("MISMATCH readOpen err\n"); fails++; }
                else if (!C.isError(x)) {
                    size_t totC=0, totR=0, gc, gr;
                    do {
                        gc = rdC(rsC, dbufC+totC, 33333);
                        gr = rdR(rsR, dbufR+totR, 33333);
                        if (gc!=gr) { printf("MISMATCH read %zu vs %zu\n", gc, gr); fails++; break; }
                        totC += gc; totR += gr;
                    } while (gc > 0 && totC < n+1000);
                    if (totC != n) { printf("read total %zu != %zu\n", totC, n); fails++; }
                    else if (memcmp(dbufC, src, n)) { printf("read data wrong\n"); fails++; }
                    else if (memcmp(dbufC, dbufR, totC)) { printf("MISMATCH read data\n"); fails++; }
                }
                if (rsC) rcC(rsC); if (rsR) rcR(rsR);
                fclose(fa); fclose(fb);
            }
        }
    }

    printf("checks=%d fails=%d\n", checks, fails);
    return fails ? 1 : 0;
}
