//! Phase D — cross-cutting matrices that no single `CONFIGS.md`/`ERRORS.md` row
//! owns on its own.
//!
//! These close the two gaps that per-row tests structurally cannot:
//!
//! 1. The `(content shape) x (offset) x (length)` cross-product, exhaustively at
//!    small sizes — this is where `can_access_at_index` / `buffer_at_offset` /
//!    the `after_end - number_c_string` offset update interact.
//! 2. Inputs that would behave differently if the accepted-byte set diverged by
//!    even one character — in particular the forms libc `strtod` recognises but
//!    the C scanner filters out first (hex floats, `nan(...)`, `infinity`,
//!    leading whitespace, digit separators).

mod common;

use common::*;

/* ------------------------------------------------------------------------- */

/// Exhaustive: every string over the accepted alphabet of length 0..=3, crossed
/// with every `offset` and every `length` in `0..=len+1`.
#[test]
fn exhaustive_shape_offset_length_matrix() {
    let alpha = ACCEPTED;
    let n = alpha.len();

    let mut strings: Vec<Vec<u8>> = vec![Vec::new()];
    for a in alpha {
        strings.push(vec![*a]);
    }
    for i in 0..(n * n) {
        strings.push(vec![alpha[i % n], alpha[i / n]]);
    }
    for i in 0..(n * n * n) {
        strings.push(vec![alpha[i % n], alpha[(i / n) % n], alpha[i / (n * n)]]);
    }
    assert_eq!(strings.len(), 1 + 15 + 225 + 3375);

    let mut compared = 0usize;
    for bytes in &strings {
        let len = bytes.len();
        for length in 0..=(len + 1) {
            for offset in 0..=(len + 1) {
                let case = Case::from_bytes(bytes).length(length).offset(offset);
                let c = observe_c(&case);
                let r = observe_rust(&case);
                assert_eq!(
                    c,
                    r,
                    "divergence: bytes={:?} length={length} offset={offset}",
                    String::from_utf8_lossy(bytes)
                );
                compared += 1;
            }
        }
    }
    assert!(compared > 50_000, "only {compared} comparisons");
}

/// Same idea one size up, but with the alphabet reduced to a representative
/// class per switch arm, so the 5- and 6-character cross-product stays cheap
/// while still covering every arm and every offset/length.
#[test]
fn exhaustive_representative_alphabet_length_six() {
    // one digit, the two signs, both exponent letters, the decimal point, and a
    // byte that hits `default:`
    let alpha: &[u8] = b"1.+-eEz";
    let n = alpha.len();
    let total = n.pow(5);
    for i in 0..total {
        let mut bytes = Vec::with_capacity(5);
        let mut k = i;
        for _ in 0..5 {
            bytes.push(alpha[k % n]);
            k /= n;
        }
        let case = Case::from_bytes(&bytes);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(
            c,
            r,
            "divergence: bytes={:?}",
            String::from_utf8_lossy(&bytes)
        );
    }

    // 6-wide, with a rotating offset so the offset path is covered too.
    let total6 = n.pow(6);
    for i in 0..total6 {
        let mut bytes = Vec::with_capacity(6);
        let mut k = i;
        for _ in 0..6 {
            bytes.push(alpha[k % n]);
            k /= n;
        }
        let offset = i % 7;
        let case = Case::from_bytes(&bytes).length(6).offset(offset);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(
            c,
            r,
            "divergence: bytes={:?} offset={offset}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

/// Forms that libc `strtod` accepts but the C scanner must filter out *before*
/// strtod ever sees them. If the accepted-byte set diverged by a single
/// character (`x`, `X`, `p`, `P`, `n`, `a`, `i`, `f`, `(`, `)`, space, `_`),
/// these are the inputs where it becomes observable.
#[test]
fn strtod_special_forms_are_filtered_by_the_scanner() {
    let probes: &[&str] = &[
        // hex floats: the C stops at 'x'/'X', so these parse as plain 0
        "0x10", "0X10", "0x1p3", "0X1P3", "0x1.8p1", "0xAB", "0x", "0X",
        "-0x10", "+0X1f", "0x7fffffff", "0xfffffffffffff",
        // infinity / nan spellings
        "inf", "INF", "Inf", "infinity", "INFINITY", "-inf", "+inf",
        "nan", "NAN", "NaN", "nan(0)", "nan(123)", "-nan", "nan()",
        // leading whitespace (strtod skips it, the C scanner does not)
        " 1", "  1", "\t1", "\n1", "\r1", "\u{b}1", "\u{c}1", " -1", " +1", " .5",
        // digit separators / other punctuation strtod may or may not like
        "1_000", "1'000", "1,000", "1 000", "1;5",
        // exponent letters that are NOT in the accepted set
        "1d5", "1D5", "1p5", "1P5", "1f5", "1F5",
        // 'e'/'E' are accepted, so these must go all the way through strtod
        "1e5", "1E5", "1e+5", "1E-5", "1.5e-3",
        // combinations right at the filter boundary
        "0x1e5", "1e5x", "1ex5", ".x", "x.", "+x", "-x",
    ];
    for s in probes {
        let case = Case::from_str(s);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(c, r, "divergence for {s:?}: C={c:?} Rust={r:?}");
    }

    // Spot-check a few against the behaviour the C source dictates, so the test
    // is not merely self-consistent.
    let o = diff_str("0x10");
    assert_eq!(o.ret, 1, "'0' parses, scan stops at 'x'");
    assert_eq!(o.valueint, 0);
    assert_eq!(o.buf_offset, 1, "only the '0' is consumed, NOT the hex form");

    let o = diff_str(" 1");
    assert_eq!(o.ret, 0, "leading space hits `default:` with an empty scan");
    assert_eq!(o.buf_offset, 0);

    let o = diff_str("nan");
    assert_eq!(o.ret, 0, "'n' hits `default:`");

    let o = diff_str("1e5");
    assert_eq!(o.ret, 1);
    assert_eq!(o.valueint, 100_000);
    assert_eq!(o.buf_offset, 3);
}

/// Every accepted byte, deleted from an otherwise-valid number: if the Rust
/// scanner were missing any single arm, the offset/value would drift here.
#[test]
fn each_accepted_byte_is_load_bearing() {
    let templates: &[&str] = &[
        "1.5e+3", "1.5E-3", "-2.5e10", "+0.125E+2", "9.87654321e-12",
        "0.0e0", "-.5e-1", "+.25E+1", "12345.6789e+300", "-1.7e-308",
    ];
    for t in templates {
        // the whole thing
        let o = diff_str(t);
        assert_eq!(o.ret, 1, "{t:?}");
        assert_eq!(o.buf_offset, t.len(), "{t:?} must consume everything");
        // every prefix
        for k in 0..=t.len() {
            diff_str(&t[..k]);
        }
        // every single-character deletion
        for k in 0..t.len() {
            let mut s = t.to_string();
            s.remove(k);
            diff_str(&s);
        }
        // every single-character duplication
        for k in 0..t.len() {
            let mut s = t.to_string();
            s.insert(k, t.as_bytes()[k] as char);
            diff_str(&s);
        }
        // every position replaced by every accepted byte
        for k in 0..t.len() {
            for a in ACCEPTED {
                let mut b = t.as_bytes().to_vec();
                b[k] = *a;
                diff(&Case::from_bytes(&b));
            }
        }
    }
}

/// Wide randomized sweep with all axes varied simultaneously.
#[test]
fn wide_fuzz_all_axes() {
    let mut rng = Rng::new(0xD00D);
    for _ in 0..300_000 {
        let n = rng.below(64) as usize;
        let mut bytes = Vec::with_capacity(n);
        for _ in 0..n {
            match rng.below(12) {
                0 => bytes.push(rng.next_u64() as u8),
                1 => bytes.push(*rng.pick(b"xXpPnaifNAIF_,' \t")),
                _ => bytes.push(*rng.pick(ACCEPTED)),
            }
        }
        // length never exceeds the logical content, so no comparison ever looks
        // at bytes outside `bytes`.
        let length = if rng.below(8) == 0 {
            rng.below((n + 1) as u64) as usize
        } else {
            n
        };
        let offset = match rng.below(10) {
            0 => rng.below((n + 2) as u64) as usize,
            1 => length,
            2 => length + rng.range(1, 8) as usize,
            _ => 0,
        };
        let mut case = Case::from_bytes(&bytes).length(length).offset(offset);
        case.depth = rng.next_u64() as usize;
        case.item_type = rng.next_u64() as i32;
        case.item_valueint = rng.next_u64() as i32;
        case.item_valuedouble_bits = rng.next_u64();
        diff(&case);
    }
}

/// Long inputs combined with partial `strtod` consumption and non-zero offsets.
#[test]
fn long_inputs_with_partial_consumption() {
    let mut rng = Rng::new(0xD11D);
    for _ in 0..2000 {
        let head = rng.range(1, 200) as usize;
        let mut s: String = (0..head).map(|_| rng.digit() as char).collect();
        // a tail that is scanned but not consumed by strtod
        let tail = rng.choose(&["e", "e+", "e-", ".", "..", "+", "-", "E", "E-", "1e", ""]);
        s.push_str(tail);
        let extra = rng.range(0, 200) as usize;
        for _ in 0..extra {
            s.push(*rng.pick(ACCEPTED) as char);
        }
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        for offset in [0usize, 1, len / 2, len.saturating_sub(1)] {
            let case = Case::from_bytes(&bytes).length(len).offset(offset);
            let c = observe_c(&case);
            let r = observe_rust(&case);
            assert_eq!(c, r, "divergence for {s:?} offset={offset}");
        }
    }
}
