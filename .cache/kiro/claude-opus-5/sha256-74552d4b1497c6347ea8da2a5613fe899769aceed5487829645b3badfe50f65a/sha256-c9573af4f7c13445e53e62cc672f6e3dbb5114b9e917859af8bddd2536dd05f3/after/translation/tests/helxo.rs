//! CONFIGS rows 73/74: `helxo`, whose only observable is what it `printf`s.
//!
//! This lives in its own test binary containing exactly ONE test: capturing
//! `helxo`'s output requires redirecting fd 1 for the whole process, so nothing
//! else — including libtest's own progress output — may write to stdout while a
//! capture is in flight.

mod common;
use common::*;
use std::ffi::c_char;

// --- rows 73/74: helxo ---------------------------------------------------

fn helxo_outputs(letter: u8) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = libs();
    let out_c = capture_stdout("c", || unsafe { (c.helxo)(letter as c_char) });
    let out_r = capture_stdout("r", || unsafe { (r.helxo)(letter as c_char) });
    (out_c, out_r)
}

fn row73_helxo_all_256_bytes() {
    for b in 0u16..=255 {
        set_seed(DEFAULT_SEED);
        let (oc, or) = helxo_outputs(b as u8);
        assert_eq!(
            oc, or,
            "row73: helxo({b:#04x}) stdout differs\n C={:?}\n R={:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or)
        );
        // the demo builds "hel<letter>o" one line per entry
        let mut want = Vec::new();
        want.extend_from_slice(b"bob h\nsally e\nfred l\njen ");
        want.push(b as u8);
        want.extend_from_slice(b"\ndoug o\n");
        assert_eq!(oc, want, "row73: helxo({b:#04x}) unexpected C output");
    }
}

fn row74_helxo_under_various_seeds() {
    for &seed in &[0usize, 1, 0x31415926, usize::MAX, 0xdead_beef, 0xa5a5_a5a5_a5a5_a5a5] {
        for &letter in &[b'l', b'o', 0u8, 0xff, b'A'] {
            set_seed(seed);
            let (oc, or) = helxo_outputs(letter);
            assert_eq!(oc, or, "row74: seed={seed:#x} letter={letter:#04x}");
            // insertion order (hence output order) must be seed-independent
            let mut want = Vec::new();
            want.extend_from_slice(b"bob h\nsally e\nfred l\njen ");
            want.push(letter);
            want.extend_from_slice(b"\ndoug o\n");
            assert_eq!(oc, want, "row74: seed={seed:#x} letter={letter:#04x} content");
        }
    }
    set_seed(DEFAULT_SEED);
}

fn row74b_helxo_repeated_calls_same_process() {
    // helxo advances each library's global hash seed (one table per call), so a
    // long run checks that the two seed streams stay in lock-step.
    set_seed(DEFAULT_SEED);
    for i in 0..40u8 {
        let letter = b'a' + (i % 26);
        let (oc, or) = helxo_outputs(letter);
        assert_eq!(oc, or, "row74b: call #{i}");
    }
    set_seed(DEFAULT_SEED);
}


#[test]
fn helxo_rows_73_and_74() {
    let _g = serial();
    row73_helxo_all_256_bytes();
    row74_helxo_under_various_seeds();
    row74b_helxo_repeated_calls_same_process();
}
