//! Phase B/C for the remaining PUBLIC entry points not owned by another test
//! file: the `*_advanced` / `*_simpleArgs` / `initCStream_*` / `reset*Stream`
//! family, custom allocators, the struct-by-value parameter setters, and the
//! low-level block-header helpers.
//!
//! These are the "second-tier" API surfaces real bindings actually use
//! (`_simpleArgs` exists specifically for FFI binders), and each has its own
//! validation code in the C.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;
type CDict = *mut std::ffi::c_void;
type DDict = *mut std::ffi::c_void;

/// `ZSTD_customMem` = { ZSTD_allocFunction, ZSTD_freeFunction, void* opaque }
#[repr(C)]
#[derive(Copy, Clone)]
struct CustomMem {
    alloc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> *mut std::ffi::c_void>,
    free: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void)>,
    opaque: *mut std::ffi::c_void,
}

/// `ZSTD_compressionParameters`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct CParams {
    window_log: u32,
    chain_log: u32,
    hash_log: u32,
    search_log: u32,
    min_match: u32,
    target_length: u32,
    strategy: i32,
}

/// `ZSTD_frameParameters`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct FParams {
    content_size_flag: i32,
    checksum_flag: i32,
    no_dict_id_flag: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct Params {
    c: CParams,
    f: FParams,
}

type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_errCode = unsafe extern "C" fn(usize) -> i32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;

// ------------------------------------------------------- custom allocators

/// Counting allocator so we can prove the custom hooks were actually used.
static mut ALLOCS: u64 = 0;
static mut FREES: u64 = 0;

unsafe extern "C" fn my_alloc(_opaque: *mut std::ffi::c_void, size: usize) -> *mut std::ffi::c_void {
    unsafe {
        ALLOCS += 1;
        // over-allocate and store the layout size so `my_free` can rebuild it
        let total = size + 16;
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut usize).write(total);
        p.add(16) as *mut std::ffi::c_void
    }
}

unsafe extern "C" fn my_free(_opaque: *mut std::ffi::c_void, address: *mut std::ffi::c_void) {
    unsafe {
        if address.is_null() {
            return;
        }
        FREES += 1;
        let base = (address as *mut u8).sub(16);
        let total = (base as *mut usize).read();
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

/// `ZSTD_create*_advanced` with (a) the default allocator (all-NULL customMem)
/// and (b) a real custom allocator. Both libraries must produce byte-identical
/// output through contexts built either way.
#[test]
fn custom_allocator_contexts_match() {
    let i = impls();
    let (c_cca, r_cca) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> CCtx>("ZSTD_createCCtx_advanced");
    let (c_dca, r_dca) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> DCtx>("ZSTD_createDCtx_advanced");
    let (c_csa, r_csa) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> CCtx>("ZSTD_createCStream_advanced");
    let (c_dsa, r_dsa) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> DCtx>("ZSTD_createDStream_advanced");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_csfree, r_csfree) = i.pair::<Fn_freeCCtx>("ZSTD_freeCStream");
    let (c_dsfree, r_dsfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDStream");
    let (c_cc, r_cc) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, i32) -> usize>(
            "ZSTD_compressCCtx",
        );
    let (c_dd, r_dd) =
        i.pair::<unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_decompressDCtx",
        );
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let default_mem = CustomMem {
        alloc: None,
        free: None,
        opaque: std::ptr::null_mut(),
    };
    let custom_mem = CustomMem {
        alloc: Some(my_alloc),
        free: Some(my_free),
        opaque: std::ptr::null_mut(),
    };

    let mut rng = Rng::new(0xA110_C000);

    for (label, mem) in [("default", default_mem), ("custom", custom_mem)] {
        let before = unsafe { ALLOCS };

        let cc = unsafe { c_cca(mem) };
        let rc = unsafe { r_cca(mem) };
        let cd = unsafe { c_dca(mem) };
        let rd = unsafe { r_dca(mem) };
        assert!(
            !cc.is_null() && !rc.is_null() && !cd.is_null() && !rd.is_null(),
            "[{label}] create*_advanced returned NULL"
        );

        for &lvl in &[1i32, 3, 9, 19] {
            for &len in &[0usize, 1, 5000, 200_000] {
                let src = gen_shape(Shape::SkewedText, len, &mut rng);
                let cap = unsafe { c_bound(len) };
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = unsafe { c_cc(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
                let b = unsafe { r_cc(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
                let tag = format!("[{label}] compressCCtx lvl={lvl} len={len}");
                assert_eq_dbg(&tag, a, b);
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);

                let mut d1 = vec![0u8; len + 8];
                let mut d2 = vec![0u8; len + 8];
                let x = unsafe { c_dd(cd, d1.as_mut_ptr(), d1.len(), cb.as_ptr(), a) };
                let y = unsafe { r_dd(rd, d2.as_mut_ptr(), d2.len(), rb.as_ptr(), b) };
                assert_eq_dbg(&format!("{tag} / decode"), x, y);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d1[..x]);
            }
        }

        // the streaming context variants too
        let cs = unsafe { c_csa(mem) };
        let rs = unsafe { r_csa(mem) };
        let ds = unsafe { c_dsa(mem) };
        let dsr = unsafe { r_dsa(mem) };
        assert!(!cs.is_null() && !rs.is_null() && !ds.is_null() && !dsr.is_null());

        unsafe {
            c_csfree(cs);
            r_csfree(rs);
            c_dsfree(ds);
            r_dsfree(dsr);
            c_cfree(cc);
            r_cfree(rc);
            c_dfree(cd);
            r_dfree(rd);
        }

        if label == "custom" {
            let used = unsafe { ALLOCS } - before;
            assert!(
                used > 0,
                "custom allocator was never called — the test would be vacuous"
            );
            assert_eq_dbg("custom allocator alloc/free balance", unsafe { ALLOCS }, unsafe {
                FREES
            });
        }
    }
}

/// A custom allocator that always fails, to drive the C's
/// `memory_allocation` error paths deterministically.
unsafe extern "C" fn failing_alloc(
    _opaque: *mut std::ffi::c_void,
    _size: usize,
) -> *mut std::ffi::c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn noop_free(_o: *mut std::ffi::c_void, _a: *mut std::ffi::c_void) {}

/// Phase C: allocation failure must be reported identically (NULL context /
/// `ZSTD_error_memory_allocation`), not crash.
#[test]
fn allocation_failure_paths_match() {
    let i = impls();
    let (c_cca, r_cca) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> CCtx>("ZSTD_createCCtx_advanced");
    let (c_dca, r_dca) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> DCtx>("ZSTD_createDCtx_advanced");
    let (c_csa, r_csa) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> CCtx>("ZSTD_createCStream_advanced");
    let (c_dsa, r_dsa) =
        i.pair::<unsafe extern "C" fn(CustomMem) -> DCtx>("ZSTD_createDStream_advanced");
    // ZSTD_createCDict_advanced(const void* dict, size_t dictSize,
    //                           ZSTD_dictLoadMethod_e, ZSTD_dictContentType_e,
    //                           ZSTD_compressionParameters cParams,   <-- STRUCT by value
    //                           ZSTD_customMem customMem)
    // The 5th parameter is a 28-byte struct, not an int; declaring it as `i32`
    // mis-lays out the whole call and segfaults.
    let (c_cd, r_cd) = i.pair::<unsafe extern "C" fn(
        *const u8,
        usize,
        i32,
        i32,
        CParams,
        CustomMem,
    ) -> CDict>("ZSTD_createCDict_advanced");
    let (c_gc, _) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> CParams>("ZSTD_getCParams");

    let bad = CustomMem {
        alloc: Some(failing_alloc),
        free: Some(noop_free),
        opaque: std::ptr::null_mut(),
    };

    let a = unsafe { c_cca(bad) };
    let b = unsafe { r_cca(bad) };
    assert_eq_dbg("createCCtx_advanced(failing alloc)", a.is_null(), b.is_null());
    assert!(a.is_null(), "C should fail to allocate");

    let a = unsafe { c_dca(bad) };
    let b = unsafe { r_dca(bad) };
    assert_eq_dbg("createDCtx_advanced(failing alloc)", a.is_null(), b.is_null());

    let a = unsafe { c_csa(bad) };
    let b = unsafe { r_csa(bad) };
    assert_eq_dbg("createCStream_advanced(failing alloc)", a.is_null(), b.is_null());

    let a = unsafe { c_dsa(bad) };
    let b = unsafe { r_dsa(bad) };
    assert_eq_dbg("createDStream_advanced(failing alloc)", a.is_null(), b.is_null());

    let dict = vec![7u8; 4096];
    let cpar = unsafe { c_gc(3, 4096, dict.len()) };
    let a = unsafe { c_cd(dict.as_ptr(), dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cpar, bad) };
    let b = unsafe { r_cd(dict.as_ptr(), dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cpar, bad) };
    assert_eq_dbg("createCDict_advanced(failing alloc)", a.is_null(), b.is_null());
    assert!(a.is_null(), "C should fail to allocate the CDict");
}

/// `ZSTD_CCtx_setCParams` / `setFParams` / `setParams` /
/// `ZSTD_CCtxParams_init_advanced` pass whole structs by value.
#[test]
fn struct_by_value_param_setters_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_sc, r_sc) = i.pair::<unsafe extern "C" fn(CCtx, CParams) -> usize>("ZSTD_CCtx_setCParams");
    let (c_sf, r_sf) = i.pair::<unsafe extern "C" fn(CCtx, FParams) -> usize>("ZSTD_CCtx_setFParams");
    let (c_sp, r_sp) = i.pair::<unsafe extern "C" fn(CCtx, Params) -> usize>("ZSTD_CCtx_setParams");
    let (c_gc, r_gc) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> CParams>("ZSTD_getCParams");
    let (c_gp, r_gp) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> Params>("ZSTD_getParams");
    let (c_c2, r_c2) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compress2",
        );
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_cdE, r_cdE) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    // CCtxParams_init_advanced on a params object
    let (c_pnew, r_pnew) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtxParams");
    let (c_pia, r_pia) =
        i.pair::<unsafe extern "C" fn(CCtx, Params) -> usize>("ZSTD_CCtxParams_init_advanced");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cp = unsafe { c_pnew() };
    let rp = unsafe { r_pnew() };
    let mut rng = Rng::new(0x57B0_0001);

    // valid cParams derived from getCParams, plus randomized (often invalid) ones
    let mut cparam_cases: Vec<CParams> = Vec::new();
    for lvl in [-10i32, 1, 3, 9, 19, 22] {
        for ss in [0u64, 1000, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
            cparam_cases.push(unsafe { c_gc(lvl, ss, 0) });
        }
    }
    for _ in 0..400 {
        cparam_cases.push(CParams {
            window_log: rng.range(0, 35) as u32,
            chain_log: rng.range(0, 35) as u32,
            hash_log: rng.range(0, 35) as u32,
            search_log: rng.range(0, 35) as u32,
            min_match: rng.range(0, 10) as u32,
            target_length: rng.range(0, 5000) as u32,
            strategy: rng.range(0, 11) as i32,
        });
    }

    let src = gen_shape(Shape::SkewedText, 20_000, &mut rng);
    let cap = unsafe { c_bound(src.len()) } + 64;

    for (k, cpar) in cparam_cases.iter().enumerate() {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
        }
        let (a, b) = unsafe { (c_sc(cc, *cpar), r_sc(rc, *cpar)) };
        assert_eq_dbg(&format!("ZSTD_CCtx_setCParams[{k}]({cpar:?})"), a, b);
        unsafe {
            assert_eq_dbg(
                &format!("ZSTD_CCtx_setCParams[{k}] code"),
                c_cdE(a),
                r_cdE(b),
            )
        };
        if unsafe { c_isE(a) } != 0 {
            continue;
        }
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let x = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let y = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq_dbg(&format!("compress2 after setCParams[{k}]"), x, y);
        if unsafe { c_isE(x) } == 0 {
            assert_bytes_eq(&format!("frame after setCParams[{k}]"), &cb[..x], &rb[..y]);
        }
    }

    // fParams — all 8 flag combinations plus out-of-range values
    for cs in [-1i32, 0, 1, 2] {
        for ck in [-1i32, 0, 1, 2] {
            for nd in [-1i32, 0, 1, 2] {
                let f = FParams {
                    content_size_flag: cs,
                    checksum_flag: ck,
                    no_dict_id_flag: nd,
                };
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                }
                let (a, b) = unsafe { (c_sf(cc, f), r_sf(rc, f)) };
                assert_eq_dbg(&format!("ZSTD_CCtx_setFParams({f:?})"), a, b);
                unsafe {
                    assert_eq_dbg(&format!("setFParams({f:?}) code"), c_cdE(a), r_cdE(b))
                };
                if unsafe { c_isE(a) } != 0 {
                    continue;
                }
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                let y = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                assert_eq_dbg(&format!("compress2 after setFParams({f:?})"), x, y);
                if unsafe { c_isE(x) } == 0 {
                    assert_bytes_eq(&format!("frame setFParams({f:?})"), &cb[..x], &rb[..y]);
                }
            }
        }
    }

    // full ZSTD_parameters
    for lvl in [-5i32, 1, 3, 12, 19, 22] {
        for ss in [0u64, 5000, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
            let p = unsafe { c_gp(lvl, ss, 0) };
            let p2 = unsafe { r_gp(lvl, ss, 0) };
            assert_eq_dbg(&format!("getParams({lvl},{ss})"), p, p2);
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let (a, b) = unsafe { (c_sp(cc, p), r_sp(rc, p)) };
            assert_eq_dbg(&format!("ZSTD_CCtx_setParams({lvl},{ss})"), a, b);

            // and CCtxParams_init_advanced with the same struct
            let (x, y) = unsafe { (c_pia(cp, p), r_pia(rp, p)) };
            assert_eq_dbg(&format!("ZSTD_CCtxParams_init_advanced({lvl},{ss})"), x, y);

            if unsafe { c_isE(a) } != 0 {
                continue;
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let m = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            let n = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            assert_eq_dbg(&format!("compress2 after setParams({lvl},{ss})"), m, n);
            if unsafe { c_isE(m) } == 0 {
                assert_bytes_eq(&format!("frame setParams({lvl},{ss})"), &cb[..m], &rb[..n]);
            }
        }
    }

    unsafe {
        c_pfree(cp);
        r_pfree(rp);
        c_free(cc);
        r_free(rc);
    }
}

/// `ZSTD_compress_advanced` — the deprecated one-shot with explicit
/// `ZSTD_parameters` and a dictionary.
#[test]
fn compress_advanced_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_ca, r_ca) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *mut u8,
        usize,
        *const u8,
        usize,
        *const u8,
        usize,
        Params,
    ) -> usize>("ZSTD_compress_advanced");
    let (c_gp, _) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> Params>("ZSTD_getParams");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cdE, r_cdE) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_du, r_du) = i.pair::<unsafe extern "C" fn(
        DCtx,
        *mut u8,
        usize,
        *const u8,
        usize,
        *const u8,
        usize,
    ) -> usize>("ZSTD_decompress_usingDict");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0xADFA_0001);

    let dicts: Vec<(String, Vec<u8>)> = vec![
        ("none".into(), vec![]),
        ("tiny".into(), vec![1u8, 2, 3]),
        ("rand4k".into(), gen_shape(Shape::Random, 4096, &mut rng)),
        ("text32k".into(), gen_shape(Shape::SkewedText, 32768, &mut rng)),
    ];

    for (dn, dict) in &dicts {
        for &lvl in &[-3i32, 1, 3, 9, 19] {
            for &shape in &[Shape::SkewedText, Shape::Random, Shape::Constant] {
                for &len in &[0usize, 1, 4000, 150_000] {
                    let src = gen_shape(shape, len, &mut rng);
                    let params = unsafe { c_gp(lvl, len as u64, dict.len()) };
                    let cap = unsafe { c_bound(len) } + 64;
                    // include undersized destinations
                    for dcap in [cap, cap / 2, 8, 0] {
                        let mut cb = vec![0u8; dcap.max(1)];
                        let mut rb = vec![0u8; dcap.max(1)];
                        let a = unsafe {
                            c_ca(
                                cc,
                                cb.as_mut_ptr(),
                                dcap,
                                src.as_ptr(),
                                len,
                                dict.as_ptr(),
                                dict.len(),
                                params,
                            )
                        };
                        let b = unsafe {
                            r_ca(
                                rc,
                                rb.as_mut_ptr(),
                                dcap,
                                src.as_ptr(),
                                len,
                                dict.as_ptr(),
                                dict.len(),
                                params,
                            )
                        };
                        let tag = format!(
                            "compress_advanced dict={dn} lvl={lvl} shape={shape:?} len={len} dcap={dcap}"
                        );
                        assert_eq_dbg(&tag, a, b);
                        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cdE(a), r_cdE(b)) };
                        if unsafe { c_isE(a) } != 0 {
                            continue;
                        }
                        assert_bytes_eq(&tag, &cb[..a], &rb[..b]);

                        // round trip with the same dict, cross-library
                        let mut d1 = vec![0u8; len + 16];
                        let mut d2 = vec![0u8; len + 16];
                        let x = unsafe {
                            c_du(
                                cd,
                                d1.as_mut_ptr(),
                                d1.len(),
                                rb.as_ptr(),
                                b,
                                dict.as_ptr(),
                                dict.len(),
                            )
                        };
                        let y = unsafe {
                            r_du(
                                rd,
                                d2.as_mut_ptr(),
                                d2.len(),
                                cb.as_ptr(),
                                a,
                                dict.as_ptr(),
                                dict.len(),
                            )
                        };
                        assert_eq_dbg(&format!("{tag} / cross decode"), x, y);
                        assert_eq_dbg(&format!("{tag} / decode len"), x, len);
                        assert_bytes_eq(&format!("{tag} / payload"), &src, &d1[..x]);
                    }
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// `ZSTD_compressStream2_simpleArgs` / `ZSTD_decompressStream_simpleArgs` — the
/// pointer-free variants that language binders use. Positions are updated
/// in-place, so both the return code AND both positions must match every call.
#[test]
fn simple_args_streaming_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_sa, r_sa) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *mut u8,
        usize,
        *mut usize,
        *const u8,
        usize,
        *mut usize,
        i32,
    ) -> usize>("ZSTD_compressStream2_simpleArgs");
    let (c_dsa, r_dsa) = i.pair::<unsafe extern "C" fn(
        DCtx,
        *mut u8,
        usize,
        *mut usize,
        *const u8,
        usize,
        *mut usize,
    ) -> usize>("ZSTD_decompressStream_simpleArgs");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cdE, r_cdE) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0x51A0_0001);

    for &lvl in &[1i32, 3, 19] {
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 3000, 140_000] {
                let src = gen_shape(shape, len, &mut rng);
                let cap = unsafe { c_bound(len) } + 64;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];

                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                    c_set(cc, ZSTD_c_compressionLevel, lvl);
                    r_set(rc, ZSTD_c_compressionLevel, lvl);
                }

                // one-shot through simpleArgs with ZSTD_e_end
                let mut cdp = 0usize;
                let mut csp = 0usize;
                let mut rdp = 0usize;
                let mut rsp = 0usize;
                let a = unsafe {
                    c_sa(
                        cc,
                        cb.as_mut_ptr(),
                        cap,
                        &mut cdp,
                        src.as_ptr(),
                        len,
                        &mut csp,
                        ZSTD_e_end,
                    )
                };
                let b = unsafe {
                    r_sa(
                        rc,
                        rb.as_mut_ptr(),
                        cap,
                        &mut rdp,
                        src.as_ptr(),
                        len,
                        &mut rsp,
                        ZSTD_e_end,
                    )
                };
                let tag = format!("simpleArgs compress lvl={lvl} shape={shape:?} len={len}");
                assert_eq_dbg(&tag, a, b);
                assert_eq_dbg(&format!("{tag} dstPos"), cdp, rdp);
                assert_eq_dbg(&format!("{tag} srcPos"), csp, rsp);
                assert_bytes_eq(&tag, &cb[..cdp], &rb[..rdp]);
                assert_eq_dbg(&format!("{tag} completed"), a, 0);

                // decode via simpleArgs
                let mut d1 = vec![0u8; len + 64];
                let mut d2 = vec![0u8; len + 64];
                let mut ddp = 0usize;
                let mut dsp = 0usize;
                let mut ddp2 = 0usize;
                let mut dsp2 = 0usize;
                let x = unsafe {
                    c_dsa(
                        cd,
                        d1.as_mut_ptr(),
                        d1.len(),
                        &mut ddp,
                        cb.as_ptr(),
                        cdp,
                        &mut dsp,
                    )
                };
                let y = unsafe {
                    r_dsa(
                        rd,
                        d2.as_mut_ptr(),
                        d2.len(),
                        &mut ddp2,
                        rb.as_ptr(),
                        rdp,
                        &mut dsp2,
                    )
                };
                assert_eq_dbg(&format!("{tag} / decode rc"), x, y);
                assert_eq_dbg(&format!("{tag} / decode dstPos"), ddp, ddp2);
                assert_eq_dbg(&format!("{tag} / decode srcPos"), dsp, dsp2);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d1[..ddp]);
            }
        }
    }

    // Phase C: undersized dst, out-of-range endOp
    let src = gen_shape(Shape::SkewedText, 5000, &mut rng);
    for endop in [-1i32, 0, 1, 2, 3, 99] {
        for dcap in [0usize, 1, 16] {
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let mut cb = vec![0u8; dcap.max(1)];
            let mut rb = vec![0u8; dcap.max(1)];
            let mut a1 = 0usize;
            let mut a2 = 0usize;
            let mut b1 = 0usize;
            let mut b2 = 0usize;
            let a = unsafe {
                c_sa(cc, cb.as_mut_ptr(), dcap, &mut a1, src.as_ptr(), src.len(), &mut a2, endop)
            };
            let b = unsafe {
                r_sa(rc, rb.as_mut_ptr(), dcap, &mut b1, src.as_ptr(), src.len(), &mut b2, endop)
            };
            let tag = format!("simpleArgs endOp={endop} dcap={dcap}");
            assert_eq_dbg(&tag, a, b);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cdE(a), r_cdE(b)) };
            assert_eq_dbg(&format!("{tag} dstPos"), a1, b1);
            assert_eq_dbg(&format!("{tag} srcPos"), a2, b2);
            let _ = c_isE;
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// The whole `ZSTD_initCStream_*` / `ZSTD_resetCStream` / `ZSTD_initDStream_*` /
/// `ZSTD_resetDStream` family, each of which has its own init path in the C.
#[test]
fn init_reset_stream_variants_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCStream");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCStream");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDStream");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDStream");

    let (c_i1, r_i1) = i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_initCStream");
    let (c_i2, r_i2) =
        i.pair::<unsafe extern "C" fn(CCtx, i32, u64) -> usize>("ZSTD_initCStream_srcSize");
    let (c_i3, r_i3) = i
        .pair::<unsafe extern "C" fn(CCtx, *const u8, usize, i32) -> usize>(
            "ZSTD_initCStream_usingDict",
        );
    let (c_i4, r_i4) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *const u8,
        usize,
        Params,
        u64,
    ) -> usize>("ZSTD_initCStream_advanced");
    let (c_i5, r_i5) =
        i.pair::<unsafe extern "C" fn(CCtx, CDict) -> usize>("ZSTD_initCStream_usingCDict");
    let (c_i6, r_i6) = i.pair::<unsafe extern "C" fn(
        CCtx,
        CDict,
        FParams,
        u64,
    ) -> usize>("ZSTD_initCStream_usingCDict_advanced");
    let (c_rcs, r_rcs) = i.pair::<unsafe extern "C" fn(CCtx, u64) -> usize>("ZSTD_resetCStream");

    let (c_d1, r_d1) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_initDStream");
    let (c_d2, r_d2) = i
        .pair::<unsafe extern "C" fn(DCtx, *const u8, usize) -> usize>("ZSTD_initDStream_usingDict");
    let (c_d3, r_d3) =
        i.pair::<unsafe extern "C" fn(DCtx, DDict) -> usize>("ZSTD_initDStream_usingDDict");
    let (c_rds, r_rds) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_resetDStream");

    let (c_ccd, r_ccd) =
        i.pair::<unsafe extern "C" fn(*const u8, usize, i32) -> CDict>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = i.pair::<unsafe extern "C" fn(CDict) -> usize>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) =
        i.pair::<unsafe extern "C" fn(*const u8, usize) -> DDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = i.pair::<unsafe extern "C" fn(DDict) -> usize>("ZSTD_freeDDict");

    let (c_cs, r_cs) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize>(
            "ZSTD_compressStream",
        );
    let (c_en, r_en) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut ZSTD_outBuffer) -> usize>("ZSTD_endStream");
    let (c_ds, r_ds) =
        i.pair::<unsafe extern "C" fn(DCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize>(
            "ZSTD_decompressStream",
        );
    let (c_gp, _) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> Params>("ZSTD_getParams");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0x1417_0001);

    let dict = gen_shape(Shape::SkewedText, 8192, &mut rng);
    let ccdict = unsafe { c_ccd(dict.as_ptr(), dict.len(), 3) };
    let rcdict = unsafe { r_ccd(dict.as_ptr(), dict.len(), 3) };
    let cddict = unsafe { c_cdd(dict.as_ptr(), dict.len()) };
    let rddict = unsafe { r_cdd(dict.as_ptr(), dict.len()) };
    assert!(!ccdict.is_null() && !rcdict.is_null() && !cddict.is_null() && !rddict.is_null());

    /// One full stream through compressStream+endStream, returning the frame.
    macro_rules! run_stream {
        ($csf:expr, $enf:expr, $ctx:expr, $src:expr, $cap:expr) => {{
            let mut out = vec![0u8; $cap];
            let mut got = Vec::new();
            let mut inb = ZSTD_inBuffer {
                src: $src.as_ptr(),
                size: $src.len(),
                pos: 0,
            };
            let mut err = 0usize;
            loop {
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr(),
                    size: out.len(),
                    pos: 0,
                };
                let rc = unsafe { $csf($ctx, &mut ob, &mut inb) };
                got.extend_from_slice(&out[..ob.pos]);
                if unsafe { c_isE(rc) } != 0 {
                    err = rc;
                    break;
                }
                if inb.pos == inb.size {
                    break;
                }
            }
            if err == 0 {
                loop {
                    let mut ob = ZSTD_outBuffer {
                        dst: out.as_mut_ptr(),
                        size: out.len(),
                        pos: 0,
                    };
                    let rc = unsafe { $enf($ctx, &mut ob) };
                    got.extend_from_slice(&out[..ob.pos]);
                    if unsafe { c_isE(rc) } != 0 {
                        err = rc;
                        break;
                    }
                    if rc == 0 {
                        break;
                    }
                }
            }
            (got, err)
        }};
    }

    for &len in &[0usize, 1, 2500, 130_000] {
        let src = gen_shape(Shape::Tabular, len, &mut rng);
        let cap = unsafe { c_bound(len) } + 1024;

        // --- initCStream(level)
        for &lvl in &[1i32, 3, 19] {
            unsafe {
                assert_eq_dbg("initCStream", c_i1(cc, lvl), r_i1(rc, lvl));
            }
            let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
            let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
            assert_eq_dbg(&format!("initCStream lvl={lvl} len={len} err"), ae, be);
            assert_bytes_eq(&format!("initCStream lvl={lvl} len={len}"), &a, &b);
        }

        // --- initCStream_srcSize(level, pledged)
        for &lvl in &[1i32, 9] {
            for pledged in [len as u64, ZSTD_CONTENTSIZE_UNKNOWN, 0] {
                unsafe {
                    assert_eq_dbg(
                        "initCStream_srcSize",
                        c_i2(cc, lvl, pledged),
                        r_i2(rc, lvl, pledged),
                    );
                }
                let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
                let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
                assert_eq_dbg(
                    &format!("initCStream_srcSize lvl={lvl} pledged={pledged} err"),
                    ae,
                    be,
                );
                assert_bytes_eq(
                    &format!("initCStream_srcSize lvl={lvl} pledged={pledged} len={len}"),
                    &a,
                    &b,
                );
            }
        }

        // --- initCStream_usingDict
        for &lvl in &[1i32, 9] {
            for d in [&dict[..], &[][..]] {
                unsafe {
                    assert_eq_dbg(
                        "initCStream_usingDict",
                        c_i3(cc, d.as_ptr(), d.len(), lvl),
                        r_i3(rc, d.as_ptr(), d.len(), lvl),
                    );
                }
                let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
                let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
                assert_eq_dbg(
                    &format!("initCStream_usingDict lvl={lvl} dict={} err", d.len()),
                    ae,
                    be,
                );
                assert_bytes_eq(
                    &format!("initCStream_usingDict lvl={lvl} dict={} len={len}", d.len()),
                    &a,
                    &b,
                );
            }
        }

        // --- initCStream_advanced
        for &lvl in &[1i32, 9, 19] {
            let params = unsafe { c_gp(lvl, len as u64, dict.len()) };
            for pledged in [len as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                unsafe {
                    assert_eq_dbg(
                        "initCStream_advanced",
                        c_i4(cc, dict.as_ptr(), dict.len(), params, pledged),
                        r_i4(rc, dict.as_ptr(), dict.len(), params, pledged),
                    );
                }
                let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
                let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
                assert_eq_dbg(
                    &format!("initCStream_advanced lvl={lvl} pledged={pledged} err"),
                    ae,
                    be,
                );
                assert_bytes_eq(
                    &format!("initCStream_advanced lvl={lvl} pledged={pledged} len={len}"),
                    &a,
                    &b,
                );
            }
        }

        // --- initCStream_usingCDict (+_advanced)
        unsafe {
            assert_eq_dbg(
                "initCStream_usingCDict",
                c_i5(cc, ccdict),
                r_i5(rc, rcdict),
            );
        }
        let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
        let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
        assert_eq_dbg(&format!("initCStream_usingCDict len={len} err"), ae, be);
        assert_bytes_eq(&format!("initCStream_usingCDict len={len}"), &a, &b);

        for cs in [0i32, 1] {
            for ck in [0i32, 1] {
                for nd in [0i32, 1] {
                    let fp = FParams {
                        content_size_flag: cs,
                        checksum_flag: ck,
                        no_dict_id_flag: nd,
                    };
                    for pledged in [len as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                        unsafe {
                            assert_eq_dbg(
                                "initCStream_usingCDict_advanced",
                                c_i6(cc, ccdict, fp, pledged),
                                r_i6(rc, rcdict, fp, pledged),
                            );
                        }
                        let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
                        let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
                        assert_eq_dbg(
                            &format!("initCStream_usingCDict_advanced {fp:?} p={pledged} err"),
                            ae,
                            be,
                        );
                        assert_bytes_eq(
                            &format!(
                                "initCStream_usingCDict_advanced {fp:?} p={pledged} len={len}"
                            ),
                            &a,
                            &b,
                        );
                    }
                }
            }
        }

        // --- resetCStream after an init (documented requirement)
        unsafe {
            c_i1(cc, 3);
            r_i1(rc, 3);
        }
        for pledged in [len as u64, 0u64, ZSTD_CONTENTSIZE_UNKNOWN, 12345] {
            unsafe {
                assert_eq_dbg(
                    &format!("resetCStream({pledged})"),
                    c_rcs(cc, pledged),
                    r_rcs(rc, pledged),
                );
            }
            let (a, ae) = run_stream!(c_cs, c_en, cc, src, cap);
            let (b, be) = run_stream!(r_cs, r_en, rc, src, cap);
            assert_eq_dbg(&format!("resetCStream({pledged}) len={len} err"), ae, be);
            assert_bytes_eq(&format!("resetCStream({pledged}) len={len}"), &a, &b);
        }

        // --- decoder init variants over a dictionary-compressed frame
        unsafe {
            c_i3(cc, dict.as_ptr(), dict.len(), 3);
        }
        let (frame, ferr) = run_stream!(c_cs, c_en, cc, src, cap);
        assert_eq_dbg("dict frame produced", ferr, 0);

        for variant in 0..4 {
            let (x, y) = unsafe {
                match variant {
                    0 => (c_d1(cd), r_d1(rd)),
                    1 => (
                        c_d2(cd, dict.as_ptr(), dict.len()),
                        r_d2(rd, dict.as_ptr(), dict.len()),
                    ),
                    2 => (c_d3(cd, cddict), r_d3(rd, rddict)),
                    _ => {
                        c_d2(cd, dict.as_ptr(), dict.len());
                        r_d2(rd, dict.as_ptr(), dict.len());
                        (c_rds(cd), r_rds(rd))
                    }
                }
            };
            assert_eq_dbg(&format!("dstream init variant {variant}"), x, y);

            // variant 0 (no dict) legitimately fails on a dict frame; both must
            // agree either way.
            let mut co = vec![0u8; len + 1024];
            let mut ro = vec![0u8; len + 1024];
            let mut ci = ZSTD_inBuffer {
                src: frame.as_ptr(),
                size: frame.len(),
                pos: 0,
            };
            let mut ri = ci;
            let mut cob = ZSTD_outBuffer {
                dst: co.as_mut_ptr(),
                size: co.len(),
                pos: 0,
            };
            let mut rob = ZSTD_outBuffer {
                dst: ro.as_mut_ptr(),
                size: ro.len(),
                pos: 0,
            };
            let a = unsafe { c_ds(cd, &mut cob, &mut ci) };
            let b = unsafe { r_ds(rd, &mut rob, &mut ri) };
            assert_eq_dbg(&format!("dstream variant {variant} rc"), a, b);
            assert_eq_dbg(&format!("dstream variant {variant} in.pos"), ci.pos, ri.pos);
            assert_eq_dbg(&format!("dstream variant {variant} out.pos"), cob.pos, rob.pos);
            assert_bytes_eq(
                &format!("dstream variant {variant} payload"),
                &co[..cob.pos],
                &ro[..rob.pos],
            );
        }
    }

    unsafe {
        c_fcd(ccdict);
        r_fcd(rcdict);
        c_fdd(cddict);
        r_fdd(rddict);
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// `ZSTD_DCtx_getParameter` / `ZSTD_DCtx_setFormat` / `ZSTD_DCtx_setMaxWindowSize`
/// and the `ZSTD_DDict_dictContent` / `ZSTD_DDict_dictSize` accessors.
#[test]
fn dctx_param_accessors_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_free, r_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_set, r_set) = i.pair::<unsafe extern "C" fn(DCtx, i32, i32) -> usize>(
        "ZSTD_DCtx_setParameter",
    );
    let (c_get, r_get) =
        i.pair::<unsafe extern "C" fn(DCtx, i32, *mut i32) -> usize>("ZSTD_DCtx_getParameter");
    let (c_sf, r_sf) =
        i.pair::<unsafe extern "C" fn(DCtx, i32) -> usize>("ZSTD_DCtx_setFormat");
    let (c_mw, r_mw) =
        i.pair::<unsafe extern "C" fn(DCtx, usize) -> usize>("ZSTD_DCtx_setMaxWindowSize");
    let (c_rst, r_rst) = i.pair::<unsafe extern "C" fn(DCtx, i32) -> usize>("ZSTD_DCtx_reset");
    let (c_db, r_db) = i.pair::<unsafe extern "C" fn(i32) -> ZSTD_bounds>("ZSTD_dParam_getBounds");
    let (c_cdE, r_cdE) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let (c_cdd, r_cdd) =
        i.pair::<unsafe extern "C" fn(*const u8, usize) -> DDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = i.pair::<unsafe extern "C" fn(DDict) -> usize>("ZSTD_freeDDict");
    let (c_dc, r_dc) =
        i.pair::<unsafe extern "C" fn(DDict) -> *const u8>("ZSTD_DDict_dictContent");
    let (c_dsz, r_dsz) = i.pair::<unsafe extern "C" fn(DDict) -> usize>("ZSTD_DDict_dictSize");

    let cd = unsafe { c_new() };
    let rd = unsafe { r_new() };

    // full dParameter sweep with getParameter readback, incl. invalid ids
    let ids = [
        ZSTD_d_windowLogMax,
        ZSTD_d_format,
        ZSTD_d_stableOutBuffer,
        ZSTD_d_forceIgnoreChecksum,
        ZSTD_d_refMultipleDDicts,
        ZSTD_d_disableHuffmanAssembly,
        ZSTD_d_maxBlockSize,
        i32::MIN,
        -1,
        0,
        99,
        101,
        1006,
        99999,
        i32::MAX,
    ];
    for id in ids {
        let b = unsafe { c_db(id) };
        let b2 = unsafe { r_db(id) };
        assert_eq_dbg(&format!("dParam_getBounds({id})"), b, b2);

        let mut probes: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX];
        if b.error == 0 {
            for base in [b.lower_bound, b.upper_bound] {
                for d in [-1i32, 0, 1] {
                    probes.push(base.saturating_add(d));
                }
            }
        }
        for v in probes {
            unsafe {
                c_rst(cd, ZSTD_reset_session_and_parameters);
                r_rst(rd, ZSTD_reset_session_and_parameters);
            }
            let (x, y) = unsafe { (c_set(cd, id, v), r_set(rd, id, v)) };
            let tag = format!("DCtx_setParameter({id},{v})");
            assert_eq_dbg(&tag, x, y);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cdE(x), r_cdE(y)) };

            let mut o1 = -777i32;
            let mut o2 = -777i32;
            let (g1, g2) = unsafe { (c_get(cd, id, &mut o1), r_get(rd, id, &mut o2)) };
            assert_eq_dbg(&format!("{tag} / get rc"), g1, g2);
            assert_eq_dbg(&format!("{tag} / get value"), o1, o2);
        }
    }

    // setFormat with valid and out-of-range enum values
    for f in [0i32, 1, 2, -1, 77, i32::MIN, i32::MAX] {
        unsafe {
            c_rst(cd, ZSTD_reset_session_and_parameters);
            r_rst(rd, ZSTD_reset_session_and_parameters);
            let (x, y) = (c_sf(cd, f), r_sf(rd, f));
            assert_eq_dbg(&format!("DCtx_setFormat({f})"), x, y);
            assert_eq_dbg(&format!("DCtx_setFormat({f}) code"), c_cdE(x), r_cdE(y));
        }
    }

    // setMaxWindowSize across the valid/invalid boundary
    for w in [
        0usize,
        1,
        1 << 9,
        1 << 10,
        1 << 20,
        1 << 27,
        (1 << 27) + 1,
        1 << 31,
        usize::MAX,
    ] {
        unsafe {
            c_rst(cd, ZSTD_reset_session_and_parameters);
            r_rst(rd, ZSTD_reset_session_and_parameters);
            let (x, y) = (c_mw(cd, w), r_mw(rd, w));
            assert_eq_dbg(&format!("DCtx_setMaxWindowSize({w})"), x, y);
            assert_eq_dbg(
                &format!("DCtx_setMaxWindowSize({w}) code"),
                c_cdE(x),
                r_cdE(y),
            );
        }
    }

    // DDict accessors, including the empty dictionary
    let mut rng = Rng::new(0xDD1C_0001);
    for dlen in [0usize, 1, 100, 8192] {
        let dict = gen_shape(Shape::SkewedText, dlen, &mut rng);
        let a = unsafe { c_cdd(dict.as_ptr(), dlen) };
        let b = unsafe { r_cdd(dict.as_ptr(), dlen) };
        assert_eq_dbg(
            &format!("createDDict({dlen}) null-ness"),
            a.is_null(),
            b.is_null(),
        );
        if a.is_null() || b.is_null() {
            continue;
        }
        unsafe {
            assert_eq_dbg(&format!("DDict_dictSize({dlen})"), c_dsz(a), r_dsz(b));
            let n = c_dsz(a);
            let pa = c_dc(a);
            let pb = r_dc(b);
            assert_eq_dbg(
                &format!("DDict_dictContent({dlen}) null-ness"),
                pa.is_null(),
                pb.is_null(),
            );
            if !pa.is_null() && !pb.is_null() && n > 0 {
                let sa = std::slice::from_raw_parts(pa, n);
                let sb = std::slice::from_raw_parts(pb, n);
                assert_bytes_eq(&format!("DDict_dictContent({dlen}) bytes"), sa, sb);
            }
            c_fdd(a);
            r_fdd(b);
        }
    }

    unsafe {
        c_free(cd);
        r_free(rd);
    }
}

/// Low-level block-header helpers: `ZSTD_getcBlockSize`,
/// `ZSTD_writeLastEmptyBlock`, `ZSTD_cycleLog`, `ZSTD_getBlockSize`.
#[test]
fn block_header_helpers_match() {
    let i = impls();
    let (c_cl, r_cl) = i.pair::<unsafe extern "C" fn(u32, i32) -> u32>("ZSTD_cycleLog");
    for hl in 0u32..=32 {
        for strat in -2i32..=11 {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_cycleLog({hl},{strat})"),
                    c_cl(hl, strat),
                    r_cl(hl, strat),
                )
            };
        }
    }

    let (c_we, r_we) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize) -> usize>("ZSTD_writeLastEmptyBlock");
    for cap in [0usize, 1, 2, 3, 4, 8, 64] {
        let mut a = vec![0xAAu8; cap.max(1)];
        let mut b = vec![0x55u8; cap.max(1)];
        let x = unsafe { c_we(a.as_mut_ptr(), cap) };
        let y = unsafe { r_we(b.as_mut_ptr(), cap) };
        assert_eq_dbg(&format!("writeLastEmptyBlock({cap})"), x, y);
        if x <= usize::MAX - 200 {
            assert_bytes_eq(&format!("writeLastEmptyBlock({cap}) bytes"), &a[..x], &b[..y]);
        }
    }
    // NULL dst with zero capacity
    unsafe {
        assert_eq_dbg(
            "writeLastEmptyBlock(NULL,0)",
            c_we(std::ptr::null_mut(), 0),
            r_we(std::ptr::null_mut(), 0),
        );
    }

    // ZSTD_getcBlockSize parses a 3-byte block header into a blockProperties_t.
    // blockProperties_t = { blockType_e blockType; U32 lastBlock; U32 origSize; }
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
    struct BlockProps {
        block_type: i32,
        last_block: u32,
        orig_size: u32,
    }
    let (c_gb, r_gb) =
        i.pair::<unsafe extern "C" fn(*const u8, usize, *mut BlockProps) -> usize>(
            "ZSTD_getcBlockSize",
        );

    let mut rng = Rng::new(0xB1C0_0001);
    // exhaustive-ish over the 3 header bytes plus short buffers
    for _ in 0..20_000 {
        let n = rng.range(0, 5);
        let mut buf = [0u8; 8];
        for k in 0..8 {
            buf[k] = rng.byte();
        }
        let mut p1 = BlockProps::default();
        let mut p2 = BlockProps::default();
        let x = unsafe { c_gb(buf.as_ptr(), n, &mut p1) };
        let y = unsafe { r_gb(buf.as_ptr(), n, &mut p2) };
        let tag = format!("getcBlockSize(n={n}, hdr={:02x?})", &buf[..n.min(3)]);
        assert_eq_dbg(&tag, x, y);
        assert_eq_dbg(&format!("{tag} props"), p1, p2);
    }
    // and every possible 3-byte header for the low bits that matter
    for b0 in 0u16..=255 {
        for b1 in [0u8, 1, 0x7F, 0x80, 0xFF] {
            for b2 in [0u8, 1, 0x7F, 0x80, 0xFF] {
                let buf = [b0 as u8, b1, b2];
                let mut p1 = BlockProps::default();
                let mut p2 = BlockProps::default();
                let x = unsafe { c_gb(buf.as_ptr(), 3, &mut p1) };
                let y = unsafe { r_gb(buf.as_ptr(), 3, &mut p2) };
                let tag = format!("getcBlockSize({b0:02x} {b1:02x} {b2:02x})");
                assert_eq_dbg(&tag, x, y);
                assert_eq_dbg(&format!("{tag} props"), p1, p2);
            }
        }
    }
}
