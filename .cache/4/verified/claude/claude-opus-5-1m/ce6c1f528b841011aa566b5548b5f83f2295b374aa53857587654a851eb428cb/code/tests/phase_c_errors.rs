// Phase C — error/rejection-path differential tests, one per ERRORS.md row.
//
// The C library has NO explicit error surface (no error returns, no asserts, no
// null checks, no range checks — see ERRORS.md), so these rows cover the
// implicit rejection behaviour it does have: faulting pointers, signed-overflow
// wraparound at the range boundaries, zero/negative/oversized `iterations`, and
// arbitrary out-of-range integer values crossing the FFI boundary.
//
// Each row asserts C and Rust agree on the SAME outcome — the same fatal signal
// for the faulting rows, the same wrapped value for the overflow rows, and the
// same (empty) output for the rejected-length rows.
//
// Run single-threaded:  cargo test -- --test-threads=1

mod common;

use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const MIN: c_int = c_int::MIN;
const MAX: c_int = c_int::MAX;

// ---------------------------------------------------------------------------
// E1 / E2 — faulting pointers, run in a child process
// ---------------------------------------------------------------------------

/// Child-process worker. Inert unless the driving env vars are set, so it is a
/// no-op during a normal test run.
#[test]
fn crash_child_helper() {
    let (Ok(which), Ok(kind)) = (
        std::env::var("STATICALIAS_CRASH_LIB"),
        std::env::var("STATICALIAS_CRASH_PTR"),
    ) else {
        return; // not the child: nothing to do
    };

    let lib = load_single(&which);
    let p: *mut c_int = match kind.as_str() {
        "null" => std::ptr::null_mut(),
        "wild" => 0x1usize as *mut c_int,
        other => panic!("unknown pointer kind {other:?}"),
    };

    // The C dereferences `*outer` with no null check; this must fault.
    let ret = unsafe { (lib.static_alias)(p) };

    // Reaching here means no fault happened — report it distinctly.
    println!("NO_FAULT ret={ret:?}");
    std::process::exit(42);
}

/// Runs the helper in a child and returns (signal, exit_code).
fn run_crash_child(which: &str, kind: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["crash_child_helper", "--exact", "--test-threads=1"])
        .env("STATICALIAS_CRASH_LIB", which)
        .env("STATICALIAS_CRASH_PTR", kind)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn crash child");
    (out.status.signal(), out.status.code())
}

#[test]
fn err_e1_null_pointer_segv_both() {
    let c = run_crash_child("c", "null");
    let r = run_crash_child("rust", "null");
    assert_eq!(
        c, r,
        "static_alias(NULL): C and Rust must fail identically (signal, code)"
    );
    assert_eq!(
        c.0,
        Some(libc::SIGSEGV),
        "static_alias(NULL) must die on SIGSEGV, got {c:?}"
    );
}

#[test]
fn err_e2_wild_pointer_segv_both() {
    let c = run_crash_child("c", "wild");
    let r = run_crash_child("rust", "wild");
    assert_eq!(
        c, r,
        "static_alias(0x1): C and Rust must fail identically (signal, code)"
    );
    assert_eq!(
        c.0,
        Some(libc::SIGSEGV),
        "static_alias(0x1) must die on SIGSEGV, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// E3 — `inner += *outer` overflow at INT_MAX
// ---------------------------------------------------------------------------
#[test]
fn err_e3_inner_add_overflow_intmax() {
    let mut h = harness();
    for base in [1, 2, 7, 1 << 30, MAX] {
        h.set_inner(base);
        let o = h.sa(MAX); // MAX >= base  =>  then branch, inner += MAX
        assert_eq!(o.cls, Cls::Inner);
        assert_eq!(
            o.ret_val,
            base.wrapping_add(MAX),
            "inner({base}) += INT_MAX must wrap"
        );
        assert_eq!(o.buf_after, MAX, "then branch must not touch *outer");
    }
    // The canonical case: 1 + INT_MAX == INT_MIN
    h.set_inner(1);
    let o = h.sa(MAX);
    assert_eq!(o.ret_val, MIN);
}

// ---------------------------------------------------------------------------
// E4 — aliased doubling overflows, reaching the 0 fixpoint
// ---------------------------------------------------------------------------
#[test]
fn err_e4_aliased_doubling_overflow_to_zero() {
    let mut h = harness();
    h.set_inner(1);
    let mut expect: c_int = 1;
    for step in 0..31 {
        let o = h.sa_aliased();
        expect = expect.wrapping_add(expect);
        assert_eq!(o.ret_val, expect, "doubling step {step}");
    }
    assert_eq!(expect, MIN, "2^31 wraps to INT_MIN");
    // One more doubling: INT_MIN + INT_MIN == 0
    let o = h.sa_aliased();
    assert_eq!(o.ret_val, 0);
    assert_eq!(h.probe(), 0);
    // 0 is absorbing
    for _ in 0..5 {
        assert_eq!(h.sa_aliased().ret_val, 0);
    }
}

// ---------------------------------------------------------------------------
// E5 — `*outer += inner` overflow at INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn err_e5_outer_add_overflow_intmin() {
    let mut h = harness();
    for base in [1, 2, 1000, 1 << 30, MAX] {
        h.set_inner(base);
        // INT_MIN < inner  =>  else branch, *outer += inner, wraps
        let o = h.sa(MIN);
        assert_eq!(o.cls, Cls::Outer);
        assert_eq!(
            o.buf_after,
            MIN.wrapping_add(base),
            "INT_MIN += inner({base}) must wrap"
        );
        assert_eq!(o.ret_val, o.buf_after);
        assert_eq!(h.probe(), base, "else branch leaves inner alone");
    }
}

// ---------------------------------------------------------------------------
// E6 — exactly one step below the `>=` branch boundary
// ---------------------------------------------------------------------------
#[test]
fn err_e6_one_below_branch_boundary() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 106);
    let mut bases: Vec<c_int> = vec![1, 0, -1, 2, MAX, MIN + 1, 1 << 30];
    for _ in 0..80 {
        bases.push(rng.i32_any());
    }
    for base in bases {
        if base == MIN {
            continue; // MIN-1 wraps to MAX, which is >= MIN: then branch, not else
        }
        h.set_inner(base);
        // one below -> else
        let o = h.sa(base - 1);
        assert_eq!(o.cls, Cls::Outer, "inner-1 must take the else branch");
        assert_eq!(h.probe(), base);
        // exactly at the boundary -> then
        let o = h.sa(base);
        assert_eq!(o.cls, Cls::Inner, "inner must take the then branch");
    }
}

// ---------------------------------------------------------------------------
// E7 — iterations == 0 ("zero length"): no output, no state change
// ---------------------------------------------------------------------------
#[test]
fn err_e7_zero_iterations_no_output() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 107);
    let mut inits: Vec<c_int> = vec![0, 1, -1, MAX, MIN];
    for _ in 0..40 {
        inits.push(rng.i32_any());
    }
    for initial in inits {
        let base = rng.i32_in(-1000, 1000);
        h.set_inner(base);
        let out = h.driver(initial, 0);
        assert!(
            out.is_empty(),
            "driver({initial}, 0) must print nothing, got {:?}",
            String::from_utf8_lossy(&out)
        );
        assert_eq!(h.probe(), base, "iterations==0 must not touch inner");
    }
}

// ---------------------------------------------------------------------------
// E8 — iterations < 0 ("negative length"): no output, no state change
// ---------------------------------------------------------------------------
#[test]
fn err_e8_negative_iterations_no_output() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 108);
    let mut iters: Vec<c_int> = vec![-1, -2, -1000, MIN, MIN + 1, -(1 << 30)];
    for _ in 0..40 {
        iters.push(-rng.i32_in(1, MAX));
    }
    for it in iters {
        let base = rng.i32_in(-1000, 1000);
        h.set_inner(base);
        let initial = rng.i32_any();
        let out = h.driver(initial, it);
        assert!(
            out.is_empty(),
            "driver({initial}, {it}) must print nothing, got {:?}",
            String::from_utf8_lossy(&out)
        );
        assert_eq!(h.probe(), base, "negative iterations must not touch inner");
    }
}

// ---------------------------------------------------------------------------
// E9 — extreme initial_value
// ---------------------------------------------------------------------------
#[test]
fn err_e9_extreme_initial_values() {
    let mut h = harness();
    for initial in [MIN, MIN + 1, MAX, MAX - 1] {
        for base in [1, 0, -1, 2, MAX, MIN + 1, 1 << 30] {
            for iters in [1, 2, 5] {
                h.set_inner(base);
                let out = h.driver(initial, iters);
                assert_eq!(
                    out,
                    model_driver_bytes(base, initial, iters),
                    "driver({initial},{iters}) with inner={base}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — "oversized length": many iterations, everything wrapping repeatedly
// ---------------------------------------------------------------------------
#[test]
fn err_e10_oversized_iterations() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 110);
    for _ in 0..10 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let initial = base + rng.i32_in(0, 1 << 20);
        let iters = 200;
        let out = h.driver(initial, iters);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, iters),
            "driver({initial},{iters}) with inner={base}"
        );
        let lines = out.iter().filter(|b| **b == b'\n').count();
        assert_eq!(lines as c_int, iters, "one line per iteration");
    }
}

// ---------------------------------------------------------------------------
// E11 — arbitrary / "out-of-range" integer bit patterns across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_e11_arbitrary_int_bit_patterns() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 111);

    // The API declares no enum, so every 32-bit pattern is a legal `int`. Feed
    // deliberately hostile values (all-ones, sign bit only, alternating bits,
    // and pure random) to both entry points; neither may reject them.
    let hostile: Vec<c_int> = vec![
        0,
        -1,                     // 0xFFFFFFFF
        MIN,                    // 0x80000000
        MAX,                    // 0x7FFFFFFF
        0x5555_5555u32 as c_int,
        0xAAAA_AAAAu32 as c_int,
        0x0000_FFFFu32 as c_int,
        0xFFFF_0000u32 as c_int,
        0xDEAD_BEEFu32 as c_int,
        0xCAFE_BABEu32 as c_int,
        1,
        -2,
    ];

    for &val in &hostile {
        let base = rng.i32_in(-1000, 1000);
        h.set_inner(base);
        let o = h.sa(val); // asserts C == Rust
        if val >= base {
            assert_eq!(o.cls, Cls::Inner);
            assert_eq!(o.ret_val, base.wrapping_add(val));
        } else {
            assert_eq!(o.cls, Cls::Outer);
            assert_eq!(o.buf_after, val.wrapping_add(base));
        }
    }

    // Same through `driver`, in both argument positions.
    for &initial in &hostile {
        for &it in &[0, 1, 2, 3] {
            let base = rng.i32_in(-1000, 1000);
            h.set_inner(base);
            let out = h.driver(initial, it);
            assert_eq!(out, model_driver_bytes(base, initial, it));
        }
    }
    // Hostile patterns in the `iterations` position too.
    for &it in &hostile {
        let base = rng.i32_in(-1000, 1000);
        h.set_inner(base);
        let initial = rng.i32_any();
        let clamped = if it > 0 && it <= 64 { it } else { it };
        if clamped > 64 {
            continue; // keep runtime bounded; large positives covered by E10
        }
        let out = h.driver(initial, clamped);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, clamped),
            "driver({initial}, {clamped}) inner={base}"
        );
    }

    // Fully random sweep over both parameters.
    for _ in 0..200 {
        let base = rng.i32_any();
        if base == MIN {
            continue;
        }
        h.set_inner(base);
        let val = rng.i32_any();
        h.sa_np(val);
    }
}
