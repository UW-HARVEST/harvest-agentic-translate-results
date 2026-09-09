//! Harness self-check: proves the two `.so` files really are distinct objects
//! and that `syms()` returns pointers into each of them (so a divergence would
//! actually be observable).
mod common;
use common::*;

#[test]
fn harness_loads_two_distinct_libraries() {
    let (c, r) = syms::<FnCompressBound>("LZ4_compressBound");
    let ca = c as usize;
    let ra = r as usize;
    assert_ne!(ca, ra, "C and Rust LZ4_compressBound resolved to the SAME address — the harness is only testing one library");

    // Resolve the containing object for each pointer via dladdr-like check:
    // both files must exist and differ in size/content.
    let cbytes = std::fs::read(C_SO).expect("C .so readable");
    let rbytes = std::fs::read(RS_SO).expect("Rust .so readable");
    assert_ne!(cbytes, rbytes, "the two .so paths point at identical bytes");
    eprintln!(
        "harness OK: C .so {} bytes, Rust .so {} bytes; compressBound @ {:#x} vs {:#x}",
        cbytes.len(),
        rbytes.len(),
        ca,
        ra
    );
}

/// Negative control: a symbol we deliberately mis-name must fail to resolve in
/// BOTH libraries, proving `syms()` is not silently succeeding.
#[test]
#[should_panic(expected = "missing")]
fn harness_rejects_unknown_symbol() {
    let _ = syms::<FnCompressBound>("LZ4_this_symbol_does_not_exist");
}

/// Cross-check: the Rust `.so` output must round-trip through the C decoder and
/// vice-versa. If the harness were accidentally calling one library twice this
/// would still pass, so it is combined with the address check above.
#[test]
fn cross_library_roundtrip() {
    let (cc, rc) = syms::<FnCompressDefault>("LZ4_compress_default");
    let (cd, rd) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..200 {
        let len = rng.range(0, 5000);
        let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
        let bound = unsafe { cb(len as i32) }.max(1) as usize;
        let mut cbuf = vec![0u8; bound];
        let mut rbuf = vec![0u8; bound];
        let cn = unsafe { cc(src.as_ptr(), cbuf.as_mut_ptr(), len as i32, bound as i32) };
        let rn = unsafe { rc(src.as_ptr(), rbuf.as_mut_ptr(), len as i32, bound as i32) };
        assert_eq!(cn, rn);
        // Rust output -> C decoder
        let mut out = vec![0u8; len + 8];
        let n = unsafe { cd(rbuf.as_ptr(), out.as_mut_ptr(), rn, (len + 8) as i32) };
        assert_eq!(n, len as i32, "Rust-compressed block rejected by C decoder");
        assert_eq!(&out[..len], &src[..]);
        // C output -> Rust decoder
        let mut out = vec![0u8; len + 8];
        let n = unsafe { rd(cbuf.as_ptr(), out.as_mut_ptr(), cn, (len + 8) as i32) };
        assert_eq!(n, len as i32, "C-compressed block rejected by Rust decoder");
        assert_eq!(&out[..len], &src[..]);
    }
}
