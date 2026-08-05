// Phase B — LZ4 Frame dictionary API differential tests.
mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct FrameInfo {
    block_size_id: c_uint,
    block_mode: c_uint,
    content_checksum: c_uint,
    frame_type: c_uint,
    content_size: u64,
    dict_id: c_uint,
    block_checksum: c_uint,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Preferences {
    frame_info: FrameInfo,
    compression_level: c_int,
    auto_flush: c_uint,
    favor_dec_speed: c_uint,
    reserved: [c_uint; 3],
}
fn base_prefs() -> Preferences {
    Preferences {
        frame_info: FrameInfo {
            block_size_id: 0,
            block_mode: 0,
            content_checksum: 0,
            frame_type: 0,
            content_size: 0,
            dict_id: 0,
            block_checksum: 0,
        },
        compression_level: 0,
        auto_flush: 0,
        favor_dec_speed: 0,
        reserved: [0; 3],
    }
}

const LZ4F_VERSION: c_uint = 100;

type IsError = unsafe extern "C" fn(usize) -> c_uint;
type CreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
type CreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
type CreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FreeCDict = unsafe extern "C" fn(*mut c_void);
type CompressFrameBound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
type CompressFrameUsingCDict = unsafe extern "C" fn(
    *mut c_void, // cctx
    *mut c_void, // dst
    usize,
    *const c_void, // src
    usize,
    *const c_void, // cdict
    *const Preferences,
) -> usize;
type DecompressUsingDict = unsafe extern "C" fn(
    *mut c_void, // dctx
    *mut c_void, // dst
    *mut usize,
    *const c_void, // src
    *mut usize,
    *const c_void, // dict
    usize,
    *const c_void, // opts
) -> usize;

unsafe fn decode_usingdict(libs: &Libs, use_rust: bool, frame: &[u8], dict: &[u8], expected: &[u8]) {
    let (cd, fd, dud): (
        libloading::Symbol<CreateDctx>,
        libloading::Symbol<FreeDctx>,
        libloading::Symbol<DecompressUsingDict>,
    ) = if use_rust {
        (rsym(libs, b"LZ4F_createDecompressionContext"),
         rsym(libs, b"LZ4F_freeDecompressionContext"),
         rsym(libs, b"LZ4F_decompress_usingDict"))
    } else {
        (csym(libs, b"LZ4F_createDecompressionContext"),
         csym(libs, b"LZ4F_freeDecompressionContext"),
         csym(libs, b"LZ4F_decompress_usingDict"))
    };
    let ie: libloading::Symbol<IsError> = if use_rust { rsym(libs, b"LZ4F_isError") } else { csym(libs, b"LZ4F_isError") };
    let mut dctx: *mut c_void = std::ptr::null_mut();
    assert_eq!(cd(&mut dctx, LZ4F_VERSION), 0);
    let mut out = vec![0u8; expected.len().max(1)];
    let mut sc = 0usize;
    let mut dp = 0usize;
    loop {
        let mut src_sz = frame.len() - sc;
        let mut dst_sz = out.len() - dp;
        let ret = dud(
            dctx,
            out.as_mut_ptr().add(dp) as *mut c_void,
            &mut dst_sz,
            frame.as_ptr().add(sc) as *const c_void,
            &mut src_sz,
            dict.as_ptr() as *const c_void,
            dict.len(),
            std::ptr::null(),
        );
        assert_eq!(ie(ret), 0, "decompress_usingDict errored (rust={})", use_rust);
        sc += src_sz;
        dp += dst_sz;
        if ret == 0 { break; }
        if sc >= frame.len() && dst_sz == 0 { break; }
    }
    assert_eq!(dp, expected.len(), "usingDict decoded len (rust={})", use_rust);
    assert_eq!(&out[..dp], expected, "usingDict content (rust={})", use_rust);
    fd(dctx);
}

#[test]
fn test_compressframe_usingcdict() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xcd1c);
    unsafe {
        let c_create: libloading::Symbol<CreateCDict> = csym(&libs, b"LZ4F_createCDict");
        let r_create: libloading::Symbol<CreateCDict> = rsym(&libs, b"LZ4F_createCDict");
        let c_free: libloading::Symbol<FreeCDict> = csym(&libs, b"LZ4F_freeCDict");
        let r_free: libloading::Symbol<FreeCDict> = rsym(&libs, b"LZ4F_freeCDict");
        let c_cctx: libloading::Symbol<CreateCctx> = csym(&libs, b"LZ4F_createCompressionContext");
        let r_cctx: libloading::Symbol<CreateCctx> = rsym(&libs, b"LZ4F_createCompressionContext");
        let c_fctx: libloading::Symbol<FreeCctx> = csym(&libs, b"LZ4F_freeCompressionContext");
        let r_fctx: libloading::Symbol<FreeCctx> = rsym(&libs, b"LZ4F_freeCompressionContext");
        let c_cf: libloading::Symbol<CompressFrameUsingCDict> = csym(&libs, b"LZ4F_compressFrame_usingCDict");
        let r_cf: libloading::Symbol<CompressFrameUsingCDict> = rsym(&libs, b"LZ4F_compressFrame_usingCDict");
        let c_fb: libloading::Symbol<CompressFrameBound> = csym(&libs, b"LZ4F_compressFrameBound");
        let ie: libloading::Symbol<IsError> = csym(&libs, b"LZ4F_isError");

        for &dictsz in &[100usize, 4096, 70000] {
            for &datasz in &[100usize, 5000, 70000] {
                let dict = rng.compressible(dictsz);
                let data = rng.compressible(datasz);
                let prefs = base_prefs();

                let ccd = c_create(dict.as_ptr() as *const c_void, dictsz);
                let rcd = r_create(dict.as_ptr() as *const c_void, dictsz);
                assert!(!ccd.is_null() && !rcd.is_null());

                let mut cctx: *mut c_void = std::ptr::null_mut();
                let mut rctx: *mut c_void = std::ptr::null_mut();
                assert_eq!(c_cctx(&mut cctx, LZ4F_VERSION), 0);
                assert_eq!(r_cctx(&mut rctx, LZ4F_VERSION), 0);

                let bound = c_fb(datasz, &prefs);
                let mut cdst = vec![0u8; bound];
                let mut rdst = vec![0u8; bound];
                let cn = c_cf(cctx, cdst.as_mut_ptr() as *mut c_void, bound, data.as_ptr() as *const c_void, datasz, ccd, &prefs);
                let rn = r_cf(rctx, rdst.as_mut_ptr() as *mut c_void, bound, data.as_ptr() as *const c_void, datasz, rcd, &prefs);
                assert_eq!(ie(cn), 0, "C compressFrame_usingCDict errored");
                assert_eq!(cn, rn, "usingCDict ret dictsz={} datasz={}", dictsz, datasz);
                assert_eq!(&cdst[..cn], &rdst[..rn], "usingCDict bytes dictsz={} datasz={}", dictsz, datasz);

                // decode with dict (both libs)
                decode_usingdict(&libs, false, &cdst[..cn], &dict, &data);
                decode_usingdict(&libs, true, &rdst[..rn], &dict, &data);

                c_free(ccd);
                r_free(rcd);
                c_fctx(cctx);
                r_fctx(rctx);
            }
        }
    }
}

type CompressBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const Preferences) -> usize;
type CompressBeginUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const Preferences,
) -> usize;
type CompressBound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
type CompressUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
) -> usize;
type CompressEnd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void) -> usize;

#[test]
fn test_compressbegin_usingdict() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xbe91);
    unsafe {
        for &dictsz in &[100usize, 4096, 70000] {
            let dict = rng.compressible(dictsz);
            let data = rng.compressible(6000);
            let prefs = base_prefs();
            let cframe = begin_usingdict_stream(&libs, false, &prefs, &dict, &data);
            let rframe = begin_usingdict_stream(&libs, true, &prefs, &dict, &data);
            assert_eq!(cframe, rframe, "compressBegin_usingDict frame dictsz={}", dictsz);
            decode_usingdict(&libs, false, &cframe, &dict, &data);
            decode_usingdict(&libs, true, &rframe, &dict, &data);
        }
    }
}

unsafe fn begin_usingdict_stream(libs: &Libs, use_rust: bool, prefs: &Preferences, dict: &[u8], data: &[u8]) -> Vec<u8> {
    macro_rules! sym {
        ($t:ty, $name:expr) => {{
            let s: libloading::Symbol<$t> = if use_rust { rsym(libs, $name) } else { csym(libs, $name) };
            s
        }};
    }
    let create = sym!(CreateCctx, b"LZ4F_createCompressionContext");
    let free = sym!(FreeCctx, b"LZ4F_freeCompressionContext");
    let begin = sym!(CompressBeginUsingDict, b"LZ4F_compressBegin_usingDict");
    let bound = sym!(CompressBound, b"LZ4F_compressBound");
    let update = sym!(CompressUpdate, b"LZ4F_compressUpdate");
    let end = sym!(CompressEnd, b"LZ4F_compressEnd");
    let ie = sym!(IsError, b"LZ4F_isError");

    let mut cctx: *mut c_void = std::ptr::null_mut();
    assert_eq!(create(&mut cctx, LZ4F_VERSION), 0);
    let chunk = 4096usize;
    let cap = bound(chunk, prefs) + 64;
    let mut out = vec![0u8; 19 + data.len() + cap * (data.len() / chunk + 2) + 128];
    let mut written = 0usize;
    let hdr = begin(cctx, out.as_mut_ptr() as *mut c_void, out.len(), dict.as_ptr() as *const c_void, dict.len(), prefs);
    assert_eq!(ie(hdr), 0, "begin_usingDict errored (rust={})", use_rust);
    written += hdr;
    let mut off = 0;
    while off < data.len() {
        let this = chunk.min(data.len() - off);
        let dstcap = bound(this, prefs);
        let n = update(cctx, out.as_mut_ptr().add(written) as *mut c_void, dstcap, data.as_ptr().add(off) as *const c_void, this, std::ptr::null());
        assert_eq!(ie(n), 0);
        written += n;
        off += this;
    }
    let ecap = bound(0, prefs);
    let en = end(cctx, out.as_mut_ptr().add(written) as *mut c_void, ecap, std::ptr::null());
    assert_eq!(ie(en), 0);
    written += en;
    free(cctx);
    out.truncate(written);
    out
}
