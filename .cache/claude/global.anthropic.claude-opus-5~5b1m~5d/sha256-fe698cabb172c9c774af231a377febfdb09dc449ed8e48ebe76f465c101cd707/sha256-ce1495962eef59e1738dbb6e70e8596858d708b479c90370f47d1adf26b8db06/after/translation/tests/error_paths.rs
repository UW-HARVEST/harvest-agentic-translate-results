//! Phase C - error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md` (E1..E10).
//!
//! `rev16` is a total function: the mechanical greps recorded in `ERRORS.md`
//! found zero error returns, zero asserts, zero range checks, zero pointer
//! parameters, zero length parameters and zero enum parameters in the C source.
//! Every one of the 2^32 `uint32_t` values is therefore a *valid* input with a
//! defined result, and there is no rejection to match.
//!
//! The obligation these tests discharge is the mirror image: for each input that
//! *would* be the invalid/boundary case in an API that had one, the Rust must
//! reject it in exactly the same way the C does - i.e. **not at all**, returning
//! the identical value without panicking, aborting, or trapping on overflow.
//! Each test therefore pins the exact `u32` the C returns and requires the Rust
//! to produce it bit for bit.

mod common;

use common::{assert_same, assert_same_all, Rng};

// ---------------------------------------------------------------------------
// E1 - no null-pointer input exists (the parameter is a by-value scalar).
//      Closest representable analogue: the all-zero argument.
// ---------------------------------------------------------------------------
#[test]
fn e1_null_pointer_analogue_is_zero_argument() {
    let got = assert_same("E1", 0x0000_0000);
    assert_eq!(got, 0x0000_0000, "expected C to return 0 for a zero argument");
}

// ---------------------------------------------------------------------------
// E2 - zero-length analogue: empty 16-bit payload, high garbage present.
// ---------------------------------------------------------------------------
#[test]
fn e2_zero_length_analogue_high_garbage_only() {
    let got = assert_same("E2", 0xFFFF_0000);
    assert_eq!(
        got, 0x0000_0000,
        "statement 1's 16-bit masks must discard bits 16..31"
    );

    // Any high-half garbage with an empty low half must behave the same.
    let mut rng = Rng::new(0xE2_5EED);
    for _ in 0..50_000 {
        let arg = (rng.next_u16() as u32) << 16;
        let v = assert_same("E2", arg);
        assert_eq!(v, 0, "0x{arg:08X} should collapse to 0");
    }
}

// ---------------------------------------------------------------------------
// E3 - oversized-length analogue: the numeric maximum of the type.
// ---------------------------------------------------------------------------
#[test]
fn e3_numeric_maximum_is_not_rejected() {
    let got = assert_same("E3", u32::MAX);
    assert_eq!(got, 0x0000_FFFF, "0xFFFFFFFF must map to 0x0000FFFF");
}

// ---------------------------------------------------------------------------
// E4 / E5 - one step past, and one step below, the 16-bit window the masks span.
// ---------------------------------------------------------------------------
#[test]
fn e4_one_past_the_mask_window() {
    let got = assert_same("E4", 0x0001_0000);
    assert_eq!(
        got, 0x0000_0000,
        "the first bit above the window is silently discarded, not an error"
    );
}

#[test]
fn e5_largest_value_inside_the_mask_window() {
    let got = assert_same("E5", 0x0000_FFFF);
    assert_eq!(got, 0x0000_FFFF);
}

#[test]
fn e4_e5_window_boundary_neighbourhood() {
    // Walk both sides of the 0xFFFF / 0x10000 boundary.
    let args = (0xFF00u32..=0x1_00FF).chain(0x1_FF00..=0x2_00FF);
    assert_same_all("E4/E5", args);
}

// ---------------------------------------------------------------------------
// E6 - out-of-range "enum" analogue. There is no enum parameter, so the analogue
//      is each bit position outside the honoured window taken on its own.
// ---------------------------------------------------------------------------
#[test]
fn e6_every_out_of_window_bit_alone() {
    for k in 16..32 {
        let arg = 1u32 << k;
        let got = assert_same("E6", arg);
        assert_eq!(
            got, 0x0000_0000,
            "bit {k} alone must yield 0, got 0x{got:08X}"
        );
    }

    // Also feed values that would be out-of-range discriminants for a C enum
    // (C enums accept any int, so these are real inputs the C must handle).
    let sentinels: [u32; 12] = [
        0x0001_0000,
        0x7FFF_0000,
        0x8000_0000,
        0xDEAD_0000,
        0xFFFF_0000,
        u32::MAX,
        0x0000_0000,
        (-1i32) as u32,
        (i32::MIN) as u32,
        (i32::MAX) as u32,
        0xCAFE_BABE,
        0xBADD_C0DE,
    ];
    assert_same_all("E6", sentinels);
}

// ---------------------------------------------------------------------------
// E7 - values that are negative when misread as int32_t.
// ---------------------------------------------------------------------------
#[test]
fn e7_sign_bit_set_arguments() {
    // The C library is the oracle; assert_same pins Rust to whatever C returns.
    let args: [u32; 8] = [
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
        0x8000_8000,
        0xFFFF_8000,
        0x8000_FFFF,
        (i32::MIN as u32).wrapping_add(1),
    ];
    assert_same_all("E7", args);

    // Broad randomised sweep restricted to the sign-bit-set half of the domain,
    // which is exactly the region a signed misinterpretation would corrupt.
    let mut rng = Rng::new(0xE7_5EED);
    for _ in 0..500_000 {
        assert_same("E7", rng.next_u32() | 0x8000_0000);
    }
}

// ---------------------------------------------------------------------------
// E8 - shift-overflow hazard: maximise each `<<` operand.
// ---------------------------------------------------------------------------
#[test]
fn e8_maximal_left_shift_operands() {
    // Largest pre-shift value for each of the four statements, plus their
    // combinations, so every `<<` in the chain sees its widest input.
    let base: [u32; 4] = [0x0000_5555, 0x0000_3333, 0x0000_0F0F, 0x0000_00FF];
    assert_same_all("E8", base);

    let mut args = Vec::new();
    for &a in &base {
        for &b in &base {
            args.push(a | b);
            args.push(a & b);
            args.push(a ^ b);
            args.push((a | b) | 0xFFFF_0000); // with high garbage too
        }
    }
    assert_same_all("E8", args);

    // Debug-profile Rust traps on arithmetic overflow, so a wrong widening in
    // the translation would abort here rather than diverge silently; the release
    // .so under test would instead wrap. Either way the values must match C.
    assert_same("E8", 0x0000_FFFF);
    assert_same("E8", 0xFFFF_FFFF);
}

// ---------------------------------------------------------------------------
// E9 - purity / re-entrancy: no hidden state anywhere.
// ---------------------------------------------------------------------------
#[test]
fn e9_function_is_pure_and_stateless() {
    let c = common::c_rev16();
    let r = common::rust_rev16();

    // Repeat the same argument many times.
    let first_c = unsafe { c(0x1234_5678) };
    let first_r = unsafe { r(0x1234_5678) };
    assert_eq!(first_c, first_r);
    for _ in 0..100_000 {
        assert_eq!(unsafe { c(0x1234_5678) }, first_c, "[E9] C became stateful");
        assert_eq!(unsafe { r(0x1234_5678) }, first_r, "[E9] Rust became stateful");
    }

    // Interleave different arguments and re-check the pinned one in between.
    let mut rng = Rng::new(0xE9_5EED);
    for _ in 0..200_000 {
        let noise = rng.next_u32();
        let cn = unsafe { c(noise) };
        let rn = unsafe { r(noise) };
        assert_eq!(cn, rn, "[E9] divergence for 0x{noise:08X}");
        assert_eq!(unsafe { c(0x1234_5678) }, first_c);
        assert_eq!(unsafe { r(0x1234_5678) }, first_r);
    }

    // And from several threads at once.
    let mut handles = Vec::new();
    for t in 0..4u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0xE9_D00D ^ t);
            for _ in 0..100_000 {
                let a = rng.next_u32();
                assert_eq!(unsafe { c(a) }, unsafe { r(a) });
            }
            assert_eq!(unsafe { c(0x1234_5678) }, unsafe { r(0x1234_5678) });
        }));
    }
    for h in handles {
        h.join().expect("[E9] worker thread panicked");
    }
}

// ---------------------------------------------------------------------------
// E10 - ABI width: a dirty upper half in the argument register must be ignored
//       by both libraries, matching `uint32_t` / `c_uint`.
// ---------------------------------------------------------------------------
#[test]
fn e10_dirty_upper_register_half_is_ignored() {
    let cw = common::c_rev16_wide();
    let rw = common::rust_rev16_wide();

    let mut rng = Rng::new(0xE10_5EED);
    for _ in 0..200_000 {
        let low32 = rng.next_u32();
        let dirt = rng.next_u32();
        let wide = ((dirt as u64) << 32) | low32 as u64;

        // Per the SysV ABI only the low 32 bits of the return register are
        // meaningful for a `uint32_t` result, so mask before comparing.
        let c = unsafe { cw(wide) } & 0xFFFF_FFFF;
        let r = unsafe { rw(wide) } & 0xFFFF_FFFF;
        assert_eq!(
            c, r,
            "[E10] divergence for wide arg 0x{wide:016X} (low 0x{low32:08X})"
        );

        // The dirty half must not change the answer the narrow ABI gives.
        let narrow = assert_same("E10", low32) as u64;
        assert_eq!(
            c, narrow,
            "[E10] dirty upper half changed the result for 0x{low32:08X}"
        );
    }
}
