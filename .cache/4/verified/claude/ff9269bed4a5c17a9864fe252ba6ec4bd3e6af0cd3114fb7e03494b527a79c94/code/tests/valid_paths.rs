//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1 … C27). Every test drives the *exported*
//! `driver` symbol of the C `.so` and of the Rust `.so` through `dlopen`/`dlsym`
//! and asserts the captured `stdout` bytes are identical.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// C1 — x == 0 (every byte takes the zero-padding path)
// ---------------------------------------------------------------------------
#[test]
fn c1_all_zero_bytes() {
    diff_batch("C1 x=0", &[0]);
    // repeated, to be sure nothing is state dependent
    diff_batch("C1 x=0 x8", &[0; 8]);
}

// ---------------------------------------------------------------------------
// C2 — 0x01..=0x09 in byte 0 (padded decimal digit)
// ---------------------------------------------------------------------------
#[test]
fn c2_low_byte_decimal_digits() {
    let inputs: Vec<i32> = (0x01..=0x09).collect();
    diff_batch("C2 0x01..0x09", &inputs);
}

// ---------------------------------------------------------------------------
// C3 — 0x0a..=0x0f in byte 0 (padded letter digit)
// ---------------------------------------------------------------------------
#[test]
fn c3_low_byte_letter_digits() {
    let inputs: Vec<i32> = (0x0a..=0x0f).collect();
    diff_batch("C3 0x0a..0x0f", &inputs);
}

// ---------------------------------------------------------------------------
// C4 — 0x10..=0x7f in byte 0 (two digits, high bit clear)
// ---------------------------------------------------------------------------
#[test]
fn c4_low_byte_two_digits_positive() {
    let inputs: Vec<i32> = (0x10..=0x7f).collect();
    diff_batch("C4 0x10..0x7f", &inputs);
}

// ---------------------------------------------------------------------------
// C5 — 0x80..=0xff in byte 0 (high bit set: unsigned promotion, no sign extend)
// ---------------------------------------------------------------------------
#[test]
fn c5_low_byte_high_bit_set() {
    let inputs: Vec<i32> = (0x80..=0xff).collect();
    diff_batch("C5 0x80..0xff", &inputs);
}

// ---------------------------------------------------------------------------
// C6/C7/C8 — per-position byte sweeps (all 256 values in each byte slot)
// ---------------------------------------------------------------------------
#[test]
fn c6_byte1_sweep() {
    let inputs: Vec<i32> = (0u32..=255).map(|b| (b << 8) as i32).collect();
    diff_batch("C6 byte1 sweep", &inputs);
}

#[test]
fn c7_byte2_sweep() {
    let inputs: Vec<i32> = (0u32..=255).map(|b| (b << 16) as i32).collect();
    diff_batch("C7 byte2 sweep", &inputs);
}

#[test]
fn c8_byte3_msb_sweep() {
    let inputs: Vec<i32> = (0u32..=255).map(|b| (b << 24) as i32).collect();
    // sanity: the upper half of this sweep is negative
    assert!(inputs.iter().any(|&x| x < 0));
    diff_batch("C8 byte3 sweep", &inputs);
}

// ---------------------------------------------------------------------------
// C9 — four distinct non-zero letter-heavy bytes
// ---------------------------------------------------------------------------
#[test]
fn c9_distinct_letter_heavy_bytes() {
    let inputs: Vec<i32> = [
        0xABCDEF12u32,
        0xDEADBEEF,
        0xFEEDFACE,
        0x0F1E2D3C,
        0xCAFEBABE,
        0x12345678,
        0x9ABCDEF0,
        0xA5A5A5A5,
        0x5A5A5A5A,
    ]
    .into_iter()
    .map(|v| v as i32)
    .collect();
    diff_batch("C9 letter heavy", &inputs);
}

// ---------------------------------------------------------------------------
// C10 — NUL bytes in every position (no string-style truncation)
// ---------------------------------------------------------------------------
#[test]
fn c10_embedded_nul_bytes() {
    let inputs: Vec<i32> = [
        0x00FF00FFu32,
        0xFF0000FF,
        0x0000FF00,
        0xFF00FF00,
        0x00000001,
        0x01000000,
        0x00010000,
        0x00000100,
        0x00FFFF00,
        0xFF00_0000,
    ]
    .into_iter()
    .map(|v| v as i32)
    .collect();
    diff_batch("C10 embedded NUL", &inputs);
}

// ---------------------------------------------------------------------------
// C11 — x == -1
// ---------------------------------------------------------------------------
#[test]
fn c11_minus_one() {
    diff_batch("C11 x=-1", &[-1, -1, -1]);
}

// ---------------------------------------------------------------------------
// C12 — signed extremes
// ---------------------------------------------------------------------------
#[test]
fn c12_signed_extremes() {
    diff_batch(
        "C12 extremes",
        &[i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0, -1],
    );
}

// ---------------------------------------------------------------------------
// C13 — exhaustive 0..=0xFFFF
// ---------------------------------------------------------------------------
#[test]
fn c13_exhaustive_low_16_bits() {
    let inputs: Vec<i32> = (0i32..=0xFFFF).collect();
    diff_batch("C13 exhaustive 0..0xFFFF", &inputs);
}

// ---------------------------------------------------------------------------
// C14 — exhaustive 0xFFFF0000..=0xFFFFFFFF
// ---------------------------------------------------------------------------
#[test]
fn c14_exhaustive_negative_low_16_bits() {
    let inputs: Vec<i32> = (0u32..=0xFFFF).map(|l| (0xFFFF_0000u32 | l) as i32).collect();
    diff_batch("C14 exhaustive 0xFFFF0000..", &inputs);
}

// ---------------------------------------------------------------------------
// C15 — randomized full-domain sweep (fixed seed)
// ---------------------------------------------------------------------------
#[test]
fn c15_randomized_full_domain() {
    let mut rng = Rng::new(SEED);
    // Uniform over the whole 32-bit domain …
    let uniform: Vec<i32> = (0..10_000).map(|_| rng.next_i32()).collect();
    diff_batch("C15 uniform random", &uniform);
    // … plus shape-biased values (zeros, single bytes, halves).
    let shaped = rng.sample(10_000);
    diff_batch("C15 shaped random", &shaped);
}

// ---------------------------------------------------------------------------
// C16 — zero calls (empty stream baseline)
// ---------------------------------------------------------------------------
#[test]
fn c16_zero_calls_empty_stream() {
    let l = libs();
    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {});
        let rust_out = env.capture_file(|| {});
        (c_out, rust_out)
    });
    assert!(c_out.is_empty(), "C emitted bytes without being called");
    assert!(rust_out.is_empty(), "Rust emitted bytes without being called");
    assert_streams_match("C16 zero calls", &[], &c_out, &rust_out);
    // and prove the libraries really are loaded
    let _ = l.c_driver();
    let _ = l.rust_driver();
}

// ---------------------------------------------------------------------------
// C17 — N sequential calls in one capture (accumulation & ordering)
// ---------------------------------------------------------------------------
#[test]
fn c17_call_count_axis() {
    let mut rng = Rng::new(SEED ^ 0x17);
    for n in [1usize, 2, 3, 17, 256] {
        let inputs = rng.sample(n);
        diff_batch(&format!("C17 n={n}"), &inputs);
    }
}

// ---------------------------------------------------------------------------
// C18 — interleaved C/Rust calls into the same buffered stdout
// ---------------------------------------------------------------------------
#[test]
fn c18_interleaved_same_stream() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0x18);
    let inputs = rng.sample(400);

    let (a, b) = with_stdout(|env| {
        // [C, R, C, R, …]
        let a = env.capture_file(|| {
            for (i, &x) in inputs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { c(x) }
                } else {
                    unsafe { r(x) }
                }
            }
        });
        // [R, C, R, C, …] — must produce the identical stream
        let b = env.capture_file(|| {
            for (i, &x) in inputs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { r(x) }
                } else {
                    unsafe { c(x) }
                }
            }
        });
        (a, b)
    });
    assert_streams_match("C18 interleaved", &inputs, &a, &b);
}

// ---------------------------------------------------------------------------
// C19/C20/C21 — stdout buffering modes
// ---------------------------------------------------------------------------
fn buffering_mode_row(label: &str, mode: c_int, size: usize) {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ mode as u64);
    let inputs = rng.sample(500);

    let (rc, c_out, rust_out) = with_stdout(|env| {
        let rc = env.set_mode(mode, size);
        let c_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { c(x) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { r(x) }
            }
        });
        // restore the default fully-buffered mode
        env.set_mode(IOFBF, 4096);
        (rc, c_out, rust_out)
    });
    assert_eq!(rc, 0, "[{label}] setvbuf(mode={mode}) failed");
    assert_streams_match(label, &inputs, &c_out, &rust_out);
}

#[test]
fn c19_fully_buffered() {
    buffering_mode_row("C19 _IOFBF", IOFBF, 4096);
}

#[test]
fn c20_line_buffered() {
    buffering_mode_row("C20 _IOLBF", IOLBF, 4096);
}

#[test]
fn c21_unbuffered() {
    buffering_mode_row("C21 _IONBF", IONBF, 0);
}

// ---------------------------------------------------------------------------
// C22 — stdout is a pipe
// ---------------------------------------------------------------------------
#[test]
fn c22_stdout_is_a_pipe() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0x22);
    // keep each capture well under the 64 KiB pipe capacity
    let inputs = rng.sample(1_000);

    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_pipe(|| {
            for &x in &inputs {
                unsafe { c(x) }
            }
        });
        let rust_out = env.capture_pipe(|| {
            for &x in &inputs {
                unsafe { r(x) }
            }
        });
        (c_out, rust_out)
    });
    assert_streams_match("C22 pipe", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// C23 — called from a spawned thread
// ---------------------------------------------------------------------------
#[test]
fn c23_called_from_spawned_thread() {
    let l = libs();
    // Raw function pointers are `Copy + Send`, so they can cross the thread
    // boundary while the `Library` stays alive in the process-wide OnceLock.
    let c: DriverFn = *l.c_driver();
    let r: DriverFn = *l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0x23);
    let inputs = rng.sample(500);

    let (c_out, rust_out) = with_stdout(|env| {
        let ic = inputs.clone();
        let c_out = env.capture_file(move || {
            std::thread::spawn(move || {
                for &x in &ic {
                    unsafe { c(x) }
                }
            })
            .join()
            .unwrap();
        });
        let ir = inputs.clone();
        let rust_out = env.capture_file(move || {
            std::thread::spawn(move || {
                for &x in &ir {
                    unsafe { r(x) }
                }
            })
            .join()
            .unwrap();
        });
        (c_out, rust_out)
    });
    assert_streams_match("C23 spawned thread", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// C24 — non-"C" locale (hex conversion must stay locale independent)
// ---------------------------------------------------------------------------
#[test]
fn c24_non_c_locale() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0x24);
    let inputs = rng.sample(500);

    for name in ["en_US.UTF-8", "de_DE.UTF-8", "C.UTF-8", "POSIX"] {
        let (applied, c_out, rust_out) = with_locale(LC_ALL, name, |applied| {
            let (c_out, rust_out) = with_stdout(|env| {
                let c_out = env.capture_file(|| {
                    for &x in &inputs {
                        unsafe { c(x) }
                    }
                });
                let rust_out = env.capture_file(|| {
                    for &x in &inputs {
                        unsafe { r(x) }
                    }
                });
                (c_out, rust_out)
            });
            (applied, c_out, rust_out)
        });
        // Whether or not the locale exists on this box, both libraries must
        // agree; when it does exist the row is genuinely exercised.
        assert_streams_match(
            &format!("C24 locale={name} (applied={applied})"),
            &inputs,
            &c_out,
            &rust_out,
        );
    }
}

// ---------------------------------------------------------------------------
// C25 — dirty upper 32 register bits (SysV ABI marshalling)
// ---------------------------------------------------------------------------
#[test]
fn c25_dirty_upper_register_bits() {
    let l = libs();
    let c64 = l.c_driver64();
    let r64 = l.rust_driver64();
    let mut rng = Rng::new(SEED ^ 0x25);
    let raw: Vec<i64> = (0..500).map(|_| rng.next_u64() as i64).collect();
    let inputs: Vec<i32> = raw.iter().map(|&v| v as u64 as u32 as i32).collect();

    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { c64(v) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { r64(v) }
            }
        });
        (c_out, rust_out)
    });
    assert_streams_match("C25 dirty upper bits", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// C26 — same input repeated (no residual state)
// ---------------------------------------------------------------------------
#[test]
fn c26_repeated_identical_input() {
    let mut rng = Rng::new(SEED ^ 0x26);
    for _ in 0..8 {
        let x = rng.next_interesting_i32();
        diff_batch(&format!("C26 repeat 0x{:08x}", x as u32), &vec![x; 64]);
    }
}

// ---------------------------------------------------------------------------
// C27 — single-bit and single-nibble walks over all 32 bit positions
// ---------------------------------------------------------------------------
#[test]
fn c27_bit_and_nibble_walk() {
    let bits: Vec<i32> = (0..32).map(|k| (1u32 << k) as i32).collect();
    diff_batch("C27 single bit walk", &bits);

    let nibbles: Vec<i32> = (0..8).map(|k| (0xFu32 << (4 * k)) as i32).collect();
    diff_batch("C27 single nibble walk", &nibbles);

    let inverted_bits: Vec<i32> = (0..32).map(|k| !(1u32 << k) as i32).collect();
    diff_batch("C27 inverted bit walk", &inverted_bits);

    // cumulative fills from both ends
    let mut fills = Vec::new();
    for k in 0..=32u32 {
        let v = if k == 32 { u32::MAX } else { (1u32 << k) - 1 };
        fills.push(v as i32);
        fills.push(!v as i32);
    }
    diff_batch("C27 cumulative fills", &fills);
}
