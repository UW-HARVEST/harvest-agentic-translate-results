// Phase C — error / rejection-path differential tests.
//
// One test per row of ERRORS.md, plus the generic C-API boundaries (NULL
// pointers, zero and oversized lengths, one-past-range values).

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// row 1 / 2 / 3 — call_fma's explicit `if (len == 0) return 0;` early-out
// ---------------------------------------------------------------------------

#[test]
fn err01_call_fma_len_zero() {
    let mut rng = Rng::new(SEED ^ 0x101);
    for _ in 0..2000 {
        let data: Vec<i32> = (0..1 + rng.below(8)).map(|_| rng.i32_corner()).collect();
        let c = unsafe { (c_impl().call_fma)(data.as_ptr(), 0) };
        let r = unsafe { (rust_impl().call_fma)(data.as_ptr(), 0) };
        assert_eq!(c, 0, "C must return the 0 sentinel for len == 0");
        assert_eq!(c, r, "call_fma(len=0) diverged");
    }
}

#[test]
fn err02_call_fma_len_zero_null_data() {
    // The early-out fires before `data` is ever dereferenced, so a NULL data
    // pointer is well defined here and must not fault in either library.
    let c = unsafe { (c_impl().call_fma)(std::ptr::null(), 0) };
    let r = unsafe { (rust_impl().call_fma)(std::ptr::null(), 0) };
    assert_eq!(c, 0);
    assert_eq!(c, r);
}

#[test]
fn err03_call_fma_len_one_boundary() {
    // One step past the rejected `len == 0`: must now read data[0].
    let mut rng = Rng::new(SEED ^ 0x103);
    for _ in 0..3000 {
        let v = rng.i32_corner();
        let data = [v, 0xDEAD_BEEFu32 as i32];
        let c = unsafe { (c_impl().call_fma)(data.as_ptr(), 1) };
        let r = unsafe { (rust_impl().call_fma)(data.as_ptr(), 1) };
        assert_eq!(c, v, "C: call_fma(len=1) must return data[0]");
        assert_eq!(c, r, "call_fma(len=1) diverged for data[0]={v}");
    }
}

// ---------------------------------------------------------------------------
// row 4 — call_fma with a negative `len` (C undefined behaviour)
// ---------------------------------------------------------------------------

#[test]
fn err04_call_fma_negative_len_ub() {
    // `int out[len]` with len < 0 is UB (C11 6.7.6.2p5).  Empirically, the C
    // build returns indeterminate garbage that changes from run to run
    // (observed: -991075438, then 32767) and faults outright for INT_MIN.
    // There is therefore no C behaviour to be identical to.  What we *can* and
    // do assert is that the Rust export is total: it returns the 0 sentinel,
    // never panics (the cdylib is panic=abort, so a panic would kill the
    // child) and never faults.
    let rust_call_fma = rust_impl().call_fma;
    let c_call_fma = c_impl().call_fma;
    for len in [-1i32, -2, -3, -100, i32::MIN + 1, i32::MIN] {
        let data = [7i32, 8, 9, 10];
        let p = data.as_ptr();
        // exit code 0 == returned the 0 sentinel, 7 == returned something else
        let out = run_isolated_code(move || {
            if unsafe { rust_call_fma(p, len) } == 0 { 0 } else { 7 }
        });
        assert_eq!(
            out,
            Outcome::Exited(0),
            "Rust call_fma(len={len}) must return 0 without crashing"
        );
    }
    // And confirm the C side really is the UB case we documented, i.e. it is
    // *not* a stable value we should be reproducing.
    let data = [7i32, 8, 9, 10];
    let p = data.as_ptr();
    let c_int_min = run_isolated(move || {
        let _ = unsafe { c_call_fma(p, i32::MIN) };
    });
    assert!(
        matches!(c_int_min, Outcome::Signaled(_)),
        "expected the documented C UB (fault) for call_fma(len=INT_MIN), got {c_int_min:?}"
    );
}

// ---------------------------------------------------------------------------
// rows 5-8 — fma_array's loop guard rejects len <= 0 (well defined)
// ---------------------------------------------------------------------------

#[test]
fn err05_fma_array_len_zero_no_writes() {
    let mut rng = Rng::new(SEED ^ 0x105);
    for _ in 0..2000 {
        let n = 1 + rng.below(8);
        let out: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        // assert_fma_array_eq compares every backing byte afterwards, so an
        // unexpected store in either library shows up here.
        assert_fma_array_eq(Alias::Distinct, 0, &out, &m1, &m2, &ad);
        // canary form: `out` must still hold its seed values in both
        let mut c_buf = out.clone();
        let mut r_buf = out.clone();
        unsafe {
            (c_impl().fma_array)(c_buf.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 0);
            (rust_impl().fma_array)(r_buf.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 0);
        }
        assert_eq!(c_buf, out, "C wrote to out despite len == 0");
        assert_eq!(r_buf, out, "Rust wrote to out despite len == 0");
    }
}

#[test]
fn err06_fma_array_len_zero_all_null() {
    let n = std::ptr::null_mut::<c_int>();
    let cn = std::ptr::null::<c_int>();
    let cf = c_impl().fma_array;
    let rf = rust_impl().fma_array;
    let c = run_isolated(move || unsafe { cf(n, cn, cn, cn, 0) });
    let r = run_isolated(move || unsafe { rf(n, cn, cn, cn, 0) });
    assert_eq!(c, Outcome::Exited(0), "C must tolerate len == 0 with NULLs");
    assert_eq!(c, r, "fma_array(NULL.., 0) diverged");
}

#[test]
fn err07_fma_array_negative_len_no_writes() {
    let mut rng = Rng::new(SEED ^ 0x107);
    for len in [-1i32, -2, -7, -100, i32::MIN + 1, i32::MIN] {
        for _ in 0..200 {
            let n = 1 + rng.below(8);
            let out: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            let m1: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            let m2: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            let ad: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            assert_fma_array_eq(Alias::Distinct, len, &out, &m1, &m2, &ad);
            let mut c_buf = out.clone();
            let mut r_buf = out.clone();
            unsafe {
                (c_impl().fma_array)(
                    c_buf.as_mut_ptr(),
                    m1.as_ptr(),
                    m2.as_ptr(),
                    ad.as_ptr(),
                    len,
                );
                (rust_impl().fma_array)(
                    r_buf.as_mut_ptr(),
                    m1.as_ptr(),
                    m2.as_ptr(),
                    ad.as_ptr(),
                    len,
                );
            }
            assert_eq!(c_buf, out, "C wrote to out despite len={len}");
            assert_eq!(r_buf, out, "Rust wrote to out despite len={len}");
        }
    }
}

#[test]
fn err08_fma_array_negative_len_all_null() {
    let nm = std::ptr::null_mut::<c_int>();
    let cn = std::ptr::null::<c_int>();
    let cf = c_impl().fma_array;
    let rf = rust_impl().fma_array;
    for len in [-1i32, -2, i32::MIN] {
        let c = run_isolated(move || unsafe { cf(nm, cn, cn, cn, len) });
        let r = run_isolated(move || unsafe { rf(nm, cn, cn, cn, len) });
        assert_eq!(c, Outcome::Exited(0), "C must tolerate len={len} with NULLs");
        assert_eq!(c, r, "fma_array(NULL.., {len}) diverged");
    }
}

// ---------------------------------------------------------------------------
// rows 9-12 — driver's `sscanf(...) != 1` rejection and the 100-item cap
// ---------------------------------------------------------------------------

#[test]
fn err09_driver_empty_input() {
    // sscanf returns EOF on the very first iteration => i == 0 => prints "0\n"
    assert_driver_cases(&[("", "0\n")]);
}

#[test]
fn err10_driver_no_parseable_number() {
    const BAD: [&str; 26] = [
        "", " ", "  ", "\t", "\n", "\r\n", "\x0b", "\x0c", "abc", "+", "-", "++", "--", "+-",
        "x", "X", ".", ",", "0x", "e5", "-x", "+ 1", "- 1", "#!", "%d", "\u{7f}\u{1}",
    ];
    let cases: Vec<(&str, &str)> = BAD.iter().map(|s| (*s, "0\n")).collect();
    assert_driver_cases(&cases);
}

#[test]
fn err11_driver_partial_parse_break() {
    // The loop breaks at the first unparseable token; the printed value is the
    // last SUCCESSFULLY parsed integer.
    let cases: [(&str, &str); 12] = [
        ("1 2 x 4", "2\n"),
        ("1,2", "1\n"),
        ("7 -", "7\n"),
        ("7 +", "7\n"),
        ("5 abc 9", "5\n"),
        ("0x10 7", "0\n"),
        ("12abc 5", "12\n"),
        ("3 4 5.6 7", "5\n"),
        ("-8;9", "-8\n"),
        ("1 2 3 \t\n", "3\n"),
        ("42", "42\n"),
        ("  -42  ", "-42\n"),
    ];
    assert_driver_cases(&cases);
    // randomized: a garbage token spliced into an otherwise valid list
    let mut rng = Rng::new(SEED ^ 0x111);
    let mut inputs: Vec<String> = Vec::new();
    const JUNK: [&str; 10] = ["x", "abc", "+", "-", ",", ".", "0x", "e", "!", "/"];
    for _ in 0..1000 {
        let n = 1 + rng.below(12);
        let vals: Vec<i32> = (0..n).map(|_| rng.range_i32(-99_999, 99_999)).collect();
        let cut = rng.below(n + 1);
        let mut s = String::new();
        for (i, v) in vals.iter().enumerate() {
            if i == cut {
                s.push_str(rng.pick(&JUNK));
                s.push(' ');
            }
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&v.to_string());
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn err12_driver_over_100_items_cap() {
    let mut rng = Rng::new(SEED ^ 0x112);
    let mut inputs: Vec<String> = Vec::new();
    let mut expect: Vec<String> = Vec::new();
    for &n in &[101usize, 102, 128, 150, 200, 500] {
        for _ in 0..10 {
            let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            inputs.push(
                vals.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            // hard cap `i < 100`: the 100th value is what gets printed
            expect.push(format!("{}\n", vals[99]));
        }
    }
    let out = assert_driver_batch_bytes(&inputs);
    for (k, e) in expect.iter().enumerate() {
        assert_eq!(
            String::from_utf8_lossy(&out[k]),
            *e,
            "C must cap at 100 items (input #{k})"
        );
    }
}

// ---------------------------------------------------------------------------
// rows 13/14 — %d values outside / at the edge of the int range
// ---------------------------------------------------------------------------

#[test]
fn err13_driver_int_range_overflow() {
    // glibc's %d converts with strtol and stores the low 32 bits of the
    // (clamped) long.  Both libraries go through the very same libc entry
    // point (__isoc99_sscanf), so the results must be identical.
    let cases: [(&str, &str); 10] = [
        ("2147483648", "-2147483648\n"),  // INT_MAX + 1
        ("-2147483649", "2147483647\n"),  // INT_MIN - 1
        ("99999999999999999999", "-1\n"), // clamps to LONG_MAX -> low 32 bits
        ("-99999999999999999999", "0\n"), // clamps to LONG_MIN -> low 32 bits
        ("4294967296", "0\n"),
        ("4294967295", "-1\n"),
        ("9223372036854775807", "-1\n"),  // LONG_MAX
        ("-9223372036854775808", "0\n"),  // LONG_MIN
        ("9223372036854775808", "-1\n"),  // LONG_MAX + 1 -> clamped
        ("2147483648 5", "5\n"),
    ];
    assert_driver_cases(&cases);
    // randomized out-of-range magnitudes
    let mut rng = Rng::new(SEED ^ 0x113);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..800 {
        let digits = 10 + rng.below(20);
        let mut s = String::new();
        if rng.below(2) == 0 {
            s.push('-');
        }
        s.push(char::from(b'1' + rng.below(9) as u8));
        for _ in 1..digits {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn err14_driver_int_extremes() {
    assert_driver_cases(&[
        ("2147483647", "2147483647\n"),
        ("-2147483648", "-2147483648\n"),
        ("+2147483647", "2147483647\n"),
        ("0", "0\n"),
        ("-0", "0\n"),
        ("+0", "0\n"),
        ("0000000000000000007", "7\n"),
    ]);
}

// ---------------------------------------------------------------------------
// row 15 — signed int overflow inside fma_array
// ---------------------------------------------------------------------------

#[test]
fn err15_fma_array_signed_overflow() {
    // Overflow of `mul1[i] * mul2[i] + add[i]` is UB in C; the C library is
    // built at -O0 with no -fwrapv/-ftrapv, so gcc emits wrapping imul/add.
    // The Rust translation uses wrapping_mul/wrapping_add and must agree.
    const CORNERS: [i32; 10] = [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -2,
        -1,
        1,
        2,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &CORNERS {
        for &b in &CORNERS {
            for &c in &CORNERS {
                assert_fma_array_eq(Alias::Distinct, 1, &[0], &[a], &[b], &[c]);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x115);
    for _ in 0..2000 {
        let n = 1 + rng.below(16);
        let out = vec![0i32; n];
        let m1: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        assert_fma_array_eq(Alias::Distinct, n as i32, &out, &m1, &m2, &ad);
    }
}

// ---------------------------------------------------------------------------
// rows 16-18 — oversized lengths / inputs and the exact 100 boundary
// ---------------------------------------------------------------------------

#[test]
fn err16_call_fma_oversized_len() {
    // Run on a big stack: the C VLAs need ~12 * len bytes of the caller's stack.
    on_big_stack(|| {
        let mut rng = Rng::new(SEED ^ 0x116);
        for &len in &[10_000usize, 65_536, 100_000, 1_000_000] {
            let data: Vec<i32> = (0..len).map(|_| rng.i32_full()).collect();
            let c = unsafe { (c_impl().call_fma)(data.as_ptr(), len as c_int) };
            let r = unsafe { (rust_impl().call_fma)(data.as_ptr(), len as c_int) };
            assert_eq!(c, data[len - 1], "C: call_fma must return data[len-1]");
            assert_eq!(c, r, "call_fma(len={len}) diverged");
        }
    });
}

// ---------------------------------------------------------------------------
// row 21 — `len` beyond the caller's stack budget (resource exhaustion)
// ---------------------------------------------------------------------------

#[test]
fn err21_call_fma_len_beyond_stack_budget() {
    // `call_fma` declares THREE `int[len]` VLAs and never checks `len` against
    // the available stack, so for `len > RLIMIT_STACK / 12` the C build simply
    // walks off its stack and dies with SIGSEGV.  Measured on an 8 MiB stack:
    // len = 690_000 still works, len = 700_000 faults.
    //
    // This is resource exhaustion, not a computed result: there is no value for
    // the Rust translation to reproduce.  The Rust version allocates the three
    // arrays on the HEAP, so it keeps working.  This is a documented, deliberate
    // deviation (ERRORS.md row 21); the test pins down BOTH halves of it:
    //   (a) the two libraries agree for every `len` that fits the stack, and
    //   (b) past that boundary the C side really is a stack fault, while the
    //       Rust side returns the mathematically correct answer.
    const STACK: usize = 8 * 1024 * 1024;

    // (a) agreement just below the C stack limit (8 MiB / 12 ~= 699 050)
    for &len in &[600_000usize, 650_000] {
        let (c, r) = on_stack_of(STACK, move || {
            let data: Vec<i32> = (0..len).map(|i| i as i32).collect();
            let c = unsafe { (c_impl().call_fma)(data.as_ptr(), len as c_int) };
            let r = unsafe { (rust_impl().call_fma)(data.as_ptr(), len as c_int) };
            (c, r)
        });
        assert_eq!(c, (len - 1) as i32, "C must still work at len={len}");
        assert_eq!(c, r, "call_fma(len={len}) diverged below the stack limit");
    }

    // (b) past the limit: C faults, Rust does not
    let big: usize = 4_000_000;
    let c_outcome = on_stack_of(STACK, move || {
        let data: Vec<i32> = (0..big).map(|i| i as i32).collect();
        let p = data.as_ptr();
        let f = c_impl().call_fma;
        run_isolated(move || {
            let _ = unsafe { f(p, big as c_int) };
        })
    });
    assert!(
        matches!(c_outcome, Outcome::Signaled(11)),
        "expected the documented C stack exhaustion (SIGSEGV) at len={big}, got {c_outcome:?}"
    );

    let r_outcome = on_stack_of(STACK, move || {
        let data: Vec<i32> = (0..big).map(|i| i as i32).collect();
        let p = data.as_ptr();
        let f = rust_impl().call_fma;
        run_isolated_code(move || {
            // heap-allocated, so this must succeed and be arithmetically right
            if unsafe { f(p, big as c_int) } == (big - 1) as i32 { 0 } else { 7 }
        })
    });
    assert_eq!(
        r_outcome,
        Outcome::Exited(0),
        "Rust call_fma(len={big}) must heap-allocate and return data[len-1]"
    );
}

#[test]
fn err17_driver_oversized_input() {
    let mut rng = Rng::new(SEED ^ 0x117);
    let mut inputs: Vec<String> = Vec::new();
    let mut expect: Vec<String> = Vec::new();
    for _ in 0..5 {
        let mut vals: Vec<i32> = Vec::new();
        let mut s = String::with_capacity(120_000);
        while s.len() < 100_000 {
            let v = rng.i32_full();
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&v.to_string());
            vals.push(v);
        }
        inputs.push(s);
        expect.push(format!("{}\n", vals[99]));
    }
    let out = assert_driver_batch_bytes(&inputs);
    for (k, e) in expect.iter().enumerate() {
        assert_eq!(
            String::from_utf8_lossy(&out[k]),
            *e,
            "C must still cap at 100 for oversized input #{k}"
        );
    }
}

#[test]
fn err18_driver_exactly_100_items() {
    let mut rng = Rng::new(SEED ^ 0x118);
    let mut inputs: Vec<String> = Vec::new();
    let mut expect: Vec<String> = Vec::new();
    // exactly 100 (the largest count that is not capped), then 99
    for &n in &[100usize, 99] {
        for _ in 0..50 {
            let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            inputs.push(
                vals.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            expect.push(format!("{}\n", vals[n - 1]));
        }
    }
    let out = assert_driver_batch_bytes(&inputs);
    for (k, e) in expect.iter().enumerate() {
        assert_eq!(String::from_utf8_lossy(&out[k]), *e, "input #{k}");
    }
}

// ---------------------------------------------------------------------------
// rows 19-20 — NULL pointer dereference (fork-isolated)
// ---------------------------------------------------------------------------

#[test]
fn err19_driver_null_input_ub() {
    // driver(NULL) reaches __isoc99_sscanf(NULL, ...) in both libraries.
    let cd = c_impl().driver;
    let rd = rust_impl().driver;
    let c = run_isolated(move || unsafe { cd(std::ptr::null()) });
    let r = run_isolated(move || unsafe { rd(std::ptr::null()) });
    assert!(
        matches!(c, Outcome::Signaled(11)),
        "expected the documented C UB (SIGSEGV) for driver(NULL), got {c:?}"
    );
    assert_eq!(c, r, "driver(NULL) outcome diverged");
}

#[test]
fn err20_call_fma_null_data_positive_len_ub() {
    let cc = c_impl().call_fma;
    let rc = rust_impl().call_fma;
    let cf = c_impl().fma_array;
    let rf = rust_impl().fma_array;
    for len in [1i32, 2, 4, 100] {
        let c = run_isolated(move || {
            let _ = unsafe { cc(std::ptr::null(), len) };
        });
        let r = run_isolated(move || {
            let _ = unsafe { rc(std::ptr::null(), len) };
        });
        assert!(
            matches!(c, Outcome::Signaled(11)),
            "expected the documented C UB (SIGSEGV) for call_fma(NULL,{len}), got {c:?}"
        );
        assert_eq!(c, r, "call_fma(NULL,{len}) outcome diverged");
    }
    // fma_array with a NULL input and a positive len is the same UB
    for len in [1i32, 8] {
        let mut buf = vec![0i32; 8];
        let p = buf.as_mut_ptr();
        let nn = std::ptr::null::<c_int>();
        let ok = vec![1i32; 8];
        let okp = ok.as_ptr();
        let c = run_isolated(move || unsafe { cf(p, nn, okp, okp, len) });
        let r = run_isolated(move || unsafe { rf(p, nn, okp, okp, len) });
        assert!(
            matches!(c, Outcome::Signaled(11)),
            "expected SIGSEGV from C fma_array(NULL mul1, len={len}), got {c:?}"
        );
        assert_eq!(c, r, "fma_array(NULL mul1, {len}) outcome diverged");
    }
    // Each of the three *input* pointers individually NULL: the faulting load
    // is a different one of the three in each case.
    for len in [1i32, 8] {
        for which in 0..3usize {
            let mut buf = vec![0i32; 8];
            let p = buf.as_mut_ptr();
            let ok = vec![3i32; 8];
            let okp = ok.as_ptr();
            let nn = std::ptr::null::<c_int>();
            let (m1, m2, ad) = match which {
                0 => (nn, okp, okp),
                1 => (okp, nn, okp),
                _ => (okp, okp, nn),
            };
            let c = run_isolated(move || unsafe { cf(p, m1, m2, ad, len) });
            let r = run_isolated(move || unsafe { rf(p, m1, m2, ad, len) });
            assert!(
                matches!(c, Outcome::Signaled(11)),
                "expected SIGSEGV from C fma_array(input #{which} NULL, len={len}), got {c:?}"
            );
            assert_eq!(c, r, "fma_array(input #{which} NULL, len={len}) diverged");
        }
    }
    // NULL *out* with valid inputs and a positive len: all three loads succeed
    // and it is the STORE that faults.  Without this case a translation could
    // use a checked store (which aborts) instead of a plain one (which faults)
    // and no test would notice.
    for len in [1i32, 2, 8] {
        let ok = vec![5i32; 8];
        let okp = ok.as_ptr();
        let nm = std::ptr::null_mut::<c_int>();
        let c = run_isolated(move || unsafe { cf(nm, okp, okp, okp, len) });
        let r = run_isolated(move || unsafe { rf(nm, okp, okp, okp, len) });
        assert!(
            matches!(c, Outcome::Signaled(11)),
            "expected SIGSEGV from C fma_array(NULL out, len={len}), got {c:?}"
        );
        assert_eq!(c, r, "fma_array(NULL out, len={len}) outcome diverged");
    }
}

// ---------------------------------------------------------------------------
// generic boundaries required by the task, beyond the table
// ---------------------------------------------------------------------------

#[test]
fn err22_no_enum_parameters_int_boundary_sweep() {
    // The public API has no enum parameters, so the analogous "value with no
    // valid meaning crossing the FFI boundary" is an out-of-domain `int len`.
    // fma_array's domain is well defined for every int (loop guard), so sweep
    // the whole boundary neighbourhood of 0 and of INT_MIN/INT_MAX.
    let mut rng = Rng::new(SEED ^ 0x121);
    let mut lens: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        8,
        9,
        16,
        17,
    ];
    for _ in 0..64 {
        lens.push(rng.range_i32(-40, 16));
    }
    for len in lens {
        let n = 16usize;
        let out: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.i32_corner()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.i32_corner()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.i32_corner()).collect();
        // len is clamped to the buffer size for the positive side so that both
        // libraries stay in-bounds; negative / zero lens go through verbatim.
        let effective = if len > n as i32 { n as i32 } else { len };
        assert_fma_array_eq(Alias::Distinct, effective, &out, &m1, &m2, &ad);
    }
    // call_fma: 0 and one step past it in both directions is covered by rows
    // 1/3/4; here check the positive boundary against the buffer size.
    for len in [1i32, 2, 15, 16] {
        let data: Vec<i32> = (0..16).map(|_| rng.i32_corner()).collect();
        let c = unsafe { (c_impl().call_fma)(data.as_ptr(), len) };
        let r = unsafe { (rust_impl().call_fma)(data.as_ptr(), len) };
        assert_eq!(c, data[len as usize - 1]);
        assert_eq!(c, r);
    }
}

#[test]
fn err23_driver_exhaustive_short_inputs() {
    // every single-byte input (NUL cannot appear inside a C string)
    let mut inputs: Vec<Vec<u8>> = (1u8..=255).map(|b| vec![b]).collect();
    // and every two-byte ASCII input
    for a in 1u8..=127 {
        for b in 1u8..=127 {
            inputs.push(vec![a, b]);
        }
    }
    // and every three-byte input over the bytes that actually matter to %d
    const INTERESTING: [u8; 16] = [
        b' ', b'\t', b'\n', b'+', b'-', b'0', b'1', b'9', b'x', b'a', b'.', b',', b'e', 0x0b,
        0x0c, 0x7f,
    ];
    for &a in &INTERESTING {
        for &b in &INTERESTING {
            for &c in &INTERESTING {
                inputs.push(vec![a, b, c]);
            }
        }
    }
    assert_driver_batch_bytes(&inputs);
}
