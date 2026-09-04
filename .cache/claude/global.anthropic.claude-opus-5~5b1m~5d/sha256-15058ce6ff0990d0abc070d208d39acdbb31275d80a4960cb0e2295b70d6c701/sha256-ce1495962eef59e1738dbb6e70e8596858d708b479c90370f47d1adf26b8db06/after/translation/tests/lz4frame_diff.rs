//! Phase B differential tests for the LZ4 Frame API (lz4frame.c): one-shot and
//! streaming compression across the full preferences cross-product, dictionary
//! compression (CDict), uncompressed blocks, and frame decoding.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressFrameBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCompressBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompressBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnCompressBeginUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressBeginUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressBeginInternal = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnCreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
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
type FnResetDctx = unsafe extern "C" fn(*mut c_void);
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
type FnCompressFrameUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    pub alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub calloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

type FnCreateCctxAdvanced = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnCreateDctxAdvanced = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnCreateCDictAdvanced =
    unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;

pub struct Api {
    pub compress_frame: FnCompressFrame,
    pub compress_frame_bound: FnCompressFrameBound,
    pub compress_bound: FnCompressBound,
    pub create_cctx: FnCreateCctx,
    pub free_cctx: FnFreeCctx,
    pub compress_begin: FnCompressBegin,
    pub compress_begin_using_dict: FnCompressBeginUsingDict,
    pub compress_begin_using_dict_once: FnCompressBeginUsingDict,
    pub compress_begin_using_cdict: FnCompressBeginUsingCDict,
    pub compress_begin_internal: FnCompressBeginInternal,
    pub compress_update: FnCompressUpdate,
    pub uncompressed_update: FnCompressUpdate,
    pub flush: FnFlush,
    pub compress_end: FnFlush,
    pub create_dctx: FnCreateDctx,
    pub free_dctx: FnFreeDctx,
    pub header_size: FnHeaderSize,
    pub get_frame_info: FnGetFrameInfo,
    pub decompress: FnDecompress,
    pub decompress_using_dict: FnDecompressUsingDict,
    pub reset_dctx: FnResetDctx,
    pub get_block_size: FnGetBlockSize,
    pub create_cdict: FnCreateCDict,
    pub free_cdict: FnFreeCDict,
    pub compress_frame_using_cdict: FnCompressFrameUsingCDict,
    pub is_error: FnIsError,
    pub get_error_name: FnGetErrorName,
    pub get_error_code: FnGetErrorCode,
    pub get_version: FnUIntVoid,
    pub level_max: FnIntVoid,
    pub create_cctx_advanced: FnCreateCctxAdvanced,
    pub create_dctx_advanced: FnCreateDctxAdvanced,
    pub create_cdict_advanced: FnCreateCDictAdvanced,
}

pub fn bind(l: &Lib) -> Api {
    Api {
        compress_frame: l.sym("LZ4F_compressFrame"),
        compress_frame_bound: l.sym("LZ4F_compressFrameBound"),
        compress_bound: l.sym("LZ4F_compressBound"),
        create_cctx: l.sym("LZ4F_createCompressionContext"),
        free_cctx: l.sym("LZ4F_freeCompressionContext"),
        compress_begin: l.sym("LZ4F_compressBegin"),
        compress_begin_using_dict: l.sym("LZ4F_compressBegin_usingDict"),
        compress_begin_using_dict_once: l.sym("LZ4F_compressBegin_usingDictOnce"),
        compress_begin_using_cdict: l.sym("LZ4F_compressBegin_usingCDict"),
        compress_begin_internal: l.sym("LZ4F_compressBegin_internal"),
        compress_update: l.sym("LZ4F_compressUpdate"),
        uncompressed_update: l.sym("LZ4F_uncompressedUpdate"),
        flush: l.sym("LZ4F_flush"),
        compress_end: l.sym("LZ4F_compressEnd"),
        create_dctx: l.sym("LZ4F_createDecompressionContext"),
        free_dctx: l.sym("LZ4F_freeDecompressionContext"),
        header_size: l.sym("LZ4F_headerSize"),
        get_frame_info: l.sym("LZ4F_getFrameInfo"),
        decompress: l.sym("LZ4F_decompress"),
        decompress_using_dict: l.sym("LZ4F_decompress_usingDict"),
        reset_dctx: l.sym("LZ4F_resetDecompressionContext"),
        get_block_size: l.sym("LZ4F_getBlockSize"),
        create_cdict: l.sym("LZ4F_createCDict"),
        free_cdict: l.sym("LZ4F_freeCDict"),
        compress_frame_using_cdict: l.sym("LZ4F_compressFrame_usingCDict"),
        is_error: l.sym("LZ4F_isError"),
        get_error_name: l.sym("LZ4F_getErrorName"),
        get_error_code: l.sym("LZ4F_getErrorCode"),
        get_version: l.sym("LZ4F_getVersion"),
        level_max: l.sym("LZ4F_compressionLevel_max"),
        create_cctx_advanced: l.sym("LZ4F_createCompressionContext_advanced"),
        create_dctx_advanced: l.sym("LZ4F_createDecompressionContext_advanced"),
        create_cdict_advanced: l.sym("LZ4F_createCDict_advanced"),
    }
}

pub fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

pub const BSIDS: [c_int; 5] = [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB];
pub const LEVELS: [c_int; 10] = [0, 1, 2, 3, 6, 9, 10, 11, 12, -1];

/// Decompress a whole frame with one library, feeding src in `src_chunk` sized
/// pieces and accepting at most `dst_chunk` bytes of output per call.
/// Returns (rc_of_last_call, decoded bytes) or Err(error_code).
#[allow(clippy::too_many_arguments)]
pub unsafe fn decode_frame(
    api: &Api,
    dctx: *mut c_void,
    frame: &[u8],
    src_chunk: usize,
    dst_chunk: usize,
    opts: *const LZ4F_decompressOptions_t,
) -> Result<Vec<u8>, usize> {
    let mut out = Vec::new();
    let mut sp = 0usize;
    let mut dbuf = vec![0u8; dst_chunk.max(1)];
    let mut guard = 0u32;
    loop {
        guard += 1;
        if guard > 5_000_000 {
            panic!("decode_frame did not terminate");
        }
        let mut ssz = (frame.len() - sp).min(src_chunk.max(1));
        let mut dsz = dbuf.len();
        let rc = (api.decompress)(
            dctx,
            dbuf.as_mut_ptr() as *mut c_void,
            &mut dsz,
            frame.as_ptr().add(sp) as *const c_void,
            &mut ssz,
            opts,
        );
        if (api.is_error)(rc) != 0 {
            return Err(rc);
        }
        out.extend_from_slice(&dbuf[..dsz]);
        sp += ssz;
        if rc == 0 {
            // frame completed
            return Ok(out);
        }
        if ssz == 0 && dsz == 0 {
            if sp >= frame.len() {
                // ran out of input before the frame ended
                return Ok(out);
            }
            panic!("decode_frame stalled at sp={} of {}", sp, frame.len());
        }
    }
}

fn prefs_desc(p: &LZ4F_preferences_t) -> String {
    format!(
        "bsid={} bmode={} cchk={} bchk={} ftype={} csize={} dictID={} lvl={} autoFlush={} favor={}",
        p.frameInfo.blockSizeID,
        p.frameInfo.blockMode,
        p.frameInfo.contentChecksumFlag,
        p.frameInfo.blockChecksumFlag,
        p.frameInfo.frameType,
        p.frameInfo.contentSize,
        p.frameInfo.dictID,
        p.compressionLevel,
        p.autoFlush,
        p.favorDecSpeed
    )
}

// --- CONFIGS: scalar accessors ----------------------------------------------
#[test]
fn frame_scalar_accessors() {
    let (c, r) = pair();
    unsafe {
        assert_eq!((c.get_version)(), (r.get_version)());
        assert_eq!((c.level_max)(), (r.level_max)());
        // LZ4F_getBlockSize over valid AND out-of-range enum values
        for id in -3i32..=12 {
            let a = (c.get_block_size)(id);
            let b = (r.get_block_size)(id);
            assert_eq!(a, b, "LZ4F_getBlockSize({}) -> {} vs {}", id, fmt_lz4f(a), fmt_lz4f(b));
        }
        for id in [i32::MIN, i32::MAX, 1000, -1000] {
            assert_eq!(
                (c.get_block_size)(id),
                (r.get_block_size)(id),
                "LZ4F_getBlockSize({})",
                id
            );
        }
        // isError / getErrorName / getErrorCode across the whole error range
        for code in 0..=(LZ4F_ERROR_NAMES.len() + 3) {
            let v = (0usize).wrapping_sub(code);
            assert_eq!((c.is_error)(v), (r.is_error)(v), "LZ4F_isError(-{})", code);
            assert_eq!(
                cstr((c.get_error_name)(v)),
                cstr((r.get_error_name)(v)),
                "LZ4F_getErrorName(-{})",
                code
            );
            assert_eq!(
                (c.get_error_code)(v),
                (r.get_error_code)(v),
                "LZ4F_getErrorCode(-{})",
                code
            );
        }
        for v in [0usize, 1, 100, usize::MAX / 2, usize::MAX] {
            assert_eq!((c.is_error)(v), (r.is_error)(v), "LZ4F_isError({})", v);
            assert_eq!(
                cstr((c.get_error_name)(v)),
                cstr((r.get_error_name)(v)),
                "LZ4F_getErrorName({})",
                v
            );
            assert_eq!((c.get_error_code)(v), (r.get_error_code)(v), "LZ4F_getErrorCode({})", v);
        }
    }
}

// --- CONFIGS: compressBound / compressFrameBound over the whole matrix -------
#[test]
fn frame_bounds() {
    let (c, r) = pair();
    let sizes = [
        0usize, 1, 2, 63, 64, 65535, 65536, 65537, 262144, 262145, 1 << 20, (1 << 20) + 1,
        1 << 22, (1 << 22) + 1, 10 << 20,
    ];
    for &bsid in &BSIDS {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &cchk in &[0, 1] {
                for &bchk in &[0, 1] {
                    for &af in &[0u32, 1] {
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = bmode;
                        p.frameInfo.contentChecksumFlag = cchk;
                        p.frameInfo.blockChecksumFlag = bchk;
                        p.autoFlush = af;
                        for &n in &sizes {
                            unsafe {
                                assert_eq!(
                                    (c.compress_bound)(n, &p),
                                    (r.compress_bound)(n, &p),
                                    "LZ4F_compressBound({}, {})",
                                    n,
                                    prefs_desc(&p)
                                );
                                assert_eq!(
                                    (c.compress_frame_bound)(n, &p),
                                    (r.compress_frame_bound)(n, &p),
                                    "LZ4F_compressFrameBound({}, {})",
                                    n,
                                    prefs_desc(&p)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // NULL preferences
    for &n in &sizes {
        unsafe {
            assert_eq!(
                (c.compress_bound)(n, std::ptr::null()),
                (r.compress_bound)(n, std::ptr::null()),
                "LZ4F_compressBound({}, NULL)",
                n
            );
            assert_eq!(
                (c.compress_frame_bound)(n, std::ptr::null()),
                (r.compress_frame_bound)(n, std::ptr::null()),
                "LZ4F_compressFrameBound({}, NULL)",
                n
            );
        }
    }
}

// --- CONFIGS: LZ4F_compressFrame over the full preferences cross-product -----
#[test]
fn frame_compress_frame_matrix() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4001);
    let sizes = [0usize, 1, 13, 100, 65535, 65536, 65537, 200_000];
    for &bsid in &BSIDS {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &cchk in &[0, 1] {
                for &bchk in &[0, 1] {
                    for &lvl in &[0i32, 1, 3, 9, 12] {
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = bmode;
                        p.frameInfo.contentChecksumFlag = cchk;
                        p.frameInfo.blockChecksumFlag = bchk;
                        p.compressionLevel = lvl;
                        for &n in &sizes {
                            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                            let data = gen(shape, n, &mut rng);
                            let cap = unsafe { (c.compress_frame_bound)(n, &p) };
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
                            assert_eq!(
                                a,
                                b,
                                "compressFrame rc n={} {} -> {} vs {}",
                                n,
                                prefs_desc(&p),
                                fmt_lz4f(a),
                                fmt_lz4f(b)
                            );
                            assert!(
                                unsafe { (c.is_error)(a) } == 0,
                                "compressFrame failed: {}",
                                fmt_lz4f(a)
                            );
                            assert_bytes_eq(
                                &format!("compressFrame n={} {}", n, prefs_desc(&p)),
                                &cb[..a],
                                &rb[..b],
                            );
                            // cross round-trip: each library decodes the other's frame
                            unsafe {
                                let mut cd: *mut c_void = std::ptr::null_mut();
                                let mut rd: *mut c_void = std::ptr::null_mut();
                                assert_eq!((c.create_dctx)(&mut cd, LZ4F_VERSION), 0);
                                assert_eq!((r.create_dctx)(&mut rd, LZ4F_VERSION), 0);
                                let x = decode_frame(&c, cd, &rb[..b], usize::MAX, 1 << 20, std::ptr::null())
                                    .expect("C decode of Rust frame");
                                let y = decode_frame(&r, rd, &cb[..a], usize::MAX, 1 << 20, std::ptr::null())
                                    .expect("Rust decode of C frame");
                                assert_bytes_eq("frame cross round-trip (C)", &x, &data);
                                assert_bytes_eq("frame cross round-trip (Rust)", &y, &data);
                                (c.free_dctx)(cd);
                                (r.free_dctx)(rd);
                            }
                        }
                    }
                }
            }
        }
    }
    // NULL preferences
    for &n in &[0usize, 1, 13, 100_000] {
        let data = gen(Shape::Text, n, &mut rng);
        let cap = unsafe { (c.compress_frame_bound)(n, std::ptr::null()) };
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let a = unsafe {
            (c.compress_frame)(
                cb.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                n,
                std::ptr::null(),
            )
        };
        let b = unsafe {
            (r.compress_frame)(
                rb.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                n,
                std::ptr::null(),
            )
        };
        assert_eq!(a, b, "compressFrame(NULL prefs) rc n={}", n);
        assert_bytes_eq("compressFrame(NULL prefs)", &cb[..a], &rb[..b]);
    }
}

// --- CONFIGS: contentSize / dictID / frameType / favorDecSpeed --------------
#[test]
fn frame_header_fields() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4002);
    for &ftype in &[LZ4F_FRAME, LZ4F_SKIPPABLE_FRAME] {
        for &dict_id in &[0u32, 1, 0xDEAD_BEEF, u32::MAX] {
            for &favor in &[0u32, 1] {
                for &n in &[0usize, 1, 1000, 100_000] {
                    for &known_size in &[false, true] {
                        let data = gen(Shape::Text, n, &mut rng);
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.frameType = ftype;
                        p.frameInfo.dictID = dict_id;
                        p.frameInfo.contentSize = if known_size { n as u64 } else { 0 };
                        p.favorDecSpeed = favor;
                        p.compressionLevel = if favor == 1 { 12 } else { 1 };
                        let cap = unsafe { (c.compress_frame_bound)(n, &p) };
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
                        assert_eq!(a, b, "header fields rc {}", prefs_desc(&p));
                        assert_bytes_eq(&format!("header fields {}", prefs_desc(&p)), &cb[..a], &rb[..b]);
                        if unsafe { (c.is_error)(a) } != 0 {
                            continue;
                        }
                        // LZ4F_headerSize and LZ4F_getFrameInfo must agree
                        for sz in [0usize, 1, 4, 5, 6, 7, 8, 19, a.min(cap)] {
                            let hs_c = unsafe { (c.header_size)(cb.as_ptr() as *const c_void, sz) };
                            let hs_r = unsafe { (r.header_size)(rb.as_ptr() as *const c_void, sz) };
                            assert_eq!(
                                hs_c,
                                hs_r,
                                "LZ4F_headerSize(srcSize={}) {} -> {} vs {}",
                                sz,
                                prefs_desc(&p),
                                fmt_lz4f(hs_c),
                                fmt_lz4f(hs_r)
                            );
                        }
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let mut ci = LZ4F_frameInfo_t::default();
                            let mut ri = LZ4F_frameInfo_t::default();
                            let mut cs = a;
                            let mut rs = b;
                            let x = (c.get_frame_info)(cd, &mut ci, cb.as_ptr() as *const c_void, &mut cs);
                            let y = (r.get_frame_info)(rd, &mut ri, rb.as_ptr() as *const c_void, &mut rs);
                            assert_eq!(
                                x,
                                y,
                                "getFrameInfo rc {} -> {} vs {}",
                                prefs_desc(&p),
                                fmt_lz4f(x),
                                fmt_lz4f(y)
                            );
                            assert_eq!(cs, rs, "getFrameInfo consumed {}", prefs_desc(&p));
                            assert_eq!(ci, ri, "getFrameInfo frameInfo {}", prefs_desc(&p));
                            (c.free_dctx)(cd);
                            (r.free_dctx)(rd);
                        }
                    }
                }
            }
        }
    }
}

// --- CONFIGS: streaming compression, full option cross-product (randomized) --
#[test]
fn frame_streaming_random_configs() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4003);
    for iter in 0..400 {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = BSIDS[rng.below(BSIDS.len())];
        p.frameInfo.blockMode = (rng.below(2)) as c_int;
        p.frameInfo.contentChecksumFlag = (rng.below(2)) as c_int;
        p.frameInfo.blockChecksumFlag = (rng.below(2)) as c_int;
        p.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() };
        p.compressionLevel = LEVELS[rng.below(LEVELS.len())];
        p.autoFlush = (rng.below(2)) as c_uint;
        p.favorDecSpeed = (rng.below(2)) as c_uint;
        let stable_src = (rng.below(2)) as c_uint;
        let copts = LZ4F_compressOptions_t {
            stableSrc: stable_src,
            reserved: [0; 3],
        };
        let copts_ptr: *const LZ4F_compressOptions_t = if rng.below(4) == 0 {
            std::ptr::null()
        } else {
            &copts
        };

        let total = rng.range(0, 300_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(shape, total, &mut rng);
        if rng.below(2) == 0 {
            p.frameInfo.contentSize = total as u64;
        }

        // random chunking of the input, plus occasional explicit flushes
        let maxchunk = [1usize, 17, 300, 4096, 70_000, 300_000][rng.below(6)];
        let mut chunks = Vec::new();
        let mut left = total;
        while left > 0 {
            let n = rng.range(1, left.min(maxchunk) + 1);
            chunks.push(n);
            left -= n;
        }

        unsafe {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc_: *mut c_void = std::ptr::null_mut();
            assert_eq!((c.create_cctx)(&mut cc, LZ4F_VERSION), 0);
            assert_eq!((r.create_cctx)(&mut rc_, LZ4F_VERSION), 0);

            let mut cframe: Vec<u8> = Vec::new();
            let mut rframe: Vec<u8> = Vec::new();

            let hdr_cap = LZ4F_HEADER_SIZE_MAX;
            let mut chb = vec![0u8; hdr_cap];
            let mut rhb = vec![0u8; hdr_cap];
            let a = (c.compress_begin)(cc, chb.as_mut_ptr() as *mut c_void, hdr_cap, &p);
            let b = (r.compress_begin)(rc_, rhb.as_mut_ptr() as *mut c_void, hdr_cap, &p);
            assert_eq!(
                a,
                b,
                "iter={} compressBegin rc {} -> {} vs {}",
                iter,
                prefs_desc(&p),
                fmt_lz4f(a),
                fmt_lz4f(b)
            );
            assert!((c.is_error)(a) == 0, "compressBegin failed {}", fmt_lz4f(a));
            assert_bytes_eq("compressBegin header", &chb[..a], &rhb[..b]);
            cframe.extend_from_slice(&chb[..a]);
            rframe.extend_from_slice(&rhb[..b]);

            // Hoist the output buffers: LZ4F_compressBound() can be several MB
            // for large blockSizeIDs, so allocating per chunk is prohibitive.
            let maxcap = (c.compress_bound)(chunks.iter().copied().max().unwrap_or(0), &p)
                .max((c.compress_bound)(0, &p));
            let mut cbuf = vec![0u8; maxcap.max(1)];
            let mut rbuf = vec![0u8; maxcap.max(1)];
            let mut off = 0usize;
            for (ci, &n) in chunks.iter().enumerate() {
                let cap = (c.compress_bound)(n, &p);
                let cb = &mut cbuf[..cap];
                let rb = &mut rbuf[..cap];
                let a = (c.compress_update)(
                    cc,
                    cb.as_mut_ptr() as *mut c_void,
                    cap,
                    data.as_ptr().add(off) as *const c_void,
                    n,
                    copts_ptr,
                );
                let b = (r.compress_update)(
                    rc_,
                    rb.as_mut_ptr() as *mut c_void,
                    cap,
                    data.as_ptr().add(off) as *const c_void,
                    n,
                    copts_ptr,
                );
                assert_eq!(
                    a,
                    b,
                    "iter={} chunk={} compressUpdate rc n={} {} -> {} vs {}",
                    iter,
                    ci,
                    n,
                    prefs_desc(&p),
                    fmt_lz4f(a),
                    fmt_lz4f(b)
                );
                assert!((c.is_error)(a) == 0, "compressUpdate failed {}", fmt_lz4f(a));
                assert_bytes_eq(
                    &format!("iter={} chunk={} compressUpdate", iter, ci),
                    &cb[..a],
                    &rb[..b],
                );
                cframe.extend_from_slice(&cb[..a]);
                rframe.extend_from_slice(&rb[..b]);
                off += n;
                // occasional explicit flush
                if rng.below(6) == 0 {
                    let fcap = (c.compress_bound)(0, &p);
                    let cb = &mut cbuf[..fcap];
                    let rb = &mut rbuf[..fcap];
                    let a = (c.flush)(cc, cb.as_mut_ptr() as *mut c_void, fcap, copts_ptr);
                    let b = (r.flush)(rc_, rb.as_mut_ptr() as *mut c_void, fcap, copts_ptr);
                    assert_eq!(
                        a,
                        b,
                        "iter={} chunk={} flush rc -> {} vs {}",
                        iter,
                        ci,
                        fmt_lz4f(a),
                        fmt_lz4f(b)
                    );
                    assert_bytes_eq("flush", &cb[..a], &rb[..b]);
                    cframe.extend_from_slice(&cb[..a]);
                    rframe.extend_from_slice(&rb[..b]);
                }
            }
            let ecap = (c.compress_bound)(0, &p);
            let cb = &mut cbuf[..ecap];
            let rb = &mut rbuf[..ecap];
            let a = (c.compress_end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, copts_ptr);
            let b = (r.compress_end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, copts_ptr);
            assert_eq!(
                a,
                b,
                "iter={} compressEnd rc {} -> {} vs {}",
                iter,
                prefs_desc(&p),
                fmt_lz4f(a),
                fmt_lz4f(b)
            );
            assert!((c.is_error)(a) == 0, "compressEnd failed {}", fmt_lz4f(a));
            assert_bytes_eq("compressEnd", &cb[..a], &rb[..b]);
            cframe.extend_from_slice(&cb[..a]);
            rframe.extend_from_slice(&rb[..b]);
            assert_bytes_eq(
                &format!("iter={} whole frame {}", iter, prefs_desc(&p)),
                &cframe,
                &rframe,
            );

            (c.free_cctx)(cc);
            (r.free_cctx)(rc_);

            // decode with both, using random src/dst fragmentation.
            // Byte-at-a-time fragmentation is quadratic in the frame size, so it
            // is only applied to the smaller frames (the code path it exercises
            // does not depend on the total size).
            let tiny_ok = cframe.len() <= 40_000;
            let src_chunk = if tiny_ok {
                [1usize, 7, 100, 4096, usize::MAX][rng.below(5)]
            } else {
                [100usize, 4096, usize::MAX][rng.below(3)]
            };
            let dst_chunk = if tiny_ok {
                [1usize, 13, 1000, 65536, 1 << 21][rng.below(5)]
            } else {
                [1000usize, 65536, 1 << 21][rng.below(3)]
            };
            let dopts = LZ4F_decompressOptions_t {
                stableDst: 0,
                skipChecksums: (rng.below(2)) as c_uint,
                reserved1: 0,
                reserved0: 0,
            };
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            (c.create_dctx)(&mut cd, LZ4F_VERSION);
            (r.create_dctx)(&mut rd, LZ4F_VERSION);
            let x = decode_frame(&c, cd, &cframe, src_chunk, dst_chunk, &dopts);
            let y = decode_frame(&r, rd, &rframe, src_chunk, dst_chunk, &dopts);
            match (&x, &y) {
                (Ok(a), Ok(b)) => {
                    assert_bytes_eq(
                        &format!("iter={} decode {}", iter, prefs_desc(&p)),
                        a,
                        b,
                    );
                    assert_bytes_eq(&format!("iter={} decode content", iter), a, &data);
                }
                (Err(a), Err(b)) => assert_eq!(a, b, "iter={} decode error mismatch", iter),
                _ => panic!(
                    "iter={} decode divergence: C={:?} Rust={:?}",
                    iter,
                    x.as_ref().map(|v| v.len()).map_err(|e| fmt_lz4f(*e)),
                    y.as_ref().map(|v| v.len()).map_err(|e| fmt_lz4f(*e))
                ),
            }
            (c.free_dctx)(cd);
            (r.free_dctx)(rd);
        }
    }
}

// --- CONFIGS: LZ4F_uncompressedUpdate ---------------------------------------
#[test]
fn frame_uncompressed_update() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4004);
    for &bsid in &BSIDS {
        for &cchk in &[0, 1] {
            for &bchk in &[0, 1] {
                for &af in &[0u32, 1] {
                    for &mix in &[false, true] {
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
                        p.frameInfo.contentChecksumFlag = cchk;
                        p.frameInfo.blockChecksumFlag = bchk;
                        p.autoFlush = af;
                        let total = 120_000usize;
                        let data = gen(Shape::Text, total, &mut rng);
                        unsafe {
                            let mut cc: *mut c_void = std::ptr::null_mut();
                            let mut rc_: *mut c_void = std::ptr::null_mut();
                            (c.create_cctx)(&mut cc, LZ4F_VERSION);
                            (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                            let mut chb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                            let mut rhb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                            let a = (c.compress_begin)(
                                cc,
                                chb.as_mut_ptr() as *mut c_void,
                                LZ4F_HEADER_SIZE_MAX,
                                &p,
                            );
                            let b = (r.compress_begin)(
                                rc_,
                                rhb.as_mut_ptr() as *mut c_void,
                                LZ4F_HEADER_SIZE_MAX,
                                &p,
                            );
                            assert_eq!(a, b);
                            let mut cframe = chb[..a].to_vec();
                            let mut rframe = rhb[..b].to_vec();
                            let mut off = 0usize;
                            let mut i = 0;
                            while off < total {
                                let n = rng.range(1, 40_000).min(total - off);
                                // dst must fit the raw block for uncompressedUpdate
                                let cap = (c.compress_bound)(n, &p).max(n + 64);
                                let mut cb = vec![0u8; cap];
                                let mut rb = vec![0u8; cap];
                                let use_raw = !mix || (i % 2 == 0);
                                let (a, b) = if use_raw {
                                    (
                                        (c.uncompressed_update)(
                                            cc,
                                            cb.as_mut_ptr() as *mut c_void,
                                            cap,
                                            data.as_ptr().add(off) as *const c_void,
                                            n,
                                            std::ptr::null(),
                                        ),
                                        (r.uncompressed_update)(
                                            rc_,
                                            rb.as_mut_ptr() as *mut c_void,
                                            cap,
                                            data.as_ptr().add(off) as *const c_void,
                                            n,
                                            std::ptr::null(),
                                        ),
                                    )
                                } else {
                                    (
                                        (c.compress_update)(
                                            cc,
                                            cb.as_mut_ptr() as *mut c_void,
                                            cap,
                                            data.as_ptr().add(off) as *const c_void,
                                            n,
                                            std::ptr::null(),
                                        ),
                                        (r.compress_update)(
                                            rc_,
                                            rb.as_mut_ptr() as *mut c_void,
                                            cap,
                                            data.as_ptr().add(off) as *const c_void,
                                            n,
                                            std::ptr::null(),
                                        ),
                                    )
                                };
                                assert_eq!(
                                    a,
                                    b,
                                    "uncompressedUpdate(raw={}) rc n={} {} -> {} vs {}",
                                    use_raw,
                                    n,
                                    prefs_desc(&p),
                                    fmt_lz4f(a),
                                    fmt_lz4f(b)
                                );
                                assert_bytes_eq(
                                    &format!("uncompressedUpdate(raw={}) n={}", use_raw, n),
                                    &cb[..a.min(cap)],
                                    &rb[..b.min(cap)],
                                );
                                if (c.is_error)(a) != 0 {
                                    break;
                                }
                                cframe.extend_from_slice(&cb[..a]);
                                rframe.extend_from_slice(&rb[..b]);
                                off += n;
                                i += 1;
                            }
                            let ecap = (c.compress_bound)(0, &p);
                            let mut cb = vec![0u8; ecap];
                            let mut rb = vec![0u8; ecap];
                            let a = (c.compress_end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                            let b = (r.compress_end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                            assert_eq!(a, b, "compressEnd after uncompressedUpdate");
                            cframe.extend_from_slice(&cb[..a]);
                            rframe.extend_from_slice(&rb[..b]);
                            assert_bytes_eq("uncompressedUpdate whole frame", &cframe, &rframe);
                            (c.free_cctx)(cc);
                            (r.free_cctx)(rc_);
                            // round-trip
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let x = decode_frame(&c, cd, &cframe, 4096, 8192, std::ptr::null()).unwrap();
                            let y = decode_frame(&r, rd, &rframe, 4096, 8192, std::ptr::null()).unwrap();
                            assert_bytes_eq("uncompressedUpdate decode", &x, &y);
                            assert_bytes_eq("uncompressedUpdate content", &x, &data[..off]);
                            (c.free_dctx)(cd);
                            (r.free_dctx)(rd);
                        }
                    }
                }
            }
        }
    }
}

// --- CONFIGS: dictionary compression (CDict + usingDict) --------------------
#[test]
fn frame_dictionary() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4005);
    for &ds in &[0usize, 1, 13, 1000, 65535, 65536, 65537, 200_000] {
        let dict = gen(Shape::Text, ds, &mut rng);
        unsafe {
            let ccd = (c.create_cdict)(dict.as_ptr() as *const c_void, ds);
            let rcd = (r.create_cdict)(dict.as_ptr() as *const c_void, ds);
            assert_eq!(ccd.is_null(), rcd.is_null(), "createCDict(ds={})", ds);
            for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                for &lvl in &[0i32, 1, 3, 9, 12] {
                    for &n in &[0usize, 1, 13, 1000, 100_000] {
                        let mut data = gen(Shape::Text, n, &mut rng);
                        let k = n.min(ds);
                        if k > 0 {
                            data[..k].copy_from_slice(&dict[..k]);
                        }
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockMode = bmode;
                        p.compressionLevel = lvl;
                        p.frameInfo.dictID = 0x1234;
                        let cap = (c.compress_frame_bound)(n, &p);

                        // 1) LZ4F_compressFrame_usingCDict
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let a = (c.compress_frame_using_cdict)(
                            cc,
                            cb.as_mut_ptr() as *mut c_void,
                            cap,
                            data.as_ptr() as *const c_void,
                            n,
                            ccd,
                            &p,
                        );
                        let b = (r.compress_frame_using_cdict)(
                            rc_,
                            rb.as_mut_ptr() as *mut c_void,
                            cap,
                            data.as_ptr() as *const c_void,
                            n,
                            rcd,
                            &p,
                        );
                        assert_eq!(
                            a,
                            b,
                            "compressFrame_usingCDict rc ds={} n={} {} -> {} vs {}",
                            ds,
                            n,
                            prefs_desc(&p),
                            fmt_lz4f(a),
                            fmt_lz4f(b)
                        );
                        assert_bytes_eq(
                            &format!("compressFrame_usingCDict ds={} n={} {}", ds, n, prefs_desc(&p)),
                            &cb[..a.min(cap)],
                            &rb[..b.min(cap)],
                        );
                        if (c.is_error)(a) == 0 {
                            // round-trip via decompress_usingDict
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let mut co = vec![0u8; n + 4096];
                            let mut ro = vec![0u8; n + 4096];
                            let mut cdst = co.len();
                            let mut rdst = ro.len();
                            let mut csrc = a;
                            let mut rsrc = b;
                            let x = (c.decompress_using_dict)(
                                cd,
                                co.as_mut_ptr() as *mut c_void,
                                &mut cdst,
                                cb.as_ptr() as *const c_void,
                                &mut csrc,
                                dict.as_ptr() as *const c_void,
                                ds,
                                std::ptr::null(),
                            );
                            let y = (r.decompress_using_dict)(
                                rd,
                                ro.as_mut_ptr() as *mut c_void,
                                &mut rdst,
                                rb.as_ptr() as *const c_void,
                                &mut rsrc,
                                dict.as_ptr() as *const c_void,
                                ds,
                                std::ptr::null(),
                            );
                            assert_eq!(
                                x,
                                y,
                                "decompress_usingDict rc ds={} n={} -> {} vs {}",
                                ds,
                                n,
                                fmt_lz4f(x),
                                fmt_lz4f(y)
                            );
                            assert_eq!(cdst, rdst, "decompress_usingDict dstSize ds={} n={}", ds, n);
                            assert_eq!(csrc, rsrc, "decompress_usingDict srcSize ds={} n={}", ds, n);
                            assert_bytes_eq("decompress_usingDict", &co[..cdst], &ro[..rdst]);
                            if (c.is_error)(x) == 0 && cdst == n {
                                assert_bytes_eq("decompress_usingDict content", &co[..n], &data);
                            }
                            (c.free_dctx)(cd);
                            (r.free_dctx)(rd);
                        }
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);

                        // 2) streaming with compressBegin_usingCDict / usingDict /
                        //    usingDictOnce / compressBegin_internal
                        for mode in 0..4 {
                            let mut cc: *mut c_void = std::ptr::null_mut();
                            let mut rc_: *mut c_void = std::ptr::null_mut();
                            (c.create_cctx)(&mut cc, LZ4F_VERSION);
                            (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                            let mut chb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                            let mut rhb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                            let dp = dict.as_ptr() as *const c_void;
                            let (a, b) = match mode {
                                0 => (
                                    (c.compress_begin_using_cdict)(
                                        cc,
                                        chb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        ccd,
                                        &p,
                                    ),
                                    (r.compress_begin_using_cdict)(
                                        rc_,
                                        rhb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        rcd,
                                        &p,
                                    ),
                                ),
                                1 => (
                                    (c.compress_begin_using_dict)(
                                        cc,
                                        chb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        dp,
                                        ds,
                                        &p,
                                    ),
                                    (r.compress_begin_using_dict)(
                                        rc_,
                                        rhb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        dp,
                                        ds,
                                        &p,
                                    ),
                                ),
                                2 => (
                                    (c.compress_begin_using_dict_once)(
                                        cc,
                                        chb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        dp,
                                        ds,
                                        &p,
                                    ),
                                    (r.compress_begin_using_dict_once)(
                                        rc_,
                                        rhb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        dp,
                                        ds,
                                        &p,
                                    ),
                                ),
                                _ => (
                                    (c.compress_begin_internal)(
                                        cc,
                                        chb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        std::ptr::null(),
                                        0,
                                        ccd,
                                        &p,
                                    ),
                                    (r.compress_begin_internal)(
                                        rc_,
                                        rhb.as_mut_ptr() as *mut c_void,
                                        LZ4F_HEADER_SIZE_MAX,
                                        std::ptr::null(),
                                        0,
                                        rcd,
                                        &p,
                                    ),
                                ),
                            };
                            assert_eq!(
                                a,
                                b,
                                "compressBegin mode={} rc ds={} {} -> {} vs {}",
                                mode,
                                ds,
                                prefs_desc(&p),
                                fmt_lz4f(a),
                                fmt_lz4f(b)
                            );
                            assert_bytes_eq(
                                &format!("compressBegin mode={} header ds={}", mode, ds),
                                &chb[..a.min(LZ4F_HEADER_SIZE_MAX)],
                                &rhb[..b.min(LZ4F_HEADER_SIZE_MAX)],
                            );
                            if (c.is_error)(a) != 0 {
                                (c.free_cctx)(cc);
                                (r.free_cctx)(rc_);
                                continue;
                            }
                            let mut cframe = chb[..a].to_vec();
                            let mut rframe = rhb[..b].to_vec();
                            // feed in two chunks to exercise the dict-once path
                            let mut off = 0usize;
                            while off < n {
                                let m = ((n - off) / 2 + 1).min(n - off);
                                let ucap = (c.compress_bound)(m, &p);
                                let mut cb = vec![0u8; ucap];
                                let mut rb = vec![0u8; ucap];
                                let a = (c.compress_update)(
                                    cc,
                                    cb.as_mut_ptr() as *mut c_void,
                                    ucap,
                                    data.as_ptr().add(off) as *const c_void,
                                    m,
                                    std::ptr::null(),
                                );
                                let b = (r.compress_update)(
                                    rc_,
                                    rb.as_mut_ptr() as *mut c_void,
                                    ucap,
                                    data.as_ptr().add(off) as *const c_void,
                                    m,
                                    std::ptr::null(),
                                );
                                assert_eq!(
                                    a,
                                    b,
                                    "dict streaming update mode={} rc ds={} n={} -> {} vs {}",
                                    mode,
                                    ds,
                                    n,
                                    fmt_lz4f(a),
                                    fmt_lz4f(b)
                                );
                                assert_bytes_eq(
                                    &format!("dict streaming update mode={} ds={}", mode, ds),
                                    &cb[..a.min(ucap)],
                                    &rb[..b.min(ucap)],
                                );
                                cframe.extend_from_slice(&cb[..a]);
                                rframe.extend_from_slice(&rb[..b]);
                                off += m;
                            }
                            let ecap = (c.compress_bound)(0, &p);
                            let mut cb = vec![0u8; ecap];
                            let mut rb = vec![0u8; ecap];
                            let a = (c.compress_end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                            let b = (r.compress_end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                            assert_eq!(a, b, "dict compressEnd mode={} ds={}", mode, ds);
                            cframe.extend_from_slice(&cb[..a]);
                            rframe.extend_from_slice(&rb[..b]);
                            assert_bytes_eq(
                                &format!("dict frame mode={} ds={} n={} {}", mode, ds, n, prefs_desc(&p)),
                                &cframe,
                                &rframe,
                            );
                            (c.free_cctx)(cc);
                            (r.free_cctx)(rc_);
                        }
                    }
                }
            }
            (c.free_cdict)(ccd);
            (r.free_cdict)(rcd);
        }
    }
}

// --- CONFIGS: *_advanced constructors with custom allocators ----------------
unsafe extern "C" fn my_alloc(_op: *mut c_void, size: usize) -> *mut c_void {
    libc_malloc(size)
}
unsafe extern "C" fn my_calloc(_op: *mut c_void, size: usize) -> *mut c_void {
    let p = libc_malloc(size);
    if !p.is_null() {
        std::ptr::write_bytes(p as *mut u8, 0, size);
    }
    p
}
unsafe extern "C" fn my_free(_op: *mut c_void, p: *mut c_void) {
    libc_free(p)
}

// minimal libc bindings (avoids adding the `libc` crate)
extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn frame_advanced_constructors() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4006);
    let default_cmem = LZ4F_CustomMem {
        alloc: None,
        calloc: None,
        free: None,
        opaque: std::ptr::null_mut(),
    };
    let custom_cmem = LZ4F_CustomMem {
        alloc: Some(my_alloc),
        calloc: Some(my_calloc),
        free: Some(my_free),
        opaque: std::ptr::null_mut(),
    };
    let alloc_only_cmem = LZ4F_CustomMem {
        alloc: Some(my_alloc),
        calloc: None,
        free: Some(my_free),
        opaque: std::ptr::null_mut(),
    };
    for (name, cmem) in [
        ("defaultCMem", default_cmem),
        ("customCMem", custom_cmem),
        ("allocOnlyCMem", alloc_only_cmem),
    ] {
        for &version in &[LZ4F_VERSION, 0, 99, 101, u32::MAX] {
            unsafe {
                let cc = (c.create_cctx_advanced)(cmem, version);
                let rc_ = (r.create_cctx_advanced)(cmem, version);
                assert_eq!(
                    cc.is_null(),
                    rc_.is_null(),
                    "createCompressionContext_advanced({}, v={})",
                    name,
                    version
                );
                let cd = (c.create_dctx_advanced)(cmem, version);
                let rd = (r.create_dctx_advanced)(cmem, version);
                assert_eq!(
                    cd.is_null(),
                    rd.is_null(),
                    "createDecompressionContext_advanced({}, v={})",
                    name,
                    version
                );
                if !cc.is_null() && !cd.is_null() {
                    // a full round trip using the advanced contexts
                    let n = 50_000usize;
                    let data = gen(Shape::Text, n, &mut rng);
                    let p = LZ4F_preferences_t::default();
                    let mut chb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                    let mut rhb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                    let a = (c.compress_begin)(cc, chb.as_mut_ptr() as *mut c_void, LZ4F_HEADER_SIZE_MAX, &p);
                    let b = (r.compress_begin)(rc_, rhb.as_mut_ptr() as *mut c_void, LZ4F_HEADER_SIZE_MAX, &p);
                    assert_eq!(a, b);
                    let mut cframe = chb[..a].to_vec();
                    let mut rframe = rhb[..b].to_vec();
                    let ucap = (c.compress_bound)(n, &p);
                    let mut cb = vec![0u8; ucap];
                    let mut rb = vec![0u8; ucap];
                    let a = (c.compress_update)(
                        cc,
                        cb.as_mut_ptr() as *mut c_void,
                        ucap,
                        data.as_ptr() as *const c_void,
                        n,
                        std::ptr::null(),
                    );
                    let b = (r.compress_update)(
                        rc_,
                        rb.as_mut_ptr() as *mut c_void,
                        ucap,
                        data.as_ptr() as *const c_void,
                        n,
                        std::ptr::null(),
                    );
                    assert_eq!(a, b);
                    cframe.extend_from_slice(&cb[..a]);
                    rframe.extend_from_slice(&rb[..b]);
                    let ecap = (c.compress_bound)(0, &p);
                    let mut cb = vec![0u8; ecap];
                    let mut rb = vec![0u8; ecap];
                    let a = (c.compress_end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                    let b = (r.compress_end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                    assert_eq!(a, b);
                    cframe.extend_from_slice(&cb[..a]);
                    rframe.extend_from_slice(&rb[..b]);
                    assert_bytes_eq(&format!("advanced frame {}", name), &cframe, &rframe);
                    let x = decode_frame(&c, cd, &cframe, 1000, 4096, std::ptr::null()).unwrap();
                    let y = decode_frame(&r, rd, &rframe, 1000, 4096, std::ptr::null()).unwrap();
                    assert_bytes_eq("advanced decode", &x, &y);
                    assert_bytes_eq("advanced content", &x, &data);
                }
                if !cc.is_null() {
                    (c.free_cctx)(cc);
                }
                if !rc_.is_null() {
                    (r.free_cctx)(rc_);
                }
                if !cd.is_null() {
                    (c.free_dctx)(cd);
                }
                if !rd.is_null() {
                    (r.free_dctx)(rd);
                }
            }
            // createCDict_advanced
            for &ds in &[0usize, 13, 70_000] {
                let dict = gen(Shape::Text, ds, &mut rng);
                unsafe {
                    let ccd = (c.create_cdict_advanced)(cmem, dict.as_ptr() as *const c_void, ds);
                    let rcd = (r.create_cdict_advanced)(cmem, dict.as_ptr() as *const c_void, ds);
                    assert_eq!(
                        ccd.is_null(),
                        rcd.is_null(),
                        "createCDict_advanced({}, ds={})",
                        name,
                        ds
                    );
                    if !ccd.is_null() {
                        // use it once to make sure the digested dict is identical
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let n = 20_000usize;
                        let mut data = gen(Shape::Text, n, &mut rng);
                        let k = n.min(ds);
                        if k > 0 {
                            data[..k].copy_from_slice(&dict[..k]);
                        }
                        let p = LZ4F_preferences_t::default();
                        let cap = (c.compress_frame_bound)(n, &p);
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let a = (c.compress_frame_using_cdict)(
                            cc,
                            cb.as_mut_ptr() as *mut c_void,
                            cap,
                            data.as_ptr() as *const c_void,
                            n,
                            ccd,
                            &p,
                        );
                        let b = (r.compress_frame_using_cdict)(
                            rc_,
                            rb.as_mut_ptr() as *mut c_void,
                            cap,
                            data.as_ptr() as *const c_void,
                            n,
                            rcd,
                            &p,
                        );
                        assert_eq!(a, b, "advanced CDict compress rc {} ds={}", name, ds);
                        assert_bytes_eq("advanced CDict compress", &cb[..a.min(cap)], &rb[..b.min(cap)]);
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);
                    }
                    (c.free_cdict)(ccd);
                    (r.free_cdict)(rcd);
                }
            }
        }
    }
    // NULL / zero-size dictionaries
    unsafe {
        let a = (c.create_cdict)(std::ptr::null(), 0);
        let b = (r.create_cdict)(std::ptr::null(), 0);
        assert_eq!(a.is_null(), b.is_null(), "createCDict(NULL,0)");
        (c.free_cdict)(a);
        (r.free_cdict)(b);
        // freeCDict(NULL) must be a no-op in both
        (c.free_cdict)(std::ptr::null_mut());
        (r.free_cdict)(std::ptr::null_mut());
    }
}

// --- CONFIGS: skippable frames + concatenated frames ------------------------
#[test]
fn frame_skippable_and_concatenated() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4007);
    // build a stream of several frames back to back, decode with both
    let mut cstream = Vec::new();
    let mut rstream = Vec::new();
    let mut expected = Vec::new();
    for i in 0..6 {
        let n = rng.range(0, 50_000);
        let data = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], n, &mut rng);
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = BSIDS[i % BSIDS.len()];
        p.frameInfo.contentChecksumFlag = (i % 2) as c_int;
        p.frameInfo.blockChecksumFlag = ((i / 2) % 2) as c_int;
        p.compressionLevel = [0i32, 1, 9, 12][i % 4];
        let cap = unsafe { (c.compress_frame_bound)(n, &p) };
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
        assert_eq!(a, b);
        cstream.extend_from_slice(&cb[..a]);
        rstream.extend_from_slice(&rb[..b]);
        expected.extend_from_slice(&data);

        // inject a skippable frame between frames
        let skip_len = rng.range(0, 200);
        let magic: u32 = 0x184D2A50 + (i as u32 % 16);
        let mut skip = Vec::new();
        skip.extend_from_slice(&magic.to_le_bytes());
        skip.extend_from_slice(&(skip_len as u32).to_le_bytes());
        skip.extend_from_slice(&gen(Shape::Random, skip_len, &mut rng));
        cstream.extend_from_slice(&skip);
        rstream.extend_from_slice(&skip);
    }
    assert_bytes_eq("concatenated stream", &cstream, &rstream);
    // decode all frames in sequence with one dctx
    for &src_chunk in &[1usize, 13, 4096, usize::MAX] {
        unsafe {
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            (c.create_dctx)(&mut cd, LZ4F_VERSION);
            (r.create_dctx)(&mut rd, LZ4F_VERSION);
            let mut cout = Vec::new();
            let mut rout = Vec::new();
            let mut csp = 0usize;
            let mut rsp = 0usize;
            let mut cbuf = vec![0u8; 1 << 16];
            let mut rbuf = vec![0u8; 1 << 16];
            let mut guard = 0;
            while csp < cstream.len() && guard < 10_000_000 {
                guard += 1;
                let mut cs = (cstream.len() - csp).min(src_chunk.max(1));
                let mut ds = cbuf.len();
                let x = (c.decompress)(
                    cd,
                    cbuf.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    cstream.as_ptr().add(csp) as *const c_void,
                    &mut cs,
                    std::ptr::null(),
                );
                let mut rs = (rstream.len() - rsp).min(src_chunk.max(1));
                let mut dr = rbuf.len();
                let y = (r.decompress)(
                    rd,
                    rbuf.as_mut_ptr() as *mut c_void,
                    &mut dr,
                    rstream.as_ptr().add(rsp) as *const c_void,
                    &mut rs,
                    std::ptr::null(),
                );
                assert_eq!(
                    x,
                    y,
                    "concatenated decompress rc src_chunk={} -> {} vs {}",
                    src_chunk,
                    fmt_lz4f(x),
                    fmt_lz4f(y)
                );
                assert_eq!(cs, rs, "concatenated consumed src_chunk={}", src_chunk);
                assert_eq!(ds, dr, "concatenated produced src_chunk={}", src_chunk);
                assert_bytes_eq("concatenated output", &cbuf[..ds], &rbuf[..dr]);
                if (c.is_error)(x) != 0 {
                    break;
                }
                cout.extend_from_slice(&cbuf[..ds]);
                rout.extend_from_slice(&rbuf[..dr]);
                csp += cs;
                rsp += rs;
                if cs == 0 && ds == 0 {
                    break;
                }
            }
            assert_bytes_eq("concatenated decode", &cout, &rout);
            assert_bytes_eq("concatenated content", &cout, &expected);
            (c.free_dctx)(cd);
            (r.free_dctx)(rd);
        }
    }
}

// --- CONFIGS: stableDst / skipChecksums / resetDecompressionContext ---------
#[test]
fn frame_decompress_options() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4008);
    for &bsid in &BSIDS {
        for &cchk in &[0, 1] {
            for &bchk in &[0, 1] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = bsid;
                p.frameInfo.contentChecksumFlag = cchk;
                p.frameInfo.blockChecksumFlag = bchk;
                let n = 150_000usize;
                let data = gen(Shape::Text, n, &mut rng);
                let cap = unsafe { (c.compress_frame_bound)(n, &p) };
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
                let frame = &frame[..flen];
                for &stable_dst in &[0u32, 1] {
                    for &skip in &[0u32, 1] {
                        let opts = LZ4F_decompressOptions_t {
                            stableDst: stable_dst,
                            skipChecksums: skip,
                            reserved1: 0,
                            reserved0: 0,
                        };
                        // stableDst requires a stable destination: decode into one
                        // big buffer, advancing the pointer
                        unsafe {
                            let mut cd: *mut c_void = std::ptr::null_mut();
                            let mut rd: *mut c_void = std::ptr::null_mut();
                            (c.create_dctx)(&mut cd, LZ4F_VERSION);
                            (r.create_dctx)(&mut rd, LZ4F_VERSION);
                            let mut cout = vec![0u8; n + 4096];
                            let mut rout = vec![0u8; n + 4096];
                            let mut cdp = 0usize;
                            let mut rdp = 0usize;
                            let mut sp = 0usize;
                            let src_chunk = [1usize, 17, 4096, usize::MAX][rng.below(4)];
                            let dst_chunk = [1usize, 100, 65536][rng.below(3)];
                            let mut guard = 0;
                            loop {
                                guard += 1;
                                if guard > 5_000_000 {
                                    panic!("stalled");
                                }
                                let mut cs = (frame.len() - sp).min(src_chunk.max(1));
                                let mut rs = cs;
                                let mut cdz = dst_chunk.min(cout.len() - cdp);
                                let mut rdz = dst_chunk.min(rout.len() - rdp);
                                let x = (c.decompress)(
                                    cd,
                                    cout.as_mut_ptr().add(cdp) as *mut c_void,
                                    &mut cdz,
                                    frame.as_ptr().add(sp) as *const c_void,
                                    &mut cs,
                                    &opts,
                                );
                                let y = (r.decompress)(
                                    rd,
                                    rout.as_mut_ptr().add(rdp) as *mut c_void,
                                    &mut rdz,
                                    frame.as_ptr().add(sp) as *const c_void,
                                    &mut rs,
                                    &opts,
                                );
                                assert_eq!(
                                    x,
                                    y,
                                    "decompress opts rc stableDst={} skip={} -> {} vs {}",
                                    stable_dst,
                                    skip,
                                    fmt_lz4f(x),
                                    fmt_lz4f(y)
                                );
                                assert_eq!(cs, rs, "decompress opts consumed");
                                assert_eq!(cdz, rdz, "decompress opts produced");
                                assert_bytes_eq(
                                    "decompress opts output",
                                    &cout[cdp..cdp + cdz],
                                    &rout[rdp..rdp + rdz],
                                );
                                if (c.is_error)(x) != 0 {
                                    break;
                                }
                                cdp += cdz;
                                rdp += rdz;
                                sp += cs;
                                if x == 0 {
                                    break;
                                }
                                if cs == 0 && cdz == 0 {
                                    break;
                                }
                            }
                            assert_eq!(cdp, rdp);
                            assert_bytes_eq("decompress opts content", &cout[..cdp], &data[..cdp]);
                            // resetDecompressionContext then reuse
                            (c.reset_dctx)(cd);
                            (r.reset_dctx)(rd);
                            let x = decode_frame(&c, cd, frame, usize::MAX, 1 << 20, &opts).unwrap();
                            let y = decode_frame(&r, rd, frame, usize::MAX, 1 << 20, &opts).unwrap();
                            assert_bytes_eq("after reset", &x, &y);
                            assert_bytes_eq("after reset content", &x, &data);
                            (c.free_dctx)(cd);
                            (r.free_dctx)(rd);
                        }
                    }
                }
            }
        }
    }
}

// --- CONFIGS: compressBegin buffer-sizing matrix (autoFlush x blockMode) ----
#[test]
fn frame_begin_buffer_matrix() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4009);
    for &bsid in &BSIDS {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &af in &[0u32, 1] {
                for &lvl in &[0i32, 1, 2, 3, 9, 10, 12] {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.blockMode = bmode;
                    p.autoFlush = af;
                    p.compressionLevel = lvl;
                    let total = 250_000usize;
                    let data = gen(Shape::Text, total, &mut rng);
                    unsafe {
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc_: *mut c_void = std::ptr::null_mut();
                        (c.create_cctx)(&mut cc, LZ4F_VERSION);
                        (r.create_cctx)(&mut rc_, LZ4F_VERSION);
                        let mut chb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                        let mut rhb = vec![0u8; LZ4F_HEADER_SIZE_MAX];
                        let a = (c.compress_begin)(cc, chb.as_mut_ptr() as *mut c_void, LZ4F_HEADER_SIZE_MAX, &p);
                        let b = (r.compress_begin)(rc_, rhb.as_mut_ptr() as *mut c_void, LZ4F_HEADER_SIZE_MAX, &p);
                        assert_eq!(a, b, "begin matrix rc {}", prefs_desc(&p));
                        assert_bytes_eq("begin matrix header", &chb[..a], &rhb[..b]);
                        let mut cframe = chb[..a].to_vec();
                        let mut rframe = rhb[..b].to_vec();
                        // feed with sizes deliberately straddling the block size
                        let bs = (c.get_block_size)(if bsid == 0 { LZ4F_MAX64KB } else { bsid });
                        let sizes = [1usize, bs / 2, bs - 1, bs, bs + 1, 2 * bs + 3];
                        let mut off = 0usize;
                        for &m0 in &sizes {
                            let m = m0.min(total - off);
                            if m == 0 {
                                continue;
                            }
                            let ucap = (c.compress_bound)(m, &p);
                            let mut cb = vec![0u8; ucap];
                            let mut rb = vec![0u8; ucap];
                            let a = (c.compress_update)(
                                cc,
                                cb.as_mut_ptr() as *mut c_void,
                                ucap,
                                data.as_ptr().add(off) as *const c_void,
                                m,
                                std::ptr::null(),
                            );
                            let b = (r.compress_update)(
                                rc_,
                                rb.as_mut_ptr() as *mut c_void,
                                ucap,
                                data.as_ptr().add(off) as *const c_void,
                                m,
                                std::ptr::null(),
                            );
                            assert_eq!(
                                a,
                                b,
                                "begin matrix update rc m={} {} -> {} vs {}",
                                m,
                                prefs_desc(&p),
                                fmt_lz4f(a),
                                fmt_lz4f(b)
                            );
                            assert_bytes_eq("begin matrix update", &cb[..a.min(ucap)], &rb[..b.min(ucap)]);
                            cframe.extend_from_slice(&cb[..a]);
                            rframe.extend_from_slice(&rb[..b]);
                            off += m;
                        }
                        let ecap = (c.compress_bound)(0, &p);
                        let mut cb = vec![0u8; ecap];
                        let mut rb = vec![0u8; ecap];
                        let a = (c.compress_end)(cc, cb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                        let b = (r.compress_end)(rc_, rb.as_mut_ptr() as *mut c_void, ecap, std::ptr::null());
                        assert_eq!(a, b, "begin matrix end rc {}", prefs_desc(&p));
                        cframe.extend_from_slice(&cb[..a]);
                        rframe.extend_from_slice(&rb[..b]);
                        assert_bytes_eq(
                            &format!("begin matrix frame {}", prefs_desc(&p)),
                            &cframe,
                            &rframe,
                        );
                        (c.free_cctx)(cc);
                        (r.free_cctx)(rc_);
                        let mut cd: *mut c_void = std::ptr::null_mut();
                        let mut rd: *mut c_void = std::ptr::null_mut();
                        (c.create_dctx)(&mut cd, LZ4F_VERSION);
                        (r.create_dctx)(&mut rd, LZ4F_VERSION);
                        let x = decode_frame(&c, cd, &cframe, 4096, 1 << 20, std::ptr::null()).unwrap();
                        let y = decode_frame(&r, rd, &rframe, 4096, 1 << 20, std::ptr::null()).unwrap();
                        assert_bytes_eq("begin matrix decode", &x, &y);
                        assert_bytes_eq("begin matrix content", &x, &data[..off]);
                        (c.free_dctx)(cd);
                        (r.free_dctx)(rd);
                    }
                }
            }
        }
    }
}
