//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so` via `libloading`, and asserts they return the SAME
//! sentinel (`cJSON_bool` 0 = false / 1 = true) and leave identical state.

mod common;

use common::*;

const FALSE: std::ffi::c_int = 0;
const TRUE: std::ffi::c_int = 1;

/* ---- row 1: input_buffer == NULL ---------------------------------- */

#[test]
fn err_null_input_buffer() {
    let p = pair();
    for pre in [CJson::garbage(), CJson { type_: 0, valueint: 0, valuedouble: 0.0 }] {
        let mut c_item = pre;
        let mut r_item = pre;
        let c_ret = unsafe { p.c.parse_number_raw(&mut c_item, std::ptr::null_mut()) };
        let r_ret = unsafe { p.rust.parse_number_raw(&mut r_item, std::ptr::null_mut()) };
        assert_eq!(c_ret, FALSE, "C must return false for NULL input_buffer");
        assert_eq!(r_ret, c_ret, "return sentinel mismatch");
        // item must be untouched by both
        assert_eq!(
            (c_item.type_, c_item.valueint, c_item.valuedouble.to_bits()),
            (r_item.type_, r_item.valueint, r_item.valuedouble.to_bits()),
        );
        assert_eq!(
            (c_item.type_, c_item.valueint, c_item.valuedouble.to_bits()),
            (pre.type_, pre.valueint, pre.valuedouble.to_bits()),
        );
    }
    // also with item == NULL: both return before touching either pointer
    let c_ret = unsafe { p.c.parse_number_raw(std::ptr::null_mut(), std::ptr::null_mut()) };
    let r_ret = unsafe { p.rust.parse_number_raw(std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_eq!(c_ret, FALSE);
    assert_eq!(r_ret, FALSE);
}

/* ---- row 2: input_buffer->content == NULL ------------------------- */

#[test]
fn err_null_content() {
    for (length, offset, depth) in [
        (0usize, 0usize, 0usize),
        (10, 0, 0),
        (10, 5, 3),
        (0, 99, 7),
        (usize::MAX, usize::MAX, usize::MAX),
        (1, 0, 0),
    ] {
        let o = diff(
            &format!("err2/{length}/{offset}"),
            None,
            length,
            offset,
            depth,
            CJson::garbage(),
        );
        assert_eq!(o.ret, FALSE, "NULL content must be rejected");
        assert_eq!(o.buf_offset, offset, "offset must be unchanged");
        assert_eq!(o.buf_length, length);
        assert_eq!(o.buf_depth, depth);
        assert_eq!(o.type_, CJson::garbage().type_, "item untouched");
    }
}

/* ---- row 4: non-numeric first byte -> strtod consumed nothing ----- */

#[test]
fn err_non_numeric_first_byte() {
    for s in [
        &b"abc"[..], b"null", b"true", b"false", b" 1", b"\t1", b"\n1",
        b"{", b"}", b"[", b"]", b",", b"\"1\"", b"\0123", b"x1", b"NaN",
        b"nan", b"inf", b"Inf", b"INFINITY", b"#1", b"/1",
        // NOTE: "0x10" is NOT here — the C scanner accepts the leading '0',
        // stops at 'x', and strtod("0") succeeds, so C returns true. Hex is
        // covered as a success case in cfg11/cfg21.
    ] {
        let o = diff_str("err4", s);
        assert_eq!(
            o.ret, FALSE,
            "expected rejection for {:?}",
            String::from_utf8_lossy(s)
        );
        assert_eq!(o.buf_offset, 0, "offset must not advance on rejection");
        assert_eq!(o.type_, CJson::garbage().type_, "item untouched");
    }
}

#[test]
fn err_random_non_numeric() {
    let rng = Rng::new(SEED);
    for k in 0..4000 {
        // first byte guaranteed outside the accepted class
        let first = loop {
            let b = rng.byte();
            if !NUMERIC_ALPHABET.contains(&b) {
                break b;
            }
        };
        let mut v = vec![first];
        let n = rng.range(0, 12) as usize;
        v.extend((0..n).map(|_| rng.byte()));
        let o = diff(&format!("err4/rand#{k}"), Some(&v), v.len(), 0, 5, CJson::garbage());
        assert_eq!(o.ret, FALSE);
        assert_eq!(o.buf_offset, 0);
    }
}

/* ---- row 5: prefix scanned but strtod rejects it ------------------ */

#[test]
fn err_scanned_but_unparsable() {
    for s in [
        &b"+"[..], b"-", b".", b"e", b"E", b"+-", b"-+", b"-.", b"+.",
        b"e5", b"E+1", b".e1", b"+e", b"--1", b"++", b"..", b"...",
        b"ee", b"eE", b"e.", b".+", b".-", b"+.e", b"-.-", b"e-5",
        b"E-", b"+E", b"-e", b".E.", b"+++", b"---", b"-.e-",
    ] {
        let o = diff_str("err5", s);
        assert_eq!(
            o.ret, FALSE,
            "expected rejection for {:?}",
            String::from_utf8_lossy(s)
        );
        assert_eq!(o.buf_offset, 0, "offset must not advance");
        assert_eq!(o.type_, CJson::garbage().type_, "item untouched");
        assert_eq!(o.valueint, CJson::garbage().valueint);
        assert_eq!(o.valuedouble_bits, CJson::garbage().valuedouble.to_bits());
    }
}

#[test]
fn err_random_sign_exp_soup() {
    let rng = Rng::new(SEED ^ 0xFFFF);
    for k in 0..6000 {
        // only signs/dots/exponent letters: never a valid strtod number
        let n = rng.range(1, 8) as usize;
        let v = rand_from(&rng, b"+-.eE", n);
        let o = diff(&format!("err5/rand#{k}"), Some(&v), v.len(), 0, 0, CJson::garbage());
        assert_eq!(
            o.ret, FALSE,
            "sign/exponent-only input {:?} must be rejected",
            String::from_utf8_lossy(&v)
        );
        assert_eq!(o.buf_offset, 0);
    }
}

/* ---- rows 6, 7, 8: empty window / zero length / offset past end --- */

#[test]
fn err_empty_window() {
    let content = b"98765";
    let o = diff("err6/off==len", Some(content), 5, 5, 0, CJson::garbage());
    assert_eq!(o.ret, FALSE);
    assert_eq!(o.buf_offset, 5);
}

#[test]
fn err_offset_ge_length() {
    let content = b"12345678";
    for len in 0..=8usize {
        for off in len..len + 5 {
            let o = diff(
                &format!("err6/{len}/{off}"),
                Some(content),
                len,
                off,
                1,
                CJson::garbage(),
            );
            assert_eq!(
                o.ret, FALSE,
                "offset {off} >= length {len} must be rejected"
            );
            assert_eq!(o.buf_offset, off);
            assert_eq!(o.type_, CJson::garbage().type_);
        }
    }
}

#[test]
fn err_zero_length() {
    for off in [0usize, 1, 7, usize::MAX] {
        let content = b"12345";
        let o = diff(&format!("err7/{off}"), Some(content), 0, off, 0, CJson::garbage());
        assert_eq!(o.ret, FALSE, "length == 0 must be rejected");
    }
}

#[test]
fn err_offset_past_end() {
    let content = b"1";
    for off in [2usize, 3, 100, 1 << 20, usize::MAX / 2] {
        let o = diff(&format!("err8/{off}"), Some(content), 1, off, 0, CJson::garbage());
        assert_eq!(o.ret, FALSE, "offset {off} past end must be rejected");
        assert_eq!(o.buf_offset, off);
    }
}

/* ---- row 9: offset == SIZE_MAX (size_t wrap in can_access_at_index) */

#[test]
fn err_offset_size_max() {
    let content = b"12345";
    for len in [0usize, 1, 5, usize::MAX] {
        // NOTE: offset == usize::MAX with length == usize::MAX would make
        // `offset + 0 < length` false, so the window is still empty.
        let o = diff(
            &format!("err9/len{len}"),
            Some(content),
            len,
            usize::MAX,
            0,
            CJson::garbage(),
        );
        assert_eq!(
            o.ret, FALSE,
            "offset == SIZE_MAX must yield an empty window and be rejected"
        );
        assert_eq!(o.buf_offset, usize::MAX);
    }
    for off in [usize::MAX - 1, usize::MAX - 2] {
        let o = diff(&format!("err9/off{off}"), Some(content), 5, off, 0, CJson::garbage());
        assert_eq!(o.ret, FALSE);
    }
}

/* ---- row 11: item == NULL on every error path --------------------- */

#[test]
fn err_null_item_on_error_paths() {
    let p = pair();
    // Each case must fail BEFORE `item` is dereferenced, so NULL is safe.
    let cases: &[(&[u8], usize, usize)] = &[
        (b"abc", 3, 0),      // row 4
        (b"+", 1, 0),        // row 5
        (b"-", 1, 0),        // row 5
        (b".", 1, 0),        // row 5
        (b"e", 1, 0),        // row 5
        (b"12345", 0, 0),    // row 7 zero length
        (b"12345", 5, 5),    // row 6 empty window
        (b"12345", 3, 9),    // row 8 offset past end
        (b"12345", 5, usize::MAX), // row 9 wrap
    ];
    for (i, (s, length, offset)) in cases.iter().enumerate() {
        let mut c_buf = ParseBuffer {
            content: s.as_ptr(),
            length: *length,
            offset: *offset,
            depth: 4,
        };
        let mut r_buf = c_buf;
        let c_ret = unsafe { p.c.parse_number_raw(std::ptr::null_mut(), &mut c_buf) };
        let r_ret = unsafe { p.rust.parse_number_raw(std::ptr::null_mut(), &mut r_buf) };
        assert_eq!(c_ret, FALSE, "case {i}: C must reject before touching item");
        assert_eq!(r_ret, c_ret, "case {i}: sentinel mismatch");
        assert_eq!(
            (c_buf.length, c_buf.offset, c_buf.depth),
            (r_buf.length, r_buf.offset, r_buf.depth),
            "case {i}: buffer state mismatch"
        );
    }
    // NULL content with NULL item
    let mut c_buf = ParseBuffer { content: std::ptr::null(), length: 9, offset: 1, depth: 2 };
    let mut r_buf = c_buf;
    let c_ret = unsafe { p.c.parse_number_raw(std::ptr::null_mut(), &mut c_buf) };
    let r_ret = unsafe { p.rust.parse_number_raw(std::ptr::null_mut(), &mut r_buf) };
    assert_eq!((c_ret, r_ret), (FALSE, FALSE));
}

/* ---- row 12: oversized length (SIZE_MAX) with terminating first byte */

#[test]
fn err_size_max_length() {
    // First byte is outside the accepted class, so the scanner terminates on
    // `default: goto loop_end` before any out-of-bounds read can happen, even
    // though `length` claims the buffer is enormous.
    for s in [&b"abc"[..], b"!", b"\0", b"{", b" "] {
        for length in [usize::MAX, usize::MAX - 1, 1 << 40] {
            let o = diff(
                &format!("err12/{length}"),
                Some(s),
                length,
                0,
                0,
                CJson::garbage(),
            );
            assert_eq!(
                o.ret, FALSE,
                "non-numeric first byte with oversized length must be rejected"
            );
            assert_eq!(o.buf_length, length);
        }
    }
}

/* ---- rows 13/14/15: saturation and one-step-inside boundaries ----- */

#[test]
fn bound_int_max() {
    for s in [
        &b"2147483647"[..], b"2147483648", b"2147483647.0", b"2147483647.9",
        b"1e999", b"1e300", b"2.147483648e9", b"99999999999",
    ] {
        let o = diff_str("bound/max", s);
        assert_eq!(o.ret, TRUE, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.valueint, i32::MAX, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.type_, 8);
    }
}

#[test]
fn bound_int_min() {
    for s in [
        &b"-2147483648"[..], b"-2147483649", b"-2147483648.0",
        b"-2147483648.9", b"-1e999", b"-1e300", b"-2.147483649e9",
        b"-99999999999",
    ] {
        let o = diff_str("bound/min", s);
        assert_eq!(o.ret, TRUE, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.valueint, i32::MIN, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.type_, 8);
    }
}

#[test]
fn bound_one_inside() {
    for (s, want) in [
        (&b"2147483646"[..], 2147483646i32),
        (b"2147483646.9", 2147483646),
        (b"-2147483647", -2147483647),
        (b"-2147483647.9", -2147483647),
        // NOTE: "2147483646.999999999" is NOT here — it rounds to exactly
        // 2147483647.0 as a double, so it takes the `>= INT_MAX` saturation
        // branch and yields INT_MAX. Covered in `bound_int_max`.
        (b"2147483645.5", 2147483645),
    ] {
        let o = diff_str("bound/inside", s);
        assert_eq!(o.ret, TRUE);
        assert_eq!(o.valueint, want, "{:?}", String::from_utf8_lossy(s));
    }
    // exhaustive sweep of the ±2^31 neighbourhood
    let mut cases = Vec::new();
    for d in -40i64..=40 {
        cases.push(format!("{}", 2147483647i64 + d));
        cases.push(format!("{}", -2147483648i64 + d));
        cases.push(format!("{}.5", 2147483647i64 + d));
        cases.push(format!("{}.5", -2147483648i64 + d));
    }
    for c in &cases {
        diff_str("bound/neighbourhood", c.as_bytes());
    }
}

/* ---- row 16: underflow / subnormal -------------------------------- */

#[test]
fn bound_underflow() {
    for s in [
        &b"1e-999"[..], b"-1e-999", b"1e-400", b"1e-324", b"1e-325",
        b"4.9406564584124654e-324", b"2.4703282292062327e-324",
    ] {
        let o = diff_str("bound/underflow", s);
        assert_eq!(o.ret, TRUE, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.valueint, 0);
        assert_eq!(o.type_, 8);
    }
}

/* ---- row 17: out-of-range "enum" values in item.type -------------- */

#[test]
fn err_garbage_type_field() {
    // C enums / ints accept ANY value across FFI. `cJSON.type` is a plain int
    // and `cJSON_Number` == 8; feed values with no valid cJSON variant.
    const BOGUS: [i32; 12] = [
        -1, 0, 1, 2, 4, 7, 8, 9, 255, 12345, i32::MAX, i32::MIN,
    ];
    for t in BOGUS {
        // success path: type must be overwritten with 8
        let o = diff(
            &format!("err17/ok/{t}"),
            Some(b"123"),
            3,
            0,
            0,
            CJson { type_: t, valueint: -7, valuedouble: 1.25 },
        );
        assert_eq!(o.ret, TRUE);
        assert_eq!(o.type_, 8, "cJSON_Number must overwrite type {t}");

        // failure path: type must be preserved verbatim
        let o = diff(
            &format!("err17/fail/{t}"),
            Some(b"zzz"),
            3,
            0,
            0,
            CJson { type_: t, valueint: -7, valuedouble: 1.25 },
        );
        assert_eq!(o.ret, FALSE);
        assert_eq!(o.type_, t, "failure must preserve type {t}");
        assert_eq!(o.valueint, -7);
        assert_eq!(o.valuedouble_bits, 1.25f64.to_bits());
    }
}

/* ---- row 18: depth ignored ---------------------------------------- */

#[test]
fn cfg_depth_ignored() {
    for depth in [0usize, 1, usize::MAX, usize::MAX - 1, 1 << 63] {
        for s in [&b"7"[..], b"zzz"] {
            let o = diff(
                &format!("err18/{depth}"),
                Some(s),
                s.len(),
                0,
                depth,
                CJson::garbage(),
            );
            assert_eq!(o.buf_depth, depth, "depth must be returned unmodified");
        }
    }
}

/* ---- row 19: no terminator, numeric run reaches `length` ---------- */

#[test]
fn cfg_no_terminator() {
    // exact-size heap allocation, no NUL anywhere
    for s in ["1", "12", "-3", "4.5", "6e7", "-8.9e-10", "2147483648"] {
        let exact: Box<[u8]> = s.as_bytes().to_vec().into_boxed_slice();
        let o = diff(
            &format!("err19/{s}"),
            Some(&exact),
            exact.len(),
            0,
            0,
            CJson::garbage(),
        );
        assert_eq!(o.ret, TRUE, "{s}");
        assert_eq!(o.buf_offset, exact.len(), "{s}: must consume the whole window");
    }
}

/* ---- row 20: interior NUL ---------------------------------------- */

#[test]
fn err_interior_nul() {
    let content: &[u8] = b"12\0 34";
    let o = diff("err20", Some(content), content.len(), 0, 0, CJson::garbage());
    assert_eq!(o.ret, TRUE);
    assert_eq!(o.buf_offset, 2, "scan must stop at the interior NUL");
    assert_eq!(o.valueint, 12);
    // NUL as the very first byte -> rejection (covered by row 4 too)
    let o = diff("err20/first", Some(b"\012"), 3, 0, 0, CJson::garbage());
    assert_eq!(o.ret, FALSE);
    // NUL in the middle of an exponent
    let o = diff("err20/exp", Some(b"1e\0 5"), 5, 0, 0, CJson::garbage());
    assert_eq!(o.ret, TRUE);
    assert_eq!(o.buf_offset, 1, "strtod backs off the dangling exponent");
}

/* ---- generic FFI boundary sweep ---------------------------------- */

#[test]
fn generic_boundary_sweep() {
    // Every single byte value as a whole 1-byte input, plus each byte value
    // preceded and followed by a digit — exhaustively across the boundary.
    for b in 0u16..=255 {
        let byte = b as u8;
        diff_str("generic/solo", &[byte]);
        diff_str("generic/after", &[b'1', byte]);
        diff_str("generic/before", &[byte, b'1']);
        diff_str("generic/around", &[b'1', byte, b'2']);
        diff_str("generic/exp", &[b'1', b'e', byte]);
        diff_str("generic/dot", &[b'1', b'.', byte]);
        diff_str("generic/sign", &[b'-', byte]);
    }
    // one step past each documented bound
    let content = b"1234567890";
    for length in 0..=content.len() + 2 {
        for offset in 0..=content.len() + 2 {
            diff(
                &format!("generic/{length}/{offset}"),
                Some(content),
                length,
                offset,
                0,
                CJson::garbage(),
            );
        }
    }
}
