//! Level 3: the public entry point `helxo`, which is the only function declared
//! in `include/lib.h`. Its observable behaviour is what it prints, so fd 1 is
//! redirected and the raw bytes produced by the C `.so` and the Rust `.so` are
//! compared.

mod common;

use common::*;
use std::ffi::c_char;

fn helxo_output(im: &Impl, letter: c_char) -> Vec<u8> {
    capture_stdout(&format!("{}_{}", im.name, letter as i32), || unsafe {
        (im.helxo)(letter)
    })
}

#[test]
fn helxo_output_matches_for_every_char() {
    let (c, r) = load_pair();
    let _g = seeded(&c, &r, 0x3141_5926);

    for v in c_char::MIN..=c_char::MAX {
        let co = helxo_output(&c, v);
        let ro = helxo_output(&r, v);
        assert_same(&format!("helxo({v})"), &co, &ro);
        // Sanity: the demo prints one line per surviving entry ("jen" is
        // inserted twice), and `letter` itself may be a newline or a NUL.
        assert!(
            co.starts_with(b"bob h\nsally e\nfred l\njen "),
            "helxo({v}) unexpected output: {:?}",
            String::from_utf8_lossy(&co)
        );
        assert!(co.ends_with(b"doug o\n"));
    }
}

#[test]
fn helxo_repeated_calls_match() {
    let (c, r) = load_pair();
    let _g = seeded(&c, &r, 0x3141_5926);

    // Repeated invocations advance each library's global `stbds_hash_seed`, so
    // the sequence of outputs must stay in lock-step too.
    let mut co = Vec::new();
    let mut ro = Vec::new();
    for i in 0..40u8 {
        let letter = (b'a' + i % 26) as c_char;
        co.extend_from_slice(&helxo_output(&c, letter));
        ro.extend_from_slice(&helxo_output(&r, letter));
    }
    assert_same("helxo repeated", &co, &ro);
}

/// The same check but starting from a range of different global seeds, so the
/// hash tables built inside `helxo` take different shapes.
#[test]
fn helxo_matches_across_seeds() {
    let (c, r) = load_pair();
    for seed in [
        0usize,
        1,
        2,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xdead_beef_cafe_babe,
        12345,
    ] {
        let _g = seeded(&c, &r, seed);
        for letter in [b'A' as c_char, b'z' as c_char, 0, -1, 127, -128] {
            let co = helxo_output(&c, letter);
            let ro = helxo_output(&r, letter);
            assert_same(&format!("helxo({letter}) seed={seed:#x}"), &co, &ro);
        }
    }
}
