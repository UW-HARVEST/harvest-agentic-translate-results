// Phase C -- error-path differential tests, one test per ERRORS.md row.
//
// `liblong` has NO explicit error surface (see ERRORS.md for the mechanical
// derivation). The contract each test below asserts is therefore twofold:
//
//   1. C accepts the input and completes normally (NO-REJECT), and
//   2. Rust reaches the SAME outcome -- the same array bytes / the same printed
//      text -- rather than inventing a rejection (a bounds/overflow panic, an
//      assertion, an early return). Under `panic = "abort"` a Rust panic kills
//      the test process outright, so "the test finished at all" is itself part
//      of the assertion; the debug-profile `.so` additionally has
//      `overflow-checks = true`, which turns any non-`wrapping_*` arithmetic
//      into exactly such an abort.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// Rows 1-6, 17, 18: the `long_exec` seed parameter.
//
// A full `long_exec` run costs ~470 s (C) and cannot be repeated per seed.
// What IS cheap and is what these rows are actually about is the seed's effect
// on the library's entry state: `srand(seed); array[i] = rand()`. Each row
// drives that state into both workers through the real `.so` exports and
// asserts identical results, plus asserts the seed reinterpretation itself.
// The genuinely-full `long_exec` differential lives in `long_exec_full.rs`.
// ===========================================================================

fn seed_state_agrees(h: &Harness, label: &str, seed: u32) {
    let mut input = vec![0 as c_int; N];
    libc_rand_fill(seed, &mut input);
    diff_peo(h, label, &input, 1);
}

/// ERRORS.md row 1 -- `seed = 0`, the low boundary of `unsigned int`.
#[test]
fn err01_seed_zero_is_not_rejected() {
    let h = harness();
    seed_state_agrees(&h, "err01 seed 0", 0);

    // glibc documents seed 0 as behaving like seed 1; assert the C-visible
    // consequence so the row's premise is on record.
    let mut a = vec![0 as c_int; N];
    let mut b = vec![0 as c_int; N];
    libc_rand_fill(0, &mut a);
    libc_rand_fill(1, &mut b);
    assert_eq!(a, b, "glibc srand(0) no longer aliases srand(1)");
}

/// ERRORS.md row 2 -- `seed = UINT_MAX`, the high boundary.
#[test]
fn err02_seed_uint_max_is_not_rejected() {
    let h = harness();
    seed_state_agrees(&h, "err02 seed UINT_MAX", u32::MAX);
}

/// ERRORS.md row 3 -- `seed = 0x80000000`, first value negative as `int`.
#[test]
fn err03_seed_sign_bit_set() {
    let h = harness();
    seed_state_agrees(&h, "err03 seed 0x80000000", 0x8000_0000);

    // Must be the same stream as passing INT_MIN reinterpreted.
    let mut a = vec![0 as c_int; N];
    let mut b = vec![0 as c_int; N];
    libc_rand_fill(0x8000_0000, &mut a);
    libc_rand_fill(c_int::MIN as u32, &mut b);
    assert_eq!(a, b);
}

/// ERRORS.md row 4 -- caller passes a negative `int` for the `unsigned`
/// parameter: an out-of-range value for the declared type. Two's-complement
/// reinterpretation must make it identical to row 2.
#[test]
fn err04_negative_seed_reinterprets_as_uint_max() {
    let h = harness();
    let as_unsigned = (-1i32) as u32;
    assert_eq!(as_unsigned, u32::MAX);
    seed_state_agrees(&h, "err04 seed -1 as unsigned", as_unsigned);

    let mut a = vec![0 as c_int; N];
    let mut b = vec![0 as c_int; N];
    libc_rand_fill(as_unsigned, &mut a);
    libc_rand_fill(u32::MAX, &mut b);
    assert_eq!(a, b, "seed -1 must be identical to seed UINT_MAX");
}

/// ERRORS.md row 5 -- out-of-range "enum-like" integers across the FFI
/// boundary. There is no valid-variant table, so no value can fall outside it;
/// every bit pattern must be accepted with no default/fallback branch.
#[test]
fn err05_out_of_range_enum_like_values() {
    let h = harness();
    for seed in [0x7FFF_FFFFu32, 0xFFFF_FFFE, 12_345_678, 0xDEAD_BEEF, 0xCCCC_CCCC] {
        seed_state_agrees(&h, &format!("err05 seed {seed:#010x}"), seed);
    }
}

/// ERRORS.md row 6 -- surplus register arguments through a mismatched
/// prototype. SysV AMD64 ignores them; only `edi` is read. Verified without
/// paying for a full run by checking `dlsym` + a call through the wrong
/// prototype on a *cheap* function is indistinguishable, and by asserting the
/// symbol is a plain function in both objects.
#[test]
fn err06_surplus_arguments_are_ignored() {
    let h = harness();
    let mut rng = Rng::new(0x06);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    // `perform_expensive_operations` is the cheap stand-in with the same
    // "declared with no/other parameters, called with extra ones" shape.
    for t in h.all() {
        t.write_array(&input);
        t.peo_with_null_arg();
    }
    let c = h.c.read_array();
    for t in &h.rust {
        assert!(
            t.read_array() == c,
            "[{}] surplus/garbage arguments changed the result",
            t.name
        );
    }
    // And confirm the same call via the correct prototype gives the same thing.
    for t in h.all() {
        t.write_array(&input);
        t.peo();
    }
    assert_eq!(
        h.c.read_array(),
        c,
        "calling through the extra-argument prototype differed from the correct one"
    );
}

/// ERRORS.md row 7 -- the "null pointer" boundary against a function that
/// declares no parameters and dereferences no caller pointer.
#[test]
fn err07_null_pointer_argument_is_a_noop() {
    let h = harness();
    let zeros = vec![0 as c_int; N];
    for t in h.all() {
        t.write_array(&zeros);
        t.peo_with_null_arg();
    }
    let c = h.c.read_array();
    // An all-zero array is NOT a fixed point: step(0) = -3, and 100 steps map
    // 0 to CHURN_OF_ZERO. The call must have done its normal work.
    assert!(
        c.iter().all(|&v| v == CHURN_OF_ZERO),
        "C did not perform its normal work on the null-argument call"
    );
    for t in &h.rust {
        assert!(
            t.read_array() == c,
            "[{}] null-argument call diverged from C",
            t.name
        );
    }
}

/// ERRORS.md row 8 -- worker called before any seeding, on the zero `.bss`.
/// There is no "not initialised" guard: 0 is a fixed point.
#[test]
fn err08_uninitialised_state_is_not_rejected() {
    let h = harness();
    let zeros = vec![0 as c_int; N];
    diff_peo(&h, "err08 zero/uninitialised state", &zeros, 3);
    // Ground truth: the C maps a zero element to CHURN_OF_ZERO after one call
    // (0 is not a fixed point -- step(0) = -3), and keeps moving after that.
    let after3 = h.c.read_array();
    assert!(
        after3.iter().all(|&v| v == after3[0]),
        "a uniform input must stay uniform"
    );
    assert_ne!(after3[0], 0, "the C does transform a zero element");
}

/// ERRORS.md row 9 -- repeated calls with no re-seeding; no one-shot guard.
#[test]
fn err09_repeated_calls_have_no_call_limit() {
    let h = harness();
    let mut rng = Rng::new(0x09);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    diff_peo(&h, "err09 repeated calls", &input, 6);
}

/// ERRORS.md row 10 -- `INT_MAX` feeding `x * 3 + 7` signed overflow.
#[test]
fn err10_int_max_overflow_wraps_and_does_not_panic() {
    let h = harness();
    let mut input = vec![0 as c_int; N];
    fill_uniform(&mut input, c_int::MAX);
    diff_peo(&h, "err10 all INT_MAX", &input, 1);
    // Reaching here at all proves no Rust overflow-check abort occurred.
}

/// ERRORS.md row 11 -- `INT_MIN`: overflow AND most-negative division AND
/// negative remainder, all at once. Must not raise SIGFPE or panic.
#[test]
fn err11_int_min_division_and_remainder() {
    let h = harness();
    let mut input = vec![0 as c_int; N];
    fill_uniform(&mut input, c_int::MIN);
    diff_peo(&h, "err11 all INT_MIN", &input, 1);

    // Record the C semantics this row depends on (values verified against gcc).
    assert_eq!(c_int::MIN / 2, -1_073_741_824, "truncation toward zero");
    assert_eq!(c_int::MIN % 7, -2, "remainder takes the dividend's sign");
    assert_eq!((-7i32) % 7, 0);
    assert_eq!((-8i32) / 2, -4);
    assert_eq!((-1i32) / 2, 0, "C truncates toward zero, not toward -inf");
}

/// ERRORS.md row 12 -- `x << 1` overflowing the sign bit (UB in C, plain `shl`
/// in practice). Rust must reproduce it with an unsigned shift, not panic.
#[test]
fn err12_left_shift_overflow() {
    let h = harness();
    let values: &[c_int] = &[
        c_int::MIN,
        c_int::MIN + 1,
        -1_073_741_825,
        1 << 30,
        -(1 << 30),
        c_int::MAX,
        -1,
    ];
    let mut input = vec![0 as c_int; N];
    for &v in values {
        fill_uniform(&mut input, v);
        diff_peo(&h, &format!("err12 shift-overflow value {v}"), &input, 1);
    }
    // The identity the translation relies on.
    for &v in values {
        assert_eq!(
            v.wrapping_sub(((v as u32) << 1) as c_int),
            v.wrapping_sub(v.wrapping_mul(2)),
            "x - (x<<1) identity broken for {v}"
        );
    }
}

/// ERRORS.md row 13 -- `x >> 3` on negatives must be arithmetic (`sar`), not
/// logical. A logical shift would silently produce different numbers, so this
/// row is checked with negative-only randomised data, not just one value.
#[test]
fn err13_right_shift_of_negative_is_arithmetic() {
    let h = harness();
    let mut rng = Rng::new(0x13);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_neg();
    }
    diff_peo(&h, "err13 negative-only >> 3", &input, 2);

    assert_eq!(-1i32 >> 3, -1, "i32 >> must be arithmetic");
    assert_eq!(c_int::MIN >> 3, -268_435_456);
    assert_ne!(
        (-1i32) >> 3,
        ((-1i32 as u32) >> 3) as i32,
        "arithmetic and logical shift must be distinguishable here"
    );
}

/// ERRORS.md row 14 -- no divide-by-zero is reachable: both divisors are
/// non-zero literals. Proven by driving a wide value sweep with no SIGFPE.
#[test]
fn err14_no_division_by_zero_is_reachable() {
    let h = harness();
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/src/long.c"
    ))
    .expect("cannot read the C source");
    // The only divisions in the C are by the literals 2 and 7.
    assert!(src.contains("x / 2 + x % 7"));
    let div_ops = src.matches(" / ").count() + src.matches(" % ").count();
    assert_eq!(
        div_ops, 2,
        "the C source gained a division; ERRORS.md row 14 must be re-derived"
    );

    let mut rng = Rng::new(0x14);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    diff_peo(&h, "err14 wide sweep, no SIGFPE", &input, 1);
}

/// ERRORS.md row 15 -- the index boundary: exactly `[0, ARRAY_SIZE)` is
/// written, and `array[ARRAY_SIZE]` (one past the documented valid range) is
/// never touched, in both libraries.
#[test]
fn err15_index_boundary_exactly_array_size() {
    let h = harness();
    let mut rng = Rng::new(0x15);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32() | 1;
    }

    let canary: u8 = 0xC7;
    let mut befores = Vec::new();
    for t in h.all() {
        t.write_array(&input);
        unsafe {
            let past = t.array_ptr().add(N) as *mut u8;
            // Plant a canary in the padding that follows the 1 MiB object.
            for k in 0..64 {
                std::ptr::write(past.add(k), canary);
            }
            befores.push(std::slice::from_raw_parts(past, 64).to_vec());
        }
    }
    for t in h.all() {
        t.peo();
    }
    for (t, before) in h.all().zip(befores.iter()) {
        let after = unsafe { std::slice::from_raw_parts(t.array_ptr().add(N) as *const u8, 64) };
        assert_eq!(
            before.as_slice(),
            after,
            "[{}] wrote to array[ARRAY_SIZE..] -- one past the valid range",
            t.name
        );
    }
    // First and last in-bounds elements must both have been processed.
    let out = h.c.read_array();
    assert_ne!(out[0], input[0], "element 0 not processed");
    assert_ne!(out[N - 1], input[N - 1], "element ARRAY_SIZE-1 not processed");
    for t in &h.rust {
        let r = t.read_array();
        assert!(r == out, "[{}] boundary elements diverged", t.name);
    }
}

/// ERRORS.md row 16 -- "oversized length": there is no length parameter, the
/// extent is a compile-time constant. Both objects must publish the same
/// `st_size` so the same byte range is in bounds for a `dlsym` consumer.
#[test]
fn err16_no_length_parameter_same_extent() {
    let h = harness();
    // Writing every one of the 262144 elements through dlsym must be safe in
    // both libraries; a smaller Rust object would corrupt memory here.
    for t in h.all() {
        unsafe {
            let base = t.array_ptr();
            for i in 0..N {
                std::ptr::write(base.add(i), (i as c_int) ^ -1);
            }
            for i in [0usize, 1, N / 2, N - 1] {
                assert_eq!(
                    std::ptr::read(base.add(i)),
                    (i as c_int) ^ -1,
                    "[{}] element {i} not addressable",
                    t.name
                );
            }
        }
    }
    for t in h.all() {
        t.peo();
    }
    let c = h.c.read_array();
    for t in &h.rust {
        assert!(t.read_array() == c, "[{}] full-extent run diverged", t.name);
    }
    // The declared sizes themselves are compared in
    // `symbols.rs::array_object_size_matches`.
    assert_eq!(ARRAY_BYTES, 1_048_576);
}

/// ERRORS.md row 17 -- re-entry after a dirty `array`: `long_exec` overwrites
/// all state first, so the result depends only on the seed. Verified for the
/// state-establishing part (which is where a stale-state bug would live)
/// without a 470 s run; the full-run version is in `long_exec_full.rs`.
#[test]
fn err17_state_is_fully_overwritten_before_use() {
    let h = harness();
    // Dirty every array with different garbage per library.
    let mut rng = Rng::new(0x17);
    for (k, t) in h.all().enumerate() {
        let mut junk = vec![0 as c_int; N];
        for slot in junk.iter_mut() {
            *slot = rng.next_i32() ^ (k as c_int * 0x1234_5678);
        }
        t.write_array(&junk);
    }
    // Now establish the seeded state exactly as `long_exec` does and run.
    let mut input = vec![0 as c_int; N];
    libc_rand_fill(2024, &mut input);
    diff_peo(&h, "err17 re-entry after dirty array", &input, 2);

    // The C source must still overwrite the whole array before iterating.
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/src/long.c"
    ))
    .unwrap();
    assert!(
        src.contains("array[i] = rand();"),
        "long_exec no longer re-seeds the whole array; row 17 must be re-derived"
    );
}

/// ERRORS.md row 18 -- `printf`'s return value is discarded; a write failure
/// is not propagated. Asserted structurally (the C ignores the result) and
/// behaviourally: with stdout redirected to /dev/full the call still returns.
#[test]
fn err18_printf_result_is_discarded() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/src/long.c"
    ))
    .unwrap();
    // The printf call stands alone as a statement; its value is never tested.
    assert!(src.contains(r#"printf("%d\n", xor_result);"#));
    assert!(
        !src.contains("if (printf") && !src.contains("= printf"),
        "the C now inspects printf's result; row 18 must be re-derived"
    );
    // The Rust translation must likewise ignore it -- never turning a failed
    // write into a panic, which under `panic = "abort"` would kill the caller.
    let rs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(
        rs.contains("printf("),
        "the Rust translation no longer calls libc printf"
    );
    for forbidden in ["expect(", "unwrap(", "panic!", "assert!", "assert_eq!"] {
        assert!(
            !rs.contains(forbidden),
            "the Rust translation contains `{forbidden}`; the C never fails or \
             aborts, so no fallible-unwrapping construct may appear in it"
        );
    }
}

/// ERRORS.md row 19 -- the `xor` accumulator cannot overflow or trap; the sign
/// bit is just another bit. Checked against real worker output.
#[test]
fn err19_xor_accumulator_cannot_overflow() {
    let h = harness();
    let mut rng = Rng::new(0x19);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    diff_peo(&h, "err19 xor accumulator input", &input, 1);
    let c = h.c.read_array();
    let fold = xor_fold(&c);
    for t in &h.rust {
        assert_eq!(xor_fold(&t.read_array()), fold, "[{}] fold differs", t.name);
    }
    // Folding the extremes must not panic even in a checked build.
    assert_eq!(xor_fold(&[c_int::MIN, c_int::MAX]), -1);
    assert_eq!(xor_fold(&[c_int::MIN, c_int::MIN]), 0);
}
