//! Phase C — ERRORS.md rows covered by `phase_c_misc`:
//! `common/error_private.h` (5 sites), `common/xxhash.h` (3 sites),
//! `common/pool.c` (6 sites), `common/threading.c` (3 sites),
//! `compress/zstd_cwksp.h` (8 sites), `compress/zstdmt_compress.c` (27 sites,
//! non-MT build), and the deprecated ZBUFF surface.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

type FnFromBuf = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnVoidPtr = unsafe extern "C" fn(*const c_void) -> usize;

#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        assert_eq!(
            c,
            r,
            "{what}: C={c:#x} (code {} = {}), Rust={r:#x} (code {} = {})",
            gcc(c),
            cstr(nc(c)),
            gcr(r),
            cstr(nr(r))
        );
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: name mismatch");
    }
}

// ------------------------------------------------------------------ error_private.h

/// `ERR_isError` / `ERR_getErrorCode` / `ERR_getErrorName` / `ERR_getErrorString`
/// and their `ZSTD_`-prefixed public wrappers, over the whole `size_t` sentinel
/// range and over every `ZSTD_ErrorCode` including values with no variant.
#[test]
fn err_error_private_surface() {
    unsafe {
        let (isc, isr) = duo::<FnIsError>("ZSTD_isError");
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (gnc, gnr) = duo::<FnErrName>("ZSTD_getErrorName");
        let (gsc, gsr) = duo::<unsafe extern "C" fn(c_uint) -> *const c_char>("ZSTD_getErrorString");
        let (esc, esr) = duo::<unsafe extern "C" fn(c_int) -> *const c_char>("ERR_getErrorString");

        // every sentinel (size_t)-n for n in 0..=256 plus the whole neighbourhood
        for n in 0..=256usize {
            let v = 0usize.wrapping_sub(n);
            eqv(&format!("ZSTD_isError(-{n})"), isc(v), isr(v));
            eqv(&format!("ZSTD_getErrorCode(-{n})"), gcc(v), gcr(v));
            eqv(&format!("ZSTD_getErrorName(-{n})"), cstr(gnc(v)), cstr(gnr(v)));
        }
        // every ZSTD_ErrorCode int, including values with no enum variant and
        // negative / huge values crossing the FFI boundary
        let mut codes: Vec<c_int> = (-8..=200).collect();
        codes.extend([i32::MIN, i32::MIN + 1, -1000, 1000, 100000, i32::MAX]);
        let mut rng = Rng::new(0xF001);
        for _ in 0..3000 {
            codes.push(rng.next_u32() as c_int);
        }
        for c in codes {
            eqv(
                &format!("ERR_getErrorString({c})"),
                cstr(esc(c)),
                cstr(esr(c)),
            );
            eqv(
                &format!("ZSTD_getErrorString({c})"),
                cstr(gsc(c as c_uint)),
                cstr(gsr(c as c_uint)),
            );
        }
        // the sibling error surfaces
        for (ise, nm) in [
            ("FSE_isError", "FSE_getErrorName"),
            ("HUF_isError", "HUF_getErrorName"),
            ("ZDICT_isError", "ZDICT_getErrorName"),
            ("ZBUFF_isError", "ZBUFF_getErrorName"),
        ] {
            let (a, b) = duo::<FnIsError>(ise);
            let (na, nb) = duo::<FnErrName>(nm);
            for n in 0..=200usize {
                let v = 0usize.wrapping_sub(n);
                eqv(&format!("{ise}(-{n})"), a(v), b(v));
                eqv(&format!("{nm}(-{n})"), cstr(na(v)), cstr(nb(v)));
            }
            let mut rng = Rng::new(0xF002 ^ ise.len() as u64);
            for _ in 0..500 {
                let v = rng.next_u64() as usize;
                eqv(&format!("{ise}({v})"), a(v), b(v));
                eqv(&format!("{nm}({v})"), cstr(na(v)), cstr(nb(v)));
            }
        }
        let (hc, hr) = duo::<FnIsError>("HIST_isError");
        for n in 0..=200usize {
            let v = 0usize.wrapping_sub(n);
            eqv(&format!("HIST_isError(-{n})"), hc(v), hr(v));
        }
    }
}

// ------------------------------------------------------------------ xxhash

/// The three `return`-on-bad-input sites in `xxhash.h`: NULL state, NULL src
/// with non-zero length is UB (excluded), and `XXH*_reset` on a NULL state.
#[test]
fn err_xxhash_boundaries() {
    unsafe {
        // XXH32/64 with (NULL, 0) is well defined (len == 0 short-circuits)
        let (h32c, h32r) =
            duo::<unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint>("ZSTD_XXH32");
        let (h64c, h64r) = duo::<
            unsafe extern "C" fn(*const c_void, usize, c_ulonglong) -> c_ulonglong,
        >("ZSTD_XXH64");
        for seed in [0u32, 1, 0x9E3779B1, u32::MAX] {
            eqv(
                &format!("ZSTD_XXH32(NULL,0,{seed})"),
                h32c(std::ptr::null(), 0, seed),
                h32r(std::ptr::null(), 0, seed),
            );
        }
        for seed in [0u64, 1, u64::MAX] {
            eqv(
                &format!("ZSTD_XXH64(NULL,0,{seed})"),
                h64c(std::ptr::null(), 0, seed),
                h64r(std::ptr::null(), 0, seed),
            );
        }

        // NOTE: `XXH32_reset`/`XXH64_reset` guard the state pointer only with
        // `XXH_ASSERT(statePtr != NULL)` (xxhash.h:3116), which this build
        // compiles away, and then `memset(statePtr, 0, sizeof(*statePtr))`, so a
        // NULL state segfaults the C. Excluded (CONFIGS.md "C preconditions").
        // `XXH*_update(state, NULL, len)` IS defined: `if (input==NULL) {
        // XXH_ASSERT(len == 0); return XXH_OK; }` (xxhash.h:3130) returns
        // before touching `input`, for ANY len - that path is tested below.
        let (r32c, r32r) =
            duo::<unsafe extern "C" fn(*mut c_void, c_uint) -> c_int>("ZSTD_XXH32_reset");
        let (r64c, r64r) =
            duo::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> c_int>("ZSTD_XXH64_reset");

        // _update with a NULL input on a valid state
        let (c32c, c32r) = duo::<FnPtr0>("ZSTD_XXH32_createState");
        let (f32c, f32r) = duo::<unsafe extern "C" fn(*mut c_void) -> c_int>("ZSTD_XXH32_freeState");
        let (u32c, u32r) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int,
        >("ZSTD_XXH32_update");
        let (d32c, d32r) = duo::<unsafe extern "C" fn(*const c_void) -> c_uint>("ZSTD_XXH32_digest");
        let sc = c32c();
        let sr = c32r();
        assert!(!sc.is_null() && !sr.is_null());
        eqv("XXH32_reset", r32c(sc, 7), r32r(sr, 7));
        eqv(
            "XXH32_update(NULL,0)",
            u32c(sc, std::ptr::null(), 0),
            u32r(sr, std::ptr::null(), 0),
        );
        eqv(
            "XXH32_update(NULL,10)",
            u32c(sc, std::ptr::null(), 10),
            u32r(sr, std::ptr::null(), 10),
        );
        eqv("XXH32_digest after NULL updates", d32c(sc), d32r(sr));
        eqv("XXH32_freeState", f32c(sc), f32r(sr));
        eqv(
            "XXH32_freeState(NULL)",
            f32c(std::ptr::null_mut()),
            f32r(std::ptr::null_mut()),
        );

        let (c64c, c64r) = duo::<FnPtr0>("ZSTD_XXH64_createState");
        let (f64c, f64r) = duo::<unsafe extern "C" fn(*mut c_void) -> c_int>("ZSTD_XXH64_freeState");
        let (u64c, u64r) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int,
        >("ZSTD_XXH64_update");
        let (d64c, d64r) =
            duo::<unsafe extern "C" fn(*const c_void) -> c_ulonglong>("ZSTD_XXH64_digest");
        let sc = c64c();
        let sr = c64r();
        eqv("XXH64_reset", r64c(sc, 7), r64r(sr, 7));
        eqv(
            "XXH64_update(NULL,0)",
            u64c(sc, std::ptr::null(), 0),
            u64r(sr, std::ptr::null(), 0),
        );
        eqv(
            "XXH64_update(NULL,10)",
            u64c(sc, std::ptr::null(), 10),
            u64r(sr, std::ptr::null(), 10),
        );
        eqv("XXH64_digest after NULL updates", d64c(sc), d64r(sr));
        eqv("XXH64_freeState", f64c(sc), f64r(sr));
        eqv(
            "XXH64_freeState(NULL)",
            f64c(std::ptr::null_mut()),
            f64r(std::ptr::null_mut()),
        );

        // canonical conversions with NULL pointers are UB in the C
        // (unconditional writes/reads), so only the value axis is probed.
        #[repr(C)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        struct Canon4 {
            digest: [u8; 4],
        }
        #[repr(C)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        struct Canon8 {
            digest: [u8; 8],
        }
        let (cf32c, cf32r) =
            duo::<unsafe extern "C" fn(*mut Canon4, c_uint)>("ZSTD_XXH32_canonicalFromHash");
        let (hf32c, hf32r) =
            duo::<unsafe extern "C" fn(*const Canon4) -> c_uint>("ZSTD_XXH32_hashFromCanonical");
        let mut rng = Rng::new(0xF003);
        for _ in 0..2000 {
            let v = rng.next_u32();
            let mut a = Canon4::default();
            let mut b = Canon4::default();
            cf32c(&mut a, v);
            cf32r(&mut b, v);
            eqv(&format!("XXH32_canonicalFromHash({v})"), a, b);
            eqv(
                &format!("XXH32_hashFromCanonical({a:?})"),
                hf32c(&a),
                hf32r(&b),
            );
        }
        let (cf64c, cf64r) =
            duo::<unsafe extern "C" fn(*mut Canon8, c_ulonglong)>("ZSTD_XXH64_canonicalFromHash");
        let (hf64c, hf64r) = duo::<unsafe extern "C" fn(*const Canon8) -> c_ulonglong>(
            "ZSTD_XXH64_hashFromCanonical",
        );
        for _ in 0..2000 {
            let v = rng.next_u64();
            let mut a = Canon8::default();
            let mut b = Canon8::default();
            cf64c(&mut a, v);
            cf64r(&mut b, v);
            eqv(&format!("XXH64_canonicalFromHash({v})"), a, b);
            eqv(
                &format!("XXH64_hashFromCanonical({a:?})"),
                hf64c(&a),
                hf64r(&b),
            );
        }
    }
}

// ------------------------------------------------------------------ pool.c / threading.c

#[test]
fn err_pool_boundaries() {
    unsafe {
        let (crc, crr) = duo::<unsafe extern "C" fn(usize, usize) -> *mut c_void>("POOL_create");
        let (cac, car) = duo::<
            unsafe extern "C" fn(usize, usize, ZSTD_customMem) -> *mut c_void,
        >("POOL_create_advanced");
        let (frc, frr) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_free");
        let (rsc, rsr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> c_int>("POOL_resize");
        let (szc, szr) = duo::<FnVoidPtr>("POOL_sizeof");
        let (jjc, jjr) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_joinJobs");

        // sizes at and beyond every boundary
        for nt in [0usize, 1, 2, 4, 1024, usize::MAX / 2, usize::MAX] {
            for qs in [0usize, 1, 4, 1024, usize::MAX / 2, usize::MAX] {
                let a = crc(nt, qs);
                let b = crr(nt, qs);
                eqv(
                    &format!("POOL_create({nt},{qs}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
                eqv(&format!("POOL_sizeof({nt},{qs})"), szc(a), szr(b));
                eqv(
                    &format!("POOL_resize({nt},{qs})"),
                    rsc(a, nt),
                    rsr(b, nt),
                );
                jjc(a);
                jjr(b);
                frc(a);
                frr(b);

                let a = cac(nt, qs, ZSTD_customMem::default());
                let b = car(nt, qs, ZSTD_customMem::default());
                eqv(
                    &format!("POOL_create_advanced({nt},{qs}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
                eqv(&format!("POOL_sizeof adv({nt},{qs})"), szc(a), szr(b));
                frc(a);
                frr(b);
            }
        }
        // NULL handling
        eqv("POOL_sizeof(NULL)", szc(std::ptr::null()), szr(std::ptr::null()));
        eqv(
            "POOL_resize(NULL,4)",
            rsc(std::ptr::null_mut(), 4),
            rsr(std::ptr::null_mut(), 4),
        );
        frc(std::ptr::null_mut());
        frr(std::ptr::null_mut());
        jjc(std::ptr::null_mut());
        jjr(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------------ zstdmt (non-MT build)

#[test]
fn err_zstdmt_non_mt() {
    unsafe {
        // `ZSTDMT_createCCtx_advanced()` always returns NULL in a build without
        // ZSTD_MULTITHREAD (zstdmt_compress.c:992-1001). Only `freeCCtx` and
        // `sizeof_CCtx` accept NULL; every other ZSTDMT entry point
        // dereferences `mtctx` unconditionally, so NULL is UB (see CONFIGS.md).
        let (cc, cr) = duo::<
            unsafe extern "C" fn(c_uint, ZSTD_customMem, *mut c_void) -> *mut c_void,
        >("ZSTDMT_createCCtx_advanced");
        let m = ZSTD_customMem::default();
        let mut rng = Rng::new(0xF004);
        let mut nbs: Vec<c_uint> = vec![0, 1, 2, 4, 16, 256, u32::MAX, u32::MAX - 1];
        for _ in 0..200 {
            nbs.push(rng.next_u32());
        }
        for nb in nbs {
            let a = cc(nb, m, std::ptr::null_mut());
            let b = cr(nb, m, std::ptr::null_mut());
            eqv(
                &format!("ZSTDMT_createCCtx_advanced({nb})"),
                a as usize,
                b as usize,
            );
        }
        let (fc, fr) = duo::<FnFreePtr>("ZSTDMT_freeCCtx");
        eqcode(
            "ZSTDMT_freeCCtx(NULL)",
            fc(std::ptr::null_mut()),
            fr(std::ptr::null_mut()),
        );
        let (sc, sr) = duo::<FnFreePtr>("ZSTDMT_sizeof_CCtx");
        eqv(
            "ZSTDMT_sizeof_CCtx(NULL)",
            sc(std::ptr::null_mut()),
            sr(std::ptr::null_mut()),
        );
        // and setting nbWorkers > 0 on a normal CCtx (the reachable rejection)
        let cctx = CtxPair::cctx();
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (c2c, c2r) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize,
        >("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let src = gen_class(4, 20_000, 1);
        for nw in [-1, 0, 1, 2, 4, 200, i32::MAX] {
            eqcode(
                "reset",
                rc(cctx.c, ZSTD_reset_session_and_parameters),
                rr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            eqcode(
                &format!("set nbWorkers={nw}"),
                spc(cctx.c, ZSTD_c_nbWorkers, nw),
                spr(cctx.r, ZSTD_c_nbWorkers, nw),
            );
            let cap = bd(src.len());
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let a = c2c(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            let b = c2r(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            eqcode(&format!("compress2 with nbWorkers={nw}"), a, b);
            eqbuf(&format!("compress2 with nbWorkers={nw} dst"), &oc, &or_);
        }
    }
}

// ------------------------------------------------------------------ ZBUFF error paths

#[test]
fn err_zbuff_paths() {
    unsafe {
        type FnCreate = unsafe extern "C" fn() -> *mut c_void;
        type FnZbInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
        type FnZbInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize;
        type FnZbCont = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
        ) -> usize;
        type FnZbFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize) -> usize;
        type FnZbDInit = unsafe extern "C" fn(*mut c_void) -> usize;

        let (ccc, ccr) = duo::<FnCreate>("ZBUFF_createCCtx");
        let (fcc, fcr) = duo::<FnFreePtr>("ZBUFF_freeCCtx");
        let (dcc, dcr) = duo::<FnCreate>("ZBUFF_createDCtx");
        let (fdc, fdr) = duo::<FnFreePtr>("ZBUFF_freeDCtx");
        let (ic, ir) = duo::<FnZbInit>("ZBUFF_compressInit");
        let (idc, idr) = duo::<FnZbInitDict>("ZBUFF_compressInitDictionary");
        let (contc, contr) = duo::<FnZbCont>("ZBUFF_compressContinue");
        let (flc, flr) = duo::<FnZbFlush>("ZBUFF_compressFlush");
        let (endc, endr) = duo::<FnZbFlush>("ZBUFF_compressEnd");
        let (dic, dir) = duo::<FnZbDInit>("ZBUFF_decompressInit");
        let (dcontc, dcontr) = duo::<FnZbCont>("ZBUFF_decompressContinue");

        let cc = ccc();
        let cr = ccr();
        let dc = dcc();
        let dr = dcr();
        assert!(!cc.is_null() && !cr.is_null() && !dc.is_null() && !dr.is_null());

        // out-of-range compression levels
        for lvl in [i32::MIN, -131073, -131072, -1, 0, 1, 22, 23, 100, i32::MAX] {
            eqcode(
                &format!("ZBUFF_compressInit({lvl})"),
                ic(cc, lvl),
                ir(cr, lvl),
            );
        }
        // NULL / tiny dictionaries
        let dict = gen_class(4, 128, 1);
        for ds in [0usize, 1, 7, 8, 128] {
            for lvl in [1, 3, 22] {
                let dp = if ds == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_void
                };
                eqcode(
                    &format!("ZBUFF_compressInitDictionary(ds={ds},lvl={lvl})"),
                    idc(cc, dp, ds, lvl),
                    idr(cr, dp, ds, lvl),
                );
            }
        }

        // continue / flush / end before init and with zero-size buffers
        eqcode("ZBUFF_compressInit(3)", ic(cc, 3), ir(cr, 3));
        let src = gen_class(4, 5000, 2);
        for (dcap, ssz) in [
            (0usize, 0usize),
            (0, 5000),
            (1, 5000),
            (1, 0),
            (4096, 0),
            (4096, 5000),
        ] {
            let mut a = vec![0xC1u8; dcap.max(1)];
            let mut b = vec![0xC1u8; dcap.max(1)];
            let mut dca = dcap;
            let mut dcb = dcap;
            let mut sa = ssz;
            let mut sb = ssz;
            let x = contc(
                cc,
                a.as_mut_ptr() as *mut c_void,
                &mut dca,
                src.as_ptr() as *const c_void,
                &mut sa,
            );
            let y = contr(
                cr,
                b.as_mut_ptr() as *mut c_void,
                &mut dcb,
                src.as_ptr() as *const c_void,
                &mut sb,
            );
            eqcode(&format!("ZBUFF_compressContinue(cap={dcap},n={ssz})"), x, y);
            eqv(
                &format!("ZBUFF_compressContinue(cap={dcap},n={ssz}) dstConsumed"),
                dca,
                dcb,
            );
            eqv(
                &format!("ZBUFF_compressContinue(cap={dcap},n={ssz}) srcConsumed"),
                sa,
                sb,
            );
            eqbuf(
                &format!("ZBUFF_compressContinue(cap={dcap},n={ssz}) dst"),
                &a,
                &b,
            );
        }
        for dcap in [0usize, 1, 2, 3, 4096] {
            let mut a = vec![0xC2u8; dcap.max(1)];
            let mut b = vec![0xC2u8; dcap.max(1)];
            let mut dca = dcap;
            let mut dcb = dcap;
            let x = flc(cc, a.as_mut_ptr() as *mut c_void, &mut dca);
            let y = flr(cr, b.as_mut_ptr() as *mut c_void, &mut dcb);
            eqcode(&format!("ZBUFF_compressFlush(cap={dcap})"), x, y);
            eqv(&format!("ZBUFF_compressFlush(cap={dcap}) consumed"), dca, dcb);
            eqbuf(&format!("ZBUFF_compressFlush(cap={dcap}) dst"), &a, &b);
        }
        for dcap in [0usize, 1, 2, 3, 4096] {
            let mut a = vec![0xC3u8; dcap.max(1)];
            let mut b = vec![0xC3u8; dcap.max(1)];
            let mut dca = dcap;
            let mut dcb = dcap;
            let x = endc(cc, a.as_mut_ptr() as *mut c_void, &mut dca);
            let y = endr(cr, b.as_mut_ptr() as *mut c_void, &mut dcb);
            eqcode(&format!("ZBUFF_compressEnd(cap={dcap})"), x, y);
            eqv(&format!("ZBUFF_compressEnd(cap={dcap}) consumed"), dca, dcb);
            eqbuf(&format!("ZBUFF_compressEnd(cap={dcap}) dst"), &a, &b);
        }

        // decompression of garbage / truncated frames
        eqcode("ZBUFF_decompressInit", dic(dc), dir(dr));
        let mut rng = Rng::new(0xF005);
        for i in 0..300 {
            eqcode("ZBUFF_decompressInit(loop)", dic(dc), dir(dr));
            let n = rng.below(64);
            let bad = rng.bytes(n);
            for dcap in [0usize, 1, 4096] {
                let mut a = vec![0xC4u8; dcap.max(1)];
                let mut b = vec![0xC4u8; dcap.max(1)];
                let mut dca = dcap;
                let mut dcb = dcap;
                let mut sa = bad.len();
                let mut sb = bad.len();
                let x = dcontc(
                    dc,
                    a.as_mut_ptr() as *mut c_void,
                    &mut dca,
                    bad.as_ptr() as *const c_void,
                    &mut sa,
                );
                let y = dcontr(
                    dr,
                    b.as_mut_ptr() as *mut c_void,
                    &mut dcb,
                    bad.as_ptr() as *const c_void,
                    &mut sb,
                );
                eqcode(
                    &format!("ZBUFF_decompressContinue i={i} n={n} cap={dcap}"),
                    x,
                    y,
                );
                eqv(
                    &format!("ZBUFF_decompressContinue i={i} dstConsumed"),
                    dca,
                    dcb,
                );
                eqv(
                    &format!("ZBUFF_decompressContinue i={i} srcConsumed"),
                    sa,
                    sb,
                );
                eqbuf(&format!("ZBUFF_decompressContinue i={i} dst"), &a, &b);
            }
        }

        eqcode("ZBUFF_freeCCtx", fcc(cc), fcr(cr));
        eqcode("ZBUFF_freeDCtx", fdc(dc), fdr(dr));
    }
}

// ------------------------------------------------------------------ cwksp / static alloc

/// `compress/zstd_cwksp.h` has 8 rejection sites, all reached through
/// `ZSTD_initStatic*` with an undersized / misaligned workspace.
#[test]
fn err_cwksp_workspace_too_small() {
    unsafe {
        let (icc, icr) =
            duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>("ZSTD_initStaticCCtx");
        let (isc, isr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>(
            "ZSTD_initStaticCStream",
        );
        let (idc, idr) =
            duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>("ZSTD_initStaticDCtx");
        let (ids, idsr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>(
            "ZSTD_initStaticDStream",
        );
        let (ec, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCCtxSize");
        let (es, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCStreamSize");
        let (ed, _) = duo::<FnSizeT0>("ZSTD_estimateDCtxSize");
        let (eds, _) = duo::<FnSizeT1>("ZSTD_estimateDStreamSize");

        // undersized, zero, one byte, and misaligned workspaces
        for lvl in [1, 3, 9, 19, 22] {
            let need = ec(lvl);
            for sz in [
                0usize,
                1,
                8,
                64,
                need / 4,
                need / 2,
                need - 1,
                need,
                need + 1,
            ] {
                for off in [0usize, 1, 3, 7] {
                    let mut wa = vec![0u8; sz + 64];
                    let mut wb = vec![0u8; sz + 64];
                    let a = icc(wa.as_mut_ptr().add(off) as *mut c_void, sz);
                    let b = icr(wb.as_mut_ptr().add(off) as *mut c_void, sz);
                    eqv(
                        &format!("initStaticCCtx(lvl={lvl},ws={sz},off={off}) null?"),
                        a.is_null(),
                        b.is_null(),
                    );
                }
            }
            let need = es(lvl);
            for sz in [0usize, 1, need / 2, need - 1, need, need + 1] {
                let mut wa = vec![0u8; sz + 64];
                let mut wb = vec![0u8; sz + 64];
                let a = isc(wa.as_mut_ptr() as *mut c_void, sz);
                let b = isr(wb.as_mut_ptr() as *mut c_void, sz);
                eqv(
                    &format!("initStaticCStream(lvl={lvl},ws={sz}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
            }
        }
        let need = ed();
        for sz in [0usize, 1, 8, need / 2, need - 1, need, need + 1] {
            for off in [0usize, 1, 3, 7] {
                let mut wa = vec![0u8; sz + 64];
                let mut wb = vec![0u8; sz + 64];
                let a = idc(wa.as_mut_ptr().add(off) as *mut c_void, sz);
                let b = idr(wb.as_mut_ptr().add(off) as *mut c_void, sz);
                eqv(
                    &format!("initStaticDCtx(ws={sz},off={off}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
            }
        }
        for w in [1usize << 10, 1 << 17, 1 << 20] {
            let need = eds(w);
            for sz in [0usize, 1, need / 2, need - 1, need, need + 1] {
                let mut wa = vec![0u8; sz + 64];
                let mut wb = vec![0u8; sz + 64];
                let a = ids(wa.as_mut_ptr() as *mut c_void, sz);
                let b = idsr(wb.as_mut_ptr() as *mut c_void, sz);
                eqv(
                    &format!("initStaticDStream(w={w},ws={sz}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
            }
        }
        // NULL workspace
        eqv(
            "initStaticCCtx(NULL,0) null?",
            icc(std::ptr::null_mut(), 0).is_null(),
            icr(std::ptr::null_mut(), 0).is_null(),
        );
        eqv(
            "initStaticDCtx(NULL,0) null?",
            idc(std::ptr::null_mut(), 0).is_null(),
            idr(std::ptr::null_mut(), 0).is_null(),
        );
    }
}
