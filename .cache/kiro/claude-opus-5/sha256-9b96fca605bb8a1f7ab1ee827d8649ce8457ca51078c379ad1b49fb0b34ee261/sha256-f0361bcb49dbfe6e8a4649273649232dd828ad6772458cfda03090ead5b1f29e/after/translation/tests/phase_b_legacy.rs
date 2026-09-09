//! Phase B rows 63–70 and Phase C rows K1–K8: the legacy `ZSTDv01..ZSTDv07`
//! decoders and their `ZBUFFv0x` streaming front ends
//! (`ZSTD_LEGACY_SUPPORT=5` in `c_src/CMakeLists.txt`).
//!
//! There is no legacy ENCODER in this library, so these rows drive the legacy
//! decoders over their whole input space: correct magic numbers with
//! randomised bodies, every truncation of those inputs, pure garbage, and the
//! `*_decompressContinue` drivers with and without initialisation. Every C and
//! Rust return value, out-parameter and output byte is compared.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

/// Legacy magic numbers, from `c_src/src/legacy/zstd_v0*.h`.
const LEGACY_MAGIC: [(u32, &str); 7] = [
    (0xFD2FB51E, "v01"),
    (0xFD2FB522, "v02"),
    (0xFD2FB523, "v03"),
    (0xFD2FB524, "v04"),
    (0xFD2FB525, "v05"),
    (0xFD2FB526, "v06"),
    (0xFD2FB527, "v07"),
];

/// `ZSTD_frameSizeInfo` as returned by `ZSTDv0x_findFrameSizeInfoLegacy`
/// (the legacy signature writes through two out-pointers instead).
type FFrameSizeInfo =
    unsafe extern "C" fn(*const c_void, Sz, *mut Sz, *mut c_ulonglong);
#[allow(non_camel_case_types)]
type c_ulonglong = u64;

type FDecompress = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz) -> Sz;
type FDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
type FCreate = unsafe extern "C" fn() -> *mut c_void;
type FFree = unsafe extern "C" fn(*mut c_void) -> Sz;
type FReset = unsafe extern "C" fn(*mut c_void) -> Sz;
type FNext = unsafe extern "C" fn(*const c_void) -> Sz;
type FContinue = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;

/// Build the input corpus: valid magic + randomised body, every truncation,
/// and pure garbage.
fn legacy_inputs(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (magic, name) in LEGACY_MAGIC {
        for &blen in &[0usize, 1, 2, 3, 4, 5, 8, 13, 64, 512, 4096] {
            let mut v = magic.to_le_bytes().to_vec();
            v.extend((0..blen).map(|_| rng.byte()));
            out.push((format!("{name}-magic+rand{blen}"), v));
            // a body that looks like a plausible header: small values
            let mut v = magic.to_le_bytes().to_vec();
            v.extend((0..blen).map(|i| (i % 7) as u8));
            out.push((format!("{name}-magic+low{blen}"), v));
            // a body that is all 0xFF
            let mut v = magic.to_le_bytes().to_vec();
            v.extend(std::iter::repeat(0xFFu8).take(blen));
            out.push((format!("{name}-magic+ff{blen}"), v));
        }
    }
    // modern magic and garbage
    out.push(("modern-magic".into(), 0xFD2FB528u32.to_le_bytes().to_vec()));
    out.push(("zero4".into(), vec![0u8; 4]));
    out.push(("empty".into(), Vec::new()));
    for n in [1usize, 2, 3, 7, 16, 64, 1024] {
        out.push((format!("garbage{n}"), gen(Shape::Incompressible, n, rng)));
        out.push((format!("zeros{n}"), vec![0u8; n]));
    }
    out
}

/// Every truncation of every input, so the header parsers are driven at each
/// boundary.
fn truncations(v: &[u8]) -> Vec<usize> {
    let mut s: Vec<usize> = (0..=v.len().min(24)).collect();
    for extra in [v.len() / 2, v.len().saturating_sub(1), v.len()] {
        if !s.contains(&extra) {
            s.push(extra);
        }
    }
    s.sort_unstable();
    s.dedup();
    s
}

/// K1/K5 — `isError`, `getErrorName` and the magic-number accessors.
#[test]
fn k1_k5_legacy_error_and_magic() {
    let p = libs();
    for v in 1..=7 {
        let nm = format!("ZSTDv0{v}_isError");
        if p.has(&nm) {
            let (c, r) = p.sym::<FnIsError>(&nm);
            for code in 0..=130usize {
                let neg = 0usize.wrapping_sub(code);
                unsafe {
                    eq(&format!("{nm}({code})"), c(code), r(code));
                    eq(&format!("{nm}(-{code})"), c(neg), r(neg));
                }
            }
        }
        let nm = format!("ZSTDv0{v}_getErrorName");
        if p.has(&nm) {
            let (c, r) = p.sym::<FnGetErrName>(&nm);
            for code in 0..=130usize {
                let neg = 0usize.wrapping_sub(code);
                unsafe {
                    eq(&format!("{nm}(-{code})"), cstr(c(neg)), cstr(r(neg)));
                }
            }
        }
    }
    for v in 4..=7 {
        for suffix in ["sizeofDCtx", "estimateDCtxSize"] {
            let nm = format!("ZSTDv0{v}_{suffix}");
            if p.has(&nm) {
                let (c, r) = p.sym::<FnVoidSz>(&nm);
                unsafe { eq(&nm, c(), r()) };
            }
        }
    }
    for v in 1..=7 {
        for suffix in ["isError", "getErrorName", "recommendedDInSize", "recommendedDOutSize"] {
            let nm = format!("ZBUFFv0{v}_{suffix}");
            if !p.has(&nm) {
                continue;
            }
            if suffix.starts_with("recommended") {
                let (c, r) = p.sym::<FnVoidSz>(&nm);
                unsafe { eq(&nm, c(), r()) };
            } else if suffix == "isError" {
                let (c, r) = p.sym::<FnIsError>(&nm);
                for code in 0..=130usize {
                    let neg = 0usize.wrapping_sub(code);
                    unsafe { eq(&format!("{nm}(-{code})"), c(neg), r(neg)) };
                }
            } else {
                let (c, r) = p.sym::<FnGetErrName>(&nm);
                for code in 0..=130usize {
                    let neg = 0usize.wrapping_sub(code);
                    unsafe { eq(&format!("{nm}(-{code})"), cstr(c(neg)), cstr(r(neg))) };
                }
            }
        }
    }
}

/// Rows 63–69 / K2/K3/K4/K6 — one-shot `decompress`, `decompressDCtx`,
/// `findFrameSizeInfoLegacy` and `getFrameParams` for every legacy version,
/// over the whole input corpus and all truncations.
#[test]
fn row63_row69_legacy_oneshot() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x63);
    let inputs = legacy_inputs(&mut rng);

    for v in 1..=7 {
        let dnm = format!("ZSTDv0{v}_decompress");
        let (c_de, r_de) = p.sym::<FDecompress>(&dnm);
        let ienm = format!("ZSTDv0{v}_isError");
        let has_ie = p.has(&ienm);
        let fsnm = format!("ZSTDv0{v}_findFrameSizeInfoLegacy");
        let (c_fs, r_fs) = p.sym::<FFrameSizeInfo>(&fsnm);

        unsafe {
            for (name, input) in &inputs {
                for cut in truncations(input) {
                    let ip = if input.is_empty() {
                        std::ptr::null()
                    } else {
                        input.as_ptr() as *const c_void
                    };
                    let tag = format!("{dnm} {name} cut={cut}");

                    // K4/K6 — frame size info first, it must agree exactly
                    let (mut ccs, mut rcs) = (0usize, 0usize);
                    let (mut cdb, mut rdb) = (0u64, 0u64);
                    c_fs(ip, cut, &mut ccs, &mut cdb);
                    r_fs(ip, cut, &mut rcs, &mut rdb);
                    eq(&format!("{tag}: cSize out"), ccs, rcs);
                    eq(&format!("{tag}: dBound out"), cdb, rdb);

                    // K2/K3 — one-shot decompression into several capacities
                    for &cap in &[0usize, 1, 64, 4096, 1 << 17] {
                        let mut cb = vec![0xC3u8; cap.max(1)];
                        let mut rb = vec![0xC3u8; cap.max(1)];
                        let cn = c_de(cb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                        let rn = r_de(rb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                        eq(&format!("{tag}: decompress(cap={cap}) ret"), cn, rn);
                        let is_err = if has_ie {
                            let (c_ie, _) = p.sym::<FnIsError>(&ienm);
                            c_ie(cn) != 0
                        } else {
                            cn > usize::MAX - 256
                        };
                        if !is_err && cn <= cap {
                            eq_bytes(&format!("{tag}: decompress(cap={cap}) out"), &cb[..cn], &rb[..rn]);
                        }
                        // whatever was written, the two buffers must match
                        eq_bytes(&format!("{tag}: decompress(cap={cap}) buffer image"), &cb, &rb);
                    }

                    // the DCtx variants, where they exist
                    let ddnm = format!("ZSTDv0{v}_decompressDCtx");
                    if p.has(&ddnm) {
                        let (c_c, r_c) = p.sym::<FCreate>(&format!("ZSTDv0{v}_createDCtx"));
                        let (c_f, r_f) = p.sym::<FFree>(&format!("ZSTDv0{v}_freeDCtx"));
                        let (c_dd, r_dd) = p.sym::<FDecompressDCtx>(&ddnm);
                        let cc = c_c();
                        let rc = r_c();
                        if !cc.is_null() && !rc.is_null() {
                            for &cap in &[0usize, 64, 1 << 17] {
                                let mut cb = vec![0xC3u8; cap.max(1)];
                                let mut rb = vec![0xC3u8; cap.max(1)];
                                let cn = c_dd(cc, cb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                                let rn = r_dd(rc, rb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                                eq(&format!("{tag}: decompressDCtx(cap={cap}) ret"), cn, rn);
                                eq_bytes(&format!("{tag}: decompressDCtx buffer image"), &cb, &rb);
                            }
                        }
                        c_f(cc);
                        r_f(rc);
                    }
                }
            }
        }
    }
}

/// K6 — `ZSTDv0{5,6,7}_getFrameParams`.
#[test]
fn k6_legacy_get_frame_params() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x66);
    let inputs = legacy_inputs(&mut rng);

    /// `ZSTDv05_parameters` / `ZSTDv06_frameParams` / `ZSTDv07_frameParams`
    /// are all read only through the out-pointer, so a generously sized,
    /// canary-filled scratch struct is compared byte-for-byte.
    const PBYTES: usize = 128;
    type FGetParams = unsafe extern "C" fn(*mut u8, *const c_void, Sz) -> Sz;

    for v in 5..=7 {
        let nm = format!("ZSTDv0{v}_getFrameParams");
        if !p.has(&nm) {
            continue;
        }
        let (c, r) = p.sym::<FGetParams>(&nm);
        unsafe {
            for (name, input) in &inputs {
                for cut in truncations(input) {
                    let ip = if input.is_empty() {
                        std::ptr::null()
                    } else {
                        input.as_ptr() as *const c_void
                    };
                    let mut cp = [0xA9u8; PBYTES];
                    let mut rp = [0xA9u8; PBYTES];
                    let a = c(cp.as_mut_ptr(), ip, cut);
                    let b = r(rp.as_mut_ptr(), ip, cut);
                    eq(&format!("{nm} {name} cut={cut} ret"), a, b);
                    eq_bytes(&format!("{nm} {name} cut={cut} params image"), &cp, &rp);
                }
            }
        }
    }
}

/// K8 / rows 63–69 — the `*_decompressContinue` drivers, both without any
/// initialisation and after `reset`/`decompressBegin`, fed
/// `nextSrcSizeToDecompress` bytes at a time.
#[test]
fn k8_legacy_continue_drivers() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x68);
    let inputs = legacy_inputs(&mut rng);

    for v in 1..=7 {
        let cnm = format!("ZSTDv0{v}_createDCtx");
        let fnm = format!("ZSTDv0{v}_freeDCtx");
        let nnm = format!("ZSTDv0{v}_nextSrcSizeToDecompress");
        let conm = format!("ZSTDv0{v}_decompressContinue");
        if !(p.has(&cnm) && p.has(&fnm) && p.has(&nnm) && p.has(&conm)) {
            continue;
        }
        let (c_c, r_c) = p.sym::<FCreate>(&cnm);
        let (c_f, r_f) = p.sym::<FFree>(&fnm);
        let (c_n, r_n) = p.sym::<FNext>(&nnm);
        let (c_co, r_co) = p.sym::<FContinue>(&conm);
        let rsnm = format!("ZSTDv0{v}_resetDCtx");
        let bgnm = format!("ZSTDv0{v}_decompressBegin");

        unsafe {
            for (name, input) in &inputs {
                for init_mode in 0..3 {
                    let cc = c_c();
                    let rc = r_c();
                    if cc.is_null() || rc.is_null() {
                        c_f(cc);
                        r_f(rc);
                        continue;
                    }
                    match init_mode {
                        // K8 — no initialisation at all
                        0 => {}
                        1 if p.has(&rsnm) => {
                            let (c_rs, r_rs) = p.sym::<FReset>(&rsnm);
                            eq(&format!("{rsnm} ret"), c_rs(cc), r_rs(rc));
                        }
                        2 if p.has(&bgnm) => {
                            let (c_bg, r_bg) = p.sym::<FReset>(&bgnm);
                            eq(&format!("{bgnm} ret"), c_bg(cc), r_bg(rc));
                        }
                        _ => {
                            c_f(cc);
                            r_f(rc);
                            continue;
                        }
                    }
                    let tag = format!("{conm} {name} init={init_mode}");
                    let mut off = 0usize;
                    let mut cout = vec![0xD7u8; 1 << 17];
                    let mut rout = vec![0xD7u8; 1 << 17];
                    let mut cpos = 0usize;
                    let mut rpos = 0usize;
                    // bounded number of steps: garbage input must not spin
                    for _step in 0..64 {
                        let cn = c_n(cc);
                        let rn = r_n(rc);
                        eq(&format!("{tag}: nextSrcSizeToDecompress@{off}"), cn, rn);
                        if cn == 0 || cn > input.len() {
                            break;
                        }
                        if off + cn > input.len() {
                            break;
                        }
                        if cpos + (1 << 12) > cout.len() {
                            break;
                        }
                        let cr = c_co(
                            cc,
                            cout[cpos..].as_mut_ptr() as *mut c_void,
                            cout.len() - cpos,
                            input[off..].as_ptr() as *const c_void,
                            cn,
                        );
                        let rr = r_co(
                            rc,
                            rout[rpos..].as_mut_ptr() as *mut c_void,
                            rout.len() - rpos,
                            input[off..].as_ptr() as *const c_void,
                            cn,
                        );
                        eq(&format!("{tag}: decompressContinue@{off} ret"), cr, rr);
                        // error sentinel? then stop, but the images must match
                        if cr > usize::MAX - 256 {
                            break;
                        }
                        eq_bytes(
                            &format!("{tag}: decompressContinue@{off} out"),
                            &cout[cpos..cpos + cr],
                            &rout[rpos..rpos + rr],
                        );
                        cpos += cr;
                        rpos += rr;
                        off += cn;
                    }
                    eq_bytes(&format!("{tag}: whole output"), &cout[..cpos], &rout[..rpos]);
                    eq(&format!("{fnm} ret"), c_f(cc), r_f(rc));
                }
            }
        }
    }
}

/// Row 70 / K7 — legacy frames fed to the MODERN entry points, which route
/// through `zstd_legacy.h` (`ZSTD_LEGACY_SUPPORT=5`).
#[test]
fn row70_k7_legacy_via_modern_api() {
    let p = libs();
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FU64 = unsafe extern "C" fn(*const c_void, Sz) -> u64;
    type FSZ = unsafe extern "C" fn(*const c_void, Sz) -> Sz;
    type FU = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    let (c_fcs, r_fcs) = p.sym::<FU64>("ZSTD_getFrameContentSize");
    let (c_fds, r_fds) = p.sym::<FU64>("ZSTD_findDecompressedSize");
    let (c_db, r_db) = p.sym::<FU64>("ZSTD_decompressBound");
    let (c_ffcs, r_ffcs) = p.sym::<FSZ>("ZSTD_findFrameCompressedSize");
    let (c_isf, r_isf) = p.sym::<FU>("ZSTD_isFrame");
    let (c_gds, r_gds) = p.sym::<FU64>("ZSTD_getDecompressedSize");

    let mut rng = Rng::new(SEED ^ 0x70);
    let inputs = legacy_inputs(&mut rng);
    unsafe {
        for (name, input) in &inputs {
            for cut in truncations(input) {
                let ip = if input.is_empty() {
                    std::ptr::null()
                } else {
                    input.as_ptr() as *const c_void
                };
                let tag = format!("modern-on-legacy {name} cut={cut}");
                eq(&format!("{tag}: isFrame"), c_isf(ip, cut), r_isf(ip, cut));
                eq(&format!("{tag}: getFrameContentSize"), c_fcs(ip, cut), r_fcs(ip, cut));
                eq(&format!("{tag}: getDecompressedSize"), c_gds(ip, cut), r_gds(ip, cut));
                eq(&format!("{tag}: findDecompressedSize"), c_fds(ip, cut), r_fds(ip, cut));
                eq(&format!("{tag}: decompressBound"), c_db(ip, cut), r_db(ip, cut));
                eq(&format!("{tag}: findFrameCompressedSize"), c_ffcs(ip, cut), r_ffcs(ip, cut));
                for &cap in &[0usize, 64, 1 << 17] {
                    let mut cb = vec![0xE1u8; cap.max(1)];
                    let mut rb = vec![0xE1u8; cap.max(1)];
                    let cn = c_de(cb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                    let rn = r_de(rb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                    eq(&format!("{tag}: ZSTD_decompress(cap={cap}) ret"), cn, rn);
                    if c_ie(cn) == 0 && cn <= cap {
                        eq_bytes(&format!("{tag}: decoded"), &cb[..cn], &rb[..rn]);
                    }
                    eq_bytes(&format!("{tag}: buffer image"), &cb, &rb);
                }
            }
        }
    }
}

/// Rows 66–69 — the `ZBUFFv0{4,5,6,7}_*` streaming decoders.
#[test]
fn row66_row69_zbuff_legacy() {
    let p = libs();
    type FNew = unsafe extern "C" fn() -> *mut c_void;
    type FFreeZ = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FInit = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FCont =
        unsafe extern "C" fn(*mut c_void, *mut c_void, *mut Sz, *const c_void, *mut Sz) -> Sz;

    let mut rng = Rng::new(SEED ^ 0x69);
    let inputs = legacy_inputs(&mut rng);
    let dict = gen(Shape::TextLike, 1024, &mut rng);

    for v in 4..=7 {
        let cnm = format!("ZBUFFv0{v}_createDCtx");
        let fnm = format!("ZBUFFv0{v}_freeDCtx");
        let inm = format!("ZBUFFv0{v}_decompressInit");
        let conm = format!("ZBUFFv0{v}_decompressContinue");
        let ienm = format!("ZBUFFv0{v}_isError");
        if !(p.has(&cnm) && p.has(&fnm) && p.has(&inm) && p.has(&conm)) {
            continue;
        }
        let (c_c, r_c) = p.sym::<FNew>(&cnm);
        let (c_f, r_f) = p.sym::<FFreeZ>(&fnm);
        let (c_i, r_i) = p.sym::<FInit>(&inm);
        let (c_co, r_co) = p.sym::<FCont>(&conm);
        let (c_ie, _) = p.sym::<FnIsError>(&ienm);
        let dinm = format!("ZBUFFv0{v}_decompressInitDictionary");
        let (c_din, r_din) = p.sym::<FnVoidSz>("ZBUFF_recommendedDInSize");
        let (c_dout, r_dout) = p.sym::<FnVoidSz>("ZBUFF_recommendedDOutSize");

        unsafe {
            eq("ZBUFF_recommendedDInSize", c_din(), r_din());
            eq("ZBUFF_recommendedDOutSize", c_dout(), r_dout());
            for (name, input) in &inputs {
                for use_dict in [false, true] {
                    if use_dict && !p.has(&dinm) {
                        continue;
                    }
                    for in_chunk in [1usize, 7, 4096, usize::MAX] {
                        let cc = c_c();
                        let rc = r_c();
                        if cc.is_null() || rc.is_null() {
                            c_f(cc);
                            r_f(rc);
                            continue;
                        }
                        let (a, b) = if use_dict {
                            let (c_d, r_d) = p.sym::<FInitDict>(&dinm);
                            (
                                c_d(cc, dict.as_ptr() as *const c_void, dict.len()),
                                r_d(rc, dict.as_ptr() as *const c_void, dict.len()),
                            )
                        } else {
                            (c_i(cc), r_i(rc))
                        };
                        eq(&format!("{inm} {name} dict={use_dict} ret"), a, b);
                        if c_ie(a) != 0 {
                            c_f(cc);
                            r_f(rc);
                            continue;
                        }
                        let tag = format!("{conm} {name} dict={use_dict} chunk={in_chunk}");
                        let mut cout = vec![0xB4u8; 1 << 16];
                        let mut rout = vec![0xB4u8; 1 << 16];
                        let mut cacc: Vec<u8> = Vec::new();
                        let mut racc: Vec<u8> = Vec::new();
                        let mut off = 0usize;
                        let mut steps = 0;
                        while off < input.len() && steps < 512 {
                            steps += 1;
                            let n = in_chunk.min(input.len() - off);
                            let mut cdst = cout.len();
                            let mut rdst = rout.len();
                            let mut csrc = n;
                            let mut rsrc = n;
                            let cr = c_co(
                                cc,
                                cout.as_mut_ptr() as *mut c_void,
                                &mut cdst,
                                input[off..].as_ptr() as *const c_void,
                                &mut csrc,
                            );
                            let rr = r_co(
                                rc,
                                rout.as_mut_ptr() as *mut c_void,
                                &mut rdst,
                                input[off..].as_ptr() as *const c_void,
                                &mut rsrc,
                            );
                            eq(&format!("{tag}: ret@{off}"), cr, rr);
                            eq(&format!("{tag}: dstPos@{off}"), cdst, rdst);
                            eq(&format!("{tag}: srcPos@{off}"), csrc, rsrc);
                            eq_bytes(&format!("{tag}: out@{off}"), &cout[..cdst], &rout[..rdst]);
                            cacc.extend_from_slice(&cout[..cdst]);
                            racc.extend_from_slice(&rout[..rdst]);
                            if c_ie(cr) != 0 {
                                break;
                            }
                            if csrc == 0 && cdst == 0 {
                                break;
                            }
                            off += csrc;
                        }
                        eq_bytes(&format!("{tag}: whole output"), &cacc, &racc);
                        eq(&format!("{fnm} ret"), c_f(cc), r_f(rc));
                    }
                }
            }
        }
    }
}

/// Row 69 — the `ZSTDv07`-only extras: `createDDict`, `decompress_usingDDict`,
/// `decompress_usingDict`, `insertBlock`, `isSkipFrame`, `getDecompressedSize`,
/// `copyDCtx`, `decompressBlock`, and the same for v05/v06 where present.
#[test]
fn row67_row69_legacy_dict_and_block_apis() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x67);
    let inputs = legacy_inputs(&mut rng);
    let dict = gen(Shape::TextLike, 2048, &mut rng);

    type FCreate = unsafe extern "C" fn() -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FUsingDict = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz,
    ) -> Sz;
    type FBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FBlock = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
    type FU64 = unsafe extern "C" fn(*const c_void, Sz) -> u64;
    type FU = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FCreateDDict = unsafe extern "C" fn(*const c_void, Sz) -> *mut c_void;
    type FUsingDDict =
        unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void) -> Sz;
    type FIns = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;

    for v in 5..=7 {
        let cnm = format!("ZSTDv0{v}_createDCtx");
        let fnm = format!("ZSTDv0{v}_freeDCtx");
        if !(p.has(&cnm) && p.has(&fnm)) {
            continue;
        }
        let (c_c, r_c) = p.sym::<FCreate>(&cnm);
        let (c_f, r_f) = p.sym::<FFree>(&fnm);
        let udnm = format!("ZSTDv0{v}_decompress_usingDict");
        let bdnm = format!("ZSTDv0{v}_decompressBegin_usingDict");
        let blnm = format!("ZSTDv0{v}_decompressBlock");
        let cpnm = format!("ZSTDv0{v}_copyDCtx");

        unsafe {
            for (name, input) in &inputs {
                for cut in truncations(input) {
                    let ip = if input.is_empty() {
                        std::ptr::null()
                    } else {
                        input.as_ptr() as *const c_void
                    };
                    let tag = format!("v0{v} {name} cut={cut}");
                    if p.has(&udnm) {
                        let (c_ud, r_ud) = p.sym::<FUsingDict>(&udnm);
                        let cc = c_c();
                        let rc = r_c();
                        for &cap in &[0usize, 64, 1 << 16] {
                            let mut cb = vec![0x7Eu8; cap.max(1)];
                            let mut rb = vec![0x7Eu8; cap.max(1)];
                            let cn = c_ud(cc, cb.as_mut_ptr() as *mut c_void, cap, ip, cut, dict.as_ptr() as *const c_void, dict.len());
                            let rn = r_ud(rc, rb.as_mut_ptr() as *mut c_void, cap, ip, cut, dict.as_ptr() as *const c_void, dict.len());
                            eq(&format!("{tag}: {udnm}(cap={cap}) ret"), cn, rn);
                            eq_bytes(&format!("{tag}: {udnm} buffer image"), &cb, &rb);
                        }
                        c_f(cc);
                        r_f(rc);
                    }
                    if p.has(&bdnm) && p.has(&blnm) {
                        let (c_bd, r_bd) = p.sym::<FBeginDict>(&bdnm);
                        let (c_bl, r_bl) = p.sym::<FBlock>(&blnm);
                        let cc = c_c();
                        let rc = r_c();
                        eq(
                            &format!("{tag}: {bdnm} ret"),
                            c_bd(cc, dict.as_ptr() as *const c_void, dict.len()),
                            r_bd(rc, dict.as_ptr() as *const c_void, dict.len()),
                        );
                        for &cap in &[0usize, 64, 1 << 16] {
                            let mut cb = vec![0x7Fu8; cap.max(1)];
                            let mut rb = vec![0x7Fu8; cap.max(1)];
                            let cn = c_bl(cc, cb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                            let rn = r_bl(rc, rb.as_mut_ptr() as *mut c_void, cap, ip, cut);
                            eq(&format!("{tag}: {blnm}(cap={cap}) ret"), cn, rn);
                            eq_bytes(&format!("{tag}: {blnm} buffer image"), &cb, &rb);
                        }
                        if p.has(&cpnm) {
                            let (c_cp, r_cp) = p.sym::<FCopy>(&cpnm);
                            let cc2 = c_c();
                            let rc2 = r_c();
                            c_cp(cc2, cc);
                            r_cp(rc2, rc);
                            let (c_n, r_n) =
                                p.sym::<FNext>(&format!("ZSTDv0{v}_nextSrcSizeToDecompress"));
                            eq(&format!("{tag}: after {cpnm} next"), c_n(cc2), r_n(rc2));
                            c_f(cc2);
                            r_f(rc2);
                        }
                        c_f(cc);
                        r_f(rc);
                    }
                }
            }
        }
    }

    // v07-only extras
    unsafe {
        // int ZSTDv07_isSkipFrame(ZSTDv07_DCtx* dctx) -- takes a CONTEXT, not a
        // buffer (zstd_v07.c:2901). Drive it on a context that has just been
        // fed each input via decompressBegin + decompressContinue.
        if p.has("ZSTDv07_isSkipFrame") {
            type FSkip = unsafe extern "C" fn(*const c_void) -> c_int;
            let (c_sk, r_sk) = p.sym::<FSkip>("ZSTDv07_isSkipFrame");
            let (c_c, r_c) = p.sym::<FCreate>("ZSTDv07_createDCtx");
            let (c_f, r_f) = p.sym::<FFree>("ZSTDv07_freeDCtx");
            let (c_bg, r_bg) = p.sym::<unsafe extern "C" fn(*mut c_void) -> Sz>("ZSTDv07_decompressBegin");
            let (c_n, r_n) = p.sym::<FNext>("ZSTDv07_nextSrcSizeToDecompress");
            let (c_co, r_co) =
                p.sym::<unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz>(
                    "ZSTDv07_decompressContinue",
                );
            for (name, input) in &inputs {
                let cc = c_c();
                let rc = r_c();
                if cc.is_null() || rc.is_null() {
                    c_f(cc);
                    r_f(rc);
                    continue;
                }
                eq(&format!("ZSTDv07_decompressBegin {name}"), c_bg(cc), r_bg(rc));
                eq(&format!("ZSTDv07_isSkipFrame {name} fresh"), c_sk(cc), r_sk(rc));
                let mut cout = vec![0u8; 1 << 16];
                let mut rout = vec![0u8; 1 << 16];
                let mut off = 0usize;
                for _ in 0..8 {
                    let n = c_n(cc);
                    eq(&format!("ZSTDv07 next {name}@{off}"), n, r_n(rc));
                    if n == 0 || n > input.len() || off + n > input.len() {
                        break;
                    }
                    let cr = c_co(cc, cout.as_mut_ptr() as *mut c_void, cout.len(),
                                  input[off..].as_ptr() as *const c_void, n);
                    let rr = r_co(rc, rout.as_mut_ptr() as *mut c_void, rout.len(),
                                  input[off..].as_ptr() as *const c_void, n);
                    eq(&format!("ZSTDv07 continue {name}@{off}"), cr, rr);
                    eq(&format!("ZSTDv07_isSkipFrame {name}@{off}"), c_sk(cc), r_sk(rc));
                    if cr > usize::MAX - 256 {
                        break;
                    }
                    off += n;
                }
                c_f(cc);
                r_f(rc);
            }
        }
        if p.has("ZSTDv07_getDecompressedSize") {
            let (c, r) = p.sym::<FU64>("ZSTDv07_getDecompressedSize");
            for (name, input) in &inputs {
                for cut in truncations(input) {
                    let ip = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                    eq(
                        &format!("ZSTDv07_getDecompressedSize {name} cut={cut}"),
                        c(ip, cut),
                        r(ip, cut),
                    );
                }
            }
        }
        if p.has("ZSTDv07_createDDict") && p.has("ZSTDv07_decompress_usingDDict") {
            let (c_cd, r_cd) = p.sym::<FCreateDDict>("ZSTDv07_createDDict");
            let (c_fd, r_fd) = p.sym::<FFree>("ZSTDv07_freeDDict");
            let (c_ud, r_ud) = p.sym::<FUsingDDict>("ZSTDv07_decompress_usingDDict");
            let (c_c, r_c) = p.sym::<FCreate>("ZSTDv07_createDCtx");
            let (c_f, r_f) = p.sym::<FFree>("ZSTDv07_freeDCtx");
            for d in [Vec::new(), dict.clone()] {
                let dp = if d.is_empty() { std::ptr::null() } else { d.as_ptr() as *const c_void };
                let cdd = c_cd(dp, d.len());
                let rdd = r_cd(dp, d.len());
                eq("ZSTDv07_createDDict nullness", cdd.is_null(), rdd.is_null());
                if cdd.is_null() {
                    continue;
                }
                let cc = c_c();
                let rc = r_c();
                for (name, input) in &inputs {
                    for cut in truncations(input) {
                        let ip = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                        for &cap in &[0usize, 64, 1 << 16] {
                            let mut cb = vec![0x6Du8; cap.max(1)];
                            let mut rb = vec![0x6Du8; cap.max(1)];
                            let cn = c_ud(cc, cb.as_mut_ptr() as *mut c_void, cap, ip, cut, cdd);
                            let rn = r_ud(rc, rb.as_mut_ptr() as *mut c_void, cap, ip, cut, rdd);
                            eq(
                                &format!("ZSTDv07_decompress_usingDDict {name} cut={cut} cap={cap} ret"),
                                cn, rn,
                            );
                            eq_bytes("ZSTDv07_decompress_usingDDict buffer image", &cb, &rb);
                        }
                    }
                }
                c_f(cc);
                r_f(rc);
                eq("ZSTDv07_freeDDict", c_fd(cdd), r_fd(rdd));
            }
        }
    }
}
