//! Harness self-checks: prove the differential tests really compare two
//! distinct libraries and would actually notice a divergence.

mod common;

use common::{Pair, c_so_path, rust_so_path};

/// The two loaded objects must be different files, and both must resolve
/// `crc16` to different addresses (i.e. we are not accidentally comparing the C
/// library against itself).
#[test]
fn loads_two_distinct_libraries() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.exists(), "C .so missing: {}", c.display());
    assert!(r.exists(), "Rust .so missing: {}", r.display());
    assert_ne!(
        std::fs::canonicalize(&c).unwrap(),
        std::fs::canonicalize(&r).unwrap(),
        "C and Rust .so must be distinct files"
    );
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());

    let p = Pair::load();
    let ca = p.c as usize;
    let ra = p.r as usize;
    assert_ne!(ca, ra, "crc16 resolved to the same address in both libraries");
}

/// Negative control: the comparison helper must catch a real divergence.
/// We emulate a "wrong" Rust implementation by feeding the Rust export a
/// deliberately different seed and checking the results differ (which is what
/// the assertion inside `both` would report).
#[test]
fn detects_divergence() {
    let p = Pair::load();
    let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
    let good = unsafe { (p.r)(data.as_ptr(), 9, 0x0000) };
    let bad = unsafe { (p.r)(data.as_ptr(), 9, 0x0001) };
    assert_ne!(
        good, bad,
        "sanity: different seeds must give different CRCs, otherwise the \
         differential assertions could never fail"
    );
    let c_good = unsafe { (p.c)(data.as_ptr(), 9, 0x0000) };
    assert_eq!(c_good, good);
    assert_ne!(c_good, bad);
}

/// An independent, from-scratch bit-wise CRC-16/FLAC (poly 0x8005, MSB-first,
/// init = seed, no reflection, no final xor) must agree with BOTH libraries.
/// This pins down the algorithm itself, not just C-vs-Rust agreement.
#[test]
fn matches_independent_bitwise_reference() {
    fn reference(data: &[u8], mut crc: u16) -> u16 {
        for &b in data {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ 0x8005;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    let p = Pair::load();
    let mut rng = common::Rng::new(0xC0FFEE);
    for len in 0u32..200 {
        let data = rng.bytes(len as usize);
        for &seed in &[0x0000u16, 0xFFFF, 0x1234, rng.next_u16()] {
            let v = p.both(&data, len, seed);
            assert_eq!(
                v,
                reference(&data, seed),
                "len={len} seed={seed:#06x}: libraries disagree with the \
                 independent bitwise CRC-16 reference"
            );
        }
    }

    // Published check value for CRC-16/UMTS a.k.a. CRC-16/BUYPASS (poly 0x8005,
    // init 0x0000, no reflection, no final xor) over the ASCII string
    // "123456789" is 0xFEE8 — which is exactly this library's CRC.
    let msg = b"123456789";
    assert_eq!(p.both(msg, msg.len() as u32, 0x0000), 0xFEE8);
}

/// The C build produces a shared library only (see `c_src/CMakeLists.txt`:
/// a single `add_library(... SHARED src/lib.c)`), so there is no driver binary
/// whose stdout could be compared. This test documents/asserts that fact so the
/// Phase B "binary stdout" requirement is provably N/A.
#[test]
fn c_project_has_no_driver_binary() {
    let cml = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/CMakeLists.txt");
    let txt = std::fs::read_to_string(&cml).expect("read CMakeLists.txt");
    assert!(
        !txt.contains("add_executable"),
        "CMakeLists.txt now builds an executable — stdout differential testing \
         would be required"
    );
    assert!(txt.contains("add_library"));
}
