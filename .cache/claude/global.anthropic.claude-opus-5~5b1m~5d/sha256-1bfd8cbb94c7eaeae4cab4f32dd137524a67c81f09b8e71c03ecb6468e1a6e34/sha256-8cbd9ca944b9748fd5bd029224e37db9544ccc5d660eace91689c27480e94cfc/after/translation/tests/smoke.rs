mod common;
use common::*;
use std::os::raw::{c_int, c_uint};

type FnVersion = unsafe extern "C" fn() -> c_uint;
type FnVersionStr = unsafe extern "C" fn() -> *const std::os::raw::c_char;
type FnCompressBound = unsafe extern "C" fn(usize) -> usize;
type FnCompress =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;

#[test]
fn smoke_version_and_roundtrip() {
    let (cv, rv) = unsafe { pair::<FnVersion>("ZSTD_versionNumber") };
    assert_eq!(unsafe { cv() }, unsafe { rv() }, "ZSTD_versionNumber");

    let (cs, rs) = unsafe { pair::<FnVersionStr>("ZSTD_versionString") };
    let cstr = unsafe { std::ffi::CStr::from_ptr(cs()) }.to_owned();
    let rstr = unsafe { std::ffi::CStr::from_ptr(rs()) }.to_owned();
    assert_eq!(cstr, rstr, "ZSTD_versionString");

    let (cb, rb) = unsafe { pair::<FnCompressBound>("ZSTD_compressBound") };
    for n in [0usize, 1, 100, 131072, 1 << 20] {
        assert_eq!(unsafe { cb(n) }, unsafe { rb(n) }, "compressBound({n})");
    }

    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };

    let mut rng = Rng::new(1);
    let src = gen(Shape::Text, 5000, &mut rng);
    let cap = unsafe { cb(src.len()) };
    let mut cbuf = vec![0u8; cap];
    let mut rbuf = vec![0u8; cap];
    let cn = unsafe { cc(cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), 3) };
    let rn = unsafe { rc(rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), 3) };
    assert_eq!(cn, rn, "compressed size");
    assert!(!is_error(cn));
    assert_bytes_eq("ZSTD_compress output", &cbuf[..cn], &rbuf[..rn]);

    let mut co = vec![0u8; src.len()];
    let mut ro = vec![0u8; src.len()];
    let cdn = unsafe { cd(co.as_mut_ptr(), co.len(), cbuf.as_ptr(), cn) };
    let rdn = unsafe { rd(ro.as_mut_ptr(), ro.len(), rbuf.as_ptr(), rn) };
    assert_eq!(cdn, rdn);
    assert_eq!(cdn, src.len());
    assert_bytes_eq("decompressed", &co[..cdn], &src);
    assert_bytes_eq("decompressed rust", &ro[..rdn], &src);
}
