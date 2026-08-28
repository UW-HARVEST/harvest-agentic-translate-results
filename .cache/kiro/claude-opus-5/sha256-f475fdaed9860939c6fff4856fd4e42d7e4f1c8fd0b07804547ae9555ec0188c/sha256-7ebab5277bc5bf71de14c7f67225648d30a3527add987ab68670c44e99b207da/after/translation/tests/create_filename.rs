//! Top level exported function: `FIO_createFilename_fromOutDir`.
//!
//! The C function returns a `calloc`'d buffer of
//! `strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1` bytes.
//! We compare that *entire* allocation byte-for-byte (calloc zeroes it, so the
//! tail must be zero in both implementations too).

mod common;

use common::{free, GuardedCStr, Libs};
use std::os::raw::{c_char, c_void};

unsafe fn strlen(mut p: *const c_char) -> usize {
    let mut n = 0;
    while unsafe { *p } != 0 {
        n += 1;
        p = unsafe { p.add(1) };
    }
    n
}

fn paths() -> Vec<&'static [u8]> {
    vec![
        b"",
        b"/",
        b"//",
        b"a",
        b"file.txt",
        b"/file.txt",
        b"dir/file.txt",
        b"/abs/dir/file.txt",
        b"./rel/file.txt",
        b"../up/file.txt",
        b"dir/",
        b"dir//file",
        b"C:\\win\\path\\file.txt",
        b"mixed\\sep/path\\file",
        b"\\only\\backslashes",
        b"no-separator",
        b".hidden",
        b"dir/.hidden",
        b"space in name/file name.bin",
        b"\xff\xfe/\x80\x81",
        b"very/long/path/that/keeps/going/and/going/until/the/end/file.zst",
    ]
}

fn out_dirs() -> Vec<&'static [u8]> {
    vec![
        b"",             // triggers the original out-of-bounds read
        b"/",            // ends with separator
        b"out",          // no trailing separator
        b"out/",         // trailing separator
        b"out//",        // double trailing separator
        b"a",
        b"/",
        b"/tmp/out",
        b"/tmp/out/",
        b"..",
        b"../",
        b".",
        b"./",
        b"deep/nested/output/dir",
        b"deep/nested/output/dir/",
        b"win\\style",
        b"win\\style\\",
        b"has space",
        b"has space/",
        b"\x80\xff",
        b"\x80\xff/",
        b"trailing\x2f",
    ]
}

#[test]
fn create_filename_from_out_dir_matches_c() {
    let libs = Libs::load();
    let (c_extract, _) = libs.extract_filename();
    let (c_fn, r_fn) = libs.create_filename();

    let mut cases = 0usize;
    // guard byte preceding outDirName: decides the empty-string branch
    for guard in [b'/', b'x', 0u8, b'\\'] {
        for od in out_dirs() {
            let out_dir = GuardedCStr::new(guard, od);
            for p in paths() {
                let path = GuardedCStr::new(b'z', p);
                for suffix_len in [0usize, 1, 4, 7, 64] {
                    let c_ret = unsafe { c_fn(path.ptr(), out_dir.ptr(), suffix_len) };
                    let r_ret = unsafe { r_fn(path.ptr(), out_dir.ptr(), suffix_len) };
                    assert!(!c_ret.is_null() && !r_ret.is_null());

                    // Allocation size, computed from the C ground truth.
                    let od_len = unsafe { strlen(out_dir.ptr()) };
                    let fname = unsafe { c_extract(path.ptr(), b'/' as i8 as c_char) };
                    let fname_len = unsafe { strlen(fname) };
                    let total = od_len + 1 + fname_len + suffix_len + 1;

                    let c_bytes =
                        unsafe { std::slice::from_raw_parts(c_ret as *const u8, total) };
                    let r_bytes =
                        unsafe { std::slice::from_raw_parts(r_ret as *const u8, total) };
                    assert_eq!(
                        c_bytes,
                        r_bytes,
                        "FIO_createFilename_fromOutDir mismatch:\n  guard={guard:#04x}\n  outDir={:?}\n  path={:?}\n  suffixLen={suffix_len}\n  C   ={:?}\n  Rust={:?}",
                        String::from_utf8_lossy(od),
                        String::from_utf8_lossy(p),
                        String::from_utf8_lossy(c_bytes),
                        String::from_utf8_lossy(r_bytes),
                    );

                    unsafe {
                        free(c_ret as *mut c_void);
                        free(r_ret as *mut c_void);
                    }
                    cases += 1;
                }
            }
        }
    }
    assert!(cases > 0);
    eprintln!("FIO_createFilename_fromOutDir: {cases} cases compared");
}

/// Large `suffixLen` values: only sizing is affected, but the zero tail must
/// still match exactly.
#[test]
fn create_filename_large_suffix_len() {
    let libs = Libs::load();
    let (c_extract, _) = libs.extract_filename();
    let (c_fn, r_fn) = libs.create_filename();

    for suffix_len in [1024usize, 65536, 1 << 20] {
        let out_dir = GuardedCStr::new(b'x', b"/tmp/output");
        let path = GuardedCStr::new(b'z', b"/some/dir/input.tar");
        let c_ret = unsafe { c_fn(path.ptr(), out_dir.ptr(), suffix_len) };
        let r_ret = unsafe { r_fn(path.ptr(), out_dir.ptr(), suffix_len) };
        let od_len = unsafe { strlen(out_dir.ptr()) };
        let fname = unsafe { c_extract(path.ptr(), b'/' as i8 as c_char) };
        let total = od_len + 1 + unsafe { strlen(fname) } + suffix_len + 1;
        let c_bytes = unsafe { std::slice::from_raw_parts(c_ret as *const u8, total) };
        let r_bytes = unsafe { std::slice::from_raw_parts(r_ret as *const u8, total) };
        assert_eq!(c_bytes, r_bytes, "mismatch at suffixLen={suffix_len}");
        unsafe {
            free(c_ret as *mut c_void);
            free(r_ret as *mut c_void);
        }
    }
}

unsafe extern "C" {
    /// glibc: usable size of a heap block. Comparing it between the two
    /// implementations catches divergence in the `calloc` request size, which
    /// the returned bytes alone cannot reveal (the tail is zeroed either way).
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

#[test]
fn create_filename_allocation_size_matches_c() {
    let libs = Libs::load();
    let (c_fn, r_fn) = libs.create_filename();

    // Sizes large enough that glibc does not round the request into a shared
    // size class, so a one-byte difference is observable.
    for suffix_len in [0usize, 1, 4, 7, 64, 1024, 4096, 65536, 1 << 20] {
        for od in [b"".as_slice(), b"/", b"out", b"out/", b"/tmp/output"] {
            for p in [
                b"".as_slice(),
                b"f",
                b"dir/file.txt",
                b"/abs/dir/file.tar.gz",
            ] {
                let out_dir = GuardedCStr::new(b'x', od);
                let path = GuardedCStr::new(b'z', p);
                let c_ret = unsafe { c_fn(path.ptr(), out_dir.ptr(), suffix_len) };
                let r_ret = unsafe { r_fn(path.ptr(), out_dir.ptr(), suffix_len) };
                let c_sz = unsafe { malloc_usable_size(c_ret as *mut c_void) };
                let r_sz = unsafe { malloc_usable_size(r_ret as *mut c_void) };
                assert_eq!(
                    c_sz,
                    r_sz,
                    "allocation size mismatch: outDir={:?} path={:?} suffixLen={suffix_len} (C {c_sz} vs Rust {r_sz})",
                    String::from_utf8_lossy(od),
                    String::from_utf8_lossy(p),
                );
                unsafe {
                    free(c_ret as *mut c_void);
                    free(r_ret as *mut c_void);
                }
            }
        }
    }
}
