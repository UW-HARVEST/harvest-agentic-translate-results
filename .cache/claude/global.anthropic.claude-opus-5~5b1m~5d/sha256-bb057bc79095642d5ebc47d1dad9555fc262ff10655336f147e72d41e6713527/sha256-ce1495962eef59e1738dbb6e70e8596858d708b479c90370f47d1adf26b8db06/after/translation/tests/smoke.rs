mod common;
use common::*;

#[test]
fn smoke_loads_both_libs_and_hashes_match() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut data = *b"hello world";
        let hc = (c.hash_bytes)(data.as_mut_ptr() as *mut _, data.len(), 12345);
        let hr = (rs.hash_bytes)(data.as_mut_ptr() as *mut _, data.len(), 12345);
        assert_eq!(hc, hr, "hash_bytes divergence");
        let mut s = *b"hello world\0";
        let sc = (c.hash_string)(s.as_mut_ptr() as *mut _, 999);
        let sr = (rs.hash_string)(s.as_mut_ptr() as *mut _, 999);
        assert_eq!(sc, sr, "hash_string divergence");
        eprintln!("C so   = {}", c.path.display());
        eprintln!("RUST so= {}", rs.path.display());
    });
}
