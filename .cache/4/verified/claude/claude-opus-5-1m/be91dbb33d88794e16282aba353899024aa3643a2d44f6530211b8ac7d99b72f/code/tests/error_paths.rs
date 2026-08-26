// Phase C — error-path differential tests, one test per row of ERRORS.md.
//
// `sieve` has no explicit rejection path (see ERRORS.md), so these tests cover
// the implicit ones: the exact C semantics of the loop-exit predicate (truncating
// `%`), the ignored `printf` failure, the unguarded signed overflow, the domain
// extremes, and the generic FFI boundaries. Both `.so`s are always driven through
// their exported `sieve` symbol.

mod common;

use common::*;

/// E1: there is no error return; a child that calls `sieve` must exit 0 in both
/// implementations (no abort, no panic, no signal) and produce identical bytes.
#[test]
fn e1_no_error_return_path() {
    let (c, r) = funcs();
    for val in [9, 0, -1, 7, -37, 123_456] {
        let cc = child_run(c, val, Dest::Pipe);
        let rr = child_run(r, val, Dest::Pipe);
        assert_eq!(
            cc.status, 0,
            "C child for sieve({val}) exited with raw status {} ({cc:?})",
            cc.status
        );
        assert!(!cc.timed_out && !cc.capped, "C child cut short: {cc:?}");
        assert_eq!(
            cc.status, rr.status,
            "exit status differs for sieve({val}): C {} vs Rust {}",
            cc.status, rr.status
        );
        assert_eq!(cc, rr, "child result differs for sieve({val})");
    }
}

/// E2: `-1 % 10 == -1` in C (truncation toward zero), so the loop must NOT stop
/// at `-1`; a floor-mod translation would stop after a single line.
#[test]
fn e2_negative_remainder_never_equals_nine() {
    let (c, r) = funcs();
    let co = run_one(c, -1, CAP);
    let ro = run_one(r, -1, CAP);
    assert_eq!(co, ro, "sieve(-1) diverged");
    // Positive proof that neither stops early: 11 lines, not 1.
    assert_eq!(
        co.bytes.iter().filter(|&&b| b == b'\n').count(),
        11,
        "C did not run -1..9"
    );
    assert_eq!(ro.bytes.iter().filter(|&&b| b == b'\n').count(), 11);
}

/// E3: every negative value whose C remainder is `-9` (a naive `abs`/floor mod
/// would see "9" and break immediately).
#[test]
fn e3_negative_residue_minus_nine() {
    let vals = [-9, -19, -29, -99, -109, -1009, -100_009];
    for &v in &vals {
        assert_eq!(v % 10, -9, "test premise");
    }
    assert_same_all("residue -9", vals);
    let (c, r) = funcs();
    for &v in &vals {
        let co = run_one(c, v, CAP);
        let n = co.bytes.iter().filter(|&&b| b == b'\n').count();
        assert_eq!(n, (9 - v) as usize + 1, "C stopped early for sieve({v})");
        assert_eq!(
            run_one(r, v, CAP)
                .bytes
                .iter()
                .filter(|&&b| b == b'\n')
                .count(),
            n,
            "Rust line count differs for sieve({v})"
        );
    }
}

/// E4: negative multiples of ten (`val % 10 == 0`).
#[test]
fn e4_negative_multiple_of_ten() {
    let vals = [-10, -20, -100, -1000, -10_000];
    for &v in &vals {
        assert_eq!(v % 10, 0, "test premise");
    }
    assert_same_all("negative multiples of 10", vals);
}

/// E5: a `printf` write failure (`/dev/full`, ENOSPC) is ignored by the C: the
/// loop still terminates normally and the child exits 0. Rust must match.
#[test]
fn e5_printf_write_error_ignored() {
    let (c, r) = funcs();
    // -20000 => ~20k lines => libc's 4 KiB buffer is flushed (and fails) many
    // times inside `printf` itself, not only at the final fflush.
    for val in [-20_000, 3] {
        let cc = child_run(c, val, Dest::DevFull);
        let rr = child_run(r, val, Dest::DevFull);
        if cc.status == 97 << 8 {
            eprintln!("skipping E5: /dev/full unavailable");
            return;
        }
        assert!(!cc.timed_out, "C child hung on /dev/full for sieve({val})");
        assert_eq!(
            cc.status, 0,
            "C child on /dev/full for sieve({val}) raw status {}",
            cc.status
        );
        assert_eq!(
            cc.status, rr.status,
            "status differs on /dev/full for sieve({val}): C {} vs Rust {}",
            cc.status, rr.status
        );
        assert_eq!(
            cc.timed_out, rr.timed_out,
            "termination differs on /dev/full for sieve({val})"
        );
    }
}

/// E6: fd 1 closed (`EBADF`) — the ignored `printf` error must still let the
/// function return normally in both implementations.
#[test]
fn e6_stdout_closed() {
    let (c, r) = funcs();
    for val in [-1000, 9, 4] {
        let cc = child_run(c, val, Dest::Closed);
        let rr = child_run(r, val, Dest::Closed);
        assert!(
            !cc.timed_out,
            "C child hung with closed stdout for sieve({val})"
        );
        assert_eq!(
            cc.status, 0,
            "C child with closed stdout for sieve({val}) raw status {}",
            cc.status
        );
        assert_eq!(
            cc.status, rr.status,
            "status differs with closed stdout for sieve({val}): C {} vs Rust {}",
            cc.status, rr.status
        );
        assert_eq!(cc.timed_out, rr.timed_out);
    }
}

/// E7: the signed-overflow boundary `[INT_MAX-7, INT_MAX]`. The shipped C build
/// wraps to `INT_MIN` and keeps counting; Rust must wrap identically and must not
/// panic. Compared as an output prefix in a forked child.
#[test]
fn e7_int_max_overflow_wraps() {
    let (c, r) = funcs();
    const WANT: usize = 8192;
    for val in 2_147_483_640..=2_147_483_647i32 {
        let co = child_prefix(c, val, WANT);
        let ro = child_prefix(r, val, WANT);
        assert!(
            co.len() >= 1024,
            "C child produced only {} bytes for sieve({val})",
            co.len()
        );
        assert_eq!(co.len(), ro.len(), "prefix length differs for sieve({val})");
        assert!(co == ro, "overflow prefix differs for sieve({val})");
        // the wrap itself must be visible in both streams
        let ctext = String::from_utf8_lossy(&co).into_owned();
        assert!(
            ctext.contains("2147483647\n-2147483648\n"),
            "C did not wrap INT_MAX -> INT_MIN for sieve({val}); head={:?}",
            &ctext[..ctext.len().min(120)]
        );
        assert!(String::from_utf8_lossy(&ro).contains("2147483647\n-2147483648\n"));
    }
}

/// E8: `INT_MIN` is accepted, not rejected (prefix comparison; the full run is
/// ~2^31 lines).
#[test]
fn e8_int_min_accepted() {
    let (c, r) = funcs();
    const WANT: usize = 8192;
    let co = child_prefix(c, i32::MIN, WANT);
    let ro = child_prefix(r, i32::MIN, WANT);
    assert!(
        co.starts_with(b"-2147483648\n"),
        "C head: {:?}",
        String::from_utf8_lossy(&co[..co.len().min(24)])
    );
    assert_eq!(co.len(), ro.len());
    assert!(co == ro, "INT_MIN prefix differs");
}

/// E9 / C18: out-of-range bits across the FFI boundary — the caller passes a
/// 64-bit value whose high half is garbage; the `int` parameter must be the low
/// 32 bits for both implementations. (This is the FFI analogue of passing an
/// out-of-range enum value: a C API accepts any bit pattern.)
#[test]
fn e9_ffi_high_bits_ignored() {
    let (cw, rw) = funcs_wide();
    let (c, _) = funcs();
    const CAP1: usize = 1 << 20;
    let cases: [(i64, i32); 6] = [
        (0x7FFF_FFFF_0000_0003u64 as i64, 3),
        (-1i64, -1), // 0xFFFF_FFFF_FFFF_FFFF -> -1
        (0x0000_0001_0000_0009u64 as i64, 9),
        (0xDEAD_BEEF_0000_0000u64 as i64, 0),
        (0x1234_5678_FFFF_FFF7u64 as i64, -9),
        (0xFFFF_FFFF_7FFF_FFD7u64 as i64, 2_147_483_607),
    ];
    for (wide, narrow) in cases {
        let (co, c_capped) = capture_pipe_capped(|| unsafe { cw(wide) }, CAP1);
        let (ro, r_capped) = capture_pipe_capped(|| unsafe { rw(wide) }, CAP1);
        let expect = run_one(c, narrow, CAP).bytes;
        assert!(!co.is_empty(), "no C output for wide arg 0x{:016X}", wide as u64);
        assert!(!c_capped, "C exceeded cap for wide arg 0x{:016X}", wide as u64);
        assert_eq!(
            c_capped, r_capped,
            "cap behaviour differs for wide arg 0x{:016X}",
            wide as u64
        );
        assert_eq!(
            co, ro,
            "wide-arg call diverged for 0x{:016X} (low32 = {narrow})",
            wide as u64
        );
        assert_eq!(
            co, expect,
            "C did not truncate 0x{:016X} to {narrow}",
            wide as u64
        );
    }
}

/// E10: generic boundary sweep across the whole domain.
#[test]
fn e10_boundary_sweep() {
    // bounded ends (full comparison)
    assert_same_all(
        "boundaries (bounded)",
        [
            -1,
            0,
            1,
            8,
            9,
            10,
            11,
            -2,
            -8,
            -9,
            -10,
            -11,
            2_147_483_629,
            2_147_483_630,
            2_147_483_638,
            2_147_483_639,
        ],
    );
    // unbounded / overflowing ends (prefix comparison)
    let (c, r) = funcs();
    for val in [i32::MIN, i32::MIN + 1, i32::MAX] {
        let co = child_prefix(c, val, 4096);
        let ro = child_prefix(r, val, 4096);
        assert!(co.len() >= 1024, "C child output too short for sieve({val})");
        assert_eq!(co.len(), ro.len(), "prefix length differs for sieve({val})");
        assert!(co == ro, "prefix differs for sieve({val})");
    }
}

/// E11: the null-pointer / length class of errors does not exist in this ABI —
/// asserted structurally, so the row is provably N/A rather than untested.
#[test]
fn e11_no_pointer_parameters() {
    let header = include_str!("../c_src/include/sieve.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("sieve(") && !l.trim_start().starts_with("//"))
        .collect();
    assert_eq!(decls.len(), 1, "unexpected public declarations: {decls:?}");
    assert!(
        decls[0].contains("void sieve(int"),
        "declaration changed: {:?}",
        decls[0]
    );
    assert!(
        !decls[0].contains('*'),
        "a pointer parameter appeared: {:?}",
        decls[0]
    );
    assert_eq!(decls[0].matches(',').count(), 0, "more than one parameter");
    // both libraries really export the symbol exercised above
    assert!(c_lib_exports_sieve(), "C .so must export `sieve`");
    assert!(rust_lib_exports_sieve(), "Rust .so must export `sieve`");
}
