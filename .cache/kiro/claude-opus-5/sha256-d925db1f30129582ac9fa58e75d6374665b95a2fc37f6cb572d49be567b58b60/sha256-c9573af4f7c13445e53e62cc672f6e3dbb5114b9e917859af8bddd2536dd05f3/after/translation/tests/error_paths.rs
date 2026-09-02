// Phase C — error-path differential tests, one per row of ERRORS.md.
//
// Rows E3 / E8 / E12 are undefined-behaviour rows where the C process dies.
// They are exercised for real, but in a forked child process (`crash_worker`
// re-execs this test binary), so the parent can compare the exact termination
// signal instead of guessing.

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// E1 — divide_multiplier: b == 0 -> division skipped, multiplier unchanged,
//      operation_count still incremented.
// ===========================================================================

#[test]
fn e1_divide_by_zero_is_skipped() {
    for seed_mult in [1i32, 0, -1, 7, 64, 65, 511, i32::MIN, i32::MAX, -12345] {
        let p = LibPair::fresh(&format!("e1_{seed_mult}"));
        let (c, r) = p.apis();
        if seed_mult != 1 {
            let cv = unsafe { (c.multiply_with_multiplier)(seed_mult, 1) };
            let rv = unsafe { (r.multiply_with_multiplier)(seed_mult, 1) };
            assert_eq!(cv, rv, "E1 seed multiplier={seed_mult}");
            assert_eq!(cv, seed_mult, "E1 seed produced {cv}");
        }
        // the rejection itself, several times
        for k in 0..3 {
            let a = [0i32, i32::MIN, i32::MAX][k];
            let cv = unsafe { (c.divide_multiplier)(a, 0) };
            let rv = unsafe { (r.divide_multiplier)(a, 0) };
            assert_eq!(
                cv, rv,
                "E1 divide_multiplier({a}, 0) with multiplier={seed_mult}: C={cv} Rust={rv}"
            );
            assert_eq!(
                cv, seed_mult,
                "E1 multiplier must be UNCHANGED by b==0, got {cv}"
            );
        }
        // operation_count was still bumped: prove it via findrep, which folds
        // operation_count * 010 into its result. Both must agree.
        let cv = unsafe { (c.findrep)(1, 0, 0, 0) };
        let rv = unsafe { (r.findrep)(1, 0, 0, 0) };
        assert_eq!(cv, rv, "E1 findrep after b==0 rejections (mult={seed_mult})");
    }
}

// ===========================================================================
// E2 — divide_multiplier: |b| > |multiplier|, b == INT_MIN, truncation to 0
// ===========================================================================

#[test]
fn e2_divide_truncates_toward_zero() {
    let cases: &[(i32, i32)] = &[
        (1, 2),
        (1, i32::MIN),
        (1, i32::MAX),
        (-1, 2),
        (7, 8),
        (7, -8),
        (-7, 8),
        (-7, -8),
        (100, 3),
        (-100, 3),
        (100, -3),
        (-100, -3),
        (i32::MAX, 2),
        (i32::MAX, i32::MIN),
        (i32::MIN, 2),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MIN),
        (0, 5),
        (0, -5),
    ];
    for &(seed_mult, b) in cases {
        let p = LibPair::fresh("e2");
        let (c, r) = p.apis();
        if seed_mult != 1 {
            let cv = unsafe { (c.multiply_with_multiplier)(seed_mult, 1) };
            let rv = unsafe { (r.multiply_with_multiplier)(seed_mult, 1) };
            assert_eq!(cv, rv, "E2 seed {seed_mult}");
            assert_eq!(cv, seed_mult);
        }
        let cv = unsafe { (c.divide_multiplier)(0, b) };
        let rv = unsafe { (r.divide_multiplier)(0, b) };
        assert_eq!(
            cv, rv,
            "E2 divide_multiplier(_, {b}) with multiplier={seed_mult}: C={cv} Rust={rv}"
        );
    }
}

// ===========================================================================
// E4 — find_and_replace_char: needle absent -> no write at all
// ===========================================================================

fn cmp_replace_exact(c: &Api<'_>, r: &Api<'_>, s: &[u8], needle: i32, expect_unchanged: bool) {
    let mut cb = scratch(0xAA);
    let mut rb = scratch(0xAA);
    set_cstr(&mut cb, s);
    set_cstr(&mut rb, s);
    let before = as_u8(&cb);
    unsafe { (c.find_and_replace_char)(cb.as_mut_ptr(), needle) };
    unsafe { (r.find_and_replace_char)(rb.as_mut_ptr(), needle) };
    assert_eq!(
        as_u8(&cb),
        as_u8(&rb),
        "find_and_replace_char({:?}, {needle}):\n  C   ={}\n  Rust={}",
        String::from_utf8_lossy(s),
        show(&cb),
        show(&rb)
    );
    if expect_unchanged {
        assert_eq!(
            as_u8(&cb),
            before,
            "expected NO write for needle {needle} in {:?}",
            String::from_utf8_lossy(s)
        );
    }
}

#[test]
fn e4_needle_absent_no_write() {
    let p = LibPair::fresh("e4");
    let (c, r) = p.apis();
    for s in [
        &b""[..],
        &b"a"[..],
        &b"hello"[..],
        &b"Function pointer example with static vars"[..],
        &b"Octal: 0123, Decimal: 83"[..],
        &[0x80u8, 0xFE, 0x01][..],
    ] {
        for needle in [b'Q' as i32, b'~' as i32, 0x7F, 1, 2] {
            if s.contains(&(needle as u8)) {
                continue;
            }
            cmp_replace_exact(&c, &r, s, needle, true);
        }
    }
}

// ===========================================================================
// E5 — empty string (strlen == 0 -> memchr over 0 bytes)
// ===========================================================================

#[test]
fn e5_empty_string_never_written() {
    let p = LibPair::fresh("e5");
    let (c, r) = p.apis();
    for needle in [
        0i32,
        1,
        b'a' as i32,
        b'X' as i32,
        255,
        256,
        -1,
        i32::MIN,
        i32::MAX,
    ] {
        cmp_replace_exact(&c, &r, b"", needle, true);
    }
}

// ===========================================================================
// E6 — needle == 0 (the terminator lies outside the strlen window)
// ===========================================================================

#[test]
fn e6_needle_zero_never_matches() {
    let p = LibPair::fresh("e6");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..400 {
        let len = rng.below(60) as usize;
        let s: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
        cmp_replace_exact(&c, &r, &s, 0, true);
        // also multiples of 256, whose low byte is 0 -> same rejection
        for k in [256i32, 512, 0x10000, -256, i32::MIN] {
            cmp_replace_exact(&c, &r, &s, k, true);
        }
    }
}

// ===========================================================================
// E7 — needle outside unsigned char range: C memchr truncates to (unsigned char)
// ===========================================================================

#[test]
fn e7_needle_out_of_char_range_truncates() {
    let p = LibPair::fresh("e7");
    let (c, r) = p.apis();

    // exhaustive: every low byte, presented via several different full-width
    // ints that share that low byte
    for lo in 0u32..256 {
        let s: Vec<u8> = if lo == 0 {
            b"abc".to_vec()
        } else {
            vec![b'a', lo as u8, b'z', lo as u8]
        };
        for hi in [0u32, 1, 2, 0xFF, 0x1234, 0x7FFF_FF] {
            let needle = ((hi << 8) | lo) as i32;
            cmp_replace_exact(&c, &r, &s, needle, false);
            let needle_neg = needle.wrapping_neg();
            cmp_replace_exact(&c, &r, &s, needle_neg, false);
        }
        // sign-extended form
        cmp_replace_exact(&c, &r, &s, (lo as u8 as i8) as i32, false);
        // deliberately extremal
        cmp_replace_exact(&c, &r, &s, i32::MIN | (lo as i32), false);
        cmp_replace_exact(&c, &r, &s, i32::MAX & !0xFF | (lo as i32), false);
    }

    // the specific "out-of-range enum" style values called out in the task
    let s = b"A quick brown fox";
    for needle in [
        256i32,
        321,
        0x141,
        -1,
        i32::MIN,
        i32::MAX,
        0x100,
        0xFF00,
        0x4141,
        -0xBF,
        1 << 30,
        (1u32 << 31) as i32,
    ] {
        cmp_replace_exact(&c, &r, s, needle, false);
    }
}

// ===========================================================================
// E9 / E10 / E11 — validate_and_normalize clamping and non-clamping
// ===========================================================================

#[test]
fn e9_clamped_below_lower_threshold() {
    let p = LibPair::fresh("e9");
    let (c, r) = p.apis();
    for v in 1..0o100 {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "E9 validate_and_normalize({v}): C={cv} Rust={rv}");
        assert_eq!(cv, 0o100, "E9 expected clamp to 0100 for {v}, got {cv}");
    }
}

#[test]
fn e10_clamped_above_upper_threshold() {
    let p = LibPair::fresh("e10");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x10);
    let mut vals: Vec<i32> = (0o1000..0o1100).collect();
    vals.push(0o777 + 1);
    vals.push(i32::MAX);
    vals.push(i32::MAX - 1);
    vals.push(1 << 30);
    for _ in 0..2000 {
        vals.push(0o777_i32.wrapping_add(1 + (rng.next_u32() >> 1) as i32));
    }
    for v in vals {
        if v <= 0o777 {
            continue;
        }
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "E10 validate_and_normalize({v}): C={cv} Rust={rv}");
        assert_eq!(cv, 0o777, "E10 expected clamp to 0777 for {v}, got {cv}");
    }
}

#[test]
fn e11_non_positive_is_never_clamped() {
    let p = LibPair::fresh("e11");
    let (c, r) = p.apis();
    let mut vals: Vec<i32> = vec![0, i32::MIN, i32::MIN + 1, -1];
    for v in -2000..=0 {
        vals.push(v);
    }
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..4000 {
        let v = -((rng.next_u32() >> 1) as i32);
        vals.push(v);
    }
    for v in vals {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "E11 validate_and_normalize({v}): C={cv} Rust={rv}");
        assert_eq!(cv, v, "E11 expected identity for {v}, got {cv}");
    }
    // the exact boundary trio
    for (v, want) in [(0i32, 0i32), (1, 0o100), (0o100, 0o100), (0o777, 0o777), (0o1000, 0o777)] {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "E11 boundary {v}");
        assert_eq!(cv, want, "E11 boundary {v} expected {want}, got {cv}");
    }
}

// ===========================================================================
// E13 / E14 — findrep sentinel and the all-zero dispatch
// ===========================================================================

#[test]
fn e13_findrep_sentinel_never_zero() {
    // The `if (!result_exists) result = 0777;` branch makes 0 unreachable as a
    // return value. Hammer it from many states and confirm C and Rust agree
    // and that neither ever returns 0.
    let mut rng = Rng::new(SEED ^ 0x13);
    for trial in 0..200 {
        let p = LibPair::fresh(&format!("e13_{trial}"));
        let (c, r) = p.apis();
        // randomly perturb the hidden state first
        for _ in 0..rng.below(4) {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            match rng.below(3) {
                0 => {
                    let cv = unsafe { (c.add_to_accumulator)(a, b) };
                    let rv = unsafe { (r.add_to_accumulator)(a, b) };
                    assert_eq!(cv, rv, "E13 perturb add");
                }
                1 => {
                    let cv = unsafe { (c.subtract_from_accumulator)(a, b) };
                    let rv = unsafe { (r.subtract_from_accumulator)(a, b) };
                    assert_eq!(cv, rv, "E13 perturb subtract");
                }
                _ => {
                    let cv = unsafe { (c.multiply_with_multiplier)(a, b) };
                    let rv = unsafe { (r.multiply_with_multiplier)(a, b) };
                    assert_eq!(cv, rv, "E13 perturb multiply");
                }
            }
        }
        for k in 0..4 {
            let q = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            let cv = unsafe { (c.findrep)(q.0, q.1, q.2, q.3) };
            let rv = unsafe { (r.findrep)(q.0, q.1, q.2, q.3) };
            assert_eq!(cv, rv, "E13 trial {trial} call {k} params={q:?}");
            assert_ne!(cv, 0, "E13: C returned 0 despite the sentinel");
        }
    }
}

#[test]
fn e14_all_zero_params_skips_both_dispatches() {
    let p = LibPair::fresh("e14");
    let (c, r) = p.apis();
    // fresh: accumulator 0, multiplier 1, operation_count 0.
    // active_params == 0 so neither operations[0] nor operations[1] runs.
    let cv = unsafe { (c.findrep)(0, 0, 0, 0) };
    let rv = unsafe { (r.findrep)(0, 0, 0, 0) };
    assert_eq!(cv, rv, "E14 findrep(0,0,0,0) fresh: C={cv} Rust={rv}");
    // operation_count must still be 0 -> no op ran. Confirm indirectly: the
    // next add_to_accumulator must return exactly the value it adds.
    let ca = unsafe { (c.add_to_accumulator)(5, 0) };
    let ra = unsafe { (r.add_to_accumulator)(5, 0) };
    assert_eq!(ca, ra, "E14 accumulator after all-zero findrep");
    assert_eq!(ca, 5, "E14 accumulator should still have been 0, got {}", ca - 5);

    // repeat on a non-fresh pair
    for i in 0..16 {
        let cv = unsafe { (c.findrep)(0, 0, 0, 0) };
        let rv = unsafe { (r.findrep)(0, 0, 0, 0) };
        assert_eq!(cv, rv, "E14 repeat {i}: C={cv} Rust={rv}");
    }
}

// ===========================================================================
// Generic FFI boundaries: zero / extremal / one-past-range on every export
// ===========================================================================

#[test]
fn generic_extremal_ints_on_every_export() {
    const VALS: [i32; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0o100 - 1,
        0o100 + 1,
        0o777 - 1,
        0o777 + 1,
        0o150,
    ];
    // scalar exports: full cross-product of extremal args, each on fresh state
    for &a in &VALS {
        for &b in &VALS {
            let p = LibPair::fresh("gen_scalar");
            let (c, r) = p.apis();
            let cv = unsafe { (c.add_to_accumulator)(a, b) };
            let rv = unsafe { (r.add_to_accumulator)(a, b) };
            assert_eq!(cv, rv, "add_to_accumulator({a},{b})");
            let cv = unsafe { (c.subtract_from_accumulator)(a, b) };
            let rv = unsafe { (r.subtract_from_accumulator)(a, b) };
            assert_eq!(cv, rv, "subtract_from_accumulator({a},{b})");
            let cv = unsafe { (c.multiply_with_multiplier)(a, b) };
            let rv = unsafe { (r.multiply_with_multiplier)(a, b) };
            assert_eq!(cv, rv, "multiply_with_multiplier({a},{b})");
            // skip only the single hardware-trapping pair (ERRORS.md E3)
            let mult_now = cv;
            if !(mult_now == i32::MIN && b == -1) {
                let cv = unsafe { (c.divide_multiplier)(a, b) };
                let rv = unsafe { (r.divide_multiplier)(a, b) };
                assert_eq!(cv, rv, "divide_multiplier({a},{b}) mult={mult_now}");
            }
            let cv = unsafe { (c.validate_and_normalize)(a) };
            let rv = unsafe { (r.validate_and_normalize)(a) };
            assert_eq!(cv, rv, "validate_and_normalize({a})");

            let mut cb = scratch(0xAA);
            let mut rb = scratch(0xAA);
            unsafe { (c.process_octal_string)(cb.as_mut_ptr(), a) };
            unsafe { (r.process_octal_string)(rb.as_mut_ptr(), a) };
            assert_eq!(as_u8(&cb), as_u8(&rb), "process_octal_string({a})");

            let cv = unsafe { (c.findrep)(a, b, a, b) };
            let rv = unsafe { (r.findrep)(a, b, a, b) };
            assert_eq!(cv, rv, "findrep({a},{b},{a},{b})");
        }
    }
}

#[test]
fn generic_zero_and_oversized_string_lengths() {
    let p = LibPair::fresh("gen_len");
    let (c, r) = p.apis();
    // length 0 up to the largest string that fits the 256-byte buffer
    for len in [0usize, 1, 2, 3, 49, 50, 51, 99, 100, 127, 128, 200, 254, 255] {
        let s: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        for needle in [b'a' as i32, b'z' as i32, 0, 255, -1, i32::MIN, i32::MAX] {
            let mut cb = scratch(0x5A);
            let mut rb = scratch(0x5A);
            set_cstr(&mut cb, &s);
            set_cstr(&mut rb, &s);
            unsafe { (c.find_and_replace_char)(cb.as_mut_ptr(), needle) };
            unsafe { (r.find_and_replace_char)(rb.as_mut_ptr(), needle) };
            assert_eq!(
                as_u8(&cb),
                as_u8(&rb),
                "find_and_replace_char(len={len}, {needle})\n  C   ={}\n  Rust={}",
                show(&cb),
                show(&rb)
            );
        }
    }
}

// ===========================================================================
// E3 / E8 / E12 — undefined-behaviour rows, run in an isolated child process
// so the exact termination signal can be compared.
// ===========================================================================

const CRASH_ENV: &str = "FINDREP_CRASH_MODE";

/// Worker: when `FINDREP_CRASH_MODE` is set, perform the UB action against one
/// implementation and (if it survives) print a marker. Otherwise no-op, so this
/// is a trivially passing test in the normal run.
#[test]
fn crash_worker() {
    let mode = match std::env::var(CRASH_ENV) {
        Ok(m) => m,
        Err(_) => return,
    };
    let p = LibPair::fresh("crashworker");
    let (c, r) = p.apis();
    let (which, action) = mode.split_once(':').expect("mode is `impl:action`");
    let use_rust = which == "rust";

    match action {
        // E12: process_octal_string(NULL, ...) -> strcpy to NULL
        "null_octal" => {
            let f = if use_rust {
                &r.process_octal_string
            } else {
                &c.process_octal_string
            };
            unsafe { f(std::ptr::null_mut(), 0o123) };
            println!("SURVIVED rv=void");
        }
        // E8: find_and_replace_char(NULL, ...) -> strlen(NULL)
        "null_replace" => {
            let f = if use_rust {
                &r.find_and_replace_char
            } else {
                &c.find_and_replace_char
            };
            unsafe { f(std::ptr::null_mut(), b'a' as i32) };
            println!("SURVIVED rv=void");
        }
        // E3: multiplier == INT_MIN, b == -1 -> signed division overflow
        "int_min_div_minus_one" => {
            let (mul, div) = if use_rust {
                (&r.multiply_with_multiplier, &r.divide_multiplier)
            } else {
                (&c.multiply_with_multiplier, &c.divide_multiplier)
            };
            let m = unsafe { mul(i32::MIN, 1) };
            assert_eq!(m, i32::MIN, "seeding multiplier to INT_MIN failed: {m}");
            let v = unsafe { div(0, -1) };
            println!("SURVIVED rv={v}");
        }
        other => panic!("unknown crash action {other}"),
    }
}

/// (signal, stdout) of the isolated child.
fn run_crash_child(mode: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "crash_worker", "--nocapture", "--test-threads=1"])
        .env(CRASH_ENV, mode)
        .output()
        .expect("spawn crash worker");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (out.status.signal(), out.status.code(), stdout)
}

#[test]
fn e12_null_dest_process_octal_string_same_signal() {
    let (csig, ccode, cout) = run_crash_child("c:null_octal");
    let (rsig, rcode, rout) = run_crash_child("rust:null_octal");
    eprintln!("E12 C: signal={csig:?} code={ccode:?} out={cout:?}");
    eprintln!("E12 R: signal={rsig:?} code={rcode:?} out={rout:?}");
    assert!(
        csig.is_some(),
        "E12: expected the C implementation to die on a NULL dest, got code={ccode:?}"
    );
    assert_eq!(
        csig, rsig,
        "E12: NULL dest must kill both with the SAME signal (C={csig:?}, Rust={rsig:?})"
    );
    assert!(!cout.contains("SURVIVED"), "E12: C unexpectedly survived");
    assert!(!rout.contains("SURVIVED"), "E12: Rust unexpectedly survived");
}

#[test]
fn e8_null_str_find_and_replace_same_signal() {
    let (csig, ccode, cout) = run_crash_child("c:null_replace");
    let (rsig, rcode, rout) = run_crash_child("rust:null_replace");
    eprintln!("E8 C: signal={csig:?} code={ccode:?} out={cout:?}");
    eprintln!("E8 R: signal={rsig:?} code={rcode:?} out={rout:?}");
    assert!(
        csig.is_some(),
        "E8: expected the C implementation to die on a NULL str, got code={ccode:?}"
    );
    assert_eq!(
        csig, rsig,
        "E8: NULL str must kill both with the SAME signal (C={csig:?}, Rust={rsig:?})"
    );
    assert!(!cout.contains("SURVIVED"), "E8: C unexpectedly survived");
    assert!(!rout.contains("SURVIVED"), "E8: Rust unexpectedly survived");
}

#[test]
fn e3_int_min_div_minus_one_same_signal() {
    // C: `multiplier /= b` with multiplier == INT_MIN and b == -1 is signed
    // integer overflow; on x86-64 the emitted `idiv` raises SIGFPE and the
    // process dies. The Rust translation emits the same `idiv` (see
    // `c_idiv` in src/lib.rs), so both must terminate with the SAME signal.
    let (csig, ccode, cout) = run_crash_child("c:int_min_div_minus_one");
    let (rsig, rcode, rout) = run_crash_child("rust:int_min_div_minus_one");
    eprintln!("E3 C: signal={csig:?} code={ccode:?} out={cout:?}");
    eprintln!("E3 R: signal={rsig:?} code={rcode:?} out={rout:?}");
    assert_eq!(
        csig,
        Some(8),
        "E3: expected the C to die with SIGFPE(8); got signal={csig:?} code={ccode:?} out={cout:?}"
    );
    assert_eq!(
        rsig, csig,
        "E3: Rust must trap identically to C. C signal={csig:?}, \
         Rust signal={rsig:?} code={rcode:?} out={rout:?}"
    );
    assert!(!cout.contains("SURVIVED"), "E3: C unexpectedly survived");
    assert!(!rout.contains("SURVIVED"), "E3: Rust unexpectedly survived");
}

#[test]
fn e3b_divide_still_correct_for_all_non_trapping_inputs() {
    // Guard against the `c_idiv` inline-asm helper regressing the ordinary
    // cases: exhaustively cross-product interesting dividends and divisors.
    const M: [i32; 17] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        63,
        64,
        65,
        511,
        -511,
        1000,
        -1000,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
    ];
    const B: [i32; 18] = [
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        10,
        -10,
        64,
        -64,
        511,
        -511,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    for &m in &M {
        for &b in &B {
            if m == i32::MIN && b == -1 {
                continue; // the trapping pair, covered by e3_..._same_signal
            }
            let p = LibPair::fresh("e3b");
            let (c, r) = p.apis();
            if m != 1 {
                let cv = unsafe { (c.multiply_with_multiplier)(m, 1) };
                let rv = unsafe { (r.multiply_with_multiplier)(m, 1) };
                assert_eq!(cv, rv, "E3b seed {m}");
                assert_eq!(cv, m);
            }
            let cv = unsafe { (c.divide_multiplier)(0, b) };
            let rv = unsafe { (r.divide_multiplier)(0, b) };
            assert_eq!(cv, rv, "E3b {m} / {b}: C={cv} Rust={rv}");
        }
    }
    // and a big randomized sweep on a single shared state
    let p = LibPair::fresh("e3b_rand");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x3B);
    let mut mult = 1i32;
    for i in 0..20000 {
        if i % 5 == 0 {
            let (x, y) = (rng.interesting_i32(), rng.interesting_i32());
            let cm = unsafe { (c.multiply_with_multiplier)(x, y) };
            let rm = unsafe { (r.multiply_with_multiplier)(x, y) };
            assert_eq!(cm, rm, "E3b rand step {i} multiply({x},{y})");
            mult = cm;
        }
        let mut b = rng.interesting_i32();
        if mult == i32::MIN && b == -1 {
            b = 3;
        }
        let cv = unsafe { (c.divide_multiplier)(rng.next_i32(), b) };
        let rv = unsafe { (r.divide_multiplier)(rng.next_i32(), b) };
        assert_eq!(cv, rv, "E3b rand step {i} divide(_, {b}) mult={mult}");
        mult = cv;
    }
}

#[test]
fn e3_int_min_div_minus_one_unreachable_via_findrep() {
    // Prove the trapping pair cannot be produced by `findrep`: it only ever
    // calls operations[3] as `selected_op(multiplier, 2)`, i.e. b == 2. Drive
    // findrep from a state with multiplier == INT_MIN and confirm both sides
    // agree and neither dies.
    let p = LibPair::fresh("e3_reach");
    let (c, r) = p.apis();
    let cm = unsafe { (c.multiply_with_multiplier)(i32::MIN, 1) };
    let rm = unsafe { (r.multiply_with_multiplier)(i32::MIN, 1) };
    assert_eq!(cm, rm);
    assert_eq!(cm, i32::MIN);
    let mut rng = Rng::new(SEED ^ 0x03);
    for i in 0..64 {
        let q = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let cv = unsafe { (c.findrep)(q.0, q.1, q.2, q.3) };
        let rv = unsafe { (r.findrep)(q.0, q.1, q.2, q.3) };
        assert_eq!(cv, rv, "E3 reach step {i} params={q:?}");
    }
}
