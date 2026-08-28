//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `bitwriter_add` has NO error surface at all (no error return, no assert, no
//! null/range check — greps in `ERRORS.md`), so each row asserts the *inverse*
//! obligation: the C **accepts** the invalid input, mangles state via
//! UB-shift / unsigned wraparound, and returns `0`.  The Rust `.so` must return
//! the identical sentinel AND the identical mangled state.
//!
//! Both implementations are always reached through `libloading` + the exported
//! `bitwriter_add` symbol.

mod common;

use common::{load_pair, Bitwriter, Checker, Impl, Rng};

// ---------------------------------------------------------------------------
// E1 — bw == NULL: unchecked dereference, must fault identically.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    /// `_exit`, not `exit`: the forked child must not run atexit handlers or
    /// flush the parent's buffers.
    fn _exit(code: i32) -> !;
}

/// Sentinel exit code used when the call unexpectedly *returns* instead of faulting.
const RETURNED_SENTINEL: i32 = 42;

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signaled(i32),
    Exited(i32),
}

/// Call `imp` with a NULL `bw` in a forked child and report how the child died.
///
/// The library is already `dlopen`ed in the parent before forking, so the child
/// performs no allocation / no `dlopen` — it only invokes the cached fn pointer.
fn null_call_outcome(imp: &Impl, bits: u32, val: u64) -> Outcome {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // Child: no allocation, no unwinding, no test-harness cleanup.
        let rc = unsafe { (imp.raw())(std::ptr::null_mut(), bits, val) };
        // Reached only if the deref did NOT fault.
        unsafe { _exit(RETURNED_SENTINEL + (rc & 0xF)) };
    }
    let mut status: i32 = 0;
    let w = unsafe { waitpid(pid, &mut status as *mut i32, 0) };
    assert_eq!(w, pid, "waitpid failed");

    if status & 0x7f != 0 && status & 0x7f != 0x7f {
        Outcome::Signaled(status & 0x7f)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}

#[test]
fn e1_null_bw_pointer_faults_identically() {
    let p = load_pair();
    for (bits, val) in [(0u32, 0u64), (1, 1), (32, 0xDEAD_BEEF), (64, u64::MAX), (u32::MAX, 7)] {
        let c = null_call_outcome(&p.c, bits, val);
        let r = null_call_outcome(&p.rust, bits, val);
        assert_eq!(
            c, r,
            "[E1] NULL bw diverged for bits={bits} val=0x{val:016x}: C={c:?} Rust={r:?}"
        );
        // Both must actually fault rather than silently returning.
        assert!(
            matches!(c, Outcome::Signaled(_)),
            "[E1] expected C to fault on NULL bw, got {c:?}"
        );
        eprintln!("[E1] bits={bits}: both faulted identically -> {c:?}");
    }
}

// ---------------------------------------------------------------------------
// E2 — bits == 0: line-8 shift count is 64 (out of range for a 64-bit shift)
// ---------------------------------------------------------------------------
#[test]
fn e2_bits_zero_out_of_range_shift() {
    let p = load_pair();
    let mut r = Rng::new(0xE002);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let init = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            r.next_u64() as usize,
        );
        // accepted, never rejected
        let (_, rc) = common::diff_one(&p, init, 0, r.interesting_u64())
            .unwrap_or_else(|e| panic!("[E2] {e}"));
        assert_eq!(rc, 0, "[E2] expected sentinel 0");
        ck.check(init, 0, r.interesting_u64());
    }
    ck.finish("E2");
}

// ---------------------------------------------------------------------------
// E3 — bits == 64: boundary, exactly the operand width
// ---------------------------------------------------------------------------
#[test]
fn e3_bits_eq_width_boundary() {
    let p = load_pair();
    let mut r = Rng::new(0xE003);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let init = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            0,
        );
        let val = r.interesting_u64();
        let (_, rc) =
            common::diff_one(&p, init, 64, val).unwrap_or_else(|e| panic!("[E3] {e}"));
        assert_eq!(rc, 0, "[E3] expected sentinel 0");
        ck.check(init, 64, val);
    }
    ck.finish("E3");
}

// ---------------------------------------------------------------------------
// E4 — bits == 65: one step past the maximum meaningful width
// ---------------------------------------------------------------------------
#[test]
fn e4_bits_one_past_valid_range() {
    let p = load_pair();
    let mut r = Rng::new(0xE004);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let init = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            0,
        );
        let val = r.interesting_u64();
        let (_, rc) =
            common::diff_one(&p, init, 65, val).unwrap_or_else(|e| panic!("[E4] {e}"));
        assert_eq!(rc, 0, "[E4] not rejected: expected sentinel 0");
        ck.check(init, 65, val);
    }
    ck.finish("E4");
}

// ---------------------------------------------------------------------------
// E5 — oversized `bits` (no upper bound check exists)
// ---------------------------------------------------------------------------
#[test]
fn e5_bits_oversized_lengths() {
    let p = load_pair();
    let mut r = Rng::new(0xE005);
    let mut ck = Checker::new(&p);
    for bits in [0x8000_0000u32, 0x8000_0001, 0xC000_0000, 0xFFFF_FFFE, u32::MAX] {
        for _ in 0..2_000 {
            let init = Bitwriter::new(
                r.interesting_u64(),
                r.interesting_bits(),
                r.next_u32(),
                r.next_u32(),
                r.next_u32(),
                0,
            );
            let val = r.interesting_u64();
            let (_, rc) =
                common::diff_one(&p, init, bits, val).unwrap_or_else(|e| panic!("[E5] {e}"));
            assert_eq!(rc, 0, "[E5] expected sentinel 0 for bits={bits}");
            ck.check(init, bits, val);
        }
    }
    ck.finish("E5");
}

// ---------------------------------------------------------------------------
// E6 — bw->bits == 64: invalid internal state, out-of-range `>>` counts
// ---------------------------------------------------------------------------
#[test]
fn e6_bwbits_eq_width_invalid_state() {
    let p = load_pair();
    let mut r = Rng::new(0xE006);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let init = Bitwriter::new(r.interesting_u64(), 64, r.next_u32(), r.next_u32(), r.next_u32(), 0);
        let bits = r.interesting_bits();
        let val = r.interesting_u64();
        let (_, rc) =
            common::diff_one(&p, init, bits, val).unwrap_or_else(|e| panic!("[E6] {e}"));
        assert_eq!(rc, 0, "[E6] expected sentinel 0");
        ck.check(init, bits, val);
    }
    ck.finish("E6");
}

// ---------------------------------------------------------------------------
// E7 — bw->bits > 64 up to u32::MAX, never validated
// ---------------------------------------------------------------------------
#[test]
fn e7_bwbits_grossly_invalid_state() {
    let p = load_pair();
    let mut r = Rng::new(0xE007);
    let mut ck = Checker::new(&p);
    for bwbits in [65u32, 66, 127, 128, 1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, u32::MAX] {
        for _ in 0..1_500 {
            let init =
                Bitwriter::new(r.interesting_u64(), bwbits, r.next_u32(), r.next_u32(), r.next_u32(), 0);
            let bits = r.interesting_bits();
            let val = r.interesting_u64();
            let (_, rc) =
                common::diff_one(&p, init, bits, val).unwrap_or_else(|e| panic!("[E7] {e}"));
            assert_eq!(rc, 0, "[E7] expected sentinel 0 for bw->bits={bwbits}");
            ck.check(init, bits, val);
        }
    }
    ck.finish("E7");
}

// ---------------------------------------------------------------------------
// E8 — the ONE defensive construct: `i < 100` bails out the stalled loop
//      (bw->bits == 63, bits >= 1  =>  b == 0, no progress)
// ---------------------------------------------------------------------------
#[test]
fn e8_iteration_cap_bails_out_stalled_loop() {
    let p = load_pair();
    let mut r = Rng::new(0xE008);
    let mut ck = Checker::new(&p);

    // The C must TERMINATE (the guard is what prevents an infinite loop) and
    // return 0, leaving bw->bits untouched at 63.
    for bits in [1u32, 2, 5, 63, 64, 65, 100, 4096, u32::MAX] {
        let init = Bitwriter::new(0xFFFF_FFFF_FFFF_FFFF, 63, 3, 4, 5, 0);
        let (after, rc) =
            common::diff_one(&p, init, bits, 0xFFFF_FFFF_FFFF_FFFF).unwrap_or_else(|e| panic!("[E8] {e}"));
        assert_eq!(rc, 0, "[E8] expected sentinel 0");
        // The stalled loop makes no progress (b == 0), so `bits` is never
        // decremented; the post-loop `bw->bits += bits` (line 22) therefore adds
        // the FULL original `bits` to the unchanged 63, wrapping mod 2^32.
        assert_eq!(
            after.bits,
            63u32.wrapping_add(bits),
            "[E8] post-loop bw->bits must be 63 + bits (wrapping) for bits={bits}"
        );
        ck.check(init, bits, 0xFFFF_FFFF_FFFF_FFFF);
    }
    for _ in 0..10_000 {
        let init = Bitwriter::new(r.interesting_u64(), 63, r.next_u32(), r.next_u32(), r.next_u32(), 0);
        let bits = r.range(1, u32::MAX);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("E8");
}

// ---------------------------------------------------------------------------
// E9 — cap reached via bits == 0 while bw->bits >= 64 (b clamped to 0)
// ---------------------------------------------------------------------------
#[test]
fn e9_iteration_cap_via_zero_bits() {
    let p = load_pair();
    let mut r = Rng::new(0xE009);
    let mut ck = Checker::new(&p);
    for bwbits in [64u32, 65, 127, 128, 1000, 0x8000_0000, u32::MAX] {
        let init = Bitwriter::new(0xFFFF_FFFF_FFFF_FFFF, bwbits, 1, 2, 3, 0);
        let (after, rc) =
            common::diff_one(&p, init, 0, u64::MAX).unwrap_or_else(|e| panic!("[E9] {e}"));
        assert_eq!(rc, 0, "[E9] expected sentinel 0");
        assert_eq!(after.bits, bwbits, "[E9] stalled loop must leave bw->bits unchanged");
        ck.check(init, 0, u64::MAX);
    }
    for _ in 0..10_000 {
        let bwbits = r.range(64, u32::MAX);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, r.next_u32(), r.next_u32(), r.next_u32(), 0);
        ck.check(init, 0, r.interesting_u64());
    }
    ck.finish("E9");
}

// ---------------------------------------------------------------------------
// E10 — bw->bits + bits overflows unsigned int: loop NOT entered
// ---------------------------------------------------------------------------
#[test]
fn e10_loop_condition_u32_overflow() {
    let p = load_pair();
    let mut r = Rng::new(0xE010);
    let mut ck = Checker::new(&p);

    // canonical: 0xFFFFFFFF + 1 wraps to 0, which is NOT >= 64
    let init = Bitwriter::new(0x0F0F_0F0F_0F0F_0F0F, u32::MAX, 1, 2, 3, 0);
    let (after, rc) =
        common::diff_one(&p, init, 1, u64::MAX).unwrap_or_else(|e| panic!("[E10] {e}"));
    assert_eq!(rc, 0, "[E10] expected sentinel 0");
    // loop skipped => bw->bits becomes 0xFFFFFFFF + 1 == 0 (wrapped)
    assert_eq!(after.bits, 0, "[E10] bw->bits must wrap to 0");

    for _ in 0..20_000 {
        let k = r.range(0, 5_000_000);
        let j = r.range(0, 63);
        let bwbits = u32::MAX - k;
        let bits = k + 1 + j;
        assert_eq!(bwbits.wrapping_add(bits), j);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, r.next_u32(), r.next_u32(), r.next_u32(), 0);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("E10");
}

// ---------------------------------------------------------------------------
// E11 — bw->tot overflow wraps silently, unchecked
// ---------------------------------------------------------------------------
#[test]
fn e11_tot_overflow_unchecked() {
    let p = load_pair();
    let mut r = Rng::new(0xE011);
    let mut ck = Checker::new(&p);

    let init = Bitwriter::new(0, 0, 0, 0, u32::MAX, 0);
    let (after, rc) = common::diff_one(&p, init, 1, 0).unwrap_or_else(|e| panic!("[E11] {e}"));
    assert_eq!(rc, 0, "[E11] expected sentinel 0");
    assert_eq!(after.tot, 0, "[E11] tot must wrap 0xFFFFFFFF + 1 -> 0");

    for tot in [0xFFFF_FF00u32, 0xFFFF_FFFE, u32::MAX] {
        for _ in 0..3_000 {
            let init = Bitwriter::new(r.interesting_u64(), r.interesting_bits(), 0, 0, tot, 0);
            ck.check(init, r.interesting_bits(), r.interesting_u64());
        }
    }
    ck.finish("E11");
}

// ---------------------------------------------------------------------------
// E12 — out-of-range "enum"-style ints across the FFI boundary.
//       `bits` is the only scalar selector; every u32 is a legal C input.
// ---------------------------------------------------------------------------
#[test]
fn e12_out_of_range_scalar_selectors() {
    let p = load_pair();
    let mut r = Rng::new(0xE012);
    let mut ck = Checker::new(&p);

    const EDGE: [u32; 14] = [
        0, 1, 63, 64, 65, 127, 128, 255, 256, 1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    for bits in EDGE {
        for bwbits in EDGE {
            for &(bwval, val) in &[
                (0u64, 0u64),
                (u64::MAX, u64::MAX),
                (0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210),
            ] {
                let init = Bitwriter::new(bwval, bwbits, 0x1111, 0x2222, 0x3333, 0);
                let (_, rc) = common::diff_one(&p, init, bits, val)
                    .unwrap_or_else(|e| panic!("[E12] {e}"));
                assert_eq!(rc, 0, "[E12] expected sentinel 0 for bits={bits} bw->bits={bwbits}");
                ck.check(init, bits, val);
            }
        }
    }
    // randomized: every u32 is in range for the C, so sample the whole space
    for _ in 0..30_000 {
        let init = Bitwriter::new(r.interesting_u64(), r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32(), 0);
        ck.check(init, r.next_u32(), r.next_u64());
    }
    ck.finish("E12");
}

// ---------------------------------------------------------------------------
// E13 — fields the C never touches must be preserved and never dereferenced
// ---------------------------------------------------------------------------
#[test]
fn e13_untouched_fields_not_dereferenced() {
    let p = load_pair();
    let mut r = Rng::new(0xE013);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        // deliberately bogus, non-null, unmapped, and misaligned pointers
        let buf = match r.next_u64() % 4 {
            0 => 0x1usize,
            1 => 0xDEAD_BEEFusize,
            2 => usize::MAX,
            _ => (r.next_u64() | 1) as usize,
        };
        let pos = r.next_u32();
        let len = r.next_u32();
        let init = Bitwriter::new(r.interesting_u64(), r.interesting_bits(), pos, len, r.next_u32(), buf);
        let bits = r.interesting_bits();
        let val = r.interesting_u64();

        let (after, rc) =
            common::diff_one(&p, init, bits, val).unwrap_or_else(|e| panic!("[E13] {e}"));
        assert_eq!(rc, 0, "[E13] expected sentinel 0");
        assert_eq!(after.pos, pos, "[E13] pos changed");
        assert_eq!(after.len, len, "[E13] len changed");
        assert_eq!(after.buffer as usize, buf, "[E13] buffer changed / dereferenced");
        ck.check(init, bits, val);
    }
    ck.finish("E13");
}

// ---------------------------------------------------------------------------
// Cross-cutting: the return value is UNCONDITIONALLY 0 in both implementations.
// ---------------------------------------------------------------------------
#[test]
fn e_all_return_zero_unconditionally() {
    let p = load_pair();
    let mut r = Rng::new(0xE0FF);
    for _ in 0..100_000 {
        let mut cs = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            r.next_u64() as usize,
        );
        let mut rs = cs;
        let bits = r.interesting_bits();
        let val = r.interesting_u64();
        let rc_c = p.c.call(&mut cs, bits, val);
        let rc_r = p.rust.call(&mut rs, bits, val);
        assert_eq!(rc_c, 0, "C returned non-zero for bits={bits}");
        assert_eq!(rc_r, 0, "Rust returned non-zero for bits={bits}");
    }
    eprintln!("[E-all] OK: 100000 cases, both always returned sentinel 0");
}
