// Phase C -- error / rejection-path differential tests.
//
// One test per row of ERRORS.md. The C library has no error-return channel at
// all (`void sieve(int)`, zero `return`s, zero asserts, zero range checks), so
// "same error result" means: same stdout byte stream AND same process
// termination status for the exact triggering input.

mod common;

use common::*;

// --- row 1 -----------------------------------------------------------------
// There is no error channel: no return value, no sentinel, no errno use.
// Both sides must therefore *accept* every input and exit normally.
#[test]
fn err_01_no_error_return_channel_exists() {
    // The header signature has no way to report failure.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/sieve.h"),
    )
    .unwrap();
    assert!(
        hdr.contains("void sieve(int start);"),
        "C public API changed; ERRORS.md must be re-derived"
    );

    // And empirically: a batch of assorted inputs is accepted by both, with
    // identical output and a clean exit (the child asserts exit status 0).
    let vals: Vec<i64> = vec![9, 0, -1, -9, 123, 2_147_483_639, -1000];
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 2 -----------------------------------------------------------------
// Every negative input: `val % 10` is in -9..=0 and can never equal 9, so the
// documented "stops when it ends in 9" contract is violated -- the loop runs
// up to +9 instead. Randomized to cover all negative remainder classes.
#[test]
fn err_02_negative_never_matches_mod9() {
    let mut rng = Pcg32::new(0xE770_0002);
    let vals: Vec<i64> = (0..150).map(|_| rng.range(-2_000, -1)).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));

    // Each negative start emits exactly (10 - val) lines: val..=9.
    let total_lines: i64 = vals.iter().map(|v| 10 - v).sum();
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count() as i64,
        total_lines
    );

    // Spot-check the property that defines this row: the last line of a
    // single negative run is "9", never the negative value ending in 9.
    let one = assert_same(&[-19]);
    assert!(one.ends_with(b"\n9\n"), "negative run must end at +9");
    assert!(
        one.starts_with(b"-19\n-18\n"),
        "must not break at -19 even though it 'ends in 9'"
    );
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn err_03_negative_nine_does_not_terminate_early() {
    let out = assert_same(&[-9]);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 19);
    assert_eq!(out, expected(&[-9]));
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn err_04_negative_multiple_of_ten() {
    let vals: Vec<i64> = vec![-10, -30, -70, -100, -2_000];
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 5 -----------------------------------------------------------------
// The signed-overflow region: val in [INT_MAX-7, INT_MAX]. No representable
// value >= val ends in 9, so `val++` overflows. At -O0 (the project sets no
// -O flag) the emitted `addl $1, -0x4(%rbp)` wraps to INT_MIN, after which
// the loop climbs ~2^31 more values. Compared as a bounded prefix.
#[test]
fn err_05_int_max_overflow_wraps() {
    const N: usize = 256 * 1024;
    for v in 2_147_483_640i64..=2_147_483_647 {
        let out = assert_same_prefix(v, N);
        // The pre-overflow values appear first, then the wrap to INT_MIN.
        let head = String::from_utf8_lossy(&out[..64.min(out.len())]).to_string();
        assert!(
            head.starts_with(&format!("{v}\n")),
            "prefix for sieve({v}) starts with {head:?}"
        );
        assert!(
            String::from_utf8_lossy(&out[..4096]).contains("2147483647\n-2147483648\n"),
            "wrap point INT_MAX -> INT_MIN not observed for sieve({v})"
        );
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn err_06_int_max_exact_prefix() {
    const N: usize = 1024 * 1024;
    let out = assert_same_prefix(i32::MAX as i64, N);
    assert!(out.starts_with(b"2147483647\n-2147483648\n-2147483647\n"));
    // Cross-check against the model for the first few thousand lines.
    let model = {
        let mut m = Vec::new();
        let mut val: i32 = i32::MAX;
        while m.len() < 4096 {
            m.extend_from_slice(format!("{val}\n").as_bytes());
            if val % 10 == 9 {
                break;
            }
            val = val.wrapping_add(1);
        }
        m
    };
    assert_eq!(&out[..4096], &model[..4096]);
}

// --- row 7 -----------------------------------------------------------------
// INT_MIN: extreme of the negative case, and the only %d value whose negation
// is unrepresentable. ~2^31 lines -> bounded prefix comparison.
#[test]
fn err_07_int_min_prefix() {
    const N: usize = 1024 * 1024;
    let out = assert_same_prefix(i32::MIN as i64, N);
    assert!(out.starts_with(b"-2147483648\n-2147483647\n-2147483646\n"));
}

// --- row 8 -----------------------------------------------------------------
// "Out-of-range enum" analogue: the API takes a raw `int`, so all 2^32 bit
// patterns are in range and none may be rejected. Bounded patterns are run to
// completion; the two patterns that imply ~10^9 iterations are compared as
// prefixes.
#[test]
fn err_08_arbitrary_bit_patterns() {
    let bounded: [u32; 10] = [
        0x0000_0000,
        0x0000_0009,
        0x0000_000A,
        0x7FFF_FFF7, //  2147483639
        0x7FFF_FFF6, //  2147483638
        0xFFFF_FFFF, //           -1
        0xFFFF_FFF7, //           -9
        0xFFFF_FFF6, //          -10
        0xFFFF_FC17, //        -1001
        0x5555_5555, //   1431655765 (terminates after 5 lines)
    ];
    let vals: Vec<i64> = bounded.iter().map(|&p| p as i32 as i64).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));

    // Hostile patterns whose loops are ~10^9 long: prefix-compare instead.
    for pat in [0x8000_0000u32, 0xAAAA_AAAA, 0xDEAD_BEEF] {
        let v = pat as i32 as i64;
        let pfx = assert_same_prefix(v, 64 * 1024);
        assert!(
            pfx.starts_with(format!("{v}\n").as_bytes()),
            "pattern {pat:#010x} ({v}) prefix mismatch with its own first line"
        );
    }
}

// --- row 9 -----------------------------------------------------------------
// Generic C-API boundaries. There is no pointer or length parameter to pass
// NULL / 0 / oversized for, so the scalar boundaries stand in for them.
#[test]
fn err_09_generic_scalar_boundaries() {
    assert_eq!(assert_same(&[0]).iter().filter(|&&b| b == b'\n').count(), 10);
    assert_eq!(assert_same(&[1]).iter().filter(|&&b| b == b'\n').count(), 9);
    assert_eq!(
        assert_same(&[-1]).iter().filter(|&&b| b == b'\n').count(),
        11
    );
    // one step past each end of the "documented" single-digit range
    let vals: Vec<i64> = vec![-1, 0, 1, 8, 9, 10, 11];
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 10 ----------------------------------------------------------------
// Calling convention: the callee reads only %edi (`mov %edi,-0x4(%rbp)`), so
// garbage in the upper 32 bits of %rdi must be ignored identically by both.
#[test]
fn err_10_upper_register_bits_ignored() {
    let high: [i64; 6] = [
        0x1234_5678_0000_0009,
        0x7FFF_FFFF_0000_0000,
        -1i64 & !0xFFFF_FFFF, // upper bits set, low 32 == 0
        0xDEAD_BEEF_FFFF_FFF7u64 as i64,
        0x0000_0001_0000_0005,
        i64::MIN | 0x0000_0000_0000_0007,
    ];
    let out_wide = assert_same_wide(&high);

    // And the observable result must equal what the truncated low 32 bits give
    // through the normal prototype.
    let truncated: Vec<i64> = high.iter().map(|&v| v as i32 as i64).collect();
    let out_narrow = assert_same(&truncated);
    assert_eq!(
        out_wide, out_narrow,
        "upper argument-register bits leaked into the result"
    );
    assert_eq!(out_wide, expected(&truncated));
}

// --- row 11 ----------------------------------------------------------------
// No static/global state exists, so N calls in one process must equal the
// concatenation of N single-call processes.
#[test]
fn err_11_no_hidden_state_between_calls() {
    let vals: Vec<i64> = vec![9, 9, 0, -3, 7, -9, 42, 0];
    let batched = assert_same(&vals);
    let mut concatenated = Vec::new();
    for &v in &vals {
        concatenated.extend_from_slice(&assert_same(&[v]));
    }
    assert_eq!(
        batched, concatenated,
        "output depends on call history -- hidden state"
    );
}

// --- row 12 ----------------------------------------------------------------
// `printf`'s return value is never checked by the C code, so a write error
// (fd 1 closed -> EBADF on every call) is silently swallowed: the loop still
// runs to completion and the function returns normally. The Rust translation
// must also ignore the result rather than panicking/aborting.
#[test]
fn err_12_ignores_printf_failure_on_closed_stdout() {
    let vals: Vec<i64> = vec![3, -5, 9, 0];
    let c = run_with_closed_stdout(&c_lib(), &vals);
    let r = run_with_closed_stdout(&rust_lib(), &vals);
    assert_eq!(c, Some(0), "C did not survive a closed stdout");
    assert_eq!(
        c, r,
        "divergent termination with closed stdout: C={c:?} rust={r:?}"
    );
}
