// Phase B/C — LZ4 Frame API (lz4frame.c) differential tests.
mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ---- Struct layouts mirroring lz4frame.h (x86_64 LE) ----
#[repr(C)]
#[derive(Clone, Copy)]
struct FrameInfo {
    block_size_id: c_uint,   // LZ4F_blockSizeID_t
    block_mode: c_uint,      // LZ4F_blockMode_t
    content_checksum: c_uint,// LZ4F_contentChecksum_t
    frame_type: c_uint,      // LZ4F_frameType_t
    content_size: u64,       // unsigned long long
    dict_id: c_uint,         // unsigned
    block_checksum: c_uint,  // LZ4F_blockChecksum_t
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

impl Default for FrameInfo {
    fn default() -> Self {
        FrameInfo {
            block_size_id: 0,
            block_mode: 0,
            content_checksum: 0,
            frame_type: 0,
            content_size: 0,
            dict_id: 0,
            block_checksum: 0,
        }
    }
}
impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            frame_info: FrameInfo::default(),
            compression_level: 0,
            auto_flush: 0,
            favor_dec_speed: 0,
            reserved: [0; 3],
        }
    }
}

type CompressFrameBound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
type CompressFrame =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
type IsError = unsafe extern "C" fn(usize) -> c_uint;
type GetErrorCode = unsafe extern "C" fn(usize) -> c_int;
type CreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
type CreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
type Decompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
) -> usize;
type GetBlockSize = unsafe extern "C" fn(c_uint) -> usize;
type CompressBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const Preferences) -> usize;
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
type HeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

const LZ4F_VERSION: c_uint = 100;

/// Full-frame roundtrip via one-shot compressFrame + streaming decompress, on BOTH libs;
/// assert compressed frames identical and both decode to original.
unsafe fn frame_roundtrip(libs: &Libs, prefs: &Preferences, input: &[u8]) {
    let c_cfb: libloading::Symbol<CompressFrameBound> = csym(libs, b"LZ4F_compressFrameBound");
    let r_cfb: libloading::Symbol<CompressFrameBound> = rsym(libs, b"LZ4F_compressFrameBound");
    let c_cf: libloading::Symbol<CompressFrame> = csym(libs, b"LZ4F_compressFrame");
    let r_cf: libloading::Symbol<CompressFrame> = rsym(libs, b"LZ4F_compressFrame");
    let c_ie: libloading::Symbol<IsError> = csym(libs, b"LZ4F_isError");

    let cb = c_cfb(input.len(), prefs);
    let rb = r_cfb(input.len(), prefs);
    assert_eq!(cb, rb, "compressFrameBound mismatch len={}", input.len());

    let mut cdst = vec![0u8; cb];
    let mut rdst = vec![0u8; rb];
    let cn = c_cf(cdst.as_mut_ptr() as *mut c_void, cb, input.as_ptr() as *const c_void, input.len(), prefs);
    let rn = r_cf(rdst.as_mut_ptr() as *mut c_void, rb, input.as_ptr() as *const c_void, input.len(), prefs);
    assert_eq!(c_ie(cn), 0, "C compressFrame errored");
    assert_eq!(cn, rn, "compressFrame ret differ len={}", input.len());
    assert_eq!(&cdst[..cn], &rdst[..rn], "compressed frame bytes differ len={}", input.len());

    // Decompress both frames using each library's own decompressor.
    decode_and_check(libs, &cdst[..cn], input, false);
    decode_and_check(libs, &rdst[..rn], input, true);
}

unsafe fn decode_and_check(libs: &Libs, frame: &[u8], expected: &[u8], _rust_frame: bool) {
    // Decode with BOTH libs, verifying each reproduces expected and consumes fully.
    for use_rust in [false, true] {
        let (cd, fd, dec): (
            libloading::Symbol<CreateDctx>,
            libloading::Symbol<FreeDctx>,
            libloading::Symbol<Decompress>,
        ) = if use_rust {
            (rsym(libs, b"LZ4F_createDecompressionContext"),
             rsym(libs, b"LZ4F_freeDecompressionContext"),
             rsym(libs, b"LZ4F_decompress"))
        } else {
            (csym(libs, b"LZ4F_createDecompressionContext"),
             csym(libs, b"LZ4F_freeDecompressionContext"),
             csym(libs, b"LZ4F_decompress"))
        };
        let mut dctx: *mut c_void = std::ptr::null_mut();
        let e = cd(&mut dctx, LZ4F_VERSION);
        assert_eq!(e, 0);
        let mut out = vec![0u8; expected.len().max(1)];
        let mut src_consumed_total = 0usize;
        let mut dst_produced_total = 0usize;
        loop {
            let mut src_sz = frame.len() - src_consumed_total;
            let mut dst_sz = out.len() - dst_produced_total;
            let ret = dec(
                dctx,
                out.as_mut_ptr().add(dst_produced_total) as *mut c_void,
                &mut dst_sz,
                frame.as_ptr().add(src_consumed_total) as *const c_void,
                &mut src_sz,
                std::ptr::null(),
            );
            let ie: libloading::Symbol<IsError> = if use_rust { rsym(libs, b"LZ4F_isError") } else { csym(libs, b"LZ4F_isError") };
            assert_eq!(ie(ret), 0, "decompress errored (rust={})", use_rust);
            src_consumed_total += src_sz;
            dst_produced_total += dst_sz;
            if ret == 0 { break; }
            if src_consumed_total >= frame.len() && dst_sz == 0 { break; }
        }
        assert_eq!(dst_produced_total, expected.len(), "decoded len (rust={})", use_rust);
        assert_eq!(&out[..dst_produced_total], expected, "decoded content (rust={})", use_rust);
        fd(dctx);
    }
}

fn base_prefs() -> Preferences {
    Preferences::default()
}

#[test]
fn test_frame_default_roundtrip() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xf00d);
    unsafe {
        for &sz in &[0usize, 1, 100, 4096, 65536, 200000] {
            frame_roundtrip(&libs, &base_prefs(), &rng.compressible(sz));
            frame_roundtrip(&libs, &base_prefs(), &rng.random(sz));
        }
    }
}

#[test]
fn test_frame_options_matrix() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0071);
    unsafe {
        for &block_size_id in &[0u32, 4, 5, 6, 7] {
            for &block_mode in &[0u32, 1] {
                for &content_checksum in &[0u32, 1] {
                    for &block_checksum in &[0u32, 1] {
                        for &level in &[0i32, 3, 9, 12, -1] {
                            for &auto_flush in &[0u32, 1] {
                                let mut prefs = base_prefs();
                                prefs.frame_info.block_size_id = block_size_id;
                                prefs.frame_info.block_mode = block_mode;
                                prefs.frame_info.content_checksum = content_checksum;
                                prefs.frame_info.block_checksum = block_checksum;
                                prefs.compression_level = level;
                                prefs.auto_flush = auto_flush;
                                let data = rng.compressible(3000);
                                frame_roundtrip(&libs, &prefs, &data);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_frame_content_size() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xc51e);
    unsafe {
        for &sz in &[0usize, 100, 5000, 70000] {
            let mut prefs = base_prefs();
            prefs.frame_info.content_size = sz as u64;
            let data = rng.compressible(sz);
            frame_roundtrip(&libs, &prefs, &data);
        }
    }
}

#[test]
fn test_get_block_size() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<GetBlockSize> = csym(&libs, b"LZ4F_getBlockSize");
        let r: libloading::Symbol<GetBlockSize> = rsym(&libs, b"LZ4F_getBlockSize");
        let c_ie: libloading::Symbol<IsError> = csym(&libs, b"LZ4F_isError");
        let r_ie: libloading::Symbol<IsError> = rsym(&libs, b"LZ4F_isError");
        for id in 0u32..=10 {
            let cv = c(id);
            let rv = r(id);
            assert_eq!(cv, rv, "getBlockSize({})", id);
            assert_eq!(c_ie(cv), r_ie(rv), "getBlockSize isError({})", id);
        }
    }
}

#[test]
fn test_compressionlevel_max_and_version() {
    let libs = Libs::load();
    unsafe {
        let cm: libloading::Symbol<unsafe extern "C" fn() -> c_int> = csym(&libs, b"LZ4F_compressionLevel_max");
        let rm: libloading::Symbol<unsafe extern "C" fn() -> c_int> = rsym(&libs, b"LZ4F_compressionLevel_max");
        assert_eq!(cm(), rm());
        let cv: libloading::Symbol<unsafe extern "C" fn() -> c_uint> = csym(&libs, b"LZ4F_getVersion");
        let rv: libloading::Symbol<unsafe extern "C" fn() -> c_uint> = rsym(&libs, b"LZ4F_getVersion");
        assert_eq!(cv(), rv());
    }
}

#[test]
fn test_frame_streaming_update() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x57ea);
    unsafe {
        for &auto_flush in &[0u32, 1] {
            for &block_mode in &[0u32, 1] {
                let mut prefs = base_prefs();
                prefs.auto_flush = auto_flush;
                prefs.frame_info.block_mode = block_mode;
                let data = rng.compressible(50000);

                let (c_frame, c_written) = stream_compress(&libs, &prefs, &data, false, false);
                let (r_frame, r_written) = stream_compress(&libs, &prefs, &data, true, false);
                assert_eq!(c_written, r_written, "streaming update total len (af={}, bm={})", auto_flush, block_mode);
                assert_eq!(c_frame, r_frame, "streaming update frame bytes (af={}, bm={})", auto_flush, block_mode);
                decode_and_check(&libs, &c_frame, &data, false);
            }
        }
    }
}

#[test]
fn test_frame_streaming_flush() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xf105);
    unsafe {
        let mut prefs = base_prefs();
        prefs.auto_flush = 0;
        let data = rng.compressible(30000);
        let (c_frame, _) = stream_compress(&libs, &prefs, &data, false, true);
        let (r_frame, _) = stream_compress(&libs, &prefs, &data, true, true);
        assert_eq!(c_frame, r_frame, "streaming flush frame bytes");
        decode_and_check(&libs, &c_frame, &data, false);
    }
}

/// Stream-compress `data` in chunks with compressBegin/Update/[flush]/End.
/// Returns (frame bytes, total written). If `use_rust`, uses Rust exports.
unsafe fn stream_compress(
    libs: &Libs,
    prefs: &Preferences,
    data: &[u8],
    use_rust: bool,
    do_flush: bool,
) -> (Vec<u8>, usize) {
    macro_rules! sym {
        ($t:ty, $name:expr) => {{
            let s: libloading::Symbol<$t> = if use_rust { rsym(libs, $name) } else { csym(libs, $name) };
            s
        }};
    }
    let create = sym!(CreateCctx, b"LZ4F_createCompressionContext");
    let free = sym!(FreeCctx, b"LZ4F_freeCompressionContext");
    let begin = sym!(CompressBegin, b"LZ4F_compressBegin");
    let bound = sym!(CompressBound, b"LZ4F_compressBound");
    let update = sym!(CompressUpdate, b"LZ4F_compressUpdate");
    let flush = sym!(CompressEnd, b"LZ4F_flush");
    let end = sym!(CompressEnd, b"LZ4F_compressEnd");
    let ie = sym!(IsError, b"LZ4F_isError");

    let mut cctx: *mut c_void = std::ptr::null_mut();
    assert_eq!(create(&mut cctx, LZ4F_VERSION), 0);

    let chunk = 4096usize;
    let cap = bound(chunk, prefs) + 64;
    let mut out = vec![0u8; 19 + data.len() + cap * (data.len() / chunk + 2) + 64];
    let mut written = 0usize;

    let hdr = begin(cctx, out.as_mut_ptr() as *mut c_void, out.len(), prefs);
    assert_eq!(ie(hdr), 0);
    written += hdr;

    let mut off = 0;
    let mut toggle = false;
    while off < data.len() {
        let this = chunk.min(data.len() - off);
        let dstcap = bound(this, prefs);
        assert!(written + dstcap <= out.len());
        let n = update(
            cctx,
            out.as_mut_ptr().add(written) as *mut c_void,
            dstcap,
            data.as_ptr().add(off) as *const c_void,
            this,
            std::ptr::null(),
        );
        assert_eq!(ie(n), 0, "update errored (rust={})", use_rust);
        written += n;
        off += this;

        if do_flush && toggle {
            let fcap = bound(0, prefs);
            let fn_ = flush(cctx, out.as_mut_ptr().add(written) as *mut c_void, fcap, std::ptr::null());
            assert_eq!(ie(fn_), 0, "flush errored");
            written += fn_;
        }
        toggle = !toggle;
    }

    let ecap = bound(0, prefs);
    let en = end(cctx, out.as_mut_ptr().add(written) as *mut c_void, ecap, std::ptr::null());
    assert_eq!(ie(en), 0, "end errored (rust={})", use_rust);
    written += en;

    free(cctx);
    out.truncate(written);
    (out, written)
}

#[test]
fn test_header_size() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x4EAd);
    unsafe {
        // Build a frame, then query headerSize on both.
        let prefs = base_prefs();
        let c_cf: libloading::Symbol<CompressFrame> = csym(&libs, b"LZ4F_compressFrame");
        let c_cfb: libloading::Symbol<CompressFrameBound> = csym(&libs, b"LZ4F_compressFrameBound");
        let data = rng.compressible(2000);
        let cb = c_cfb(data.len(), &prefs);
        let mut frame = vec![0u8; cb];
        let n = c_cf(frame.as_mut_ptr() as *mut c_void, cb, data.as_ptr() as *const c_void, data.len(), &prefs);
        let c_hs: libloading::Symbol<HeaderSize> = csym(&libs, b"LZ4F_headerSize");
        let r_hs: libloading::Symbol<HeaderSize> = rsym(&libs, b"LZ4F_headerSize");
        for &avail in &[5usize, 7, 10, n] {
            let cv = c_hs(frame.as_ptr() as *const c_void, avail);
            let rv = r_hs(frame.as_ptr() as *const c_void, avail);
            assert_eq!(cv, rv, "headerSize avail={}", avail);
        }
    }
}

#[test]
fn test_uncompressed_update() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0cba);
    unsafe {
        // uncompressedUpdate only supported with blockIndependent
        let mut prefs = base_prefs();
        prefs.frame_info.block_mode = 1; // independent
        let data = rng.random(10000);

        let cframe = stream_uncompressed(&libs, &prefs, &data, false);
        let rframe = stream_uncompressed(&libs, &prefs, &data, true);
        assert_eq!(cframe, rframe, "uncompressedUpdate frame bytes");
        decode_and_check(&libs, &cframe, &data, false);
    }
}

unsafe fn stream_uncompressed(libs: &Libs, prefs: &Preferences, data: &[u8], use_rust: bool) -> Vec<u8> {
    macro_rules! sym {
        ($t:ty, $name:expr) => {{
            let s: libloading::Symbol<$t> = if use_rust { rsym(libs, $name) } else { csym(libs, $name) };
            s
        }};
    }
    let create = sym!(CreateCctx, b"LZ4F_createCompressionContext");
    let free = sym!(FreeCctx, b"LZ4F_freeCompressionContext");
    let begin = sym!(CompressBegin, b"LZ4F_compressBegin");
    let bound = sym!(CompressBound, b"LZ4F_compressBound");
    let uupdate = sym!(CompressUpdate, b"LZ4F_uncompressedUpdate");
    let end = sym!(CompressEnd, b"LZ4F_compressEnd");
    let ie = sym!(IsError, b"LZ4F_isError");

    let mut cctx: *mut c_void = std::ptr::null_mut();
    assert_eq!(create(&mut cctx, LZ4F_VERSION), 0);
    let chunk = 4096usize;
    let cap = bound(chunk, prefs) + 64;
    let mut out = vec![0u8; 19 + data.len() + cap * (data.len() / chunk + 2) + 128];
    let mut written = 0usize;
    let hdr = begin(cctx, out.as_mut_ptr() as *mut c_void, out.len(), prefs);
    assert_eq!(ie(hdr), 0);
    written += hdr;
    let mut off = 0;
    while off < data.len() {
        let this = chunk.min(data.len() - off);
        let dstcap = bound(this, prefs);
        let n = uupdate(cctx, out.as_mut_ptr().add(written) as *mut c_void, dstcap, data.as_ptr().add(off) as *const c_void, this, std::ptr::null());
        assert_eq!(ie(n), 0, "uncompressedUpdate errored (rust={})", use_rust);
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
