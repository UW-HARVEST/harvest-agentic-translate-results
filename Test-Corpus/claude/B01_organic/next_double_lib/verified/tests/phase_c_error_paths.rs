//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundary conditions.
//!
//! The C library has **no** error-return surface (verified mechanically in
//! `ERRORS.md`): `next_double` is total over all 2^128 states and never signals
//! failure. The error surface is therefore the FFI boundary itself, which is
//! what these tests pin down.

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

use common::{CnRnd, load_pair};

/// Env var used to re-enter this test binary as a child that deliberately
/// performs the NULL dereference.
const CHILD_VAR: &str = "HARVEST_NULL_DEREF_IMPL";

/// ERRORS.md row 1 — `rnd == NULL`.
///
/// The C code dereferences `rnd` unconditionally (`lib.c:4`, no NULL check), so
/// passing NULL is undefined behaviour that manifests as a fatal signal rather
/// than an error code. Both implementations must terminate the *same way* —
/// same signal, and neither may "recover" and return a value.
///
/// The dereference is performed in a child process so this test can observe the
/// termination status of each implementation.
#[test]
fn null_pointer_terminates_identically() {
    let c = run_null_deref_child("c");
    let rust = run_null_deref_child("rust");

    eprintln!("null-deref outcome: C = {c:?}, Rust = {rust:?}");

    assert_eq!(
        c, rust,
        "ERRORS.md row 1: C and Rust must terminate identically on a NULL \
         `cn_rnd_t *`.\n  C    = {c:?}\n  Rust = {rust:?}"
    );

    // And specifically: neither may survive the NULL dereference. The C code
    // has no NULL check, so a clean exit would mean the Rust side had added one.
    match c {
        Outcome::Signal(sig) => assert!(
            sig == 11 || sig == 7 || sig == 6,
            "expected SIGSEGV/SIGBUS/SIGABRT from the NULL dereference, got signal {sig}"
        ),
        other => panic!("expected the NULL dereference to raise a fatal signal, got {other:?}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_null_deref_child(which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", "null_deref_child", "--ignored", "--nocapture"])
        .env(CHILD_VAR, which)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to spawn the null-deref child process");

    match status.signal() {
        Some(sig) => Outcome::Signal(sig),
        None => Outcome::Exit(status.code().unwrap_or(-1)),
    }
}

/// Child half of [`null_pointer_terminates_identically`]. Not run directly.
#[test]
#[ignore = "child process helper, spawned by null_pointer_terminates_identically"]
fn null_deref_child() {
    let which = std::env::var(CHILD_VAR).unwrap_or_default();
    let p = load_pair();
    let imp = match which.as_str() {
        "c" => &p.c,
        "rust" => &p.rust,
        other => panic!("child invoked without a valid {CHILD_VAR} (got {other:?})"),
    };
    eprintln!("child: calling {}::next_double(NULL)", imp.name);
    let bits = unsafe { imp.call_raw(std::ptr::null_mut()) };
    // Reaching this line means the implementation did NOT fault, i.e. it added
    // a NULL check the C source does not have. Use a distinctive exit code.
    eprintln!("child: survived NULL deref, returned {bits:#018x}");
    std::process::exit(42);
}

/// ERRORS.md row 2 — misaligned `cn_rnd_t *`.
///
/// The C code has no alignment check and issues plain 64-bit loads/stores, so on
/// x86-64 an unaligned pointer is handled transparently. Both implementations
/// must read the same bytes, return the same bit pattern, and write back the
/// same bytes.
#[test]
fn misaligned_pointer_matches() {
    let p = load_pair();
    let mut r = common::rng();

    for case in 0..128 {
        for off in 1usize..8 {
            let s = r.next_state();
            let build = || {
                // 32-byte aligned scratch; struct placed at byte offset `off`.
                let mut buf = [0u8; 32];
                buf[off..off + 8].copy_from_slice(&s.state[0].to_ne_bytes());
                buf[off + 8..off + 16].copy_from_slice(&s.state[1].to_ne_bytes());
                buf
            };
            let mut cbuf = build();
            let mut rbuf = build();

            let cbits = unsafe { p.c.call_raw(cbuf.as_mut_ptr().add(off).cast::<CnRnd>()) };
            let rbits = unsafe { p.rust.call_raw(rbuf.as_mut_ptr().add(off).cast::<CnRnd>()) };

            assert_eq!(
                cbits, rbits,
                "ERRORS.md row 2: misaligned (offset {off}) return bits differ \
                 for state {:#018x?}",
                s.state
            );
            assert_eq!(
                cbuf, rbuf,
                "ERRORS.md row 2: misaligned (offset {off}, case {case}) buffers \
                 differ after the call for state {:#018x?}",
                s.state
            );
        }
    }
}

/// ERRORS.md row 3 — the all-zero state is NOT rejected.
///
/// A hardened PRNG would refuse the xorshift128+ zero state; this C code does
/// not. Both implementations must accept it and return the same value, with no
/// error signalling of any kind.
#[test]
fn zero_state_is_not_rejected() {
    let p = load_pair();
    let mut cs = CnRnd::new(0, 0);
    let mut rs = CnRnd::new(0, 0);
    for i in 0..256 {
        let cb = p.c.call(&mut cs);
        let rb = p.rust.call(&mut rs);
        assert_eq!(cb, rb, "ERRORS.md row 3: iteration {i} bits differ");
        assert_eq!(cb, 0, "ERRORS.md row 3: expected +0.0 (no rejection)");
        assert_eq!(cs, CnRnd::new(0, 0), "ERRORS.md row 3: C state must stay zero");
        assert_eq!(cs, rs, "ERRORS.md row 3: iteration {i} state differs");
    }
}

/// ERRORS.md row 4 — the saturated state is NOT rejected.
#[test]
fn saturated_state_is_not_rejected() {
    let p = load_pair();
    p.assert_stream_eq(
        "ERRORS.md row 4: saturated state",
        CnRnd::new(u64::MAX, u64::MAX),
        256,
    );
    // Also the two half-saturated variants.
    p.assert_stream_eq("ERRORS.md row 4: lo saturated", CnRnd::new(u64::MAX, 0), 256);
    p.assert_stream_eq("ERRORS.md row 4: hi saturated", CnRnd::new(0, u64::MAX), 256);
}

/// Generic boundary: values one step past every "interesting" magnitude.
///
/// No parameter of `next_double` has a documented valid range, so the analogue
/// is the numeric boundaries of the two state words. Each word is driven with
/// `0`, `1`, `MAX-1`, `MAX`, the powers of two and their neighbours (`2^k - 1`,
/// `2^k`, `2^k + 1`), crossed against each other.
#[test]
fn numeric_boundaries_cross_product() {
    let p = load_pair();

    let mut vals: Vec<u64> = vec![0, 1, 2, u64::MAX - 1, u64::MAX];
    for k in 0..64u32 {
        let pow = 1u64 << k;
        vals.push(pow.wrapping_sub(1));
        vals.push(pow);
        vals.push(pow.wrapping_add(1));
    }
    vals.sort_unstable();
    vals.dedup();

    for (i, &a) in vals.iter().enumerate() {
        for &b in vals.iter() {
            p.assert_stream_eq(
                &format!("boundary/({a:#x},{b:#x})"),
                CnRnd::new(a, b),
                2,
            );
        }
        // Keep the runtime sane while still covering the full cross product.
        let _ = i;
    }
}

/// Generic boundary: there is no `enum` or integer mode/flag parameter in the
/// public API, so "out-of-range enum value across FFI" has no representative
/// input. This test documents and *enforces* that fact: if the API ever gains
/// such a parameter, the signature assertion below stops compiling/matching.
#[test]
fn no_enum_or_flag_parameter_exists() {
    // The exported signature is exactly `double next_double(cn_rnd_t *)`.
    // Loading it under that exact type from BOTH .so files is the strongest
    // check available at this boundary.
    let p = load_pair();
    let mut s = CnRnd::new(0xDEAD_BEEF_DEAD_BEEF, 0xCAFE_BABE_CAFE_BABE);
    let mut t = s;
    assert_eq!(p.c.call(&mut s), p.rust.call(&mut t));
    assert_eq!(s, t);

    // And confirm the C header really declares no enum / no second parameter.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/include/lib.h"),
    )
    .expect("read lib.h");
    assert!(
        !hdr.contains("enum"),
        "lib.h gained an enum; ERRORS.md must grow an out-of-range-enum row"
    );
    assert!(
        hdr.contains("double next_double(cn_rnd_t *rnd);"),
        "the public signature changed; re-derive ERRORS.md / CONFIGS.md"
    );
}

/// Generic boundary: repeated calls must never trap, panic, or abort in Rust
/// where C simply computes (e.g. no debug-build overflow panic from `x + y`,
/// no shift-overflow panic from `x << 23`). Running the whole loop inside the
/// test process would abort it on any such panic, which is exactly the
/// detection we want.
#[test]
fn no_rust_panic_where_c_computes() {
    let p = load_pair();
    let mut r = common::rng();
    // States picked to maximise the chance of arithmetic edge cases.
    for i in 0..8192 {
        let s = match i % 4 {
            0 => CnRnd::new(u64::MAX, u64::MAX),
            1 => CnRnd::new(1u64 << 63, 1u64 << 63),
            2 => {
                let x = r.next_u64();
                CnRnd::new(x, (!x).wrapping_add(1))
            }
            _ => r.next_state(),
        };
        p.assert_stream_eq(&format!("no-panic/case{i}"), s, 4);
    }
}
