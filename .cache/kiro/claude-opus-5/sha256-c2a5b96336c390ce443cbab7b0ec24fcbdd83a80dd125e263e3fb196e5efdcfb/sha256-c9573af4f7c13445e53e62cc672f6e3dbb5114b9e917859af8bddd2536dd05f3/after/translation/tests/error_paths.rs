//! Phase C — error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. The only failure mode of `bin2hex` is
//! `abort()`, which terminates the process, so every call is made in a forked
//! child and the parent asserts the child died from exactly `SIGABRT` — for both
//! the C `.so` and the Rust `.so`.

mod common;

use common::{assert_both_abort, assert_both_accept, both, in_child, outcome_of, Outcome, Rng};
use std::ffi::c_char;

/// The literal in the C source: `(18446744073709551615UL) / 2`.
const LIMIT: usize = 9_223_372_036_854_775_807;

#[test]
fn limit_constant_matches_the_c_source() {
    assert_eq!(LIMIT, (18446744073709551615u64 / 2) as usize);
    assert_eq!(LIMIT, 0x7FFF_FFFF_FFFF_FFFF);
}

/// E1 — `bin_len == LIMIT` exactly, with `hex_maxlen = usize::MAX` so the second
/// `||` operand is false and only the first check can fire.
#[test]
fn e1_bin_len_at_limit() {
    let (c, r) = both();
    // usize::MAX > LIMIT * 2 (= 0xFFFF_FFFF_FFFF_FFFE), so operand 2 is false.
    assert!(usize::MAX > LIMIT.wrapping_mul(2));
    assert_both_abort(&c, &r, usize::MAX, LIMIT, false, "E1 bin_len == LIMIT");
}

/// E2 — `bin_len > LIMIT`.
#[test]
fn e2_bin_len_above_limit() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE000_0002);
    let mut cases = vec![
        LIMIT + 1,
        0x8000_0000_0000_0000,
        usize::MAX - 1,
        usize::MAX,
    ];
    for _ in 0..8 {
        cases.push(rng.range(LIMIT + 1, usize::MAX));
    }
    for bin_len in cases {
        assert_both_abort(
            &c,
            &r,
            usize::MAX,
            bin_len,
            false,
            &format!("E2 bin_len={bin_len}"),
        );
    }
}

/// E3 — `hex_maxlen == bin_len * 2` exactly: room for the digits but not the
/// NUL terminator. Real allocations are handed in to prove the guard, not a
/// segfault, is what kills the process.
#[test]
fn e3_hex_maxlen_exactly_two_n() {
    let (c, r) = both();
    let mut lens: Vec<usize> = vec![1, 2, 3, 7, 8, 16, 31, 32, 255, 256, 257, 1024];
    let mut rng = Rng::new(0xE000_0003);
    for _ in 0..12 {
        lens.push(rng.range(1, 8192));
    }
    for n in lens {
        assert_both_abort(&c, &r, n * 2, n, true, &format!("E3 n={n}"));
    }
}

/// E4 — `hex_maxlen < bin_len * 2`, including `hex_maxlen == 0`.
#[test]
fn e4_hex_maxlen_short() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE000_0004);
    for n in [1usize, 2, 3, 7, 8, 16, 31, 32, 255, 256, 257, 1024] {
        for hex_maxlen in [0usize, 1, n, n * 2 - 1] {
            assert_both_abort(
                &c,
                &r,
                hex_maxlen,
                n,
                true,
                &format!("E4 n={n} hex_maxlen={hex_maxlen}"),
            );
        }
        for _ in 0..4 {
            let hex_maxlen = rng.range(0, n * 2);
            assert_both_abort(
                &c,
                &r,
                hex_maxlen,
                n,
                true,
                &format!("E4 rand n={n} hex_maxlen={hex_maxlen}"),
            );
        }
    }
}

/// E5 — `hex_maxlen == 0` with `bin_len == 0`: even the empty input is rejected
/// because `0 <= 0`. Also exercised with NULL pointers, since the guard runs
/// before any dereference.
#[test]
fn e5_zero_maxlen_zero_len() {
    let (c, r) = both();
    assert_both_abort(&c, &r, 0, 0, true, "E5 real buffers");
    assert_both_abort(&c, &r, 0, 0, false, "E5 NULL pointers");
}

/// E6 — both `||` operands true at once.
#[test]
fn e6_both_conditions_true() {
    let (c, r) = both();
    for &(hm, bl) in &[
        (0usize, usize::MAX),
        (0, LIMIT),
        (1, usize::MAX),
        (LIMIT, usize::MAX),
    ] {
        assert_both_abort(&c, &r, hm, bl, false, &format!("E6 hm={hm} bl={bl}"));
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary rows G1..G8
// ---------------------------------------------------------------------------

/// G1 / G2 — NULL `hex` and/or NULL `bin` with `hex_maxlen == 0`: the guard
/// fires first, so both implementations abort rather than segfault.
#[test]
fn g1_g2_null_pointers_with_zero_maxlen() {
    let (c, r) = both();
    let mut real = vec![0u8; 64];
    let real_hex = real.as_mut_ptr().cast::<c_char>();
    let real_bin = real.as_ptr();
    let n: *mut c_char = std::ptr::null_mut();
    let nb: *const u8 = std::ptr::null();

    for (label, hexp, binp) in [
        ("G1 both NULL", n, nb),
        ("G2 hex NULL", n, real_bin),
        ("G2 bin NULL", real_hex, nb),
        ("G2 both real", real_hex, real_bin),
    ] {
        let oc = outcome_of(&c, hexp, 0, binp, 0);
        let or = outcome_of(&r, hexp, 0, binp, 0);
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGABRT),
            "{label}: C should abort on hex_maxlen == 0"
        );
        assert_eq!(or, oc, "{label}: Rust outcome differs from C ({oc:?} vs {or:?})");
    }
}

/// G3 — NULL `bin` with `bin_len == 0` and a usable `hex`: accepted by both,
/// writing exactly one NUL byte.
#[test]
fn g3_null_bin_is_accepted_when_len_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0x6000_0003);
    for i in 0..64 {
        let hex_maxlen = if i == 0 { 1 } else { rng.range(1, 256) };
        let mut hex_c = vec![0xAAu8; hex_maxlen + common::GUARD];
        let mut hex_r = vec![0xAAu8; hex_maxlen + common::GUARD];

        // Both must survive the call (no abort, no segfault) and produce the
        // same buffer. Run in a child first to prove neither dies.
        let hc = hex_c.as_mut_ptr() as usize;
        let hr = hex_r.as_mut_ptr() as usize;
        let cf = c.bin2hex;
        let rf = r.bin2hex;
        assert_eq!(
            in_child(|| unsafe {
                std::hint::black_box(cf(hc as *mut c_char, hex_maxlen, std::ptr::null(), 0));
            }),
            Outcome::Exited(0),
            "G3 i={i}: C aborted on NULL bin with bin_len == 0"
        );
        assert_eq!(
            in_child(|| unsafe {
                std::hint::black_box(rf(hr as *mut c_char, hex_maxlen, std::ptr::null(), 0));
            }),
            Outcome::Exited(0),
            "G3 i={i}: Rust aborted on NULL bin with bin_len == 0"
        );

        let rc = unsafe { (c.bin2hex)(hex_c.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0) };
        let rr = unsafe { (r.bin2hex)(hex_r.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0) };
        assert_eq!(rc.cast::<u8>(), hex_c.as_mut_ptr());
        assert_eq!(rr.cast::<u8>(), hex_r.as_mut_ptr());
        assert_eq!(hex_c, hex_r, "G3 i={i}: buffers diverged");
        assert_eq!(hex_c[0], 0, "G3 i={i}: NUL not written");
        assert_eq!(hex_c[1], 0xAA, "G3 i={i}: wrote past the terminator");
    }
}

/// G4 — smallest accepted call: `bin_len == 0`, `hex_maxlen == 1`.
#[test]
fn g4_smallest_accepted() {
    let (c, r) = both();
    assert_both_accept(&c, &r, 1, 0, "G4");
    let mut hex_c = [0xAAu8; 16];
    let mut hex_r = [0xAAu8; 16];
    let bin = [0u8; 1];
    unsafe {
        (c.bin2hex)(hex_c.as_mut_ptr().cast(), 1, bin.as_ptr(), 0);
        (r.bin2hex)(hex_r.as_mut_ptr().cast(), 1, bin.as_ptr(), 0);
    }
    assert_eq!(hex_c, hex_r);
    assert_eq!(hex_c[0], 0);
    assert_eq!(hex_c[1], 0xAA);
}

/// G5 — one step either side of the `bin_len` range boundary.
#[test]
fn g5_bin_len_range_boundary() {
    let (c, r) = both();
    // Exactly at the limit -> rejected by operand 1.
    assert_both_abort(&c, &r, usize::MAX, LIMIT, false, "G5 at limit");
    // One below the limit -> operand 1 false, but no real `hex_maxlen` can
    // exceed (LIMIT-1)*2 = 0xFFFF_FFFF_FFFF_FFFC, so operand 2 fires.
    assert_both_abort(
        &c,
        &r,
        usize::MAX - 4,
        LIMIT - 1,
        false,
        "G5 one below, hm too small",
    );

    // The only `hex_maxlen` values the guard accepts for `bin_len == LIMIT - 1`.
    // The loop would then write ~16 EiB, so the observable behaviour is "runs
    // off the end of the buffer". Real mappings with a PROT_NONE guard page make
    // that deterministic: both implementations must reach the loop and die from
    // SIGSEGV at the guard page, NOT from the SIGABRT of the rejection path.
    let hex_region = common::GuardedRegion::new(2);
    let bin_region = common::GuardedRegion::new(2);
    bin_region.fill(0x5A);
    hex_region.fill(0xAA);
    for hm in [usize::MAX - 2, usize::MAX - 1, usize::MAX] {
        assert!(
            !(LIMIT - 1 >= LIMIT || hm <= (LIMIT - 1).wrapping_mul(2)),
            "G5 hm={hm}: test bug, the source-derived guard rejects this"
        );
        let oc = outcome_of(
            &c,
            hex_region.ptr().cast(),
            hm,
            bin_region.ptr(),
            LIMIT - 1,
        );
        let or = outcome_of(
            &r,
            hex_region.ptr().cast(),
            hm,
            bin_region.ptr(),
            LIMIT - 1,
        );
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGSEGV),
            "G5 hm={hm}: C should have passed the guard and run off the buffer"
        );
        assert_eq!(oc, or, "G5 hm={hm}: outcomes differ (C {oc:?}, Rust {or:?})");
    }
}

/// G6 — maximum slack `hex_maxlen` accepted, nothing written past
/// `bin_len * 2`.
#[test]
fn g6_hex_maxlen_usize_max_accepted() {
    let (c, r) = both();
    let mut rng = Rng::new(0x6000_0006);
    for i in 0..64 {
        let n = rng.range(0, 32);
        let bin = rng.bytes(n);
        let alloc = n * 2 + 1 + common::GUARD;
        let mut hex_c = vec![0xAAu8; alloc];
        let mut hex_r = vec![0xAAu8; alloc];
        let rc = unsafe {
            (c.bin2hex)(hex_c.as_mut_ptr().cast(), usize::MAX, bin.as_ptr(), n)
        };
        let rr = unsafe {
            (r.bin2hex)(hex_r.as_mut_ptr().cast(), usize::MAX, bin.as_ptr(), n)
        };
        assert_eq!(rc.cast::<u8>(), hex_c.as_mut_ptr());
        assert_eq!(rr.cast::<u8>(), hex_r.as_mut_ptr());
        assert_eq!(hex_c, hex_r, "G6 i={i} n={n}");
        assert!(
            hex_c[n * 2 + 1..].iter().all(|&b| b == 0xAA),
            "G6 i={i}: wrote past bin_len*2"
        );
    }
}

/// G7 — there is no enum / mode / flag parameter in the API, so the
/// "out-of-range enum value" class does not exist for `bin2hex`. What *can* be
/// out of range is the two `size_t` arguments, whose entire value space is
/// swept here across the guard's decision boundary so that C and Rust are shown
/// to agree on accept-vs-abort for every shape of argument, including values a
/// C caller could pass by mistake.
#[test]
fn g7_full_size_t_argument_sweep_agrees() {
    let (c, r) = both();

    // The guard decision is a pure function of (hex_maxlen, bin_len). Compute
    // it from the C source and confirm both .so's agree, over a wide sweep.
    fn c_guard_aborts(hex_maxlen: usize, bin_len: usize) -> bool {
        bin_len >= LIMIT || hex_maxlen <= bin_len.wrapping_mul(2)
    }

    let mut rng = Rng::new(0x6000_0007);
    let mut probes: Vec<(usize, usize)> = Vec::new();
    let interesting: [usize; 14] = [
        0,
        1,
        2,
        3,
        63,
        64,
        255,
        256,
        LIMIT - 1,
        LIMIT,
        LIMIT + 1,
        0x8000_0000_0000_0000,
        usize::MAX - 1,
        usize::MAX,
    ];
    for &hm in &interesting {
        for &bl in &interesting {
            probes.push((hm, bl));
        }
    }
    for _ in 0..24 {
        probes.push((rng.next_u64() as usize, rng.next_u64() as usize));
    }

    for (hm, bl) in probes {
        let expect_abort = c_guard_aborts(hm, bl);
        // Only probe the *reject* verdicts in a child with NULL pointers: an
        // accepted verdict with a huge bin_len would try to read unmapped
        // memory, which is covered by the valid-path tests instead.
        if !expect_abort {
            continue;
        }
        let oc = outcome_of(&c, std::ptr::null_mut(), hm, std::ptr::null(), bl);
        let or = outcome_of(&r, std::ptr::null_mut(), hm, std::ptr::null(), bl);
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGABRT),
            "G7 hm={hm} bl={bl}: C verdict mismatch with the source-derived guard"
        );
        assert_eq!(oc, or, "G7 hm={hm} bl={bl}: C {oc:?} vs Rust {or:?}");
    }
}

/// G8 — the return value is always the `hex` argument, never NULL, on every
/// accepted call.
#[test]
fn g8_return_is_the_hex_argument() {
    let (c, r) = both();
    let mut rng = Rng::new(0x6000_0008);
    for _ in 0..200 {
        let n = rng.range(0, 200);
        let bin = rng.bytes(n);
        let off = rng.range(0, 15);
        let mut buf_c = vec![0u8; off + n * 2 + 1 + common::GUARD];
        let mut buf_r = vec![0u8; off + n * 2 + 1 + common::GUARD];
        let pc = unsafe { buf_c.as_mut_ptr().add(off) };
        let pr = unsafe { buf_r.as_mut_ptr().add(off) };
        let rc = unsafe { (c.bin2hex)(pc.cast(), n * 2 + 1, bin.as_ptr(), n) };
        let rr = unsafe { (r.bin2hex)(pr.cast(), n * 2 + 1, bin.as_ptr(), n) };
        assert!(!rc.is_null() && !rr.is_null());
        assert_eq!(rc.cast::<u8>(), pc);
        assert_eq!(rr.cast::<u8>(), pr);
        assert_eq!(buf_c, buf_r);
    }
}
