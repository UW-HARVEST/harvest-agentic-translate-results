// Phase B/C — differential tests for the LZ4 block API (lz4.c / lz4hc.c).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

type CompressDefault =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type CompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type CompressBound = unsafe extern "C" fn(c_int) -> c_int;
type DecompressSafe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type DecompressPartial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type CompressDestSize =
    unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type CompressExtState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type SizeofState = unsafe extern "C" fn() -> c_int;
type CompressHC =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;

fn sizes() -> Vec<usize> {
    vec![0, 1, 2, 3, 15, 16, 17, 63, 64, 100, 255, 256, 1000, 4096, 65535, 65536, 100000]
}

#[test]
fn test_compress_bound() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let r: libloading::Symbol<CompressBound> = rsym(&libs, b"LZ4_compressBound");
        for sz in [-1, 0, 1, 100, 65536, 0x7E000000i32, 0x7E000001i32, i32::MAX] {
            assert_eq!(c(sz), r(sz), "compressBound({})", sz);
        }
    }
}

#[test]
fn test_version() {
    let libs = Libs::load();
    unsafe {
        let cv: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
            csym(&libs, b"LZ4_versionNumber");
        let rv: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
            rsym(&libs, b"LZ4_versionNumber");
        assert_eq!(cv(), rv());

        let cs: libloading::Symbol<unsafe extern "C" fn() -> *const c_char> =
            csym(&libs, b"LZ4_versionString");
        let rs: libloading::Symbol<unsafe extern "C" fn() -> *const c_char> =
            rsym(&libs, b"LZ4_versionString");
        let cstr = std::ffi::CStr::from_ptr(cs());
        let rstr = std::ffi::CStr::from_ptr(rs());
        assert_eq!(cstr, rstr);
    }
}

// Helper: compress with both, compare compressed bytes, then decompress both & compare.
unsafe fn roundtrip_default(libs: &Libs, input: &[u8]) {
    let cc: libloading::Symbol<CompressDefault> = csym(libs, b"LZ4_compress_default");
    let rc: libloading::Symbol<CompressDefault> = rsym(libs, b"LZ4_compress_default");
    let cbound: libloading::Symbol<CompressBound> = csym(libs, b"LZ4_compressBound");
    let cap = cbound(input.len() as c_int).max(1) as usize;

    let mut cdst = vec![0u8; cap];
    let mut rdst = vec![0u8; cap];
    let src = input.as_ptr() as *const c_char;
    let cn = cc(src, cdst.as_mut_ptr() as *mut c_char, input.len() as c_int, cap as c_int);
    let rn = rc(src, rdst.as_mut_ptr() as *mut c_char, input.len() as c_int, cap as c_int);
    assert_eq!(cn, rn, "compress_default returns differ (len={})", input.len());
    assert!(cn > 0 || input.is_empty(), "compress failed len={}", input.len());
    assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "compressed bytes differ len={}", input.len());

    // decompress with both from the C compressed buffer
    let cds: libloading::Symbol<DecompressSafe> = csym(libs, b"LZ4_decompress_safe");
    let rds: libloading::Symbol<DecompressSafe> = rsym(libs, b"LZ4_decompress_safe");
    let dcap = input.len().max(1);
    let mut cout = vec![0u8; dcap];
    let mut rout = vec![0u8; dcap];
    let cdn = cds(cdst.as_ptr() as *const c_char, cout.as_mut_ptr() as *mut c_char, cn, dcap as c_int);
    let rdn = rds(cdst.as_ptr() as *const c_char, rout.as_mut_ptr() as *mut c_char, cn, dcap as c_int);
    assert_eq!(cdn, rdn, "decompress returns differ");
    assert_eq!(cdn as usize, input.len(), "decompressed size mismatch");
    assert_eq!(&cout[..cdn as usize], input, "C decompress != original");
    assert_eq!(&rout[..rdn as usize], input, "Rust decompress != original");
}

#[test]
fn test_compress_default_roundtrip() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xABCDEF);
    unsafe {
        for &sz in sizes().iter() {
            for _ in 0..8 {
                roundtrip_default(&libs, &rng.compressible(sz));
                roundtrip_default(&libs, &rng.random(sz));
            }
        }
    }
}

#[test]
fn test_compress_fast_acceleration() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x1234);
    unsafe {
        let cc: libloading::Symbol<CompressFast> = csym(&libs, b"LZ4_compress_fast");
        let rc: libloading::Symbol<CompressFast> = rsym(&libs, b"LZ4_compress_fast");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        for accel in [-5i32, 0, 1, 2, 10, 1000, 65537, 100000, i32::MAX] {
            for &sz in &[0usize, 1, 100, 4096, 65536] {
                let input = rng.compressible(sz);
                let cap = cbound(sz as c_int).max(1) as usize;
                let mut cdst = vec![0u8; cap];
                let mut rdst = vec![0u8; cap];
                let cn = cc(input.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, accel);
                let rn = rc(input.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, accel);
                assert_eq!(cn, rn, "compress_fast accel={} sz={}", accel, sz);
                assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "bytes accel={} sz={}", accel, sz);
            }
        }
    }
}

#[test]
fn test_compress_fast_extState() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x777);
    unsafe {
        let css: libloading::Symbol<SizeofState> = csym(&libs, b"LZ4_sizeofState");
        let rss: libloading::Symbol<SizeofState> = rsym(&libs, b"LZ4_sizeofState");
        assert_eq!(css(), rss(), "sizeofState");
        let state_sz = css() as usize;

        for name in [&b"LZ4_compress_fast_extState"[..], &b"LZ4_compress_fast_extState_fastReset"[..]] {
            let cc: libloading::Symbol<CompressExtState> = csym(&libs, name);
            let rc: libloading::Symbol<CompressExtState> = rsym(&libs, name);
            let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
            for &sz in &[0usize, 1, 100, 4096, 20000] {
                for accel in [1i32, 5, 100] {
                    let input = rng.compressible(sz);
                    let cap = cbound(sz as c_int).max(1) as usize;
                    let mut cstate = vec![0u8; state_sz + 16];
                    let mut rstate = vec![0u8; state_sz + 16];
                    let mut cdst = vec![0u8; cap];
                    let mut rdst = vec![0u8; cap];
                    let cn = cc(cstate.as_mut_ptr() as *mut c_void, input.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, accel);
                    let rn = rc(rstate.as_mut_ptr() as *mut c_void, input.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, accel);
                    assert_eq!(cn, rn, "{:?} sz={} accel={}", String::from_utf8_lossy(name), sz, accel);
                    assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "bytes {:?} sz={}", String::from_utf8_lossy(name), sz);
                }
            }
        }
    }
}

#[test]
fn test_compress_destSize() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x999);
    unsafe {
        let cc: libloading::Symbol<CompressDestSize> = csym(&libs, b"LZ4_compress_destSize");
        let rc: libloading::Symbol<CompressDestSize> = rsym(&libs, b"LZ4_compress_destSize");
        for &sz in &[0usize, 1, 100, 4096, 30000] {
            for &dstcap in &[0usize, 1, 8, 64, 500, 5000] {
                let input = rng.compressible(sz);
                let mut csrc = sz as c_int;
                let mut rsrc = sz as c_int;
                let mut cdst = vec![0u8; dstcap.max(1)];
                let mut rdst = vec![0u8; dstcap.max(1)];
                let cn = cc(input.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, &mut csrc, dstcap as c_int);
                let rn = rc(input.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, &mut rsrc, dstcap as c_int);
                assert_eq!(cn, rn, "destSize ret sz={} cap={}", sz, dstcap);
                assert_eq!(csrc, rsrc, "destSize srcConsumed sz={} cap={}", sz, dstcap);
                assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "destSize bytes sz={} cap={}", sz, dstcap);
            }
        }
    }
}

#[test]
fn test_decompress_safe_partial() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x555);
    unsafe {
        let cc: libloading::Symbol<CompressDefault> = csym(&libs, b"LZ4_compress_default");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let cp: libloading::Symbol<DecompressPartial> = csym(&libs, b"LZ4_decompress_safe_partial");
        let rp: libloading::Symbol<DecompressPartial> = rsym(&libs, b"LZ4_decompress_safe_partial");
        for &sz in &[1usize, 100, 4096, 20000] {
            let input = rng.compressible(sz);
            let cap = cbound(sz as c_int).max(1) as usize;
            let mut comp = vec![0u8; cap];
            let cn = cc(input.as_ptr() as *const c_char, comp.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
            for &target in &[0usize, 1, sz / 2, sz, sz + 10] {
                let dcap = sz;
                let mut cout = vec![0u8; dcap.max(1)];
                let mut rout = vec![0u8; dcap.max(1)];
                let cr = cp(comp.as_ptr() as *const c_char, cout.as_mut_ptr() as *mut c_char, cn, target as c_int, dcap as c_int);
                let rr = rp(comp.as_ptr() as *const c_char, rout.as_mut_ptr() as *mut c_char, cn, target as c_int, dcap as c_int);
                assert_eq!(cr, rr, "partial ret sz={} target={}", sz, target);
                if cr > 0 {
                    assert_eq!(&cout[..cr as usize], &rout[..rr as usize], "partial bytes sz={} target={}", sz, target);
                }
            }
        }
    }
}

#[test]
fn test_compress_hc_levels() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0C42);
    unsafe {
        let cc: libloading::Symbol<CompressHC> = csym(&libs, b"LZ4_compress_HC");
        let rc: libloading::Symbol<CompressHC> = rsym(&libs, b"LZ4_compress_HC");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let cds: libloading::Symbol<DecompressSafe> = csym(&libs, b"LZ4_decompress_safe");
        for level in [-1i32, 0, 1, 2, 3, 6, 9, 10, 11, 12, 15, 100] {
            for &sz in &[0usize, 1, 100, 4096, 30000] {
                let input = rng.compressible(sz);
                let cap = cbound(sz as c_int).max(1) as usize;
                let mut cdst = vec![0u8; cap];
                let mut rdst = vec![0u8; cap];
                let cn = cc(input.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                let rn = rc(input.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                assert_eq!(cn, rn, "HC ret level={} sz={}", level, sz);
                assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "HC bytes level={} sz={}", level, sz);
                // verify roundtrip
                if cn > 0 {
                    let mut out = vec![0u8; sz.max(1)];
                    let dn = cds(cdst.as_ptr() as *const c_char, out.as_mut_ptr() as *mut c_char, cn, sz as c_int);
                    assert_eq!(dn as usize, sz);
                    assert_eq!(&out[..dn as usize], &input[..]);
                }
            }
        }
    }
}
