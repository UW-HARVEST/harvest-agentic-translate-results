//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries.
//! Every test drives BOTH shared objects through their exported `parse_number`
//! symbol and asserts the SAME rejection value (`(cJSON_bool)0`) *and* the same
//! post-state byte image.

mod harness;

use harness::*;
use std::ffi::c_int;

const C_FALSE: c_int = 0;
const C_TRUE: c_int = 1;

/// Call the C `.so` alone and return its verdict, so tests can assert the
/// *absolute* expected error value from `ERRORS.md`, not just "both agree".
fn c_verdict(content: &[u8], length: usize, offset: usize) -> (c_int, CJson, ParseBuffer) {
    let mut item = SENTINEL_ITEM;
    let mut buf = ParseBuffer {
        content: content.as_ptr(),
        length,
        offset,
        depth: 0,
    };
    // Safety: C-ABI symbol, valid pointers.
    let r = unsafe { (c_parse_number())(&mut item, &mut buf) };
    (r, item, buf)
}

/* ============ row 1 — input_buffer == NULL ================================ */

#[test]
fn err_row01_null_input_buffer() {
    // Absolute expectation: the C returns false (0) and leaves *item alone.
    let mut item = SENTINEL_ITEM;
    // Safety: the C NULL-checks input_buffer before any dereference.
    let r = unsafe { (c_parse_number())(&mut item, std::ptr::null_mut()) };
    assert_eq!(r, C_FALSE, "C must return false for NULL input_buffer");
    assert_eq!(item.type_, SENTINEL_ITEM.type_, "C must not touch item.type");
    assert_eq!(item.valueint, SENTINEL_ITEM.valueint);
    assert_eq!(
        item.valuedouble.to_bits(),
        SENTINEL_ITEM.valuedouble.to_bits()
    );

    // Differential, across many item pre-states.
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..N {
        let pre = CJson {
            type_: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        diff_null_buffer("err01", pre);
    }
    diff_null_buffer("err01/sentinel", SENTINEL_ITEM);
}

/* ============ row 2 — input_buffer->content == NULL ======================= */

#[test]
fn err_row02_null_content() {
    let mut item = SENTINEL_ITEM;
    let mut buf = ParseBuffer {
        content: std::ptr::null(),
        length: 10,
        offset: 0,
        depth: 4,
    };
    // Safety: the C NULL-checks content before any read.
    let r = unsafe { (c_parse_number())(&mut item, &mut buf) };
    assert_eq!(r, C_FALSE, "C must return false for NULL content");
    assert_eq!(buf.offset, 0, "C must not advance offset");
    assert_eq!(buf.length, 10);
    assert_eq!(buf.depth, 4);
    assert_eq!(item.type_, SENTINEL_ITEM.type_);

    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..N {
        let length = if rng.bool() { rng.next_u64() as usize } else { rng.below(64) };
        let offset = if rng.bool() { rng.next_u64() as usize } else { rng.below(64) };
        let depth = rng.next_u64() as usize;
        let pre = CJson {
            type_: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        diff_null_content("err02", length, offset, depth, pre);
    }
    for &(length, offset) in &[
        (0usize, 0usize),
        (0, usize::MAX),
        (usize::MAX, 0),
        (usize::MAX, usize::MAX),
        (1, 0),
    ] {
        diff_null_content("err02/edges", length, offset, 0, SENTINEL_ITEM);
    }
}

/* ============ row 3 — malloc failure ===================================== */

#[test]
fn err_row03_allocation_failure_documented() {
    // The C path is `malloc(number_string_length + 1) == NULL -> return false`.
    // It cannot be induced from outside without interposing the allocator, and
    // interposing would affect only one of the two libraries (they use different
    // allocators), so a differential test of the failure itself is impossible.
    //
    // What IS verifiable, and is verified here, is the precondition that makes
    // the two sides request the same amount: `number_string_length + 1` never
    // overflows and never exceeds the accessible input range, so neither side
    // can be pushed into a spurious allocation failure that the other avoids.
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..200 {
        let n = rng.range(1, 2048);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        // number_string_length is bounded by the accessible range...
        let (_r, _i, b) = c_verdict(&s, s.len(), 0);
        assert!(b.offset <= s.len(), "offset must stay inside the input");
        // ...and the same call through both libraries agrees.
        diff("err03/bounded", &s);
    }
    // Largest realistic run: 1 MiB of accepted bytes. Both must succeed
    // identically (i.e. neither reports a bogus allocation failure).
    let big = vec![b'9'; 1 << 20];
    diff("err03/1MiB", &big);
    let mut big2 = vec![b'0'; 1 << 20];
    big2[0] = b'1';
    big2.push(b'}');
    diff("err03/1MiB+term", &big2);
}

/* ============ row 4a — zero-length run, non-numeric lead ================== */

#[test]
fn err_row04a_zero_length_non_numeric_lead() {
    for s in [
        "x1", "null", "true", "false", " 1", "\t1", "\n1", "{", "}", "[", "]", ",", ":", "\"1\"",
        "a", "Z", "/", "\\", "~", "\u{7f}",
    ] {
        let (r, item, buf) = c_verdict(s.as_bytes(), s.len(), 0);
        assert_eq!(r, C_FALSE, "C must reject {s:?}");
        assert_eq!(buf.offset, 0, "offset must not advance for {s:?}");
        assert_eq!(item.type_, SENTINEL_ITEM.type_, "item untouched for {s:?}");
        diff("err04a", s.as_bytes());
    }
    // every byte that is NOT in the accepted set must reject when it leads
    let mut rejected = 0usize;
    for b in 0u8..=255 {
        if ACCEPTED.contains(&b) {
            continue;
        }
        let content = [b, b'1', b'2'];
        let (r, _i, buf) = c_verdict(&content, 3, 0);
        assert_eq!(r, C_FALSE, "C must reject leading byte {b:#04x}");
        assert_eq!(buf.offset, 0);
        rejected += 1;
        diff("err04a/allbytes", &content);
    }
    assert_eq!(rejected, 256 - ACCEPTED.len());
}

/* ============ row 4b — zero-length run, empty accessible range ============ */

#[test]
fn err_row04b_zero_length_empty_range() {
    let backing = b"123456789".to_vec();
    // length == 0
    let (r, _i, b) = c_verdict(&backing, 0, 0);
    assert_eq!(r, C_FALSE, "C must reject length==0");
    assert_eq!(b.offset, 0);
    // offset == length
    let (r, _i, b) = c_verdict(&backing, 9, 9);
    assert_eq!(r, C_FALSE, "C must reject offset==length");
    assert_eq!(b.offset, 9);
    // offset > length
    let (r, _i, b) = c_verdict(&backing, 4, 9);
    assert_eq!(r, C_FALSE, "C must reject offset>length");
    assert_eq!(b.offset, 9);

    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..N {
        let length = rng.below(backing.len() + 1);
        let offset = length + rng.below(2048);
        diff_full("err04b", &backing, length, offset, rng.next_u64() as usize, SENTINEL_ITEM);
    }
    for &(length, offset) in &[
        (0usize, 0usize),
        (9, 9),
        (9, 10),
        (0, usize::MAX),
        (1, usize::MAX),
        (usize::MAX, usize::MAX),
    ] {
        diff_full("err04b/edges", &backing, length, offset, 0, SENTINEL_ITEM);
    }
}

/* ============ row 4c — non-empty run with no valid strtod prefix ========== */

#[test]
fn err_row04c_nonempty_but_unparsable() {
    let cases: &[&str] = &[
        "+", "-", ".", "e", "E", "+.", "-.", ".+", ".-", "e5", "E5", "e+5", "e-5", ".e5", ".E5",
        "++1", "--1", "+-1", "-+1", "-e", "+e", "-E", "..1", "...", "..", ".e", ".E", "+e5",
        "-.e5", "eE", "Ee", "+++", "---", ".-.", "-..", "+..", "e.", "E.", "e..", "+.e",
    ];
    for s in cases {
        let (r, item, buf) = c_verdict(s.as_bytes(), s.len(), 0);
        assert_eq!(r, C_FALSE, "C must reject {s:?} (strtod consumes nothing)");
        assert_eq!(buf.offset, 0, "offset must not advance for {s:?}");
        assert_eq!(item.type_, SENTINEL_ITEM.type_, "item untouched for {s:?}");
        assert_eq!(item.valueint, SENTINEL_ITEM.valueint);
        assert_eq!(
            item.valuedouble.to_bits(),
            SENTINEL_ITEM.valuedouble.to_bits(),
            "valuedouble must NOT be written for {s:?}"
        );
        diff("err04c", s.as_bytes());
    }

    // Randomized: accepted-charset strings containing NO digit can never form a
    // valid strtod prefix, so every one of them must be rejected by both.
    let no_digit = b"+-eE.";
    let mut rng = Rng::new(SEED ^ 105);
    let mut checked = 0usize;
    for _ in 0..(N * 4) {
        let n = rng.range(1, 10);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(no_digit)).collect();
        let (r, _i, b) = c_verdict(&s, s.len(), 0);
        assert_eq!(
            r,
            C_FALSE,
            "C must reject digit-free accepted-charset input {:?}",
            String::from_utf8_lossy(&s)
        );
        assert_eq!(b.offset, 0);
        diff("err04c/rand", &s);
        checked += 1;
    }
    assert!(checked > 0);
}

/* ============ rows 5/6 — INT_MAX / INT_MIN saturation ==================== */

#[test]
fn err_row05_saturate_high() {
    let cases: &[&str] = &[
        "2147483647",
        "2147483648",
        "1e999",
        "1e400",
        "3000000000",
        "1e300",
        "2147483647.9",
        "9999999999999999999999",
    ];
    for s in cases {
        let (r, item, _b) = c_verdict(s.as_bytes(), s.len(), 0);
        assert_eq!(r, C_TRUE, "C must accept {s:?}");
        assert_eq!(item.valueint, i32::MAX, "C must saturate {s:?} to INT_MAX");
        assert_eq!(item.type_, 1 << 3, "type must be cJSON_Number");
        diff("err05", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 4);
        s.push(b'e');
        s.extend_from_slice(&rng.digits_range(2, 4));
        let txt = String::from_utf8(s.clone()).unwrap();
        let (r, item, _b) = c_verdict(&s, s.len(), 0);
        if r == C_TRUE && item.valuedouble >= 2147483647.0 {
            assert_eq!(item.valueint, i32::MAX, "saturation for {txt:?}");
        }
        diff("err05/rand", &s);
    }
}

#[test]
fn err_row06_saturate_low() {
    let cases: &[&str] = &[
        "-2147483648",
        "-2147483649",
        "-1e999",
        "-1e400",
        "-3000000000",
        "-1e300",
        "-2147483648.1",
        "-9999999999999999999999",
    ];
    for s in cases {
        let (r, item, _b) = c_verdict(s.as_bytes(), s.len(), 0);
        assert_eq!(r, C_TRUE, "C must accept {s:?}");
        assert_eq!(item.valueint, i32::MIN, "C must saturate {s:?} to INT_MIN");
        assert_eq!(item.type_, 1 << 3, "type must be cJSON_Number");
        diff("err06", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..N {
        let mut s = vec![b'-'];
        s.extend_from_slice(&rng.digits_range(1, 4));
        s.push(b'e');
        s.extend_from_slice(&rng.digits_range(2, 4));
        let txt = String::from_utf8(s.clone()).unwrap();
        let (r, item, _b) = c_verdict(&s, s.len(), 0);
        if r == C_TRUE && item.valuedouble <= -2147483648.0 {
            assert_eq!(item.valueint, i32::MIN, "saturation for {txt:?}");
        }
        diff("err06/rand", &s);
    }
}

/* ============ G6 — out-of-range "enum"/tag values across the FFI ========== */

#[test]
fn err_row06b_out_of_range_type_tag_preserved() {
    // `lib.h` has no C `enum`; the type tag and `cJSON_bool` are plain `int`,
    // so ANY int is a legal input across the boundary. On the failure paths the
    // C never writes `type`, so an out-of-range tag must survive untouched --
    // and on success it must be replaced by exactly `cJSON_Number == 8`.
    let tags: &[i32] = &[
        i32::MIN,
        -1,
        0,
        1,
        7,
        8,
        9,
        1 << 3,
        1 << 30,
        i32::MAX,
        0x7f7f_7f7f,
        0x8000_0000u32 as i32,
        0xFFFF_FFFEu32 as i32,
    ];
    let failing: &[&str] = &["", "x", "+", "-", ".", "e", "++1", " 1"];
    let succeeding: &[&str] = &["0", "42", "-1", "1e999", "-1e999", "-0", "1e", "1.2.3"];

    for &tag in tags {
        let pre = CJson {
            type_: tag,
            valueint: 0x0BAD_BEEFu32 as i32,
            valuedouble: f64::from_bits(0xFFF8_0000_0000_0001),
        };
        for s in failing {
            let mut item = pre;
            let mut buf = ParseBuffer {
                content: s.as_ptr(),
                length: s.len(),
                offset: 0,
                depth: 0,
            };
            // Safety: C-ABI symbol, valid pointers.
            let r = unsafe { (c_parse_number())(&mut item, &mut buf) };
            assert_eq!(r, C_FALSE, "C must reject {s:?}");
            assert_eq!(item.type_, tag, "failing path must preserve tag {tag}");
            diff_full("err06b/fail", s.as_bytes(), s.len(), 0, 0, pre);
        }
        for s in succeeding {
            let mut item = pre;
            let mut buf = ParseBuffer {
                content: s.as_ptr(),
                length: s.len(),
                offset: 0,
                depth: 0,
            };
            // Safety: C-ABI symbol, valid pointers.
            let r = unsafe { (c_parse_number())(&mut item, &mut buf) };
            assert_eq!(r, C_TRUE, "C must accept {s:?}");
            assert_eq!(item.type_, 8, "success path must set cJSON_Number");
            diff_full("err06b/ok", s.as_bytes(), s.len(), 0, 0, pre);
        }
    }

    // The RETURN value is a `cJSON_bool` (int): assert the C only ever produces
    // exactly 0 or 1, and the Rust matches bit-for-bit (covered by `diff`).
    let mut rng = Rng::new(SEED ^ 108);
    for _ in 0..(N * 2) {
        let n = rng.range(1, 20);
        let s: Vec<u8> = (0..n)
            .map(|_| if rng.bool() { *rng.pick(ACCEPTED) } else { rng.byte() })
            .collect();
        let (r, _i, _b) = c_verdict(&s, s.len(), 0);
        assert!(r == 0 || r == 1, "cJSON_bool must be 0/1, got {r}");
        diff("err06b/bool", &s);
    }
}

/* ============ G3 — oversized length ====================================== */

#[test]
fn err_boundary_oversized_length() {
    // `length` lies, but a non-accepted byte inside the real allocation stops
    // the scan, so no OOB read occurs on either side.
    let mut rng = Rng::new(SEED ^ 109);
    for _ in 0..N {
        let mut s: Vec<u8> = Vec::new();
        if rng.bool() {
            s.extend_from_slice(&rng.digits_range(1, 10));
        }
        s.push(non_accepted_byte(&mut rng));
        for &length in &[usize::MAX, usize::MAX - 1, 1 << 48, s.len() + 1_000] {
            diff_full("gb3", &s, length, 0, 0, SENTINEL_ITEM);
        }
    }
    for s in ["}", "1}", "-1}", ".}", "e}", "+}"] {
        diff_full("gb3/fixed", s.as_bytes(), usize::MAX, 0, 0, SENTINEL_ITEM);
    }
}

/* ============ G4 — offset one past / far past the end ==================== */

#[test]
fn err_boundary_offset_past_end() {
    let backing = b"1234567890".to_vec();
    let mut rng = Rng::new(SEED ^ 110);
    for &length in &[0usize, 1, 5, 10] {
        for delta in [0usize, 1, 2, 3, 100, 1 << 20] {
            diff_full("gb4", &backing, length, length + delta, 0, SENTINEL_ITEM);
        }
        // offset == SIZE_MAX exercises the wrapping `offset + index`
        diff_full("gb4/max", &backing, length, usize::MAX, 0, SENTINEL_ITEM);
        diff_full("gb4/max-1", &backing, length, usize::MAX - 1, 0, SENTINEL_ITEM);
    }
    for _ in 0..N {
        let length = rng.below(11);
        let offset = length.saturating_add(rng.next_u64() as usize);
        diff_full("gb4/rand", &backing, length, offset, rng.next_u64() as usize, SENTINEL_ITEM);
    }
}

/* ============ G5 — one step past the documented int range ================ */

#[test]
fn err_boundary_int_range_edges() {
    let imax = 2147483647.0f64;
    let imin = -2147483648.0f64;
    let mut vals: Vec<String> = vec![
        "2147483646".into(),
        "2147483647".into(),
        "2147483648".into(),
        "-2147483647".into(),
        "-2147483648".into(),
        "-2147483649".into(),
    ];
    // ±1 ULP around the two clamp thresholds, printed with full precision.
    for base in [imax, imin] {
        for v in [
            base,
            f64::from_bits(base.to_bits() + 1),
            f64::from_bits(base.to_bits() - 1),
        ] {
            vals.push(format!("{v:.20e}"));
            vals.push(format!("{v:?}"));
        }
    }
    // and the fractional neighbourhood of both clamps
    for f in [
        "2147483646.5",
        "2147483647.5",
        "2147483647.9999999999",
        "-2147483647.5",
        "-2147483648.5",
        "-2147483648.0000000001",
        "0.9999999999",
        "-0.9999999999",
        "1.5",
        "-1.5",
        "0.5",
        "-0.5",
    ] {
        vals.push(f.into());
    }
    for s in &vals {
        // strings from {:?}/{:e} may contain non-accepted bytes; that is fine,
        // the point is that both sides agree on where the scan stops.
        diff("gb5", s.as_bytes());
    }
    // Exhaustive-ish sweep of the clamp region as plain integers.
    for d in -300i64..=300 {
        let hi = (2_147_483_647i64 + d).to_string();
        let lo = (-2_147_483_648i64 + d).to_string();
        diff("gb5/hi", hi.as_bytes());
        diff("gb5/lo", lo.as_bytes());
    }
    // Truncation-toward-zero must match for in-range fractional values.
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..(N * 2) {
        let neg = rng.bool();
        let int_part = rng.digits_range(1, 10);
        let frac = rng.digits_range(1, 8);
        let mut s: Vec<u8> = Vec::new();
        if neg {
            s.push(b'-');
        }
        s.extend_from_slice(&int_part);
        s.push(b'.');
        s.extend_from_slice(&frac);
        diff("gb5/trunc", &s);
    }
}

/* ============ G8 — content that is not NUL-terminated ==================== */

#[test]
fn err_boundary_unterminated_content() {
    // Exact-fit allocations made only of accepted bytes: there is no '\0' in
    // the caller's buffer at all, so the implementation MUST rely on its own
    // copy + appended terminator.
    let mut rng = Rng::new(SEED ^ 112);
    for _ in 0..(N * 2) {
        let n = rng.range(1, 32);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        assert!(!s.contains(&0));
        // length == alloc size: the scan may run right up to the last byte.
        diff_full("gb8", &s, s.len(), 0, 0, SENTINEL_ITEM);
        // and starting at every offset inside it
        let off = rng.below(s.len());
        diff_full("gb8/off", &s, s.len(), off, 0, SENTINEL_ITEM);
    }
    for s in ["1", "1e", "1e5", "-", ".", "+", "1.2", "2147483648", "1e999", "-1e999"] {
        let v: Vec<u8> = s.bytes().collect();
        diff_full("gb8/fixed", &v, v.len(), 0, 0, SENTINEL_ITEM);
    }
}
