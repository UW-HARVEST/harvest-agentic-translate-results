//! Phase B — valid-path differential tests for the convenience entry point
//! `void siphash(int init)`, rows 31..=38 of `CONFIGS.md`.
//!
//! `siphash` communicates through `stdout`, so each row redirects fd 1 to a temp
//! file and compares the emitted bytes. fd 1 is process-global and libtest's own
//! result lines also go to fd 1, so this file deliberately exposes a single
//! `#[test]` that drives the per-row functions `row31_*` .. `row38_*` in
//! sequence: with only one test in the binary, nothing else can write to fd 1
//! while a capture window is open. Per-row progress goes to stderr.

mod common;

use common::{capture_stdout, diff_hash, diff_siphash, impls, Rng};
use std::ffi::c_void;

/// Independently recompute what `siphash(init)` must print, using the C
/// implementation's own `stbds_hash_bytes` — this proves the captured text is
/// really the 64 digests of the `mem` buffer and that the capture works, not
/// just that two byte blobs happen to be equal.
fn expected_text(init: i32) -> String {
    let (c, _) = impls();
    let mut mem = [0u8; 64];
    let mut z: i32 = init;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }
    let mut s = String::new();
    for i in 0..64usize {
        let h = unsafe { (c.hash_bytes)(mem.as_mut_ptr() as *mut c_void, i, 0) };
        s.push_str("  { ");
        for j in 0..8usize {
            s.push_str(&format!("0x{:02x}, ", ((h >> (j * 8)) & 255) as u8));
        }
        s.push_str(" },\n");
    }
    s
}

/// Differential + independent-recomputation check for one `init`.
#[track_caller]
fn check(init: i32, ctx: &str) -> String {
    let got = diff_siphash(init);
    let text = String::from_utf8(got).expect("siphash output must be ASCII");
    assert_eq!(
        text.lines().count(),
        64,
        "{ctx}: siphash must print exactly 64 lines"
    );
    assert_eq!(
        text,
        expected_text(init),
        "{ctx}: captured text disagrees with an independent recomputation from \
         the C stbds_hash_bytes"
    );
    text
}

// --- CONFIGS.md row 31 ------------------------------------------------------
fn row31_siphash_init_0() {
    let text = check(0, "row31 init=0");
    // Structural checks on the emitted format.
    for l in text.lines() {
        assert!(
            l.starts_with("  { 0x") && l.ends_with(",  },"),
            "row31: unexpected line format: {l:?}"
        );
        assert_eq!(l.matches("0x").count(), 8, "row31: 8 bytes per line: {l:?}");
    }
}

// --- CONFIGS.md row 32 ------------------------------------------------------
fn row32_siphash_small_positive() {
    for init in [1i32, 2, 3, 42, 127] {
        check(init, &format!("row32 init={init}"));
    }
}

// --- CONFIGS.md row 33 ------------------------------------------------------
fn row33_siphash_signext_boundary() {
    // Drives the `mem` bytes across the 0x80 sign-extension boundary.
    for init in [0x7fi32, 0x80, 0x81, 0xc0, 0xff, 0x100, 0x1ff] {
        check(init, &format!("row33 init={init:#x}"));
    }
}

// --- CONFIGS.md row 34 ------------------------------------------------------
fn row34_siphash_negative() {
    for init in [-1i32, -2, -64, -128, -200, -255, -256, -1000, -65536] {
        check(init, &format!("row34 init={init}"));
    }
}

// --- CONFIGS.md row 35 ------------------------------------------------------
fn row35_siphash_int_min() {
    for init in [i32::MIN, i32::MIN + 1, i32::MIN + 63] {
        check(init, &format!("row35 init={init}"));
    }
}

// --- CONFIGS.md row 36 ------------------------------------------------------
fn row36_siphash_int_max_overflow() {
    // `z++` at src/lib.c:118 overflows `int` part-way through the 64 iterations.
    for init in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 32,
        i32::MAX - 63,
        i32::MAX - 64,
    ] {
        check(init, &format!("row36 init={init}"));
    }
}

// --- CONFIGS.md row 37 ------------------------------------------------------
fn row37_siphash_random_inits(rng: &mut Rng) {
    for t in 0..64 {
        let init = rng.next_i32();
        check(init, &format!("row37 t={t} init={init}"));
    }
}

// --- CONFIGS.md row 38 ------------------------------------------------------
fn row38_interleaved_no_hidden_state(rng: &mut Rng) {
    let (c, r) = impls();
    let mut buf = [0u8; 96];
    let baseline = diff_siphash(12345);
    for t in 0..24 {
        // Hammer the low-level entry point between the two siphash calls.
        rng.fill(&mut buf);
        let len = rng.range(0, 96);
        let seed = rng.next_usize();
        diff_hash(&mut buf, len, seed, &format!("row38 t={t}"));

        // Calling siphash again must give byte-identical output (no state).
        let again = diff_siphash(12345);
        assert_eq!(
            baseline, again,
            "row38: siphash(12345) output changed on repeat (t={t}) — hidden state?"
        );
    }
    // Two back-to-back calls in one capture window must produce the output
    // twice, in order, for both implementations.
    let cc = capture_stdout(|| unsafe {
        (c.siphash)(7);
        (c.siphash)(-7);
    });
    let rr = capture_stdout(|| unsafe {
        (r.siphash)(7);
        (r.siphash)(-7);
    });
    assert_eq!(
        cc, rr,
        "row38: back-to-back siphash(7); siphash(-7) output differs"
    );
    let mut concat = expected_text(7);
    concat.push_str(&expected_text(-7));
    assert_eq!(
        String::from_utf8(cc).unwrap(),
        concat,
        "row38: back-to-back output is not the concatenation of both digests"
    );
}

#[test]
fn siphash_configuration_rows_31_to_38() {
    let mut rng = Rng::new(0x51_9A_5F_00);

    eprintln!("  [CONFIGS.md row 31] row31_siphash_init_0");
    row31_siphash_init_0();
    eprintln!("  [CONFIGS.md row 32] row32_siphash_small_positive");
    row32_siphash_small_positive();
    eprintln!("  [CONFIGS.md row 33] row33_siphash_signext_boundary");
    row33_siphash_signext_boundary();
    eprintln!("  [CONFIGS.md row 34] row34_siphash_negative");
    row34_siphash_negative();
    eprintln!("  [CONFIGS.md row 35] row35_siphash_int_min");
    row35_siphash_int_min();
    eprintln!("  [CONFIGS.md row 36] row36_siphash_int_max_overflow");
    row36_siphash_int_max_overflow();
    eprintln!("  [CONFIGS.md row 37] row37_siphash_random_inits");
    row37_siphash_random_inits(&mut rng);
    eprintln!("  [CONFIGS.md row 38] row38_interleaved_no_hidden_state");
    row38_interleaved_no_hidden_state(&mut rng);

    eprintln!("  all siphash rows 31..=38 PASSED");
}
