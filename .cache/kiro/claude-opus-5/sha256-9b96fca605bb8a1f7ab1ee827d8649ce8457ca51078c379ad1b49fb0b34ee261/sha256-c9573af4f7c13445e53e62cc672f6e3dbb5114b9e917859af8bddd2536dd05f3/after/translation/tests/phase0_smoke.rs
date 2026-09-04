//! Phase 0 — harness smoke test: both `.so`s load and the simplest one-shot
//! round trip agrees byte-for-byte.

mod common;
use common::*;
use std::os::raw::c_void;

#[test]
fn smoke_version() {
    let (c, r) = fnpair!("ZSTD_versionNumber", unsafe extern "C" fn() -> u32);
    unsafe { assert_eq!(c(), r(), "ZSTD_versionNumber") };
    let (c, r) = fnpair!(
        "ZSTD_versionString",
        unsafe extern "C" fn() -> *const std::os::raw::c_char
    );
    unsafe { assert_eq!(cstr(c()), cstr(r()), "ZSTD_versionString") };
}

#[test]
fn smoke_compress_roundtrip() {
    let (cc, rc) = fnpair!("ZSTD_compress", FnCompress);
    let (cd, rd) = fnpair!("ZSTD_decompress", FnDecompress);
    let (cb, rb) = fnpair!("ZSTD_compressBound", FnSizeSize);

    let mut rng = Rng::new(1);
    let src = gen(Shape::Text, 4096, &mut rng);

    unsafe {
        assert_eq!(cb(src.len()), rb(src.len()), "compressBound");
        let cap = cb(src.len());
        let mut a = vec![0u8; cap];
        let mut b = vec![0u8; cap];
        for lvl in [1, 3, 9, 19] {
            let na = cc(
                a.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                lvl,
            );
            let nb = rc(
                b.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                lvl,
            );
            assert_eq!(na, nb, "compress size @lvl{lvl}");
            assert_bytes_eq(&format!("compress bytes @lvl{lvl}"), &a[..na], &b[..nb]);

            // cross-decompress: C output through Rust and vice versa
            let mut o1 = vec![0u8; src.len()];
            let mut o2 = vec![0u8; src.len()];
            let d1 = cd(
                o1.as_mut_ptr() as *mut c_void,
                o1.len(),
                b.as_ptr() as *const c_void,
                nb,
            );
            let d2 = rd(
                o2.as_mut_ptr() as *mut c_void,
                o2.len(),
                a.as_ptr() as *const c_void,
                na,
            );
            assert_eq!(d1, src.len());
            assert_eq!(d2, src.len());
            assert_bytes_eq("roundtrip", &src, &o1);
            assert_bytes_eq("roundtrip", &src, &o2);
        }
    }
}
