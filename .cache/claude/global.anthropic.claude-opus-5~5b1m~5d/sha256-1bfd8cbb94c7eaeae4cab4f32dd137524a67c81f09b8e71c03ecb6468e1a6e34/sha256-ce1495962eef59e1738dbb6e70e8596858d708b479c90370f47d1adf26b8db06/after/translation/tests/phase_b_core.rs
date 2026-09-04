//! Phase B — CONFIGS.md rows 1..30: pure helpers, bounds, one-shot
//! compress/decompress, frame introspection.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------------ row 1, 2, 13

#[test]
fn row01_version() {
    unsafe {
        let (c, r) = duo::<FnUint0>("ZSTD_versionNumber");
        eqv("ZSTD_versionNumber", c(), r());
        let (c, r) = duo::<unsafe extern "C" fn() -> *const c_char>("ZSTD_versionString");
        eqv("ZSTD_versionString", cstr(c()), cstr(r()));
        let (c, r) = duo::<FnUint0>("ZSTD_XXH_versionNumber");
        eqv("ZSTD_XXH_versionNumber", c(), r());
        let (c, r) = duo::<FnUint0>("FSE_versionNumber");
        eqv("FSE_versionNumber", c(), r());
    }
}

#[test]
fn row02_clevels() {
    unsafe {
        for n in ["ZSTD_maxCLevel", "ZSTD_minCLevel", "ZSTD_defaultCLevel"] {
            let (c, r) = duo::<FnInt0>(n);
            eqv(n, c(), r());
        }
    }
}

#[test]
fn row13_stream_sizes() {
    unsafe {
        for n in [
            "ZSTD_CStreamInSize",
            "ZSTD_CStreamOutSize",
            "ZSTD_DStreamInSize",
            "ZSTD_DStreamOutSize",
        ] {
            let (c, r) = duo::<FnSizeT0>(n);
            eqv(n, c(), r());
        }
        for n in [
            "ZBUFF_recommendedCInSize",
            "ZBUFF_recommendedCOutSize",
            "ZBUFF_recommendedDInSize",
            "ZBUFF_recommendedDOutSize",
        ] {
            let (c, r) = duo::<FnSizeT0>(n);
            eqv(n, c(), r());
        }
    }
}

// ------------------------------------------------------------------ row 3

#[test]
fn row03_compress_bound() {
    unsafe {
        let (c, r) = duo::<FnSizeT1>("ZSTD_compressBound");
        let mut cases: Vec<usize> = vec![
            0,
            1,
            2,
            3,
            127,
            128,
            129,
            1 << 10,
            (1 << 17) - 1,
            1 << 17,
            (1 << 17) + 1,
            1 << 20,
            1 << 24,
            1 << 30,
            0xFF00FF00,
            usize::MAX / 2,
            usize::MAX - 1,
            usize::MAX,
        ];
        let mut rng = Rng::new(0xB0011);
        for _ in 0..2000 {
            cases.push(rng.next_u64() as usize >> (rng.below(64) as u32));
        }
        for s in cases {
            eqv(&format!("ZSTD_compressBound({s})"), c(s), r(s));
        }
        // HUF_compressBound / FSE-side bounds
        let (c, r) = duo::<FnSizeT1>("HUF_compressBound");
        for s in [0usize, 1, 2, 1000, 1 << 17, 1 << 24] {
            eqv(&format!("HUF_compressBound({s})"), c(s), r(s));
        }
        let (c, r) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        for s in [0usize, 1, 3, 100, 1 << 17, 1 << 24, usize::MAX / 4] {
            eqv(&format!("ZSTD_sequenceBound({s})"), c(s), r(s));
        }
    }
}

// ------------------------------------------------------------------ row 5, 146

#[test]
fn row05_error_strings() {
    unsafe {
        let (isec, iser) = duo::<FnIsError>("ZSTD_isError");
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (gnc, gnr) = duo::<FnErrName>("ZSTD_getErrorName");
        let (gsc, gsr) = duo::<unsafe extern "C" fn(c_uint) -> *const c_char>("ZSTD_getErrorString");
        let (esc, esr) = duo::<unsafe extern "C" fn(c_int) -> *const c_char>("ERR_getErrorString");

        for code in 0..140u32 {
            eqv(
                &format!("ZSTD_getErrorString({code})"),
                cstr(gsc(code)),
                cstr(gsr(code)),
            );
            eqv(
                &format!("ERR_getErrorString({code})"),
                cstr(esc(code as c_int)),
                cstr(esr(code as c_int)),
            );
        }
        // negative / out-of-range enum values crossing the FFI boundary
        for code in [-1i32, -100, 1000, i32::MIN, i32::MAX] {
            eqv(
                &format!("ERR_getErrorString({code})"),
                cstr(esc(code)),
                cstr(esr(code)),
            );
            eqv(
                &format!("ZSTD_getErrorString({code})"),
                cstr(gsc(code as c_uint)),
                cstr(gsr(code as c_uint)),
            );
        }
        let mut vals: Vec<usize> = vec![0, 1, 2, 100, usize::MAX];
        for n in 0..200usize {
            vals.push(usize::MAX - n);
        }
        let mut rng = Rng::new(5);
        for _ in 0..500 {
            vals.push(rng.next_u64() as usize);
        }
        for v in vals {
            eqv(&format!("ZSTD_isError({v})"), isec(v), iser(v));
            eqv(&format!("ZSTD_getErrorCode({v})"), gcc(v), gcr(v));
            eqv(
                &format!("ZSTD_getErrorName({v})"),
                cstr(gnc(v)),
                cstr(gnr(v)),
            );
        }
        // sibling error surfaces
        for (ise, nm) in [
            ("FSE_isError", "FSE_getErrorName"),
            ("HUF_isError", "HUF_getErrorName"),
            ("HIST_isError", ""),
            ("ZDICT_isError", "ZDICT_getErrorName"),
            ("ZBUFF_isError", "ZBUFF_getErrorName"),
        ] {
            let (a, b) = duo::<FnIsError>(ise);
            for v in [0usize, 1, 10, usize::MAX, usize::MAX - 20, usize::MAX - 200] {
                eqv(&format!("{ise}({v})"), a(v), b(v));
            }
            if !nm.is_empty() {
                let (a, b) = duo::<FnErrName>(nm);
                for v in [0usize, 1, usize::MAX, usize::MAX - 20, usize::MAX - 44] {
                    eqv(&format!("{nm}({v})"), cstr(a(v)), cstr(b(v)));
                }
            }
        }
    }
}

// ------------------------------------------------------------------ rows 6, 7

#[test]
fn row06_cparam_bounds() {
    unsafe {
        let (c, r) = duo::<FnGetBounds>("ZSTD_cParam_getBounds");
        for (name, p) in ALL_CPARAMS {
            eqv(&format!("ZSTD_cParam_getBounds({name})"), c(*p), r(*p));
        }
        // out-of-range enum values (C enums accept any int)
        for p in [
            -1i32, 0, 1, 9, 11, 99, 108, 129, 131, 159, 165, 199, 203, 399, 403, 499, 501, 999,
            1003, 1018, 1019, 2000, i32::MIN, i32::MAX,
        ] {
            eqv(&format!("ZSTD_cParam_getBounds({p})"), c(p), r(p));
        }
        let mut rng = Rng::new(6);
        for _ in 0..3000 {
            let p = rng.next_u32() as i32;
            eqv(&format!("ZSTD_cParam_getBounds({p})"), c(p), r(p));
        }
    }
}

#[test]
fn row07_dparam_bounds() {
    unsafe {
        let (c, r) = duo::<FnGetBounds>("ZSTD_dParam_getBounds");
        for (name, p) in ALL_DPARAMS {
            eqv(&format!("ZSTD_dParam_getBounds({name})"), c(*p), r(*p));
        }
        for p in [
            -1i32, 0, 99, 101, 999, 1006, 1007, 2000, i32::MIN, i32::MAX,
        ] {
            eqv(&format!("ZSTD_dParam_getBounds({p})"), c(p), r(p));
        }
        let mut rng = Rng::new(7);
        for _ in 0..3000 {
            let p = rng.next_u32() as i32;
            eqv(&format!("ZSTD_dParam_getBounds({p})"), c(p), r(p));
        }
    }
}

// ------------------------------------------------------------------ rows 8-12

fn level_grid() -> Vec<c_int> {
    let mut v: Vec<c_int> = vec![-131072, -1000, -22, -10, -5, -4, -3, -2, -1, 0];
    v.extend(1..=22);
    v.push(23);
    v.push(100);
    v
}

fn srcsize_grid() -> Vec<c_ulonglong> {
    vec![
        0,
        1,
        7,
        512,
        1024,
        16 * 1024,
        128 * 1024,
        1 << 20,
        1 << 24,
        1u64 << 30,
        1u64 << 40,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ]
}

fn dictsize_grid() -> Vec<usize> {
    vec![0, 1, 256, 1024, 8 * 1024, 112 * 1024, 1 << 20]
}

#[test]
fn row08_09_getcparams_getparams() {
    unsafe {
        let (cc, cr) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let (pc, pr) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters>(
                "ZSTD_getParams",
            );
        for lvl in level_grid() {
            for ss in srcsize_grid() {
                for ds in dictsize_grid() {
                    let w = format!("ZSTD_getCParams({lvl},{ss},{ds})");
                    eqv(&w, cc(lvl, ss, ds), cr(lvl, ss, ds));
                    let w = format!("ZSTD_getParams({lvl},{ss},{ds})");
                    eqv(&w, pc(lvl, ss, ds), pr(lvl, ss, ds));
                }
            }
        }
        // randomized
        let mut rng = Rng::new(89);
        for _ in 0..4000 {
            let lvl = rng.range(-40, 30);
            let ss = rng.next_u64() >> rng.below(64) as u32;
            let ds = (rng.next_u64() >> rng.below(48) as u32) as usize;
            eqv(
                &format!("ZSTD_getCParams({lvl},{ss},{ds})"),
                cc(lvl, ss, ds),
                cr(lvl, ss, ds),
            );
            eqv(
                &format!("ZSTD_getParams({lvl},{ss},{ds})"),
                pc(lvl, ss, ds),
                pr(lvl, ss, ds),
            );
        }
    }
}

#[test]
fn row10_11_adjust_check_cparams() {
    unsafe {
        let (ac, ar) = duo::<
            unsafe extern "C" fn(
                ZSTD_compressionParameters,
                c_ulonglong,
                usize,
            ) -> ZSTD_compressionParameters,
        >("ZSTD_adjustCParams");
        let (kc, kr) =
            duo::<unsafe extern "C" fn(ZSTD_compressionParameters) -> usize>("ZSTD_checkCParams");
        let (gc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );

        // start from every level's cParams, then perturb every field
        let mut cases: Vec<ZSTD_compressionParameters> = Vec::new();
        for lvl in level_grid() {
            for ss in [0u64, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                cases.push(gc(lvl, ss, 0));
            }
        }
        let base = gc(3, 0, 0);
        for wl in [0u32, 9, 10, 11, 27, 30, 31, 32, 40] {
            let mut p = base;
            p.windowLog = wl;
            cases.push(p);
        }
        for hl in [0u32, 5, 6, 7, 30, 31, 32] {
            let mut p = base;
            p.hashLog = hl;
            cases.push(p);
        }
        for cl in [0u32, 5, 6, 7, 29, 30, 31] {
            let mut p = base;
            p.chainLog = cl;
            cases.push(p);
        }
        for sl in [0u32, 1, 2, 30, 31, 32] {
            let mut p = base;
            p.searchLog = sl;
            cases.push(p);
        }
        for mm in [0u32, 2, 3, 4, 5, 6, 7, 8] {
            let mut p = base;
            p.minMatch = mm;
            cases.push(p);
        }
        for tl in [0u32, 1, 16, 999, 131072, 131073] {
            let mut p = base;
            p.targetLength = tl;
            cases.push(p);
        }
        for st in 0..12u32 {
            let mut p = base;
            p.strategy = st;
            cases.push(p);
        }
        let mut rng = Rng::new(1011);
        for _ in 0..4000 {
            cases.push(ZSTD_compressionParameters {
                windowLog: rng.range(0, 33) as u32,
                chainLog: rng.range(0, 33) as u32,
                hashLog: rng.range(0, 33) as u32,
                searchLog: rng.range(0, 33) as u32,
                minMatch: rng.range(0, 9) as u32,
                targetLength: rng.range(0, 140000) as u32,
                strategy: rng.range(0, 11) as u32,
            });
        }

        for p in cases {
            eqv(&format!("ZSTD_checkCParams({p:?})"), kc(p), kr(p));
            for ss in [0u64, 1, 1024, 1 << 20, 1u64 << 32, ZSTD_CONTENTSIZE_UNKNOWN] {
                for ds in [0usize, 1024, 1 << 20] {
                    eqv(
                        &format!("ZSTD_adjustCParams({p:?},{ss},{ds})"),
                        ac(p, ss, ds),
                        ar(p, ss, ds),
                    );
                }
            }
        }
    }
}

#[test]
fn row12_cycle_log() {
    unsafe {
        let (c, r) = duo::<unsafe extern "C" fn(c_uint, c_int) -> c_uint>("ZSTD_cycleLog");
        for hl in 0..40u32 {
            for st in 0..12i32 {
                eqv(&format!("ZSTD_cycleLog({hl},{st})"), c(hl, st), r(hl, st));
            }
        }
    }
}

// ------------------------------------------------------------------ rows 14-20

#[test]
fn row14_sizeof_contexts() {
    unsafe {
        type FnSz = unsafe extern "C" fn(*const c_void) -> usize;
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let cs = CtxPair::cstream();
        let ds = CtxPair::dstream();

        let (a, b) = duo::<FnSz>("ZSTD_sizeof_CCtx");
        eqv("sizeof_CCtx fresh", a(cctx.c), b(cctx.r));
        let (a, b) = duo::<FnSz>("ZSTD_sizeof_DCtx");
        eqv("sizeof_DCtx fresh", a(dctx.c), b(dctx.r));
        let (a, b) = duo::<FnSz>("ZSTD_sizeof_CStream");
        eqv("sizeof_CStream fresh", a(cs.c), b(cs.r));
        let (a, b) = duo::<FnSz>("ZSTD_sizeof_DStream");
        eqv("sizeof_DStream fresh", a(ds.c), b(ds.r));
        // NULL is documented-safe (returns 0)
        let (a, b) = duo::<FnSz>("ZSTD_sizeof_CCtx");
        eqv("sizeof_CCtx NULL", a(std::ptr::null()), b(std::ptr::null()));

        // after use, at several levels
        let (cc, cr) = duo::<FnCompressCCtx>("ZSTD_compressCCtx");
        let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        for lvl in [1, 5, 12, 19, 22] {
            let src = gen_class(4, 40_000, lvl as u64);
            let cap = bd(src.len());
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let nc = cc(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                lvl,
            );
            let nr = cr(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                lvl,
            );
            eqv("compressCCtx ret", nc, nr);
            eqbuf("compressCCtx dst", &oc[..nc], &or_[..nr]);
            let (a, b) = duo::<FnSz>("ZSTD_sizeof_CCtx");
            eqv(&format!("sizeof_CCtx after level {lvl}"), a(cctx.c), b(cctx.r));

            let mut pc = vec![0u8; src.len() + 1];
            let mut pr = vec![0u8; src.len() + 1];
            let mc = dc(
                dctx.c,
                pc.as_mut_ptr() as *mut c_void,
                pc.len(),
                oc.as_ptr() as *const c_void,
                nc,
            );
            let mr = dr(
                dctx.r,
                pr.as_mut_ptr() as *mut c_void,
                pr.len(),
                or_.as_ptr() as *const c_void,
                nr,
            );
            eqv("decompressDCtx ret", mc, mr);
            eqbuf("decompressDCtx dst", &pc, &pr);
            let (a, b) = duo::<FnSz>("ZSTD_sizeof_DCtx");
            eqv(&format!("sizeof_DCtx after level {lvl}"), a(dctx.c), b(dctx.r));
        }
    }
}

#[test]
fn row15_16_17_18_19_estimates() {
    unsafe {
        let (a, b) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCCtxSize");
        for lvl in level_grid() {
            eqv(&format!("estimateCCtxSize({lvl})"), a(lvl), b(lvl));
        }
        let (a, b) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCStreamSize");
        for lvl in level_grid() {
            eqv(&format!("estimateCStreamSize({lvl})"), a(lvl), b(lvl));
        }
        let (a, b) = duo::<FnSizeT0>("ZSTD_estimateDCtxSize");
        eqv("estimateDCtxSize", a(), b());

        let (gc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let (ac, ar) = duo::<unsafe extern "C" fn(ZSTD_compressionParameters) -> usize>(
            "ZSTD_estimateCCtxSize_usingCParams",
        );
        let (sc, sr) = duo::<unsafe extern "C" fn(ZSTD_compressionParameters) -> usize>(
            "ZSTD_estimateCStreamSize_usingCParams",
        );
        let mut params: Vec<ZSTD_compressionParameters> = Vec::new();
        for lvl in level_grid() {
            for ss in [0u64, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                params.push(gc(lvl, ss, 0));
            }
        }
        let mut rng = Rng::new(1516);
        for _ in 0..600 {
            let mut p = gc(rng.range(1, 22), 0, 0);
            p.windowLog = rng.range(10, 27) as u32;
            p.hashLog = rng.range(6, 26) as u32;
            p.chainLog = rng.range(6, 26) as u32;
            p.searchLog = rng.range(1, 20) as u32;
            p.minMatch = rng.range(3, 7) as u32;
            p.strategy = rng.range(1, 9) as u32;
            params.push(p);
        }
        for p in &params {
            eqv(&format!("estimateCCtxSize_usingCParams({p:?})"), ac(*p), ar(*p));
            eqv(
                &format!("estimateCStreamSize_usingCParams({p:?})"),
                sc(*p),
                sr(*p),
            );
        }

        let (dc, dr) = duo::<FnSizeT1>("ZSTD_estimateDStreamSize");
        for w in [
            0usize,
            1,
            1 << 10,
            1 << 17,
            1 << 20,
            1 << 27,
            (1usize << 31) - 1,
        ] {
            eqv(&format!("estimateDStreamSize({w})"), dc(w), dr(w));
        }

        let (cdc, cdr) =
            duo::<unsafe extern "C" fn(usize, c_int) -> usize>("ZSTD_estimateCDictSize");
        for ds in dictsize_grid() {
            for lvl in [1, 3, 9, 19, 22] {
                eqv(
                    &format!("estimateCDictSize({ds},{lvl})"),
                    cdc(ds, lvl),
                    cdr(ds, lvl),
                );
            }
        }
        let (cac, car) = duo::<
            unsafe extern "C" fn(usize, ZSTD_compressionParameters, c_int) -> usize,
        >("ZSTD_estimateCDictSize_advanced");
        for ds in dictsize_grid() {
            for p in params.iter().take(40) {
                for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    eqv(
                        &format!("estimateCDictSize_advanced({ds},{p:?},{dlm})"),
                        cac(ds, *p, dlm),
                        car(ds, *p, dlm),
                    );
                }
            }
        }
        let (ddc, ddr) =
            duo::<unsafe extern "C" fn(usize, c_int) -> usize>("ZSTD_estimateDDictSize");
        for ds in dictsize_grid() {
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef, 5] {
                eqv(
                    &format!("estimateDDictSize({ds},{dlm})"),
                    ddc(ds, dlm),
                    ddr(ds, dlm),
                );
            }
        }
    }
}

#[test]
fn row17_estimate_dstream_from_frame_and_margins() {
    unsafe {
        let (fc, fr) = duo::<FnDecompress2>("ZSTD_estimateDStreamSize_fromFrame");
        let (mc, mr) = duo::<FnDecompress2>("ZSTD_decompressionMargin");
        let (bc, br) = duo::<unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> usize>(
            "ZSTD_decodingBufferSize_min",
        );
        for w in [0u64, 1, 1024, 1 << 17, 1 << 20, 1u64 << 31] {
            for f in [0u64, 1, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                eqv(
                    &format!("decodingBufferSize_min({w},{f})"),
                    bc(w, f),
                    br(w, f),
                );
            }
        }
        let mut rng = Rng::new(17);
        for i in 0..60 {
            let sz = rng.below(300_000);
            let cls = rng.below(N_CLASSES);
            let src = gen_class(cls, sz, i);
            let frame = c_compress(&src, rng.range(-3, 19));
            for take in [frame.len(), frame.len() / 2, 4, 1, 0] {
                let s = &frame[..take.min(frame.len())];
                eqv(
                    &format!("estimateDStreamSize_fromFrame len={}", s.len()),
                    fc(s.as_ptr() as *const c_void, s.len()),
                    fr(s.as_ptr() as *const c_void, s.len()),
                );
                eqv(
                    &format!("decompressionMargin len={}", s.len()),
                    mc(s.as_ptr() as *const c_void, s.len()),
                    mr(s.as_ptr() as *const c_void, s.len()),
                );
            }
        }
    }
}

pub type FnDecompress2 = unsafe extern "C" fn(*const c_void, usize) -> usize;
pub type FnU64FromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
pub type FnUFromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_uint;

// ------------------------------------------------------------------ rows 21-24

fn cmp_roundtrip(level: c_int, src: &[u8], what: &str) {
    unsafe {
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (cc, cr) = duo::<FnCompress>("ZSTD_compress");
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let cap = bd(src.len());
        let mut oc = vec![0xCDu8; cap];
        let mut or_ = vec![0xCDu8; cap];
        let nc = cc(
            oc.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        );
        let nr = cr(
            or_.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        );
        eqv(&format!("{what} compress ret"), nc, nr);
        eqbuf(&format!("{what} compress dst"), &oc, &or_);
        if is_err(nc) {
            return;
        }
        let mut pc = vec![0x3Cu8; src.len() + 8];
        let mut pr = vec![0x3Cu8; src.len() + 8];
        let mc = dc(
            pc.as_mut_ptr() as *mut c_void,
            pc.len(),
            oc.as_ptr() as *const c_void,
            nc,
        );
        let mr = dr(
            pr.as_mut_ptr() as *mut c_void,
            pr.len(),
            or_.as_ptr() as *const c_void,
            nr,
        );
        eqv(&format!("{what} decompress ret"), mc, mr);
        eqbuf(&format!("{what} decompress dst"), &pc, &pr);
        assert!(!is_err(mc), "{what}: decompression failed");
        assert_eq!(&pc[..mc], src, "{what}: round-trip content mismatch");
    }
}

#[test]
fn row21_oneshot_all_levels_sizes_classes() {
    let levels: Vec<c_int> = vec![-131072, -5, -3, -1, 0, 1, 2, 3, 6, 9, 12, 16, 19, 20, 22];
    for &lvl in &levels {
        for &sz in SIZES.iter() {
            // keep the big sizes to a couple of levels to bound runtime
            if sz > 200_000 && !(lvl == 1 || lvl == 3 || lvl == 19) {
                continue;
            }
            for cls in 0..N_CLASSES {
                let src = gen_class(cls, sz, (lvl as i64 as u64) ^ 0x1234);
                cmp_roundtrip(
                    lvl,
                    &src,
                    &format!("row21 lvl={lvl} size={sz} class={}", CLASS_NAMES[cls]),
                );
            }
        }
    }
}

#[test]
fn row21b_oneshot_random_sizes() {
    let mut rng = Rng::new(0x21B);
    for i in 0..400 {
        let sz = rng.below(70_000);
        let cls = rng.below(N_CLASSES);
        let lvl = rng.range(-7, 22);
        let src = gen_class(cls, sz, i);
        cmp_roundtrip(lvl, &src, &format!("row21b i={i} lvl={lvl} size={sz}"));
    }
}

#[test]
fn row22_cctx_reuse() {
    unsafe {
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let (cc, cr) = duo::<FnCompressCCtx>("ZSTD_compressCCtx");
        let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let mut rng = Rng::new(22);
        for i in 0..300 {
            let sz = rng.below(50_000);
            let cls = rng.below(N_CLASSES);
            let lvl = rng.range(-5, 22);
            let src = gen_class(cls, sz, i);
            let cap = bd(sz);
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let nc = cc(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
                lvl,
            );
            let nr = cr(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
                lvl,
            );
            eqv(&format!("row22 i={i} compressCCtx ret"), nc, nr);
            eqbuf(&format!("row22 i={i} compressCCtx dst"), &oc, &or_);
            let mut pc = vec![0u8; sz + 4];
            let mut pr = vec![0u8; sz + 4];
            let mc = dc(
                dctx.c,
                pc.as_mut_ptr() as *mut c_void,
                pc.len(),
                oc.as_ptr() as *const c_void,
                nc,
            );
            let mr = dr(
                dctx.r,
                pr.as_mut_ptr() as *mut c_void,
                pr.len(),
                or_.as_ptr() as *const c_void,
                nr,
            );
            eqv(&format!("row22 i={i} decompressDCtx ret"), mc, mr);
            eqbuf(&format!("row22 i={i} decompressDCtx dst"), &pc, &pr);
        }
    }
}

#[test]
fn row23_exact_capacities() {
    unsafe {
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (cc, cr) = duo::<FnCompress>("ZSTD_compress");
        let mut rng = Rng::new(23);
        for i in 0..120 {
            let sz = 1 + rng.below(30_000);
            let cls = rng.below(N_CLASSES);
            let lvl = rng.range(-3, 19);
            let src = gen_class(cls, sz, i);
            let exact = c_compress(&src, lvl).len();
            for cap in [bd(sz), exact + 1, exact, exact.saturating_sub(1), exact / 2, 1, 0] {
                let mut oc = vec![0x11u8; cap.max(1)];
                let mut or_ = vec![0x11u8; cap.max(1)];
                let nc = cc(
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    lvl,
                );
                let nr = cr(
                    or_.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    lvl,
                );
                eqv(&format!("row23 i={i} cap={cap} ret"), nc, nr);
                eqbuf(&format!("row23 i={i} cap={cap} dst"), &oc, &or_);
            }
        }
    }
}

#[test]
fn row24_compress2() {
    unsafe {
        let cctx = CtxPair::cctx();
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (cc, cr) = duo::<FnDecompressDCtx>("ZSTD_compress2");
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let mut rng = Rng::new(24);
        for i in 0..200 {
            let sz = rng.below(60_000);
            let cls = rng.below(N_CLASSES);
            let lvl = rng.range(-5, 22);
            eqv(
                "row24 setParameter(level)",
                sp_c(cctx.c, ZSTD_c_compressionLevel, lvl),
                sp_r(cctx.r, ZSTD_c_compressionLevel, lvl),
            );
            let src = gen_class(cls, sz, i);
            let cap = bd(sz);
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let nc = cc(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            let nr = cr(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            eqv(&format!("row24 i={i} compress2 ret"), nc, nr);
            eqbuf(&format!("row24 i={i} compress2 dst"), &oc, &or_);
        }
    }
}

// ------------------------------------------------------------------ rows 25-29

/// Build a multi-frame stream: n zstd frames (varying flags) with optional
/// skippable frames interleaved.
fn multi_frame(seed: u64, nframes: usize, with_skippable: bool) -> (Vec<u8>, usize) {
    unsafe {
        let mut rng = Rng::new(seed);
        let cctx = CtxPair::cctx();
        let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = duo::<FnDecompressDCtx>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (wsk, _) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize,
        >("ZSTD_writeSkippableFrame");
        let mut out = Vec::new();
        let mut total = 0usize;
        for _ in 0..nframes {
            let sz = rng.below(20_000);
            let src = gen_class(rng.below(N_CLASSES), sz, rng.next_u64());
            sp(cctx.c, ZSTD_c_compressionLevel, rng.range(1, 9));
            sp(cctx.c, ZSTD_c_contentSizeFlag, (rng.below(2)) as c_int);
            sp(cctx.c, ZSTD_c_checksumFlag, (rng.below(2)) as c_int);
            let cap = bd(sz);
            let mut buf = vec![0u8; cap];
            let n = c2(
                cctx.c,
                buf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            assert!(!is_err(n));
            out.extend_from_slice(&buf[..n]);
            total += sz;
            if with_skippable && rng.below(2) == 0 {
                let plen = rng.below(64);
                let pay = rng.bytes(plen);
                let mut sk = vec![0u8; plen + 8];
                let m = wsk(
                    sk.as_mut_ptr() as *mut c_void,
                    sk.len(),
                    pay.as_ptr() as *const c_void,
                    plen,
                    (rng.below(16)) as c_uint,
                );
                assert!(!is_err(m));
                out.extend_from_slice(&sk[..m]);
            }
        }
        (out, total)
    }
}

#[test]
fn row25_frame_size_queries() {
    unsafe {
        let (fcc, fcr) = duo::<FnDecompress2>("ZSTD_findFrameCompressedSize");
        let (fdc, fdr) = duo::<FnU64FromBuf>("ZSTD_findDecompressedSize");
        let (gfc, gfr) = duo::<FnU64FromBuf>("ZSTD_getFrameContentSize");
        let (gdc, gdr) = duo::<FnU64FromBuf>("ZSTD_getDecompressedSize");
        let (dbc, dbr) = duo::<FnU64FromBuf>("ZSTD_decompressBound");
        for seed in 0..40u64 {
            for nf in [1usize, 2, 3, 8] {
                for sk in [false, true] {
                    let (buf, _) = multi_frame(seed * 97 + nf as u64, nf, sk);
                    for take in [buf.len(), buf.len() / 2, 8, 4, 1, 0] {
                        let s = &buf[..take.min(buf.len())];
                        let p = s.as_ptr() as *const c_void;
                        let w = format!("row25 seed={seed} nf={nf} sk={sk} take={}", s.len());
                        eqv(&format!("{w} findFrameCompressedSize"), fcc(p, s.len()), fcr(p, s.len()));
                        eqv(&format!("{w} findDecompressedSize"), fdc(p, s.len()), fdr(p, s.len()));
                        eqv(&format!("{w} getFrameContentSize"), gfc(p, s.len()), gfr(p, s.len()));
                        eqv(&format!("{w} getDecompressedSize"), gdc(p, s.len()), gdr(p, s.len()));
                        eqv(&format!("{w} decompressBound"), dbc(p, s.len()), dbr(p, s.len()));
                    }
                }
            }
        }
    }
}

#[test]
fn row26_27_frame_headers() {
    unsafe {
        let (ghc, ghr) = duo::<
            unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize) -> usize,
        >("ZSTD_getFrameHeader");
        let (gac, gar) = duo::<
            unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize, c_int) -> usize,
        >("ZSTD_getFrameHeader_advanced");
        let (fhc, fhr) = duo::<FnDecompress2>("ZSTD_frameHeaderSize");
        let cctx = CtxPair::cctx();
        let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (pl, _) = duo::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize>(
            "ZSTD_CCtx_setPledgedSrcSize",
        );
        let (c2, _) = duo::<FnDecompressDCtx>("ZSTD_compress2");
        let (rst, _) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        // every FCS field width comes from different (srcSize, contentSizeFlag,
        // windowLog) combinations; every DID width from dictIDFlag + dict id
        let sizes = [0usize, 1, 200, 300, 70_000, 200_000];
        for &sz in &sizes {
            for csf in [0, 1] {
                for cks in [0, 1] {
                    for did in [0, 1] {
                        for wl in [10, 17, 23] {
                            for fmt in [0, 1] {
                                rst(cctx.c, ZSTD_reset_session_and_parameters);
                                sp(cctx.c, ZSTD_c_contentSizeFlag, csf);
                                sp(cctx.c, ZSTD_c_checksumFlag, cks);
                                sp(cctx.c, ZSTD_c_dictIDFlag, did);
                                sp(cctx.c, ZSTD_c_windowLog, wl);
                                sp(cctx.c, ZSTD_c_format, fmt);
                                sp(cctx.c, ZSTD_c_compressionLevel, 3);
                                let src = gen_class(4, sz, 26);
                                let cap = bd(sz) + 64;
                                let mut buf = vec![0u8; cap];
                                let n = c2(
                                    cctx.c,
                                    buf.as_mut_ptr() as *mut c_void,
                                    cap,
                                    src.as_ptr() as *const c_void,
                                    sz,
                                );
                                assert!(!is_err(n), "helper compress2 failed");
                                let frame = &buf[..n];
                                for take in [n, 18, 14, 9, 6, 5, 4, 3, 2, 1, 0] {
                                    let s = &frame[..take.min(n)];
                                    let p = s.as_ptr() as *const c_void;
                                    let mut hc = ZSTD_frameHeader::default();
                                    let mut hr = ZSTD_frameHeader::default();
                                    let a = ghc(&mut hc, p, s.len());
                                    let b = ghr(&mut hr, p, s.len());
                                    let w = format!(
                                        "row26 sz={sz} csf={csf} cks={cks} did={did} wl={wl} fmt={fmt} take={}",
                                        s.len()
                                    );
                                    eqv(&format!("{w} getFrameHeader ret"), a, b);
                                    eqv(&format!("{w} getFrameHeader out"), hc, hr);
                                    for f in [0, 1, 2, -1, 99] {
                                        let mut hc = ZSTD_frameHeader::default();
                                        let mut hr = ZSTD_frameHeader::default();
                                        let a = gac(&mut hc, p, s.len(), f);
                                        let b = gar(&mut hr, p, s.len(), f);
                                        eqv(&format!("{w} getFrameHeader_advanced({f}) ret"), a, b);
                                        eqv(&format!("{w} getFrameHeader_advanced({f}) out"), hc, hr);
                                    }
                                    eqv(
                                        &format!("{w} frameHeaderSize"),
                                        fhc(p, s.len()),
                                        fhr(p, s.len()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row28_isframe() {
    unsafe {
        let (ifc, ifr) = duo::<FnUFromBuf>("ZSTD_isFrame");
        let (isc, isr) = duo::<FnUFromBuf>("ZSTD_isSkippableFrame");
        let mut cases: Vec<Vec<u8>> = Vec::new();
        for seed in 0..6u64 {
            let (b, _) = multi_frame(seed, 1 + seed as usize, seed % 2 == 0);
            cases.push(b);
        }
        // all magic numbers, alone and with payload
        let mut magics: Vec<u32> = vec![ZSTD_MAGICNUMBER, ZSTD_MAGIC_DICTIONARY];
        magics.extend(LEGACY_MAGICS);
        for v in 0..16u32 {
            magics.push(ZSTD_MAGIC_SKIPPABLE_START + v);
        }
        magics.push(ZSTD_MAGIC_SKIPPABLE_START + 16);
        magics.push(ZSTD_MAGICNUMBER + 1);
        magics.push(0);
        for m in magics {
            let mut v = m.to_le_bytes().to_vec();
            cases.push(v.clone());
            v.extend_from_slice(&[0u8; 12]);
            cases.push(v);
        }
        let mut rng = Rng::new(28);
        for _ in 0..300 {
            let n = rng.below(20);
            cases.push(rng.bytes(n));
        }
        for (i, b) in cases.iter().enumerate() {
            let p = b.as_ptr() as *const c_void;
            for take in [b.len(), b.len() / 2, 4, 3, 1, 0] {
                let l = take.min(b.len());
                eqv(&format!("row28 i={i} isFrame len={l}"), ifc(p, l), ifr(p, l));
                eqv(
                    &format!("row28 i={i} isSkippableFrame len={l}"),
                    isc(p, l),
                    isr(p, l),
                );
            }
        }
    }
}

#[test]
fn row29_skippable_frames() {
    unsafe {
        let (wc, wr) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize,
        >("ZSTD_writeSkippableFrame");
        let (rc, rr) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize,
        >("ZSTD_readSkippableFrame");
        let mut rng = Rng::new(29);
        for mv in 0..18u32 {
            for &plen in &[0usize, 1, 2, 7, 64, 1000] {
                let pay = gen_class(rng.below(N_CLASSES), plen, mv as u64);
                for cap in [plen + 8, plen + 7, plen, 8, 4, 0] {
                    let mut oc = vec![0x55u8; cap.max(1)];
                    let mut or_ = vec![0x55u8; cap.max(1)];
                    let a = wc(
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        pay.as_ptr() as *const c_void,
                        plen,
                        mv,
                    );
                    let b = wr(
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        pay.as_ptr() as *const c_void,
                        plen,
                        mv,
                    );
                    eqv(&format!("row29 write mv={mv} plen={plen} cap={cap}"), a, b);
                    eqbuf(
                        &format!("row29 write dst mv={mv} plen={plen} cap={cap}"),
                        &oc,
                        &or_,
                    );
                    if is_err(a) {
                        continue;
                    }
                    let frame = &oc[..a];
                    for rcap in [plen, plen + 1, plen.saturating_sub(1), 0] {
                        let mut dc = vec![0x77u8; rcap.max(1)];
                        let mut dr = vec![0x77u8; rcap.max(1)];
                        let mut vc: c_uint = 0xDEAD;
                        let mut vr: c_uint = 0xDEAD;
                        let x = rc(
                            dc.as_mut_ptr() as *mut c_void,
                            rcap,
                            &mut vc,
                            frame.as_ptr() as *const c_void,
                            frame.len(),
                        );
                        let y = rr(
                            dr.as_mut_ptr() as *mut c_void,
                            rcap,
                            &mut vr,
                            frame.as_ptr() as *const c_void,
                            frame.len(),
                        );
                        eqv(&format!("row29 read mv={mv} plen={plen} rcap={rcap}"), x, y);
                        eqv(&format!("row29 read magicVariant mv={mv}"), vc, vr);
                        eqbuf(&format!("row29 read dst mv={mv} rcap={rcap}"), &dc, &dr);
                    }
                    // NULL magicVariant pointer is allowed by the C API
                    let mut dc = vec![0u8; plen.max(1)];
                    let mut dr = vec![0u8; plen.max(1)];
                    let x = rc(
                        dc.as_mut_ptr() as *mut c_void,
                        plen,
                        std::ptr::null_mut(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                    );
                    let y = rr(
                        dr.as_mut_ptr() as *mut c_void,
                        plen,
                        std::ptr::null_mut(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                    );
                    eqv("row29 read NULL variant", x, y);
                    eqbuf("row29 read NULL variant dst", &dc, &dr);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 30

#[test]
fn row30_usingdict_oneshot() {
    unsafe {
        type FnCU = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const c_void,
            usize,
            c_int,
        ) -> usize;
        type FnDU = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const c_void,
            usize,
        ) -> usize;
        let (cc, cr) = duo::<FnCU>("ZSTD_compress_usingDict");
        let (dc, dr) = duo::<FnDU>("ZSTD_decompress_usingDict");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let mut rng = Rng::new(30);
        for i in 0..120 {
            let dsz = [0usize, 1, 7, 64, 1024, 8192][rng.below(6)];
            let dict = gen_class(rng.below(N_CLASSES), dsz, i);
            let sz = rng.below(40_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i + 1000);
            let lvl = rng.range(-3, 19);
            let cap = bd(sz);
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let dp = if dsz == 0 {
                std::ptr::null()
            } else {
                dict.as_ptr() as *const c_void
            };
            let a = cc(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
                dp,
                dsz,
                lvl,
            );
            let b = cr(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
                dp,
                dsz,
                lvl,
            );
            eqv(&format!("row30 i={i} compress_usingDict"), a, b);
            eqbuf(&format!("row30 i={i} compress_usingDict dst"), &oc, &or_);
            if is_err(a) {
                continue;
            }
            let mut pc = vec![0u8; sz + 4];
            let mut pr = vec![0u8; sz + 4];
            let x = dc(
                dctx.c,
                pc.as_mut_ptr() as *mut c_void,
                pc.len(),
                oc.as_ptr() as *const c_void,
                a,
                dp,
                dsz,
            );
            let y = dr(
                dctx.r,
                pr.as_mut_ptr() as *mut c_void,
                pr.len(),
                or_.as_ptr() as *const c_void,
                b,
                dp,
                dsz,
            );
            eqv(&format!("row30 i={i} decompress_usingDict"), x, y);
            eqbuf(&format!("row30 i={i} decompress_usingDict dst"), &pc, &pr);
            assert!(!is_err(x));
            assert_eq!(&pc[..x], &src[..]);
        }
    }
}

// ------------------------------------------------------------------ row 4

#[test]
fn row04_decompress_bound_garbage() {
    unsafe {
        let (dbc, dbr) = duo::<FnU64FromBuf>("ZSTD_decompressBound");
        let mut rng = Rng::new(4);
        for i in 0..400 {
            let n = rng.below(64);
            let b = rng.bytes(n);
            eqv(
                &format!("row04 garbage i={i}"),
                dbc(b.as_ptr() as *const c_void, b.len()),
                dbr(b.as_ptr() as *const c_void, b.len()),
            );
        }
        // NULL / zero
        eqv(
            "row04 null",
            dbc(std::ptr::null(), 0),
            dbr(std::ptr::null(), 0),
        );
    }
}
