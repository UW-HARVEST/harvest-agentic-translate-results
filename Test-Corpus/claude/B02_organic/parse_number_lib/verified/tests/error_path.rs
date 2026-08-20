//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test constructs the exact invalid
//! input/condition, calls BOTH `.so`s and asserts they return the same
//! `cJSON_bool` sentinel *and* leave the out-parameters in the same state.

mod common;

use common::*;

/// The set of bytes the C `switch` accepts (everything else hits `default:`).
const CHARSET: &[u8] = b"0123456789+-.eE";

// ---------------------------------------------------------------------------
// E1 — input_buffer == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_e1_null_input_buffer() {
    let out = assert_same(&Case::new("123").buffer_null());
    assert_eq!(out.ret, C_FALSE, "C returns false for NULL input_buffer");
    // *item must be untouched.
    assert_eq!(out.item_type, POISON_TYPE);
    assert_eq!(out.item_valueint, POISON_VALUEINT);
    assert_eq!(out.item_double_bits, POISON_DOUBLE_BITS);
}

// ---------------------------------------------------------------------------
// E2 — input_buffer == NULL && item == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_e2_both_null() {
    let out = assert_same(&Case::new("123").buffer_null().item_null());
    assert_eq!(out.ret, C_FALSE);
}

// ---------------------------------------------------------------------------
// E3 — input_buffer->content == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_e3_null_content() {
    for &(len, off) in &[
        (0usize, 0usize),
        (0, 7),
        (10, 0),
        (10, 5),
        (10, 10),
        (usize::MAX, 0),
        (usize::MAX, usize::MAX),
        (1, usize::MAX),
    ] {
        let case = Case::new("").content_null().length(len).offset(off);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{}", case.label());
        assert_eq!(out.item_type, POISON_TYPE, "{}", case.label());
        assert_eq!(out.item_valueint, POISON_VALUEINT);
        assert_eq!(out.item_double_bits, POISON_DOUBLE_BITS);
        assert_eq!(out.buf_offset, off, "offset must not move");
        assert_eq!(out.buf_length, len);
        assert_eq!(out.buf_depth, POISON_DEPTH);
    }
}

// ---------------------------------------------------------------------------
// E4 — content == NULL && item == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_e4_null_content_null_item() {
    let out = assert_same(&Case::new("").content_null().item_null().length(4));
    assert_eq!(out.ret, C_FALSE);
}

// ---------------------------------------------------------------------------
// E5 — malloc failure (documented unreachable). Verified indirectly: the
// temporary allocation size `number_string_length + 1` is always bounded by the
// caller's own buffer, so it can never be an attacker-chosen huge allocation.
// ---------------------------------------------------------------------------
#[test]
fn err_e5_temp_allocation_is_caller_bounded() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    for _ in 0..2000 {
        let n = rng.range(1, 64);
        let mut content: Vec<u8> = (0..n).map(|_| *rng.pick(CHARSET)).collect();
        content.push(b','); // guaranteed terminator inside the real allocation
        let len = content.len();
        let off = rng.below(len);
        let case = Case::new(&content).length(len).offset(off);
        let out = assert_same(&case);
        // Consumed bytes (hence the allocation) never exceed length - offset.
        assert!(
            out.buf_offset >= off && out.buf_offset <= len,
            "{} -> {out:?}",
            case.label()
        );
    }
}

// ---------------------------------------------------------------------------
// E6 — first byte at `offset` is not in the accepted charset
// ---------------------------------------------------------------------------
#[test]
fn err_e6_first_byte_not_numeric() {
    for b in 0u16..=255 {
        let b = b as u8;
        if CHARSET.contains(&b) {
            continue;
        }
        let content = vec![b, b'1', b'2', b'3'];
        let case = Case::new(&content);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "byte {b:#04x}: {}", case.label());
        assert_eq!(out.item_type, POISON_TYPE, "byte {b:#04x}");
        assert_eq!(out.item_valueint, POISON_VALUEINT);
        assert_eq!(out.item_double_bits, POISON_DOUBLE_BITS);
        assert_eq!(out.buf_offset, 0);
        assert_eq!(out.buf_depth, POISON_DEPTH);
    }
    // Multi-byte JSON literals / whitespace that reach the same `default:` label.
    for s in [
        " 1", "\t1", "\n1", "\r1", "null", "true", "false", "[1]", "{\"a\":1}", "NaN",
        "Infinity", "-", // (handled by E9, kept here for the literal set)
        "x1", "#1", "\u{00e9}1",
    ] {
        let out = assert_same_str(s);
        if s == "-" {
            assert_eq!(out.ret, C_FALSE);
        } else if !s.starts_with(|c: char| c.is_ascii_digit()) {
            assert_eq!(out.ret, C_FALSE, "{s:?} must be rejected");
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — length == 0
// ---------------------------------------------------------------------------
#[test]
fn err_e7_zero_length() {
    for content in ["", "1", "12345", "-1.5e3"] {
        let case = Case::new(content).length(0);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{}", case.label());
        assert_eq!(out.buf_offset, 0);
        assert_eq!(out.item_type, POISON_TYPE);
    }
    // Also with item == NULL: the failure happens before *item is touched.
    let out = assert_same(&Case::new("12345").length(0).item_null());
    assert_eq!(out.ret, C_FALSE);
}

// ---------------------------------------------------------------------------
// E8 — offset >= length
// ---------------------------------------------------------------------------
#[test]
fn err_e8_offset_at_or_past_end() {
    let content = b"12345".to_vec();
    for off in [5usize, 6, 7, 100, usize::MAX / 2, usize::MAX - 1, usize::MAX] {
        let case = Case::new(&content).length(5).offset(off);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{}", case.label());
        assert_eq!(out.buf_offset, off, "offset must not move");
        assert_eq!(out.item_type, POISON_TYPE);
        assert_eq!(out.buf_depth, POISON_DEPTH);
    }
    // offset == length for many random lengths.
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..500 {
        let n = rng.range(1, 40);
        let content = rng.digits(n);
        let case = Case::new(&content).length(n).offset(n);
        assert_eq!(assert_same(&case).ret, C_FALSE, "{}", case.label());
    }
}

// ---------------------------------------------------------------------------
// E9 — charset-only, but strtod consumes nothing
// ---------------------------------------------------------------------------
#[test]
fn err_e9_charset_but_unparsable() {
    const UNPARSABLE: &[&str] = &[
        "+", "-", ".", "e", "E", "+.", "-.", ".+", ".-", "e5", "E-3", "e+5", "E+", "e-", "eE",
        "Ee", "++1", "--1", "+-1", "-+1", ".e1", ".E1", "..1", "...", "+e", "-E", "+E5", "-e5",
        "++", "--", "+-", "-+", ".+1", ".-1", "e.", "E.", "e.5", "..", ".ee", "+.e", "-.E",
    ];
    for s in UNPARSABLE {
        let case = Case::new(s);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{s:?} must be a parse_error");
        assert_eq!(out.item_type, POISON_TYPE, "{s:?}");
        assert_eq!(out.item_valueint, POISON_VALUEINT, "{s:?}");
        assert_eq!(out.item_double_bits, POISON_DOUBLE_BITS, "{s:?}");
        assert_eq!(out.buf_offset, 0, "{s:?}");
        // item == NULL is safe on this path too (nothing is written).
        assert_eq!(assert_same(&Case::new(s).item_null()).ret, C_FALSE, "{s:?}");
    }
    // Randomized: charset strings that begin with a non-digit, non-'.' byte are
    // very often unparsable — compare whatever the C decides.
    let mut rng = Rng::new(SEED ^ 0x9);
    const LEADERS: &[u8] = b"+-eE.";
    for _ in 0..5000 {
        let n = rng.range(1, 6);
        let mut s = vec![*rng.pick(LEADERS)];
        for _ in 0..n {
            s.push(*rng.pick(LEADERS));
        }
        assert_same(&Case::new(&s));
    }
}

// ---------------------------------------------------------------------------
// E10 — `length` truncates the token down to something unparsable
// ---------------------------------------------------------------------------
#[test]
fn err_e10_truncated_to_unparsable() {
    for &(content, len) in &[
        (&b"-12"[..], 1usize),  // sees "-"
        (&b"+12"[..], 1),       // sees "+"
        (&b".5"[..], 1),        // sees "."
        (&b"e5"[..], 1),        // sees "e"
        (&b"-.5"[..], 2),       // sees "-."
        (&b"--5"[..], 2),       // sees "--"
        (&b"1"[..], 0),         // sees ""
    ] {
        let case = Case::new(content).length(len);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{}", case.label());
        assert_eq!(out.buf_offset, 0, "{}", case.label());
        assert_eq!(out.item_type, POISON_TYPE);
    }
}

// ---------------------------------------------------------------------------
// E11 — saturation at INT_MAX  (number >= INT_MAX)
// ---------------------------------------------------------------------------
#[test]
fn err_e11_saturate_int_max() {
    for s in [
        "2147483647",
        "2147483647.0",
        "2147483648",
        "2147483649",
        "4294967296",
        "1e10",
        "1e30",
        "1e308",
        "1e309",
        "1e999",
        "1e99999",
        "2147483646.9999999999999999999",
        "9223372036854775808",
        "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, i32::MAX, "{s:?} must saturate to INT_MAX");
        assert_eq!(out.item_type, CJSON_NUMBER, "{s:?}");
    }
    // "1e309" and friends overflow to +inf in strtod.
    let out = assert_same_str("1e400");
    assert_eq!(f64::from_bits(out.item_double_bits), f64::INFINITY);
    assert_eq!(out.item_valueint, i32::MAX);
}

// ---------------------------------------------------------------------------
// E12 — saturation at INT_MIN  (number <= (double)INT_MIN)
// ---------------------------------------------------------------------------
#[test]
fn err_e12_saturate_int_min() {
    for s in [
        "-2147483648",
        "-2147483648.0",
        "-2147483649",
        "-4294967296",
        "-1e10",
        "-1e30",
        "-1e308",
        "-1e309",
        "-1e999",
        "-1e99999",
        "-9223372036854775808",
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, i32::MIN, "{s:?} must saturate to INT_MIN");
        assert_eq!(out.item_type, CJSON_NUMBER, "{s:?}");
    }
    let out = assert_same_str("-1e400");
    assert_eq!(f64::from_bits(out.item_double_bits), f64::NEG_INFINITY);
    assert_eq!(out.item_valueint, i32::MIN);
}

// ---------------------------------------------------------------------------
// E13 — one step INSIDE each limit must NOT saturate
// ---------------------------------------------------------------------------
#[test]
fn err_e13_one_step_inside_limits() {
    for &(s, want) in &[
        ("2147483646", 2147483646i32),
        ("2147483646.5", 2147483646),
        // NB: 2147483646.9999999 rounds UP to exactly 2147483647.0 as a double
        // and therefore *does* take the saturating branch — see below.
        ("-2147483647", -2147483647),
        ("-2147483647.5", -2147483647),
        ("0", 0),
        ("-0", 0),
        ("0.9", 0),
        ("-0.9", 0),
        ("1.9999", 1),
        ("-1.9999", -1),
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, want, "{s:?}");
    }
    // Exactly at the limits (must saturate) vs. the nearest representable
    // double just inside them (must truncate).
    let just_below_max = f64::from_bits((2147483647.0f64).to_bits() - 1);
    let s = format!("{just_below_max:.20}");
    let out = assert_same_str(&s);
    assert_eq!(out.ret, C_TRUE);
    assert_eq!(out.item_valueint, 2147483646, "{s}");

    let just_above_min = f64::from_bits((-2147483648.0f64).to_bits() - 1);
    let s = format!("{just_above_min:.20}");
    let out = assert_same_str(&s);
    assert_eq!(out.ret, C_TRUE);
    assert_eq!(out.item_valueint, -2147483647, "{s}");

    // A decimal so close to the limit that `strtod` rounds it ONTO the limit
    // must take the saturating branch (`>=` / `<=`, not `>` / `<`).
    for &(s, want) in &[
        ("2147483646.9999999", i32::MAX),
        ("-2147483647.9999999", i32::MIN),
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, want, "{s:?}");
    }
}

// ---------------------------------------------------------------------------
// E14 — strtod consumes only a PREFIX: offset advances by the consumed amount
// ---------------------------------------------------------------------------
#[test]
fn err_e14_partial_consumption() {
    for &(s, want_offset) in &[
        ("1e", 1usize),
        ("1E", 1),
        ("1e+", 1),
        ("1e-", 1),
        ("1.", 2),
        ("1.2.3", 3),
        ("1e5e5", 3),
        ("1-2", 1),
        ("1+2", 1),
        ("12--", 2),
        (".5.5", 2),
        ("0.1.2.3", 3),
        ("1..", 2),
        ("5e", 1),
        ("5ee5", 1),
        ("1e2e", 3),
        ("-1e", 2),
        ("+1.", 3),
    ] {
        let case = Case::new(s);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.buf_offset, want_offset, "{s:?} offset");
        // `number_string_length` is the whole charset run; `offset` advances only
        // by what strtod actually consumed, which is <= that run.
        assert!(out.buf_offset <= s.len(), "{s:?}");
    }
    // Rows where strtod consumes a STRICT prefix of the scanned charset run.
    for &(s, want_offset) in &[
        ("1e", 1usize),
        ("1e+", 1),
        ("1.2.3", 3),
        ("1e5e5", 3),
        ("1-2", 1),
        ("1+2", 1),
        ("12--", 2),
        (".5.5", 2),
        ("1..", 2),
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.buf_offset, want_offset, "{s:?}");
        assert!(
            out.buf_offset < s.len(),
            "{s:?} must be a strict prefix consumption"
        );
    }
}

// ---------------------------------------------------------------------------
// E15 — underflow / subnormals (errno is ignored by the C)
// ---------------------------------------------------------------------------
#[test]
fn err_e15_underflow() {
    for s in [
        "1e-309", "1e-320", "5e-324", "4e-324", "2e-324", "1e-324", "1e-400", "1e-999",
        "1e-99999", "-1e-999", "-5e-324", "0e-99999",
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, 0, "{s:?}");
        assert_eq!(out.item_type, CJSON_NUMBER, "{s:?}");
    }
}

// ---------------------------------------------------------------------------
// E16 — out-of-range "enum" values for item->type across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_e16_out_of_range_type_field() {
    const TYPES: &[i32] = &[
        -1,
        0,
        i32::MAX,
        i32::MIN,
        1 << 30,
        CJSON_NUMBER,
        0x7FFF_FFFE,
        0xF, // cJSON_Number | others
        -0x8000_0000i64 as i32,
    ];
    const VALUEINTS: &[i32] = &[0, -1, i32::MAX, i32::MIN, 0x5555_5555];
    const DOUBLES: &[u64] = &[
        0,
        0x8000_0000_0000_0000,        // -0.0
        0x7FF0_0000_0000_0000,        // +inf
        0xFFF0_0000_0000_0000,        // -inf
        0x7FF8_0000_0000_0000,        // quiet NaN
        0x7FF8_0000_DEAD_BEEF,        // NaN with payload
        0xFFFF_FFFF_FFFF_FFFF,        // negative NaN, all payload bits set
        0x0000_0000_0000_0001,        // smallest subnormal
    ];
    for &t in TYPES {
        for &vi in VALUEINTS {
            for &d in DOUBLES {
                // Success path: all three fields must be overwritten identically.
                let out = assert_same(&Case::new("42.5").item_state(t, vi, d));
                assert_eq!(out.ret, C_TRUE);
                assert_eq!(out.item_type, CJSON_NUMBER);
                assert_eq!(out.item_valueint, 42);
                assert_eq!(f64::from_bits(out.item_double_bits), 42.5);

                // Failure path: all three fields must be preserved bit-for-bit,
                // including NaN payloads.
                for bad in ["+", "]", ""] {
                    let out = assert_same(&Case::new(bad).item_state(t, vi, d));
                    assert_eq!(out.ret, C_FALSE, "{bad:?}");
                    assert_eq!(out.item_type, t);
                    assert_eq!(out.item_valueint, vi);
                    assert_eq!(out.item_double_bits, d);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E17 — `depth` is never read or written
// ---------------------------------------------------------------------------
#[test]
fn err_e17_depth_preserved() {
    for depth in [0usize, 1, 42, usize::MAX, usize::MAX - 1, 1 << 63] {
        for s in ["123", "+", "", "-1.5e-3", "]"] {
            let case = Case::new(s).depth(depth);
            let out = assert_same(&case);
            assert_eq!(out.buf_depth, depth, "{}", case.label());
        }
    }
}

// ---------------------------------------------------------------------------
// E18 — oversized `length` (SIZE_MAX) with an in-buffer terminator
// ---------------------------------------------------------------------------
#[test]
fn err_e18_oversized_length() {
    for &(content, want_ret, want_off) in &[
        (&b"12,"[..], C_TRUE, 2usize),
        (&b"12]"[..], C_TRUE, 2),
        (&b"-3.5e2 "[..], C_TRUE, 6),
        (&b",12"[..], C_FALSE, 0),
        (&b"+,"[..], C_FALSE, 0),
        (&b"\0"[..], C_FALSE, 0),
    ] {
        for &len in &[usize::MAX, usize::MAX - 1, 1 << 40, content.len() + 1000] {
            let case = Case::new(content).length(len);
            let out = assert_same(&case);
            assert_eq!(out.ret, want_ret, "{}", case.label());
            assert_eq!(out.buf_offset, want_off, "{}", case.label());
            assert_eq!(out.buf_length, len);
        }
    }
}

// ---------------------------------------------------------------------------
// E19 — `offset + index` unsigned wraparound in can_access_at_index
// ---------------------------------------------------------------------------
#[test]
fn err_e19_offset_wraparound() {
    let content = b"12345678".to_vec();
    for &(off, len) in &[
        (usize::MAX, 8usize),
        (usize::MAX, 1),
        (usize::MAX, usize::MAX),
        (usize::MAX - 1, 8),
        (usize::MAX - 7, 8),
        (1usize << 63, 8),
    ] {
        let case = Case::new(&content).length(len).offset(off);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_FALSE, "{}", case.label());
        assert_eq!(out.buf_offset, off, "offset must not move");
        assert_eq!(out.item_type, POISON_TYPE);
    }
}

// ---------------------------------------------------------------------------
// E20 — embedded NUL stops the scan via `default:`
// ---------------------------------------------------------------------------
#[test]
fn err_e20_embedded_nul() {
    let content = b"1\0 2".to_vec();
    let case = Case::new(&content).length(4);
    let out = assert_same(&case);
    assert_eq!(out.ret, C_TRUE);
    assert_eq!(out.buf_offset, 1);
    assert_eq!(out.item_valueint, 1);

    // NUL first -> rejection.
    let content = b"\0123".to_vec();
    let out = assert_same(&Case::new(&content).length(4));
    assert_eq!(out.ret, C_FALSE);

    // NUL in the middle of what would otherwise be a longer number.
    let content = b"-12.5\0e9".to_vec();
    let out = assert_same(&Case::new(&content).length(8));
    assert_eq!(out.ret, C_TRUE);
    assert_eq!(out.buf_offset, 5);
    assert_eq!(f64::from_bits(out.item_double_bits), -12.5);
}

// ---------------------------------------------------------------------------
// Generic boundaries required in addition to the table
// ---------------------------------------------------------------------------
#[test]
fn err_generic_null_and_size_boundaries() {
    // Every combination of the three pointer-nullability axes that does not
    // dereference NULL in the C (item is only dereferenced on success).
    for &(buffer_null, content_null, item_null) in &[
        (true, false, false),
        (true, false, true),
        (true, true, false),
        (true, true, true),
        (false, true, false),
        (false, true, true),
    ] {
        let mut case = Case::new("123");
        if buffer_null {
            case = case.buffer_null();
        }
        if content_null {
            case = case.content_null();
        }
        if item_null {
            case = case.item_null();
        }
        assert_eq!(assert_same(&case).ret, C_FALSE, "{}", case.label());
    }

    // Zero / one / oversized lengths.
    for &len in &[0usize, 1, 2, 3, usize::MAX] {
        let case = Case::new(b"12,".to_vec()).length(len);
        assert_same(&case);
    }

    // One step past every documented boundary value.
    for s in [
        "2147483646", "2147483647", "2147483648", // INT_MAX - 1 / INT_MAX / +1
        "-2147483647", "-2147483648", "-2147483649", // INT_MIN + 1 / INT_MIN / -1
        "1.7976931348623157e308",  // DBL_MAX
        "1.7976931348623159e308",  // one step past DBL_MAX -> +inf
        "-1.7976931348623157e308",
        "-1.7976931348623159e308",
        "2.2250738585072014e-308", // DBL_MIN (normal)
        "2.2250738585072011e-308", // subnormal
        "4.9406564584124654e-324", // DBL_TRUE_MIN
        "2.4703282292062328e-324", // half of it -> rounds to 0 or TRUE_MIN
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
    }
}

/// Out-of-range values for every scalar field crossed over the FFI boundary,
/// randomized. Nothing here may crash or diverge.
#[test]
fn err_generic_random_field_values() {
    let mut rng = Rng::new(SEED ^ 0xBAD);
    for _ in 0..4000 {
        let n = rng.range(0, 8);
        let content: Vec<u8> = (0..n).map(|_| *rng.pick(CHARSET)).collect();
        let clen = content.len();
        let length = match rng.below(4) {
            0 => 0,
            1 => clen,
            2 => rng.below(clen + 1),
            _ => clen,
        };
        let offset = match rng.below(4) {
            0 => 0,
            1 => rng.below(clen + 1),
            2 => length,
            _ => usize::MAX - rng.below(4),
        };
        let case = Case::new(&content)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item_state(
                rng.next_u64() as i32,
                rng.next_u64() as i32,
                rng.next_u64(),
            );
        assert_same(&case);
    }
}
