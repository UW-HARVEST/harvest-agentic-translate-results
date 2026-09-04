//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C library has no error-return channel (see `ERRORS.md` for the
//! mechanical grep evidence), so the rows cover the UB/boundary cases that do
//! exist plus explicit, source-verified not-applicable rows.

mod common;
use common::{both, c_so_path, rust_release_so_path, rust_so_path, CnRnd, SplitMix64};

use std::path::PathBuf;
use std::process::Command;

/// Run `next_double(NULL)` against `so` in a child process and report
/// `(exit_code, fatal_signal)`.
fn null_call_outcome(so: &PathBuf) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["phase_c_e1_null_pointer_both_segfault", "--exact", "--nocapture"])
        .env("E1_NULL_TARGET", so)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    #[cfg(unix)]
    let sig = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let sig: Option<i32> = None;
    (out.status.code(), sig)
}

// ------------------------------------------------------------------ E1
/// `rnd == NULL`: the C dereferences unconditionally with no null check, so it
/// dies with SIGSEGV. Each library is exercised in a child process and the two
/// outcomes are compared exactly (same exit code AND same fatal signal), not
/// merely "both failed somehow".
///
/// The comparison is made against the **release** Rust cdylib, because that is
/// the shipped artifact and is built the same way the C `.so` is: with no
/// undefined-behaviour instrumentation. A `debug_assertions` build enables
/// Rust's `ub_checks`, which intentionally trap the null dereference and abort
/// (SIGABRT) instead of faulting; that is a development diagnostic, not an ABI
/// difference, and is asserted separately below.
#[test]
fn phase_c_e1_null_pointer_both_segfault() {
    // Child mode: dlopen the given .so, call next_double(NULL), exit 0 if it
    // somehow returns.
    if let Ok(target) = std::env::var("E1_NULL_TARGET") {
        let lib = unsafe { libloading::Library::new(&target) }.expect("child dlopen");
        let f: libloading::Symbol<unsafe extern "C" fn(*mut CnRnd) -> f64> =
            unsafe { lib.get(b"next_double\0") }.expect("child symbol");
        let v = unsafe { f(std::ptr::null_mut()) };
        println!("returned {v}");
        std::process::exit(0);
    }

    const SIGSEGV: i32 = 11;
    const SIGABRT: i32 = 6;

    let (c_code, c_sig) = null_call_outcome(&c_so_path());
    assert_eq!(
        (c_code, c_sig),
        (None, Some(SIGSEGV)),
        "E1: expected the C library to die with SIGSEGV on NULL, \
         got code={c_code:?} signal={c_sig:?}"
    );

    // Primary differential assertion: shipped (release) artifacts must match.
    let (r_code, r_sig) = null_call_outcome(&rust_release_so_path());
    assert_eq!(
        (c_code, c_sig),
        (r_code, r_sig),
        "E1: NULL-pointer behaviour differs between the C .so and the release Rust .so.\n  \
         C   : code={c_code:?} signal={c_sig:?}\n  Rust: code={r_code:?} signal={r_sig:?}"
    );

    // Secondary: whatever profile the tests themselves run under must still die
    // fatally on NULL — it must never silently return a value.
    let (d_code, d_sig) = null_call_outcome(&rust_so_path());
    assert!(
        d_sig == Some(SIGSEGV) || d_sig == Some(SIGABRT),
        "E1: the test-profile Rust .so did not die fatally on NULL \
         (code={d_code:?} signal={d_sig:?}) — it must not accept a null pointer"
    );
    if !common::tests_have_debug_assertions() {
        assert_eq!(
            d_sig,
            Some(SIGSEGV),
            "E1: without debug_assertions the Rust .so must fault exactly like the C"
        );
    }
}

// ------------------------------------------------------------------ E2
/// Misaligned `cn_rnd_t`: no alignment check exists in the C, and x86-64
/// tolerates unaligned 8-byte access, so both must succeed and agree.
#[test]
fn phase_c_e2_misaligned_state() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xE2_5EED_0000_00E2);

    for i in 0..256 {
        let (x, y) = (g.next_u64(), g.next_u64());

        // Two independent misaligned buffers, one per library, at odd offsets.
        for off in [1usize, 3, 5, 7] {
            let mut cbuf = vec![0u8; 16 + 8];
            let mut rbuf = vec![0u8; 16 + 8];
            cbuf[off..off + 8].copy_from_slice(&x.to_ne_bytes());
            cbuf[off + 8..off + 16].copy_from_slice(&y.to_ne_bytes());
            rbuf[off..off + 8].copy_from_slice(&x.to_ne_bytes());
            rbuf[off + 8..off + 16].copy_from_slice(&y.to_ne_bytes());

            let cp = unsafe { cbuf.as_mut_ptr().add(off) } as *mut CnRnd;
            let rp = unsafe { rbuf.as_mut_ptr().add(off) } as *mut CnRnd;
            assert_eq!(cp as usize % 8, off % 8, "E2: buffer not misaligned as intended");

            let cv = unsafe { c.call_raw(cp) }.to_bits();
            let rv = unsafe { r.call_raw(rp) }.to_bits();
            assert_eq!(cv, rv, "E2 #{i} off={off}: return diverged on misaligned state");
            assert_eq!(
                &cbuf[off..off + 16],
                &rbuf[off..off + 16],
                "E2 #{i} off={off}: written-back state bytes diverged"
            );

            // And it must equal the aligned result for the same logical input.
            let mut aligned = CnRnd::new(x, y);
            let expect = c.next_bits(&mut aligned);
            assert_eq!(cv, expect, "E2 #{i} off={off}: misaligned != aligned (C)");
        }
    }
}

// ------------------------------------------------------------------ E3
/// The struct sits flush against the end of a mapped region: the code must read
/// and write exactly 16 bytes and never touch the guard bytes past `state[1]`.
#[test]
fn phase_c_e3_no_overread_past_struct() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xE3_5EED_0000_00E3);

    const GUARD: usize = 32;
    for i in 0..256 {
        let (x, y) = (g.next_u64(), g.next_u64());

        // Layout: [16-byte struct][GUARD sentinel bytes]. If either library
        // over-writes, the sentinel changes; over-reads would show up as a
        // value difference because the two buffers carry different sentinels.
        let mut cbuf = vec![0u8; 16 + GUARD];
        let mut rbuf = vec![0u8; 16 + GUARD];
        cbuf[..8].copy_from_slice(&x.to_ne_bytes());
        cbuf[8..16].copy_from_slice(&y.to_ne_bytes());
        rbuf[..8].copy_from_slice(&x.to_ne_bytes());
        rbuf[8..16].copy_from_slice(&y.to_ne_bytes());
        for k in 0..GUARD {
            cbuf[16 + k] = 0xA5;
            rbuf[16 + k] = 0x5A; // deliberately DIFFERENT sentinels
        }

        let cv = unsafe { c.call_raw(cbuf.as_mut_ptr() as *mut CnRnd) }.to_bits();
        let rv = unsafe { r.call_raw(rbuf.as_mut_ptr() as *mut CnRnd) }.to_bits();

        assert_eq!(
            cv, rv,
            "E3 #{i}: return diverged — one side may be reading past state[1]"
        );
        assert_eq!(&cbuf[..16], &rbuf[..16], "E3 #{i}: state write-back diverged");
        assert!(
            cbuf[16..].iter().all(|&b| b == 0xA5),
            "E3 #{i}: C wrote past the struct"
        );
        assert!(
            rbuf[16..].iter().all(|&b| b == 0x5A),
            "E3 #{i}: Rust wrote past the struct"
        );
    }
}

// ------------------------------------------------------------------ E4
/// No `enum` / mode / flag parameter exists anywhere in the public API, so
/// "out-of-range enum value across FFI" is vacuous. Assert the justification
/// still holds against the real header, so this row cannot silently rot.
#[test]
fn phase_c_e4_no_enum_parameters_exist() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read lib.h");

    assert!(
        !header.contains("enum"),
        "E4: lib.h now declares an enum — ERRORS.md row E4 must be replaced with \
         real out-of-range-enum differential tests:\n{header}"
    );
    // The single public function takes exactly one pointer parameter.
    assert!(
        header.contains("double next_double(cn_rnd_t *rnd);"),
        "E4: public signature changed; re-derive ERRORS.md:\n{header}"
    );
}

// ------------------------------------------------------------------ E5
/// No length / size / count / buffer parameter exists, so "zero and oversized
/// lengths" is vacuous. The degenerate all-zero *value* case is a valid input
/// covered by CONFIGS.md row C1; re-checked here for the boundary requirement.
#[test]
fn phase_c_e5_no_length_parameters_exist() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read lib.h");

    // NB: deliberately does not include the substring `n_`, which occurs
    // innocently inside `cn_rnd_t`.
    for kw in ["size_t", "len", "count", "size", "num", "nmemb", "capacity"] {
        assert!(
            !header.contains(kw),
            "E5: lib.h now mentions `{kw}` — a length parameter may have appeared; \
             re-derive ERRORS.md"
        );
    }
    assert!(
        header.contains("uint64_t state[2]"),
        "E5: state is no longer a fixed uint64_t[2]; re-derive ERRORS.md"
    );

    // Boundary: the smallest and largest possible state values.
    let (c, r) = both();
    for st in [
        CnRnd::new(0, 0),
        CnRnd::new(0, 1),
        CnRnd::new(1, 0),
        CnRnd::new(u64::MAX, 0),
        CnRnd::new(0, u64::MAX),
        CnRnd::new(u64::MAX, u64::MAX),
    ] {
        let mut cs = st;
        let mut rs = st;
        for k in 0..4 {
            assert_eq!(
                c.next_bits(&mut cs),
                r.next_bits(&mut rs),
                "E5: boundary state {st:?} diverged at call {k}"
            );
            assert_eq!(cs, rs, "E5: boundary state {st:?} state diverged at call {k}");
        }
    }
}
