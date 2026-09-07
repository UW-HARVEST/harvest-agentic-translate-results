//! Phase B — `helxo`, the only symbol in `c_src/include/lib.h`.
//!
//! CONFIGS.md rows 51-52.  `helxo` communicates exclusively through `printf`,
//! so the comparison is of raw stdout bytes.  The project builds no driver
//! executable (`CMakeLists.txt` has only `add_library(... SHARED)` and
//! `Cargo.toml` only `crate-type = ["cdylib"]`), so this is the stdout
//! comparison the task's "binary" clause asks for.

mod common;

use common::*;
use std::ffi::c_char;

#[test]
fn row51_helxo_all_char_values() {
    let p = load_pair();
    for v in 0..=255u8 {
        let letter = v as c_char;
        unsafe {
            (p.c.rand_seed)(0x31415926);
            (p.r.rand_seed)(0x31415926);
        }
        let out_c = capture_stdout("c", || unsafe { (p.c.helxo)(letter) });
        let out_r = capture_stdout("r", || unsafe { (p.r.helxo)(letter) });
        assert_eq!(
            out_c, out_r,
            "helxo({}) stdout mismatch\n  C   ={:02x?}\n  RUST={:02x?}",
            v, out_c, out_r
        );
        // sanity: the C really did print something
        assert!(!out_c.is_empty(), "helxo({}) produced no output", v);
    }
}

#[test]
fn row52_helxo_repeated_calls_advance_the_global_seed() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 0, 1, usize::MAX, 0xabcd_ef01_2345_6789] {
        unsafe {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
        }
        // 40 successive calls: each one builds a fresh hash index, which bakes
        // in and then advances the global seed, so the whole sequence is a
        // single stateful trace.
        let out_c = capture_stdout("c_seq", || {
            for i in 0..40u8 {
                unsafe { (p.c.helxo)((b'a' + (i % 26)) as c_char) }
            }
        });
        let out_r = capture_stdout("r_seq", || {
            for i in 0..40u8 {
                unsafe { (p.r.helxo)((b'a' + (i % 26)) as c_char) }
            }
        });
        assert_eq!(
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r),
            "helxo sequence mismatch for seed {:#x}",
            seed
        );
        assert_eq!(out_c, out_r, "helxo sequence byte mismatch seed {:#x}", seed);
    }
}

#[test]
fn row51b_helxo_interleaved_with_seed_changes() {
    let p = load_pair();
    let mut rng = Rng::new(0x5151);
    let mut out_c = Vec::new();
    let mut out_r = Vec::new();
    for _ in 0..60 {
        let seed = rng.next_u64() as usize;
        let letter = rng.byte() as c_char;
        out_c.extend(capture_stdout("c_i", || unsafe {
            (p.c.rand_seed)(seed);
            (p.c.helxo)(letter);
            (p.c.helxo)(letter);
        }));
        out_r.extend(capture_stdout("r_i", || unsafe {
            (p.r.rand_seed)(seed);
            (p.r.helxo)(letter);
            (p.r.helxo)(letter);
        }));
    }
    assert_eq!(out_c, out_r, "interleaved helxo/rand_seed stdout mismatch");
}
