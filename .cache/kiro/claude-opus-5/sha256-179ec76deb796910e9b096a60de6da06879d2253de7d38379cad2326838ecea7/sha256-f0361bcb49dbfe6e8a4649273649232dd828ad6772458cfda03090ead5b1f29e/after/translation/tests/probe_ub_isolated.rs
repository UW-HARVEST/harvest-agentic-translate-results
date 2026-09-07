//! Fork-isolated characterization of the C's genuinely UNDEFINED paths.
//!
//! Two inputs make the C read or write memory it does not own. They cannot be
//! asserted in-process because the C may segfault, so each call runs in a forked
//! child and the exit status is the measurement:
//!
//! 1. `cache->count > 3` — the C writes past `int saveA[3]` **and** past the
//!    4-slot `c2sv a,b,c,d` simplex, corrupting its own stack frame.
//! 2. an out-of-range `C2_TYPE` passed straight to `c2GJK` — `c2MakeProxy` writes
//!    nothing, so `c2Support(pA.verts, pA.count, d)` loops `count` times over an
//!    8-element array with an uninitialised `count`.
//!
//! What these tests assert is the only thing that can be asserted about UB: that
//! the Rust does not fail *differently* from the C — it must not turn the C's
//! survivable cases into crashes, nor the C's crashes into silent wrong answers
//! that a caller might trust.
//!
//! The corresponding DEFINED behaviour is asserted normally elsewhere:
//! `errors_phase_c.rs::err_row25_unchecked_cache_indices` (counts 1..=3) and
//! `level5_dispatch.rs` / `errors_phase_c.rs` (unknown type via the dispatchers).

mod common;
use common::*;
use std::ffi::c_int;

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Runs `f` in a forked child; returns the raw wait status.
fn probe<F: FnOnce()>(f: F) -> i32 {
    unsafe {
        let pid = fork();
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status = 0i32;
        waitpid(pid, &mut status, 0);
        status
    }
}

fn describe(status: i32) -> String {
    let sig = status & 0x7f;
    if sig != 0 {
        format!("killed by signal {sig}")
    } else {
        format!("exited with code {}", (status >> 8) & 0xff)
    }
}

#[test]
fn which_side_crashes_on_oversized_cache_count() {
    let (c, r) = apis();
    let mut rows = Vec::new();
    println!("cache->count | C result | Rust result");
    for count in [1i32, 2, 3, 4, 5, 6, 7] {
        let run = |api: &'static Api| {
            move || {
                let a = ShapeBuf::from_circle(c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 1.0,
                });
                let b = ShapeBuf::from_circle(c2Circle {
                    p: c2v { x: 3.0, y: 0.0 },
                    r: 1.0,
                });
                let mut cache = c2GJKCache {
                    metric: 0.0,
                    count,
                    iA: [0, 0, 0],
                    iB: [0, 0, 0],
                    div: 0.0,
                };
                let mut oa = c2v::default();
                let mut ob = c2v::default();
                let mut it: c_int = 0;
                unsafe {
                    (api.c2GJK)(
                        a.as_ptr(),
                        C2_TYPE_CIRCLE,
                        std::ptr::null(),
                        b.as_ptr(),
                        C2_TYPE_CIRCLE,
                        std::ptr::null(),
                        &mut oa,
                        &mut ob,
                        1,
                        &mut it,
                        &mut cache,
                    );
                }
            }
        };
        let cs = probe(run(c));
        let rs = probe(run(r));
        println!("{count:>12} | {} | {}", describe(cs), describe(rs));
        rows.push((count, cs, rs));
    }

    // The two builds must agree on WHICH counts are survivable, otherwise the
    // Rust would be trading the C's silent corruption for a hard crash (or vice
    // versa) at a different threshold.
    for (count, cs, rs) in &rows {
        let c_died = cs & 0x7f != 0;
        let r_died = rs & 0x7f != 0;
        assert_eq!(
            c_died, r_died,
            "cache->count = {count}: C {} but Rust {} — the crash thresholds must match",
            describe(*cs),
            describe(*rs)
        );
    }
    // Sanity: the counts the library can itself produce are all survivable.
    for (count, cs, _) in &rows {
        if *count <= 3 {
            assert_eq!(cs & 0x7f, 0, "cache->count = {count} must be survivable");
        }
    }
    println!(
        "Both builds survive cache->count <= {} and both SIGSEGV beyond it.",
        rows.iter()
            .filter(|(_, cs, _)| cs & 0x7f == 0)
            .map(|(n, _, _)| *n)
            .max()
            .unwrap()
    );
}

/// CONFIGS.md row 74 — an out-of-range `C2_TYPE` passed directly to `c2GJK`.
///
/// `c2MakeProxy` has no `default:` arm, so for an unknown type the C leaves its
/// `c2Proxy pA;` stack struct uninitialised and then calls
/// `c2Support(pA.verts, pA.count, d)` with a garbage `count`. Whether that
/// segfaults depends on the caller's leftover stack bytes, so the behaviour is
/// measured per-call in a forked child rather than asserted.
///
/// The Rust zero-initialises its proxy (`count == 0`), which makes it
/// deterministic: `c2Support` returns index 0 and the search terminates. This
/// test records how often each side survives, and requires that the Rust never
/// crash where the C survives — the translation must not be *less* robust than
/// the original on this path.
#[test]
fn invalid_type_straight_into_c2gjk() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0x74_74);
    let mut c_ok = 0usize;
    let mut r_ok = 0usize;
    let mut total = 0usize;
    let mut rust_crashed_where_c_survived = 0usize;

    for &bad in &BAD_TYPES {
        for &good in &VALID_TYPES {
            for ur in [0, 1] {
                let shape = rand_shape(&mut g, good);
                let bytes = shape.0;
                for (ta, tb) in [(bad, good), (good, bad), (bad, bad)] {
                    let run = |api: &'static Api| {
                        move || {
                            let mut a = ShapeBuf::zeroed();
                            let mut b = ShapeBuf::zeroed();
                            if ta == good {
                                a.0 = bytes;
                            }
                            if tb == good {
                                b.0 = bytes;
                            }
                            let mut oa = c2v::default();
                            let mut ob = c2v::default();
                            let mut it: c_int = 0;
                            unsafe {
                                (api.c2GJK)(
                                    a.as_ptr(),
                                    ta,
                                    std::ptr::null(),
                                    b.as_ptr(),
                                    tb,
                                    std::ptr::null(),
                                    &mut oa,
                                    &mut ob,
                                    ur,
                                    &mut it,
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                    };
                    let cs = probe(run(c));
                    let rs = probe(run(r));
                    let c_alive = cs & 0x7f == 0;
                    let r_alive = rs & 0x7f == 0;
                    total += 1;
                    if c_alive {
                        c_ok += 1;
                    }
                    if r_alive {
                        r_ok += 1;
                    }
                    if c_alive && !r_alive {
                        rust_crashed_where_c_survived += 1;
                        eprintln!(
                            "Rust crashed where C survived: typeA={ta} typeB={tb} ur={ur} \
                             (C {}, Rust {})",
                            describe(cs),
                            describe(rs)
                        );
                    }
                }
            }
        }
    }
    println!(
        "row74 invalid type into c2GJK: {total} configurations; \
         C survived {c_ok}, Rust survived {r_ok}"
    );
    assert_eq!(
        rust_crashed_where_c_survived, 0,
        "the Rust must never crash on an input the C survives"
    );
    assert_eq!(
        r_ok, total,
        "the Rust zero-initialises its proxy, so it should survive every case"
    );
}
