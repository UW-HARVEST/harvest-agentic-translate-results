//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH `libdriver.so` files
//! (C and Rust) with `libloading` and compares the bytes they emit through the
//! FFI boundary. Randomized rows use a fixed PRNG seed, so failures reproduce.

mod harness;

use harness::*;
use std::ffi::c_int;

const SEED: u64 = 0x0bad_c0ff_ee12_3456;

// ---------------------------------------------------------------- C1..C4
// Fixed boundary values.

#[test]
fn c1_zero_all_bytes_00() {
    assert_same(0, "C1");
    assert_eq!(c_out(0), b"00000000\n".to_vec(), "C1 ground truth");
}

#[test]
fn c2_one_smallest_positive() {
    assert_same(1, "C2");
}

#[test]
fn c3_minus_one_all_bytes_ff() {
    assert_same(-1, "C3");
    assert_eq!(c_out(-1), b"ffffffff\n".to_vec(), "C3 ground truth");
}

#[test]
fn c4_whole_value_boundaries() {
    for x in [
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        2,
        0,
        -1,
        1,
    ] {
        assert_same(x, "C4");
    }
    assert_same_batch(
        &[c_int::MAX, c_int::MIN, -2, 2, c_int::MIN + 1, c_int::MAX - 1],
        "C4",
    );
}

// ---------------------------------------------------------------- C5..C8
// Randomized value ranges (property-style, `cases()` inputs each).

#[test]
fn c5_random_small_non_negative() {
    let mut rng = Rng::new(SEED ^ 5);
    let xs: Vec<c_int> = (0..cases())
        .map(|_| (rng.next_u32() & 0xff) as c_int)
        .collect();
    for &x in &xs {
        assert_same(x, "C5");
    }
    assert_same_batch(&xs, "C5");
}

#[test]
fn c6_random_two_byte_values() {
    let mut rng = Rng::new(SEED ^ 6);
    let xs: Vec<c_int> = (0..cases())
        .map(|_| (0x100 + (rng.next_u32() % (0x10000 - 0x100))) as c_int)
        .collect();
    for &x in &xs {
        assert_same(x, "C6");
    }
    assert_same_batch(&xs, "C6");
}

#[test]
fn c7_random_full_32_bit_range() {
    let mut rng = Rng::new(SEED ^ 7);
    let xs: Vec<c_int> = (0..cases()).map(|_| rng.next_i32()).collect();
    for &x in &xs {
        assert_same(x, "C7");
    }
    assert_same_batch(&xs, "C7");
}

/// Extension of row C7: a very wide sweep compared with batch captures only, so
/// hundreds of thousands of distinct inputs cost just a handful of process
/// round-trips. Covers (a) a strided walk across the whole 2^32 input space and
/// (b) a large uniformly random sample.
#[test]
fn c7b_wide_sweep_of_the_whole_input_space() {
    // Odd stride coprime with 2^32 => the walk never repeats a value.
    const STRIDE: u32 = 0x9e37_79b9;
    let n = (cases() * 64).clamp(4096, 262_144);

    let mut v: u32 = 0;
    let strided: Vec<c_int> = (0..n)
        .map(|_| {
            let x = v as c_int;
            v = v.wrapping_add(STRIDE);
            x
        })
        .collect();
    assert_same_batch(&strided, "C7b/strided");

    let mut rng = Rng::new(SEED ^ 0x7b);
    let random: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
    assert_same_batch(&random, "C7b/random");
}

#[test]
fn c8_random_negative_values() {
    let mut rng = Rng::new(SEED ^ 8);
    let xs: Vec<c_int> = (0..cases())
        .map(|_| (rng.next_u32() | 0x8000_0000) as c_int)
        .collect();
    for &x in &xs {
        assert!(x < 0, "C8 generator must produce negatives");
        assert_same(x, "C8");
    }
    assert_same_batch(&xs, "C8");
}

// ---------------------------------------------------------------- C9..C12
// Exhaustive per-byte value classes, one byte position at a time.

fn exhaustive_single_byte(position: usize, row: &str) {
    let xs: Vec<c_int> = (0u32..=0xff)
        .map(|v| c_int::from_ne_bytes({
            let mut b = [0u8; 4];
            b[position] = v as u8;
            b
        }))
        .collect();
    for &x in &xs {
        assert_same(x, row);
    }
    assert_same_batch(&xs, row);
}

#[test]
fn c9_exhaustive_byte_position_0() {
    exhaustive_single_byte(0, "C9");
}

#[test]
fn c10_exhaustive_byte_position_1() {
    exhaustive_single_byte(1, "C10");
}

#[test]
fn c11_exhaustive_byte_position_2() {
    exhaustive_single_byte(2, "C11");
}

#[test]
fn c12_exhaustive_byte_position_3() {
    exhaustive_single_byte(3, "C12");
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_exhaustive_all_four_bytes_equal() {
    let xs: Vec<c_int> = (0u32..=0xff)
        .map(|v| c_int::from_ne_bytes([v as u8; 4]))
        .collect();
    for &x in &xs {
        assert_same(x, "C13");
    }
    assert_same_batch(&xs, "C13");
}

// ---------------------------------------------------------------- C14
// Byte-order discrimination: four distinct non-zero bytes.
#[test]
fn c14_distinct_non_zero_bytes_byte_order() {
    // Hand-picked canary first: a wrong byte order shows up immediately.
    let canary = c_int::from_ne_bytes([0x01, 0x02, 0x03, 0x04]);
    assert_same(canary, "C14");
    assert_eq!(
        c_out(canary),
        b"01020304\n".to_vec(),
        "C14: bytes must be printed in memory order, lowest address first"
    );

    let mut rng = Rng::new(SEED ^ 14);
    let mut xs = Vec::new();
    while xs.len() < cases() {
        let mut b = [0u8; 4];
        for slot in b.iter_mut() {
            *slot = 1 + (rng.below(255) as u8); // 0x01..=0xff
        }
        if b[0] != b[1] && b[0] != b[2] && b[0] != b[3] && b[1] != b[2] && b[1] != b[3] && b[2] != b[3]
        {
            xs.push(c_int::from_ne_bytes(b));
        }
    }
    for &x in &xs {
        assert_same(x, "C14");
    }
    assert_same_batch(&xs, "C14");
}

// ---------------------------------------------------------------- C15, C16
#[test]
fn c15_powers_of_two() {
    let xs: Vec<c_int> = (0..32).map(|k| (1u32 << k) as c_int).collect();
    for &x in &xs {
        assert_same(x, "C15");
    }
    assert_same_batch(&xs, "C15");
}

#[test]
fn c16_complement_of_powers_of_two() {
    let xs: Vec<c_int> = (0..32).map(|k| !(1u32 << k) as c_int).collect();
    for &x in &xs {
        assert_same(x, "C16");
    }
    assert_same_batch(&xs, "C16");
}

// ---------------------------------------------------------------- C17
// Repeated calls: 0, 1, 2 and many calls inside a single capture.
#[test]
fn c17_repeated_calls_in_one_capture() {
    let cf = c_driver();
    let rf = rust_driver();

    // Zero calls: both must emit nothing at all.
    let c_empty = capture_file(|| {});
    assert!(c_empty.is_empty(), "C17: empty capture must be empty");

    for n in [1usize, 2, 3, 17] {
        let mut rng = Rng::new(SEED ^ (0x1700 + n as u64));
        let xs: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        assert_same_batch(&xs, "C17");

        let c = capture_file(|| {
            for &x in &xs {
                unsafe { cf(x) }
            }
        });
        assert_eq!(c.len(), 9 * n, "C17: {n} calls must emit exactly {} bytes", 9 * n);
        let r = capture_file(|| {
            for &x in &xs {
                unsafe { rf(x) }
            }
        });
        assert_eq!(c, r, "C17: divergence for a sequence of {n} calls");
    }

    // Many calls, and interleaved C/Rust calls in one capture: the two
    // implementations must be indistinguishable in the shared stream.
    let mut rng = Rng::new(SEED ^ 0x17ff);
    let xs: Vec<c_int> = (0..cases()).map(|_| rng.next_i32()).collect();
    assert_same_batch(&xs, "C17");

    let interleaved_c = capture_file(|| {
        for &x in &xs {
            unsafe { cf(x) };
            unsafe { cf(x) };
        }
    });
    let interleaved_mixed = capture_file(|| {
        for &x in &xs {
            unsafe { cf(x) };
            unsafe { rf(x) };
        }
    });
    assert_eq!(
        interleaved_c, interleaved_mixed,
        "C17: interleaving Rust calls with C calls changed the byte stream"
    );
}

// ---------------------------------------------------------------- C18
// Low-level ABI shape: drive the exported symbol through u32/u64 signatures.
#[test]
fn c18_abi_argument_width() {
    let cu32 = c_driver_u32();
    let ru32 = rust_driver_u32();
    let mut rng = Rng::new(SEED ^ 18);

    for _ in 0..cases() {
        let v = rng.next_u32();
        let c = capture_file(|| unsafe { cu32(v) });
        let r = capture_file(|| unsafe { ru32(v) });
        assert_eq!(
            c, r,
            "C18: driver called as fn(u32) with {v:#010x} diverged"
        );
        assert_eq!(c, expected(v as c_int), "C18: unexpected C output for {v:#010x}");
    }

    // Same value delivered as `int` and as `u32` must behave identically.
    let cf = c_driver();
    for v in [0u32, 1, 0x8000_0000, 0xffff_ffff, 0x7fff_ffff] {
        let via_i32 = capture_file(|| unsafe { cf(v as c_int) });
        let via_u32 = capture_file(|| unsafe { cu32(v) });
        assert_eq!(via_i32, via_u32, "C18: C differs by declared arg type");
        let r = capture_file(|| unsafe { ru32(v) });
        assert_eq!(via_i32, r, "C18: Rust differs from C for {v:#010x}");
    }
}

// ---------------------------------------------------------------- C19
// Output-stream shape: fully buffered regular file vs. pipe.
#[test]
fn c19_stdout_buffering_modes() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = Rng::new(SEED ^ 19);
    // Keep well under the 64 KiB pipe capacity: 512 calls = 4608 bytes.
    let n = cases().min(512);
    let xs: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();

    let c_file = capture_file(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let r_file = capture_file(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });
    let c_pipe = capture_pipe(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let r_pipe = capture_pipe(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });

    assert_eq!(c_file, r_file, "C19: divergence with stdout on a regular file");
    assert_eq!(c_pipe, r_pipe, "C19: divergence with stdout on a pipe");
    assert_eq!(
        c_file, c_pipe,
        "C19: C output differs between buffering modes (test harness sanity)"
    );
    assert_eq!(
        r_file, r_pipe,
        "C19: Rust output differs between buffering modes"
    );
    assert_eq!(c_file.len(), 9 * n, "C19: unexpected total length");
}

// ---------------------------------------------------------------- C20
// print_hex invariants: exactly 4 iterations, lowercase hex, trailing newline.
#[test]
fn c20_output_shape_invariants() {
    let mut rng = Rng::new(SEED ^ 20);
    let mut xs: Vec<c_int> = vec![0, 1, -1, c_int::MAX, c_int::MIN];
    xs.extend((0..cases()).map(|_| rng.next_i32()));

    for &x in &xs {
        let c = c_out(x);
        let r = rust_out(x);
        assert_shape(&c, x, "C20/C");
        assert_shape(&r, x, "C20/Rust");
        assert_eq!(c, r, "C20: divergence for {x:#010x}");
        // The loop bound `i < len` never degenerates: 4 bytes were printed.
        assert_eq!(c[..8].len(), 8, "C20: must print sizeof(int)==4 bytes");
    }
}

// ---------------------------------------------------------------- C21
// Coverage argument for the full 2^32 input space.
//
// `print_hex` formats each of the four bytes independently, so the behaviour of
// `driver` is completely determined by
//   (a) the 4 x 256 "digit pair emitted for byte value v at position p" cells, and
//   (b) the property that byte positions do not influence each other.
// Rows C9..C12 already compare all 1024 cells of (a) between C and Rust
// exhaustively. This row establishes (b) for BOTH implementations: every output
// is exactly the concatenation of the per-byte cells. Together they extend the
// differential result from the sampled inputs to all 2^32 of them.
#[test]
fn c21_per_byte_independence_extends_coverage_to_all_inputs() {
    // Build the exhaustive per-position digit table from each library.
    let mut c_table = [[[0u8; 2]; 256]; 4];
    let mut r_table = [[[0u8; 2]; 256]; 4];
    for position in 0..4usize {
        let xs: Vec<c_int> = (0u32..=0xff)
            .map(|v| {
                let mut b = [0u8; 4];
                b[position] = v as u8;
                c_int::from_ne_bytes(b)
            })
            .collect();
        // One capture per position instead of 256: cheap and exhaustive.
        let cf = c_driver();
        let rf = rust_driver();
        let c = capture_file(|| {
            for &x in &xs {
                unsafe { cf(x) }
            }
        });
        let r = capture_file(|| {
            for &x in &xs {
                unsafe { rf(x) }
            }
        });
        assert_eq!(c, r, "C21: per-byte table differs at position {position}");
        assert_eq!(c.len(), 256 * 9, "C21: unexpected capture length");
        for v in 0..256usize {
            let rec = &c[v * 9..v * 9 + 9];
            let digits = &rec[position * 2..position * 2 + 2];
            c_table[position][v] = [digits[0], digits[1]];
            let rec_r = &r[v * 9..v * 9 + 9];
            r_table[position][v] = [rec_r[position * 2], rec_r[position * 2 + 1]];
        }
    }
    assert_eq!(
        c_table, r_table,
        "C21: the exhaustive 4x256 per-byte digit tables of C and Rust differ"
    );

    // Now check independence: arbitrary byte combinations must equal the
    // concatenation of the per-byte cells, for both libraries.
    let compose = |table: &[[[u8; 2]; 256]; 4], x: c_int| -> Vec<u8> {
        let mut out = Vec::with_capacity(9);
        for (position, b) in x.to_ne_bytes().into_iter().enumerate() {
            out.extend_from_slice(&table[position][b as usize]);
        }
        out.push(b'\n');
        out
    };

    let mut rng = Rng::new(SEED ^ 21);
    let mut xs: Vec<c_int> = vec![0, 1, -1, c_int::MAX, c_int::MIN];
    xs.extend((0..(cases() * 16).max(4096)).map(|_| rng.next_i32()));

    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_file(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let r = capture_file(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });
    assert_eq!(c, r, "C21: divergence over the independence sample");

    let mut want = Vec::with_capacity(xs.len() * 9);
    for &x in &xs {
        want.extend_from_slice(&compose(&c_table, x));
    }
    assert_eq!(
        c, want,
        "C21: the C output is NOT the concatenation of its own per-byte table -- \
         byte positions interact, so the exhaustive coverage argument does not hold"
    );
}

// ---------------------------------------------------------------- C22
// Exhaustive pairwise (2-wise) byte-value coverage: for every one of the 6 pairs
// of byte positions, every one of the 2^16 value combinations, with the two
// remaining bytes held at 0x00 and at 0xff. 786_432 inputs per implementation,
// compared with batch captures.
#[test]
fn c22_exhaustive_pairwise_byte_value_coverage() {
    const PAIRS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let cf = c_driver();
    let rf = rust_driver();

    for filler in [0x00u8, 0xff] {
        for (p, q) in PAIRS {
            // 256 chunks of 256 inputs keeps each capture small.
            for hi in 0u32..=0xff {
                let xs: Vec<c_int> = (0u32..=0xff)
                    .map(|lo| {
                        let mut b = [filler; 4];
                        b[p] = hi as u8;
                        b[q] = lo as u8;
                        c_int::from_ne_bytes(b)
                    })
                    .collect();
                let c = capture_file(|| {
                    for &x in &xs {
                        unsafe { cf(x) }
                    }
                });
                let r = capture_file(|| {
                    for &x in &xs {
                        unsafe { rf(x) }
                    }
                });
                if c != r {
                    let idx = (c
                        .iter()
                        .zip(r.iter())
                        .position(|(a, b)| a != b)
                        .unwrap_or(0)
                        / 9)
                        .min(xs.len() - 1);
                    let rec = |v: &[u8]| show(&v[idx * 9..(idx * 9 + 9).min(v.len())]);
                    panic!(
                        "C22: divergence for byte pair ({p},{q}), filler {filler:#04x}, \
                         input {:#010x}:\n  C    = \"{}\"\n  Rust = \"{}\"",
                        xs[idx],
                        rec(&c),
                        rec(&r)
                    );
                }
                let mut want = Vec::with_capacity(xs.len() * 9);
                for &x in &xs {
                    want.extend_from_slice(&expected(x));
                }
                assert_eq!(
                    c, want,
                    "C22: unexpected C ground truth for byte pair ({p},{q}), filler {filler:#04x}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------- sanity
// Guard against the two `.so` handles resolving to the same code.
#[test]
fn sanity_two_distinct_implementations_are_loaded() {
    let c = c_driver();
    let r = rust_driver();
    let c_addr = *c as usize;
    let r_addr = *r as usize;
    assert_ne!(
        c_addr, r_addr,
        "dlsym returned the same address for both libraries -- the C and Rust \
         implementations are not being tested against each other"
    );
    let i = impls();
    assert!(
        i.c_path.ends_with("c_src/build/libdriver.so"),
        "unexpected C library path {}",
        i.c_path.display()
    );
    assert!(
        i.rust_path.to_string_lossy().contains("target"),
        "unexpected Rust library path {}",
        i.rust_path.display()
    );
    println!("C   .so: {}", i.c_path.display());
    println!("Rust.so: {}", i.rust_path.display());
}
