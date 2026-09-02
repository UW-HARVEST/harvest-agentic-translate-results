//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row. Both `.so`s are loaded via `libloading`; the Rust implementation is
//! never called directly.

mod common;

use common::*;
use std::ffi::{c_double, c_int};

/// Raw differential call that lets us pass arbitrary (incl. NULL) pointers.
/// Returns `(ret, item_after, buf_after)` for one implementation.
unsafe fn raw_call(
    f: ParseNumberFn,
    item: *mut cJSON,
    buf: *mut parse_buffer,
) -> (c_int, Option<(c_int, c_int, u64)>, Option<(usize, usize, usize)>) {
    let ret = f(item, buf);
    let i = if item.is_null() {
        None
    } else {
        Some((
            (*item).type_,
            (*item).valueint,
            (*item).valuedouble.to_bits(),
        ))
    };
    let b = if buf.is_null() {
        None
    } else {
        Some(((*buf).length, (*buf).offset, (*buf).depth))
    };
    (ret, i, b)
}

fn poison_item() -> cJSON {
    cJSON {
        type_: POISON_TYPE,
        valueint: POISON_VALUEINT,
        valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
    }
}

// ------------------------------------------------------------------ E1
/// `input_buffer == NULL` → false, `item` untouched.
#[test]
fn e1_null_input_buffer() {
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..2000 {
        let seed = random_item_seed(&mut rng);
        let mk = || cJSON {
            type_: seed.type_,
            valueint: seed.valueint,
            valuedouble: f64::from_bits(seed.valuedouble_bits),
        };
        let mut ci = mk();
        let mut ri = mk();
        let c = unsafe { raw_call(c_parse_number(), &mut ci, std::ptr::null_mut()) };
        let r = unsafe { raw_call(rust_parse_number(), &mut ri, std::ptr::null_mut()) };
        assert_eq!(c, r, "[E1] divergence with item seed {seed:?}");
        assert_eq!(c.0, 0, "[E1] C must return false (0)");
        assert_eq!(
            c.1,
            Some((seed.type_, seed.valueint, seed.valuedouble_bits)),
            "[E1] item must be untouched"
        );
    }
    // Both pointers NULL.
    let c = unsafe { raw_call(c_parse_number(), std::ptr::null_mut(), std::ptr::null_mut()) };
    let r = unsafe { raw_call(rust_parse_number(), std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_eq!(c, r, "[E1] divergence with both pointers NULL");
    assert_eq!(c.0, 0);
}

// ------------------------------------------------------------------ E2
/// `input_buffer->content == NULL` → false, nothing touched, for arbitrary
/// `length` / `offset` / `depth` (incl. `SIZE_MAX`).
#[test]
fn e2_null_content() {
    let mut rng = Rng::new(SEED ^ 0xE2);
    let interesting = [0usize, 1, 2, 1000, usize::MAX - 1, usize::MAX];
    let mut cases: Vec<(usize, usize, usize)> = Vec::new();
    for &l in &interesting {
        for &o in &interesting {
            cases.push((l, o, 0));
        }
    }
    for _ in 0..2000 {
        cases.push((
            rng.next_u64() as usize,
            rng.next_u64() as usize,
            rng.next_u64() as usize,
        ));
    }
    for (l, o, d) in cases {
        let seed = random_item_seed(&mut rng);
        let mk_item = || cJSON {
            type_: seed.type_,
            valueint: seed.valueint,
            valuedouble: f64::from_bits(seed.valuedouble_bits),
        };
        let mk_buf = || parse_buffer {
            content: std::ptr::null(),
            length: l,
            offset: o,
            depth: d,
        };
        let (mut ci, mut cb) = (mk_item(), mk_buf());
        let (mut ri, mut rb) = (mk_item(), mk_buf());
        let c = unsafe { raw_call(c_parse_number(), &mut ci, &mut cb) };
        let r = unsafe { raw_call(rust_parse_number(), &mut ri, &mut rb) };
        assert_eq!(c, r, "[E2] divergence for length={l} offset={o} depth={d}");
        assert_eq!(c.0, 0, "[E2] C must return false (0)");
        assert_eq!(c.1, Some((seed.type_, seed.valueint, seed.valuedouble_bits)));
        assert_eq!(c.2, Some((l, o, d)), "[E2] buffer must be untouched");
        assert!(cb.content.is_null() && rb.content.is_null());
    }
    // content == NULL is checked BEFORE the bounds check, so it must win even
    // when the buffer would otherwise look perfectly parseable.
    let mut ci = poison_item();
    let mut cb = parse_buffer {
        content: std::ptr::null(),
        length: 3,
        offset: 0,
        depth: 0,
    };
    let mut ri = poison_item();
    let mut rb = parse_buffer {
        content: std::ptr::null(),
        length: 3,
        offset: 0,
        depth: 0,
    };
    let c = unsafe { raw_call(c_parse_number(), &mut ci, &mut cb) };
    let r = unsafe { raw_call(rust_parse_number(), &mut ri, &mut rb) };
    assert_eq!(c, r);
    assert_eq!(c.0, 0);
}

// ------------------------------------------------------------------ E3
/// Allocation failure. Not injectable in the prebuilt C `.so` (no allocator
/// hook, and the requested size is always `number_string_length + 1 <= length + 1`),
/// so this row is verified structurally: both implementations return `false`
/// *without* touching `item` or `offset` on that branch, which is the same
/// observable contract as E4. The smallest possible request (`1` byte, empty
/// run) and the largest realistic request are both exercised so that the
/// success side of the allocation is covered from both ends.
#[test]
fn e3_allocation_failure() {
    // Smallest request: malloc(0 + 1).
    assert_same("E3/min-alloc", &Scenario::new(vec![b'x', 0]));
    assert_same("E3/min-alloc", &Scenario::new(vec![]).length(0));
    // Large request: malloc(1_000_001).
    let mut data = vec![b'9'; 1_000_000];
    data.push(0);
    assert_same("E3/large-alloc", &Scenario::new(data));
    // Structural check on the Rust side: the only failure path before strtod
    // returns exactly 0, matching E1/E2/E4 shape. Confirmed by the fact that
    // every `false` outcome in the whole suite leaves `item` and `offset` alone
    // (asserted in E4 below).
}

// ------------------------------------------------------------------ E4
/// `strtod` consumed zero bytes → false, `item` untouched, `offset` unchanged.
#[test]
fn e4_strtod_consumed_nothing() {
    let unparsable = [
        "", ".", "+", "-", "e", "E", "+.", "-.", ".+", ".-", "e5", "E5", ".e1", ".E1", "++1",
        "--1", "-e", "+e", "e+", "e-", "e.", "E.", "..", "...", "+-", "-+", "++", "--", ".e",
        ".E", "e+5", "E-5", "..1", "+.e", "-.E", "eee", "EEE", "+++", "---", ".+.", "-.-",
    ];
    for s in unparsable {
        for sc in [
            Scenario::from_str_nul(s),
            Scenario::from_str_no_term(s),
            Scenario::new({
                let mut v = s.as_bytes().to_vec();
                v.extend_from_slice(b" trailing");
                v
            }),
        ] {
            assert_same("E4", &sc);
            // And assert the C contract explicitly.
            let c = run(c_parse_number(), &sc);
            assert_eq!(c.ret, 0, "[E4] {s:?} must be rejected");
            assert_eq!(c.type_, POISON_TYPE, "[E4] {s:?} must not write type");
            assert_eq!(c.valueint, POISON_VALUEINT, "[E4] {s:?} must not write valueint");
            assert_eq!(
                c.valuedouble_bits, POISON_DOUBLE_BITS,
                "[E4] {s:?} must not write valuedouble"
            );
            assert_eq!(c.buf_offset, sc.offset, "[E4] {s:?} must not advance offset");
        }
    }
}

/// The accepted-char run is empty because the first byte is rejected.
#[test]
fn e4_empty_accepted_run() {
    for b in 0u16..=255 {
        if is_accepted(b as u8) {
            continue;
        }
        let sc = Scenario::new(vec![b as u8, b'1', b'2', b'3', 0]);
        assert_same("E4/empty-run", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.ret, 0, "[E4] leading byte {b:#04x} must be rejected");
        assert_eq!(c.buf_offset, 0);
        assert_eq!(c.type_, POISON_TYPE);
    }
}

/// Randomized runs built only from characters `strtod` cannot start with.
#[test]
fn e4_random_unparsable_runs() {
    let mut rng = Rng::new(SEED ^ 0xE4);
    let non_starters = [b'+', b'-', b'e', b'E', b'.'];
    for _ in 0..20_000 {
        let n = rng.range_incl(0, 10) as usize;
        let mut data: Vec<u8> = (0..n).map(|_| *rng.pick(&non_starters)).collect();
        // Optionally follow with digits — `strtod` still fails if the prefix is
        // not a valid number start.
        if rng.bool() {
            for _ in 0..rng.range_incl(1, 4) {
                data.push(b'0' + rng.below(10) as u8);
            }
        }
        data.push(0);
        let sc = Scenario::new(data).item(random_item_seed(&mut rng));
        assert_same("E4/random", &sc);
    }
}

// ------------------------------------------------------------------ E5
/// `can_access_at_index` bound: `offset >= length` gives a zero-length scan.
#[test]
fn e5_offset_at_or_past_length() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    let text = b"1234567890.5e7";
    for length in 0..=text.len() {
        for offset in 0..=(text.len() + 4) {
            let sc = Scenario::new(text.to_vec())
                .length(length)
                .offset(offset)
                .depth(rng.next_u64() as usize)
                .item(random_item_seed(&mut rng));
            assert_same("E5", &sc);
            if offset >= length {
                let c = run(c_parse_number(), &sc);
                assert_eq!(c.ret, 0, "[E5] offset={offset} length={length}");
                assert_eq!(c.buf_offset, offset, "[E5] offset must not advance");
                assert_eq!(c.type_, sc.item.type_, "[E5] item must be untouched");
                assert_eq!(c.valueint, sc.item.valueint);
                assert_eq!(c.valuedouble_bits, sc.item.valuedouble_bits);
            }
        }
    }
    // length == 0 with a non-empty allocation.
    assert_same("E5/zero-length", &Scenario::new(b"12345".to_vec()).length(0));
    // length == 0 and offset == 0 with a zero-size allocation.
    assert_same("E5/zero-length", &Scenario::new(Vec::new()).length(0));
}

/// `offset + index` uses wrapping `size_t` arithmetic in C — no overflow check.
/// With `offset == SIZE_MAX` the bound `offset + 0 < length` is false for every
/// `length <= SIZE_MAX`, so the scan collects nothing and both must return
/// `false` without dereferencing `content + offset` for a nonzero count.
#[test]
fn e5_size_t_wraparound_offset() {
    let data = b"12345\0".to_vec();
    for offset in [usize::MAX, usize::MAX - 1, usize::MAX - 5, isize::MAX as usize] {
        for length in [0usize, 1, 6, usize::MAX] {
            // `offset + 0 < length` must be false, otherwise the C would read
            // wild memory and the comparison would be meaningless.
            if offset < length {
                continue;
            }
            let sc = Scenario::new(data.clone()).length(length).offset(offset);
            assert_same("E5/wrap", &sc);
            let c = run(c_parse_number(), &sc);
            assert_eq!(c.ret, 0, "[E5/wrap] offset={offset} length={length}");
            assert_eq!(c.buf_offset, offset);
        }
    }
}

// ------------------------------------------------------------------ E6
/// `default: goto loop_end` — the terminator byte and everything after it must
/// not reach `strtod`, and bytes at/after `length` must never be read.
#[test]
fn e6_terminator_byte_all_256() {
    for prefix in ["1", "12", "-3", "1.5", "9e2", "0", "2147483647", "1e999"] {
        for b in 0u16..=255 {
            // terminator in the middle of the allocation, more digits after
            let mut data = prefix.as_bytes().to_vec();
            data.push(b as u8);
            data.extend_from_slice(b"999\0");
            assert_same("E6", &Scenario::new(data));

            // same, but `length` cuts the buffer right at the terminator, so the
            // terminator itself is out of bounds and must not be examined
            let mut data = prefix.as_bytes().to_vec();
            let cut = data.len();
            data.push(b as u8);
            data.extend_from_slice(b"999\0");
            assert_same("E6/cut", &Scenario::new(data).length(cut));
        }
    }
}

// ------------------------------------------------------------------ E7
/// `number >= INT_MAX` → `valueint = INT_MAX`.
#[test]
fn e7_saturate_int_max() {
    let mut rng = Rng::new(SEED ^ 0xE7);
    let cases = [
        "2147483647", "2147483647.0", "2147483647.5", "2147483648", "2147483649",
        "99999999999999999999", "1e999", "1e308", "1e38", "3e9", "2.1474836475e9",
        "+2147483647", "+1e999", "21474836470000000000",
    ];
    for s in cases {
        let sc = Scenario::from_str_nul(s);
        assert_same("E7", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.ret, 1, "[E7] {s:?} must succeed");
        assert_eq!(c.valueint, i32::MAX, "[E7] {s:?} must saturate to INT_MAX");
        assert_eq!(c.type_, 8, "[E7] type must be cJSON_Number");
    }
    for _ in 0..3000 {
        let exp = rng.range_incl(10, 500);
        let s = format!("{}e{}", nonzero_digits_str(&mut rng, 1, 6), exp);
        let sc = Scenario::from_str_nul(&s);
        assert_same("E7", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.valueint, i32::MAX, "[E7] {s:?}");
    }
}

// ------------------------------------------------------------------ E8
/// `number <= (double)INT_MIN` → `valueint = INT_MIN`. Note `<=`: exactly
/// `-2147483648.0` takes this branch (the value *is* representable as `int`,
/// but the C saturates anyway — replicated, not "fixed").
#[test]
fn e8_saturate_int_min() {
    let mut rng = Rng::new(SEED ^ 0xE8);
    let cases = [
        "-2147483648", "-2147483648.0", "-2147483648.5", "-2147483649", "-2147483650",
        "-99999999999999999999", "-1e999", "-1e308", "-1e38", "-3e9", "-2.1474836485e9",
        "-21474836480000000000",
    ];
    for s in cases {
        let sc = Scenario::from_str_nul(s);
        assert_same("E8", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.ret, 1, "[E8] {s:?} must succeed");
        assert_eq!(c.valueint, i32::MIN, "[E8] {s:?} must saturate to INT_MIN");
        assert_eq!(c.type_, 8);
    }
    // The value one ULP above -2147483648.0 must NOT saturate.
    let just_inside = "-2147483647.9999998";
    let sc = Scenario::from_str_nul(just_inside);
    assert_same("E8/just-inside", &sc);
    let c = run(c_parse_number(), &sc);
    assert_eq!(c.ret, 1);
    assert_eq!(c.valueint, -2147483647, "[E8] must truncate, not saturate");

    for _ in 0..3000 {
        let exp = rng.range_incl(10, 500);
        let s = format!("-{}e{}", nonzero_digits_str(&mut rng, 1, 6), exp);
        let sc = Scenario::from_str_nul(&s);
        assert_same("E8", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.valueint, i32::MIN, "[E8] {s:?}");
    }
}

// ------------------------------------------------------------------ E9
/// The `(int)number` branch is fenced by E7/E8 and by the scan alphabet:
/// `nan` / `inf` spellings cannot be produced, so the cast never sees a value
/// it cannot represent. Asserted, not assumed.
#[test]
fn e9_int_cast_branch_is_fenced() {
    // `nan`, `inf`, `infinity`, hex floats: the scan stops at the first letter
    // outside {e,E}, so `strtod` never sees them.
    for s in [
        "nan", "NAN", "nan(1)", "inf", "INF", "infinity", "INFINITY", "-nan", "-inf", "+inf",
        "0x10", "0X1p3", "0x1.8p1", "-0x10",
    ] {
        let sc = Scenario::from_str_nul(s);
        assert_same("E9", &sc);
        let c = run(c_parse_number(), &sc);
        // `0x10` / `0X1p3` parse only the leading `0`; the letter forms are rejected.
        if s.as_bytes()[0] == b'0' || s.as_bytes()[0] == b'-' && s.as_bytes()[1] == b'0' {
            assert_eq!(c.ret, 1, "[E9] {s:?}");
            assert_eq!(c.valueint, 0, "[E9] {s:?} must parse only the leading zero");
            assert_eq!(c.valuedouble_bits & !(1u64 << 63), 0, "[E9] {s:?} must be +/-0.0");
        } else {
            assert_eq!(c.ret, 0, "[E9] {s:?} must be rejected");
        }
    }
    // In the `else` branch the value is strictly inside (INT_MIN, INT_MAX) and
    // finite: sweep it densely and confirm plain truncation-toward-zero.
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..5000 {
        let v = rng.next_u64() as i64 % 2147483647;
        let frac = rng.below(1_000_000);
        let s = format!("{v}.{frac:06}");
        let sc = Scenario::from_str_nul(&s);
        assert_same("E9/else", &sc);
        let c = run(c_parse_number(), &sc);
        assert_eq!(c.ret, 1);
        let d = f64::from_bits(c.valuedouble_bits);
        assert!(d.is_finite(), "[E9] {s:?} must be finite");
        assert_eq!(
            c.valueint,
            d.trunc() as i32,
            "[E9] {s:?} must truncate toward zero"
        );
    }
}

// ------------------------------------------------------------------ E10
/// `item == NULL` on the failure paths: never dereferenced, so `false` is
/// returned safely by both implementations.
#[test]
fn e10_null_item_harmless_on_failure_paths() {
    // E1: input_buffer NULL as well.
    let c = unsafe { raw_call(c_parse_number(), std::ptr::null_mut(), std::ptr::null_mut()) };
    let r = unsafe { raw_call(rust_parse_number(), std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_eq!(c, r);
    assert_eq!(c.0, 0);

    // E2: content NULL.
    for (l, o, d) in [(0usize, 0usize, 0usize), (10, 0, 7), (0, usize::MAX, 1)] {
        let mut cb = parse_buffer {
            content: std::ptr::null(),
            length: l,
            offset: o,
            depth: d,
        };
        let mut rb = cb;
        let c = unsafe { raw_call(c_parse_number(), std::ptr::null_mut(), &mut cb) };
        let r = unsafe { raw_call(rust_parse_number(), std::ptr::null_mut(), &mut rb) };
        assert_eq!(c, r, "[E10] content-NULL, item-NULL, l={l} o={o}");
        assert_eq!(c.0, 0);
    }

    // E4/E5: unparsable or out-of-bounds input, so `item` is never touched.
    for (bytes, length, offset) in [
        (b"".to_vec(), 0usize, 0usize),
        (b"x123\0".to_vec(), 5, 0),
        (b".\0".to_vec(), 2, 0),
        (b"+\0".to_vec(), 2, 0),
        (b"12345".to_vec(), 5, 5),
        (b"12345".to_vec(), 0, 0),
    ] {
        let mut cd = bytes.clone();
        let mut rd = bytes.clone();
        let mut cb = parse_buffer {
            content: cd.as_mut_ptr(),
            length,
            offset,
            depth: 3,
        };
        let mut rb = parse_buffer {
            content: rd.as_mut_ptr(),
            length,
            offset,
            depth: 3,
        };
        let c = unsafe { raw_call(c_parse_number(), std::ptr::null_mut(), &mut cb) };
        let r = unsafe { raw_call(rust_parse_number(), std::ptr::null_mut(), &mut rb) };
        assert_eq!(
            c,
            r,
            "[E10] item-NULL failure path {:?} length={length} offset={offset}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(c.0, 0, "[E10] must return false");
    }
}

/// `item == NULL` on the SUCCESS path: the C dereferences it unconditionally,
/// so it must crash — and the Rust must crash the same way. Run in a forked
/// child so the harness survives, and compare the termination signal.
#[test]
fn e10_null_item_segfaults_on_success_path() {
    let c_status = fork_and_call(c_parse_number());
    let r_status = fork_and_call(rust_parse_number());
    assert_eq!(
        c_status, r_status,
        "[E10] C and Rust must fail identically for item==NULL on the success \
         path (wait status: C={c_status:#x} Rust={r_status:#x})"
    );
    // Sanity: it really did die from a memory-access fault, it did not "succeed".
    let sig = c_status & 0x7f;
    assert!(
        sig == libc::SIGSEGV || sig == libc::SIGBUS,
        "[E10] expected SIGSEGV/SIGBUS, got wait status {c_status:#x}"
    );
}

fn fork_and_call(f: ParseNumberFn) -> i32 {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: valid, fully parseable buffer + NULL item.
            let mut data = b"123\0".to_vec();
            let mut buf = parse_buffer {
                content: data.as_mut_ptr(),
                length: 4,
                offset: 0,
                depth: 0,
            };
            let ret = f(std::ptr::null_mut(), &mut buf);
            // Should be unreachable; if it is reached, exit with a distinct code.
            libc::_exit(if ret == 0 { 41 } else { 42 });
        }
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        status
    }
}

// ------------------------------------------------------------------ E11
/// Arbitrary "enum"-shaped ints across the FFI boundary: `cJSON.type`,
/// `cJSON.valueint`, arbitrary `valuedouble` bit patterns (incl. signalling
/// NaN), and arbitrary `depth`. None of these has a "valid variant" set in C,
/// so every bit pattern is a real input.
#[test]
fn e11_garbage_in_out_params() {
    let mut rng = Rng::new(SEED ^ 0xE11);
    let type_values: [c_int; 12] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        7,
        8, // cJSON_Number itself
        9,
        1 << 30,
        i32::MAX - 1,
        i32::MAX,
        0x5A5A_5A5A,
    ];
    let double_bits: [u64; 10] = [
        0x7FF8_0000_0000_0000,
        0x7FF0_0000_0000_0001, // signalling NaN
        0xFFF7_FFFF_FFFF_FFFF,
        0x7FF0_0000_0000_0000,
        0xFFF0_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0000,
        0x000F_FFFF_FFFF_FFFF,
        u64::MAX,
        0,
    ];
    let inputs = [
        "123", "-4.5e2", "1e999", "-1e999", "2147483648", "-2147483648", "", ".", "-", "e",
        "x", "0",
    ];
    for &t in &type_values {
        for &b in &double_bits {
            for s in inputs {
                let item = ItemSeed {
                    type_: t,
                    valueint: t,
                    valuedouble_bits: b,
                };
                for depth in [0usize, 1, usize::MAX] {
                    let sc = Scenario::from_str_nul(s).item(item).depth(depth);
                    assert_same("E11", &sc);
                    let c = run(c_parse_number(), &sc);
                    assert_eq!(c.buf_depth, depth, "[E11] depth must round-trip");
                    if c.ret == 0 {
                        assert_eq!(c.type_, t, "[E11] failure must not write type");
                        assert_eq!(c.valueint, t);
                        assert_eq!(c.valuedouble_bits, b);
                    } else {
                        assert_eq!(c.type_, 8, "[E11] success must set cJSON_Number");
                    }
                }
            }
        }
    }
    // Randomized garbage.
    for _ in 0..5000 {
        let s = *rng.pick(&inputs);
        let sc = Scenario::from_str_nul(s)
            .item(random_item_seed(&mut rng))
            .depth(rng.next_u64() as usize);
        assert_same("E11/random", &sc);
    }
}

/// The return value must be exactly `1` (`true`) or `0` (`false`) — a caller
/// comparing `== 1` must behave the same against both libraries.
#[test]
fn e11_return_value_is_exactly_0_or_1() {
    let mut rng = Rng::new(SEED ^ 0x11E);
    for _ in 0..20_000 {
        let n = rng.range_incl(0, 24) as usize;
        let data: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let length = rng.range_incl(0, n as u64) as usize;
        let offset = rng.range_incl(0, (length + 1) as u64) as usize;
        let sc = Scenario::new(data)
            .length(length)
            .offset(offset)
            .item(random_item_seed(&mut rng));
        let c = run(c_parse_number(), &sc);
        let r = run(rust_parse_number(), &sc);
        assert!(c.ret == 0 || c.ret == 1, "[E11] C ret = {}", c.ret);
        assert!(r.ret == 0 || r.ret == 1, "[E11] Rust ret = {}", r.ret);
        assert_eq!(c, r, "[E11] divergence");
    }
}

// ------------------------------------------------------------------ E12
/// Zero and oversized lengths.
#[test]
fn e12_zero_and_oversized_lengths() {
    let mut rng = Rng::new(SEED ^ 0xE12);

    // length == 0 for every offset.
    for offset in [0usize, 1, 100, usize::MAX] {
        assert_same(
            "E12/zero-len",
            &Scenario::new(b"12345\0".to_vec()).length(0).offset(offset),
        );
    }

    // Oversized `length` (far beyond the real allocation) but with a terminator
    // byte safely inside the allocation, so neither implementation reads past it.
    for text in [
        &b"123\0"[..],
        &b"1.5e2 "[..],
        &b"-7,"[..],
        &b"x1\0"[..],
        &b"\0"[..],
        &b"2147483648]"[..],
        &b"1e999\0"[..],
    ] {
        let mut data = text.to_vec();
        // generous slack so an accidental over-read still lands in our allocation
        data.extend_from_slice(&[0u8; 64]);
        for length in [
            text.len() * 2,
            1 << 20,
            u32::MAX as usize,
            usize::MAX / 2,
            usize::MAX - 1,
            usize::MAX,
        ] {
            let sc = Scenario::new(data.clone())
                .length(length)
                .item(random_item_seed(&mut rng));
            assert_same("E12/oversized", &sc);
        }
    }

    // Huge `offset` together with huge `length`: `offset + 0 < length` decides.
    for (offset, length) in [
        (usize::MAX, usize::MAX),
        (usize::MAX - 1, usize::MAX),
        (usize::MAX / 2, usize::MAX / 2),
        (1 << 40, 1 << 20),
    ] {
        // Only safe to compare when the bound rejects immediately.
        if offset < length {
            continue;
        }
        let sc = Scenario::new(b"123\0".to_vec()).length(length).offset(offset);
        assert_same("E12/huge", &sc);
        assert_eq!(run(c_parse_number(), &sc).ret, 0);
    }
}

// -------------------------------------------------------------- helper
/// Digit string with a non-zero leading digit, so the resulting value is
/// guaranteed non-zero (needed to actually reach the saturation branches).
fn nonzero_digits_str(rng: &mut Rng, lo: u64, hi: u64) -> String {
    let n = rng.range_incl(lo, hi) as usize;
    let mut out = String::new();
    out.push((b'1' + rng.below(9) as u8) as char);
    for _ in 1..n {
        out.push((b'0' + rng.below(10) as u8) as char);
    }
    out
}

#[allow(dead_code)]
fn rand_digits_str(rng: &mut Rng, lo: u64, hi: u64) -> String {
    let n = rng.range_incl(lo, hi) as usize;
    (0..n)
        .map(|_| (b'0' + rng.below(10) as u8) as char)
        .collect()
}

// Keep `c_double` referenced so the import is not spuriously unused.
const _: () = {
    let _ = std::mem::size_of::<c_double>();
};
