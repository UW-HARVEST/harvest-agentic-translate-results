//! Phase B rows 36–42, 47–52, 55–62, 71 and Phase C rows F, G, I, J, L:
//! dictionaries, the sequence API, static / custom-allocator contexts, the
//! `ZDICT_*` dictionary builder, the deprecated `ZBUFF_*` streaming API, and
//! `POOL_*`.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

const P_LEVEL: c_int = 100;
const P_CHECKSUMFLAG: c_int = 201;
const P_STRATEGY: c_int = 107;
const P_FORCEATTACHDICT: c_int = 1001;
const P_PREFETCHCDICT: c_int = 1013;
const P_BLOCKDELIM: c_int = 1008;
const P_VALIDATESEQ: c_int = 1009;
const P_REPCODERES: c_int = 1016;
const P_MINMATCH: c_int = 105;
const DP_REFMULTIPLEDDICTS: c_int = 1003;

type F2 = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;

fn diff_decompress(p: &Pair, frame: &[u8], expect: &[u8], tag: &str) {
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    unsafe {
        let cap = expect.len() + 64;
        let mut cb = vec![0xA5u8; cap];
        let mut rb = vec![0xA5u8; cap];
        let cn = c_de(cb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        let rn = r_de(rb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        eq(&format!("{tag}: decompress ret"), cn, rn);
        if c_ie(cn) != 0 {
            return;
        }
        eq_bytes(&format!("{tag}: decoded"), &cb[..cn], &rb[..rn]);
    }
}

/// Build a corpus of samples suitable for the dictionary trainers.
fn corpus(rng: &mut Rng, nb: usize, min: usize, max: usize) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(nb);
    // shared substrings so the trainers actually find something
    let shared: Vec<Vec<u8>> = (0..6)
        .map(|_| (0..48).map(|_| rng.byte()).collect())
        .collect();
    for _ in 0..nb {
        let target = min + rng.below((max - min + 1) as u32) as usize;
        let start = buf.len();
        while buf.len() - start < target {
            if rng.next_u32() % 3 == 0 {
                buf.extend_from_slice(&shared[rng.below(shared.len() as u32) as usize]);
            } else {
                for _ in 0..(1 + rng.below(32)) {
                    buf.push(rng.byte());
                }
            }
        }
        buf.truncate(start + target);
        sizes.push(target);
    }
    (buf, sizes)
}

// ===================== rows 37-42 / F — dictionaries, all load modes =====================

#[test]
fn row37_row42_dictionary_matrix() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_dreset, r_dreset) = p.sym::<FnReset>("ZSTD_DCtx_reset");
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");

    type FLoad = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FLoadAdv = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, c_int, c_int) -> Sz;
    type FPrefAdv = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, c_int) -> Sz;
    type FCreateCDict = unsafe extern "C" fn(*const c_void, Sz, c_int) -> *mut c_void;
    type FCreateCDictAdv =
        unsafe extern "C" fn(*const c_void, Sz, c_int, c_int, CParams, CustomMem) -> *mut c_void;
    type FCreateDDict = unsafe extern "C" fn(*const c_void, Sz) -> *mut c_void;
    type FCreateDDictAdv =
        unsafe extern "C" fn(*const c_void, Sz, c_int, c_int, CustomMem) -> *mut c_void;
    type FRefCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FSizeof = unsafe extern "C" fn(*const c_void) -> Sz;
    type FDictID = unsafe extern "C" fn(*const c_void) -> c_uint;
    type FDictIDBuf = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FUsingDict =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz, c_int) -> Sz;
    type FDecUsingDict =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz) -> Sz;
    type FUsingCDict =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void) -> Sz;
    type FDecUsingDDict =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void) -> Sz;
    type FGetCParams = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;

    let (c_cload, r_cload) = p.sym::<FLoad>("ZSTD_CCtx_loadDictionary");
    let (c_cloadref, r_cloadref) = p.sym::<FLoad>("ZSTD_CCtx_loadDictionary_byReference");
    let (c_cloadadv, r_cloadadv) = p.sym::<FLoadAdv>("ZSTD_CCtx_loadDictionary_advanced");
    let (c_cpref, r_cpref) = p.sym::<FLoad>("ZSTD_CCtx_refPrefix");
    let (c_cprefadv, r_cprefadv) = p.sym::<FPrefAdv>("ZSTD_CCtx_refPrefix_advanced");
    let (c_dload, r_dload) = p.sym::<FLoad>("ZSTD_DCtx_loadDictionary");
    let (c_dloadref, r_dloadref) = p.sym::<FLoad>("ZSTD_DCtx_loadDictionary_byReference");
    let (c_dloadadv, r_dloadadv) = p.sym::<FLoadAdv>("ZSTD_DCtx_loadDictionary_advanced");
    let (c_dpref, r_dpref) = p.sym::<FLoad>("ZSTD_DCtx_refPrefix");
    let (c_dprefadv, r_dprefadv) = p.sym::<FPrefAdv>("ZSTD_DCtx_refPrefix_advanced");
    let (c_ccd, r_ccd) = p.sym::<FCreateCDict>("ZSTD_createCDict");
    let (c_ccda, r_ccda) = p.sym::<FCreateCDictAdv>("ZSTD_createCDict_advanced");
    let (c_fcd, r_fcd) = p.sym::<FnFreeCtx>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) = p.sym::<FCreateDDict>("ZSTD_createDDict");
    let (c_cddr, r_cddr) = p.sym::<FCreateDDict>("ZSTD_createDDict_byReference");
    let (c_cdda, r_cdda) = p.sym::<FCreateDDictAdv>("ZSTD_createDDict_advanced");
    let (c_fdd, r_fdd) = p.sym::<FnFreeCtx>("ZSTD_freeDDict");
    let (c_refcd, r_refcd) = p.sym::<FRefCDict>("ZSTD_CCtx_refCDict");
    let (c_refdd, r_refdd) = p.sym::<FRefCDict>("ZSTD_DCtx_refDDict");
    let (c_szcd, r_szcd) = p.sym::<FSizeof>("ZSTD_sizeof_CDict");
    let (c_szdd, r_szdd) = p.sym::<FSizeof>("ZSTD_sizeof_DDict");
    let (c_idcd, r_idcd) = p.sym::<FDictID>("ZSTD_getDictID_fromCDict");
    let (c_iddd, r_iddd) = p.sym::<FDictID>("ZSTD_getDictID_fromDDict");
    let (c_idd, r_idd) = p.sym::<FDictIDBuf>("ZSTD_getDictID_fromDict");
    let (c_idf, r_idf) = p.sym::<FDictIDBuf>("ZSTD_getDictID_fromFrame");
    let (c_cud, r_cud) = p.sym::<FUsingDict>("ZSTD_compress_usingDict");
    let (c_dud, r_dud) = p.sym::<FDecUsingDict>("ZSTD_decompress_usingDict");
    let (c_cucd, r_cucd) = p.sym::<FUsingCDict>("ZSTD_compress_usingCDict");
    let (c_dudd, r_dudd) = p.sym::<FDecUsingDDict>("ZSTD_decompress_usingDDict");
    let (c_gcp, _) = p.sym::<FGetCParams>("ZSTD_getCParams");

    let mut rng = Rng::new(SEED ^ 0x37);

    // Dictionary shapes: raw content, and real trained dictionaries.
    let mut dicts: Vec<(&'static str, Vec<u8>)> = Vec::new();
    dicts.push(("empty", Vec::new()));
    dicts.push(("raw-small", gen(Shape::TextLike, 64, &mut rng)));
    dicts.push(("raw-4k", gen(Shape::TextLike, 4096, &mut rng)));
    dicts.push(("raw-random", gen(Shape::Incompressible, 8192, &mut rng)));
    {
        // a real ZDICT_trainFromBuffer dictionary
        type FTrain = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint) -> Sz;
        let (c_tr, _) = p.sym::<FTrain>("ZDICT_trainFromBuffer");
        let (buf, sizes) = corpus(&mut rng, 256, 64, 512);
        let mut d = vec![0u8; 32 * 1024];
        unsafe {
            let n = c_tr(
                d.as_mut_ptr() as *mut c_void,
                d.len(),
                buf.as_ptr() as *const c_void,
                sizes.as_ptr(),
                sizes.len() as c_uint,
            );
            let (c_zie, _) = p.sym::<FnIsError>("ZDICT_isError");
            if c_zie(n) == 0 {
                d.truncate(n);
                dicts.push(("trained", d));
            }
        }
    }

    unsafe {
        for (dname, dict) in &dicts {
            let dp = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
            let ds = dict.len();
            // F1/F2 + row 41
            eq(
                &format!("getDictID_fromDict({dname})"),
                c_idd(dp, ds),
                r_idd(dp, ds),
            );
            for cut in [0usize, 1, 4, 8, 9, 12, ds] {
                // never claim more bytes than the buffer actually has: the C
                // reads MEM_readLE32(dict) as soon as dictSize >= 8.
                if cut > ds {
                    continue;
                }
                eq(
                    &format!("getDictID_fromDict({dname}, cut={cut})"),
                    c_idd(dp, cut),
                    r_idd(dp, cut),
                );
            }

            for level in [1, 3, 9, 19] {
                // ---- row 37: CDict creation in every mode ----
                let ccd = c_ccd(dp, ds, level);
                let rcd = r_ccd(dp, ds, level);
                eq(&format!("createCDict({dname},{level}) nullness"), ccd.is_null(), rcd.is_null());
                if !ccd.is_null() {
                    eq(&format!("sizeof_CDict({dname},{level})"), c_szcd(ccd), r_szcd(rcd));
                    eq(&format!("getDictID_fromCDict({dname},{level})"), c_idcd(ccd), r_idcd(rcd));
                }
                for dlm in [0, 1] {
                    for dct in [0, 1, 2] {
                        let cp = c_gcp(level, ds as u64, ds);
                        let cca = c_ccda(dp, ds, dlm, dct, cp, CustomMem::default());
                        let rca = r_ccda(dp, ds, dlm, dct, cp, CustomMem::default());
                        eq(
                            &format!("createCDict_advanced({dname},dlm={dlm},dct={dct}) nullness"),
                            cca.is_null(),
                            rca.is_null(),
                        );
                        if !cca.is_null() {
                            eq(
                                &format!("sizeof_CDict_advanced({dname},dlm={dlm},dct={dct})"),
                                c_szcd(cca),
                                r_szcd(rca),
                            );
                            eq(
                                &format!("dictID_advanced({dname},dlm={dlm},dct={dct})"),
                                c_idcd(cca),
                                r_idcd(rca),
                            );
                        }
                        c_fcd(cca);
                        r_fcd(rca);
                        // F10/F11 — out-of-range enum ints
                        for bad in [-1, 3, 99, c_int::MAX] {
                            let x = c_ccda(dp, ds, bad, dct, cp, CustomMem::default());
                            let y = r_ccda(dp, ds, bad, dct, cp, CustomMem::default());
                            eq(&format!("createCDict_advanced(bad dlm={bad})"), x.is_null(), y.is_null());
                            c_fcd(x);
                            r_fcd(y);
                            let x = c_ccda(dp, ds, dlm, bad, cp, CustomMem::default());
                            let y = r_ccda(dp, ds, dlm, bad, cp, CustomMem::default());
                            eq(&format!("createCDict_advanced(bad dct={bad})"), x.is_null(), y.is_null());
                            c_fcd(x);
                            r_fcd(y);
                        }
                    }
                }

                // ---- row 40: DDict creation in every mode ----
                let cdd = c_cdd(dp, ds);
                let rdd = r_cdd(dp, ds);
                eq(&format!("createDDict({dname}) nullness"), cdd.is_null(), rdd.is_null());
                if !cdd.is_null() {
                    eq(&format!("sizeof_DDict({dname})"), c_szdd(cdd), r_szdd(rdd));
                    eq(&format!("getDictID_fromDDict({dname})"), c_iddd(cdd), r_iddd(rdd));
                }
                let cddr_ = c_cddr(dp, ds);
                let rddr_ = r_cddr(dp, ds);
                eq(&format!("createDDict_byReference({dname}) nullness"), cddr_.is_null(), rddr_.is_null());
                if !cddr_.is_null() {
                    eq(&format!("sizeof_DDict_byRef({dname})"), c_szdd(cddr_), r_szdd(rddr_));
                }
                for dlm in [0, 1, -1, 3, 99] {
                    for dct in [0, 1, 2, -1, 3, 99] {
                        let x = c_cdda(dp, ds, dlm, dct, CustomMem::default());
                        let y = r_cdda(dp, ds, dlm, dct, CustomMem::default());
                        eq(
                            &format!("createDDict_advanced({dname},dlm={dlm},dct={dct}) nullness"),
                            x.is_null(),
                            y.is_null(),
                        );
                        if !x.is_null() {
                            eq(&format!("sizeof_DDict_adv({dname},{dlm},{dct})"), c_szdd(x), r_szdd(y));
                            eq(&format!("dictID_DDict_adv({dname},{dlm},{dct})"), c_iddd(x), r_iddd(y));
                        }
                        c_fdd(x);
                        r_fdd(y);
                    }
                }

                // ---- rows 36/41: one-shot dict compression + decompression ----
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible, Shape::MixedEntropy] {
                    let len = 1 + rng.below(60_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let cap = c_cb(len) + 64;
                    let tag = format!("dict={dname} lvl={level} shape={shape:?} len={len}");

                    let cc = c_new();
                    let rc = r_new();
                    let mut cf = vec![0u8; cap];
                    let mut rf = vec![0u8; cap];
                    let cn = c_cud(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, dp, ds, level);
                    let rn = r_cud(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, dp, ds, level);
                    eq(&format!("{tag}: compress_usingDict ret"), cn, rn);
                    if c_ie(cn) == 0 {
                        eq_bytes(&format!("{tag}: compress_usingDict frame"), &cf[..cn], &rf[..rn]);
                        eq(
                            &format!("{tag}: getDictID_fromFrame"),
                            c_idf(cf.as_ptr() as *const c_void, cn),
                            r_idf(rf.as_ptr() as *const c_void, rn),
                        );
                        let cd = c_dnew();
                        let rd = r_dnew();
                        let mut cb = vec![0u8; len + 64];
                        let mut rb = vec![0u8; len + 64];
                        let cx = c_dud(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn, dp, ds);
                        let rx = r_dud(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn, dp, ds);
                        eq(&format!("{tag}: decompress_usingDict ret"), cx, rx);
                        if c_ie(cx) == 0 {
                            eq_bytes(&format!("{tag}: decompress_usingDict out"), &cb[..cx], &rb[..rx]);
                            eq_bytes(&format!("{tag}: dict round trip"), &src, &cb[..cx]);
                        }
                        // F9 — decode WITHOUT the dictionary
                        let cx = c_dud(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn, std::ptr::null(), 0);
                        let rx = r_dud(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn, std::ptr::null(), 0);
                        eq(&format!("{tag}: decompress without dict"), cx, rx);
                        // F8 — decode with a DIFFERENT dictionary
                        let other = &dicts[(rng.below(dicts.len() as u32)) as usize].1;
                        let op = if other.is_empty() { std::ptr::null() } else { other.as_ptr() as *const c_void };
                        let cx = c_dud(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn, op, other.len());
                        let rx = r_dud(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn, op, other.len());
                        eq(&format!("{tag}: decompress with wrong dict"), cx, rx);
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                    c_free(cc);
                    r_free(rc);

                    // ---- row 38: refCDict x forceAttachDict x prefetchCDictTables ----
                    if !ccd.is_null() {
                        for fad in [0, 1, 2, 3, -1, 4, 99] {
                            for pf in [0, 1, 2] {
                                let cc = c_new();
                                let rc = r_new();
                                c_reset(cc, 2);
                                r_reset(rc, 2);
                                let a = c_set(cc, P_FORCEATTACHDICT, fad);
                                let b = r_set(rc, P_FORCEATTACHDICT, fad);
                                eq(&format!("{tag}: set forceAttachDict={fad}"), a, b);
                                let a2 = c_set(cc, P_PREFETCHCDICT, pf);
                                let b2 = r_set(rc, P_PREFETCHCDICT, pf);
                                eq(&format!("{tag}: set prefetchCDict={pf}"), a2, b2);
                                if c_ie(a) == 0 && c_ie(a2) == 0 {
                                    eq(
                                        &format!("{tag}: refCDict"),
                                        c_refcd(cc, ccd),
                                        r_refcd(rc, rcd),
                                    );
                                    let mut cf = vec![0u8; cap];
                                    let mut rf = vec![0u8; cap];
                                    let cn = c_c2(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                                    let rn = r_c2(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                                    eq(&format!("{tag}: refCDict compress2(fad={fad},pf={pf}) ret"), cn, rn);
                                    if c_ie(cn) == 0 {
                                        eq_bytes(&format!("{tag}: refCDict frame"), &cf[..cn], &rf[..rn]);
                                        if !cdd.is_null() {
                                            let cd = c_dnew();
                                            let rd = r_dnew();
                                            eq(&format!("{tag}: refDDict"), c_refdd(cd, cdd), r_refdd(rd, rdd));
                                            let mut cb = vec![0u8; len + 64];
                                            let mut rb = vec![0u8; len + 64];
                                            let cx = c_dudd(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn, cdd);
                                            let rx = r_dudd(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn, rdd);
                                            eq(&format!("{tag}: decompress_usingDDict ret"), cx, rx);
                                            if c_ie(cx) == 0 {
                                                eq_bytes(&format!("{tag}: usingDDict out"), &cb[..cx], &rb[..rx]);
                                            }
                                            c_dfree(cd);
                                            r_dfree(rd);
                                        }
                                    }
                                }
                                c_free(cc);
                                r_free(rc);
                            }
                        }
                        // compress_usingCDict
                        let cc = c_new();
                        let rc = r_new();
                        let mut cf = vec![0u8; cap];
                        let mut rf = vec![0u8; cap];
                        let cn = c_cucd(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, ccd);
                        let rn = r_cucd(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, rcd);
                        eq(&format!("{tag}: compress_usingCDict ret"), cn, rn);
                        if c_ie(cn) == 0 {
                            eq_bytes(&format!("{tag}: compress_usingCDict frame"), &cf[..cn], &rf[..rn]);
                        }
                        c_free(cc);
                        r_free(rc);
                    }

                    // ---- row 39: every loadDictionary / refPrefix variant ----
                    for (nm, which) in [
                        ("loadDictionary", 0),
                        ("loadDictionary_byReference", 1),
                        ("refPrefix", 2),
                    ] {
                        for dct in [0, 1, 2, -1, 3, 99] {
                            for dlm in [0, 1, -1, 3] {
                                let cc = c_new();
                                let rc = r_new();
                                c_reset(cc, 2);
                                r_reset(rc, 2);
                                eq(&format!("{tag}: set level"), c_set(cc, P_LEVEL, level), r_set(rc, P_LEVEL, level));
                                let (a, b) = match which {
                                    0 if dct == 0 && dlm == 0 => (c_cload(cc, dp, ds), r_cload(rc, dp, ds)),
                                    1 if dct == 0 && dlm == 0 => (c_cloadref(cc, dp, ds), r_cloadref(rc, dp, ds)),
                                    2 if dlm == 0 => (
                                        if dct == 0 { c_cpref(cc, dp, ds) } else { c_cprefadv(cc, dp, ds, dct) },
                                        if dct == 0 { r_cpref(rc, dp, ds) } else { r_cprefadv(rc, dp, ds, dct) },
                                    ),
                                    _ => (
                                        c_cloadadv(cc, dp, ds, dlm, dct),
                                        r_cloadadv(rc, dp, ds, dlm, dct),
                                    ),
                                };
                                eq(&format!("{tag}: {nm}(dlm={dlm},dct={dct}) ret"), a, b);
                                if c_ie(a) == 0 {
                                    let mut cf = vec![0u8; cap];
                                    let mut rf = vec![0u8; cap];
                                    let cn = c_c2(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                                    let rn = r_c2(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                                    eq(&format!("{tag}: {nm} compress2 ret"), cn, rn);
                                    if c_ie(cn) == 0 {
                                        eq_bytes(&format!("{tag}: {nm} frame"), &cf[..cn], &rf[..rn]);
                                        // matching decoder-side load
                                        let cd = c_dnew();
                                        let rd = r_dnew();
                                        c_dreset(cd, 2);
                                        r_dreset(rd, 2);
                                        let (da, db) = match which {
                                            0 => (c_dload(cd, dp, ds), r_dload(rd, dp, ds)),
                                            1 => (c_dloadref(cd, dp, ds), r_dloadref(rd, dp, ds)),
                                            _ => (
                                                if dct == 0 { c_dpref(cd, dp, ds) } else { c_dprefadv(cd, dp, ds, dct) },
                                                if dct == 0 { r_dpref(rd, dp, ds) } else { r_dprefadv(rd, dp, ds, dct) },
                                            ),
                                        };
                                        eq(&format!("{tag}: decoder {nm} ret"), da, db);
                                        if c_ie(da) == 0 {
                                            let mut cb = vec![0u8; len + 64];
                                            let mut rb = vec![0u8; len + 64];
                                            let (c_dd, r_dd) = p.sym::<F2>("ZSTD_decompressDCtx");
                                            let cx = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn);
                                            let rx = r_dd(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn);
                                            eq(&format!("{tag}: {nm} decode ret"), cx, rx);
                                            if c_ie(cx) == 0 {
                                                eq_bytes(&format!("{tag}: {nm} decode out"), &cb[..cx], &rb[..rx]);
                                            }
                                        }
                                        // dloadadv with out-of-range enums
                                        eq(
                                            &format!("{tag}: DCtx_loadDictionary_advanced({dlm},{dct})"),
                                            c_dloadadv(cd, dp, ds, dlm, dct),
                                            r_dloadadv(rd, dp, ds, dlm, dct),
                                        );
                                        c_dfree(cd);
                                        r_dfree(rd);
                                    }
                                }
                                c_free(cc);
                                r_free(rc);
                            }
                        }
                    }
                }
                c_fcd(ccd);
                r_fcd(rcd);
                c_fdd(cdd);
                r_fdd(rdd);
                c_fdd(cddr_);
                r_fdd(rddr_);
            }
        }

        // F17 — dict == NULL with dictSize > 0
        let cc = c_new();
        let rc = r_new();
        eq(
            "CCtx_loadDictionary(NULL, 16)",
            c_cload(cc, std::ptr::null(), 16),
            r_cload(rc, std::ptr::null(), 16),
        );
        c_free(cc);
        r_free(rc);
        // F4 — NULL handles
        eq("sizeof_CDict(NULL)", c_szcd(std::ptr::null()), r_szcd(std::ptr::null()));
        eq("sizeof_DDict(NULL)", c_szdd(std::ptr::null()), r_szdd(std::ptr::null()));
        eq("getDictID_fromCDict(NULL)", c_idcd(std::ptr::null()), r_idcd(std::ptr::null()));
        eq("getDictID_fromDDict(NULL)", c_iddd(std::ptr::null()), r_iddd(std::ptr::null()));
        // F3 — dictID from non-frame input
        for bad in [vec![], vec![0u8], vec![1u8, 2, 3, 4], gen(Shape::Incompressible, 64, &mut rng)] {
            let bp = if bad.is_empty() { std::ptr::null() } else { bad.as_ptr() as *const c_void };
            eq(
                &format!("getDictID_fromFrame(garbage len={})", bad.len()),
                c_idf(bp, bad.len()),
                r_idf(bp, bad.len()),
            );
        }
    }
}

/// `ZSTD_customMem` — passed by value into the `*_advanced` constructors.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CustomMem {
    pub custom_alloc: Option<unsafe extern "C" fn(*mut c_void, Sz) -> *mut c_void>,
    pub custom_free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

// ============================ rows 48/49 / G — static & custom mem ============================

static mut ALLOC_CALLS: usize = 0;
static mut FREE_CALLS: usize = 0;

unsafe extern "C" fn my_alloc(_opaque: *mut c_void, size: Sz) -> *mut c_void {
    ALLOC_CALLS += 1;
    let layout = std::alloc::Layout::from_size_align(size.max(1), 16).unwrap();
    std::alloc::alloc(layout) as *mut c_void
}

unsafe extern "C" fn my_free(_opaque: *mut c_void, ptr: *mut c_void) {
    FREE_CALLS += 1;
    // Deliberately leak: the size is unknown here and the C never reports it.
    // Leaking keeps the allocator honest without needing a size map.
    let _ = ptr;
}

#[test]
fn row48_row49_static_and_custom_mem() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    type FEstI = unsafe extern "C" fn(c_int) -> Sz;
    type FInitStatic = unsafe extern "C" fn(*mut c_void, Sz) -> *mut c_void;
    type FInitStaticCDict =
        unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, c_int, c_int, CParams) -> *mut c_void;
    type FInitStaticDDict =
        unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, c_int, c_int) -> *mut c_void;
    type FEstCDict = unsafe extern "C" fn(Sz, CParams, c_int) -> Sz;
    type FGetCParams = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;
    type FAdvCtx = unsafe extern "C" fn(CustomMem) -> *mut c_void;
    type FCompressCCtx =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, c_int) -> Sz;

    let (c_ecctx, r_ecctx) = p.sym::<FEstI>("ZSTD_estimateCCtxSize");
    let (c_ecs, r_ecs) = p.sym::<FEstI>("ZSTD_estimateCStreamSize");
    let (c_edctx, r_edctx) = p.sym::<FnVoidSz>("ZSTD_estimateDCtxSize");
    let (c_isc, r_isc) = p.sym::<FInitStatic>("ZSTD_initStaticCCtx");
    let (c_iscs, r_iscs) = p.sym::<FInitStatic>("ZSTD_initStaticCStream");
    let (c_isd, r_isd) = p.sym::<FInitStatic>("ZSTD_initStaticDCtx");
    let (c_isds, r_isds) = p.sym::<FInitStatic>("ZSTD_initStaticDStream");
    let (c_iscd, r_iscd) = p.sym::<FInitStaticCDict>("ZSTD_initStaticCDict");
    let (c_isdd, r_isdd) = p.sym::<FInitStaticDDict>("ZSTD_initStaticDDict");
    let (c_ecd, r_ecd) = p.sym::<FEstCDict>("ZSTD_estimateCDictSize_advanced");
    let (c_gcp, _) = p.sym::<FGetCParams>("ZSTD_getCParams");
    let (c_cc, r_cc) = p.sym::<FCompressCCtx>("ZSTD_compressCCtx");
    let (c_dd, r_dd) = p.sym::<F2>("ZSTD_decompressDCtx");

    let mut rng = Rng::new(SEED ^ 0x48);
    unsafe {
        eq("estimateDCtxSize", c_edctx(), r_edctx());
        for level in [1, 3, 9, 19] {
            let need_c = c_ecctx(level);
            eq(&format!("estimateCCtxSize({level})"), need_c, r_ecctx(level));
            let need_cs = c_ecs(level);
            eq(&format!("estimateCStreamSize({level})"), need_cs, r_ecs(level));
            let need_d = c_edctx();

            // G1/G2/G3/G4/G5 — undersized and unaligned workspaces
            for (nm, need, cf, rf) in [
                ("initStaticCCtx", need_c, &c_isc, &r_isc),
                ("initStaticCStream", need_cs, &c_iscs, &r_iscs),
                ("initStaticDCtx", need_d, &c_isd, &r_isd),
                ("initStaticDStream", need_d, &c_isds, &r_isds),
            ] {
                for delta in [0isize, -1, -8, -(need as isize) / 2, 8, 4096] {
                    let sz = (need as isize + delta).max(0) as usize;
                    let mut ws = vec![0u8; sz + 64];
                    for off in [0usize, 1, 3, 8] {
                        if off >= ws.len() {
                            continue;
                        }
                        let ptr = ws[off..].as_mut_ptr() as *mut c_void;
                        let x = cf(ptr, sz);
                        let mut ws2 = vec![0u8; sz + 64];
                        let ptr2 = ws2[off..].as_mut_ptr() as *mut c_void;
                        let y = rf(ptr2, sz);
                        eq(
                            &format!("{nm}(need={need},sz={sz},off={off}) nullness"),
                            x.is_null(),
                            y.is_null(),
                        );
                    }
                }
                // row 48: a full round trip through an adequately sized static ctx
                if nm == "initStaticCCtx" {
                    let sz = need + 4096;
                    let mut cws = vec![0u8; sz];
                    let mut rws = vec![0u8; sz];
                    let cc = c_isc(cws.as_mut_ptr() as *mut c_void, sz);
                    let rc = r_isc(rws.as_mut_ptr() as *mut c_void, sz);
                    assert!(!cc.is_null() && !rc.is_null());
                    for shape in [Shape::TextLike, Shape::Incompressible] {
                        let len = 1 + rng.below(40_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let cap = c_cb(len) + 64;
                        let mut cf2 = vec![0u8; cap];
                        let mut rf2 = vec![0u8; cap];
                        let cn = c_cc(cc, cf2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        let rn = r_cc(rc, rf2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                        let tag = format!("staticCCtx lvl={level} shape={shape:?} len={len}");
                        eq(&format!("{tag}: compressCCtx ret"), cn, rn);
                        if c_ie(cn) == 0 {
                            eq_bytes(&format!("{tag}: frame"), &cf2[..cn], &rf2[..rn]);
                            // decode through a static DCtx
                            let dsz = need_d + 4096;
                            let mut cdws = vec![0u8; dsz];
                            let mut rdws = vec![0u8; dsz];
                            let cd = c_isd(cdws.as_mut_ptr() as *mut c_void, dsz);
                            let rd = r_isd(rdws.as_mut_ptr() as *mut c_void, dsz);
                            let mut cb = vec![0u8; len + 64];
                            let mut rb = vec![0u8; len + 64];
                            let cx = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf2.as_ptr() as *const c_void, cn);
                            let rx = r_dd(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf2.as_ptr() as *const c_void, rn);
                            eq(&format!("{tag}: static decode ret"), cx, rx);
                            if c_ie(cx) == 0 {
                                eq_bytes(&format!("{tag}: static decode out"), &cb[..cx], &rb[..rx]);
                                eq_bytes(&format!("{tag}: static round trip"), &src, &cb[..cx]);
                            }
                        }
                    }
                }
            }

            // G6/G7 — static CDict / DDict
            let dict = gen(Shape::TextLike, 4096, &mut rng);
            let cp = c_gcp(level, dict.len() as u64, dict.len());
            for dlm in [0, 1] {
                let need = c_ecd(dict.len(), cp, dlm);
                eq(&format!("estimateCDictSize_advanced({dlm})"), need, r_ecd(dict.len(), cp, dlm));
                for delta in [0isize, -1, -64, 4096] {
                    let sz = (need as isize + delta).max(0) as usize;
                    let mut cws = vec![0u8; sz + 64];
                    let mut rws = vec![0u8; sz + 64];
                    for dct in [0, 1, 2, -1, 99] {
                        let x = c_iscd(cws.as_mut_ptr() as *mut c_void, sz, dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cp);
                        let y = r_iscd(rws.as_mut_ptr() as *mut c_void, sz, dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cp);
                        eq(
                            &format!("initStaticCDict(sz={sz},dlm={dlm},dct={dct}) nullness"),
                            x.is_null(),
                            y.is_null(),
                        );
                        let x = c_isdd(cws.as_mut_ptr() as *mut c_void, sz, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                        let y = r_isdd(rws.as_mut_ptr() as *mut c_void, sz, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                        eq(
                            &format!("initStaticDDict(sz={sz},dlm={dlm},dct={dct}) nullness"),
                            x.is_null(),
                            y.is_null(),
                        );
                    }
                }
            }
        }

        // ---- row 49: custom allocators ----
        let cm = CustomMem {
            custom_alloc: Some(my_alloc),
            custom_free: Some(my_free),
            opaque: std::ptr::null_mut(),
        };
        for nm in [
            "ZSTD_createCCtx_advanced",
            "ZSTD_createCStream_advanced",
            "ZSTD_createDCtx_advanced",
            "ZSTD_createDStream_advanced",
        ] {
            let (c, r) = p.sym::<FAdvCtx>(nm);
            for mem in [CustomMem::default(), cm] {
                let x = c(mem);
                let y = r(mem);
                eq(&format!("{nm} nullness"), x.is_null(), y.is_null());
                if !x.is_null() {
                    let fname = nm.replace("create", "free").replace("_advanced", "");
                    let (cfree, rfree) = p.sym::<FnFreeCtx>(&fname);
                    eq(&format!("{fname}"), cfree(x), rfree(y));
                }
            }
        }
        // a real round trip through custom-allocated contexts
        {
            let (c_new, r_new) = p.sym::<FAdvCtx>("ZSTD_createCCtx_advanced");
            let (c_freec, r_freec) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
            let (c_dnew, r_dnew) = p.sym::<FAdvCtx>("ZSTD_createDCtx_advanced");
            let (c_freed, r_freed) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
            let cc = c_new(cm);
            let rc = r_new(cm);
            let cd = c_dnew(cm);
            let rd = r_dnew(cm);
            for level in [1, 9, 19] {
                for shape in ALL_SHAPES {
                    let len = 1 + rng.below(50_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let cap = c_cb(len) + 64;
                    let mut cf = vec![0u8; cap];
                    let mut rf = vec![0u8; cap];
                    let cn = c_cc(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                    let rn = r_cc(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, level);
                    let tag = format!("customMem lvl={level} shape={shape:?} len={len}");
                    eq(&format!("{tag}: compressCCtx ret"), cn, rn);
                    if c_ie(cn) != 0 {
                        continue;
                    }
                    eq_bytes(&format!("{tag}: frame"), &cf[..cn], &rf[..rn]);
                    let mut cb = vec![0u8; len + 64];
                    let mut rb = vec![0u8; len + 64];
                    let cx = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn);
                    let rx = r_dd(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn);
                    eq(&format!("{tag}: decode ret"), cx, rx);
                    if c_ie(cx) == 0 {
                        eq_bytes(&format!("{tag}: decode out"), &cb[..cx], &rb[..rx]);
                        eq_bytes(&format!("{tag}: round trip"), &src, &cb[..cx]);
                    }
                }
            }
            c_freec(cc);
            r_freec(rc);
            c_freed(cd);
            r_freed(rd);
        }
    }
}

// ==================== rows 50-52 / J — the sequence API ====================

#[test]
fn row50_row52_sequence_api() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    type FGenSeq =
        unsafe extern "C" fn(*mut c_void, *mut Sequence, Sz, *const c_void, Sz) -> Sz;
    type FMerge = unsafe extern "C" fn(*mut Sequence, Sz) -> Sz;
    type FCompSeq = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *const Sequence, Sz, *const c_void, Sz,
    ) -> Sz;
    type FCompSeqLit = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *const Sequence, Sz, *const c_void, Sz, Sz, Sz,
    ) -> Sz;
    let (c_gs, r_gs) = p.sym::<FGenSeq>("ZSTD_generateSequences");
    let (c_mg, r_mg) = p.sym::<FMerge>("ZSTD_mergeBlockDelimiters");
    let (c_sb, r_sb) = p.sym::<FnCompressBound>("ZSTD_sequenceBound");
    let (c_cs, r_cs) = p.sym::<FCompSeq>("ZSTD_compressSequences");
    let (c_csl, r_csl) = p.sym::<FCompSeqLit>("ZSTD_compressSequencesAndLiterals");

    let mut rng = Rng::new(SEED ^ 0x50);
    unsafe {
        for s in SIZE_AXIS {
            eq(&format!("ZSTD_sequenceBound({s})"), c_sb(s), r_sb(s));
        }
        for level in [1, 3, 9, 19] {
            for shape in ALL_SHAPES {
                for len in [1usize, 64, 1024, 70_000, 200_000] {
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("seq lvl={level} shape={shape:?} len={len}");
                    let nb_max = c_sb(len).max(1);

                    // row 50 — generateSequences
                    let cc = c_new();
                    let rc = r_new();
                    eq(&format!("{tag}: set level"), c_set(cc, P_LEVEL, level), r_set(rc, P_LEVEL, level));
                    let mut cseq = vec![Sequence::default(); nb_max + 8];
                    let mut rseq = vec![Sequence::default(); nb_max + 8];
                    let cn = c_gs(cc, cseq.as_mut_ptr(), nb_max, src.as_ptr() as *const c_void, len);
                    let rn = r_gs(rc, rseq.as_mut_ptr(), nb_max, src.as_ptr() as *const c_void, len);
                    eq(&format!("{tag}: generateSequences ret"), cn, rn);
                    if c_ie(cn) == 0 {
                        eq(&format!("{tag}: sequences"), &cseq[..cn], &rseq[..rn]);
                    }
                    // J7 — undersized sequence array
                    for small in [0usize, 1, nb_max / 2] {
                        let mut cs2 = vec![Sequence::default(); small.max(1)];
                        let mut rs2 = vec![Sequence::default(); small.max(1)];
                        c_reset(cc, 1);
                        r_reset(rc, 1);
                        eq(
                            &format!("{tag}: generateSequences(nb={small}) ret"),
                            c_gs(cc, cs2.as_mut_ptr(), small, src.as_ptr() as *const c_void, len),
                            r_gs(rc, rs2.as_mut_ptr(), small, src.as_ptr() as *const c_void, len),
                        );
                    }
                    c_free(cc);
                    r_free(rc);
                    if c_ie(cn) != 0 {
                        continue;
                    }

                    // J8 / row 50 — mergeBlockDelimiters
                    let mut cm = cseq[..cn].to_vec();
                    let mut rm = rseq[..rn].to_vec();
                    let cmn = c_mg(cm.as_mut_ptr(), cn);
                    let rmn = r_mg(rm.as_mut_ptr(), rn);
                    eq(&format!("{tag}: mergeBlockDelimiters ret"), cmn, rmn);
                    eq(&format!("{tag}: merged sequences"), &cm[..cmn], &rm[..rmn]);
                    eq(
                        &format!("{tag}: mergeBlockDelimiters(0)"),
                        c_mg(cm.as_mut_ptr(), 0),
                        r_mg(rm.as_mut_ptr(), 0),
                    );

                    // rows 51/52 — compressSequences across the option cross-product
                    for delim in [0, 1] {
                        for validate in [0, 1] {
                            for repcode in [0, 1, 2] {
                                let (seqs_c, seqs_r, nseq) = if delim == 1 {
                                    (&cseq[..cn], &rseq[..rn], cn)
                                } else {
                                    (&cm[..cmn], &rm[..rmn], cmn)
                                };
                                let cap = c_cb(len) + 4096;
                                // ZSTD_compressSequences does NOT forward the
                                // error from ZSTD_writeFrameHeader: it does
                                //   op += frameHeaderSize; assert(frameHeaderSize <= dstCapacity);
                                // so a dstCapacity below ZSTD_FRAMEHEADERSIZE_MAX
                                // makes the C advance `op` by an error code and
                                // write through a wild pointer. Only capacities
                                // that can hold a frame header are in contract.
                                for &dsz in &[cap, (cap / 4).max(64), 64usize] {
                                    let cc = c_new();
                                    let rc = r_new();
                                    c_reset(cc, 2);
                                    r_reset(rc, 2);
                                    let mut skip = false;
                                    for (prm, val) in [
                                        (P_LEVEL, level),
                                        (P_BLOCKDELIM, delim),
                                        (P_VALIDATESEQ, validate),
                                        (P_REPCODERES, repcode),
                                        (P_MINMATCH, 3),
                                    ] {
                                        let a = c_set(cc, prm, val);
                                        let b = r_set(rc, prm, val);
                                        eq(&format!("{tag}: set({prm},{val})"), a, b);
                                        if c_ie(a) != 0 {
                                            skip = true;
                                        }
                                    }
                                    if skip {
                                        c_free(cc);
                                        r_free(rc);
                                        continue;
                                    }
                                    let mut cf = vec![0u8; dsz.max(1)];
                                    let mut rf = vec![0u8; dsz.max(1)];
                                    let cx = c_cs(
                                        cc, cf.as_mut_ptr() as *mut c_void, dsz,
                                        seqs_c.as_ptr(), nseq, src.as_ptr() as *const c_void, len,
                                    );
                                    let rx = r_cs(
                                        rc, rf.as_mut_ptr() as *mut c_void, dsz,
                                        seqs_r.as_ptr(), nseq, src.as_ptr() as *const c_void, len,
                                    );
                                    eq(
                                        &format!("{tag}: compressSequences(d={delim},v={validate},rc={repcode},dsz={dsz}) ret"),
                                        cx, rx,
                                    );
                                    if c_ie(cx) == 0 {
                                        eq_bytes(&format!("{tag}: compressSequences frame"), &cf[..cx], &rf[..rx]);
                                        diff_decompress(p, &cf[..cx], &src, &tag);
                                    }
                                    c_free(cc);
                                    r_free(rc);
                                }
                            }
                        }
                    }

                    // J1/J2/J3/J4 — corrupted sequences
                    for mutate in 0..5u32 {
                        let mut bad_c = cm[..cmn].to_vec();
                        let mut bad_r = rm[..rmn].to_vec();
                        if bad_c.is_empty() {
                            break;
                        }
                        let i = rng.below(bad_c.len() as u32) as usize;
                        let patch = |s: &mut Sequence| match mutate {
                            0 => s.offset = 0,
                            1 => s.offset = u32::MAX,
                            2 => s.match_length = 1,
                            3 => s.lit_length = s.lit_length.wrapping_add(1),
                            _ => s.match_length = s.match_length.wrapping_add(7),
                        };
                        patch(&mut bad_c[i]);
                        patch(&mut bad_r[i]);
                        for validate in [0, 1] {
                            let cc = c_new();
                            let rc = r_new();
                            c_reset(cc, 2);
                            r_reset(rc, 2);
                            for (prm, val) in [(P_LEVEL, level), (P_BLOCKDELIM, 0), (P_VALIDATESEQ, validate), (P_MINMATCH, 3)] {
                                c_set(cc, prm, val);
                                r_set(rc, prm, val);
                            }
                            let cap = c_cb(len) + 4096;
                            let mut cf = vec![0u8; cap];
                            let mut rf = vec![0u8; cap];
                            let cx = c_cs(cc, cf.as_mut_ptr() as *mut c_void, cap, bad_c.as_ptr(), bad_c.len(), src.as_ptr() as *const c_void, len);
                            let rx = r_cs(rc, rf.as_mut_ptr() as *mut c_void, cap, bad_r.as_ptr(), bad_r.len(), src.as_ptr() as *const c_void, len);
                            eq(
                                &format!("{tag}: compressSequences(mutate={mutate},v={validate}) ret"),
                                cx, rx,
                            );
                            if c_ie(cx) == 0 {
                                eq_bytes(&format!("{tag}: mutated frame"), &cf[..cx], &rf[..rx]);
                            }
                            c_free(cc);
                            r_free(rc);
                        }
                    }

                    // row 52 / J10 / J11 — compressSequencesAndLiterals
                    {
                        // literals = the concatenation of the literal runs
                        let mut lits: Vec<u8> = Vec::new();
                        let mut pos = 0usize;
                        for s in &cm[..cmn] {
                            let ll = s.lit_length as usize;
                            if pos + ll <= len {
                                lits.extend_from_slice(&src[pos..pos + ll]);
                            }
                            pos += ll + s.match_length as usize;
                        }
                        for lit_cap in [lits.len(), lits.len() + 64, 0] {
                            for dec_size in [len, 0, len + 1] {
                                let cc = c_new();
                                let rc = r_new();
                                c_reset(cc, 2);
                                r_reset(rc, 2);
                                for (prm, val) in [(P_LEVEL, level), (P_BLOCKDELIM, 0), (P_VALIDATESEQ, 1), (P_MINMATCH, 3)] {
                                    c_set(cc, prm, val);
                                    r_set(rc, prm, val);
                                }
                                let cap = c_cb(len) + 4096;
                                let mut cf = vec![0u8; cap];
                                let mut rf = vec![0u8; cap];
                                let lp = if lits.is_empty() { std::ptr::null() } else { lits.as_ptr() as *const c_void };
                                let cx = c_csl(
                                    cc, cf.as_mut_ptr() as *mut c_void, cap,
                                    cm[..cmn].as_ptr(), cmn, lp, lits.len(), lit_cap, dec_size,
                                );
                                let rx = r_csl(
                                    rc, rf.as_mut_ptr() as *mut c_void, cap,
                                    rm[..rmn].as_ptr(), rmn, lp, lits.len(), lit_cap, dec_size,
                                );
                                eq(
                                    &format!("{tag}: compressSequencesAndLiterals(lc={lit_cap},ds={dec_size}) ret"),
                                    cx, rx,
                                );
                                if c_ie(cx) == 0 {
                                    eq_bytes(&format!("{tag}: CSAL frame"), &cf[..cx], &rf[..rx]);
                                }
                                c_free(cc);
                                r_free(rc);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ==================== rows 55-60 / I — the ZDICT dictionary builder ====================

/// `ZDICT_params_t`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct ZdictParams {
    compression_level: c_int,
    notification_level: c_uint,
    dict_id: c_uint,
}

/// `ZDICT_cover_params_t`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CoverParams {
    k: c_uint,
    d: c_uint,
    steps: c_uint,
    nb_threads: c_uint,
    split_point: f64,
    shrink_dict: c_uint,
    shrink_dict_max_regression: c_uint,
    z: ZdictParams,
}

/// `ZDICT_fastCover_params_t`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct FastCoverParams {
    k: c_uint,
    d: c_uint,
    f: c_uint,
    steps: c_uint,
    nb_threads: c_uint,
    split_point: f64,
    accel: c_uint,
    shrink_dict: c_uint,
    shrink_dict_max_regression: c_uint,
    z: ZdictParams,
}

#[test]
fn row55_row60_dictionary_builder() {
    let p = libs();
    type FTrain = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint) -> Sz;
    type FCover = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint, CoverParams) -> Sz;
    type FCoverOpt = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint, *mut CoverParams) -> Sz;
    type FFast = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint, FastCoverParams) -> Sz;
    type FFastOpt = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint, *mut FastCoverParams) -> Sz;
    type FFinalize = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, *const c_void, *const Sz, c_uint, ZdictParams) -> Sz;
    type FLegacy = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, *const Sz, c_uint, ZdictParams) -> Sz;
    type FAddEnt = unsafe extern "C" fn(*mut c_void, Sz, Sz, *const c_void, *const Sz, c_uint) -> Sz;
    type FDictID = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FHdrSz = unsafe extern "C" fn(*const c_void, Sz) -> Sz;

    let (c_tr, r_tr) = p.sym::<FTrain>("ZDICT_trainFromBuffer");
    let (c_cov, r_cov) = p.sym::<FCover>("ZDICT_trainFromBuffer_cover");
    let (c_covo, r_covo) = p.sym::<FCoverOpt>("ZDICT_optimizeTrainFromBuffer_cover");
    let (c_fc, r_fc) = p.sym::<FFast>("ZDICT_trainFromBuffer_fastCover");
    let (c_fco, r_fco) = p.sym::<FFastOpt>("ZDICT_optimizeTrainFromBuffer_fastCover");
    let (c_fin, r_fin) = p.sym::<FFinalize>("ZDICT_finalizeDictionary");
    let (c_leg, r_leg) = p.sym::<FLegacy>("ZDICT_trainFromBuffer_legacy");
    let (c_add, r_add) = p.sym::<FAddEnt>("ZDICT_addEntropyTablesFromBuffer");
    let (c_id, r_id) = p.sym::<FDictID>("ZDICT_getDictID");
    let (c_hs, r_hs) = p.sym::<FHdrSz>("ZDICT_getDictHeaderSize");
    let (c_zie, _) = p.sym::<FnIsError>("ZDICT_isError");

    let mut rng = Rng::new(SEED ^ 0x55);
    unsafe {
        // I1/I2/I3 — dictID and header size on garbage
        for buf in [
            vec![],
            vec![0u8],
            vec![0u8; 8],
            vec![0u8; 9],
            vec![0xFFu8; 32],
            gen(Shape::Incompressible, 128, &mut rng),
        ] {
            let bp = if buf.is_empty() { std::ptr::null() } else { buf.as_ptr() as *const c_void };
            eq(
                &format!("ZDICT_getDictID(len={})", buf.len()),
                c_id(bp, buf.len()),
                r_id(bp, buf.len()),
            );
            eq(
                &format!("ZDICT_getDictHeaderSize(len={})", buf.len()),
                c_hs(bp, buf.len()),
                r_hs(bp, buf.len()),
            );
        }

        // row 55 / I6/I7/I8 — trainFromBuffer over sample-count and capacity axes
        let mut trained: Vec<Vec<u8>> = Vec::new();
        for &nb in &[0usize, 1, 8, 64, 512] {
            for &(mn, mx) in &[(8usize, 32usize), (64, 512), (512, 2048)] {
                let (buf, sizes) = corpus(&mut rng, nb, mn, mx);
                for &cap in &[0usize, 100, 256, 4096, 65_536, 110_000] {
                    let mut cd = vec![0u8; cap.max(1)];
                    let mut rd = vec![0u8; cap.max(1)];
                    let bp = if buf.is_empty() { std::ptr::null() } else { buf.as_ptr() as *const c_void };
                    let sp = if sizes.is_empty() { std::ptr::null() } else { sizes.as_ptr() };
                    let a = c_tr(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb as c_uint);
                    let b = r_tr(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb as c_uint);
                    let tag = format!("ZDICT_trainFromBuffer nb={nb} sizes=({mn},{mx}) cap={cap}");
                    eq(&format!("{tag} ret"), a, b);
                    if c_zie(a) == 0 && a > 0 {
                        eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                        eq(
                            &format!("{tag} dictID"),
                            c_id(cd.as_ptr() as *const c_void, a),
                            r_id(rd.as_ptr() as *const c_void, b),
                        );
                        eq(
                            &format!("{tag} headerSize"),
                            c_hs(cd.as_ptr() as *const c_void, a),
                            r_hs(rd.as_ptr() as *const c_void, b),
                        );
                        if trained.len() < 4 {
                            trained.push(cd[..a].to_vec());
                        }
                    }
                }
            }
        }

        // rows 56/57 / I9-I13 — cover and fastCover, all parameter axes
        let (buf, sizes) = corpus(&mut rng, 256, 64, 1024);
        let bp = buf.as_ptr() as *const c_void;
        let sp = sizes.as_ptr();
        let nb = sizes.len() as c_uint;
        for &k in &[0u32, 16, 50, 200, 1024] {
            for &d in &[0u32, 6, 8, 7, 9] {
                for &steps in &[0u32, 4] {
                    for &split in &[0.0f64, 0.75, 1.0, 1.5, -1.0] {
                        for &shrink in &[0u32, 1] {
                            let z = ZdictParams { compression_level: 3, notification_level: 0, dict_id: 0 };
                            let cp = CoverParams {
                                k, d, steps, nb_threads: 1, split_point: split,
                                shrink_dict: shrink, shrink_dict_max_regression: 1, z,
                            };
                            for &cap in &[256usize, 8192] {
                                let mut cd = vec![0u8; cap];
                                let mut rd = vec![0u8; cap];
                                let a = c_cov(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, cp);
                                let b = r_cov(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, cp);
                                let tag = format!("cover k={k} d={d} steps={steps} split={split} shrink={shrink} cap={cap}");
                                eq(&format!("{tag} ret"), a, b);
                                if c_zie(a) == 0 && a > 0 {
                                    eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                                }
                            }
                            for &f in &[0u32, 15, 20, 23, 25, 32] {
                                for &accel in &[0u32, 1, 2, 10, 11] {
                                    let fp = FastCoverParams {
                                        k, d, f, steps, nb_threads: 1, split_point: split,
                                        accel, shrink_dict: shrink, shrink_dict_max_regression: 1, z,
                                    };
                                    let cap = 8192usize;
                                    let mut cd = vec![0u8; cap];
                                    let mut rd = vec![0u8; cap];
                                    let a = c_fc(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, fp);
                                    let b = r_fc(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, fp);
                                    let tag = format!("fastCover k={k} d={d} f={f} accel={accel} steps={steps} split={split} shrink={shrink}");
                                    eq(&format!("{tag} ret"), a, b);
                                    if c_zie(a) == 0 && a > 0 {
                                        eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // the optimize* variants also write back their chosen parameters
        for &d in &[6u32, 8] {
            for &steps in &[0u32, 2, 4] {
                for &split in &[0.0f64, 0.75, 1.0] {
                    let z = ZdictParams { compression_level: 1, notification_level: 0, dict_id: 0 };
                    let mut ccp = CoverParams {
                        k: 0, d, steps, nb_threads: 1, split_point: split,
                        shrink_dict: 0, shrink_dict_max_regression: 1, z,
                    };
                    let mut rcp = ccp;
                    let cap = 8192usize;
                    let mut cd = vec![0u8; cap];
                    let mut rd = vec![0u8; cap];
                    let a = c_covo(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, &mut ccp);
                    let b = r_covo(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, &mut rcp);
                    let tag = format!("optimizeCover d={d} steps={steps} split={split}");
                    eq(&format!("{tag} ret"), a, b);
                    eq(&format!("{tag} k"), ccp.k, rcp.k);
                    eq(&format!("{tag} d"), ccp.d, rcp.d);
                    eq(&format!("{tag} steps"), ccp.steps, rcp.steps);
                    eq(&format!("{tag} splitPoint bits"), ccp.split_point.to_bits(), rcp.split_point.to_bits());
                    eq(&format!("{tag} z"), ccp.z, rcp.z);
                    if c_zie(a) == 0 && a > 0 {
                        eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                    }

                    for &f in &[0u32, 20, 23] {
                        for &accel in &[0u32, 1, 5] {
                            let mut cfp = FastCoverParams {
                                k: 0, d, f, steps, nb_threads: 1, split_point: split,
                                accel, shrink_dict: 0, shrink_dict_max_regression: 1, z,
                            };
                            let mut rfp = cfp;
                            let mut cd = vec![0u8; cap];
                            let mut rd = vec![0u8; cap];
                            let a = c_fco(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, &mut cfp);
                            let b = r_fco(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, &mut rfp);
                            let tag = format!("optimizeFastCover d={d} f={f} accel={accel} steps={steps} split={split}");
                            eq(&format!("{tag} ret"), a, b);
                            eq(&format!("{tag} k"), cfp.k, rfp.k);
                            eq(&format!("{tag} d"), cfp.d, rfp.d);
                            eq(&format!("{tag} f"), cfp.f, rfp.f);
                            eq(&format!("{tag} accel"), cfp.accel, rfp.accel);
                            eq(&format!("{tag} splitPoint bits"), cfp.split_point.to_bits(), rfp.split_point.to_bits());
                            eq(&format!("{tag} z"), cfp.z, rfp.z);
                            if c_zie(a) == 0 && a > 0 {
                                eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                            }
                        }
                    }
                }
            }
        }

        // rows 58/59 / I4/I5/I15 — finalize, legacy trainer, addEntropyTables
        for content in [
            gen(Shape::TextLike, 1024, &mut rng),
            gen(Shape::Repetitive, 4096, &mut rng),
            Vec::new(),
        ] {
            for &lvl in &[0i32, 1, 9, 19] {
                for &dict_id in &[0u32, 1, 0xDEAD_BEEF] {
                    let z = ZdictParams { compression_level: lvl, notification_level: 0, dict_id };
                    for &cap in &[0usize, 100, 256, content.len(), content.len() + 4096, 65_536] {
                        let mut cd = vec![0u8; cap.max(1)];
                        let mut rd = vec![0u8; cap.max(1)];
                        let cpz = if content.is_empty() { std::ptr::null() } else { content.as_ptr() as *const c_void };
                        let a = c_fin(cd.as_mut_ptr() as *mut c_void, cap, cpz, content.len(), bp, sp, nb, z);
                        let b = r_fin(rd.as_mut_ptr() as *mut c_void, cap, cpz, content.len(), bp, sp, nb, z);
                        let tag = format!("finalizeDictionary clen={} lvl={lvl} id={dict_id} cap={cap}", content.len());
                        eq(&format!("{tag} ret"), a, b);
                        if c_zie(a) == 0 && a > 0 {
                            eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                            eq(
                                &format!("{tag} dictID"),
                                c_id(cd.as_ptr() as *const c_void, a),
                                r_id(rd.as_ptr() as *const c_void, b),
                            );
                        }
                    }
                }
            }
        }
        for &lvl in &[0i32, 1, 9] {
            let z = ZdictParams { compression_level: lvl, notification_level: 0, dict_id: 0 };
            for &cap in &[0usize, 256, 8192, 65_536] {
                let mut cd = vec![0u8; cap.max(1)];
                let mut rd = vec![0u8; cap.max(1)];
                let a = c_leg(cd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, z);
                let b = r_leg(rd.as_mut_ptr() as *mut c_void, cap, bp, sp, nb, z);
                let tag = format!("trainFromBuffer_legacy lvl={lvl} cap={cap}");
                eq(&format!("{tag} ret"), a, b);
                if c_zie(a) == 0 && a > 0 {
                    eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                }
            }
        }
        for d in &trained {
            for &cap in &[d.len(), d.len() + 4096, 65_536] {
                if cap < d.len() {
                    continue;
                }
                // Contract: ZDICT_addEntropyTablesFromBuffer_advanced reads the
                // dictionary content from `dictBuffer + dictBufferCapacity -
                // dictContentSize`, i.e. the content must sit at the END of the
                // buffer (it hashes that region for the generated dictID and
                // memmoves it down afterwards).
                let mut cd = vec![0u8; cap];
                let mut rd = vec![0u8; cap];
                cd[cap - d.len()..].copy_from_slice(d);
                rd[cap - d.len()..].copy_from_slice(d);
                let a = c_add(cd.as_mut_ptr() as *mut c_void, d.len(), cap, bp, sp, nb);
                let b = r_add(rd.as_mut_ptr() as *mut c_void, d.len(), cap, bp, sp, nb);
                let tag = format!("addEntropyTablesFromBuffer dlen={} cap={cap}", d.len());
                eq(&format!("{tag} ret"), a, b);
                if c_zie(a) == 0 && a > 0 {
                    eq_bytes(&format!("{tag} dict"), &cd[..a], &rd[..b]);
                    eq_bytes(&format!("{tag} whole buffer"), &cd, &rd);
                }
            }
        }

        // row 60 — every trained dictionary drives a real round trip
        let (c_cud, r_cud) = p.sym::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz, c_int) -> Sz,
        >("ZSTD_compress_usingDict");
        let (c_dud, r_dud) = p.sym::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz) -> Sz,
        >("ZSTD_decompress_usingDict");
        let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
        let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
        let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
        let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
        let (c_cbnd, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
        let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
        for d in &trained {
            for level in [1, 9, 19] {
                for shape in [Shape::TextLike, Shape::Repetitive] {
                    let len = 1 + rng.below(30_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let cap = c_cbnd(len) + 64;
                    let cc = c_new();
                    let rc = r_new();
                    let mut cf = vec![0u8; cap];
                    let mut rf = vec![0u8; cap];
                    let cn = c_cud(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, d.as_ptr() as *const c_void, d.len(), level);
                    let rn = r_cud(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, d.as_ptr() as *const c_void, d.len(), level);
                    let tag = format!("trained-dict round trip lvl={level} shape={shape:?} len={len}");
                    eq(&format!("{tag} compress ret"), cn, rn);
                    if c_ie(cn) == 0 {
                        eq_bytes(&format!("{tag} frame"), &cf[..cn], &rf[..rn]);
                        let cd = c_dnew();
                        let rd = r_dnew();
                        let mut cb = vec![0u8; len + 64];
                        let mut rb = vec![0u8; len + 64];
                        let cx = c_dud(cd, cb.as_mut_ptr() as *mut c_void, cb.len(), cf.as_ptr() as *const c_void, cn, d.as_ptr() as *const c_void, d.len());
                        let rx = r_dud(rd, rb.as_mut_ptr() as *mut c_void, rb.len(), rf.as_ptr() as *const c_void, rn, d.as_ptr() as *const c_void, d.len());
                        eq(&format!("{tag} decode ret"), cx, rx);
                        if c_ie(cx) == 0 {
                            eq_bytes(&format!("{tag} decode out"), &cb[..cx], &rb[..rx]);
                            eq_bytes(&format!("{tag} round trip"), &src, &cb[..cx]);
                        }
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                    c_free(cc);
                    r_free(rc);
                }
            }
        }
    }
}

// ==================== rows 61/62 / L — the deprecated ZBUFF API ====================

#[test]
fn row61_row62_zbuff() {
    let p = libs();
    type FNew = unsafe extern "C" fn() -> *mut c_void;
    type FNewAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FInit = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, c_int) -> Sz;
    type FInitAdv = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, Params, u64) -> Sz;
    type FCont = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut Sz, *const c_void, *mut Sz) -> Sz;
    type FFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut Sz) -> Sz;
    type FDInit = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FDInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FGetParams = unsafe extern "C" fn(c_int, u64, Sz) -> Params;

    let (c_cnew, r_cnew) = p.sym::<FNew>("ZBUFF_createCCtx");
    let (c_cnewa, r_cnewa) = p.sym::<FNewAdv>("ZBUFF_createCCtx_advanced");
    let (c_cfree, r_cfree) = p.sym::<FFree>("ZBUFF_freeCCtx");
    let (c_cinit, r_cinit) = p.sym::<FInit>("ZBUFF_compressInit");
    let (c_cinitd, r_cinitd) = p.sym::<FInitDict>("ZBUFF_compressInitDictionary");
    let (c_cinita, r_cinita) = p.sym::<FInitAdv>("ZBUFF_compressInit_advanced");
    let (c_ccont, r_ccont) = p.sym::<FCont>("ZBUFF_compressContinue");
    let (c_cflush, r_cflush) = p.sym::<FFlush>("ZBUFF_compressFlush");
    let (c_cend, r_cend) = p.sym::<FFlush>("ZBUFF_compressEnd");
    let (c_dnew, r_dnew) = p.sym::<FNew>("ZBUFF_createDCtx");
    let (c_dnewa, r_dnewa) = p.sym::<FNewAdv>("ZBUFF_createDCtx_advanced");
    let (c_dfree, r_dfree) = p.sym::<FFree>("ZBUFF_freeDCtx");
    let (c_dinit, r_dinit) = p.sym::<FDInit>("ZBUFF_decompressInit");
    let (c_dinitd, r_dinitd) = p.sym::<FDInitDict>("ZBUFF_decompressInitDictionary");
    let (c_dcont, r_dcont) = p.sym::<FCont>("ZBUFF_decompressContinue");
    let (c_cin, r_cin) = p.sym::<FnVoidSz>("ZBUFF_recommendedCInSize");
    let (c_cout, r_cout) = p.sym::<FnVoidSz>("ZBUFF_recommendedCOutSize");
    let (c_din, r_din) = p.sym::<FnVoidSz>("ZBUFF_recommendedDInSize");
    let (c_dout, r_dout) = p.sym::<FnVoidSz>("ZBUFF_recommendedDOutSize");
    let (c_ie, _) = p.sym::<FnIsError>("ZBUFF_isError");
    let (c_gp, _) = p.sym::<FGetParams>("ZSTD_getParams");

    let mut rng = Rng::new(SEED ^ 0x61);
    unsafe {
        eq("ZBUFF_recommendedCInSize", c_cin(), r_cin());
        eq("ZBUFF_recommendedCOutSize", c_cout(), r_cout());
        eq("ZBUFF_recommendedDInSize", c_din(), r_din());
        eq("ZBUFF_recommendedDOutSize", c_dout(), r_dout());

        // L2 — out-of-range compression levels are clamped like ZSTD_c_compressionLevel
        for mode in 0..3 {
            let dict = gen(Shape::TextLike, 2048, &mut rng);
            for level in [i32::MIN, -1000, -1, 0, 1, 3, 19, 22, 23, 1000, i32::MAX] {
                for in_pat in 0..5u32 {
                    for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive] {
                        let len = 1 + rng.below(60_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag = format!("zbuff mode={mode} lvl={level} inpat={in_pat} shape={shape:?} len={len}");

                        let cc = if mode == 2 { c_cnewa(CustomMem::default()) } else { c_cnew() };
                        let rc = if mode == 2 { r_cnewa(CustomMem::default()) } else { r_cnew() };
                        assert!(!cc.is_null() && !rc.is_null(), "{tag}: createCCtx");
                        let (a, b) = match mode {
                            0 => (c_cinit(cc, level), r_cinit(rc, level)),
                            1 => (
                                c_cinitd(cc, dict.as_ptr() as *const c_void, dict.len(), level),
                                r_cinitd(rc, dict.as_ptr() as *const c_void, dict.len(), level),
                            ),
                            _ => {
                                let prm = c_gp(level, len as u64, 0);
                                (
                                    c_cinita(cc, std::ptr::null(), 0, prm, len as u64),
                                    r_cinita(rc, std::ptr::null(), 0, prm, len as u64),
                                )
                            }
                        };
                        eq(&format!("{tag}: compressInit ret"), a, b);
                        if c_ie(a) != 0 {
                            c_cfree(cc);
                            r_cfree(rc);
                            continue;
                        }

                        let mut cout_buf = vec![0u8; c_cout()];
                        let mut rout_buf = vec![0u8; c_cout()];
                        let mut cstream: Vec<u8> = Vec::new();
                        let mut rstream: Vec<u8> = Vec::new();
                        let mut off = 0usize;
                        let mut failed = false;
                        while off < len {
                            let n = match in_pat {
                                0 => 1,
                                1 => 7,
                                2 => 4096,
                                3 => c_cin(),
                                _ => 1 + rng.below(9000) as usize,
                            }
                            .min(len - off);
                            let mut cdst = cout_buf.len();
                            let mut rdst = rout_buf.len();
                            let mut csrc = n;
                            let mut rsrc = n;
                            let cr = c_ccont(cc, cout_buf.as_mut_ptr() as *mut c_void, &mut cdst, src[off..].as_ptr() as *const c_void, &mut csrc);
                            let rr = r_ccont(rc, rout_buf.as_mut_ptr() as *mut c_void, &mut rdst, src[off..].as_ptr() as *const c_void, &mut rsrc);
                            eq(&format!("{tag}: compressContinue ret"), cr, rr);
                            if c_ie(cr) != 0 { failed = true; break; }
                            eq(&format!("{tag}: compressContinue dstPos"), cdst, rdst);
                            eq(&format!("{tag}: compressContinue srcPos"), csrc, rsrc);
                            eq_bytes(&format!("{tag}: compressContinue out"), &cout_buf[..cdst], &rout_buf[..rdst]);
                            cstream.extend_from_slice(&cout_buf[..cdst]);
                            rstream.extend_from_slice(&rout_buf[..rdst]);
                            off += csrc;
                            if csrc == 0 {
                                // no progress on this chunk: flush and retry
                                let mut cd2 = cout_buf.len();
                                let mut rd2 = rout_buf.len();
                                let cr = c_cflush(cc, cout_buf.as_mut_ptr() as *mut c_void, &mut cd2);
                                let rr = r_cflush(rc, rout_buf.as_mut_ptr() as *mut c_void, &mut rd2);
                                eq(&format!("{tag}: compressFlush ret"), cr, rr);
                                if c_ie(cr) != 0 { failed = true; break; }
                                eq(&format!("{tag}: compressFlush dstPos"), cd2, rd2);
                                eq_bytes(&format!("{tag}: compressFlush out"), &cout_buf[..cd2], &rout_buf[..rd2]);
                                cstream.extend_from_slice(&cout_buf[..cd2]);
                                rstream.extend_from_slice(&rout_buf[..rd2]);
                            }
                        }
                        if !failed {
                            loop {
                                let mut cd2 = cout_buf.len();
                                let mut rd2 = rout_buf.len();
                                let cr = c_cend(cc, cout_buf.as_mut_ptr() as *mut c_void, &mut cd2);
                                let rr = r_cend(rc, rout_buf.as_mut_ptr() as *mut c_void, &mut rd2);
                                eq(&format!("{tag}: compressEnd ret"), cr, rr);
                                if c_ie(cr) != 0 { failed = true; break; }
                                eq(&format!("{tag}: compressEnd dstPos"), cd2, rd2);
                                eq_bytes(&format!("{tag}: compressEnd out"), &cout_buf[..cd2], &rout_buf[..rd2]);
                                cstream.extend_from_slice(&cout_buf[..cd2]);
                                rstream.extend_from_slice(&rout_buf[..rd2]);
                                if cr == 0 { break; }
                            }
                        }
                        eq(&format!("{tag}: freeCCtx"), c_cfree(cc), r_cfree(rc));
                        if failed {
                            continue;
                        }
                        eq_bytes(&format!("{tag}: whole zbuff stream"), &cstream, &rstream);

                        // row 62 — decode it back through ZBUFF_decompress*
                        for dmode in 0..3 {
                            let cd = if dmode == 2 { c_dnewa(CustomMem::default()) } else { c_dnew() };
                            let rd = if dmode == 2 { r_dnewa(CustomMem::default()) } else { r_dnew() };
                            let (a, b) = if dmode == 1 || (mode == 1 && dmode != 1) {
                                (
                                    c_dinitd(cd, dict.as_ptr() as *const c_void, dict.len()),
                                    r_dinitd(rd, dict.as_ptr() as *const c_void, dict.len()),
                                )
                            } else {
                                (c_dinit(cd), r_dinit(rd))
                            };
                            eq(&format!("{tag}: decompressInit(dmode={dmode}) ret"), a, b);
                            if c_ie(a) != 0 {
                                c_dfree(cd);
                                r_dfree(rd);
                                continue;
                            }
                            let mut cdo = vec![0u8; c_dout()];
                            let mut rdo = vec![0u8; c_dout()];
                            let mut cacc: Vec<u8> = Vec::new();
                            let mut racc: Vec<u8> = Vec::new();
                            let mut off = 0usize;
                            let mut dfailed = false;
                            while off < cstream.len() {
                                let n = (match in_pat { 0 => 1, 1 => 7, 2 => 4096, 3 => c_din(), _ => 8191 })
                                    .min(cstream.len() - off);
                                let mut cdst = cdo.len();
                                let mut rdst = rdo.len();
                                let mut csrc = n;
                                let mut rsrc = n;
                                let cr = c_dcont(cd, cdo.as_mut_ptr() as *mut c_void, &mut cdst, cstream[off..].as_ptr() as *const c_void, &mut csrc);
                                let rr = r_dcont(rd, rdo.as_mut_ptr() as *mut c_void, &mut rdst, rstream[off..].as_ptr() as *const c_void, &mut rsrc);
                                eq(&format!("{tag}: decompressContinue(dmode={dmode}) ret"), cr, rr);
                                if c_ie(cr) != 0 { dfailed = true; break; }
                                eq(&format!("{tag}: decompressContinue dstPos"), cdst, rdst);
                                eq(&format!("{tag}: decompressContinue srcPos"), csrc, rsrc);
                                eq_bytes(&format!("{tag}: decompressContinue out"), &cdo[..cdst], &rdo[..rdst]);
                                cacc.extend_from_slice(&cdo[..cdst]);
                                racc.extend_from_slice(&rdo[..rdst]);
                                if csrc == 0 && cdst == 0 { break; }
                                off += csrc;
                            }
                            eq(&format!("{tag}: freeDCtx"), c_dfree(cd), r_dfree(rd));
                            if dfailed { continue; }
                            eq_bytes(&format!("{tag}: zbuff decode(dmode={dmode})"), &cacc, &racc);
                        }

                        // L3 — garbage into decompressContinue
                        let cd = c_dnew();
                        let rd = r_dnew();
                        c_dinit(cd);
                        r_dinit(rd);
                        let garbage = gen(Shape::Incompressible, 64, &mut rng);
                        let mut cdo = vec![0u8; 4096];
                        let mut rdo = vec![0u8; 4096];
                        let mut cdst = cdo.len();
                        let mut rdst = rdo.len();
                        let mut csrc = garbage.len();
                        let mut rsrc = garbage.len();
                        eq(
                            &format!("{tag}: decompressContinue(garbage) ret"),
                            c_dcont(cd, cdo.as_mut_ptr() as *mut c_void, &mut cdst, garbage.as_ptr() as *const c_void, &mut csrc),
                            r_dcont(rd, rdo.as_mut_ptr() as *mut c_void, &mut rdst, garbage.as_ptr() as *const c_void, &mut rsrc),
                        );
                        eq(&format!("{tag}: garbage dstPos"), cdst, rdst);
                        eq(&format!("{tag}: garbage srcPos"), csrc, rsrc);
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                }
            }
        }
    }
}

// ============================ row 71 — POOL_* ============================

unsafe extern "C" fn pool_job(_opaque: *mut c_void) {}

#[test]
fn row71_pool() {
    let p = libs();
    type FCreate = unsafe extern "C" fn(Sz, Sz) -> *mut c_void;
    type FCreateAdv = unsafe extern "C" fn(Sz, Sz, CustomMem) -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void);
    type FSizeof = unsafe extern "C" fn(*const c_void) -> Sz;
    type FResize = unsafe extern "C" fn(*mut c_void, Sz) -> c_int;
    type FAdd = unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*mut c_void)>, *mut c_void);
    type FTryAdd = unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*mut c_void)>, *mut c_void) -> c_int;
    type FJoin = unsafe extern "C" fn(*mut c_void);

    let (c_cr, r_cr) = p.sym::<FCreate>("POOL_create");
    let (c_cra, r_cra) = p.sym::<FCreateAdv>("POOL_create_advanced");
    let (c_fr, r_fr) = p.sym::<FFree>("POOL_free");
    let (c_sz, r_sz) = p.sym::<FSizeof>("POOL_sizeof");
    let (c_rs, r_rs) = p.sym::<FResize>("POOL_resize");
    let (c_ad, r_ad) = p.sym::<FAdd>("POOL_add");
    let (c_ta, r_ta) = p.sym::<FTryAdd>("POOL_tryAdd");
    let (c_jj, r_jj) = p.sym::<FJoin>("POOL_joinJobs");

    unsafe {
        for nb in [0usize, 1, 2, 4] {
            for qs in [0usize, 1, 4, 16] {
                let cp = c_cr(nb, qs);
                let rp = r_cr(nb, qs);
                eq(&format!("POOL_create({nb},{qs}) nullness"), cp.is_null(), rp.is_null());
                if !cp.is_null() {
                    eq(&format!("POOL_sizeof({nb},{qs})"), c_sz(cp), r_sz(rp));
                    for newnb in [0usize, 1, 2, 8] {
                        eq(
                            &format!("POOL_resize({nb},{qs}->{newnb})"),
                            c_rs(cp, newnb),
                            r_rs(rp, newnb),
                        );
                        eq(&format!("POOL_sizeof after resize {newnb}"), c_sz(cp), r_sz(rp));
                    }
                    for _ in 0..8 {
                        c_ad(cp, Some(pool_job), std::ptr::null_mut());
                        r_ad(rp, Some(pool_job), std::ptr::null_mut());
                        eq(
                            "POOL_tryAdd",
                            c_ta(cp, Some(pool_job), std::ptr::null_mut()),
                            r_ta(rp, Some(pool_job), std::ptr::null_mut()),
                        );
                    }
                    c_jj(cp);
                    r_jj(rp);
                    eq("POOL_sizeof after join", c_sz(cp), r_sz(rp));
                }
                c_fr(cp);
                r_fr(rp);

                let cp = c_cra(nb, qs, CustomMem::default());
                let rp = r_cra(nb, qs, CustomMem::default());
                eq(
                    &format!("POOL_create_advanced({nb},{qs}) nullness"),
                    cp.is_null(),
                    rp.is_null(),
                );
                if !cp.is_null() {
                    eq(&format!("POOL_sizeof_advanced({nb},{qs})"), c_sz(cp), r_sz(rp));
                    c_jj(cp);
                    r_jj(rp);
                }
                c_fr(cp);
                r_fr(rp);
            }
        }
        // NULL handling
        eq("POOL_sizeof(NULL)", c_sz(std::ptr::null()), r_sz(std::ptr::null()));
        c_fr(std::ptr::null_mut());
        r_fr(std::ptr::null_mut());
    }
}
