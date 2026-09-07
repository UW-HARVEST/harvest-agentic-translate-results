//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition and asserts C and
//! Rust return the SAME sentinel value (or leave the same buffer unchanged),
//! not merely "both failed somehow".

mod harness;

use harness::{assert_alloc_diff, assert_pair_eq, load, measure_pair, Rng};
use std::os::raw::{c_char, c_int};

// ===========================================================================
// Rows 1–6: arity length dispatch and the `-1` rejection
// ===========================================================================

/// `arity` with `len < 2` must return `-1` WITHOUT dereferencing `params`, so a
/// genuine null pointer is the strongest form of this test: if either side
/// touched `params` the process would fault.
fn arity_null_expect_minus1(len: c_int) {
    let p = load();
    let c = unsafe { (p.c.arity)(len, std::ptr::null()) };
    let r = unsafe { (p.rust.arity)(len, std::ptr::null()) };
    assert_eq!(c, -1, "C arity(len={len}, NULL) must return -1, got {c}");
    assert_eq!(
        c, r,
        "arity(len={len}, NULL): C returned {c}, Rust returned {r}"
    );
}

#[test]
fn row01_arity_len0_returns_minus1_without_deref() {
    arity_null_expect_minus1(0);
}

#[test]
fn row02_arity_len1_returns_minus1_without_deref() {
    arity_null_expect_minus1(1);
}

#[test]
fn row03_arity_len256_truncates_to_zero_returns_minus1() {
    // The header says `int len`; the definition says `unsigned char`. 256 has a
    // zero low byte, so the C returns -1 even though 256 >= 4. A translation
    // that read the full `int` would dispatch to arity4 and crash on NULL.
    arity_null_expect_minus1(256);
}

#[test]
fn row04_arity_len_257_to_259_truncate() {
    let p = load();
    let mut rng = Rng::new(0xC004);
    // 257 -> 1 -> rejected, with a NULL params to prove no dereference.
    arity_null_expect_minus1(257);

    // 258 -> 2 -> arity2 ; 259 -> 3 -> arity3. Same result as the untruncated
    // equivalents.
    let params: Vec<c_int> = (0..4).map(|_| rng.i32_mixed()).collect();
    for (len, equiv) in [(258, 2), (259, 3)] {
        let big = measure_pair("phase_c.rs:59 big", || unsafe { (p.c.arity)(len, params.as_ptr()) });
        let small = measure_pair("phase_c.rs:60 small", || unsafe { (p.c.arity)(equiv, params.as_ptr()) });
        assert_pair_eq(&format!("C arity({len}) vs C arity({equiv})"), small, big);

        let ctx = format!("row04 arity(len={len})");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) }, || unsafe { (p.rust.arity)(len, params.as_ptr()) });
    }
}

#[test]
fn row05_arity_negative_len_low_byte_unsigned() {
    let p = load();
    let mut rng = Rng::new(0xC005);
    let params: Vec<c_int> = (0..4).map(|_| rng.i32_mixed()).collect();

    // Low byte < 2 -> -1, and no dereference (NULL proves it).
    for len in [-256, -512, -65536, i32::MIN, -255] {
        let low = (len as u32 & 0xFF) as u8;
        if low < 2 {
            arity_null_expect_minus1(len);
        }
    }
    // i32::MIN = 0x80000000 -> low byte 0 -> -1
    arity_null_expect_minus1(i32::MIN);
    // -255 = 0xFFFFFF01 -> low byte 1 -> -1
    arity_null_expect_minus1(-255);

    // Low byte >= 2 -> dispatches; must match C exactly (unsigned compare).
    for len in [-1i32, -2, -3, -100, -254, -253, -252] {
        let low = (len as u32 & 0xFF) as c_int;
        assert!(low >= 2, "setup: len={len} low byte {low} should be >= 2");
        let ctx = format!("row05 arity(len={len}) low byte {low}");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) }, || unsafe { (p.rust.arity)(len, params.as_ptr()) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) });

        let equiv = measure_pair("phase_c.rs:99 equiv", || unsafe { (p.c.arity)(low, params.as_ptr()) });
        assert_pair_eq(
            &format!("row05 C arity({len}) must equal C arity({low}) (unsigned low byte)"),
            equiv,
            c,
        );
    }
}

#[test]
fn row06_arity_one_step_past_each_threshold() {
    let p = load();
    let mut rng = Rng::new(0xC006);
    let params: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();

    // The dispatch chain has no upper bound: 4, 5, 6, ... 255 all reach arity4.
    for len in [2, 3, 4, 5, 6, 7, 127, 128, 254, 255] {
        let ctx = format!("row06 arity(len={len})");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) }, || unsafe { (p.rust.arity)(len, params.as_ptr()) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) });

        if len >= 4 {
            let want = measure_pair("phase_c.rs:123 want", || unsafe {
                (p.c.arity4)(params[0], params[1], params[2], params[3])
            });
            assert_pair_eq(
                &format!("row06 C arity({len}) must equal C arity4 on first 4 params"),
                want,
                c,
            );
        }
    }
    // Boundaries of the rejection itself.
    arity_null_expect_minus1(0);
    arity_null_expect_minus1(1);
    let c2 = unsafe { (p.c.arity)(2, params.as_ptr()) };
    assert_ne!(c2, -1, "len=2 must NOT be rejected (or -1 is ambiguous here)");
}

// ===========================================================================
// Row 7: compare_allocations malloc-failure path
// ===========================================================================

#[test]
fn row07_compare_allocations_null_path_is_structurally_equivalent() {
    // The `ptr1 == NULL || ptr2 == NULL -> free both; return -1` branch cannot
    // be reached for a 4-byte request without exhausting the address space,
    // which would destabilise the test process. What IS observable is that
    // neither side ever returns the -1 sentinel on the success path, so a
    // caller can distinguish failure from success identically on both sides.
    let p = load();
    let mut rng = Rng::new(0xC007);
    for i in 0..2000 {
        let (v1, v2) = (rng.i32_mixed(), rng.i32_mixed());
        let ctx = format!("row07 iter {i}: compare_allocations({v1},{v2})");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.compare_allocations)(v1, v2) }, || unsafe { (p.rust.compare_allocations)(v1, v2) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.compare_allocations)(v1, v2) });
        let r = measure_pair(&ctx, || unsafe { (p.rust.compare_allocations)(v1, v2) });
        for got in c.iter().chain(r.iter()) {
            assert_ne!(
                *got, -1,
                "[row07] iter {i}: the -1 malloc-failure sentinel must not appear on the \
                 success path (val1={v1})"
            );
        }
    }
}

// ===========================================================================
// Rows 8–9: apply_bitmask default arm / out-of-range enum-style ints
// ===========================================================================

#[test]
fn row08_apply_bitmask_default_arm_is_identity() {
    let p = load();
    let mut rng = Rng::new(0xC008);
    // Values outside {0,1,2,3}, including the negative residues arity4 really
    // produces via `param1 % 4` when param1 < 0.
    let ops: Vec<c_int> = [-1, -2, -3, -4, 4, 5, 6, 100, -100]
        .iter()
        .copied()
        .chain((0..200).map(|_| {
            let mut o = rng.i32_mixed();
            if (0..=3).contains(&o) {
                o = o.wrapping_add(4);
            }
            o
        }))
        .collect();
    for (i, &op) in ops.iter().enumerate() {
        assert!(!(0..=3).contains(&op), "setup: op={op} must be out of range");
        for _ in 0..50 {
            let value = rng.i32_mixed();
            let c = unsafe { (p.c.apply_bitmask)(value, op) };
            let r = unsafe { (p.rust.apply_bitmask)(value, op) };
            assert_eq!(
                c, value,
                "[row08] i={i}: C default arm must be identity for op={op}, value={value}"
            );
            assert_eq!(c, r, "[row08] i={i}: apply_bitmask({value},{op}) C={c} Rust={r}");
        }
    }
}

#[test]
fn row09_apply_bitmask_out_of_range_enum_values_across_ffi() {
    let p = load();
    // A C enum accepts any int. These are the values a caller can legally pass
    // across the FFI boundary that have no matching `case`.
    let hostile: [c_int; 12] = [
        4,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        // 0x1_0000_0000 truncated to 32 bits is 0 -- but as an i32 argument the
        // caller can only pass the low 32 bits, so this is the honest form:
        (0x1_0000_0000u64 as u32) as c_int, // == 0, a VALID case: sanity anchor
        0x0000_0100,
        0x7FFF_FFFF,
        -0x8000_0000i64 as c_int,
        1 << 16,
        (1u32 << 31) as c_int,
    ];
    for value in [0, 1, -1, 0xFF, 0xAA, 0x55, i32::MAX, i32::MIN, 0x1234_5678] {
        for (i, &op) in hostile.iter().enumerate() {
            let c = unsafe { (p.c.apply_bitmask)(value, op) };
            let r = unsafe { (p.rust.apply_bitmask)(value, op) };
            assert_eq!(
                c, r,
                "[row09] hostile[{i}]: apply_bitmask(value={value}, operation={op}) \
                 C={c} Rust={r}"
            );
            if !(0..=3).contains(&op) {
                assert_eq!(c, value, "[row09] hostile[{i}]: op={op} must be identity");
            }
        }
    }
}

// ===========================================================================
// Rows 10–13: shift_array guard-false no-ops
// ===========================================================================

/// Compare the whole padded buffer after the call. For the guard-false rows the
/// buffer must come back bit-identical to its input on BOTH sides.
fn shift_noop_both(
    p: &harness::Pair,
    contents: &[c_int],
    size: c_int,
    positions: c_int,
    label: &str,
) {
    const PAD: usize = 4;
    let guard: c_int = 0x5A5A_5A5A;
    let build = || {
        let mut v = vec![guard; PAD];
        v.extend_from_slice(contents);
        v.extend(std::iter::repeat(guard).take(PAD));
        v
    };
    let original = build();
    let mut cbuf = build();
    let mut rbuf = build();
    unsafe {
        (p.c.shift_array)(cbuf.as_mut_ptr().add(PAD), size, positions);
        (p.rust.shift_array)(rbuf.as_mut_ptr().add(PAD), size, positions);
    }
    assert_eq!(
        cbuf, original,
        "[{label}] C must be a NO-OP for size={size} positions={positions}"
    );
    assert_eq!(
        cbuf, rbuf,
        "[{label}] size={size} positions={positions}\n  C   ={cbuf:?}\n  Rust={rbuf:?}"
    );
}

#[test]
fn row10_shift_array_positions_nonpositive_is_noop() {
    let p = load();
    let mut rng = Rng::new(0xC010);
    for _ in 0..1500 {
        let size = rng.range(1, 16);
        let contents: Vec<c_int> = (0..size as usize).map(|_| rng.i32_mixed()).collect();
        for positions in [0, -1, -2, -size, -1000, i32::MIN] {
            shift_noop_both(&p, &contents, size, positions, "row10 positions<=0");
        }
    }
    // And the null-pointer form: with positions <= 0 the C never dereferences,
    // so passing NULL is well-defined and must not fault on either side.
    unsafe {
        (p.c.shift_array)(std::ptr::null_mut(), 8, 0);
        (p.rust.shift_array)(std::ptr::null_mut(), 8, 0);
        (p.c.shift_array)(std::ptr::null_mut(), 8, -1);
        (p.rust.shift_array)(std::ptr::null_mut(), 8, -1);
        (p.c.shift_array)(std::ptr::null_mut(), 8, i32::MIN);
        (p.rust.shift_array)(std::ptr::null_mut(), 8, i32::MIN);
    }
}

#[test]
fn row11_shift_array_positions_ge_size_is_noop() {
    let p = load();
    let mut rng = Rng::new(0xC011);
    for _ in 0..1500 {
        let size = rng.range(1, 16);
        let contents: Vec<c_int> = (0..size as usize).map(|_| rng.i32_mixed()).collect();
        // positions == size is the subtle one: it must NOT zero the array.
        for positions in [size, size + 1, size + 100, i32::MAX] {
            shift_noop_both(&p, &contents, size, positions, "row11 positions>=size");
        }
    }
    // positions >= size never dereferences either -> NULL is well-defined.
    unsafe {
        (p.c.shift_array)(std::ptr::null_mut(), 4, 4);
        (p.rust.shift_array)(std::ptr::null_mut(), 4, 4);
        (p.c.shift_array)(std::ptr::null_mut(), 4, i32::MAX);
        (p.rust.shift_array)(std::ptr::null_mut(), 4, i32::MAX);
    }
}

#[test]
fn row12_shift_array_size_nonpositive_is_noop() {
    let p = load();
    let mut rng = Rng::new(0xC012);
    for _ in 0..1500 {
        let real = rng.range(1, 8) as usize;
        let contents: Vec<c_int> = (0..real).map(|_| rng.i32_mixed()).collect();
        for size in [0, -1, -2, -1000, i32::MIN] {
            for positions in [1, 2, 1000, i32::MAX, 0, -1] {
                shift_noop_both(&p, &contents, size, positions, "row12 size<=0");
            }
        }
    }
    unsafe {
        (p.c.shift_array)(std::ptr::null_mut(), 0, 1);
        (p.rust.shift_array)(std::ptr::null_mut(), 0, 1);
        (p.c.shift_array)(std::ptr::null_mut(), -1, 1);
        (p.rust.shift_array)(std::ptr::null_mut(), -1, 1);
    }
}

#[test]
fn row13_shift_array_both_negative_is_noop() {
    let p = load();
    let mut rng = Rng::new(0xC013);
    for _ in 0..1500 {
        let real = rng.range(1, 8) as usize;
        let contents: Vec<c_int> = (0..real).map(|_| rng.i32_mixed()).collect();
        let size = -rng.range(1, 1000);
        let positions = -rng.range(1, 1000);
        shift_noop_both(&p, &contents, size, positions, "row13 both negative");
        // The extreme corner: (size - positions) would overflow if evaluated.
        shift_noop_both(&p, &contents, i32::MIN, i32::MIN, "row13 INT_MIN/INT_MIN");
        shift_noop_both(&p, &contents, i32::MIN, -1, "row13 INT_MIN/-1");
    }
}

// ===========================================================================
// Rows 14–16: process_string guard
// ===========================================================================

fn ps_both_eq(p: &harness::Pair, bytes: &[u8], expect: c_int, label: &str) {
    let c = unsafe { (p.c.process_string)(bytes.as_ptr() as *const c_char) };
    let r = unsafe { (p.rust.process_string)(bytes.as_ptr() as *const c_char) };
    assert_eq!(c, expect, "[{label}] C process_string({bytes:?}) = {c}, want {expect}");
    assert_eq!(c, r, "[{label}] process_string({bytes:?}) C={c} Rust={r}");
}

#[test]
fn row14_process_string_empty_takes_second_return() {
    let p = load();
    ps_both_eq(&p, b"\0", 0, "row14 empty");
    // Buffer whose first byte is NUL but which is longer than one byte.
    ps_both_eq(&p, b"\0\0\0\0", 0, "row14 all-NUL");
}

#[test]
fn row15_process_string_leading_nul_ignores_rest() {
    let p = load();
    // The guard tests only *str, so strlen is never called and the trailing
    // bytes are irrelevant -- result is 0, not 3.
    ps_both_eq(&p, b"\0abc\0", 0, "row15 leading NUL then abc");
    let mut rng = Rng::new(0xC015);
    for i in 0..1000 {
        let n = rng.range(1, 32) as usize;
        let mut buf = vec![0u8];
        buf.extend((0..n).map(|_| rng.range(1, 255) as u8));
        buf.push(0);
        ps_both_eq(&p, &buf, 0, &format!("row15 iter {i}"));
    }
}

#[test]
fn row16_process_string_high_bit_bytes_are_truthy() {
    let p = load();
    // `char` is signed on x86-64, so bytes 0x80..0xFF make `*str` negative --
    // still non-zero, so the guard passes and strlen runs.
    for b in 0x80u16..=0xFF {
        let buf = [b as u8, 0x80, 0xFF, 0];
        ps_both_eq(&p, &buf, 3, &format!("row16 first byte {b:#04x}"));
    }
    let mut rng = Rng::new(0xC016);
    for i in 0..1000 {
        let n = rng.range(1, 40) as usize;
        let mut buf: Vec<u8> = (0..n).map(|_| rng.range(0x80, 0xFF) as u8).collect();
        buf.push(0);
        ps_both_eq(&p, &buf, n as c_int, &format!("row16 iter {i}"));
    }
}

// ===========================================================================
// Rows 17–22: arity4 guards and signed-arithmetic edges
// ===========================================================================

fn a4_pair(p: &harness::Pair, q: [c_int; 4]) -> ([c_int; 2], [c_int; 2]) {
    let c = measure_pair("phase_c.rs:420 c", || unsafe { (p.c.arity4)(q[0], q[1], q[2], q[3]) });
    let r = measure_pair("phase_c.rs:421 r", || unsafe { (p.rust.arity4)(q[0], q[1], q[2], q[3]) });
    (c, r)
}

#[test]
fn row17_arity4_p3_zero_skips_rescale_entirely() {
    let p = load();
    let mut rng = Rng::new(0xC017);
    for i in 0..1500 {
        let q = [rng.i32_mixed(), rng.i32_mixed(), 0, 0];
        let (c, r) = a4_pair(&p, q);
        assert_pair_eq(&format!("row17 iter {i}: arity4{q:?}"), c, r);
        // The guard is a SKIP, not a multiply-by-zero: the result is generally
        // non-zero, which is what distinguishes the two readings.
        let nonzero = measure_pair("phase_c.rs:435 nonzero", || unsafe { (p.c.arity4)(q[0], q[1], 0, 0) });
        assert_pair_eq(&format!("row17 iter {i}: C self-consistency"), c, nonzero);
    }
    // Concrete anchor: with p3 == 0 the value survives; a multiply would zero it.
    let (c, r) = a4_pair(&p, [4, 0, 0, 0]);
    assert_pair_eq("row17 anchor arity4(4,0,0,0)", c, r);
    assert!(
        c[0] != 0 || c[1] != 0,
        "row17: p3==0 must SKIP the rescale, not multiply by 0 (got {c:?})"
    );
}

#[test]
fn row18_arity4_p4_zero_skips_add() {
    let p = load();
    let mut rng = Rng::new(0xC018);
    for i in 0..1500 {
        let mut p3 = rng.i32_mixed();
        if p3 == 0 {
            p3 = 5;
        }
        let q = [rng.i32_mixed(), rng.i32_mixed(), p3, 0];
        let (c, r) = a4_pair(&p, q);
        assert_pair_eq(&format!("row18 iter {i}: arity4{q:?} (p4=0)"), c, r);
    }
}

#[test]
fn row19_arity4_multiply_overflow_wraps() {
    let p = load();
    // Force a large `result` then a large `param3`, so `result * param3`
    // overflows a signed 32-bit int. GCC emits a wrapping `imul`; Rust must
    // wrap too and must not panic.
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1 << 30, -(1 << 30), 0x5A5A_5A5A];
    for &p1 in &extremes {
        for &p2 in &extremes {
            for &p3 in &extremes {
                let q = [p1, p2, p3, 0];
                let (c, r) = a4_pair(&p, q);
                assert_pair_eq(&format!("row19 overflow arity4{q:?}"), c, r);
            }
        }
    }
}

#[test]
fn row20_arity4_add_overflow_wraps() {
    let p = load();
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for &p1 in &extremes {
        for &p2 in &extremes {
            for &p4 in &extremes {
                for &p3 in &[0, 1, -1, i32::MAX, i32::MIN] {
                    let q = [p1, p2, p3, p4];
                    let (c, r) = a4_pair(&p, q);
                    assert_pair_eq(&format!("row20 add-overflow arity4{q:?}"), c, r);
                }
            }
        }
    }
}

#[test]
fn row21_arity4_negative_division_truncates_toward_zero() {
    let p = load();
    let mut rng = Rng::new(0xC021);
    // Drive `result * param3` negative and check the `/ 100` rounding
    // direction: C truncates toward zero, not toward negative infinity.
    for i in 0..3000 {
        let p1 = rng.i32_small(50);
        let p2 = rng.i32_small(50);
        let p3 = if i % 2 == 0 { -rng.range(1, 1000) } else { rng.range(1, 1000) };
        let q = [p1, p2, p3, 0];
        let (c, r) = a4_pair(&p, q);
        assert_pair_eq(&format!("row21 iter {i}: arity4{q:?}"), c, r);
    }
    // Small-magnitude products where floor vs truncate actually differ.
    for p3 in [-1, -2, -3, -7, -99, -100, -101, 1, 2, 3, 7, 99, 100, 101] {
        for p1 in -8..=8 {
            for p2 in -8..=8 {
                let q = [p1, p2, p3, 0];
                let (c, r) = a4_pair(&p, q);
                assert_pair_eq(&format!("row21 rounding arity4{q:?}"), c, r);
            }
        }
    }
}

#[test]
fn row22_arity4_param1_int_min_modulo() {
    let p = load();
    // INT_MIN % 4 == 0 in C (no trap: the divisor is the literal 4, so the
    // INT_MIN / -1 overflow case cannot arise). Must select case 0.
    for &p1 in &[i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3, i32::MIN + 4] {
        assert!(
            (-3..=0).contains(&(p1 % 4)),
            "setup: {p1} % 4 = {} outside expected range",
            p1 % 4
        );
        for &p2 in &[0, 1, -1, i32::MAX, i32::MIN] {
            for &p3 in &[0, 1, -1, 100, -100, i32::MAX, i32::MIN] {
                for &p4 in &[0, 1, -1, i32::MAX, i32::MIN] {
                    let q = [p1, p2, p3, p4];
                    let (c, r) = a4_pair(&p, q);
                    assert_pair_eq(&format!("row22 INT_MIN modulo arity4{q:?}"), c, r);
                }
            }
        }
    }
}

// ===========================================================================
// Row 23: init_matrix has no validation at all
// ===========================================================================

#[test]
fn row23_init_matrix_unconditionally_writes_twelve_cells() {
    let p = load();
    let mut rng = Rng::new(0xC023);
    for i in 0..2000 {
        let poison = rng.i32_mixed();
        let mut cbuf = vec![poison; 12 + 8];
        let mut rbuf = vec![poison; 12 + 8];
        unsafe {
            (p.c.init_matrix)(cbuf.as_mut_ptr() as *mut [c_int; 4]);
            (p.rust.init_matrix)(rbuf.as_mut_ptr() as *mut [c_int; 4]);
        }
        assert_eq!(cbuf, rbuf, "[row23] iter {i}: init_matrix buffers differ");
        assert_eq!(
            &cbuf[0..12],
            &(1..=12).collect::<Vec<c_int>>()[..],
            "[row23] iter {i}: C must write 1..12 row-major"
        );
        assert_eq!(
            &cbuf[12..],
            &vec![poison; 8][..],
            "[row23] iter {i}: C must not write past the 3x4 matrix"
        );
    }
}

// ===========================================================================
// Row 24: generic zero / oversized lengths
// ===========================================================================

#[test]
fn row24_generic_zero_and_oversized_lengths() {
    let p = load();
    let mut rng = Rng::new(0xC024);

    // arity with len = 255: dispatches to arity4 and reads ONLY params[0..4].
    // A buffer of exactly 4 makes any over-read a real out-of-bounds access.
    for i in 0..500 {
        let params = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        let ctx = format!("row24 iter {i}: arity(255, [4 ints])");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity)(255, params.as_ptr()) }, || unsafe { (p.rust.arity)(255, params.as_ptr()) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity)(255, params.as_ptr()) });
        let want = measure_pair("phase_c.rs:594 want", || unsafe { (p.c.arity4)(params[0], params[1], params[2], params[3]) });
        assert_pair_eq(&format!("row24 iter {i}: arity(255) reads only 4"), want, c);
    }

    // shift_array with size = INT_MAX is NOT executed destructively: with
    // positions >= size (or <= 0) the guard rejects it, so it is safe and must
    // be a no-op on both sides. The huge-memmove case is genuine UB against a
    // small buffer and is excluded by design (see ERRORS.md non-rows).
    let contents: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();
    for positions in [0, -1, i32::MAX, i32::MIN] {
        shift_noop_both(&p, &contents, i32::MAX, positions, "row24 size=INT_MAX guarded");
    }

    // process_string with a zero-length (immediately-NUL) buffer.
    ps_both_eq(&p, b"\0", 0, "row24 zero length");

    // arity with len = 0 and a NON-null but zero-length-relevant buffer: still
    // rejected before any read.
    let empty: [c_int; 0] = [];
    let c = unsafe { (p.c.arity)(0, empty.as_ptr()) };
    let r = unsafe { (p.rust.arity)(0, empty.as_ptr()) };
    assert_eq!(c, -1);
    assert_eq!(c, r, "row24 arity(0, empty buffer) C={c} Rust={r}");
}

// ===========================================================================
// Extra generic FFI-boundary coverage required by the Phase C instructions
// ===========================================================================

#[test]
fn extra_all_int_args_exhaustive_hostile_values() {
    // Every exported int-taking function, fed the full set of boundary values a
    // hostile external caller can put in a register.
    let p = load();
    const H: [c_int; 13] = [
        0, 1, -1, 2, 3, 4, 255, 256, -256, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];
    for &a in &H {
        for &b in &H {
            let c = unsafe { (p.c.apply_bitmask)(a, b) };
            let r = unsafe { (p.rust.apply_bitmask)(a, b) };
            assert_eq!(c, r, "apply_bitmask({a},{b}) C={c} Rust={r}");

            assert_alloc_diff(
                &format!("compare_allocations({a},{b})"),
                || unsafe { (p.c.compare_allocations)(a, b) },
                || unsafe { (p.rust.compare_allocations)(a, b) },
            );

            assert_alloc_diff(
                &format!("arity2({a},{b})"),
                || unsafe { (p.c.arity2)(a, b) },
                || unsafe { (p.rust.arity2)(a, b) },
            );

            for &d in &H {
                assert_alloc_diff(
                    &format!("arity3({a},{b},{d})"),
                    || unsafe { (p.c.arity3)(a, b, d) },
                    || unsafe { (p.rust.arity3)(a, b, d) },
                );
            }
        }
    }
}

#[test]
fn extra_arity_every_low_byte_value() {
    // All 256 distinct truncated lengths, reached from three different i32
    // encodings each (n, n+256, n-256), asserting the C and Rust agree and that
    // the three encodings are indistinguishable on the C side.
    let p = load();
    let mut rng = Rng::new(0xCFFF);
    let params: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();
    for low in 0..256i32 {
        let encodings = [low, low + 256, low + 65536, low - 256, low.wrapping_sub(65536)];
        let mut baseline: Option<[c_int; 2]> = None;
        for &len in &encodings {
            if low < 2 {
                // Rejected without dereference -- prove it with NULL.
                let c = unsafe { (p.c.arity)(len, std::ptr::null()) };
                let r = unsafe { (p.rust.arity)(len, std::ptr::null()) };
                assert_eq!(c, -1, "arity(len={len}) low byte {low} must reject");
                assert_eq!(c, r, "arity(len={len}, NULL) C={c} Rust={r}");
                continue;
            }
            let ctx = format!("arity(len={len}) low byte {low}");
            // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
            assert_alloc_diff(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) }, || unsafe { (p.rust.arity)(len, params.as_ptr()) });
            // Stable phase-independent C measurement for the C-vs-C checks below.
            let c = measure_pair(&ctx, || unsafe { (p.c.arity)(len, params.as_ptr()) });
            match baseline {
                None => baseline = Some(c),
                Some(b) => assert_pair_eq(
                    &format!("C arity: encoding {len} must match low byte {low}"),
                    b,
                    c,
                ),
            }
        }
    }
}
