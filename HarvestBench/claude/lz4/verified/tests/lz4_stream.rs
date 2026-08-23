//! Differential tests for the lz4.c STREAMING + dictionary + obsolete APIs.
//!
//! These exercise the low-level entry points a real streaming consumer drives:
//! `LZ4_createStream` / `LZ4_loadDict*` / `LZ4_attach_dictionary` /
//! `LZ4_compress_fast_continue` / `LZ4_saveDict`, the matching decode-side
//! `LZ4_setStreamDecode` / `LZ4_decompress_*_continue`, the explicit
//! prefix64k / forceExtDict instantiations, and every obsolete wrapper.
//!
//! Every call goes through BOTH shared libraries' export tables.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnResetStream = unsafe extern "C" fn(*mut c_void);
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnLoadDictInternal =
    unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
type FnAttachDict = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnCompContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCompContinueLegacy =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompContinueNoCap =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnForceExtDictComp =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnSizeofState = unsafe extern "C" fn() -> c_int;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;

type FnCreateStreamDecode = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStreamDecode = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnSetStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnDecompSafeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompFastContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;

type FnDecompSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnPrefix64kSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnPrefix64kFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnSafeForceExtDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_void, usize) -> c_int;
type FnPartialForceExtDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;

// obsolete
type FnComp3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnComp4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompState4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnCompState5 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCreate = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
type FnSlideInput = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

const SENTINEL: u8 = 0xAA;

/// Size of `LZ4_stream_t`, queried from the C library at runtime.
fn stream_size() -> usize {
    let (c, r) = both::<FnSizeofState>("LZ4_sizeofStreamState");
    let cs = unsafe { c() } as usize;
    let rs = unsafe { r() } as usize;
    assert_eq!(cs, rs, "LZ4_sizeofStreamState must agree between C and Rust");
    cs
}

// ===========================================================================
// Stream lifecycle: create / init / reset / reset_fast / free
// ===========================================================================

#[test]
fn stream_lifecycle_and_init_alignment() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_is, r_is) = both::<FnInitStream>("LZ4_initStream");
    let (c_rs, r_rs) = both::<FnResetStream>("LZ4_resetStream");
    let (c_rf, r_rf) = both::<FnResetStream>("LZ4_resetStream_fast");

    let ssz = stream_size();

    unsafe {
        // create/free
        let c = c_cs();
        let r = r_cs();
        assert!(!c.is_null(), "C LZ4_createStream returned NULL");
        assert!(!r.is_null(), "Rust LZ4_createStream returned NULL");
        // A freshly created stream must have identical contents.
        assert_bytes_eq(
            "LZ4_createStream fresh state",
            std::slice::from_raw_parts(c as *const u8, ssz),
            std::slice::from_raw_parts(r as *const u8, ssz),
        );
        // reset / reset_fast must keep them identical
        c_rs(c);
        r_rs(r);
        assert_bytes_eq(
            "LZ4_resetStream state",
            std::slice::from_raw_parts(c as *const u8, ssz),
            std::slice::from_raw_parts(r as *const u8, ssz),
        );
        c_rf(c);
        r_rf(r);
        assert_bytes_eq(
            "LZ4_resetStream_fast state",
            std::slice::from_raw_parts(c as *const u8, ssz),
            std::slice::from_raw_parts(r as *const u8, ssz),
        );
        assert_eq!(c_fs(c), r_fs(r), "LZ4_freeStream return");

        // LZ4_initStream: sufficient size + correct alignment -> non-NULL
        let mut cbuf = AlignedBuf::new(ssz, 64);
        let mut rbuf = AlignedBuf::new(ssz, 64);
        let cp = c_is(cbuf.as_mut_ptr() as *mut c_void, ssz);
        let rp = r_is(rbuf.as_mut_ptr() as *mut c_void, ssz);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "LZ4_initStream nullness (aligned, size={})",
            ssz
        );
        assert!(!cp.is_null(), "C initStream should accept aligned/large-enough");
        assert_bytes_eq(
            "LZ4_initStream initialized state",
            cbuf.as_slice(),
            rbuf.as_slice(),
        );
    }
}

// ===========================================================================
// Streaming compression: multi-block sequences, blockLinked history
// ===========================================================================

/// Compress `blocks` sequentially through one stream in BOTH libraries and
/// require byte-identical output for every block plus identical stream state.
fn stream_compress_blocks(
    dict: &[u8],
    blocks: &[Vec<u8>],
    accel: c_int,
    load_mode: LoadMode,
    label: &str,
) -> Vec<Vec<u8>> {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let ssz = stream_size();

    unsafe {
        let cst = c_cs();
        let rst = r_cs();
        assert!(!cst.is_null() && !rst.is_null());

        load_mode.apply(cst, rst, dict, label);

        let mut out = Vec::new();
        // Keep all source blocks alive for the whole stream: blockLinked mode
        // requires the previous block to remain addressable.
        for (i, blk) in blocks.iter().enumerate() {
            let bound = cb(blk.len() as c_int).max(1) as usize;
            let mut cdst = vec![SENTINEL; bound];
            let mut rdst = vec![SENTINEL; bound];
            let cn = c_cc(
                cst,
                blk.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                blk.len() as c_int,
                bound as c_int,
                accel,
            );
            let rn = r_cc(
                rst,
                blk.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                blk.len() as c_int,
                bound as c_int,
                accel,
            );
            let l = format!("{} block[{}] len={}", label, i, blk.len());
            assert_eq!(cn, rn, "{}: compress_fast_continue return", l);
            assert_bytes_eq(&format!("{} output", l), &cdst, &rdst);
            assert_bytes_eq(
                &format!("{} STREAM STATE", l),
                std::slice::from_raw_parts(cst as *const u8, ssz),
                std::slice::from_raw_parts(rst as *const u8, ssz),
            );
            cdst.truncate(cn.max(0) as usize);
            out.push(cdst);
        }

        assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);
        out
    }
}

#[derive(Clone, Copy, Debug)]
enum LoadMode {
    None,
    LoadDict,
    LoadDictSlow,
    /// `LZ4_loadDict_internal` with the raw `LoadDict_mode_e` value.
    Internal(c_int),
    /// `LZ4_attach_dictionary` from a separate dictionary stream.
    Attach,
}

impl LoadMode {
    fn apply(self, cst: *mut c_void, rst: *mut c_void, dict: &[u8], label: &str) {
        let dptr = if dict.is_empty() {
            std::ptr::null()
        } else {
            dict.as_ptr() as *const c_char
        };
        unsafe {
            match self {
                LoadMode::None => {}
                LoadMode::LoadDict => {
                    let (c, r) = both::<FnLoadDict>("LZ4_loadDict");
                    assert_eq!(
                        c(cst, dptr, dict.len() as c_int),
                        r(rst, dptr, dict.len() as c_int),
                        "{}: LZ4_loadDict return",
                        label
                    );
                }
                LoadMode::LoadDictSlow => {
                    let (c, r) = both::<FnLoadDict>("LZ4_loadDictSlow");
                    assert_eq!(
                        c(cst, dptr, dict.len() as c_int),
                        r(rst, dptr, dict.len() as c_int),
                        "{}: LZ4_loadDictSlow return",
                        label
                    );
                }
                LoadMode::Internal(mode) => {
                    let (c, r) = both::<FnLoadDictInternal>("LZ4_loadDict_internal");
                    assert_eq!(
                        c(cst, dptr, dict.len() as c_int, mode),
                        r(rst, dptr, dict.len() as c_int, mode),
                        "{}: LZ4_loadDict_internal(mode={}) return",
                        label,
                        mode
                    );
                }
                LoadMode::Attach => {
                    // Build a separate dictionary stream, load the dict into it,
                    // then attach it to the working stream.
                    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
                    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
                    let (c_ad, r_ad) = both::<FnAttachDict>("LZ4_attach_dictionary");
                    let cd = c_cs();
                    let rd = r_cs();
                    assert_eq!(
                        c_ld(cd, dptr, dict.len() as c_int),
                        r_ld(rd, dptr, dict.len() as c_int),
                        "{}: attach/loadDict return",
                        label
                    );
                    c_ad(cst, cd as *const c_void);
                    r_ad(rst, rd as *const c_void);
                    // dictionary streams are intentionally leaked for the
                    // lifetime of the working stream (they must stay valid).
                    let _ = (cd, rd);
                }
            }
        }
    }
}

#[test]
fn stream_compress_multiblock_all_load_modes() {
    let mut rng = Rng::new(0x5712_EA33);
    let load_modes = [
        LoadMode::None,
        LoadMode::LoadDict,
        LoadMode::LoadDictSlow,
        LoadMode::Internal(0), // _ld_fast
        LoadMode::Internal(1), // _ld_slow
        LoadMode::Attach,
    ];

    for &lm in &load_modes {
        // dict sizes spanning: none, dictSmall(<16KB) issue path, exactly 64 KB,
        // and >64 KB (C truncates the retained history to the last 64 KB).
        for &dict_len in &[0usize, 1, 64, 4096, 16383, 16384, 65535, 65536, 80000] {
            if matches!(lm, LoadMode::None) && dict_len != 0 {
                continue;
            }
            for shape in 0..N_SHAPES {
                let dict = gen_shape(&mut rng, shape, dict_len);
                // Block-size patterns: tiny, sub-MINMATCH, straddling 64 KB,
                // and a long tail of many small blocks.
                let patterns: Vec<Vec<usize>> = vec![
                    vec![1, 1, 1],
                    vec![3, 4, 5, 12, 13],
                    vec![100, 200, 400],
                    vec![65535, 1, 65536],
                    vec![70000, 5000],
                    vec![17, 17, 17, 17, 17, 17, 17, 17],
                ];
                for pat in &patterns {
                    let blocks: Vec<Vec<u8>> =
                        pat.iter().map(|&n| gen_shape(&mut rng, shape, n)).collect();
                    for &accel in &[1i32, 3, 65537] {
                        let label = format!(
                            "stream lm={:?} dict={} shape={} pat={:?} accel={}",
                            lm,
                            dict_len,
                            shape_name(shape),
                            pat,
                            accel
                        );
                        let comps =
                            stream_compress_blocks(&dict, &blocks, accel, lm, &label);
                        // Validate the whole stream decodes back, using the
                        // decode-side streaming API, in both libraries.
                        stream_decompress_and_verify(&dict, &blocks, &comps, &label);
                    }
                }
            }
        }
    }
}

/// Decode a linked-block stream with `LZ4_decompress_safe_continue` and
/// `LZ4_decompress_fast_continue` in BOTH libraries into a contiguous buffer.
fn stream_decompress_and_verify(
    dict: &[u8],
    blocks: &[Vec<u8>],
    comps: &[Vec<u8>],
    label: &str,
) {
    let (c_cd, r_cd) = both::<FnCreateStreamDecode>("LZ4_createStreamDecode");
    let (c_fd, r_fd) = both::<FnFreeStreamDecode>("LZ4_freeStreamDecode");
    let (c_sd, r_sd) = both::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let (c_dc, r_dc) = both::<FnDecompSafeContinue>("LZ4_decompress_safe_continue");
    let (c_df, r_df) = both::<FnDecompFastContinue>("LZ4_decompress_fast_continue");

    let total: usize = blocks.iter().map(|b| b.len()).sum();
    let dptr = if dict.is_empty() {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };

    unsafe {
        // ---- safe_continue
        let cds = c_cd();
        let rds = r_cd();
        assert!(!cds.is_null() && !rds.is_null(), "{}: createStreamDecode", label);
        assert_eq!(
            c_sd(cds, dptr, dict.len() as c_int),
            r_sd(rds, dptr, dict.len() as c_int),
            "{}: LZ4_setStreamDecode return",
            label
        );

        // One contiguous destination so linked blocks see their own history.
        let mut cout = vec![SENTINEL; total + 64];
        let mut rout = vec![SENTINEL; total + 64];
        let mut off = 0usize;
        for (i, (comp, blk)) in comps.iter().zip(blocks.iter()).enumerate() {
            let cap = (total + 64 - off) as c_int;
            let cn = c_dc(
                cds,
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr().add(off) as *mut c_char,
                comp.len() as c_int,
                cap,
            );
            let rn = r_dc(
                rds,
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr().add(off) as *mut c_char,
                comp.len() as c_int,
                cap,
            );
            assert_eq!(cn, rn, "{}: safe_continue block[{}] return", label, i);
            assert_eq!(
                cn,
                blk.len() as c_int,
                "{}: safe_continue block[{}] size",
                label,
                i
            );
            off += cn.max(0) as usize;
        }
        assert_bytes_eq(&format!("{} safe_continue output", label), &cout, &rout);
        // and it must reproduce the original concatenation
        let expect: Vec<u8> = blocks.iter().flat_map(|b| b.iter().copied()).collect();
        assert_eq!(&cout[..total], &expect[..], "{}: safe_continue content", label);
        assert_eq!(c_fd(cds), r_fd(rds), "{}: freeStreamDecode", label);

        // ---- fast_continue (deprecated, needs exact original sizes)
        let cds = c_cd();
        let rds = r_cd();
        assert_eq!(
            c_sd(cds, dptr, dict.len() as c_int),
            r_sd(rds, dptr, dict.len() as c_int),
            "{}: setStreamDecode (fast) return",
            label
        );
        let mut cout = vec![SENTINEL; total + 64];
        let mut rout = vec![SENTINEL; total + 64];
        let mut off = 0usize;
        for (i, (comp, blk)) in comps.iter().zip(blocks.iter()).enumerate() {
            let cn = c_df(
                cds,
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr().add(off) as *mut c_char,
                blk.len() as c_int,
            );
            let rn = r_df(
                rds,
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr().add(off) as *mut c_char,
                blk.len() as c_int,
            );
            assert_eq!(cn, rn, "{}: fast_continue block[{}] return", label, i);
            off += blk.len();
        }
        assert_bytes_eq(&format!("{} fast_continue output", label), &cout, &rout);
        assert_eq!(c_fd(cds), r_fd(rds), "{}: freeStreamDecode (fast)", label);
    }
}

// ===========================================================================
// LZ4_saveDict
// ===========================================================================

#[test]
fn save_dict_axis() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (c_sv, r_sv) = both::<FnSaveDict>("LZ4_saveDict");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    let mut rng2 = Rng::new(0x5A7E_D1C7);

    for shape in 0..N_SHAPES {
        for &dict_len in &[0usize, 100, 4096, 65536] {
            for &blk_len in &[1usize, 500, 20000, 70000] {
                let dict = gen_shape(&mut rng2, shape, dict_len);
                let blk = gen_shape(&mut rng2, shape, blk_len);
                for &maxdict in &[0usize, 1, 100, 4096, 65535, 65536, 70000] {
                    let label = format!(
                        "saveDict shape={} dict={} blk={} maxDict={}",
                        shape_name(shape),
                        dict_len,
                        blk_len,
                        maxdict
                    );
                    unsafe {
                        let cst = c_cs();
                        let rst = r_cs();
                        let dptr = if dict.is_empty() {
                            std::ptr::null()
                        } else {
                            dict.as_ptr() as *const c_char
                        };
                        assert_eq!(
                            c_ld(cst, dptr, dict_len as c_int),
                            r_ld(rst, dptr, dict_len as c_int),
                            "{}: loadDict",
                            label
                        );
                        let bound = cb(blk_len as c_int).max(1) as usize;
                        let mut cdst = vec![SENTINEL; bound];
                        let mut rdst = vec![SENTINEL; bound];
                        let cn = c_cc(
                            cst,
                            blk.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            blk_len as c_int,
                            bound as c_int,
                            1,
                        );
                        let rn = r_cc(
                            rst,
                            blk.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            blk_len as c_int,
                            bound as c_int,
                            1,
                        );
                        assert_eq!(cn, rn, "{}: compress return", label);
                        assert_bytes_eq(&format!("{} compressed", label), &cdst, &rdst);

                        // saveDict into a caller buffer; compare the return
                        // value AND the bytes it wrote.
                        let mut csafe = vec![SENTINEL; maxdict.max(1) + 16];
                        let mut rsafe = vec![SENTINEL; maxdict.max(1) + 16];
                        let cs = c_sv(
                            cst,
                            csafe.as_mut_ptr() as *mut c_char,
                            maxdict as c_int,
                        );
                        let rs = r_sv(
                            rst,
                            rsafe.as_mut_ptr() as *mut c_char,
                            maxdict as c_int,
                        );
                        assert_eq!(cs, rs, "{}: LZ4_saveDict return", label);
                        assert_bytes_eq(
                            &format!("{} saveDict buffer", label),
                            &csafe,
                            &rsafe,
                        );
                        assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// LZ4_compress_forceExtDict — the usingExtDict compression directive
// ===========================================================================

#[test]
fn compress_force_ext_dict() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_fe, r_fe) = both::<FnForceExtDictComp>("LZ4_compress_forceExtDict");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let (c_ud, r_ud) = both::<
        unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int,
    >("LZ4_decompress_safe_usingDict");

    let mut rng = Rng::new(0xFED1_C700);
    for shape in 0..N_SHAPES {
        for &dict_len in &[0usize, 64, 4096, 16384, 65536] {
            for &len in &[1usize, 13, 500, 5000, 65547, 90000] {
                let dict = gen_shape(&mut rng, shape, dict_len);
                let src = gen_shape(&mut rng, shape, len);
                let label = format!(
                    "forceExtDict shape={} dict={} len={}",
                    shape_name(shape),
                    dict_len,
                    len
                );
                unsafe {
                    let cst = c_cs();
                    let rst = r_cs();
                    let dptr = if dict.is_empty() {
                        std::ptr::null()
                    } else {
                        dict.as_ptr() as *const c_char
                    };
                    assert_eq!(
                        c_ld(cst, dptr, dict_len as c_int),
                        r_ld(rst, dptr, dict_len as c_int),
                        "{}: loadDict",
                        label
                    );
                    let bound = cb(len as c_int).max(1) as usize;
                    let mut cdst = vec![SENTINEL; bound];
                    let mut rdst = vec![SENTINEL; bound];
                    // NOTE: LZ4_compress_forceExtDict takes no dstCapacity —
                    // it assumes the destination is compressBound-sized.
                    let cn = c_fe(
                        cst,
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    );
                    let rn = r_fe(
                        rst,
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&label, &cdst, &rdst);

                    // Round-trip against the dictionary.
                    if cn > 0 {
                        let mut cout = vec![SENTINEL; len + 32];
                        let mut rout = vec![SENTINEL; len + 32];
                        let cd = c_ud(
                            cdst.as_ptr() as *const c_char,
                            cout.as_mut_ptr() as *mut c_char,
                            cn,
                            (len + 32) as c_int,
                            dptr,
                            dict_len as c_int,
                        );
                        let rd = r_ud(
                            rdst.as_ptr() as *const c_char,
                            rout.as_mut_ptr() as *mut c_char,
                            rn,
                            (len + 32) as c_int,
                            dptr,
                            dict_len as c_int,
                        );
                        assert_eq!(cd, rd, "{}: round-trip return", label);
                        assert_bytes_eq(&format!("{} round-trip", label), &cout, &rout);
                        assert_eq!(cd, len as c_int, "{}: round-trip size", label);
                        assert_eq!(&cout[..len], &src[..], "{}: round-trip content", label);
                    }
                    assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);
                }
            }
        }
    }
}

// ===========================================================================
// Explicit prefix64k / forceExtDict decode instantiations
// ===========================================================================

#[test]
fn decompress_prefix64k_and_force_ext_dict() {
    let (c_p64s, r_p64s) = both::<FnPrefix64kSafe>("LZ4_decompress_safe_withPrefix64k");
    let (c_p64f, r_p64f) = both::<FnPrefix64kFast>("LZ4_decompress_fast_withPrefix64k");
    let (c_fed, r_fed) = both::<FnSafeForceExtDict>("LZ4_decompress_safe_forceExtDict");
    let (c_pfed, r_pfed) =
        both::<FnPartialForceExtDict>("LZ4_decompress_safe_partial_forceExtDict");
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    const K64: usize = 64 * 1024;
    let mut rng = Rng::new(0x9BEF_1234);

    for shape in 0..N_SHAPES {
        for &len in &[1usize, 13, 500, 5000, 65547, 90000] {
            // --- withPrefix64k: the 64 KB of history must sit IMMEDIATELY
            // before `dest`, so allocate one buffer and decode into its tail.
            let prefix = gen_shape(&mut rng, shape, K64);
            let src = gen_shape(&mut rng, shape, len);
            let label = format!("prefix64k shape={} len={}", shape_name(shape), len);

            let comp = unsafe {
                let cst = c_cs();
                let rst = r_cs();
                assert_eq!(
                    c_ld(cst, prefix.as_ptr() as *const c_char, K64 as c_int),
                    r_ld(rst, prefix.as_ptr() as *const c_char, K64 as c_int),
                    "{}: loadDict",
                    label
                );
                let bound = cb(len as c_int).max(1) as usize;
                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_cc(
                    cst,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                    1,
                );
                let rn = r_cc(
                    rst,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                    1,
                );
                assert_eq!(cn, rn, "{}: compress return", label);
                assert_bytes_eq(&format!("{} compressed", label), &cdst, &rdst);
                assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);
                cdst.truncate(cn.max(0) as usize);
                cdst
            };

            unsafe {
                // buffer = [64 KB prefix][decode target]
                let mut cbuf = vec![SENTINEL; K64 + len + 32];
                let mut rbuf = vec![SENTINEL; K64 + len + 32];
                cbuf[..K64].copy_from_slice(&prefix);
                rbuf[..K64].copy_from_slice(&prefix);
                let cn = c_p64s(
                    comp.as_ptr() as *const c_char,
                    cbuf.as_mut_ptr().add(K64) as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                );
                let rn = r_p64s(
                    comp.as_ptr() as *const c_char,
                    rbuf.as_mut_ptr().add(K64) as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                );
                assert_eq!(cn, rn, "{}: safe_withPrefix64k return", label);
                assert_bytes_eq(&format!("{} safe_withPrefix64k", label), &cbuf, &rbuf);
                assert_eq!(cn, len as c_int, "{}: prefix64k size", label);
                assert_eq!(
                    &cbuf[K64..K64 + len],
                    &src[..],
                    "{}: prefix64k content",
                    label
                );

                // fast_withPrefix64k
                let mut cbuf = vec![SENTINEL; K64 + len + 32];
                let mut rbuf = vec![SENTINEL; K64 + len + 32];
                cbuf[..K64].copy_from_slice(&prefix);
                rbuf[..K64].copy_from_slice(&prefix);
                let cn = c_p64f(
                    comp.as_ptr() as *const c_char,
                    cbuf.as_mut_ptr().add(K64) as *mut c_char,
                    len as c_int,
                );
                let rn = r_p64f(
                    comp.as_ptr() as *const c_char,
                    rbuf.as_mut_ptr().add(K64) as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "{}: fast_withPrefix64k return", label);
                assert_bytes_eq(&format!("{} fast_withPrefix64k", label), &cbuf, &rbuf);

                // forceExtDict with the prefix supplied as a SEPARATE buffer
                let mut cout = vec![SENTINEL; len + 32];
                let mut rout = vec![SENTINEL; len + 32];
                let cn = c_fed(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                    prefix.as_ptr() as *const c_void,
                    K64,
                );
                let rn = r_fed(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                    prefix.as_ptr() as *const c_void,
                    K64,
                );
                assert_eq!(cn, rn, "{}: safe_forceExtDict return", label);
                assert_bytes_eq(&format!("{} safe_forceExtDict", label), &cout, &rout);

                // partial_forceExtDict across target sizes
                for &t in &[0usize, 1, len / 2, len, len + 7] {
                    let mut cout = vec![SENTINEL; len + 32];
                    let mut rout = vec![SENTINEL; len + 32];
                    let cn = c_pfed(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        comp.len() as c_int,
                        t as c_int,
                        (len + 32) as c_int,
                        prefix.as_ptr() as *const c_void,
                        K64,
                    );
                    let rn = r_pfed(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        comp.len() as c_int,
                        t as c_int,
                        (len + 32) as c_int,
                        prefix.as_ptr() as *const c_void,
                        K64,
                    );
                    let l = format!("{} partial_forceExtDict target={}", label, t);
                    assert_eq!(cn, rn, "{}: return", l);
                    assert_bytes_eq(&l, &cout, &rout);
                }
            }
        }
    }
}

// ===========================================================================
// Obsolete / deprecated wrappers — every one must be exercised
// ===========================================================================

#[test]
fn obsolete_compress_and_decompress_wrappers() {
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let ssz = stream_size();
    let (c_sos, r_sos) = both::<FnSizeofState>("LZ4_sizeofStreamState");
    assert_eq!(unsafe { c_sos() }, unsafe { r_sos() }, "LZ4_sizeofStreamState");

    let (c_c3, r_c3) = both::<FnComp3>("LZ4_compress");
    let (c_c4, r_c4) = both::<FnComp4>("LZ4_compress_limitedOutput");
    let (c_ws4, r_ws4) = both::<FnCompState4>("LZ4_compress_withState");
    let (c_ws5, r_ws5) = both::<FnCompState5>("LZ4_compress_limitedOutput_withState");
    let (c_cont4, r_cont4) = both::<FnCompContinueNoCap>("LZ4_compress_continue");
    let (c_cont5, r_cont5) =
        both::<FnCompContinueLegacy>("LZ4_compress_limitedOutput_continue");
    let (c_un, r_un) = both::<FnDecompFast>("LZ4_uncompress");
    let (c_uu, r_uu) = both::<FnDecompSafe>("LZ4_uncompress_unknownOutputSize");
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_cr, r_cr) = both::<FnCreate>("LZ4_create");
    let (c_sib, r_sib) = both::<FnSlideInput>("LZ4_slideInputBuffer");
    let (c_rss, r_rss) = both::<FnResetStreamState>("LZ4_resetStreamState");

    let mut rng = Rng::new(0x0B50_1E7E);

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 4, 13, 500, 5000, 65547, 90000] {
            let src = gen_shape(&mut rng, shape, len);
            let bound = unsafe { cb(len as c_int) }.max(1) as usize;
            let label = format!("obsolete shape={} len={}", shape_name(shape), len);

            unsafe {
                // ---- LZ4_compress (no dstCapacity; assumes compressBound)
                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_c3(
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                let rn = r_c3(
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "{}: LZ4_compress return", label);
                assert_bytes_eq(&format!("{} LZ4_compress", label), &cdst, &rdst);
                let comp = cdst[..cn.max(0) as usize].to_vec();

                // ---- LZ4_compress_limitedOutput across capacities
                for &cap in &[0usize, 1, bound / 2, bound] {
                    let mut cdst = vec![SENTINEL; cap.max(1)];
                    let mut rdst = vec![SENTINEL; cap.max(1)];
                    let cn = c_c4(
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    );
                    let rn = r_c4(
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    );
                    assert_eq!(
                        cn, rn,
                        "{}: LZ4_compress_limitedOutput(cap={}) return",
                        label, cap
                    );
                    assert_bytes_eq(
                        &format!("{} LZ4_compress_limitedOutput cap={}", label, cap),
                        &cdst,
                        &rdst,
                    );
                }

                // ---- *_withState variants
                let mut cstate = AlignedBuf::new(ssz, 64);
                let mut rstate = AlignedBuf::new(ssz, 64);
                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_ws4(
                    cstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                let rn = r_ws4(
                    rstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "{}: LZ4_compress_withState return", label);
                assert_bytes_eq(&format!("{} withState", label), &cdst, &rdst);
                assert_bytes_eq(
                    &format!("{} withState STATE", label),
                    cstate.as_slice(),
                    rstate.as_slice(),
                );

                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_ws5(
                    cstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                let rn = r_ws5(
                    rstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                assert_eq!(
                    cn, rn,
                    "{}: LZ4_compress_limitedOutput_withState return",
                    label
                );
                assert_bytes_eq(&format!("{} limitedOutput_withState", label), &cdst, &rdst);

                // ---- obsolete streaming continue wrappers
                let cst = c_cs();
                let rst = r_cs();
                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_cont4(
                    cst,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                let rn = r_cont4(
                    rst,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "{}: LZ4_compress_continue return", label);
                assert_bytes_eq(&format!("{} compress_continue", label), &cdst, &rdst);
                assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);

                let cst = c_cs();
                let rst = r_cs();
                let mut cdst = vec![SENTINEL; bound];
                let mut rdst = vec![SENTINEL; bound];
                let cn = c_cont5(
                    cst,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                let rn = r_cont5(
                    rst,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                assert_eq!(
                    cn, rn,
                    "{}: LZ4_compress_limitedOutput_continue return",
                    label
                );
                assert_bytes_eq(
                    &format!("{} limitedOutput_continue", label),
                    &cdst,
                    &rdst,
                );
                assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);

                // ---- obsolete decompressors
                let mut cout = vec![SENTINEL; len + 32];
                let mut rout = vec![SENTINEL; len + 32];
                let cn = c_un(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                let rn = r_un(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "{}: LZ4_uncompress return", label);
                assert_bytes_eq(&format!("{} LZ4_uncompress", label), &cout, &rout);

                let mut cout = vec![SENTINEL; len + 32];
                let mut rout = vec![SENTINEL; len + 32];
                let cn = c_uu(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                );
                let rn = r_uu(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 32) as c_int,
                );
                assert_eq!(
                    cn, rn,
                    "{}: LZ4_uncompress_unknownOutputSize return",
                    label
                );
                assert_bytes_eq(
                    &format!("{} LZ4_uncompress_unknownOutputSize", label),
                    &cout,
                    &rout,
                );
            }
        }
    }

    // ---- LZ4_create / LZ4_slideInputBuffer / LZ4_resetStreamState
    unsafe {
        let mut cin = vec![0u8; 1024];
        let mut rin = vec![0u8; 1024];
        let cst = c_cr(cin.as_mut_ptr() as *mut c_char);
        let rst = r_cr(rin.as_mut_ptr() as *mut c_char);
        assert!(!cst.is_null(), "C LZ4_create returned NULL");
        assert!(!rst.is_null(), "Rust LZ4_create returned NULL");
        assert_bytes_eq(
            "LZ4_create fresh state",
            std::slice::from_raw_parts(cst as *const u8, ssz),
            std::slice::from_raw_parts(rst as *const u8, ssz),
        );

        // LZ4_slideInputBuffer returns the stream's `dictionary` pointer. On a
        // fresh stream it is NULL in both; compare NULL-ness (the raw addresses
        // are necessarily different between the two libraries).
        let cp = c_sib(cst);
        let rp = r_sib(rst);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "LZ4_slideInputBuffer nullness on fresh stream"
        );

        // After loading a dictionary it must point AT the dictionary the caller
        // supplied, so the returned pointer is comparable to a known value.
        let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
        let dict = vec![7u8; 4096];
        c_ld(cst, dict.as_ptr() as *const c_char, dict.len() as c_int);
        r_ld(rst, dict.as_ptr() as *const c_char, dict.len() as c_int);
        let cp = c_sib(cst);
        let rp = r_sib(rst);
        assert_eq!(
            cp as usize, rp as usize,
            "LZ4_slideInputBuffer must return the same dictionary pointer"
        );
        assert_eq!(
            cp as usize,
            dict.as_ptr() as usize,
            "LZ4_slideInputBuffer should point into the caller's dictionary"
        );

        assert_eq!(
            c_rss(cst, std::ptr::null_mut()),
            r_rss(rst, std::ptr::null_mut()),
            "LZ4_resetStreamState return"
        );
        assert_bytes_eq(
            "LZ4_resetStreamState state",
            std::slice::from_raw_parts(cst as *const u8, ssz),
            std::slice::from_raw_parts(rst as *const u8, ssz),
        );
        assert_eq!(c_fs(cst), r_fs(rst), "freeStream after LZ4_create");
    }
}
