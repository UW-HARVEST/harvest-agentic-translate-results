//! Phase B, row C31 — the EXHAUSTIVE differential test.
//!
//! `tritanopia`'s entire input domain is a 3-byte struct, i.e. 2^24 =
//! 16,777,216 values. Every single one is passed to both shared objects and the
//! three result bytes are compared. This is not a sample: it is a complete
//! proof of behavioural equivalence for the exported API, and it subsumes every
//! other row in `CONFIGS.md`.

mod common;

use common::{Harness, Rgb};

#[test]
fn c31_exhaustive_all_16m_inputs() {
    let h = Harness::new();
    eprintln!("C31: C   .so = {}", h.c_path.display());
    eprintln!("C31: Rust.so = {}", h.rust_path.display());

    let mut mismatches: Vec<(Rgb, Rgb, Rgb)> = Vec::new();
    let mut checked: u64 = 0;

    for r in 0..=255u8 {
        for g in 0..=255u8 {
            for b in 0..=255u8 {
                let x = Rgb::new(r, g, b);
                let cv = h.c(x);
                let rv = h.rust(x);
                checked += 1;
                if cv != rv && mismatches.len() < 32 {
                    mismatches.push((x, cv, rv));
                }
            }
        }
    }

    assert_eq!(checked, 1 << 24, "C31: did not visit the full domain");

    if !mismatches.is_empty() {
        let mut msg = String::from("C31 EXHAUSTIVE DIVERGENCE (first 32):\n");
        for (x, c, r) in &mismatches {
            msg += &format!(
                "  in {{{:>3},{:>3},{:>3}}}  C {{{:>3},{:>3},{:>3}}}  Rust {{{:>3},{:>3},{:>3}}}\n",
                x.r, x.g, x.b, c.r, c.g, c.b, r.r, r.g, r.b
            );
        }
        panic!("{msg}");
    }

    eprintln!("C31: all {checked} inputs byte-identical between the C and Rust .so");
}

/// The same exhaustive sweep through the `u32` ABI view, which also pins down
/// the low three bytes of the return register for every input.
#[test]
fn c31b_exhaustive_raw_abi_view() {
    let h = Harness::new();
    let mut bad = 0u64;
    let mut first: Option<(u32, u32, u32)> = None;
    for word in 0u32..(1u32 << 24) {
        let c = h.c_raw(word) & 0x00FF_FFFF;
        let r = h.rust_raw(word) & 0x00FF_FFFF;
        if c != r {
            bad += 1;
            if first.is_none() {
                first = Some((word, c, r));
            }
        }
    }
    assert_eq!(
        bad, 0,
        "C31b: {bad} divergences through the raw ABI view; first: {first:?}"
    );
    eprintln!("C31b: all 16777216 raw-ABI calls byte-identical");
}
