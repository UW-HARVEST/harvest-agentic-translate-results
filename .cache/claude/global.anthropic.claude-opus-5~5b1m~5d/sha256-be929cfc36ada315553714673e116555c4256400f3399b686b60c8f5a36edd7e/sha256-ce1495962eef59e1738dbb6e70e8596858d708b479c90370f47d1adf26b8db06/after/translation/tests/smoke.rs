mod common;
use common::*;

#[test]
fn smoke_libraries_load_and_export_everything() {
    with_libs(DEFAULT_SEED, |c, r| {
        assert_eq!(c.name, "C");
        assert_eq!(r.name, "RUST");
        eprintln!("C   .so = {}", c.path.display());
        eprintln!("RUST.so = {}", r.path.display());
    });
}

#[test]
fn smoke_hash_bytes_matches() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut buf = *b"hello world 1234";
        let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut _, buf.len(), 0x1234);
        let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut _, buf.len(), 0x1234);
        assert_eq!(hc, hr, "hash_bytes mismatch");
    });
}

#[test]
fn smoke_intmap_roundtrip() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..20 {
            let mut elem = [0u8; 8];
            elem[0..4].copy_from_slice(&k.to_ne_bytes());
            elem[4..8].copy_from_slice(&(k * 3).to_ne_bytes());
            p.put_struct(&k.to_ne_bytes(), &elem, HM_BINARY, &format!("put {k}"));
        }
        for k in 0i32..25 {
            p.geti(&k.to_ne_bytes(), HM_BINARY, &format!("get {k}"));
        }
        p.free("free");
    });
}
