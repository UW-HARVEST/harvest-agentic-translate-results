//! Phase B — CONFIGS.md rows 49 + 50: `helxo`, the `lib.h` entry point.
//!
//! `helxo` writes to `stdout`, which is captured at the *file-descriptor* level
//! (both libraries share the process' libc `stdout`). The whole row pair lives in
//! a single `#[test]` inside its own test binary so that no other test thread can
//! write to fd 1 while it is redirected.

mod common;

use common::*;

#[test]
fn cfg_49_50_helxo() {
    let (c, r, _g) = libs();
    unsafe {
        for byte in 0..=255u8 {
            let letter = byte as i8 as std::ffi::c_char;
            // identical global hash seed for both libraries before each call
            (c.rand_seed)(0x3141_5926);
            let oc = capture_stdout("c", || (c.helxo)(letter));
            (r.rand_seed)(0x3141_5926);
            let or_ = capture_stdout("r", || (r.helxo)(letter));
            assert_eq!(
                oc,
                or_,
                "helxo({}) stdout differs\nC   ={:?}\nRust={:?}",
                byte,
                String::from_utf8_lossy(&oc),
                String::from_utf8_lossy(&or_)
            );
            // sanity: the C prints the 5 keys in insertion order
            let expect: Vec<u8> = [
                b"bob h\n".to_vec(),
                b"sally e\n".to_vec(),
                b"fred l\n".to_vec(),
                {
                    let mut v = b"jen ".to_vec();
                    v.push(byte);
                    v.push(b'\n');
                    v
                },
                b"doug o\n".to_vec(),
            ]
            .concat();
            assert_eq!(oc, expect, "unexpected C output for letter {}", byte);
        }
    }
    helxo_repeated_seed_chain(c, r);
}

// row 50: the `libs()` guard is already held by the caller, so the two library
// handles are passed down (the mutex is not reentrant).
fn helxo_repeated_seed_chain(c: &Api, r: &Api) {
    let mut rng = Rng::new(0x50);
    unsafe {
        // no rand_seed in between: the global seed advances on every call, so
        // the internal table layout differs from call to call while the printed
        // output must not
        (c.rand_seed)(0);
        (r.rand_seed)(0);
        for i in 0..64 {
            let letter = (b'a' + (i % 26)) as std::ffi::c_char;
            let oc = capture_stdout("c", || (c.helxo)(letter));
            let or_ = capture_stdout("r", || (r.helxo)(letter));
            assert_eq!(oc, or_, "helxo repetition #{} diverged", i);
        }
        // and with random seeds
        for _ in 0..32 {
            let s = rng.next_u64() as usize;
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            let oc = capture_stdout("c", || (c.helxo)(b'l' as std::ffi::c_char));
            let or_ = capture_stdout("r", || (r.helxo)(b'l' as std::ffi::c_char));
            assert_eq!(oc, or_, "helxo with seed {:#x} diverged", s);
        }
    }
}
