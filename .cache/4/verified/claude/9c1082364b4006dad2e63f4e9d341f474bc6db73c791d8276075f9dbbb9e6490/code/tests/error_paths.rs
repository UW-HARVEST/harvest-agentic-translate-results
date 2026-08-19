//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every case constructs the exact invalid
//! input the C checks for (or fails to check for) and asserts that the C `.so`
//! and the Rust `.so` produce the *same* sentinel — the same return value, the
//! same stdout bytes, or the same fatal signal. Both libraries are always
//! reached through `dlopen`/`dlsym`; the Rust is never called directly.

mod common;

use common::*;
use std::os::raw::c_int;

const CANARY: i32 = 0x5A5A_5A5Au32 as i32;

// ===========================================================================
// E1 — call_fma(len == 0): the one explicit rejection in the C source.
// ===========================================================================
#[test]
fn err_e1_call_fma_len_zero() {
    let mut rng = Rng::new(0xE001);
    for (c, r) in pairs() {
        for _ in 0..500 {
            let n = 1 + rng.below(32);
            let data = rng.vec_i32(n);
            let cv = unsafe { (c.call_fma)(data.as_ptr(), 0) };
            let rv = unsafe { (r.call_fma)(data.as_ptr(), 0) };
            assert_eq!(cv, 0, "E1: C must return the 0 sentinel");
            assert_eq!(rv, cv, "E1: {} vs {}", c.name, r.name);
        }
    }
}

// ===========================================================================
// E2 — call_fma(NULL, 0): early return happens before any dereference.
// ===========================================================================
#[test]
fn err_e2_call_fma_len_zero_null_data() {
    for (c, r) in pairs() {
        let cv = unsafe { (c.call_fma)(std::ptr::null(), 0) };
        let rv = unsafe { (r.call_fma)(std::ptr::null(), 0) };
        assert_eq!(cv, 0, "E2: C must return 0 for NULL/len=0");
        assert_eq!(rv, cv, "E2: {} vs {}", c.name, r.name);
    }
    // ...and out of process, to prove neither faults.
    for (tag, so) in c_so_variants() {
        let cp = probe(&so, "call_fma_null", &["0".to_string()], None);
        let rp = probe(&rust_so(), "call_fma_null", &["0".to_string()], None);
        assert_eq!(
            (cp.code, cp.signal, cp.stdout.clone()),
            (rp.code, rp.signal, rp.stdout.clone()),
            "E2 out-of-process mismatch (C[{tag}])\n  C   : {}\n  Rust: {}",
            cp.describe(),
            rp.describe()
        );
        assert_eq!(cp.signal, None, "E2: C must not fault");
        assert_eq!(cp.stdout, b"0\n", "E2: C must print the 0 sentinel");
    }
}

// ===========================================================================
// E3 — call_fma(len == 1): minimum length that reaches the VLA path.
// ===========================================================================
#[test]
fn err_e3_call_fma_len_one() {
    let mut rng = Rng::new(0xE003);
    for (c, r) in pairs() {
        for _ in 0..1000 {
            let data = vec![rng.next_i32()];
            let cv = unsafe { (c.call_fma)(data.as_ptr(), 1) };
            let rv = unsafe { (r.call_fma)(data.as_ptr(), 1) };
            assert_eq!(cv, data[0], "E3: C must return data[0]");
            assert_eq!(rv, cv, "E3: {} vs {} data={:?}", c.name, r.name, data);
        }
    }
}

// ===========================================================================
// E4 — call_fma(len < 0): negative VLA size + out-of-bounds read == UB.
//
// `int out[len]` with a negative `len` converts to an enormous `size_t`, so the
// `alloca` moves the stack pointer by a wrapped amount and `return out[len-1]`
// then reads outside the frame. Measured (out of process, so the crashes are
// survivable):
//
//     len          libcref.so (-O0)      libcref_o2.so (-O2)
//     -1           0 / 64250770          0 / 889127570      <- garbage value
//     -2           0 / 32766             0 / 0              <- garbage value
//     -7           SIGSEGV               SIGSEGV
//     -100         SIGSEGV               0 / 32765
//     -65536       SIGSEGV               SIGSEGV
//     INT_MIN+1    SIGSEGV               SIGSEGV
//     INT_MIN      SIGSEGV               SIGSEGV
//
// The value is neither stable across runs nor across optimisation levels, and
// whether it even survives depends on the codegen, so there is no defined
// result for the Rust to reproduce. This test therefore asserts the only
// checkable properties: the Rust is memory-safe and deterministic here, and it
// never faults. The C side is exercised out of process purely to keep this
// documented behaviour honest.
// ===========================================================================
#[test]
fn err_e4_call_fma_negative_len() {
    for len in [-1i32, -2, -7, -100, -65536, i32::MIN + 1, i32::MIN] {
        let args = vec![len.to_string(), "64".to_string(), "12345".to_string()];

        // The C is UB: record it, assert nothing about the value/signal.
        for (tag, so) in c_so_variants() {
            let cp = probe(&so, "call_fma", &args, None);
            eprintln!("E4 [documented UB] C[{tag}] len={len}: {}", cp.describe());
        }

        // The Rust must be safe and reproducible.
        let rp1 = probe(&rust_so(), "call_fma", &args, None);
        let rp2 = probe(&rust_so(), "call_fma", &args, None);
        assert_eq!(
            rp1.signal, None,
            "E4: Rust must not fault for len={len}: {}",
            rp1.describe()
        );
        assert_eq!(rp1.code, Some(0), "E4: Rust len={len}: {}", rp1.describe());
        assert_eq!(
            rp1.stdout, rp2.stdout,
            "E4: Rust must be deterministic for len={len}"
        );
        assert_eq!(
            rp1.stdout, b"0\n",
            "E4: Rust returns a deterministic 0 for len={len}"
        );
    }

    // In-process (Rust only -- calling the C here would take the test runner
    // down with it): repeated calls must not fault or diverge from themselves.
    for (_c, r) in pairs() {
        let data = [7i32; 16];
        for len in [-1i32, -2, -100, i32::MIN] {
            let a = unsafe { (r.call_fma)(data.as_ptr(), len) };
            let b = unsafe { (r.call_fma)(data.as_ptr(), len) };
            assert_eq!(a, b, "E4: Rust not deterministic for len={len}");
            assert_eq!(a, 0, "E4: Rust must return 0 for len={len}");
        }
    }
}

// ===========================================================================
// E5 — call_fma(NULL, len > 0): the C dereferences data unconditionally.
// ===========================================================================
#[test]
fn err_e5_call_fma_null_data_positive_len() {
    for len in [1i32, 2, 8, 100] {
        let args = vec![len.to_string()];
        let rp = probe(&rust_so(), "call_fma_null", &args, None);
        for (tag, so) in c_so_variants() {
            let cp = probe(&so, "call_fma_null", &args, None);
            assert_eq!(
                cp.signal,
                Some(libc_sigsegv()),
                "E5: C[{tag}] must SIGSEGV for NULL/len={len}: {}",
                cp.describe()
            );
            assert_eq!(
                (cp.signal, cp.code),
                (rp.signal, rp.code),
                "E5 mismatch C[{tag}] vs Rust for len={len}\n  C   : {}\n  Rust: {}",
                cp.describe(),
                rp.describe()
            );
        }
    }
}

// ===========================================================================
// E6 — call_fma with a `len` whose VLAs overflow the stack.
//
// `int out[len]; int ones[len]; int zeros[len];` needs 12*len bytes of stack.
// For len == INT_MAX that is 24 GiB, so the C dies on the guard page: UB, not
// a reportable error. The Rust heap-allocates and therefore cannot reproduce a
// stack overflow. Documented, not asserted equal.
//
// What is asserted: (a) the C really does die abnormally there, which is what
// justifies treating the row as UB rather than as a value contract, and (b)
// the two agree for *every* length that fits on the stack, right up to the
// largest one tested (200 000 elements => 2.4 MiB), which is the defined part
// of the range.
// ===========================================================================
#[test]
fn err_e6_call_fma_large_len_documented() {
    // (b) the defined part of the range. The C puts 12*len bytes of VLAs on
    // the caller's stack, so this runs on a thread with room for them.
    with_big_stack(|| {
        let mut rng = Rng::new(0xE006);
        for (c, r) in pairs() {
            for len in [1i32, 2, 1024, 100_000, 200_000, 1_000_000, 8_000_000] {
                let data = rng.vec_i32(len as usize);
                assert_call_fma_eq(&c, &r, "E6", &data, len);
            }
        }
    });

    // (a) INT_MAX really is a crash in the C, i.e. there is no value to match.
    let args = vec![i32::MAX.to_string(), "64".to_string(), "1".to_string()];
    for (tag, so) in c_so_variants() {
        let cp = probe(&so, "call_fma", &args, None);
        assert!(
            cp.signal.is_some(),
            "E6: expected C[{tag}] to die from the VLA stack overflow at len=INT_MAX, got {}",
            cp.describe()
        );
    }
}

// ===========================================================================
// E7 — fma_array(len == 0) never dereferences its pointers (all NULL is safe).
// ===========================================================================
#[test]
fn err_e7_fma_array_len_zero_all_null() {
    for (c, r) in pairs() {
        unsafe {
            (c.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
            (r.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
        }
    }
    let args = vec!["0".to_string()];
    let rp = probe(&rust_so(), "fma_array_all_null", &args, None);
    for (tag, so) in c_so_variants() {
        let cp = probe(&so, "fma_array_all_null", &args, None);
        assert_eq!(cp.signal, None, "E7: C[{tag}] must not fault: {}", cp.describe());
        assert_eq!(
            (cp.code, cp.signal, cp.stdout.clone()),
            (rp.code, rp.signal, rp.stdout.clone()),
            "E7 mismatch C[{tag}] vs Rust\n  C   : {}\n  Rust: {}",
            cp.describe(),
            rp.describe()
        );
    }
}

// ===========================================================================
// E8 — fma_array(len < 0): same loop guard, still zero iterations.
// ===========================================================================
#[test]
fn err_e8_fma_array_negative_len_all_null() {
    for len in [-1i32, -2, -100, i32::MIN + 1, i32::MIN] {
        for (c, r) in pairs() {
            unsafe {
                (c.fma_array)(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                );
                (r.fma_array)(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                );
            }
        }
        let args = vec![len.to_string()];
        let rp = probe(&rust_so(), "fma_array_all_null", &args, None);
        for (tag, so) in c_so_variants() {
            let cp = probe(&so, "fma_array_all_null", &args, None);
            assert_eq!(
                cp.signal, None,
                "E8: C[{tag}] must not fault for len={len}: {}",
                cp.describe()
            );
            assert_eq!(
                (cp.code, cp.signal, cp.stdout.clone()),
                (rp.code, rp.signal, rp.stdout.clone()),
                "E8 mismatch C[{tag}] vs Rust for len={len}\n  C   : {}\n  Rust: {}",
                cp.describe(),
                rp.describe()
            );
        }
    }
}

// ===========================================================================
// E9 — fma_array with out == NULL and len > 0: SIGSEGV on the first store.
// ===========================================================================
#[test]
fn err_e9_fma_array_null_out() {
    for len in [1i32, 2, 8, 100] {
        let args = vec![len.to_string()];
        let rp = probe(&rust_so(), "fma_array_null_out", &args, None);
        for (tag, so) in c_so_variants() {
            let cp = probe(&so, "fma_array_null_out", &args, None);
            assert_eq!(
                cp.signal,
                Some(libc_sigsegv()),
                "E9: C[{tag}] must SIGSEGV for out=NULL len={len}: {}",
                cp.describe()
            );
            assert_eq!(
                (cp.signal, cp.code),
                (rp.signal, rp.code),
                "E9 mismatch C[{tag}] vs Rust for len={len}\n  C   : {}\n  Rust: {}",
                cp.describe(),
                rp.describe()
            );
        }
    }
}

// ===========================================================================
// E10 — fma_array with a NULL read-only input and len > 0.
// ===========================================================================
#[test]
fn err_e10_fma_array_null_inputs() {
    for which in ["mul1", "mul2", "add", "all"] {
        for len in [1i32, 8, 100] {
            let args = vec![len.to_string(), which.to_string()];
            let rp = probe(&rust_so(), "fma_array_null_in", &args, None);
            for (tag, so) in c_so_variants() {
                let cp = probe(&so, "fma_array_null_in", &args, None);
                assert_eq!(
                    cp.signal,
                    Some(libc_sigsegv()),
                    "E10: C[{tag}] must SIGSEGV for {which}=NULL len={len}: {}",
                    cp.describe()
                );
                assert_eq!(
                    (cp.signal, cp.code),
                    (rp.signal, rp.code),
                    "E10 mismatch C[{tag}] vs Rust for {which}=NULL len={len}\n  C   : {}\n  Rust: {}",
                    cp.describe(),
                    rp.describe()
                );
            }
        }
    }
}

// ===========================================================================
// E11 — fma_array has no bounds check: len one past the caller's real length
// writes one element past the end, identically in both builds.
// ===========================================================================
#[test]
fn err_e11_fma_array_one_past_end() {
    let mut rng = Rng::new(0xE011);
    for (c, r) in pairs() {
        for _ in 0..1000 {
            let n = 1 + rng.below(32);
            // Real allocation is n + 8 so the one-past-end access stays inside
            // the allocation and is therefore observable rather than fatal.
            let total = n + 8;
            let m1 = rng.vec_i32(total);
            let m2 = rng.vec_i32(total);
            let ad = rng.vec_i32(total);
            let tmpl = vec![CANARY; total];
            let len = (n + 1) as c_int;
            let (cv, rv) = run_fma_array(&c, &r, &tmpl, &m1, &m2, &ad, len, Alias::None);
            assert_eq!(cv, rv, "E11 {} vs {}: n={n} len={len}", c.name, r.name);
            // The element at index n -- the caller's "padding" -- was written.
            let expect = m1[n].wrapping_mul(m2[n]).wrapping_add(ad[n]);
            assert_eq!(cv[n], expect, "E11: C did bounds-check unexpectedly");
            assert!(
                cv[n + 1..].iter().all(|&x| x == CANARY),
                "E11: writes went further than one past the end"
            );
        }
    }

    // And out of process with a genuinely too-small buffer: an unbounded run
    // off the end must fault the same way in both.
    let args = vec!["16".to_string(), i32::MAX.to_string()];
    let rp = probe(&rust_so(), "fma_array_len", &args, None);
    for (tag, so) in c_so_variants() {
        let cp = probe(&so, "fma_array_len", &args, None);
        assert_eq!(
            (cp.signal, cp.code),
            (rp.signal, rp.code),
            "E11 out-of-range len mismatch C[{tag}] vs Rust\n  C   : {}\n  Rust: {}",
            cp.describe(),
            rp.describe()
        );
        assert_eq!(
            cp.signal,
            Some(libc_sigsegv()),
            "E11: C[{tag}] must SIGSEGV walking off the end: {}",
            cp.describe()
        );
    }
}

// ===========================================================================
// E12 — main() with empty stdin: first scanf returns EOF, i == 0.
// E22 — and `int data[100]` stays unobserved because call_fma(_, 0) returns
//       before touching it.
// ===========================================================================
#[test]
fn err_e12_main_empty_input() {
    for (tag, so) in c_so_variants() {
        assert_main_eq(&so, tag, b"", false);
        let cp = probe(&so, "main", &[], Some(b""));
        assert_eq!(cp.stdout, b"0\n", "E12: C[{tag}] must print 0");
        assert_eq!(cp.code, Some(0), "E12: C[{tag}] exit status");
    }
    assert_driver_eq(b"", false);
}

// ===========================================================================
// E13 — main() with whitespace-only stdin.
// ===========================================================================
#[test]
fn err_e13_main_whitespace_only() {
    let cases: Vec<Vec<u8>> = vec![
        b" ".to_vec(),
        b"\n".to_vec(),
        b"\t".to_vec(),
        b"\r".to_vec(),
        b"\x0b".to_vec(),
        b"\x0c".to_vec(),
        b" \t\n\r\x0b\x0c".to_vec(),
        vec![b' '; 5000],
        b"\n\n\n\n\n".to_vec(),
    ];
    for (tag, so) in c_so_variants() {
        for cs in &cases {
            assert_main_eq(&so, tag, cs, false);
            let cp = probe(&so, "main", &[], Some(cs));
            assert_eq!(
                cp.stdout,
                b"0\n",
                "E13: C[{tag}] must print 0 for {:?}",
                String::from_utf8_lossy(&cs[..cs.len().min(16)])
            );
        }
    }
    for cs in &cases {
        assert_driver_eq(cs, false);
    }
}

// ===========================================================================
// E14 / E17 — main() with a non-numeric (or sign-only) first token.
// ===========================================================================
#[test]
fn err_e14_main_leading_non_numeric() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for t in NON_NUMERIC_TOKENS {
        cases.push(t.as_bytes().to_vec());
        cases.push(format!("{t}\n").into_bytes());
        cases.push(format!("  \t{t} 5 6").into_bytes());
    }
    // Every single non-digit, non-sign, non-space byte on its own.
    for b in 0u8..=127 {
        if b.is_ascii_digit() || b == b'+' || b == b'-' || is_space(b) {
            continue;
        }
        cases.push(vec![b]);
        cases.push(vec![b, b'1', b'2']);
    }
    // Sign followed by a non-digit, and sign at end of input (E17).
    for b in [b'\0', b'a', b'.', b'-', b'+', b' '] {
        cases.push(vec![b'-', b]);
        cases.push(vec![b'+', b]);
    }
    cases.push(b"-".to_vec());
    cases.push(b"+".to_vec());

    for (tag, so) in c_so_variants() {
        for cs in &cases {
            assert_main_eq(&so, tag, cs, false);
            let cp = probe(&so, "main", &[], Some(cs));
            assert_eq!(
                cp.stdout,
                b"0\n",
                "E14: C[{tag}] must print 0 for {:?}",
                String::from_utf8_lossy(cs)
            );
        }
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

// ===========================================================================
// E15 — main() breaks mid-stream: k valid integers then a bad token.
// ===========================================================================
#[test]
fn err_e15_main_break_mid_stream() {
    let mut rng = Rng::new(0xE015);
    let mut cases: Vec<(Vec<u8>, i32)> = Vec::new();
    for _ in 0..300 {
        let k = 1 + rng.below(10);
        let vals: Vec<i32> = (0..k).map(|_| rng.next_i32()).collect();
        // Only fully non-numeric tokens, so the expected answer is exactly
        // data[k-1]; tokens with a digit prefix are covered by E16.
        let bad = rng.pick(&NON_NUMERIC_TOKENS).to_string();
        let mut s = String::new();
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&v.to_string());
        }
        s.push(' ');
        s.push_str(&bad);
        s.push_str(" 999 1000");
        cases.push((s.into_bytes(), vals[k - 1]));
    }
    cases.push((b"7 abc 9".to_vec(), 7));
    for (tag, so) in c_so_variants() {
        for (cs, expected) in &cases {
            assert_main_eq(&so, tag, cs, false);
            // Pin the C's own answer, proving the break really happened at the
            // bad token rather than at the end of the stream.
            let cp = probe(&so, "main", &[], Some(cs));
            assert_eq!(
                String::from_utf8_lossy(&cp.stdout),
                format!("{expected}\n"),
                "E15: C[{tag}] for {:?}",
                String::from_utf8_lossy(cs)
            );
        }
    }
}

// ===========================================================================
// E16 — main() with a numeric prefix immediately followed by a non-digit.
// ===========================================================================
#[test]
fn err_e16_main_numeric_prefix() {
    let cases: [(&str, &str); 12] = [
        ("0x1f", "0\n"),
        ("3.9", "3\n"),
        ("12abc", "12\n"),
        ("1,2", "1\n"),
        ("5e3", "5\n"),
        ("-0x10", "0\n"),
        ("+7z", "7\n"),
        ("007x", "7\n"),
        ("1 2 3x", "3\n"),
        // No whitespace needed between tokens: `%d` accepts the '-' as a sign,
        // so "1-2" is two successful conversions, 1 then -2.
        ("1-2", "-2\n"),
        ("2147483647x", "2147483647\n"),
        ("9999999999999999999999x", "-1\n"),
    ];
    for (tag, so) in c_so_variants() {
        for (input, expected) in cases {
            assert_main_eq(&so, tag, input.as_bytes(), false);
            let cp = probe(&so, "main", &[], Some(input.as_bytes()));
            assert_eq!(
                cp.stdout,
                expected.as_bytes(),
                "E16: C[{tag}] for {input:?}"
            );
        }
        // Also byte-at-a-time, where the pushback path matters most.
        for (input, _) in cases {
            assert_main_eq(&so, tag, input.as_bytes(), true);
        }
    }
}

// ===========================================================================
// E18 — main() with a value one step past INT_MAX / INT_MIN (no rejection,
// just a narrowing conversion).
// ===========================================================================
#[test]
fn err_e18_main_int_range_overflow() {
    let cases: [(&str, &str); 10] = [
        ("2147483647", "2147483647\n"),
        ("2147483648", "-2147483648\n"),
        ("2147483649", "-2147483647\n"),
        ("-2147483648", "-2147483648\n"),
        ("-2147483649", "2147483647\n"),
        ("-2147483650", "2147483646\n"),
        ("4294967295", "-1\n"),
        ("4294967296", "0\n"),
        ("4294967297", "1\n"),
        ("+4294967295", "-1\n"),
    ];
    for (tag, so) in c_so_variants() {
        for (input, expected) in cases {
            assert_main_eq(&so, tag, input.as_bytes(), false);
            let cp = probe(&so, "main", &[], Some(input.as_bytes()));
            assert_eq!(cp.stdout, expected.as_bytes(), "E18: C[{tag}] for {input:?}");
        }
    }
}

// ===========================================================================
// E19 — main() past LONG_MAX / LONG_MIN: glibc saturates, then narrows.
// ===========================================================================
#[test]
fn err_e19_main_long_range_saturation() {
    let mut cases: Vec<(String, String)> = vec![
        ("9223372036854775807".into(), "-1\n".into()),
        ("9223372036854775808".into(), "-1\n".into()),
        ("9223372036854775809".into(), "-1\n".into()),
        ("18446744073709551615".into(), "-1\n".into()),
        ("18446744073709551616".into(), "-1\n".into()),
        ("-9223372036854775807".into(), "1\n".into()),
        ("-9223372036854775808".into(), "0\n".into()),
        ("-9223372036854775809".into(), "0\n".into()),
        ("-18446744073709551616".into(), "0\n".into()),
        ("99999999999999999999999999999".into(), "-1\n".into()),
        ("-99999999999999999999999999999".into(), "0\n".into()),
    ];
    // 400-digit runs, positive and negative, plus a huge leading-zero run in
    // front of an in-range value (which must NOT saturate).
    cases.push((format!("9{}", "9".repeat(399)), "-1\n".into()));
    cases.push((format!("-9{}", "9".repeat(399)), "0\n".into()));
    cases.push((format!("{}123", "0".repeat(500)), "123\n".into()));
    cases.push((format!("-{}123", "0".repeat(500)), "-123\n".into()));

    for (tag, so) in c_so_variants() {
        for (input, expected) in &cases {
            assert_main_eq(&so, tag, input.as_bytes(), false);
            let cp = probe(&so, "main", &[], Some(input.as_bytes()));
            assert_eq!(
                String::from_utf8_lossy(&cp.stdout),
                *expected,
                "E19: C[{tag}] for {:?}",
                &input[..input.len().min(24)]
            );
        }
    }
    for (input, _) in &cases {
        assert_driver_eq(input.as_bytes(), false);
    }
}

// ===========================================================================
// E20 / E21 — main()'s `i < 100` cap.
// ===========================================================================
#[test]
fn err_e20_main_more_than_100() {
    let mut rng = Rng::new(0xE020);
    for count in [99usize, 100, 101, 102, 250, 1000] {
        for _ in 0..8 {
            let vals: Vec<i32> = (0..count).map(|_| rng.next_i32()).collect();
            let toks: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            let input = toks.join(" ").into_bytes();
            let expected = if count >= 100 {
                format!("{}\n", vals[99])
            } else {
                format!("{}\n", vals[count - 1])
            };
            for (tag, so) in c_so_variants() {
                assert_main_eq(&so, tag, &input, false);
                let cp = probe(&so, "main", &[], Some(&input));
                assert_eq!(
                    String::from_utf8_lossy(&cp.stdout),
                    expected,
                    "E20: C[{tag}] count={count}"
                );
            }
            assert_driver_eq(&input, false);
        }
    }
}

// ===========================================================================
// E23 — the API has no enums; the only scalar is `int len`. Feed it the
// out-of-range extremes an out-of-range enum value would occupy.
// ===========================================================================
#[test]
fn err_e23_int_len_extremes() {
    let data = [0x1234_5678i32; 8];
    for (c, r) in pairs() {
        // `fma_array` is well defined for every non-positive len (loop guard),
        // so all of these can be compared directly in process. `call_fma` is
        // only safe to call in process for len == 0; the negative values are
        // UB (see E4) and are driven out of process there.
        for len in [i32::MIN, i32::MIN + 1, -2, -1, 0] {
            if len == 0 {
                let cv = unsafe { (c.call_fma)(data.as_ptr(), len) };
                let rv = unsafe { (r.call_fma)(data.as_ptr(), len) };
                assert_eq!(cv, 0, "E23: call_fma(_, 0)");
                assert_eq!(rv, cv, "E23: {} vs {} len={len}", c.name, r.name);
            }
            let mut out = vec![CANARY; 8];
            let mut out_r = vec![CANARY; 8];
            unsafe {
                (c.fma_array)(
                    out.as_mut_ptr(),
                    data.as_ptr(),
                    data.as_ptr(),
                    data.as_ptr(),
                    len,
                );
                (r.fma_array)(
                    out_r.as_mut_ptr(),
                    data.as_ptr(),
                    data.as_ptr(),
                    data.as_ptr(),
                    len,
                );
            }
            assert_eq!(out, out_r, "E23: fma_array len={len} {} vs {}", c.name, r.name);
            assert_eq!(out, vec![CANARY; 8], "E23: fma_array len={len} must not write");
        }
        // len == 1 -- the first value that does reach the body.
        let cv = unsafe { (c.call_fma)(data.as_ptr(), 1) };
        let rv = unsafe { (r.call_fma)(data.as_ptr(), 1) };
        assert_eq!(cv, data[0]);
        assert_eq!(rv, cv, "E23: len=1 {} vs {}", c.name, r.name);
    }
    // INT_MAX is the stack-overflow / walk-off-the-end case; covered out of
    // process by E6 and E11.
}

// ===========================================================================
// E24 — main() always returns 0, whatever the input.
// ===========================================================================
#[test]
fn err_e24_main_return_value() {
    let mut rng = Rng::new(0xE024);
    let mut cases: Vec<Vec<u8>> = vec![b"".to_vec(), b"abc".to_vec(), b"1 2 3".to_vec()];
    for _ in 0..60 {
        let n = rng.below(30);
        cases.push(
            (0..n)
                .map(|_| *rng.pick(b"0123456789+- \t\nabc"))
                .collect(),
        );
    }
    for (tag, so) in c_so_variants() {
        for cs in &cases {
            let cp = probe(&so, "main", &[], Some(cs));
            let rp = probe(&rust_so(), "main", &[], Some(cs));
            assert_eq!(cp.code, Some(0), "E24: C[{tag}] must return 0: {}", cp.describe());
            assert_eq!(
                (cp.code, cp.signal, cp.stdout.clone()),
                (rp.code, rp.signal, rp.stdout.clone()),
                "E24 mismatch C[{tag}] vs Rust for {:?}\n  C   : {}\n  Rust: {}",
                String::from_utf8_lossy(cs),
                cp.describe(),
                rp.describe()
            );
        }
    }
    for cs in &cases {
        assert_driver_eq(cs, false);
    }
}

/// SIGSEGV, without pulling in the `libc` crate.
fn libc_sigsegv() -> i32 {
    11
}
