//! Harness self-check plus the exhaustive 256-value differential sweep.
//!
//! If this file fails, everything else is untrustworthy, so it also asserts the
//! harness itself works (capture is non-empty, both symbols resolve, the two
//! `.so`s really are two distinct objects).

mod common;

use common::*;
use std::ffi::c_char;

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let l = libs();
    assert_ne!(
        l.c_path.canonicalize().unwrap(),
        l.rust_path.canonicalize().unwrap(),
        "the C and Rust paths must be different objects"
    );
    // Resolving through libloading proves the export wrappers exist.
    let _ = c_driver();
    let _ = rust_driver();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.rust_path.display());
}

#[test]
fn harness_capture_actually_captures() {
    let cd = c_driver();
    let out = capture(|| unsafe { cd(b'A' as c_char) });
    assert!(!out.is_empty(), "capture returned nothing");
    let text = String::from_utf8_lossy(&out).to_string();
    // 14 printf lines, in the order driver.c emits them.
    let want_prefixes = [
        "alphanumeric: ",
        "alphabetic: ",
        "lowercase: ",
        "uppercase: ",
        "digit: ",
        "hexadecimal: ",
        "control: ",
        "graphical: ",
        "space: ",
        "blank: ",
        "printing: ",
        "punctuation: ",
        "to lower: ",
        "to upper: ",
    ];
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 14, "expected 14 lines, got {text:?}");
    for (line, want) in lines.iter().zip(want_prefixes) {
        assert!(line.starts_with(want), "line {line:?} should start {want:?}");
    }
}

/// The single most important sweep: every one of the 256 `char` bit patterns.
#[test]
fn exhaustive_all_256_char_values() {
    diff_all_chars("exhaustive sweep");
}

/// Same sweep, but with the ordering shuffled by a seeded PRNG, so any
/// dependence on call order (cached locale state, lazily initialised tables)
/// shows up rather than being masked by the monotone 0..255 walk.
#[test]
fn exhaustive_all_256_shuffled_order() {
    let mut order: Vec<u8> = (0u16..=255).map(|v| v as u8).collect();
    let mut rng = Rng::new(SEED ^ 0xA11);
    // Fisher-Yates.
    for i in (1..order.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    for v in order {
        diff_char(v as c_char, "shuffled sweep");
    }
}

/// Sanity check on the ground truth itself: the C `.so` must report the *raw*
/// masked ctype bits, not a normalised 0/1. This pins down the property the
/// translation had to replicate, straight from the C, so a later "cleanup" of
/// the Rust into `1`/`0` cannot pass unnoticed.
#[test]
fn c_ground_truth_returns_raw_ctype_masks() {
    let cd = c_driver();
    let out = capture(|| unsafe { cd(b'0' as c_char) });
    let text = String::from_utf8_lossy(&out).to_string();
    let get = |k: &str| -> i64 {
        text.lines()
            .find(|l| l.starts_with(&format!("{k}: ")))
            .unwrap_or_else(|| panic!("no {k} line in {text:?}"))
            [k.len() + 2..]
            .trim()
            .parse()
            .unwrap()
    };
    assert_eq!(get("digit"), 2048, "_ISdigit raw mask");
    assert_eq!(get("hexadecimal"), 4096, "_ISxdigit raw mask");
    assert_eq!(get("alphanumeric"), 8, "_ISalnum raw mask");
    assert_eq!(get("graphical"), 32768, "_ISgraph raw mask");
    assert_eq!(get("printing"), 16384, "_ISprint raw mask");
    assert_eq!(get("alphabetic"), 0);
    assert_eq!(get("control"), 0);

    let out = capture(|| unsafe { cd(0) });
    let text = String::from_utf8_lossy(&out).to_string();
    let get0 = |k: &str| -> i64 {
        text.lines()
            .find(|l| l.starts_with(&format!("{k}: ")))
            .unwrap()[k.len() + 2..]
            .trim()
            .parse()
            .unwrap()
    };
    assert_eq!(get0("control"), 2, "_IScntrl raw mask for NUL");
}
