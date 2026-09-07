//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact rejecting condition and asserts C and Rust
//! produce the *same* value / sentinel / fatal signal, not merely that both
//! misbehaved.

mod common;

use common::*;
use std::ffi::{c_char, CString};

const NAME: &str = "PROG_BASE_OFFSET";

/// Call `parse_env_numeric` in both libraries with the environment already set
/// up, returning both results and both stderr byte streams.
fn parse_both(name_ptr: *const c_char, default_val: i32) -> (i32, i32, Vec<u8>, Vec<u8>) {
    let b = both();
    let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(name_ptr, default_val) });
    let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(name_ptr, default_val) });
    (cv, rv, ccap.stderr, rcap.stderr)
}

#[track_caller]
fn assert_parse(ctx: &str, value: Option<&str>, default_val: i32, expect_default: Option<bool>) {
    let _g = env_guard();
    clear_prog_env();
    match value {
        Some(v) => set_env(NAME, v),
        None => unset_env(NAME),
    }
    let cname = CString::new(NAME).unwrap();
    let (cv, rv, cerr, rerr) = parse_both(cname.as_ptr(), default_val);
    clear_prog_env();

    assert_eq!(
        cv, rv,
        "DIVERGENCE [{ctx}] parse_env_numeric({NAME}={value:?}, default={default_val}): \
         C = {cv}, Rust = {rv}"
    );
    assert_eq!(
        cerr,
        rerr,
        "DIVERGENCE [{ctx}] stderr for {NAME}={value:?}:\n  C    = {}\n  Rust = {}",
        show(&cerr),
        show(&rerr)
    );
    if let Some(true) = expect_default {
        assert_eq!(
            cv, default_val,
            "[{ctx}] expected the C to fall back to default_val for {value:?}, got {cv}"
        );
    }
    if let Some(false) = expect_default {
        // Only meaningful when default_val is distinguishable.
        if default_val != cv {
            // fine: it really did not use the default
        }
    }
}

// ---------------------------------------------------------------------------
// Row 1 — variable absent
// ---------------------------------------------------------------------------
#[test]
fn err_01_missing_env_returns_default() {
    let mut r = Rng::with_seed(SEED ^ 0xC01);
    for _ in 0..300 {
        assert_parse("err01", None, r.next_i32(), Some(true));
    }
    for d in EXTREMES {
        assert_parse("err01/extremes", None, d, Some(true));
    }
    // And no stderr output at all in this path.
    let _g = env_guard();
    clear_prog_env();
    let cname = CString::new(NAME).unwrap();
    let (_, _, cerr, rerr) = parse_both(cname.as_ptr(), 1234);
    assert!(cerr.is_empty(), "C wrote to stderr on the absent-variable path: {}", show(&cerr));
    assert!(rerr.is_empty(), "Rust wrote to stderr on the absent-variable path: {}", show(&rerr));
}

// ---------------------------------------------------------------------------
// Row 2 — comma rejection (+ exact warning text)
// ---------------------------------------------------------------------------
#[test]
fn err_02_comma_returns_default() {
    let mut r = Rng::with_seed(SEED ^ 0xC02);
    for s in ["1,2", ",", "a,b", "9,", ",9", "1,2,3", "-1,-2"] {
        for _ in 0..20 {
            assert_parse("err02", Some(s), r.next_i32(), Some(true));
        }
    }
    // The warning text itself must match byte-for-byte.
    let _g = env_guard();
    clear_prog_env();
    set_env(NAME, "1,2");
    let cname = CString::new(NAME).unwrap();
    let (cv, rv, cerr, rerr) = parse_both(cname.as_ptr(), 77);
    clear_prog_env();
    assert_eq!((cv, rv), (77, 77));
    assert_eq!(cerr, rerr, "warning bytes differ:\n C={}\n R={}", show(&cerr), show(&rerr));
    assert_eq!(
        cerr,
        format!("Warning: Invalid character in {NAME}\n").into_bytes(),
        "unexpected C warning text: {}",
        show(&cerr)
    );
}

// ---------------------------------------------------------------------------
// Row 3 — semicolon rejection (+ exact warning text)
// ---------------------------------------------------------------------------
#[test]
fn err_03_semicolon_returns_default() {
    let mut r = Rng::with_seed(SEED ^ 0xC03);
    for s in ["1;2", ";", "a;b", "9;", ";9", "1;2;3", "-1;-2"] {
        for _ in 0..20 {
            assert_parse("err03", Some(s), r.next_i32(), Some(true));
        }
    }
    let _g = env_guard();
    clear_prog_env();
    set_env(NAME, "1;2");
    let cname = CString::new(NAME).unwrap();
    let (cv, rv, cerr, rerr) = parse_both(cname.as_ptr(), -5);
    clear_prog_env();
    assert_eq!((cv, rv), (-5, -5));
    assert_eq!(cerr, rerr, "warning bytes differ:\n C={}\n R={}", show(&cerr), show(&rerr));
    assert_eq!(
        cerr,
        format!("Warning: Semicolon found in {NAME}\n").into_bytes(),
        "unexpected C warning text: {}",
        show(&cerr)
    );
}

// ---------------------------------------------------------------------------
// Row 4 — both separators: the comma branch is checked first and wins
// ---------------------------------------------------------------------------
#[test]
fn err_04_comma_and_semicolon_comma_wins() {
    let _g = env_guard();
    for s in ["1,2;3", "1;2,3", ",;", ";,", ",,;;", ";;,,"] {
        clear_prog_env();
        set_env(NAME, s);
        let cname = CString::new(NAME).unwrap();
        let (cv, rv, cerr, rerr) = parse_both(cname.as_ptr(), 999);
        clear_prog_env();
        assert_eq!(cv, rv, "value differs for {s:?}: C={cv} Rust={rv}");
        assert_eq!(cv, 999, "expected default for {s:?}");
        assert_eq!(
            cerr,
            rerr,
            "warning bytes differ for {s:?}:\n C={}\n R={}",
            show(&cerr),
            show(&rerr)
        );
        assert_eq!(
            cerr,
            format!("Warning: Invalid character in {NAME}\n").into_bytes(),
            "the comma branch must win for {s:?}, C said: {}",
            show(&cerr)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 — empty value is NOT a rejection: atoi("") == 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_empty_value_is_zero_not_default() {
    let _g = env_guard();
    for d in [0i32, 1, -1, 64, i32::MIN, i32::MAX, 12345] {
        clear_prog_env();
        set_env(NAME, "");
        let cname = CString::new(NAME).unwrap();
        let (cv, rv, cerr, rerr) = parse_both(cname.as_ptr(), d);
        clear_prog_env();
        assert_eq!(cv, rv, "C={cv} Rust={rv} for empty value, default={d}");
        assert_eq!(cv, 0, "C did not return atoi(\"\") == 0 for default={d}");
        assert!(cerr.is_empty() && rerr.is_empty(), "unexpected warning on the empty-value path");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — non-numeric garbage reaches atoi
// ---------------------------------------------------------------------------
#[test]
fn err_06_non_numeric_atoi_result() {
    let mut r = Rng::with_seed(SEED ^ 0xC06);
    let junk: [&str; 16] = [
        "abc", "+", "-", "0x10", " ", "\t", "\n", " \t ", "++1", "--1", "+-1", ".5", "e10", "%",
        "_", "NaN",
    ];
    for s in junk {
        for _ in 0..10 {
            assert_parse("err06", Some(s), r.next_i32(), None);
        }
        // Pin the value the C actually produces so a "helpful" Rust rewrite is
        // caught rather than silently accepted.
        let _g = env_guard();
        clear_prog_env();
        set_env(NAME, s);
        let cname = CString::new(NAME).unwrap();
        let (cv, rv, _, _) = parse_both(cname.as_ptr(), 0x7EAD_BEEF_u32 as i32);
        clear_prog_env();
        assert_eq!(cv, rv, "C={cv} Rust={rv} for {s:?}");
        assert_ne!(cv, 0x7EAD_BEEF_u32 as i32, "{s:?} must not fall back to default_val");
    }
}

// ---------------------------------------------------------------------------
// Row 7 — atoi overflow
// ---------------------------------------------------------------------------
#[test]
fn err_07_atoi_overflow() {
    let mut r = Rng::with_seed(SEED ^ 0xC07);
    let big: [&str; 12] = [
        "2147483648",
        "2147483649",
        "-2147483649",
        "-2147483650",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775809",
        "99999999999999",
        "-99999999999999",
        "123456789012345678901234567890",
        "-123456789012345678901234567890",
    ];
    for s in big {
        for _ in 0..10 {
            assert_parse("err07", Some(s), r.next_i32(), None);
        }
    }
    for _ in 0..500 {
        let mut s = String::new();
        if r.below(2) == 0 {
            s.push('-');
        }
        for _ in 0..(11 + r.below(15) as usize) {
            s.push((b'0' + (r.below(10) as u8)) as char);
        }
        assert_parse("err07/rand", Some(&s), r.next_i32(), None);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — default_val boundary values
// ---------------------------------------------------------------------------
#[test]
fn err_08_default_val_boundaries() {
    for d in EXTREMES {
        assert_parse("err08", None, d, Some(true));
        // Same boundary default through a rejecting value.
        assert_parse("err08/comma", Some("1,2"), d, Some(true));
        assert_parse("err08/semi", Some("1;2"), d, Some(true));
    }
}

// ---------------------------------------------------------------------------
// Row 9 — envy's negative-result recovery
// ---------------------------------------------------------------------------
#[test]
fn err_09_negative_result_recovery() {
    let b = both();
    let _g = env_guard();
    clear_prog_env();
    // base_offset forced very negative so the recovery path is guaranteed.
    set_env("PROG_BASE_OFFSET", "-2000000000");
    let mut hits = 0usize;
    let cases: Vec<(i32, i32, i32, i32)> = (-50..50)
        .flat_map(|p1| (-3..3).map(move |p2| (p1, p2, 0, 0)))
        .collect();
    let (bad, _cap) = capture(|| {
        let mut bad = None;
        for &(p1, p2, p3, p4) in &cases {
            let cv = unsafe { (b.c.envy)(p1, p2, p3, p4) };
            let rv = unsafe { (b.rs.envy)(p1, p2, p3, p4) };
            if cv != rv {
                bad = Some((p1, p2, p3, p4, cv, rv));
                break;
            }
            if cv == p1 {
                // recovery returns state.base_value == param1
            }
        }
        bad
    });
    if let Some((p1, p2, p3, p4, cv, rv)) = bad {
        clear_prog_env();
        panic!("DIVERGENCE [err09] envy({p1}, {p2}, {p3}, {p4}): C = {cv}, Rust = {rv}");
    }
    // Every case here must have taken the recovery path: base_offset -2e9
    // dominates, so the result is negative before the check and the returned
    // value is exactly param1.
    let (results, _cap) = capture(|| {
        cases
            .iter()
            .map(|&(p1, p2, p3, p4)| unsafe { (b.c.envy)(p1, p2, p3, p4) })
            .collect::<Vec<i32>>()
    });
    for (i, &v) in results.iter().enumerate() {
        if v == cases[i].0 {
            hits += 1;
        }
    }
    clear_prog_env();
    assert!(
        hits >= cases.len(),
        "err09 expected every case to recover ({hits}/{})",
        cases.len()
    );
}

// ---------------------------------------------------------------------------
// Rows 10, 11 — the param3 / param4 terms are SKIPPED, not added as zero
// ---------------------------------------------------------------------------
#[test]
fn err_10_param3_zero_skips_term() {
    let b = both();
    let _g = env_guard();
    clear_prog_env();
    // multiplier = 0 makes "add 0" and "skip" numerically identical; the
    // distinguishing case is that with a huge multiplier, param3 == 0 must
    // still produce exactly the skipped-term result.
    for mult in ["0", "1", "-1", "2147483647", "-2147483648", "10"] {
        set_env("PROG_MULTIPLIER", mult);
        let (bad, _c) = capture(|| {
            let mut bad = None;
            for p1 in -20..20 {
                for p2 in -5..5 {
                    for p4 in [0, 1, -1, 8, -8] {
                        let cv = unsafe { (b.c.envy)(p1, p2, 0, p4) };
                        let rv = unsafe { (b.rs.envy)(p1, p2, 0, p4) };
                        if cv != rv {
                            bad = Some((p1, p2, p4, cv, rv));
                        }
                    }
                }
            }
            bad
        });
        if let Some((p1, p2, p4, cv, rv)) = bad {
            clear_prog_env();
            panic!("DIVERGENCE [err10] PROG_MULTIPLIER={mult} envy({p1}, {p2}, 0, {p4}): C={cv} Rust={rv}");
        }
    }
    clear_prog_env();
}

#[test]
fn err_11_param4_zero_skips_term() {
    let b = both();
    let _g = env_guard();
    clear_prog_env();
    let (bad, _c) = capture(|| {
        let mut bad = None;
        for p1 in -20..20 {
            for p2 in -5..5 {
                for p3 in [0, 1, -1, 7, -7] {
                    let cv = unsafe { (b.c.envy)(p1, p2, p3, 0) };
                    let rv = unsafe { (b.rs.envy)(p1, p2, p3, 0) };
                    if cv != rv {
                        bad = Some((p1, p2, p3, cv, rv));
                    }
                }
            }
        }
        bad
    });
    if let Some((p1, p2, p3, cv, rv)) = bad {
        panic!("DIVERGENCE [err11] envy({p1}, {p2}, {p3}, 0): C={cv} Rust={rv}");
    }
    // param4 in 1..=3 has `param4 >> 2 == 0`, so it must be indistinguishable
    // from the skipped case in BOTH libraries — a check that the guard is on
    // `param4 != 0` and not on the shifted value.
    for p4 in [1, 2, 3] {
        let (pair, _c) = capture(|| {
            let c0 = unsafe { (b.c.envy)(7, 3, 1, 0) };
            let c1 = unsafe { (b.c.envy)(7, 3, 1, p4) };
            let r0 = unsafe { (b.rs.envy)(7, 3, 1, 0) };
            let r1 = unsafe { (b.rs.envy)(7, 3, 1, p4) };
            (c0, c1, r0, r1)
        });
        let (c0, c1, r0, r1) = pair;
        assert_eq!((c0, c1), (r0, r1), "err11 divergence for param4={p4}");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — envy falls back to the octal defaults when the env is rejected
// ---------------------------------------------------------------------------
#[test]
fn err_12_envy_uses_defaults_on_rejected_env() {
    let b = both();
    let _g = env_guard();
    let params: Vec<(i32, i32, i32, i32)> = {
        let mut r = Rng::with_seed(SEED ^ 0xC12);
        (0..1000)
            .map(|_| (r.next_i32_mixed(), r.next_i32_mixed(), r.next_i32_mixed(), r.next_i32_mixed()))
            .collect()
    };

    let run_all = |cfg: &[(&str, Option<&str>)]| -> (Vec<i32>, Vec<u8>, Vec<i32>, Vec<u8>) {
        apply_env(cfg);
        let (cres, ccap) = capture(|| {
            params
                .iter()
                .map(|&(a, b2, c2, d)| unsafe { (b.c.envy)(a, b2, c2, d) })
                .collect::<Vec<i32>>()
        });
        let (rres, rcap) = capture(|| {
            params
                .iter()
                .map(|&(a, b2, c2, d)| unsafe { (b.rs.envy)(a, b2, c2, d) })
                .collect::<Vec<i32>>()
        });
        (cres, ccap.stderr, rres, rcap.stderr)
    };

    // Baseline: nothing set -> base_offset 0100 = 64, multiplier 012 = 10.
    let (base_c, _, base_r, _) = run_all(&[]);
    assert_eq!(base_c, base_r, "err12 baseline diverged");

    // Rejected values must reproduce the baseline exactly, in both libraries.
    for bad in ["1,2", "9;9", ",", ";"] {
        let (c1, cerr, r1, rerr) = run_all(&[
            ("PROG_BASE_OFFSET", Some(bad)),
            ("PROG_MULTIPLIER", Some(bad)),
        ]);
        assert_eq!(c1, r1, "err12 diverged for rejected value {bad:?}");
        assert_eq!(cerr, rerr, "err12 stderr diverged for {bad:?}:\n C={}\n R={}", show(&cerr), show(&rerr));
        assert_eq!(
            c1, base_c,
            "err12: rejected {bad:?} must fall back to the octal defaults 64 / 10"
        );
        assert!(!cerr.is_empty(), "err12: expected a warning on stderr for {bad:?}");
    }

    // Explicitly setting the octal defaults' decimal equivalents must match too.
    let (c2, _, r2, _) = run_all(&[
        ("PROG_BASE_OFFSET", Some("64")),
        ("PROG_MULTIPLIER", Some("10")),
    ]);
    assert_eq!(c2, r2, "err12 diverged for explicit 64/10");
    assert_eq!(c2, base_c, "err12: 64/10 must equal the octal defaults 0100/012");
    clear_prog_env();
}

// ---------------------------------------------------------------------------
// Rows 13, 14 — variable present but without '1'
// ---------------------------------------------------------------------------
#[test]
fn err_13_verbose_present_without_1() {
    let seeds: [FlagsWord; 4] = [0, 0xFFFF_FFFF, 0xDEAD_BEEF, 0x0000_0055];
    let b = both();
    let _g = env_guard();
    for v in ["0", "", "yes", "true", "on", "2", "\t", "one"] {
        clear_prog_env();
        set_env("PROG_VERBOSE", v);
        for &seed in &seeds {
            let mut fc = seed;
            let mut fr = seed;
            let (_, cc) = capture(|| unsafe { (b.c.init_config_from_env)(&mut fc) });
            let (_, rc) = capture(|| unsafe { (b.rs.init_config_from_env)(&mut fr) });
            assert_eq!(fc, fr, "err13 diverged for PROG_VERBOSE={v:?}: C=0x{fc:08x} Rust=0x{fr:08x}");
            assert_eq!(fc & BIT_VERBOSE, 0, "err13: verbose must be 0 for {v:?} (C word 0x{fc:08x})");
            assert_eq!(cc.stdout, rc.stdout);
            assert_eq!(cc.stderr, rc.stderr);
        }
    }
    clear_prog_env();
}

#[test]
fn err_14_debug_present_without_1() {
    let seeds: [FlagsWord; 4] = [0, 0xFFFF_FFFF, 0xDEAD_BEEF, 0x0000_00AA];
    let b = both();
    let _g = env_guard();
    for v in ["0", "", "yes", "true", "on", "2", "\t", "one"] {
        clear_prog_env();
        set_env("PROG_DEBUG", v);
        for &seed in &seeds {
            let mut fc = seed;
            let mut fr = seed;
            let (_, cc) = capture(|| unsafe { (b.c.init_config_from_env)(&mut fc) });
            let (_, rc) = capture(|| unsafe { (b.rs.init_config_from_env)(&mut fr) });
            assert_eq!(fc, fr, "err14 diverged for PROG_DEBUG={v:?}: C=0x{fc:08x} Rust=0x{fr:08x}");
            assert_eq!(fc & BIT_DEBUG, 0, "err14: debug must be 0 for {v:?} (C word 0x{fc:08x})");
            assert_eq!(cc.stdout, rc.stdout);
            assert_eq!(cc.stderr, rc.stderr);
        }
    }
    clear_prog_env();
}

// ---------------------------------------------------------------------------
// Rows 15-18 — null pointers. The C dereferences with no check, so the
// behaviour is a fatal signal; the Rust must be *identically* fatal.
// ---------------------------------------------------------------------------

#[track_caller]
fn assert_same_death(ctx: &str, c: impl FnOnce(), rs: impl FnOnce()) {
    let cd = run_in_child(c);
    let rd = run_in_child(rs);
    assert_eq!(
        cd, rd,
        "DIVERGENCE [{ctx}]: C died as {cd:?}, Rust died as {rd:?}"
    );
    // And it must actually be a crash, not a silent success in both.
    match cd {
        Death::Signaled(s) => assert_eq!(s, 11, "[{ctx}] expected SIGSEGV (11), got signal {s}"),
        Death::Exited(code) => panic!("[{ctx}] expected a fatal signal, both exited with {code}"),
    }
}

#[test]
fn err_15_null_flags_init() {
    let b = both();
    assert_same_death(
        "err15",
        || unsafe { (b.c.init_config_from_env)(std::ptr::null_mut()) },
        || unsafe { (b.rs.init_config_from_env)(std::ptr::null_mut()) },
    );
}

#[test]
fn err_16_null_flags_perform() {
    let b = both();
    assert_same_death(
        "err16",
        || {
            let v = unsafe { (b.c.perform_operation)(1, 2, std::ptr::null_mut()) };
            std::hint::black_box(v);
        },
        || {
            let v = unsafe { (b.rs.perform_operation)(1, 2, std::ptr::null_mut()) };
            std::hint::black_box(v);
        },
    );
}

#[test]
fn err_17_null_flags_apply() {
    let b = both();
    assert_same_death(
        "err17",
        || {
            let v = unsafe { (b.c.apply_bit_operations)(1, std::ptr::null_mut()) };
            std::hint::black_box(v);
        },
        || {
            let v = unsafe { (b.rs.apply_bit_operations)(1, std::ptr::null_mut()) };
            std::hint::black_box(v);
        },
    );
}

#[test]
fn err_18_null_env_name() {
    let b = both();
    assert_same_death(
        "err18",
        || {
            let v = unsafe { (b.c.parse_env_numeric)(std::ptr::null(), 7) };
            std::hint::black_box(v);
        },
        || {
            let v = unsafe { (b.rs.parse_env_numeric)(std::ptr::null(), 7) };
            std::hint::black_box(v);
        },
    );
}

// ---------------------------------------------------------------------------
// Rows 19, 20 — degenerate env_name values
// ---------------------------------------------------------------------------
#[test]
fn err_19_empty_env_name() {
    let b = both();
    let _g = env_guard();
    clear_prog_env();
    let empty = CString::new("").unwrap();
    for d in EXTREMES {
        let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(empty.as_ptr(), d) });
        let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(empty.as_ptr(), d) });
        assert_eq!(cv, rv, "err19 diverged for default={d}: C={cv} Rust={rv}");
        assert_eq!(cv, d, "err19: getenv(\"\") must be NULL so the default is returned");
        assert_eq!(ccap.stderr, rcap.stderr);
    }
}

#[test]
fn err_20_oversized_env_name() {
    let b = both();
    let _g = env_guard();
    clear_prog_env();
    for len in [1usize, 255, 256, 1024, 4096, 65536] {
        let name = CString::new("X".repeat(len)).unwrap();
        let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(name.as_ptr(), -321) });
        let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(name.as_ptr(), -321) });
        assert_eq!(cv, rv, "err20 diverged for name length {len}: C={cv} Rust={rv}");
        assert_eq!(cv, -321, "err20: absent oversized name must yield the default");
        assert_eq!(ccap.stderr, rcap.stderr);
    }
    // Oversized name that DOES exist, with a rejecting value: exercises the
    // `%s` warning path with a long name.
    let long = "Y".repeat(4096);
    set_env(&long, "1,2");
    let name = CString::new(long.clone()).unwrap();
    let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(name.as_ptr(), 42) });
    let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(name.as_ptr(), 42) });
    unset_env(&long);
    assert_eq!(cv, rv);
    assert_eq!(cv, 42);
    assert_eq!(
        ccap.stderr,
        rcap.stderr,
        "err20 long-name warning bytes differ:\n C={}\n R={}",
        show(&ccap.stderr),
        show(&rcap.stderr)
    );
}

// ---------------------------------------------------------------------------
// Row 21 — every flag-word bit pattern across the FFI boundary, including
// values with no meaningful "variant" and garbage padding bits.
// ---------------------------------------------------------------------------
#[test]
fn err_21_all_flag_bytes_and_padding_garbage() {
    let b = both();
    let mut r = Rng::with_seed(SEED ^ 0xC21);
    let _g = env_guard();
    let vals: Vec<i32> = EXTREMES
        .iter()
        .copied()
        .chain((0..24).map(|_| r.next_i32_mixed()))
        .collect();

    let mut words: Vec<FlagsWord> = Vec::new();
    for byte in 0..256u32 {
        words.push(byte);
        words.push(byte | 0xFFFF_FF00);
        words.push(byte | (r.next_u32() & 0xFFFF_FF00));
    }
    // Fully random words too — nothing constrains a caller to a "valid" value.
    for _ in 0..2000 {
        words.push(r.next_u32());
    }

    let (bad, _cap) = capture(|| {
        for &w in &words {
            for &v1 in &vals {
                for &v2 in &vals {
                    let mut fc = w;
                    let mut fr = w;
                    let cv = unsafe { (b.c.perform_operation)(v1, v2, &mut fc) };
                    let rv = unsafe { (b.rs.perform_operation)(v1, v2, &mut fr) };
                    if cv != rv || fc != fr {
                        return Some(("perform_operation", w, v1, v2, cv, rv, fc, fr));
                    }
                }
                let mut fc = w;
                let mut fr = w;
                let cv = unsafe { (b.c.apply_bit_operations)(v1, &mut fc) };
                let rv = unsafe { (b.rs.apply_bit_operations)(v1, &mut fr) };
                if cv != rv || fc != fr {
                    return Some(("apply_bit_operations", w, v1, 0, cv, rv, fc, fr));
                }
            }
        }
        None
    });
    if let Some((f, w, v1, v2, cv, rv, fc, fr)) = bad {
        panic!(
            "DIVERGENCE [err21] {f}(flags=0x{w:08x}, {v1}, {v2}): C={cv} Rust={rv} \
             (flags after: C=0x{fc:08x} Rust=0x{fr:08x})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 23 — int extremes on every scalar parameter of every entry point
// ---------------------------------------------------------------------------
#[test]
fn err_23_int_extremes() {
    let b = both();
    let _g = env_guard();
    let ext: [i32; 11] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];

    for cfg in [
        &[][..],
        &[("PROG_OPTIMIZE", Some("1"))][..],
        &[("PROG_VERBOSE", Some("1"))][..],
    ] {
        apply_env(cfg);
        let (bad, _cap) = capture(|| {
            for &a in &ext {
                for &b2 in &ext {
                    for &c in &ext {
                        for &d in &ext {
                            let cv = unsafe { (b.c.envy)(a, b2, c, d) };
                            let rv = unsafe { (b.rs.envy)(a, b2, c, d) };
                            if cv != rv {
                                return Some((a, b2, c, d, cv, rv));
                            }
                        }
                    }
                }
            }
            None
        });
        if let Some((a, b2, c, d, cv, rv)) = bad {
            clear_prog_env();
            panic!("DIVERGENCE [err23] env {cfg:?} envy({a}, {b2}, {c}, {d}): C={cv} Rust={rv}");
        }
    }
    clear_prog_env();

    // perform_operation / apply_bit_operations extremes across all log levels.
    let (bad, _cap) = capture(|| {
        for ll in 0..8u32 {
            for opt in 0..2u32 {
                let w = flag_word(0, 0, opt, 1, ll, 0);
                for &a in &ext {
                    for &b2 in &ext {
                        let mut fc = w;
                        let mut fr = w;
                        let cv = unsafe { (b.c.perform_operation)(a, b2, &mut fc) };
                        let rv = unsafe { (b.rs.perform_operation)(a, b2, &mut fr) };
                        if cv != rv {
                            return Some(("perform_operation", w, a, b2, cv, rv));
                        }
                    }
                }
            }
        }
        for verbose in 0..2u32 {
            for cache in 0..2u32 {
                let w = flag_word(verbose, 0, 0, cache, 3, 0);
                for &a in &ext {
                    let mut fc = w;
                    let mut fr = w;
                    let cv = unsafe { (b.c.apply_bit_operations)(a, &mut fc) };
                    let rv = unsafe { (b.rs.apply_bit_operations)(a, &mut fr) };
                    if cv != rv {
                        return Some(("apply_bit_operations", w, a, 0, cv, rv));
                    }
                }
            }
        }
        None
    });
    if let Some((f, w, a, b2, cv, rv)) = bad {
        panic!("DIVERGENCE [err23] {f}(flags=0x{w:08x}, {a}, {b2}): C={cv} Rust={rv}");
    }
}

// ---------------------------------------------------------------------------
// Row 24 — snprintf / BUFFER_SIZE. Truncation is unreachable because the
// longest possible "Result:%d:Complete" is 29 bytes; assert that fact against
// the C so the claim is verified rather than assumed.
// ---------------------------------------------------------------------------
#[test]
fn err_24_buffer_size_truncation_unreachable() {
    let longest = format!("Result:{}:Complete", i32::MIN);
    assert!(
        longest.len() + 1 <= 256,
        "the BUFFER_SIZE=256 truncation path would be reachable ({} bytes)",
        longest.len() + 1
    );
    // And drive envy at the extreme values that produce the longest string, in
    // both libraries, verbose on so the string is inspected by the C.
    let b = both();
    let _g = env_guard();
    apply_env(&[("PROG_VERBOSE", Some("1")), ("PROG_DEBUG", Some("1"))]);
    let cases = [
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 0, 0, 0),
        (0, 0, 0, 0),
    ];
    let (cres, ccap) = capture(|| {
        cases
            .iter()
            .map(|&(a, b2, c, d)| unsafe { (b.c.envy)(a, b2, c, d) })
            .collect::<Vec<i32>>()
    });
    let (rres, rcap) = capture(|| {
        cases
            .iter()
            .map(|&(a, b2, c, d)| unsafe { (b.rs.envy)(a, b2, c, d) })
            .collect::<Vec<i32>>()
    });
    clear_prog_env();
    assert_eq!(cres, rres, "err24 return values diverged");
    assert_eq!(
        ccap.stdout,
        rcap.stdout,
        "err24 stdout diverged:\n C={}\n R={}",
        show(&ccap.stdout),
        show(&rcap.stdout)
    );
}
