// Phase B/C — LZ4 File API (lz4file.c) differential tests via FILE* on tmpfiles.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

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
        frame_info: FrameInfo { block_size_id: 0, block_mode: 0, content_checksum: 0, frame_type: 0, content_size: 0, dict_id: 0, block_checksum: 0 },
        compression_level: 0,
        auto_flush: 0,
        favor_dec_speed: 0,
        reserved: [0; 3],
    }
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

type IsError = unsafe extern "C" fn(usize) -> c_uint;
type WriteOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type WriteClose = unsafe extern "C" fn(*mut c_void) -> usize;
type ReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type Read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type ReadClose = unsafe extern "C" fn(*mut c_void) -> usize;

unsafe fn cpath(p: &str) -> std::ffi::CString {
    std::ffi::CString::new(p).unwrap()
}

/// Compress `data` to a file using the file API (C or Rust), returning the raw file bytes.
unsafe fn file_compress(libs: &Libs, use_rust: bool, prefs: &Preferences, data: &[u8], path: &str) -> Vec<u8> {
    macro_rules! sym {
        ($t:ty, $name:expr) => {{
            let s: libloading::Symbol<$t> = if use_rust { rsym(libs, $name) } else { csym(libs, $name) };
            s
        }};
    }
    let wopen = sym!(WriteOpen, b"LZ4F_writeOpen");
    let write = sym!(Write, b"LZ4F_write");
    let wclose = sym!(WriteClose, b"LZ4F_writeClose");
    let ie = sym!(IsError, b"LZ4F_isError");

    let cp = cpath(path);
    let mode = cpath("wb");
    let fp = fopen(cp.as_ptr(), mode.as_ptr());
    assert!(!fp.is_null(), "fopen wb failed");
    let mut handle: *mut c_void = std::ptr::null_mut();
    let e = wopen(&mut handle, fp, prefs);
    assert_eq!(ie(e), 0, "writeOpen errored (rust={})", use_rust);
    // write in chunks
    let chunk = 7000usize;
    let mut off = 0;
    while off < data.len() {
        let this = chunk.min(data.len() - off);
        let n = write(handle, data.as_ptr().add(off) as *const c_void, this);
        assert_eq!(ie(n), 0, "write errored (rust={})", use_rust);
        off += this;
    }
    let e = wclose(handle);
    assert_eq!(ie(e), 0, "writeClose errored (rust={})", use_rust);
    fclose(fp);
    std::fs::read(path).unwrap()
}

/// Decompress a file using the file API (C or Rust).
/// Returns (readOpen_is_error, decoded bytes). readOpen legitimately errors when
/// the file is smaller than LZ4F_HEADER_SIZE_MAX (19) bytes — both libs must agree.
unsafe fn file_decompress(libs: &Libs, use_rust: bool, path: &str, expected_len: usize) -> (bool, Vec<u8>) {
    macro_rules! sym {
        ($t:ty, $name:expr) => {{
            let s: libloading::Symbol<$t> = if use_rust { rsym(libs, $name) } else { csym(libs, $name) };
            s
        }};
    }
    let ropen = sym!(ReadOpen, b"LZ4F_readOpen");
    let read = sym!(Read, b"LZ4F_read");
    let rclose = sym!(ReadClose, b"LZ4F_readClose");
    let ie = sym!(IsError, b"LZ4F_isError");

    let cp = cpath(path);
    let mode = cpath("rb");
    let fp = fopen(cp.as_ptr(), mode.as_ptr());
    assert!(!fp.is_null(), "fopen rb failed");
    let mut handle: *mut c_void = std::ptr::null_mut();
    let e = ropen(&mut handle, fp);
    if ie(e) != 0 {
        fclose(fp);
        return (true, Vec::new());
    }
    let mut out = vec![0u8; expected_len + 100];
    let mut total = 0usize;
    let chunk = 5000usize;
    loop {
        let want = chunk.min(out.len() - total);
        if want == 0 { break; }
        let got = read(handle, out.as_mut_ptr().add(total) as *mut c_void, want);
        assert_eq!(ie(got), 0, "read errored (rust={})", use_rust);
        if got == 0 { break; }
        total += got;
    }
    let e = rclose(handle);
    assert_eq!(ie(e), 0, "readClose errored (rust={})", use_rust);
    fclose(fp);
    out.truncate(total);
    (false, out)
}

#[test]
fn test_file_roundtrip() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xf11e);
    let dir = std::env::temp_dir();
    unsafe {
        for (i, &sz) in [0usize, 1, 1000, 50000, 200000].iter().enumerate() {
            for &bm in &[0u32, 1] {
                let mut prefs = base_prefs();
                prefs.frame_info.block_mode = bm;
                prefs.frame_info.content_checksum = 1;
                let data = rng.compressible(sz);

                let cpath = dir.join(format!("lz4_c_{}_{}.lz4", i, bm));
                let rpath = dir.join(format!("lz4_r_{}_{}.lz4", i, bm));
                let cps = cpath.to_str().unwrap();
                let rps = rpath.to_str().unwrap();

                let cbytes = file_compress(&libs, false, &prefs, &data, cps);
                let rbytes = file_compress(&libs, true, &prefs, &data, rps);
                assert_eq!(cbytes, rbytes, "file bytes differ sz={} bm={}", sz, bm);

                // Cross-decode: C reads Rust's file, Rust reads C's file.
                let (c_err, c_dec) = file_decompress(&libs, false, rps, sz);
                let (r_err, r_dec) = file_decompress(&libs, true, cps, sz);
                assert_eq!(c_err, r_err, "readOpen error-agreement sz={} bm={}", sz, bm);
                if !c_err {
                    assert_eq!(c_dec, data, "C file decode mismatch sz={}", sz);
                    assert_eq!(r_dec, data, "Rust file decode mismatch sz={}", sz);
                }

                let _ = std::fs::remove_file(&cpath);
                let _ = std::fs::remove_file(&rpath);
            }
        }
    }
}
