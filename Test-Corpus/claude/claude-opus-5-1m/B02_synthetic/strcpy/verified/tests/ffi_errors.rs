//! Phase C - error path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1-16, the library rows; rows 17-24 are
//! the driver rows and live in `tests/exe_diff.rs`), plus the generic C API
//! boundaries: NULL pointers, zero and oversized lengths and out-of-range
//! "enum" values for `operation`.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

const ALL_OPS: [c_int; 5] = [0, 1, 2, 3, 4];
const BAD_OPS: [c_int; 12] = [
    5,
    6,
    -1,
    -2,
    -3,
    99,
    1000,
    i32::MAX,
    i32::MIN,
    0x7FFF_FFFE,
    -0x8000_0000,
    0x0001_0000,
];
const FLAG_SET: [u32; 8] = [
    0,
    1,
    2,
    3,
    4,
    0x8000_0000,
    0xFFFF_FFFF,
    0x0000_00FF,
];

fn args_at(
    input: *mut c_char,
    input_len: usize,
    reference: *const c_char,
    ref_len: usize,
    operation: c_int,
    flags: u32,
) -> Args {
    Args {
        input,
        input_len,
        reference,
        ref_len,
        operation,
        flags,
    }
}

// ---------------------------------------------------------------------------
// row 1: input == NULL -> -1 (checked before the switch)
// ---------------------------------------------------------------------------

#[test]
fn err_input_null_all_ops() {
    let mut region = Region::new();
    region.place(b"START\0", b"START\0");
    let refp = region.ref_ptr();
    let mut ops: Vec<c_int> = ALL_OPS.to_vec();
    ops.extend_from_slice(&BAD_OPS);
    for op in ops {
        for flags in FLAG_SET {
            for ref_len in [0usize, 1, 6, usize::MAX] {
                let a = args_at(std::ptr::null_mut(), 0, refp, ref_len, op, flags);
                let r = diff(a, "row1 input NULL");
                assert_eq!(r, -1, "NULL input must be rejected before the switch");
                // also with a non-zero input_len and a NULL reference
                let a = args_at(std::ptr::null_mut(), 64, std::ptr::null(), ref_len, op, flags);
                let r = diff(a, "row1 input+reference NULL");
                assert_eq!(r, -1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 2-4: reference == NULL for operations 0, 2 and 4 -> -2
// ---------------------------------------------------------------------------

fn reference_null_case(op: c_int, name: &str) {
    let mut region = Region::new();
    region.place(b"VALID\0", b"VALID\0");
    for flags in FLAG_SET {
        for input_len in [0usize, 1, 5, 6, 1024, usize::MAX] {
            let inp = region.input_ptr();
            let a = args_at(inp, input_len, std::ptr::null(), 0, op, flags);
            let r = diff(a, name);
            assert_eq!(r, -2, "{name}: NULL reference must give -2");
            let inp = region.input_ptr();
            let a = args_at(inp, input_len, std::ptr::null(), usize::MAX, op, flags);
            let r = diff(a, name);
            assert_eq!(r, -2, "{name}: ref_len must not matter");
        }
    }
}

#[test]
fn err_reference_null_op0() {
    reference_null_case(0, "row2 op0 reference NULL");
}

#[test]
fn err_reference_null_op2() {
    reference_null_case(2, "row3 op2 reference NULL");
}

#[test]
fn err_reference_null_op4() {
    reference_null_case(4, "row4 op4 reference NULL");
}

// ---------------------------------------------------------------------------
// row 5: operation outside {0,1,2,3,4} -> -3
// ---------------------------------------------------------------------------

#[test]
fn err_bad_operation() {
    let rng = Rng::new(105);
    let mut region = Region::new();
    for op in BAD_OPS {
        for flags in FLAG_SET {
            let a = rand_bytes(&rng, rng.below(10), true);
            let b = rand_bytes(&rng, rng.below(10), true);
            region.place(&a, &b);
            let (la, lb) = (a.len(), b.len());
            let inp = region.input_ptr();
            let refp = region.ref_ptr();
            let ar = args_at(inp, la, refp, lb, op, flags);
            let r = diff(ar, "row5 invalid operation");
            assert_eq!(r, -3, "operation {op} must be rejected with -3");
        }
    }
    // random operations, mostly invalid
    for _ in 0..500 {
        let op = rng.next_u64() as i32;
        let a = rand_bytes(&rng, rng.below(10), true);
        let b = rand_bytes(&rng, rng.below(10), true);
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, la, refp, lb, op, rng.next_u64() as u32);
        let r = diff_auto(ar, "row5 random operation");
        if !(0..=4).contains(&op) {
            assert_eq!(r, Outcome::Value(-3), "operation {op}");
        }
    }
}

// ---------------------------------------------------------------------------
// row 6: operation 3 with NULL reference / ref_len == 0 falls back to ':'
// ---------------------------------------------------------------------------

#[test]
fn err_op3_null_ref_defaults_colon() {
    let mut region = Region::new();
    for (data, expect) in [
        (b"ab:cd\0".as_slice(), 2i32),
        (b":\0", 0),
        (b"abc\0", -1),
        (b"EMPTY\0", -3),
    ] {
        region.place(data, b"|\0");
        let len = data.len();
        // NULL reference with a non-zero ref_len -> still ':'
        let inp = region.input_ptr();
        let a = args_at(inp, len, std::ptr::null(), 7, 3, 0);
        let r = diff(a, "row6 op3 NULL reference");
        assert_eq!(r, expect);
        // non-NULL reference but ref_len == 0 -> still ':'
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, len, refp, 0, 3, 0);
        let r = diff(a, "row6 op3 ref_len 0");
        assert_eq!(r, expect);
    }
}

// ---------------------------------------------------------------------------
// row 7: validate_token, nothing matches -> 0
// ---------------------------------------------------------------------------

#[test]
fn err_validate_token_no_match() {
    let rng = Rng::new(107);
    let mut region = Region::new();
    for _ in 0..300 {
        let a = rand_bytes(&rng, 1 + rng.below(10), true);
        let mut b = rand_bytes(&rng, 1 + rng.below(10), true);
        // make sure they differ
        b[0] = a[0].wrapping_add(1) | 1;
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, la, refp, lb, 0, 0);
        let r = diff(ar, "row7 validate_token no match");
        assert_eq!(r, 0);
    }
}

// ---------------------------------------------------------------------------
// row 8: parse_command, nothing matches -> -1
// ---------------------------------------------------------------------------

#[test]
fn err_parse_command_no_match() {
    let mut region = Region::new();
    for data in [
        b"\0".as_slice(),
        b"x\0",
        b"start\0",
        b"STARTX\0",
        b"STOPX\0",
        b"ADMINX\0",
        b"admin\0",
        b" START\0",
        b"RESUMES\0",
    ] {
        for len in [0usize, 1, 2, 4, 5, 6, 7, 1024] {
            region.place(data, b"\0");
            let inp = region.input_ptr();
            let refp = region.ref_ptr();
            let ar = args_at(inp, len, refp, 1, 1, 0);
            let r = diff(ar, "row8 parse_command no match");
            if data == b"\0" || data[0] != b'S' {
                assert_eq!(r, -1, "data {data:?} len {len} must not match");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 9-10: compare_prefix returns 0
// ---------------------------------------------------------------------------

#[test]
fn err_compare_prefix_exact_no_match() {
    let rng = Rng::new(109);
    let mut region = Region::new();
    for _ in 0..300 {
        let p = rand_bytes(&rng, 1 + rng.below(12), true);
        let mut s = p.clone();
        s[0] = p[0].wrapping_add(1) | 1;
        region.place(&s, &p);
        let (ls, lp) = (s.len(), p.len());
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, ls, refp, lp, 2, 1);
        let r = diff(ar, "row9 compare_prefix exact no match");
        assert_eq!(r, 0);
    }
}

#[test]
fn err_compare_prefix_loose_no_match() {
    let rng = Rng::new(110);
    let mut region = Region::new();
    for _ in 0..300 {
        let p = rand_bytes(&rng, 1 + rng.below(12), true);
        let mut s = p.clone();
        s[0] = p[0].wrapping_add(1) | 1;
        region.place(&s, &p);
        let (ls, lp) = (s.len(), p.len());
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, ls, refp, lp, 2, 0);
        let r = diff(ar, "row10 compare_prefix loose no match");
        assert_eq!(r, 0);
    }
}

// ---------------------------------------------------------------------------
// row 11: find_delimiter with len == 0 -> -1 without touching `data`
// ---------------------------------------------------------------------------

#[test]
fn err_find_delimiter_zero_len() {
    let mut region = Region::new();
    region.place(b"NONE\0", b"|\0");
    for flags in FLAG_SET {
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, 0, refp, 2, 3, flags);
        let r = diff(a, "row11 find_delimiter len 0");
        assert_eq!(r, -1);
    }
    // `data` is not dereferenced at all when len == 0: an unmapped, non-NULL
    // pointer must still return -1 in both implementations.
    let bogus = 1usize as *mut c_char;
    let refp = region.ref_ptr();
    let a = args_at(bogus, 0, refp, 2, 3, 0);
    let r = diff_forked(a, "row11 find_delimiter len 0, bogus pointer");
    assert_eq!(r, Outcome::Value(-1));
}

// ---------------------------------------------------------------------------
// rows 12-14: find_delimiter special patterns and "not found"
// ---------------------------------------------------------------------------

#[test]
fn err_find_delimiter_none() {
    let mut region = Region::new();
    for len in [1usize, 4, 5, 100] {
        region.place(b"NONE\0", b"|\0");
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, len, refp, 2, 3, 0);
        let r = diff(a, "row12 find_delimiter NONE");
        assert_eq!(r, -2, "len {len}");
    }
    // the same data with a different delimiter is *not* the special case
    for delim in [b":", b"N", b"x"] {
        region.place(b"NONE\0", &[delim[0], 0]);
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, 5, refp, 2, 3, 0);
        diff(a, "row12 find_delimiter NONE other delimiter");
    }
}

#[test]
fn err_find_delimiter_empty() {
    let mut region = Region::new();
    for len in [1usize, 5, 6, 100] {
        region.place(b"EMPTY\0", b":\0");
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, len, refp, 2, 3, 0);
        let r = diff(a, "row13 find_delimiter EMPTY");
        assert_eq!(r, -3, "len {len}");
    }
    for delim in [b"|", b"E", b"x"] {
        region.place(b"EMPTY\0", &[delim[0], 0]);
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, 6, refp, 2, 3, 0);
        diff(a, "row13 find_delimiter EMPTY other delimiter");
    }
}

#[test]
fn err_find_delimiter_not_found() {
    let rng = Rng::new(114);
    let mut region = Region::new();
    for _ in 0..300 {
        let len = 1 + rng.below(20);
        let delim = rng.byte() | 1;
        let data: Vec<u8> = (0..len)
            .map(|_| {
                let mut c = rng.byte() | 1;
                if c == delim {
                    c = delim.wrapping_add(1) | 1;
                }
                c
            })
            .collect();
        region.place(&data, &[delim, 0]);
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, len, refp, 2, 3, 0);
        diff(ar, "row14 find_delimiter not found");
    }
    // NUL inside the range -> break, then the two strcmp specials fail
    for _ in 0..100 {
        let len = 2 + rng.below(10);
        let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        data[0] = 0;
        region.place(&data, b"%\0");
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, len, refp, 2, 3, 0);
        let r = diff(ar, "row14 find_delimiter NUL first");
        assert_eq!(r, -1);
    }
}

// ---------------------------------------------------------------------------
// row 15: match_pattern, nothing matches -> 0
// ---------------------------------------------------------------------------

#[test]
fn err_match_pattern_no_match() {
    let rng = Rng::new(115);
    let mut region = Region::new();
    for flags in [0u32, 2u32] {
        for _ in 0..300 {
            // text longer than the pattern (so the case sensitive branch does
            // not underflow), sharing no byte with it
            let tlen = 4 + rng.below(10);
            let plen = 1 + rng.below(3);
            let text: Vec<u8> = (0..tlen).map(|_| b'a' + (rng.byte() % 5)).collect();
            let pattern: Vec<u8> = (0..plen).map(|_| b'0' + (rng.byte() % 5)).collect();
            let mut text = text;
            let mut pattern = pattern;
            text.push(0);
            pattern.push(0);
            region.place(&text, &pattern);
            let (lt, lp) = (text.len(), pattern.len());
            let inp = region.input_ptr();
            let refp = region.ref_ptr();
            let ar = args_at(inp, lt, refp, lp, 4, flags);
            let r = diff(ar, "row15 match_pattern no match");
            assert_eq!(r, 0, "flags {flags:#x}");
        }
    }
}

// ---------------------------------------------------------------------------
// row 16: match_pattern case sensitive with strlen(text) < strlen(pattern):
// `text_len - pattern_len` underflows and the loop walks off the buffer.
// ---------------------------------------------------------------------------

#[test]
fn err_match_pattern_underflow() {
    let mut region = Region::new();
    // (a) the pattern happens to live ahead of the text, so the runaway loop
    //     finds it at the very same offset in both implementations
    for (text, pattern) in [
        (b"AB\0".as_slice(), b"ABCDE\0".as_slice()),
        (b"\0", b"X\0"),
        (b"a\0", b"abcdefghij\0"),
    ] {
        region.place(text, pattern);
        let (lt, lp) = (text.len(), pattern.len());
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let ar = args_at(inp, lt, refp, lp, 4, 2);
        let r = diff_forked(ar, "row16 match_pattern underflow (match ahead)");
        println!("underflow text={text:?} pattern={pattern:?} -> {r:?}");
        assert!(
            matches!(r, Outcome::Value(v) if v >= 10),
            "expected the loop to find the pattern ahead of the text: {r:?}"
        );
    }

    // (b) nothing to find ahead of the text: both implementations must walk off
    //     the end of the mapping and die from the same signal
    let mut saw_signal = false;
    for (off, text, pattern) in [
        (15000usize, b"AB\0".as_slice(), b"ZQXJVK\0".as_slice()),
        (16000, b"\0", b"ZQXJVKWY\0".as_slice()),
    ] {
        region.place(&[], pattern);
        region.write(off, text);
        let (lt, lp) = (text.len(), pattern.len());
        let inp = region.at(off);
        let refp = region.ref_ptr();
        let ar = args_at(inp, lt, refp, lp, 4, 2);
        let r = diff_forked(ar, "row16 match_pattern underflow (runs off the end)");
        println!("underflow off={off} text={text:?} pattern={pattern:?} -> {r:?}");
        saw_signal |= matches!(r, Outcome::Signal(_));
    }
    assert!(
        saw_signal,
        "the unbounded loop is expected to run off the end of the mapping"
    );
}

// ---------------------------------------------------------------------------
// generic C API boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_null_reference_ops_that_allow_it() {
    let mut region = Region::new();
    // operation 1 ignores `reference` completely, operation 3 falls back to ':'
    for data in [b"START\0".as_slice(), b"ab:c\0", b"NONE\0", b"EMPTY\0"] {
        for op in [1i32, 3] {
            for ref_len in [0usize, 1, 1024, usize::MAX] {
                region.place(data, b"\0");
                let len = data.len();
                let inp = region.input_ptr();
                let a = args_at(inp, len, std::ptr::null(), ref_len, op, 0);
                diff(a, "generic: NULL reference with op 1/3");
            }
        }
    }
}

#[test]
fn err_oversized_lengths() {
    let rng = Rng::new(200);
    let mut region = Region::new();
    for op in ALL_OPS {
        for flags in [0u32, 1, 2, 3] {
            for &len in &[
                0usize,
                1,
                1024,
                1025,
                4096,
                Region::SIZE,
                1usize << 32,
                usize::MAX / 2,
                usize::MAX,
            ] {
                let a = rand_bytes(&rng, rng.below(8), true);
                let b = rand_bytes(&rng, rng.below(8), true);
                region.place(&a, &b);
                let inp = region.input_ptr();
                let refp = region.ref_ptr();
                let ar = args_at(inp, len, refp, len, op, flags);
                diff_forked(ar, "generic: oversized lengths");
            }
        }
    }
}

#[test]
fn err_zero_length_non_null_buffers() {
    let mut region = Region::new();
    for op in ALL_OPS {
        for flags in FLAG_SET {
            region.place(&[0u8], &[0u8]);
            let inp = region.input_ptr();
            let refp = region.ref_ptr();
            let ar = args_at(inp, 0, refp, 0, op, flags);
            diff_auto(ar, "generic: zero lengths, empty strings");
        }
    }
}

#[test]
fn err_operation_one_past_range() {
    let mut region = Region::new();
    region.place(b"START\0", b"START\0");
    for op in [-1i32, 5] {
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, 6, refp, 6, op, 0);
        let r = diff(a, "generic: operation one past the valid range");
        assert_eq!(r, -3);
    }
    // and the whole neighbourhood of the switch
    for op in -8i32..12 {
        let inp = region.input_ptr();
        let refp = region.ref_ptr();
        let a = args_at(inp, 6, refp, 6, op, 2);
        let r = diff_auto(a, "generic: operation neighbourhood");
        if !(0..=4).contains(&op) {
            assert_eq!(r, Outcome::Value(-3));
        }
    }
}
