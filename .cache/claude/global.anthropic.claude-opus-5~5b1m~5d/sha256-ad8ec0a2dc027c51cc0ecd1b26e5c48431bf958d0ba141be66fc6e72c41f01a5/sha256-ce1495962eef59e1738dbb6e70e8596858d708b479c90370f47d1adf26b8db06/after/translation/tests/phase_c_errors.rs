//! Phase C — error / rejection-path differential tests, one per row of
//! `ERRORS.md`.
//!
//! Every test calls BOTH shared objects and asserts the *same* rejection: the
//! same sentinel (`-1`), the same silent no-op (byte-identical buffer), or the
//! same fatal signal — never merely "both failed somehow".
//!
//! Two row groups need special machinery:
//!
//! * **null-pointer dereference** (E10, E27, E30, E32): the C code dereferences
//!   before checking, so the only observable result is a fatal signal. The test
//!   re-executes this test binary as a child process (`child_worker`) once per
//!   implementation and compares the terminating signal.
//! * **allocation failure** (E12–E15): `malloc(sizeof(int))` never fails in
//!   practice, so a failing `malloc` is interposed with `LD_PRELOAD` in a child
//!   process. The shim is generated and compiled by this test.

mod common;

use common::{both, normalize_allocator, AllocOrder, Rng};
use std::ffi::{c_char, c_int};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

const GUARD: i32 = 0x5A5A_5A5A;

// ---------------------------------------------------------------------------
// helpers
//
// Calls that reach `malloc` are preceded by `normalize_allocator`, which forces
// a known address ordering inside `compare_allocations` and makes the result
// deterministic; see the module comment of `phase_b_valid.rs`. Each such call is
// made under both orderings, and the two results are returned as a pair.
// ---------------------------------------------------------------------------

/// `arity4` under both address orderings.
fn pair4(
    f: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    a: c_int,
    b: c_int,
    c: c_int,
    d: c_int,
) -> (c_int, c_int) {
    let mut out = [0; 2];
    for (k, order) in AllocOrder::both().into_iter().enumerate() {
        normalize_allocator(order);
        out[k] = unsafe { f(a, b, c, d) };
    }
    (out[0], out[1])
}

/// `arity` under both address orderings.
fn pair_arity(
    f: unsafe extern "C" fn(c_int, *const c_int) -> c_int,
    len: c_int,
    params: *const c_int,
) -> (c_int, c_int) {
    let mut out = [0; 2];
    for (k, order) in AllocOrder::both().into_iter().enumerate() {
        normalize_allocator(order);
        out[k] = unsafe { f(len, params) };
    }
    (out[0], out[1])
}

/// `arity` on both libraries; returns the (identical) result pair.
fn diff_arity(row: &str, len: c_int, params: *const c_int) -> (c_int, c_int) {
    let (c, r) = both();
    let cc = pair_arity(c.arity, len, params);
    let rr = pair_arity(r.arity, len, params);
    assert_eq!(cc, rr, "{row}: arity(len={len}) mismatch C={cc:?} Rust={rr:?}");
    cc
}

/// `shift_array` on identical guarded buffers; assert byte-identical results and
/// return whether the buffer changed at all.
fn diff_shift_unchanged(row: &str, contents: &[i32], size: c_int, positions: c_int) -> bool {
    let (c, r) = both();
    let pad = 8;
    let mut bc: Vec<i32> = Vec::with_capacity(contents.len() + 2 * pad);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    bc.extend_from_slice(contents);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    let before = bc.clone();
    let mut br = bc.clone();
    unsafe {
        (c.shift_array)(bc.as_mut_ptr().add(pad), size, positions);
        (r.shift_array)(br.as_mut_ptr().add(pad), size, positions);
    }
    assert_eq!(
        bc, br,
        "{row}: shift_array(size={size}, positions={positions}) buffer mismatch\n\
         C={bc:?}\nRust={br:?}"
    );
    bc == before
}

// ===========================================================================
// E1 / E2 / E3 — arity rejects len < 2
// ===========================================================================

#[test]
fn e1_arity_len0() {
    let mut rng = Rng::new(0xE1);
    for _ in 0..common::ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let got = diff_arity("E1", 0, params.as_ptr());
        assert_eq!(got, (-1, -1), "E1: arity(0, ..) must return -1");
    }
}

#[test]
fn e2_arity_len1() {
    let mut rng = Rng::new(0xE2);
    for _ in 0..common::ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let got = diff_arity("E2", 1, params.as_ptr());
        assert_eq!(got, (-1, -1), "E2: arity(1, ..) must return -1");
    }
}

#[test]
fn e3_arity_len_lt2_null_params() {
    // The guard short-circuits before any load, so a NULL buffer is safe here:
    // both libraries must return -1 rather than crash.
    for len in [0, 1, 256, 257, 512, 0x100, i32::MIN, -256, -255] {
        let truncated = (len as u32 & 0xff) as u8;
        if truncated >= 2 {
            continue;
        }
        let got = diff_arity("E3", len, std::ptr::null());
        assert_eq!(got, (-1, -1), "E3: arity({len}, NULL) must return -1");
    }
}

// ===========================================================================
// E4 / E5 / E6 — len outside `unsigned char` truncates into the reject branch
// ===========================================================================

#[test]
fn e4_arity_len_truncates_to_lt2() {
    let mut rng = Rng::new(0xE4);
    let params = [
        rng.interesting_i32(),
        rng.interesting_i32(),
        rng.interesting_i32(),
        rng.interesting_i32(),
    ];
    // Values whose low byte is 0 or 1 -> rejected with -1.
    let mut cases: Vec<i32> = vec![
        256,
        257,
        512,
        513,
        65536,
        65537,
        0x0001_0000,
        0x0001_0001,
        i32::MIN,      // 0x8000_0000 -> low byte 0
        i32::MIN + 1,  // 0x8000_0001 -> low byte 1
        -256,          // 0xFFFF_FF00 -> low byte 0
        -255,          // 0xFFFF_FF01 -> low byte 1
        0x7FFF_FF00,
        0x7FFF_FF01,
    ];
    // Plus a randomized sweep of arbitrary ints with low byte 0 or 1.
    let mut rng2 = Rng::new(0xE44);
    for _ in 0..200 {
        let hi = rng2.next_i32() & !0xff;
        cases.push(hi);
        cases.push(hi | 1);
    }
    for len in cases {
        let got = diff_arity("E4/E5/E6", len, params.as_ptr());
        assert_eq!(
            got,
            (-1, -1),
            "E4: arity({len}) truncates to {} and must return -1",
            (len as u32 & 0xff) as u8
        );
    }
}

// ===========================================================================
// E7 / E8 / E9 — len that is *not* rejected (unsigned semantics)
// ===========================================================================

#[test]
fn e5_arity_negative_len_is_unsigned() {
    let mut rng = Rng::new(0xE5);
    for _ in 0..common::ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        // -1 -> 0xFF = 255, which is NOT < 2 when compared as an unsigned char,
        // so the `else` branch runs: this must NOT be rejected.
        let got = diff_arity("E7", -1, params.as_ptr());
        assert_ne!(
            got.0, -1,
            "E7: arity(-1) must not take the reject branch unless arity4 legitimately returns -1"
        );
        // Cross-check against a direct arity4 call on the same parameters.
        let (c, _r) = both();
        let direct = pair4(c.arity4, params[0], params[1], params[2], params[3]);
        assert_eq!(
            got, direct,
            "E7: arity(-1) must be equivalent to arity4(params[0..4])"
        );
    }
}

#[test]
fn e6_arity_len_ge4_dispatch() {
    let mut rng = Rng::new(0xE6);
    for _ in 0..64 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for len in [4, 5, 6, 100, 254, 255, i32::MAX, 0x7FFF_FFFF, -1, -2] {
            let truncated = (len as u32 & 0xff) as u8;
            if truncated < 4 {
                continue;
            }
            let got = diff_arity("E8/E9", len, params.as_ptr());
            let (c, _r) = both();
            let direct = pair4(c.arity4, params[0], params[1], params[2], params[3]);
            assert_eq!(got, direct, "E8: arity({len}) must dispatch to arity4");
        }
    }
}

#[test]
fn e8_arity_reads_past_short_buffer() {
    // E11: `len == 4` with only two meaningful elements. The C code has no
    // bounds check and reads params[2]/params[3] regardless; both libraries must
    // read the same memory. The buffer is over-allocated so the test itself
    // stays memory-safe while still exercising the missing check.
    let mut rng = Rng::new(0xE8);
    for _ in 0..common::ITERS {
        let buf = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(), // "past the end" of the caller's 2 elements
            rng.interesting_i32(),
        ];
        let got = diff_arity("E11", 4, buf.as_ptr());
        let (c, _r) = both();
        let direct = pair4(c.arity4, buf[0], buf[1], buf[2], buf[3]);
        assert_eq!(got, direct, "E11: out-of-range reads must match arity4");
    }
}

// ===========================================================================
// E16 .. E19 — apply_bitmask default: branch (out-of-range "enum" values)
// ===========================================================================

#[test]
fn e11_apply_bitmask_out_of_range_operation() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE11);
    for _ in 0..common::ITERS {
        let value = rng.interesting_i32();
        for op in [4, 5, -1, -2, i32::MIN, i32::MAX, 0x1_0000, -0x1_0000] {
            let cv = unsafe { (c.apply_bitmask)(value, op) };
            let rv = unsafe { (r.apply_bitmask)(value, op) };
            assert_eq!(
                cv, rv,
                "E16/E17/E18: apply_bitmask({value}, {op}) mismatch C={cv} Rust={rv}"
            );
            assert_eq!(
                cv, value,
                "E16/E17/E18: default: branch must return `value` unchanged"
            );
        }
    }
}

#[test]
fn e12_apply_bitmask_exhaustive_operation_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE12);
    // Every operation in -300..=300 (covers all valid cases and their
    // neighbours) times several values.
    for _ in 0..8 {
        let value = rng.interesting_i32();
        for op in -300..=300 {
            let cv = unsafe { (c.apply_bitmask)(value, op) };
            let rv = unsafe { (r.apply_bitmask)(value, op) };
            assert_eq!(cv, rv, "E19: apply_bitmask({value}, {op}) mismatch");
            if !(0..=3).contains(&op) {
                assert_eq!(cv, value, "E19: apply_bitmask({value}, {op}) must pass through");
            }
        }
    }
    // Randomized sweep over the whole i32 range for `operation`.
    for _ in 0..4000 {
        let value = rng.interesting_i32();
        let op = rng.next_i32();
        let cv = unsafe { (c.apply_bitmask)(value, op) };
        let rv = unsafe { (r.apply_bitmask)(value, op) };
        assert_eq!(cv, rv, "E19: apply_bitmask({value}, {op}) mismatch");
    }
}

// ===========================================================================
// E20 .. E26, E28 — shift_array guards
// ===========================================================================

#[test]
fn e13_shift_array_rejects_nonpositive_positions() {
    let mut rng = Rng::new(0xE13);
    for _ in 0..common::ITERS {
        let contents: [i32; 8] = std::array::from_fn(|_| rng.interesting_i32());
        for positions in [0, -1, -2, -8, i32::MIN, -(rng.range(1000) as i32) - 1] {
            for size in [0, 1, 4, 8, i32::MAX, i32::MIN] {
                let unchanged = diff_shift_unchanged("E20/E21", &contents, size, positions);
                assert!(
                    unchanged,
                    "E20/E21: positions={positions} <= 0 must be a no-op (size={size})"
                );
            }
        }
    }
}

#[test]
fn e14_shift_array_rejects_positions_ge_size() {
    let mut rng = Rng::new(0xE14);
    for _ in 0..common::ITERS {
        let contents: [i32; 8] = std::array::from_fn(|_| rng.interesting_i32());
        for size in [1, 2, 4, 8] {
            for positions in [size, size + 1, size + 100, i32::MAX] {
                let unchanged = diff_shift_unchanged("E22/E23", &contents, size, positions);
                assert!(
                    unchanged,
                    "E22/E23: positions={positions} >= size={size} must be a no-op"
                );
            }
        }
    }
}

#[test]
fn e15_shift_array_zero_or_negative_size() {
    let mut rng = Rng::new(0xE15);
    for _ in 0..common::ITERS {
        let contents: [i32; 8] = std::array::from_fn(|_| rng.interesting_i32());
        for size in [0, -1, -8, i32::MIN] {
            for positions in [1, 2, 8, i32::MAX, 0, -1] {
                let unchanged = diff_shift_unchanged("E24/E25", &contents, size, positions);
                assert!(
                    unchanged,
                    "E24/E25: size={size} must be a no-op (positions={positions})"
                );
            }
        }
    }
}

#[test]
fn e16_shift_array_null_ptr_guarded() {
    // E26: NULL is fine as long as the guard rejects: the pointer is never used.
    let (c, r) = both();
    for (size, positions) in [
        (0, 0),
        (0, 1),
        (4, 0),
        (4, -1),
        (4, 4),
        (4, 5),
        (1, 1),
        (i32::MIN, 1),
        (i32::MAX, i32::MAX),
        (i32::MAX, 0),
    ] {
        unsafe {
            (c.shift_array)(std::ptr::null_mut(), size, positions);
            (r.shift_array)(std::ptr::null_mut(), size, positions);
        }
    }
    // Reaching here means neither library touched the NULL pointer.
}

#[test]
fn e17_shift_array_size_larger_than_buffer() {
    // E28: `size` bigger than the caller's array. The C code has no bounds
    // check and shifts `size` elements anyway. A generous guard region absorbs
    // the overrun so that both libraries can be compared byte for byte.
    let mut rng = Rng::new(0xE17);
    for _ in 0..common::ITERS {
        let logical = 4usize;
        let contents: [i32; 4] = std::array::from_fn(|_| rng.interesting_i32());
        for extra in 1..=4i32 {
            // pad = 8 words on each side; size = logical + extra <= 8.
            let unchanged =
                diff_shift_unchanged("E28", &contents, logical as i32 + extra, 1 + extra / 2);
            assert!(
                !unchanged,
                "E28: an in-range `positions` must actually shift (size={})",
                logical as i32 + extra
            );
        }
    }
}

// ===========================================================================
// E29 / E31 — process_string
// ===========================================================================

#[test]
fn e18_process_string_empty() {
    let (c, r) = both();
    // Empty string: `if (*str)` is false -> return 0, *not* strlen.
    for trailing in [0u8, 1, 0x41, 0xFF] {
        let buf: [c_char; 4] = [0, trailing as c_char, trailing as c_char, 0];
        let cv = unsafe { (c.process_string)(buf.as_ptr()) };
        let rv = unsafe { (r.process_string)(buf.as_ptr()) };
        assert_eq!(cv, rv, "E29: process_string(\"\") mismatch");
        assert_eq!(cv, 0, "E29: process_string(\"\") must return 0");
    }
}

#[test]
fn e19_process_string_unterminated() {
    // E31: no length parameter exists, so `strlen` runs past the "logical" end
    // of the caller's data into whatever follows. Both libraries must agree on
    // the length they compute. The NUL is placed inside an over-allocated
    // buffer so the test stays memory-safe.
    let (c, r) = both();
    let mut rng = Rng::new(0xE19);
    for _ in 0..common::ITERS {
        let logical = 1 + rng.range(16) as usize;
        let total = logical + 1 + rng.range(32) as usize;
        let mut buf: Vec<c_char> = (0..total)
            .map(|_| ((rng.range(255) + 1) as u8) as c_char)
            .collect();
        buf.push(0); // terminator far past the logical end
        let cv = unsafe { (c.process_string)(buf.as_ptr()) };
        let rv = unsafe { (r.process_string)(buf.as_ptr()) };
        assert_eq!(cv, rv, "E31: unterminated process_string mismatch");
        assert_eq!(
            cv, total as i32,
            "E31: strlen must run to the real terminator"
        );
    }
}

// ===========================================================================
// E33 — init_matrix has no size parameter
// ===========================================================================

#[test]
fn e20_init_matrix_writes_exactly_12() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE20);
    for _ in 0..common::ITERS {
        let pad = 8usize;
        let mut bc: Vec<i32> = (0..pad + 12 + pad).map(|_| rng.interesting_i32()).collect();
        let before = bc.clone();
        let mut br = bc.clone();
        unsafe {
            (c.init_matrix)(bc.as_mut_ptr().add(pad));
            (r.init_matrix)(br.as_mut_ptr().add(pad));
        }
        assert_eq!(bc, br, "E33: init_matrix mismatch");
        assert_eq!(
            &bc[pad..pad + 12],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            "E33: matrix contents"
        );
        assert_eq!(&bc[..pad], &before[..pad], "E33: wrote before the buffer");
        assert_eq!(
            &bc[pad + 12..],
            &before[pad + 12..],
            "E33: wrote past 12 ints (no size parameter exists, but it must be exactly 12)"
        );
    }
}

// ===========================================================================
// E34 / E35 / E36 / E37 — arithmetic edge cases inside arity4
// ===========================================================================

#[test]
fn e21_arity4_overflow_wraps() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE21);
    for _ in 0..common::ITERS {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        // E34: result * param3 overflows int.
        for p3 in [i32::MIN, i32::MAX, 1 << 30, -(1 << 30), 0x5555_5555] {
            let cc = pair4(c.arity4, p1, p2, p3, 0);
            let rr = pair4(r.arity4, p1, p2, p3, 0);
            assert_eq!(cc, rr, "E34: arity4({p1},{p2},{p3},0) overflow mismatch");
        }
        // E35: result + param4 overflows int.
        for p4 in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
            let cc = pair4(c.arity4, p1, p2, 0, p4);
            let rr = pair4(r.arity4, p1, p2, 0, p4);
            assert_eq!(cc, rr, "E35: arity4({p1},{p2},0,{p4}) overflow mismatch");
            let cc = pair4(c.arity4, p1, p2, i32::MAX, p4);
            let rr = pair4(r.arity4, p1, p2, i32::MAX, p4);
            assert_eq!(cc, rr, "E35: arity4({p1},{p2},MAX,{p4}) overflow mismatch");
        }
    }
}

#[test]
fn e22_arity4_negative_modulo_hits_default() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE22);
    // E36: `param1 % 4` uses C truncating remainder, so a negative param1 gives
    // a negative selector that matches no `case` -> default: (value unchanged).
    for p1 in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        -1,
        -2,
        -3,
        -4,
        -5,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 2,
        i32::MAX - 3,
    ] {
        for _ in 0..32 {
            let p2 = rng.interesting_i32();
            let cc = pair4(c.arity4, p1, p2, 0, 0);
            let rr = pair4(r.arity4, p1, p2, 0, 0);
            assert_eq!(cc, rr, "E36: arity4({p1},{p2},0,0) mismatch");
        }
    }
}

#[test]
fn e23_compare_allocations_nonpositive_val1() {
    // E37: `*uninit_ptr > 0` false -> the +10 bonus is skipped.
    let (c, r) = both();
    let mut rng = Rng::new(0xE23);
    for _ in 0..common::ITERS {
        for v1 in [0, -1, i32::MIN, -(rng.range(1000) as i32) - 1] {
            let v2 = rng.interesting_i32();
            for order in AllocOrder::both() {
                normalize_allocator(order);
                let cv = unsafe { (c.compare_allocations)(v1, v2) };
                normalize_allocator(order);
                let rv = unsafe { (r.compare_allocations)(v1, v2) };
                assert_eq!(
                    cv, rv,
                    "E37: compare_allocations({v1},{v2}) [{order:?}] C={cv} Rust={rv}"
                );
                // No +10 bonus: the result is exactly the branch number.
                assert_eq!(
                    cv,
                    order.expected_branch(),
                    "E37: bonus must be skipped for val1={v1} [{order:?}], got {cv}"
                );
            }
        }
    }
}

// ===========================================================================
// E38 — misaligned pointers (generic FFI boundary: C never checks alignment)
// ===========================================================================

/// The C code loads/stores `int`s through caller-supplied pointers with plain
/// `mov` instructions, which tolerate misalignment on x86-64. Nothing in the API
/// requires alignment, so a caller may legitimately pass a misaligned buffer;
/// both libraries must behave identically (in particular the Rust side must not
/// insert an alignment assertion).
#[test]
fn e25_misaligned_pointers() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE25);
    for _ in 0..64 {
        for off in 1..4usize {
            // ---- arity / arity dispatch through a misaligned params array ---
            let mut raw = [0u8; 64];
            for b in raw.iter_mut() {
                *b = (rng.range(256)) as u8;
            }
            let params = unsafe { raw.as_ptr().add(off) as *const c_int };
            assert_ne!(params as usize % 4, 0, "test setup: expected misalignment");
            for len in [2, 3, 4, 7] {
                let cc = pair_arity(c.arity, len, params);
                let rr = pair_arity(r.arity, len, params);
                assert_eq!(
                    cc, rr,
                    "E38: arity({len}) on a misaligned buffer (off={off}) C={cc:?} Rust={rr:?}"
                );
            }

            // ---- init_matrix into a misaligned buffer ----------------------
            let mut bc = [0u8; 128];
            for b in bc.iter_mut() {
                *b = (rng.range(256)) as u8;
            }
            let mut br = bc;
            unsafe {
                (c.init_matrix)(bc.as_mut_ptr().add(off) as *mut c_int);
                (r.init_matrix)(br.as_mut_ptr().add(off) as *mut c_int);
            }
            assert_eq!(bc, br, "E38: init_matrix on a misaligned buffer (off={off})");

            // ---- shift_array on a misaligned buffer ------------------------
            let mut sc = [0u8; 128];
            for b in sc.iter_mut() {
                *b = (rng.range(256)) as u8;
            }
            let mut sr = sc;
            let size = 1 + (rng.range(8) as c_int);
            let positions = rng.range(8) as c_int;
            unsafe {
                (c.shift_array)(sc.as_mut_ptr().add(off) as *mut c_int, size, positions);
                (r.shift_array)(sr.as_mut_ptr().add(off) as *mut c_int, size, positions);
            }
            assert_eq!(
                sc, sr,
                "E38: shift_array(size={size}, positions={positions}) misaligned (off={off})"
            );

            // ---- process_string on an odd address (char has no alignment) --
            let mut pc = [1u8; 32];
            pc[16 + off] = 0;
            let cv = unsafe { (c.process_string)(pc.as_ptr().add(off) as *const c_char) };
            let rv = unsafe { (r.process_string)(pc.as_ptr().add(off) as *const c_char) };
            assert_eq!(cv, rv, "E38: process_string at odd offset {off}");
        }
    }
}

// ===========================================================================
// E10 / E27 / E30 / E32 — null-pointer dereference: same fatal signal
// ===========================================================================

/// Scenarios executed in a child process, keyed by `CRASH_SCENARIO`.
const CRASH_SCENARIOS: &[&str] = &[
    "arity_null_len2",   // E10
    "arity_null_len3",   // E10
    "arity_null_len4",   // E10
    "arity_null_len255", // E10 (len = -1 truncates to 255)
    "shift_array_null",  // E27
    "process_string_null", // E30
    "init_matrix_null",  // E32
];

#[test]
fn e7_crash_parity() {
    for scenario in CRASH_SCENARIOS {
        let c = run_child(&[("CRASH_SCENARIO", scenario), ("CRASH_IMPL", "c")], &[]);
        let r = run_child(&[("CRASH_SCENARIO", scenario), ("CRASH_IMPL", "rust")], &[]);
        assert_eq!(
            (c.signal, c.code),
            (r.signal, r.code),
            "{scenario}: C died with signal={:?} code={:?}, Rust with signal={:?} code={:?}\n\
             C stderr: {}\nRust stderr: {}",
            c.signal,
            c.code,
            r.signal,
            r.code,
            c.stderr,
            r.stderr
        );
        assert_eq!(
            c.signal,
            Some(libc_sigsegv()),
            "{scenario}: expected SIGSEGV, got signal={:?} code={:?} stderr={}",
            c.signal,
            c.code,
            c.stderr
        );
    }
}

fn libc_sigsegv() -> i32 {
    11
}

struct ChildResult {
    signal: Option<i32>,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Re-execute this test binary, running only the (ignored) `child_worker` test.
fn run_child(env: &[(&str, &str)], extra_env: &[(&str, String)]) -> ChildResult {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.args([
        "--exact",
        "child_worker",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]);
    cmd.env("CHILD_WORKER", "1");
    for (k, v) in env {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    // Forward the library locations so the child resolves the same .so files.
    if let Ok(v) = std::env::var("C_LIB_PATH") {
        cmd.env("C_LIB_PATH", v);
    }
    if let Ok(v) = std::env::var("RUST_LIB_PATH") {
        cmd.env("RUST_LIB_PATH", v);
    }
    let out = cmd.output().expect("spawn child test process");
    ChildResult {
        signal: out.status.signal(),
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Child-process worker. Never runs during a normal `cargo test` (it is
/// `#[ignore]`d and additionally gated on `CHILD_WORKER`).
#[test]
#[ignore = "spawned as a child process by e7_crash_parity / e9_malloc_failure*"]
fn child_worker() {
    assert_eq!(
        std::env::var("CHILD_WORKER").as_deref(),
        Ok("1"),
        "child_worker must only be run as a child process"
    );
    if let Ok(scenario) = std::env::var("CRASH_SCENARIO") {
        return child_crash(&scenario);
    }
    if let Ok(mode) = std::env::var("MALLOC_FAIL_MODE") {
        return child_malloc_fail(mode.parse().expect("MALLOC_FAIL_MODE"));
    }
    panic!("child_worker invoked without a scenario");
}

fn child_crash(scenario: &str) {
    let api = match std::env::var("CRASH_IMPL").expect("CRASH_IMPL").as_str() {
        "c" => common::c_api(),
        "rust" => common::rust_api(),
        other => panic!("bad CRASH_IMPL {other}"),
    };
    // Flush before the deliberate crash so the parent sees the marker.
    println!("about to run {scenario} on {}", api.name);
    use std::io::Write;
    std::io::stdout().flush().ok();

    let sink = unsafe {
        match scenario {
            "arity_null_len2" => (api.arity)(2, std::ptr::null()),
            "arity_null_len3" => (api.arity)(3, std::ptr::null()),
            "arity_null_len4" => (api.arity)(4, std::ptr::null()),
            "arity_null_len255" => (api.arity)(-1, std::ptr::null()),
            "shift_array_null" => {
                (api.shift_array)(std::ptr::null_mut(), 4, 1);
                0
            }
            "process_string_null" => (api.process_string)(std::ptr::null()),
            "init_matrix_null" => {
                (api.init_matrix)(std::ptr::null_mut());
                0
            }
            other => panic!("bad CRASH_SCENARIO {other}"),
        }
    };
    // Should be unreachable; if the call somehow returns, report it distinctly.
    println!("survived with {sink}");
    std::process::exit(42);
}

// ===========================================================================
// E12 .. E15 — malloc failure (LD_PRELOAD interposition in a child process)
// ===========================================================================

const SHIM_SRC: &str = r#"
/* Test fixture: LD_PRELOAD shim that controls what malloc(sizeof(int)) returns.
   Not part of the library under test.

   modes:
     0  pass through
     1  every malloc(4) fails
     2  the 1st malloc(4) of a window fails
     3  the 2nd malloc(4) of a window fails
    10  both malloc(4) calls return the SAME address   -> ptr1 == ptr2
    11  the two malloc(4) calls return DESCENDING addrs -> ptr1 >  ptr2
    12  the two malloc(4) calls return ASCENDING addrs  -> ptr1 <  ptr2
   Modes 10-12 hand out addresses inside a static array, so `free` is
   interposed as well to ignore them. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

static void *(*real_malloc)(size_t);
static void (*real_free)(void *);
static int fail_mode;
static int counter;

/* Two distinct, writable, correctly-aligned int slots. */
static int slots[2];

__attribute__((constructor)) static void init(void) {
    real_malloc = dlsym(RTLD_NEXT, "malloc");
    real_free = dlsym(RTLD_NEXT, "free");
}

void failmalloc_set_mode(int mode) { fail_mode = mode; counter = 0; }

static int is_fake(void *p) {
    return p == (void *)&slots[0] || p == (void *)&slots[1];
}

void *malloc(size_t size) {
    if (!real_malloc) real_malloc = dlsym(RTLD_NEXT, "malloc");
    if (fail_mode != 0 && size == sizeof(int)) {
        int n = counter++;
        if (fail_mode == 1) return NULL;
        if (fail_mode == 2 && n == 0) return NULL;
        if (fail_mode == 3 && n == 1) return NULL;
        if (fail_mode == 10) return &slots[0];
        if (fail_mode == 11) return (n % 2 == 0) ? &slots[1] : &slots[0];
        if (fail_mode == 12) return (n % 2 == 0) ? &slots[0] : &slots[1];
    }
    return real_malloc(size);
}

void free(void *p) {
    if (!real_free) real_free = dlsym(RTLD_NEXT, "free");
    if (is_fake(p)) return;
    real_free(p);
}
"#;

/// Compile the `LD_PRELOAD` shim exactly once per test process. Several tests
/// need it and libtest runs them on parallel threads, so the build is guarded by
/// a `OnceLock` and published with an atomic rename — otherwise a child could
/// `dlopen` a half-written file.
fn build_shim() -> PathBuf {
    static SHIM: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    SHIM.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-fixtures");
        std::fs::create_dir_all(&dir).expect("create fixture dir");
        let pid = std::process::id();
        let src = dir.join(format!("failmalloc-{pid}.c"));
        let tmp = dir.join(format!("libfailmalloc-{pid}.so.tmp"));
        let so = dir.join(format!("libfailmalloc-{pid}.so"));
        std::fs::write(&src, SHIM_SRC).expect("write shim source");
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let out = Command::new(&cc)
            .args(["-shared", "-fPIC", "-O1", "-o"])
            .arg(&tmp)
            .arg(&src)
            .arg("-ldl")
            .output()
            .unwrap_or_else(|e| panic!("cannot run {cc}: {e}"));
        assert!(
            out.status.success(),
            "compiling the malloc shim failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::rename(&tmp, &so).expect("publish shim");
        so
    })
    .clone()
}

/// Child side: enable the failing malloc around a single FFI call on each
/// library and compare. Nothing may allocate inside the window.
fn child_malloc_fail(mode: i32) {
    let shim = std::env::var("SHIM_PATH").expect("SHIM_PATH");
    // dlopen of an already (pre)loaded object returns the same instance, so the
    // control symbol drives the interposer that both libraries see.
    let shim_lib = unsafe { libloading::Library::new(&shim).expect("dlopen shim") };
    let set_mode: libloading::Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { shim_lib.get(b"failmalloc_set_mode").expect("set_mode") };

    let (c, r) = both();
    let which = std::env::var("MALLOC_FAIL_TARGET").unwrap_or_else(|_| "cmp".into());
    let p: [c_int; 4] = [7, -9, 3, 5];

    // Pointer-ordering branches (shim modes 10/11/12): the two `malloc` results
    // are forced to be equal / descending / ascending, which makes
    // `compare_allocations` deterministic and reaches the `ptr1 == ptr2` branch
    // that a real allocator never produces.
    if which == "order" {
        for (v1, v2) in ORDER_VALUES {
            let cv = unsafe {
                normalize_allocator(AllocOrder::Increasing);
                set_mode(mode);
                let v = (c.compare_allocations)(v1, v2);
                set_mode(0);
                v
            };
            let rv = unsafe {
                normalize_allocator(AllocOrder::Increasing);
                set_mode(mode);
                let v = (r.compare_allocations)(v1, v2);
                set_mode(0);
                v
            };
            println!("ORDER mode={mode} v1={v1} v2={v2} C={cv} Rust={rv}");
            assert_eq!(cv, rv, "pointer-order mismatch mode={mode} ({v1},{v2})");
        }
        return;
    }

    // One measurement per address ordering. The allocator is canonicalised
    // *before* the interposer is armed (`normalize_allocator` needs a working
    // `malloc`), which keeps the pass-through mode (`mode == 0`) deterministic
    // exactly like the in-process tests. `set_mode` also resets the shim's
    // counter, so every window sees the same failure pattern.
    let call = |api: &common::Api| -> (c_int, c_int) {
        let one = |api: &common::Api, order: AllocOrder| unsafe {
            normalize_allocator(order);
            set_mode(mode);
            let v = match which.as_str() {
                "cmp" => (api.compare_allocations)(1, 2),
                "arity4" => (api.arity4)(p[0], p[1], p[2], p[3]),
                "arity2" => (api.arity2)(p[0], p[1]),
                "arity3" => (api.arity3)(p[0], p[1], p[2]),
                "arity" => (api.arity)(4, p.as_ptr()),
                other => panic!("bad MALLOC_FAIL_TARGET {other}"),
            };
            set_mode(0);
            v
        };
        (
            one(api, AllocOrder::Increasing),
            one(api, AllocOrder::Decreasing),
        )
    };

    let cv = call(c);
    let rv = call(r);
    println!("RESULT {which} mode={mode} C={},{} Rust={},{}", cv.0, cv.1, rv.0, rv.1);
    assert_eq!(cv, rv, "malloc-failure mismatch for {which} mode {mode}");
}

/// Returns `((c1, c2), (r1, r2), child)`: the two-call pair each library
/// produced in the child process.
fn run_malloc_fail_child(
    mode: i32,
    target: &str,
    shim: &PathBuf,
) -> ((i32, i32), (i32, i32), ChildResult) {
    let res = run_child(
        &[("MALLOC_FAIL_TARGET", target)],
        &[
            ("MALLOC_FAIL_MODE", mode.to_string()),
            ("SHIM_PATH", shim.display().to_string()),
            ("LD_PRELOAD", shim.display().to_string()),
        ],
    );
    assert!(
        res.signal.is_none() && res.code == Some(0),
        "malloc-failure child ({target}, mode {mode}) did not succeed: \
         code={:?} signal={:?}\nstdout: {}\nstderr: {}",
        res.code,
        res.signal,
        res.stdout,
        res.stderr
    );
    // libtest writes "test child_worker ... " without a newline, so the marker
    // is not necessarily at the start of the line.
    let line = res
        .stdout
        .lines()
        .find(|l| l.contains("RESULT "))
        .unwrap_or_else(|| panic!("child produced no RESULT line:\n{}", res.stdout));
    let parse_pair = |s: &str| -> (i32, i32) {
        let (a, b) = s.split_once(',').expect("pair");
        (a.parse().expect("lhs"), b.parse().expect("rhs"))
    };
    let mut c_val = None;
    let mut r_val = None;
    for tok in line.split_whitespace() {
        if let Some(v) = tok.strip_prefix("C=") {
            c_val = Some(parse_pair(v));
        }
        if let Some(v) = tok.strip_prefix("Rust=") {
            r_val = Some(parse_pair(v));
        }
    }
    (
        c_val.expect("C= in RESULT line"),
        r_val.expect("Rust= in RESULT line"),
        res,
    )
}

#[test]
fn e9_malloc_failure_returns_minus1() {
    let shim = build_shim();
    // Sanity check: with the interposer in pass-through mode the child must
    // behave exactly like the in-process tests.
    let (c0, r0, _) = run_malloc_fail_child(0, "cmp", &shim);
    assert_eq!(c0, r0, "E12: shim in pass-through mode changed behaviour");
    for v in [c0.0, c0.1] {
        assert!(
            (1..=13).contains(&v),
            "E12: unexpected pass-through result {v} (pair {c0:?})"
        );
    }

    // E12: first malloc fails, E13: second malloc fails, E14: both fail.
    for mode in [2, 3, 1] {
        let (cv, rv, res) = run_malloc_fail_child(mode, "cmp", &shim);
        assert_eq!(cv, rv, "E12/E13/E14 (mode {mode}): C={cv:?} Rust={rv:?}");
        assert_eq!(
            cv,
            (-1, -1),
            "E12/E13/E14 (mode {mode}): compare_allocations must return -1 \
             on allocation failure\n{}",
            res.stdout
        );
    }
}

/// (val1, val2) pairs used by the pointer-ordering tests.
const ORDER_VALUES: [(c_int, c_int); 9] = [
    (1, 2),
    (1, -2),
    (-1, 2),
    (0, 0),
    (5, 0),
    (0, 5),
    (i32::MIN, i32::MAX),
    (i32::MAX, i32::MIN),
    (-7, -9),
];

/// The three `compare_allocations` address-ordering branches
/// (`lib.c:102-108`). A real glibc heap only ever produces the `<` and `>`
/// branches, so `ptr1 == ptr2` (`result = 3`) is reached by forcing `malloc` to
/// return the same address twice.
///
/// This also pins the aliasing behaviour of `lib.c:97-98` + `lib.c:111`: with
/// `ptr1 == ptr2`, `*ptr1 = val1; *ptr2 = val2;` leaves `val2` in memory, so the
/// `*uninit_ptr > 0` bonus must be decided by **val2**, not by `val1`. A
/// translation that kept `val1` in a register would diverge here.
#[test]
fn e24_pointer_order_branches() {
    let shim = build_shim();
    for (mode, expect_order) in [(12, 1), (11, 2), (10, 3)] {
        let res = run_child(
            &[("MALLOC_FAIL_TARGET", "order")],
            &[
                ("MALLOC_FAIL_MODE", mode.to_string()),
                ("SHIM_PATH", shim.display().to_string()),
                ("LD_PRELOAD", shim.display().to_string()),
            ],
        );
        assert!(
            res.signal.is_none() && res.code == Some(0),
            "pointer-order child (mode {mode}) failed: code={:?} signal={:?}\n{}\n{}",
            res.code,
            res.signal,
            res.stdout,
            res.stderr
        );
        let mut seen = 0;
        for line in res.stdout.lines() {
            let Some(idx) = line.find("ORDER mode=") else {
                continue;
            };
            let fields: Vec<&str> = line[idx..].split_whitespace().collect();
            let get = |k: &str| -> i64 {
                fields
                    .iter()
                    .find_map(|f| f.strip_prefix(k))
                    .unwrap_or_else(|| panic!("missing {k} in {line}"))
                    .parse()
                    .unwrap()
            };
            let (v1, v2, cv, rv) = (get("v1="), get("v2="), get("C="), get("Rust="));
            assert_eq!(cv, rv, "mode {mode}: C={cv} Rust={rv} for ({v1},{v2})");
            // Value written last into the (possibly shared) slot read back by
            // `*uninit_ptr`.
            let observed = if mode == 10 { v2 } else { v1 };
            let expected = expect_order + if observed > 0 { 10 } else { 0 };
            assert_eq!(
                cv, expected,
                "mode {mode}: compare_allocations({v1},{v2}) must be {expected}, got {cv}"
            );
            seen += 1;
        }
        assert_eq!(
            seen,
            ORDER_VALUES.len(),
            "mode {mode}: expected {} ORDER lines, saw {seen}\n{}",
            ORDER_VALUES.len(),
            res.stdout
        );
    }
}

#[test]
fn e10_malloc_failure_propagates_into_arity4() {
    let shim = build_shim();
    // E15: `compare_allocations` returning -1 is *added* to `result`; it is not
    // treated as an error by arity4/arity3/arity2/arity.
    for target in ["arity4", "arity3", "arity2", "arity"] {
        for mode in [1, 2, 3] {
            let (cv, rv, res) = run_malloc_fail_child(mode, target, &shim);
            assert_eq!(
                cv, rv,
                "E15: {target} under malloc failure mode {mode}: C={cv:?} Rust={rv:?}\n{}",
                res.stdout
            );
        }
    }
}
