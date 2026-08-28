//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 … E4f, B1 … B4), plus the generic
//! FFI-boundary boundaries every C API has: null pointers, zero / oversized
//! lengths, one-past-range values, and out-of-range "enum" ints.
//!
//! `parse_number`'s only error channel is its `cJSON_bool` return value, so each
//! test asserts the *exact* sentinel (`0` for rejection, `1` for acceptance) and
//! the exact side effects, on both implementations, rather than merely "both
//! failed somehow".

mod common;

use common::*;

/// `false` as produced by `#define false ((cJSON_bool)0)`.
const C_FALSE: i32 = 0;
/// `true` as produced by `#define true ((cJSON_bool)1)`.
const C_TRUE: i32 = 1;
/// `#define cJSON_Number (1 << 3)`
const CJSON_NUMBER: i32 = 8;

/* ============================================================= E1 ========= */

#[test]
fn e1_null_input_buffer_returns_false() {
    let mut rng = Rng::new(0xE1);
    // Same rejection regardless of what else is supplied.
    for text in ["", "0", "123", "-1.5e7", "abc", &"9".repeat(5000)] {
        let mut case = Case::from_str(text).buffer_null();
        case.length = rng.next_u64() as usize;
        case.offset = rng.next_u64() as usize;
        case.depth = rng.next_u64() as usize;
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(c.ret, C_FALSE, "C must return false for NULL buffer");
        assert_eq!(r.ret, C_FALSE, "Rust must return false for NULL buffer");
        assert_eq!(c, r, "E1 divergence for {text:?}");
        // `item` must be entirely untouched by both.
        assert_eq!(c.type_, case.item_type);
        assert_eq!(c.valueint, case.item_valueint);
        assert_eq!(c.valuedouble_bits, case.item_valuedouble_bits);
    }
    // ... and with a NULL content on top of the NULL buffer.
    let case = Case::from_str("123").buffer_null().content_null();
    let (c, r) = (observe_c(&case), observe_rust(&case));
    assert_eq!((c.ret, r.ret), (C_FALSE, C_FALSE));
    assert_eq!(c, r);
}

/* ============================================================= E2 ========= */

#[test]
fn e2_null_content_returns_false() {
    let mut rng = Rng::new(0xE2);
    for _ in 0..2000 {
        let mut case = Case::from_str("42").content_null();
        // Every combination of extreme length/offset still just returns false.
        case.length = *rng.pick(&[0usize, 1, 2, 1024, usize::MAX / 2, usize::MAX]);
        case.offset = *rng.pick(&[0usize, 1, 2, 1024, usize::MAX / 2, usize::MAX]);
        case.depth = rng.next_u64() as usize;
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(c.ret, C_FALSE, "C must reject NULL content");
        assert_eq!(r.ret, C_FALSE, "Rust must reject NULL content");
        assert_eq!(c, r, "E2 divergence");
        // struct fully preserved
        assert_eq!(c.buf_length, case.length);
        assert_eq!(c.buf_offset, case.offset);
        assert_eq!(c.buf_depth, case.depth);
        assert!(c.buf_content_unchanged);
        // item fully preserved
        assert_eq!(c.type_, case.item_type);
        assert_eq!(c.valueint, case.item_valueint);
        assert_eq!(c.valuedouble_bits, case.item_valuedouble_bits);
    }
}

/* ============================================================= E3 ========= */

#[test]
fn e3_allocation_failure_contract_matches_e4() {
    // `malloc` failure (lib.c:64) cannot be provoked deterministically across a
    // dlopen'd libc that both libraries share. Its *observable contract* is
    // "return false, mutate nothing" — identical to E4 — and both
    // implementations implement it with the same `NULL` check on the same
    // `malloc` (see SYMBOLS.md: both `.so`s import `malloc@GLIBC_2.2.5`).
    //
    // What IS testable here: the allocation size is `number_string_length + 1`,
    // so a zero-length scan still allocates 1 byte and must succeed; verify both
    // take that path identically.
    let c = observe_c(&Case::from_bytes(b"").length(0));
    let r = observe_rust(&Case::from_bytes(b"").length(0));
    assert_eq!(c.ret, C_FALSE);
    assert_eq!(r.ret, C_FALSE);
    assert_eq!(c, r);
}

/* ============================================================= M1 ========= */

/// The C never checks `item != NULL` (lib.c:92 stores straight through the
/// parameter), so a NULL `item` faults. That is UB in C, but it is still a real
/// input a caller can supply, and the *observable* consequence — which fatal
/// signal, at which address, with what on stderr — is comparable if we run each
/// call in its own child process.
///
/// This is the row `ERRORS.md` used to mark "inspection only". It is a genuine
/// differential test now, and it is the one that caught a real divergence:
/// written as the place expression `(*item).valuedouble = number`, the Rust
/// aborted with a `panicked at ...: null pointer dereference occurred`
/// (SIGABRT) under `-C debug-assertions`, where the C raises SIGSEGV. The fix
/// was `item_store!` (`addr_of_mut!` + `ptr::write`) in `src/lib.rs`.
#[test]
fn m1_item_null_produces_the_same_fatal_signal() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // ---- child role: perform the faulting call and never return normally ----
    if let Ok(target) = std::env::var("M1_CRASH_TARGET") {
        let f = match target.as_str() {
            "c" => c_parse_number(),
            "rust" => rust_parse_number(),
            other => panic!("bad M1_CRASH_TARGET {other:?}"),
        };
        let bytes: &[u8] = b"12345";
        let mut buf = ParseBuffer {
            content: bytes.as_ptr(),
            length: bytes.len(),
            offset: 0,
            depth: 0,
        };
        // `item` is NULL. The C stores to it unconditionally.
        let ret = unsafe { f(std::ptr::null_mut(), &mut buf) };
        // Reaching here means no fault happened at all.
        eprintln!("M1_NO_CRASH ret={ret}");
        std::process::exit(77);
    }

    // ---- parent role: run both children and compare how they died ----------
    let exe = std::env::current_exe().expect("current_exe");
    let mut seen = Vec::new();
    for target in ["c", "rust"] {
        let out = Command::new(&exe)
            .args([
                "--exact",
                "m1_item_null_produces_the_same_fatal_signal",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("M1_CRASH_TARGET", target)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        seen.push((
            target,
            out.status.signal(),
            out.status.code(),
            stderr.contains("panicked"),
            stderr.contains("M1_NO_CRASH"),
        ));
    }

    let (_, c_sig, c_code, c_panicked, c_nocrash) = seen[0];
    let (_, r_sig, r_code, r_panicked, r_nocrash) = seen[1];

    assert!(
        !c_nocrash && !r_nocrash,
        "expected both to fault on a NULL item, got {seen:?}"
    );
    assert_eq!(
        c_sig, r_sig,
        "C and Rust died from DIFFERENT signals on a NULL `item`: {seen:?}\n\
         (C uses signal {c_sig:?}, Rust {r_sig:?}; 11 = SIGSEGV, 6 = SIGABRT)"
    );
    assert_eq!(c_code, r_code, "different exit codes: {seen:?}");
    assert_eq!(
        c_panicked, r_panicked,
        "only one of them printed a Rust panic message: {seen:?}"
    );
    assert!(
        !r_panicked,
        "the Rust must fault silently like the C, not panic: {seen:?}"
    );
    assert_eq!(
        c_sig,
        Some(11),
        "expected SIGSEGV from the C's unchecked store, got {seen:?}"
    );
}

/* ============================================================= E4 ========= */

#[test]
fn e4_strtod_consumed_nothing_returns_false() {
    // The canonical, fully-scanned-but-unparsable inputs.
    for s in ["+", "-", ".", "e", "E", "+.", "-.", ".e1", "e5", "E-2", "-e", ".-"] {
        let case = Case::from_str(s);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(c.ret, C_FALSE, "C must reject {s:?}");
        assert_eq!(r.ret, C_FALSE, "Rust must reject {s:?}");
        assert_eq!(c, r, "E4 divergence for {s:?}");
        assert_eq!(c.buf_offset, 0, "offset must not advance for {s:?}");
        assert_eq!(c.type_, case.item_type, "item preserved for {s:?}");
        assert_eq!(c.valueint, case.item_valueint);
        assert_eq!(c.valuedouble_bits, case.item_valuedouble_bits);
    }
}

#[test]
fn e4a_zero_length() {
    for text in ["", "1", "123456789", "-1.5"] {
        let case = Case::from_str(text).length(0);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_FALSE, "C: length 0 with {text:?}");
        assert_eq!(r.ret, C_FALSE, "Rust: length 0 with {text:?}");
        assert_eq!(c, r);
        assert_eq!(c.buf_offset, 0);
        assert_eq!(c.buf_length, 0);
    }
}

#[test]
fn e4b_offset_equals_length() {
    for text in ["1", "42", "-1.5e3", "0000000000"] {
        let n = text.len();
        let case = Case::from_str(text).length(n).offset(n);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_FALSE, "C: offset==length with {text:?}");
        assert_eq!(r.ret, C_FALSE, "Rust: offset==length with {text:?}");
        assert_eq!(c, r);
        assert_eq!(c.buf_offset, n, "offset must not move");
    }
}

#[test]
fn e4c_offset_past_length() {
    let mut rng = Rng::new(0xE4C);
    for text in ["1", "42", "-1.5e3"] {
        let n = text.len();
        for extra in 1..=64usize {
            let case = Case::from_str(text).length(n).offset(n + extra);
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c.ret, C_FALSE, "C: offset past end");
            assert_eq!(r.ret, C_FALSE, "Rust: offset past end");
            assert_eq!(c, r);
            assert_eq!(c.buf_offset, n + extra);
        }
    }
    // Huge (but non-wrapping) offsets.
    for _ in 0..500 {
        let off = (rng.next_u64() >> 1) as usize | (1usize << 62);
        let case = Case::from_str("123").length(3).offset(off);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!((c.ret, r.ret), (C_FALSE, C_FALSE));
        assert_eq!(c, r);
    }
}

#[test]
fn e4d_offset_size_max_wraps_in_can_access_at_index() {
    // `(offset + index) < length` wraps in C when offset == SIZE_MAX.
    for length in [0usize, 1, 3, 1024, usize::MAX - 1, usize::MAX] {
        for offset in [usize::MAX, usize::MAX - 1, usize::MAX - 2] {
            let case = Case::from_str("123").length(length).offset(offset);
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c, r, "E4d divergence length={length} offset={offset}");
            // With offset == SIZE_MAX and length <= SIZE_MAX, `offset + 0` is
            // never < length, so the scan is empty and strtod("") fails.
            if offset == usize::MAX {
                assert_eq!(c.ret, C_FALSE);
                assert_eq!(r.ret, C_FALSE);
            }
        }
    }
}

#[test]
fn e4e_first_byte_hits_default_arm() {
    // EVERY byte value outside `[0-9+\-eE.]` must be rejected by both, at
    // several buffer lengths, with `offset` untouched.
    for b in 0u16..256 {
        let b = b as u8;
        if ACCEPTED.contains(&b) {
            continue;
        }
        for len in 1usize..=6 {
            let mut bytes = vec![b];
            bytes.extend(std::iter::repeat(b'1').take(len - 1));
            let case = Case::from_bytes(&bytes);
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c.ret, C_FALSE, "C must reject leading byte {b:#04x}");
            assert_eq!(r.ret, C_FALSE, "Rust must reject leading byte {b:#04x}");
            assert_eq!(c, r, "E4e divergence for byte {b:#04x} len {len}");
            assert_eq!(c.buf_offset, 0);
        }
    }
    // Multi-byte / non-ASCII sequences too.
    for s in [
        "abc", " 1", "\t1", "\n1", "null", "true", "NaN", "nan", "inf",
        "Infinity", "\u{80}1", "\u{7f}", "\u{ff}9", "x10", "$1", "'1",
    ] {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_FALSE, "C must reject {s:?}");
        assert_eq!(r.ret, C_FALSE, "Rust must reject {s:?}");
        assert_eq!(c, r, "E4e divergence for {s:?}");
    }
    // Counterpart: a `default:` hit *after* a valid prefix is NOT a rejection —
    // the scan just stops there. `"0x10"` therefore parses as `0` (the C never
    // reaches strtod's hex path because `x` is filtered out first), and the
    // offset advances by exactly 1. Both must agree.
    for (s, want_off) in [("0x10", 1usize), ("1abc", 1), ("12 34", 2), ("0X1", 1)] {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_TRUE, "C must accept the prefix of {s:?}");
        assert_eq!(r.ret, C_TRUE, "Rust must accept the prefix of {s:?}");
        assert_eq!(c, r, "E4e divergence for {s:?}");
        assert_eq!(c.buf_offset, want_off, "{s:?}");
    }
    // Embedded NUL as the very first byte (the C cannot rely on NUL at all).
    let case = Case::from_bytes(b"\x001234").length(5);
    let (c, r) = (observe_c(&case), observe_rust(&case));
    assert_eq!((c.ret, r.ret), (C_FALSE, C_FALSE));
    assert_eq!(c, r);
}

#[test]
fn e4f_accepted_but_unparsable_exhaustive() {
    // Exhaustive sweep over every string of length 1..=3 drawn from the accepted
    // alphabet: 15 + 225 + 3375 = 3615 cases. Each is either accepted by strtod
    // or rejected; both implementations must agree on which, on the returned
    // sentinel, and on how far `offset` moved.
    let alpha = ACCEPTED;
    let mut checked_reject = 0usize;
    let mut checked_accept = 0usize;
    for a in alpha {
        let case = Case::from_bytes(&[*a]);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c, r, "E4f divergence for {:?}", *a as char);
        if c.ret == C_FALSE {
            checked_reject += 1;
            assert_eq!(c.buf_offset, 0);
        } else {
            checked_accept += 1;
            assert_eq!(c.type_, CJSON_NUMBER);
        }
        for b in alpha {
            let case = Case::from_bytes(&[*a, *b]);
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c, r, "E4f divergence for {:?}", [*a as char, *b as char]);
            if c.ret == C_FALSE {
                checked_reject += 1;
                assert_eq!(c.buf_offset, 0);
            } else {
                checked_accept += 1;
            }
            for d in alpha {
                let case = Case::from_bytes(&[*a, *b, *d]);
                let (c, r) = (observe_c(&case), observe_rust(&case));
                assert_eq!(
                    c, r,
                    "E4f divergence for {:?}",
                    [*a as char, *b as char, *d as char]
                );
                if c.ret == C_FALSE {
                    checked_reject += 1;
                    assert_eq!(c.buf_offset, 0);
                } else {
                    checked_accept += 1;
                }
            }
        }
    }
    assert_eq!(checked_reject + checked_accept, 15 + 225 + 3375);
    assert!(checked_reject > 0 && checked_accept > 0);
}

#[test]
fn e4f_length_four_and_five_exhaustive_sampled() {
    // Full 4-char sweep (50 625) plus a deterministic sample of 5-char strings.
    let alpha = ACCEPTED;
    let n = alpha.len();
    for i in 0..(n * n * n * n) {
        let bytes = [
            alpha[i % n],
            alpha[(i / n) % n],
            alpha[(i / (n * n)) % n],
            alpha[(i / (n * n * n)) % n],
        ];
        let case = Case::from_bytes(&bytes);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(
            c, r,
            "E4f divergence for {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }
    let mut rng = Rng::new(0x4F5);
    for _ in 0..60_000 {
        let len = rng.range(5, 9) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| *rng.pick(alpha)).collect();
        let case = Case::from_bytes(&bytes);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(
            c, r,
            "E4f divergence for {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

/* ============================================================= B1 ========= */

#[test]
fn b1_scan_is_bounded_by_length_only() {
    // Bytes past `length` must never influence the result.
    let mut rng = Rng::new(0xB1);
    for _ in 0..4000 {
        let n = rng.range(1, 20) as usize;
        let head: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let tail: Vec<u8> = (0..rng.range(1, 20) as usize)
            .map(|_| *rng.pick(ACCEPTED))
            .collect();

        let bounded = Case::from_bytes(&head).length(n).with_guard(&tail);
        let alone = Case::from_bytes(&head).length(n);

        let (cb, rb) = (observe_c(&bounded), observe_rust(&bounded));
        let (ca, ra) = (observe_c(&alone), observe_rust(&alone));
        assert_eq!(cb, rb, "B1 divergence (guarded)");
        assert_eq!(ca, ra, "B1 divergence (bare)");
        assert_eq!(cb, ca, "bytes past `length` changed the C result");
        assert_eq!(rb, ra, "bytes past `length` changed the Rust result");
    }
}

/* ======================================================== B2 / B3 / B4 ==== */

#[test]
fn b2_saturates_to_int_max() {
    let mut rng = Rng::new(0xB2);
    let fixed: Vec<String> = vec![
        "2147483647".into(),
        "2147483647.0000000001".into(),
        "2147483648".into(),
        "1e309".into(),
        "1e99999".into(),
        "9".repeat(310),
    ];
    for s in &fixed {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_TRUE, "C must accept {s:?}");
        assert_eq!(r.ret, C_TRUE, "Rust must accept {s:?}");
        assert_eq!(c, r, "B2 divergence for {s:?}");
        assert_eq!(c.valueint, i32::MAX, "must saturate: {s:?}");
        assert_eq!(c.type_, CJSON_NUMBER);
    }
    for _ in 0..3000 {
        let s = format!("{}", 2147483647u64 + rng.below(1u64 << 45));
        let case = Case::from_str(&s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c, r, "B2 divergence for {s:?}");
        assert_eq!(c.valueint, i32::MAX);
    }
}

#[test]
fn b3_saturates_to_int_min() {
    let mut rng = Rng::new(0xB3);
    let fixed: Vec<String> = vec![
        "-2147483648".into(),
        "-2147483648.0000000001".into(),
        "-2147483649".into(),
        "-1e309".into(),
        "-1e99999".into(),
        format!("-{}", "9".repeat(310)),
    ];
    for s in &fixed {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_TRUE, "C must accept {s:?}");
        assert_eq!(r.ret, C_TRUE, "Rust must accept {s:?}");
        assert_eq!(c, r, "B3 divergence for {s:?}");
        assert_eq!(c.valueint, i32::MIN, "must saturate: {s:?}");
    }
    for _ in 0..3000 {
        let s = format!("-{}", 2147483648u64 + rng.below(1u64 << 45));
        let case = Case::from_str(&s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c, r, "B3 divergence for {s:?}");
        assert_eq!(c.valueint, i32::MIN);
    }
}

#[test]
fn b4_exact_boundary_one_step_either_side() {
    // The documented range boundaries and one step past them, in both
    // directions, expressed exactly.
    // NOTE on the two ".999999999" rows: near 2^31 the binary64 spacing is
    // 2^31 * 2^-52 ~= 4.8e-7, so `2147483646.999999999` is *not* representable
    // and rounds up to exactly 2147483647.0 — which then trips the
    // `number >= INT_MAX` branch and saturates. Same, mirrored, for the
    // negative. That is the C's behaviour and the expectation follows it.
    let cases: [(&str, i32); 16] = [
        ("2147483646", 2147483646),
        ("2147483646.5", 2147483646),
        ("2147483646.999999999", i32::MAX),
        ("2147483647", i32::MAX),
        ("2147483647.000000001", i32::MAX),
        ("2147483648", i32::MAX),
        ("-2147483647", -2147483647),
        ("-2147483647.5", -2147483647),
        ("-2147483647.999999999", i32::MIN),
        ("-2147483648", i32::MIN),
        ("-2147483648.000000001", i32::MIN),
        ("-2147483649", i32::MIN),
        ("0", 0),
        ("-0", 0),
        ("1", 1),
        ("-1", -1),
    ];
    for (s, want) in cases {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_TRUE, "{s:?}");
        assert_eq!(r.ret, C_TRUE, "{s:?}");
        assert_eq!(c, r, "B4 divergence for {s:?}");
        assert_eq!(c.valueint, want, "{s:?}");
    }
    // Truncation toward zero across the whole representable range.
    let mut rng = Rng::new(0xB4);
    for _ in 0..20_000 {
        let mag = rng.below(2_147_483_647);
        let frac = rng.digits_between(1, 20);
        for s in [format!("{mag}.{frac}"), format!("-{mag}.{frac}")] {
            let case = Case::from_str(&s);
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c, r, "B4 divergence for {s:?}");
        }
    }
}

/* ================================================ generic FFI boundaries == */

#[test]
fn generic_null_pointers() {
    // Only two pointers cross the boundary; `item == NULL` is UB in the C (see
    // ERRORS.md M1) so it is not exercised. Both NULL-able pointers, in every
    // combination that is defined:
    let variants = [
        Case::from_str("123").buffer_null(),
        Case::from_str("123").content_null(),
        Case::from_str("123").buffer_null().content_null(),
    ];
    for case in variants {
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_FALSE);
        assert_eq!(r.ret, C_FALSE);
        assert_eq!(c, r);
    }
}

#[test]
fn generic_zero_and_oversized_lengths() {
    // `length` from 0 up to the exact content size, then one past the content
    // (still inside the padded allocation), for a variety of texts.
    for text in ["", "0", "1.5", "-1e3", "+.5e-2", "12345678901234567890"] {
        let n = text.len();
        for length in 0..=(n + 1) {
            for offset in 0..=(n + 1) {
                let case = Case::from_str(text).length(length).offset(offset);
                let (c, r) = (observe_c(&case), observe_rust(&case));
                assert_eq!(
                    c, r,
                    "divergence for {text:?} length={length} offset={offset}"
                );
                assert!(c.ret == C_FALSE || c.ret == C_TRUE);
            }
        }
    }
    // Oversized `length` values (far beyond the allocation) are only safe when
    // the scan cannot start, i.e. offset >= length is impossible — so use an
    // offset that makes `can_access_at_index` fail immediately.
    for length in [usize::MAX, usize::MAX / 2, 1usize << 40] {
        let case = Case::from_str("123").length(length).offset(usize::MAX);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_FALSE);
        assert_eq!(r.ret, C_FALSE);
        assert_eq!(c, r);
    }
}

#[test]
fn generic_out_of_range_enum_ints_in_item_type() {
    // `cJSON.type` is the library's enum-like field (`cJSON_Number == 1 << 3`).
    // C enums/ints accept any value, so feed values with no valid variant —
    // including negatives, INT_MIN/INT_MAX and every single-bit pattern — and
    // require identical handling. On success the field must be overwritten with
    // exactly `cJSON_Number`; on failure it must be preserved bit-for-bit.
    let mut types: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        2,
        4,
        7,
        8,
        9,
        16,
        255,
        256,
        0x1000_0000,
        i32::MAX - 1,
        i32::MAX,
        -559038737,
        0x7fff_fffe,
    ];
    for bit in 0..32 {
        types.push(1i32.wrapping_shl(bit));
        types.push(!(1i32.wrapping_shl(bit)));
    }

    for t in types {
        for (text, expect_ok) in [
            ("123", true),
            ("-1.5e2", true),
            ("", false),
            ("+", false),
            ("zzz", false),
        ] {
            let mut case = Case::from_str(text);
            case.item_type = t;
            let (c, r) = (observe_c(&case), observe_rust(&case));
            assert_eq!(c, r, "enum divergence: type={t} text={text:?}");
            if expect_ok {
                assert_eq!(c.ret, C_TRUE, "{text:?}");
                assert_eq!(c.type_, CJSON_NUMBER, "type must be replaced (was {t})");
            } else {
                assert_eq!(c.ret, C_FALSE, "{text:?}");
                assert_eq!(c.type_, t, "type must be preserved on failure");
            }
        }
    }
}

#[test]
fn generic_out_of_range_valueint_and_valuedouble_preimages() {
    // Every interesting bit pattern pre-loaded into the out-params, incl.
    // signalling NaN, quiet NaN, infinities, and negative zero.
    let doubles: [u64; 12] = [
        0x0000_0000_0000_0000, // +0
        0x8000_0000_0000_0000, // -0
        0x7ff0_0000_0000_0000, // +inf
        0xfff0_0000_0000_0000, // -inf
        0x7ff8_0000_0000_0000, // quiet NaN
        0x7ff0_0000_0000_0001, // signalling NaN
        0xffff_ffff_ffff_ffff,
        0x0000_0000_0000_0001, // smallest denormal
        0x000f_ffff_ffff_ffff,
        0x7fef_ffff_ffff_ffff, // DBL_MAX
        0x4000_0000_0000_0000, // 2.0
        0xdead_beef_cafe_babe,
    ];
    let ints = [i32::MIN, -1, 0, 1, i32::MAX];
    for d in doubles {
        for vi in ints {
            for text in ["7", "", "e", "-0.0", "1e999"] {
                let mut case = Case::from_str(text);
                case.item_valuedouble_bits = d;
                case.item_valueint = vi;
                let (c, r) = (observe_c(&case), observe_rust(&case));
                assert_eq!(
                    c, r,
                    "divergence with preloaded double {d:#018x} int {vi} text {text:?}"
                );
                if c.ret == C_FALSE {
                    assert_eq!(c.valuedouble_bits, d, "must be preserved");
                    assert_eq!(c.valueint, vi, "must be preserved");
                }
            }
        }
    }
}

#[test]
fn generic_one_past_documented_ranges() {
    // Exponent range boundaries of IEEE-754 binary64, one step either side.
    let probes = [
        "1e307", "1e308", "1e309", "1.7976931348623157e308",
        "1.7976931348623158e308", "1.7976931348623159e308",
        "-1.7976931348623157e308", "-1.7976931348623159e308",
        "2.2250738585072014e-308", "2.2250738585072013e-308",
        "4.9406564584124654e-324", "2.4703282292062328e-324",
        "2.4703282292062327e-324", "1e-323", "1e-324", "1e-325",
        "9007199254740992", "9007199254740993", "9007199254740994",
        "-9007199254740993", "18446744073709551615", "18446744073709551616",
        "340282366920938463463374607431768211456",
    ];
    for s in probes {
        let case = Case::from_str(s);
        let (c, r) = (observe_c(&case), observe_rust(&case));
        assert_eq!(c.ret, C_TRUE, "{s:?}");
        assert_eq!(r.ret, C_TRUE, "{s:?}");
        assert_eq!(c, r, "divergence for {s:?}");
    }
}

#[test]
fn generic_repeated_calls_do_not_leak_state() {
    // Same case run many times alternating implementations: no accumulated
    // state, no drift.
    let case = Case::from_str("3.14159e2");
    let first_c = observe_c(&case);
    let first_r = observe_rust(&case);
    assert_eq!(first_c, first_r);
    for _ in 0..5000 {
        assert_eq!(observe_c(&case), first_c);
        assert_eq!(observe_rust(&case), first_r);
    }
}
