//! Phase C — error-path differential tests for lz4frame.c.
//! Each test names the `ERRORS.md` row(s) it covers.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

type FnCompressFrame =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnBoundP = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnBeginDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnGetFrameInfo =
    unsafe extern "C" fn(*mut c_void, *mut LZ4F_frameInfo_t, *const c_void, *mut usize) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnReset = unsafe extern "C" fn(*mut c_void);
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
type FnFrameUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnBeginCDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, *const LZ4F_preferences_t) -> usize;

struct Api {
    compress_frame: FnCompressFrame,
    frame_bound: FnBoundP,
    bound: FnBoundP,
    create_cctx: FnCreateCtx,
    free_cctx: FnFreeCtx,
    begin: FnBegin,
    begin_dict: FnBeginDict,
    begin_dict_once: FnBeginDict,
    begin_cdict: FnBeginCDict,
    update: FnUpdate,
    uncompressed_update: FnUpdate,
    flush: FnFlush,
    end: FnFlush,
    create_dctx: FnCreateCtx,
    free_dctx: FnFreeCtx,
    header_size: FnHeaderSize,
    get_frame_info: FnGetFrameInfo,
    decompress: FnDecompress,
    decompress_using_dict: FnDecompressUsingDict,
    reset_dctx: FnReset,
    get_block_size: FnGetBlockSize,
    is_error: FnIsError,
    get_error_code: FnGetErrorCode,
    get_error_name: FnGetErrorName,
    create_cdict: FnCreateCDict,
    free_cdict: FnFreeCDict,
    frame_using_cdict: FnFrameUsingCDict,
}

fn bind(l: &Lib) -> Api {
    Api {
        compress_frame: l.sym("LZ4F_compressFrame"),
        frame_bound: l.sym("LZ4F_compressFrameBound"),
        bound: l.sym("LZ4F_compressBound"),
        create_cctx: l.sym("LZ4F_createCompressionContext"),
        free_cctx: l.sym("LZ4F_freeCompressionContext"),
        begin: l.sym("LZ4F_compressBegin"),
        begin_dict: l.sym("LZ4F_compressBegin_usingDict"),
        begin_dict_once: l.sym("LZ4F_compressBegin_usingDictOnce"),
        begin_cdict: l.sym("LZ4F_compressBegin_usingCDict"),
        update: l.sym("LZ4F_compressUpdate"),
        uncompressed_update: l.sym("LZ4F_uncompressedUpdate"),
        flush: l.sym("LZ4F_flush"),
        end: l.sym("LZ4F_compressEnd"),
        create_dctx: l.sym("LZ4F_createDecompressionContext"),
        free_dctx: l.sym("LZ4F_freeDecompressionContext"),
        header_size: l.sym("LZ4F_headerSize"),
        get_frame_info: l.sym("LZ4F_getFrameInfo"),
        decompress: l.sym("LZ4F_decompress"),
        decompress_using_dict: l.sym("LZ4F_decompress_usingDict"),
        reset_dctx: l.sym("LZ4F_resetDecompressionContext"),
        get_block_size: l.sym("LZ4F_getBlockSize"),
        is_error: l.sym("LZ4F_isError"),
        get_error_code: l.sym("LZ4F_getErrorCode"),
        get_error_name: l.sym("LZ4F_getErrorName"),
        create_cdict: l.sym("LZ4F_createCDict"),
        free_cdict: l.sym("LZ4F_freeCDict"),
        frame_using_cdict: l.sym("LZ4F_compressFrame_usingCDict"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

/// Compare an LZ4F return value from both libraries: the raw value, the
/// `LZ4F_isError` verdict, the `LZ4F_getErrorCode` enum and the error string.
macro_rules! same_lz4f {
    ($c:expr, $r:expr, $a:expr, $b:expr, $($m:tt)*) => {{
        let a = $a;
        let b = $b;
        assert_eq!(
            a, b,
            "{}: raw {} vs {}",
            format!($($m)*), fmt_lz4f(a), fmt_lz4f(b)
        );
        unsafe {
            assert_eq!(($c.is_error)(a), ($r.is_error)(b), "{}: isError", format!($($m)*));
            assert_eq!(
                ($c.get_error_code)(a),
                ($r.get_error_code)(b),
                "{}: getErrorCode",
                format!($($m)*)
            );
            assert_eq!(
                cstr(($c.get_error_name)(a)),
                cstr(($r.get_error_name)(b)),
                "{}: getErrorName",
                format!($($m)*)
            );
        }
        a
    }};
}

// ---------------------------------------------------------------------------
// frame header construction helpers
// ---------------------------------------------------------------------------

const MAGIC: u32 = 0x184D2204;

/// Build a syntactically explicit frame header so individual bits can be broken.
fn make_header(magic: u32, flg: u8, bd: u8, content_size: Option<u64>, dict_id: Option<u32>, good_hc: bool) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&magic.to_le_bytes());
    v.push(flg);
    v.push(bd);
    if let Some(cs) = content_size {
        v.extend_from_slice(&cs.to_le_bytes());
    }
    if let Some(d) = dict_id {
        v.extend_from_slice(&d.to_le_bytes());
    }
    // header checksum over everything after the magic
    let hc = xxh32_hc(&v[4..]);
    v.push(if good_hc { hc } else { hc ^ 0xFF });
    v
}

/// (XXH32(desc, len, 0) >> 8) & 0xFF, computed with the C library itself.
fn xxh32_hc(desc: &[u8]) -> u8 {
    type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> u32;
    let f: FnXxh32 = libs().c.sym("LZ4_XXH32");
    let h = unsafe { f(desc.as_ptr() as *const c_void, desc.len(), 0) };
    ((h >> 8) & 0xFF) as u8
}

unsafe fn feed_all(api: &Api, dctx: *mut c_void, frame: &[u8]) -> (usize, Vec<u8>) {
    let mut out = Vec::new();
    let mut sp = 0usize;
    let mut buf = vec![0u8; 1 << 18];
    let mut last = 1usize;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 1_000_000 {
            panic!("stalled");
        }
        let mut ssz = frame.len() - sp;
        let mut dsz = buf.len();
        last = (api.decompress)(
            dctx,
            buf.as_mut_ptr() as *mut c_void,
            &mut dsz,
            frame.as_ptr().add(sp) as *const c_void,
            &mut ssz,
            std::ptr::null(),
        );
        if (api.is_error)(last) != 0 {
            return (last, out);
        }
        out.extend_from_slice(&buf[..dsz]);
        sp += ssz;
        if last == 0 {
            return (0, out);
        }
        if ssz == 0 && dsz == 0 {
            return (last, out);
        }
    }
}

/// ERRORS rows 1, 2, 6: LZ4F_getBlockSize over the full int range of the enum.
#[test]
fn errf_get_block_size() {
    let (c, r) = pair();
    for id in -8i32..=16 {
        same_lz4f!(c, r, unsafe { (c.get_block_size)(id) }, unsafe { (r.get_block_size)(id) }, "getBlockSize({})", id);
    }
    for id in [i32::MIN, i32::MIN + 1, -1000, 1000, i32::MAX - 1, i32::MAX] {
        same_lz4f!(c, r, unsafe { (c.get_block_size)(id) }, unsafe { (r.get_block_size)(id) }, "getBlockSize({})", id);
    }
}

/// ERRORS rows 3-5, 8, 26-31: out-of-range enum values in LZ4F_preferences_t are
/// (mostly) *not* validated on the compression path. Whatever the C does — error
/// or silently corrupt frame — the Rust must do bit-for-bit.
#[test]
fn errf_invalid_prefs_enums() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7001);
    let bad_vals: [c_int; 10] = [-1, 1, 2, 3, 8, 9, 100, -1000, i32::MIN, i32::MAX];
    for field in 0..5 {
        for &v in &bad_vals {
            let mut p = LZ4F_preferences_t::default();
            match field {
                0 => p.frameInfo.blockSizeID = v,
                1 => p.frameInfo.blockMode = v,
                2 => p.frameInfo.contentChecksumFlag = v,
                3 => p.frameInfo.blockChecksumFlag = v,
                _ => p.frameInfo.frameType = v,
            }
            // bounds first: these must agree even when nonsensical
            for &n in &[0usize, 1, 1000, 100_000] {
                same_lz4f!(c, r, unsafe { (c.bound)(n, &p) }, unsafe { (r.bound)(n, &p) }, "compressBound(field={}, v={}, n={})", field, v, n);
                same_lz4f!(c, r, unsafe { (c.frame_bound)(n, &p) }, unsafe { (r.frame_bound)(n, &p) }, "compressFrameBound(field={}, v={}, n={})", field, v, n);
            }
            // one-shot compressFrame: use a generous, *identical* capacity for
            // both so the outcome depends only on the library logic.
            for &n in &[0usize, 1, 1000] {
                let data = gen(Shape::Text, n, &mut rng);
                let cap = 1 << 20;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = unsafe {
                    (c.compress_frame)(
                        cb.as_mut_ptr() as *mut c_void,
                        cap,
                        data.as_ptr() as *const c_void,
                        n,
                        &p,
                    )
                };
                let b = unsafe {
                    (r.compress_frame)(
                        rb.as_mut_ptr() as *mut c_void,
                        cap,
                        data.as_ptr() as *const c_void,
                        n,
                        &p,
                    )
                };
                same_lz4f!(c, r, a, b, "compressFrame(field={}, v={}, n={})", field, v, n);
                if unsafe { (c.is_error)(a) } == 0 {
                    assert_bytes_eq(
                        &format!("compressFrame bytes field={} v={} n={}", field, v, n),
                        &cb[..a],
                        &rb[..b],
                    );
                }
            }
            // streaming compressBegin with the same bogus value
            unsafe {
                let mut cc: *mut c_void = std::ptr::null_mut();
                let mut rc_: *mut c_void = std::ptr::null_mut();
                (c.create_cctx)(&mut cc, LZ4F_VERSION);
                (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                let mut chb = vec![0u8; 64];
                let mut rhb = vec![0u8; 64];
                let a = (c.begin)(cc, chb.as_mut_ptr() as *mut c_void, 64, &p);
                let b = (r.begin)(rc_, rhb.as_mut_ptr() as *mut c_void, 64, &p);
                same_lz4f!(c, r, a, b, "compressBegin(field={}, v={})", field, v);
                if (c.is_error)(a) == 0 {
                    assert_bytes_eq(
                        &format!("compressBegin header field={} v={}", field, v),
                        &chb[..a],
                        &rhb[..b],
                    );
                }
                (c.free_cctx)(cc);
                (r.free_cctx)(rc_);
            }
        }
    }
    // out-of-range compressionLevel and the boolean-ish unsigned options
    for &lvl in &[i32::MIN, -1000, -1, 0, 1, 2, 13, 100, i32::MAX] {
        for &af in &[0u32, 1, 2, u32::MAX] {
            for &fd in &[0u32, 1, 2, u32::MAX] {
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = lvl;
                p.autoFlush = af;
                p.favorDecSpeed = fd;
                let n = 5000usize;
                let data = gen(Shape::Text, n, &mut rng);
                let cap = 1 << 20;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = unsafe {
                    (c.compress_frame)(cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
                };
                let b = unsafe {
                    (r.compress_frame)(rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
                };
                same_lz4f!(c, r, a, b, "compressFrame(lvl={}, af={}, fd={})", lvl, af, fd);
                if unsafe { (c.is_error)(a) } == 0 {
                    assert_bytes_eq("compressFrame options bytes", &cb[..a], &rb[..b]);
                }
            }
        }
    }
    // reserved fields non-zero (documented as "must be zero")
    for &rv in &[1u32, u32::MAX] {
        for slot in 0..3 {
            let mut p = LZ4F_preferences_t::default();
            p.reserved[slot] = rv;
            let n = 1000usize;
            let data = gen(Shape::Text, n, &mut rng);
            let cap = 1 << 20;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe {
                (c.compress_frame)(cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
            };
            let b = unsafe {
                (r.compress_frame)(rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
            };
            same_lz4f!(c, r, a, b, "compressFrame(reserved[{}]={})", slot, rv);
            if unsafe { (c.is_error)(a) } == 0 {
                assert_bytes_eq("compressFrame reserved bytes", &cb[..a], &rb[..b]);
            }
        }
    }
    // compressOptions_t: stableSrc and reserved
    for &ss in &[0u32, 1, 2, u32::MAX] {
        for &rv in &[0u32, 1] {
            let co = LZ4F_compressOptions_t {
                stableSrc: ss,
                reserved: [rv, rv, rv],
            };
            let n = 5000usize;
            let data = gen(Shape::Text, n, &mut rng);
            unsafe {
                let mut cc: *mut c_void = std::ptr::null_mut();
                let mut rc_: *mut c_void = std::ptr::null_mut();
                (c.create_cctx)(&mut cc, LZ4F_VERSION);
                (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                let p = LZ4F_preferences_t::default();
                let mut chb = vec![0u8; 64];
                let mut rhb = vec![0u8; 64];
                (c.begin)(cc, chb.as_mut_ptr() as *mut c_void, 64, &p);
                (r.begin)(rc_, rhb.as_mut_ptr() as *mut c_void, 64, &p);
                let cap = (c.bound)(n, &p);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let a = (c.update)(cc, cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &co);
                let b = (r.update)(rc_, rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &co);
                same_lz4f!(c, r, a, b, "compressUpdate(stableSrc={}, reserved={})", ss, rv);
                assert_bytes_eq("compressUpdate opts bytes", &cb[..a.min(cap)], &rb[..b.min(cap)]);
                (c.free_cctx)(cc);
                (r.free_cctx)(rc_);
            }
        }
    }
    // decompressOptions_t: out-of-range flags and non-zero reserved
    let n = 20_000usize;
    let data = gen(Shape::Text, n, &mut rng);
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.contentChecksumFlag = 1;
    p.frameInfo.blockChecksumFlag = 1;
    let cap = unsafe { (c.frame_bound)(n, &p) };
    let mut frame = vec![0u8; cap];
    let flen = unsafe {
        (c.compress_frame)(frame.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
    };
    let frame = &frame[..flen];
    for &sd in &[0u32, 1, 2, u32::MAX] {
        for &sc in &[0u32, 1, 2, u32::MAX] {
            for &rv in &[0u32, 7] {
                let o = LZ4F_decompressOptions_t {
                    stableDst: sd,
                    skipChecksums: sc,
                    reserved1: rv,
                    reserved0: rv,
                };
                unsafe {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    (c.create_dctx)(&mut cd, LZ4F_VERSION);
                    (r.create_dctx)(&mut rd, LZ4F_VERSION);
                    let mut co = vec![0u8; n + 4096];
                    let mut ro = vec![0u8; n + 4096];
                    let mut cds = co.len();
                    let mut rds = ro.len();
                    let mut css = frame.len();
                    let mut rss = frame.len();
                    let a = (c.decompress)(
                        cd,
                        co.as_mut_ptr() as *mut c_void,
                        &mut cds,
                        frame.as_ptr() as *const c_void,
                        &mut css,
                        &o,
                    );
                    let b = (r.decompress)(
                        rd,
                        ro.as_mut_ptr() as *mut c_void,
                        &mut rds,
                        frame.as_ptr() as *const c_void,
                        &mut rss,
                        &o,
                    );
                    same_lz4f!(c, r, a, b, "decompress(stableDst={}, skip={}, rsv={})", sd, sc, rv);
                    assert_eq!(cds, rds, "decompress dstSize (sd={} sc={})", sd, sc);
                    assert_eq!(css, rss, "decompress srcSize (sd={} sc={})", sd, sc);
                    assert_bytes_eq("decompress opts out", &co[..cds], &ro[..rds]);
                    (c.free_dctx)(cd);
                    (r.free_dctx)(rd);
                }
            }
        }
    }
}

/// ERRORS row 7 + 22: dstCapacity too small for LZ4F_compressFrame /
/// LZ4F_compressBegin.
#[test]
fn errf_dst_too_small_begin_and_frame() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7002);
    for &bsid in &[0i32, 4, 5, 6, 7] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        for &n in &[0usize, 1, 1000, 100_000] {
            let data = gen(Shape::Text, n, &mut rng);
            let need = unsafe { (c.frame_bound)(n, &p) };
            for cap in [0usize, 1, 7, 18, 19, 20, need / 2, need - 1] {
                if cap >= need {
                    continue;
                }
                let mut cb = vec![0u8; cap + 1];
                let mut rb = vec![0u8; cap + 1];
                let a = unsafe {
                    (c.compress_frame)(cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
                };
                let b = unsafe {
                    (r.compress_frame)(rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
                };
                same_lz4f!(c, r, a, b, "compressFrame(cap={}, need={}, bsid={}, n={})", cap, need, bsid, n);
            }
        }
        // compressBegin needs >= LZ4F_HEADER_SIZE_MAX
        for cap in 0..=LZ4F_HEADER_SIZE_MAX + 1 {
            unsafe {
                let mut cc: *mut c_void = std::ptr::null_mut();
                let mut rc_: *mut c_void = std::ptr::null_mut();
                (c.create_cctx)(&mut cc, LZ4F_VERSION);
                (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                let mut cb = vec![0u8; cap + 1];
                let mut rb = vec![0u8; cap + 1];
                let a = (c.begin)(cc, cb.as_mut_ptr() as *mut c_void, cap, &p);
                let b = (r.begin)(rc_, rb.as_mut_ptr() as *mut c_void, cap, &p);
                same_lz4f!(c, r, a, b, "compressBegin(cap={}, bsid={})", cap, bsid);
                // and the dictionary variants
                let d = vec![7u8; 100];
                for mode in 0..2 {
                    let mut cc2: *mut c_void = std::ptr::null_mut();
                    let mut rc2: *mut c_void = std::ptr::null_mut();
                    (c.create_cctx)(&mut cc2, LZ4F_VERSION);
                    (r.create_cctx)(&mut rc2, LZ4F_VERSION);
                    let f = if mode == 0 {
                        (c.begin_dict, r.begin_dict)
                    } else {
                        (c.begin_dict_once, r.begin_dict_once)
                    };
                    let a = (f.0)(
                        cc2,
                        cb.as_mut_ptr() as *mut c_void,
                        cap,
                        d.as_ptr() as *const c_void,
                        d.len(),
                        &p,
                    );
                    let b = (f.1)(
                        rc2,
                        rb.as_mut_ptr() as *mut c_void,
                        cap,
                        d.as_ptr() as *const c_void,
                        d.len(),
                        &p,
                    );
                    same_lz4f!(c, r, a, b, "compressBegin_usingDict(mode={}, cap={})", mode, cap);
                    (c.free_cctx)(cc2);
                    (r.free_cctx)(rc2);
                }
                // CDict variant, including a NULL cdict
                let ccd = (c.create_cdict)(d.as_ptr() as *const c_void, d.len());
                let rcd = (r.create_cdict)(d.as_ptr() as *const c_void, d.len());
                for use_null in [false, true] {
                    let mut cc2: *mut c_void = std::ptr::null_mut();
                    let mut rc2: *mut c_void = std::ptr::null_mut();
                    (c.create_cctx)(&mut cc2, LZ4F_VERSION);
                    (r.create_cctx)(&mut rc2, LZ4F_VERSION);
                    let a = (c.begin_cdict)(
                        cc2,
                        cb.as_mut_ptr() as *mut c_void,
                        cap,
                        if use_null { std::ptr::null() } else { ccd },
                        &p,
                    );
                    let b = (r.begin_cdict)(
                        rc2,
                        rb.as_mut_ptr() as *mut c_void,
                        cap,
                        if use_null { std::ptr::null() } else { rcd },
                        &p,
                    );
                    same_lz4f!(c, r, a, b, "compressBegin_usingCDict(null={}, cap={})", use_null, cap);
                    (c.free_cctx)(cc2);
                    (r.free_cctx)(rc2);
                }
                (c.free_cdict)(ccd);
                (r.free_cdict)(rcd);
                (c.free_cctx)(cc);
                (r.free_cctx)(rc_);
            }
        }
    }
    // compressFrame_usingCDict with a too-small dst and a NULL cdict
    let n = 10_000usize;
    let data = gen(Shape::Text, n, &mut rng);
    let p = LZ4F_preferences_t::default();
    let need = unsafe { (c.frame_bound)(n, &p) };
    for cap in [0usize, 1, 19, need / 2, need - 1, need] {
        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc_: *mut c_void = std::ptr::null_mut();
            (c.create_cctx)(&mut cc, LZ4F_VERSION);
            (r.create_cctx)(&mut rc_, LZ4F_VERSION);
            let mut cb = vec![0u8; cap + 1];
            let mut rb = vec![0u8; cap + 1];
            let a = (c.frame_using_cdict)(
                cc,
                cb.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                n,
                std::ptr::null(),
                &p,
            );
            let b = (r.frame_using_cdict)(
                rc_,
                rb.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                n,
                std::ptr::null(),
                &p,
            );
            same_lz4f!(c, r, a, b, "compressFrame_usingCDict(NULL cdict, cap={})", cap);
            (c.free_cctx)(cc);
            (r.free_cctx)(rc_);
        }
    }
}

/// ERRORS rows 32-34, 36-43: the compression state machine's rejections.
#[test]
fn errf_compress_state_machine() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7003);
    let data = gen(Shape::Text, 200_000, &mut rng);
    let p = LZ4F_preferences_t::default();

    // (a) compressUpdate / uncompressedUpdate / flush / compressEnd before begin
    unsafe {
        let mut cc: *mut c_void = std::ptr::null_mut();
        let mut rc_: *mut c_void = std::ptr::null_mut();
        (c.create_cctx)(&mut cc, LZ4F_VERSION);
        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
        let cap = 1 << 20;
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        same_lz4f!(
            c, r,
            (c.update)(cc, cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, 100, std::ptr::null()),
            (r.update)(rc_, rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, 100, std::ptr::null()),
            "compressUpdate before compressBegin"
        );
        same_lz4f!(
            c, r,
            (c.uncompressed_update)(cc, cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, 100, std::ptr::null()),
            (r.uncompressed_update)(rc_, rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, 100, std::ptr::null()),
            "uncompressedUpdate before compressBegin"
        );
        // flush() short-circuits on an empty tmp buffer *before* the state check
        same_lz4f!(
            c, r,
            (c.flush)(cc, cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
            (r.flush)(rc_, rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
            "flush before compressBegin"
        );
        same_lz4f!(
            c, r,
            (c.end)(cc, cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
            (r.end)(rc_, rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
            "compressEnd before compressBegin"
        );
        (c.free_cctx)(cc);
        (r.free_cctx)(rc_);
    }

    // (b) compressUpdate with dstCapacity below LZ4F_compressBound
    for &bsid in &[0i32, 4, 5, 6, 7] {
        for &af in &[0u32, 1] {
            let mut p2 = LZ4F_preferences_t::default();
            p2.frameInfo.blockSizeID = bsid;
            p2.autoFlush = af;
            for &n in &[1usize, 100, 70_000] {
                let need = unsafe { (c.bound)(n, &p2) };
                for cap in [0usize, 1, 4, need / 2, need - 1] {
                    unsafe {
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let mut hb = vec![0u8; 64];
                        (c.begin)(cc, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                        (r.begin)(rc_, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                        let mut cb = vec![0u8; cap + 1];
                        let mut rb = vec![0u8; cap + 1];
                        same_lz4f!(
                            c, r,
                            (c.update)(cc, cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, std::ptr::null()),
                            (r.update)(rc_, rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, std::ptr::null()),
                            "compressUpdate(bsid={}, af={}, n={}, cap={}, need={})", bsid, af, n, cap, need
                        );
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);
                    }
                }
                // uncompressedUpdate needs dstCapacity >= srcSize + headers
                for cap in [0usize, 1, 4, n, n + 3, n + 4] {
                    unsafe {
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let mut hb = vec![0u8; 64];
                        (c.begin)(cc, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                        (r.begin)(rc_, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                        let mut cb = vec![0u8; cap + 1];
                        let mut rb = vec![0u8; cap + 1];
                        same_lz4f!(
                            c, r,
                            (c.uncompressed_update)(cc, cb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, std::ptr::null()),
                            (r.uncompressed_update)(rc_, rb.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, std::ptr::null()),
                            "uncompressedUpdate(bsid={}, af={}, n={}, cap={})", bsid, af, n, cap
                        );
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);
                    }
                }
            }
        }
    }

    // (c) flush with buffered data but no room ; compressEnd with no room
    for &bsid in &[4i32, 5] {
        let mut p2 = LZ4F_preferences_t::default();
        p2.frameInfo.blockSizeID = bsid;
        for &cchk in &[0i32, 1] {
            p2.frameInfo.contentChecksumFlag = cchk;
            unsafe {
                let mut cc: *mut c_void = std::ptr::null_mut();
                let mut rc_: *mut c_void = std::ptr::null_mut();
                (c.create_cctx)(&mut cc, LZ4F_VERSION);
                (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                let mut hb = vec![0u8; 64];
                (c.begin)(cc, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                (r.begin)(rc_, hb.as_mut_ptr() as *mut c_void, 64, &p2);
                // buffer a small amount (no flush because autoFlush == 0)
                let ucap = (c.bound)(100, &p2);
                let mut cb = vec![0u8; ucap];
                let mut rb = vec![0u8; ucap];
                (c.update)(cc, cb.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, 100, std::ptr::null());
                (r.update)(rc_, rb.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, 100, std::ptr::null());
                for cap in [0usize, 1, 4, 7, 8, 50, 100, 200] {
                    let mut cb = vec![0u8; cap + 1];
                    let mut rb = vec![0u8; cap + 1];
                    same_lz4f!(
                        c, r,
                        (c.flush)(cc, cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
                        (r.flush)(rc_, rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
                        "flush(bsid={}, cchk={}, cap={})", bsid, cchk, cap
                    );
                }
                for cap in [0usize, 1, 2, 3, 4, 5, 7, 8, 9] {
                    let mut cb = vec![0u8; cap + 1];
                    let mut rb = vec![0u8; cap + 1];
                    same_lz4f!(
                        c, r,
                        (c.end)(cc, cb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
                        (r.end)(rc_, rb.as_mut_ptr() as *mut c_void, cap, std::ptr::null()),
                        "compressEnd(bsid={}, cchk={}, cap={})", bsid, cchk, cap
                    );
                }
                (c.free_cctx)(cc);
                (r.free_cctx)(rc_);
            }
        }
    }

    // (d) declared contentSize != actual -> frameSize_wrong (row 43)
    for &(declared, actual) in &[
        (100u64, 100usize),
        (100, 99),
        (100, 101),
        (0, 100),
        (1, 0),
        (u64::MAX, 10),
    ] {
        let mut p2 = p;
        p2.frameInfo.contentSize = declared;
        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc_: *mut c_void = std::ptr::null_mut();
            (c.create_cctx)(&mut cc, LZ4F_VERSION);
            (r.create_cctx)(&mut rc_, LZ4F_VERSION);
            let mut hb = vec![0u8; 64];
            (c.begin)(cc, hb.as_mut_ptr() as *mut c_void, 64, &p2);
            (r.begin)(rc_, hb.as_mut_ptr() as *mut c_void, 64, &p2);
            let ucap = (c.bound)(actual, &p2);
            let mut cb = vec![0u8; ucap.max(1)];
            let mut rb = vec![0u8; ucap.max(1)];
            if actual > 0 {
                (c.update)(cc, cb.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, actual, std::ptr::null());
                (r.update)(rc_, rb.as_mut_ptr() as *mut c_void, ucap, data.as_ptr() as *const c_void, actual, std::ptr::null());
            }
            let ecap = (c.bound)(0, &p2);
            let mut cb = vec![0u8; ecap];
            let mut rb = vec![0u8; ecap];
            same_lz4f!(
                c, r,
                (c.end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null()),
                (r.end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null()),
                "compressEnd(declared={}, actual={})", declared, actual
            );
            (c.free_cctx)(cc);
            (r.free_cctx)(rc_);
        }
    }
}

/// ERRORS rows 59-61: LZ4F_headerSize.
#[test]
fn errf_header_size() {
    let (c, r) = pair();
    // NULL src -> srcPtr_wrong
    for &n in &[0usize, 1, 5, 19, usize::MAX] {
        same_lz4f!(
            c, r,
            unsafe { (c.header_size)(std::ptr::null(), n) },
            unsafe { (r.header_size)(std::ptr::null(), n) },
            "headerSize(NULL, {})", n
        );
    }
    let mut rng = Rng::new(0x7004);
    // short buffers, bad magic, skippable magic, and every FLG bit pattern
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for len in 0..24usize {
        inputs.push(gen(Shape::Random, len, &mut rng));
    }
    for m in [
        MAGIC,
        MAGIC ^ 1,
        0x184D2A50,
        0x184D2A5F,
        0x184D2A60,
        0x184D2A4F,
        0,
        u32::MAX,
    ] {
        for flg in [0x40u8, 0x44, 0x48, 0x4C, 0x60, 0x68, 0x69, 0x00, 0xFF] {
            let mut v = m.to_le_bytes().to_vec();
            v.push(flg);
            v.push(0x40);
            v.extend_from_slice(&[0u8; 16]);
            inputs.push(v);
        }
    }
    for v in &inputs {
        for &n in &[0usize, 1, 4, 5, 6, 7, v.len()] {
            if n > v.len() {
                continue;
            }
            same_lz4f!(
                c, r,
                unsafe { (c.header_size)(v.as_ptr() as *const c_void, n) },
                unsafe { (r.header_size)(v.as_ptr() as *const c_void, n) },
                "headerSize(len={}, hex={})", n, hexdump(v)
            );
        }
    }
}

/// ERRORS rows 50-58: every LZ4F_decodeHeader rejection, reached through
/// LZ4F_getFrameInfo and LZ4F_decompress.
#[test]
fn errf_decode_header_rejections() {
    let (c, r) = pair();
    let mut headers: Vec<(String, Vec<u8>)> = Vec::new();
    // good baseline
    headers.push(("good".into(), make_header(MAGIC, 0x40 | 0x20, 0x40, None, None, true)));
    // bad magic -> frameType_unknown
    for m in [MAGIC ^ 1, MAGIC ^ 0x1000_0000, 0, u32::MAX, 0x184D2205] {
        headers.push((format!("magic={:#x}", m), make_header(m, 0x60, 0x40, None, None, true)));
    }
    // skippable magic range
    for m in 0x184D2A50u32..=0x184D2A5Fu32 {
        headers.push((format!("skippable={:#x}", m), make_header(m, 0x60, 0x40, None, None, true)));
    }
    // FLG: version bits (bits 7-6) other than 01
    for ver in [0u8, 2, 3] {
        let flg = (ver << 6) | 0x20;
        headers.push((format!("version={}", ver), make_header(MAGIC, flg, 0x40, None, None, true)));
    }
    // FLG bit1 reserved
    headers.push(("flg reserved bit1".into(), make_header(MAGIC, 0x40 | 0x20 | 0x02, 0x40, None, None, true)));
    // BD: bit7 reserved, blockSizeID 0..3, low nibble reserved
    for bd in [0x00u8, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0xC0, 0x41, 0x4F, 0xFF] {
        headers.push((format!("bd={:#04x}", bd), make_header(MAGIC, 0x60, bd, None, None, true)));
    }
    // wrong header checksum
    headers.push(("bad HC".into(), make_header(MAGIC, 0x60, 0x40, None, None, false)));
    // with contentSize / dictID present, good and bad HC
    for &good in &[true, false] {
        headers.push((
            format!("contentSize good_hc={}", good),
            make_header(MAGIC, 0x40 | 0x20 | 0x08, 0x40, Some(1234), None, good),
        ));
        headers.push((
            format!("dictID good_hc={}", good),
            make_header(MAGIC, 0x40 | 0x20 | 0x01, 0x40, None, Some(0xABCD), good),
        ));
        headers.push((
            format!("both good_hc={}", good),
            make_header(MAGIC, 0x40 | 0x20 | 0x08 | 0x01, 0x40, Some(7), Some(9), good),
        ));
    }
    // all FLG bit combinations with a correct checksum
    for bits in 0u8..64 {
        let flg = 0x40 | bits;
        headers.push((format!("flg={:#04x}", flg), make_header(MAGIC, flg, 0x40, None, None, true)));
    }

    for (name, h) in &headers {
        // (a) LZ4F_getFrameInfo with every prefix length
        for n in 0..=h.len() {
            unsafe {
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                (c.create_dctx)(&mut cd, LZ4F_VERSION);
                (r.create_dctx)(&mut rd, LZ4F_VERSION);
                let mut ci = LZ4F_frameInfo_t::default();
                let mut ri = LZ4F_frameInfo_t::default();
                let mut cs = n;
                let mut rs = n;
                let a = (c.get_frame_info)(cd, &mut ci, h.as_ptr() as *const c_void, &mut cs);
                let b = (r.get_frame_info)(rd, &mut ri, h.as_ptr() as *const c_void, &mut rs);
                same_lz4f!(c, r, a, b, "getFrameInfo({}, prefix={})", name, n);
                assert_eq!(cs, rs, "getFrameInfo({}, prefix={}) consumed", name, n);
                assert_eq!(ci, ri, "getFrameInfo({}, prefix={}) info", name, n);
                // LZ4F_freeDecompressionContext returns dctx->dStage (row 49)
                let fa = (c.free_dctx)(cd);
                let fb = (r.free_dctx)(rd);
                assert_eq!(fa, fb, "freeDecompressionContext({}, prefix={})", name, n);
            }
        }
        // (b) LZ4F_decompress on the header alone
        for n in 0..=h.len() {
            unsafe {
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                (c.create_dctx)(&mut cd, LZ4F_VERSION);
                (r.create_dctx)(&mut rd, LZ4F_VERSION);
                let mut co = vec![0u8; 4096];
                let mut ro = vec![0u8; 4096];
                let mut cds = co.len();
                let mut rds = ro.len();
                let mut css = n;
                let mut rss = n;
                let a = (c.decompress)(
                    cd,
                    co.as_mut_ptr() as *mut c_void,
                    &mut cds,
                    h.as_ptr() as *const c_void,
                    &mut css,
                    std::ptr::null(),
                );
                let b = (r.decompress)(
                    rd,
                    ro.as_mut_ptr() as *mut c_void,
                    &mut rds,
                    h.as_ptr() as *const c_void,
                    &mut rss,
                    std::ptr::null(),
                );
                same_lz4f!(c, r, a, b, "decompress header({}, prefix={})", name, n);
                assert_eq!(cds, rds, "decompress header({}) dst", name);
                assert_eq!(css, rss, "decompress header({}) src", name);
                assert_bytes_eq("decompress header out", &co[..cds], &ro[..rds]);
                assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd), "free after header({})", name);
            }
        }
        // (c) byte-at-a-time feed, then LZ4F_getFrameInfo mid-header
        //     (-> frameDecoding_alreadyStarted, row 62)
        if h.len() >= 6 {
            unsafe {
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                (c.create_dctx)(&mut cd, LZ4F_VERSION);
                (r.create_dctx)(&mut rd, LZ4F_VERSION);
                let mut co = vec![0u8; 4096];
                let mut ro = vec![0u8; 4096];
                let mut cds = co.len();
                let mut rds = ro.len();
                let mut css = 5usize;
                let mut rss = 5usize;
                let a = (c.decompress)(
                    cd,
                    co.as_mut_ptr() as *mut c_void,
                    &mut cds,
                    h.as_ptr() as *const c_void,
                    &mut css,
                    std::ptr::null(),
                );
                let b = (r.decompress)(
                    rd,
                    ro.as_mut_ptr() as *mut c_void,
                    &mut rds,
                    h.as_ptr() as *const c_void,
                    &mut rss,
                    std::ptr::null(),
                );
                same_lz4f!(c, r, a, b, "decompress 5-byte prefix({})", name);
                let mut ci = LZ4F_frameInfo_t::default();
                let mut ri = LZ4F_frameInfo_t::default();
                let mut cs = h.len();
                let mut rs = h.len();
                let a = (c.get_frame_info)(cd, &mut ci, h.as_ptr() as *const c_void, &mut cs);
                let b = (r.get_frame_info)(rd, &mut ri, h.as_ptr() as *const c_void, &mut rs);
                same_lz4f!(c, r, a, b, "getFrameInfo mid-header({})", name);
                assert_eq!(cs, rs, "getFrameInfo mid-header consumed({})", name);
                assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
            }
        }
    }
}

/// ERRORS rows 71, 73-78: rejections inside the frame body.
#[test]
fn errf_frame_body_rejections() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7005);
    for &bsid in &[4i32, 5, 7] {
        for &cchk in &[0i32, 1] {
            for &bchk in &[0i32, 1] {
                for &bmode in &[0i32, 1] {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.contentChecksumFlag = cchk;
                    p.frameInfo.blockChecksumFlag = bchk;
                    p.frameInfo.blockMode = bmode;
                    let n = 50_000usize;
                    let data = gen(Shape::Text, n, &mut rng);
                    let cap = unsafe { (c.frame_bound)(n, &p) };
                    let mut frame = vec![0u8; cap];
                    let flen = unsafe {
                        (c.compress_frame)(
                            frame.as_mut_ptr() as *mut c_void,
                            cap,
                            data.as_ptr() as *const c_void,
                            n,
                            &p,
                        )
                    };
                    assert!(unsafe { (c.is_error)(flen) } == 0);
                    frame.truncate(flen);
                    let hsize = unsafe { (c.header_size)(frame.as_ptr() as *const c_void, flen) };

                    // (a) single-byte corruptions anywhere after the header
                    for _ in 0..120 {
                        let mut bad = frame.clone();
                        let pos = rng.range(hsize, bad.len());
                        bad[pos] ^= 1u8 << rng.below(8);
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let (ra, oa) = feed_all(&c, cd, &bad);
                            let (rb, ob) = feed_all(&r, rd, &bad);
                            same_lz4f!(
                                c, r, ra, rb,
                                "corrupt frame bsid={} cchk={} bchk={} bmode={} pos={}",
                                bsid, cchk, bchk, bmode, pos
                            );
                            assert_bytes_eq("corrupt frame output", &oa, &ob);
                            assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd), "free after corrupt");
                        }
                    }
                    // (b) truncations
                    for cut in [
                        hsize,
                        hsize + 1,
                        hsize + 2,
                        hsize + 3,
                        hsize + 4,
                        flen / 2,
                        flen - 1,
                        flen - 4,
                    ] {
                        if cut > flen {
                            continue;
                        }
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let (ra, oa) = feed_all(&c, cd, &frame[..cut]);
                            let (rb, ob) = feed_all(&r, rd, &frame[..cut]);
                            same_lz4f!(c, r, ra, rb, "truncated frame bsid={} cut={}", bsid, cut);
                            assert_bytes_eq("truncated frame output", &oa, &ob);
                            assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
                        }
                    }
                    // (c) an oversized block-size field -> maxBlockSize_invalid (row 71)
                    {
                        let mut bad = frame[..hsize].to_vec();
                        bad.extend_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
                        bad.extend_from_slice(&vec![0u8; 64]);
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let (ra, _) = feed_all(&c, cd, &bad);
                            let (rb, _) = feed_all(&r, rd, &bad);
                            same_lz4f!(c, r, ra, rb, "oversized block header bsid={}", bsid);
                            assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
                        }
                    }
                    // (d) uncompressed-block flag with an oversized size
                    for size_field in [0x8000_0000u32, 0x8000_0000 | 0x0FFF_FFFF, 0x8FFF_FFFF] {
                        let mut bad = frame[..hsize].to_vec();
                        bad.extend_from_slice(&size_field.to_le_bytes());
                        bad.extend_from_slice(&vec![0x42u8; 128]);
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let (ra, _) = feed_all(&c, cd, &bad);
                            let (rb, _) = feed_all(&r, rd, &bad);
                            same_lz4f!(
                                c, r, ra, rb,
                                "uncompressed oversized block bsid={} size={:#x}",
                                bsid, size_field
                            );
                            assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
                        }
                    }
                }
            }
        }
    }
    // (e) declared contentSize larger than actual content -> frameSize_wrong
    for &(declared, actual) in &[(1000u64, 500usize), (10, 0), (u64::MAX, 100)] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.contentSize = declared;
        let data = gen(Shape::Text, actual, &mut rng);
        // build the frame by hand so the declared size can lie
        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            (c.create_cctx)(&mut cc, LZ4F_VERSION);
            let mut hb = vec![0u8; 64];
            let hl = (c.begin)(cc, hb.as_mut_ptr() as *mut c_void, 64, &p);
            let mut frame = hb[..hl].to_vec();
            let ucap = (c.bound)(actual, &p);
            let mut ub = vec![0u8; ucap.max(1)];
            if actual > 0 {
                let ul = (c.update)(
                    cc,
                    ub.as_mut_ptr() as *mut c_void,
                    ucap,
                    data.as_ptr() as *const c_void,
                    actual,
                    std::ptr::null(),
                );
                frame.extend_from_slice(&ub[..ul]);
            }
            // flush + write the end mark manually (compressEnd would refuse)
            let fl = (c.flush)(cc, ub.as_mut_ptr() as *mut c_void, ucap.max(1), std::ptr::null());
            if (c.is_error)(fl) == 0 {
                frame.extend_from_slice(&ub[..fl]);
            }
            frame.extend_from_slice(&0u32.to_le_bytes());
            (c.free_cctx)(cc);

            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            (c.create_dctx)(&mut cd, LZ4F_VERSION);
            (r.create_dctx)(&mut rd, LZ4F_VERSION);
            let (ra, oa) = feed_all(&c, cd, &frame);
            let (rb, ob) = feed_all(&r, rd, &frame);
            same_lz4f!(c, r, ra, rb, "declared={} actual={}", declared, actual);
            assert_bytes_eq("declared size output", &oa, &ob);
            assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
        }
    }
}

/// ERRORS row 80 + generic boundaries of LZ4F_decompress_usingDict.
#[test]
fn errf_decompress_using_dict_edges() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7006);
    let dict = gen(Shape::Text, 70_000, &mut rng);
    let n = 20_000usize;
    let data = gen(Shape::Text, n, &mut rng);
    let p = LZ4F_preferences_t::default();
    let cap = unsafe { (c.frame_bound)(n, &p) };
    let mut frame = vec![0u8; cap];
    let flen = unsafe {
        (c.compress_frame)(frame.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
    };
    let frame = &frame[..flen];
    for &ds in &[0usize, 1, 65536, 70_000] {
        for &dst_cap in &[0usize, 1, 100, n, n + 4096] {
            for use_null_dict in [false, true] {
                unsafe {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    (c.create_dctx)(&mut cd, LZ4F_VERSION);
                    (r.create_dctx)(&mut rd, LZ4F_VERSION);
                    let mut co = vec![0u8; dst_cap + 1];
                    let mut ro = vec![0u8; dst_cap + 1];
                    let mut cds = dst_cap;
                    let mut rds = dst_cap;
                    let mut css = flen;
                    let mut rss = flen;
                    let dp: *const c_void = if use_null_dict {
                        std::ptr::null()
                    } else {
                        dict.as_ptr() as *const c_void
                    };
                    // a NULL dictionary pointer with a non-zero size would be UB
                    let dsz = if use_null_dict { 0 } else { ds };
                    let a = (c.decompress_using_dict)(
                        cd,
                        co.as_mut_ptr() as *mut c_void,
                        &mut cds,
                        frame.as_ptr() as *const c_void,
                        &mut css,
                        dp,
                        dsz,
                        std::ptr::null(),
                    );
                    let b = (r.decompress_using_dict)(
                        rd,
                        ro.as_mut_ptr() as *mut c_void,
                        &mut rds,
                        frame.as_ptr() as *const c_void,
                        &mut rss,
                        dp,
                        dsz,
                        std::ptr::null(),
                    );
                    same_lz4f!(
                        c, r, a, b,
                        "decompress_usingDict(ds={}, dstCap={}, nullDict={})",
                        dsz, dst_cap, use_null_dict
                    );
                    assert_eq!(cds, rds, "decompress_usingDict dst");
                    assert_eq!(css, rss, "decompress_usingDict src");
                    assert_bytes_eq("decompress_usingDict out", &co[..cds], &ro[..rds]);
                    (c.free_dctx)(cd);
                    (r.free_dctx)(rd);
                }
            }
        }
    }
}

/// ERRORS rows 13-16, 20, 21, 47, 48: constructors / destructors and the
/// (unvalidated) `version` argument.
#[test]
fn errf_context_lifecycle() {
    let (c, r) = pair();
    for &v in &[0u32, 1, 99, LZ4F_VERSION, 101, 1000, u32::MAX] {
        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc_: *mut c_void = std::ptr::null_mut();
            same_lz4f!(c, r, (c.create_cctx)(&mut cc, v), (r.create_cctx)(&mut rc_, v), "createCompressionContext(v={})", v);
            assert_eq!(cc.is_null(), rc_.is_null(), "cctx null-ness v={}", v);
            same_lz4f!(c, r, (c.free_cctx)(cc), (r.free_cctx)(rc_), "freeCompressionContext(v={})", v);
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            same_lz4f!(c, r, (c.create_dctx)(&mut cd, v), (r.create_dctx)(&mut rd, v), "createDecompressionContext(v={})", v);
            assert_eq!(cd.is_null(), rd.is_null(), "dctx null-ness v={}", v);
            // resetDecompressionContext on a fresh context, then free
            (c.reset_dctx)(cd);
            (r.reset_dctx)(rd);
            same_lz4f!(c, r, (c.free_dctx)(cd), (r.free_dctx)(rd), "freeDecompressionContext(v={})", v);
        }
    }
    // free on NULL
    unsafe {
        same_lz4f!(c, r, (c.free_cctx)(std::ptr::null_mut()), (r.free_cctx)(std::ptr::null_mut()), "freeCompressionContext(NULL)");
        same_lz4f!(c, r, (c.free_dctx)(std::ptr::null_mut()), (r.free_dctx)(std::ptr::null_mut()), "freeDecompressionContext(NULL)");
        (c.free_cdict)(std::ptr::null_mut());
        (r.free_cdict)(std::ptr::null_mut());
    }
    // createCDict with a NULL buffer / zero size, and a > 64 KB dictionary
    let mut rng = Rng::new(0x7007);
    for &ds in &[0usize, 1, 65536, 65537, 300_000] {
        let d = gen(Shape::Text, ds, &mut rng);
        unsafe {
            let a = (c.create_cdict)(d.as_ptr() as *const c_void, ds);
            let b = (r.create_cdict)(d.as_ptr() as *const c_void, ds);
            assert_eq!(a.is_null(), b.is_null(), "createCDict(ds={})", ds);
            // using it must give identical output (proves identical truncation)
            if !a.is_null() {
                let n = 20_000usize;
                let mut data = gen(Shape::Text, n, &mut rng);
                let k = n.min(ds);
                if k > 0 {
                    data[..k].copy_from_slice(&d[..k]);
                }
                let p = LZ4F_preferences_t::default();
                let cap = (c.frame_bound)(n, &p);
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let mut cc: *mut c_void = std::ptr::null_mut();
                let mut rc_: *mut c_void = std::ptr::null_mut();
                (c.create_cctx)(&mut cc, LZ4F_VERSION);
                (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                let x = (c.frame_using_cdict)(
                    cc,
                    cb.as_mut_ptr() as *mut c_void,
                    cap,
                    data.as_ptr() as *const c_void,
                    n,
                    a,
                    &p,
                );
                let y = (r.frame_using_cdict)(
                    rc_,
                    rb.as_mut_ptr() as *mut c_void,
                    cap,
                    data.as_ptr() as *const c_void,
                    n,
                    b,
                    &p,
                );
                same_lz4f!(c, r, x, y, "CDict truncation ds={}", ds);
                assert_bytes_eq("CDict truncation bytes", &cb[..x.min(cap)], &rb[..y.min(cap)]);
                (c.free_cctx)(cc);
                (r.free_cctx)(rc_);
            }
            (c.free_cdict)(a);
            (r.free_cdict)(b);
        }
    }
    unsafe {
        let a = (c.create_cdict)(std::ptr::null(), 0);
        let b = (r.create_cdict)(std::ptr::null(), 0);
        assert_eq!(a.is_null(), b.is_null(), "createCDict(NULL, 0)");
        (c.free_cdict)(a);
        (r.free_cdict)(b);
    }
}

/// Generic boundary: `LZ4F_decompress` with a zero-capacity destination and
/// a NULL destination (allowed when *dstSizePtr == 0).
#[test]
fn errf_decompress_zero_dst() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x7008);
    let n = 5000usize;
    let data = gen(Shape::Text, n, &mut rng);
    let p = LZ4F_preferences_t::default();
    let cap = unsafe { (c.frame_bound)(n, &p) };
    let mut frame = vec![0u8; cap];
    let flen = unsafe {
        (c.compress_frame)(frame.as_mut_ptr() as *mut c_void, cap, data.as_ptr() as *const c_void, n, &p)
    };
    let frame = &frame[..flen];
    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        (c.create_dctx)(&mut cd, LZ4F_VERSION);
        (r.create_dctx)(&mut rd, LZ4F_VERSION);
        // NULL dst with *dstSizePtr == 0 is the internal contract used by
        // LZ4F_getFrameInfo (lz4frame.c:1496)
        let mut cds = 0usize;
        let mut rds = 0usize;
        let mut css = flen;
        let mut rss = flen;
        let a = (c.decompress)(
            cd,
            std::ptr::null_mut(),
            &mut cds,
            frame.as_ptr() as *const c_void,
            &mut css,
            std::ptr::null(),
        );
        let b = (r.decompress)(
            rd,
            std::ptr::null_mut(),
            &mut rds,
            frame.as_ptr() as *const c_void,
            &mut rss,
            std::ptr::null(),
        );
        same_lz4f!(c, r, a, b, "decompress(NULL dst, 0)");
        assert_eq!(cds, rds);
        assert_eq!(css, rss);
        // zero srcSize
        let mut cds = 1024usize;
        let mut rds = 1024usize;
        let mut css = 0usize;
        let mut rss = 0usize;
        let mut co = vec![0u8; 1024];
        let mut ro = vec![0u8; 1024];
        let a = (c.decompress)(
            cd,
            co.as_mut_ptr() as *mut c_void,
            &mut cds,
            frame.as_ptr() as *const c_void,
            &mut css,
            std::ptr::null(),
        );
        let b = (r.decompress)(
            rd,
            ro.as_mut_ptr() as *mut c_void,
            &mut rds,
            frame.as_ptr() as *const c_void,
            &mut rss,
            std::ptr::null(),
        );
        same_lz4f!(c, r, a, b, "decompress(srcSize=0)");
        assert_eq!(cds, rds);
        assert_eq!(css, rss);
        assert_eq!((c.free_dctx)(cd), (r.free_dctx)(rd));
    }
}
