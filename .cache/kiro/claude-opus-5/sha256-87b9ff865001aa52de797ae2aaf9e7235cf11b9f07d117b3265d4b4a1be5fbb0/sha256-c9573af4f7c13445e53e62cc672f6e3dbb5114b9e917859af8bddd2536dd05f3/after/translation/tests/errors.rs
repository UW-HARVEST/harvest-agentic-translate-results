//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no conditional
//! rejection at all (0 `if`, 0 `assert`, 0 error sentinels — its only exit is
//! an unconditional `return 0;`), so "the same error/rejection" here means
//! *the same non-error*: both sides must return the identical sentinel `0` and
//! emit the identical bytes for every abusive input, rather than one of them
//! trapping, aborting, or returning something else.
//!
//! Every call is made through `dlsym` on the respective `.so`.

mod common;

use common::*;
use std::ffi::{c_char, c_double, c_int, c_longlong, c_void};

/// Runs `body` against C and against Rust and asserts the returned values and
/// the captured stdout bytes are identical.
fn diff<T: std::fmt::Debug + PartialEq>(
    label: &str,
    mut body: impl FnMut(&Pair, Impl) -> T,
) -> (T, T) {
    let pair = load_pair();
    let (c_ret, c_out) = capture(Sink::File, Buffering::Default, || body(&pair, Impl::C));
    let (r_ret, r_out) = capture(Sink::File, Buffering::Default, || body(&pair, Impl::Rust));
    assert_eq!(
        c_ret, r_ret,
        "{label}: returned values differ (C={c_ret:?}, Rust={r_ret:?})"
    );
    assert_same_bytes(label, &c_out, &r_out);
    (c_ret, r_ret)
}

// ---------------------------------------------------------------------- E1
/// The only `return` in the library is `return 0;`, with no guard in front of
/// it. Assert that: across many calls, both implementations return exactly the
/// sentinel `0` — never a negative error code, never a varying value.
#[test]
fn errors_e1_return_is_unconditionally_zero() {
    let (c_rets, r_rets) = diff("E1", |p, w| {
        let f = p.helloworld(w);
        (0..256).map(|_| unsafe { f() }).collect::<Vec<c_int>>()
    });
    assert!(
        c_rets.iter().all(|&r| r == 0),
        "E1: C returned a non-zero value: {c_rets:?}"
    );
    assert!(
        r_rets.iter().all(|&r| r == 0),
        "E1: Rust returned a non-zero value: {r_rets:?}"
    );
    // And explicitly not the usual failure sentinels.
    assert!(!c_rets.contains(&-1) && !r_rets.contains(&-1));
}

// ---------------------------------------------------------------------- E2
/// Null pointer argument. `int helloworld();` is an unprototyped declaration,
/// so passing a pointer is something a real C caller can do; the callee names
/// no parameter and therefore never dereferences it. Both sides must ignore it
/// identically instead of faulting.
#[test]
fn errors_e2_null_pointer_argument() {
    diff("E2", |p, w| {
        let addr = p.helloworld_addr(w);
        unsafe {
            let f1: unsafe extern "C" fn(*const c_void) -> c_int = std::mem::transmute(addr);
            let f2: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int =
                std::mem::transmute(addr);
            let f3: unsafe extern "C" fn(*const c_char, *mut c_void, *const c_int) -> c_int =
                std::mem::transmute(addr);
            vec![
                f1(std::ptr::null()),
                f2(std::ptr::null(), std::ptr::null()),
                f3(std::ptr::null(), std::ptr::null_mut(), std::ptr::null()),
                // A deliberately wild, non-null, non-dereferenceable pointer:
                // proves neither side reads through it.
                f1(usize::MAX as *const c_void),
                f1(1usize as *const c_void),
            ]
        }
    });
}

// ---------------------------------------------------------------------- E3
/// Zero and oversized length arguments.
#[test]
fn errors_e3_zero_and_oversized_length_arguments() {
    diff("E3", |p, w| {
        let addr = p.helloworld_addr(w);
        unsafe {
            let fz: unsafe extern "C" fn(usize) -> c_int = std::mem::transmute(addr);
            let fp: unsafe extern "C" fn(*const c_void, usize) -> c_int =
                std::mem::transmute(addr);
            vec![
                fz(0),
                fz(1),
                fz(usize::MAX),
                fz(usize::MAX / 2),
                fz(i64::MAX as usize),
                fp(std::ptr::null(), 0),
                fp(std::ptr::null(), usize::MAX),
                // Negative "length" reinterpreted, as C's implicit int
                // conversions would deliver it.
                fz((-1i64) as usize),
            ]
        }
    });
}

// ---------------------------------------------------------------------- E4
/// Out-of-range enum values across the FFI boundary. A C `enum` parameter
/// accepts any `int`, so values with no valid variant are real inputs. This
/// library declares no enum and performs no `switch`, so every value must be
/// ignored identically — the point is to prove the Rust side does not, for
/// example, match on a value and panic on the fallthrough.
#[test]
fn errors_e4_out_of_range_enum_values() {
    // Exhaustive over the interesting neighbourhood, plus the extremes.
    let mut values: Vec<c_int> = vec![c_int::MIN, c_int::MIN + 1, -2, -1, 0, 1, 2, 3, 255, 256];
    values.extend([c_int::MAX - 1, c_int::MAX]);
    let mut rng = Rng::new(SEED ^ 0xE4);
    for _ in 0..32 {
        values.push(rng.next_u64() as c_int);
    }

    diff("E4", |p, w| {
        let addr = p.helloworld_addr(w);
        unsafe {
            let f1: unsafe extern "C" fn(c_int) -> c_int = std::mem::transmute(addr);
            let f2: unsafe extern "C" fn(c_int, c_int) -> c_int = std::mem::transmute(addr);
            values
                .iter()
                .map(|&v| f1(v) | f2(v, v.wrapping_neg()))
                .collect::<Vec<c_int>>()
        }
    });
}

// ---------------------------------------------------------------------- E5
/// One step past any "documented valid range": the extremes of every scalar
/// register class.
#[test]
fn errors_e5_scalar_extremes_one_past_range() {
    diff("E5", |p, w| {
        let addr = p.helloworld_addr(w);
        unsafe {
            let fi: unsafe extern "C" fn(c_int) -> c_int = std::mem::transmute(addr);
            let fl: unsafe extern "C" fn(c_longlong) -> c_int = std::mem::transmute(addr);
            let fd: unsafe extern "C" fn(c_double) -> c_int = std::mem::transmute(addr);
            let fu: unsafe extern "C" fn(u64) -> c_int = std::mem::transmute(addr);
            vec![
                fi(c_int::MIN),
                fi(c_int::MAX),
                fl(c_longlong::MIN),
                fl(c_longlong::MAX),
                fu(0),
                fu(u64::MAX),
                fd(f64::NAN),
                fd(f64::INFINITY),
                fd(f64::NEG_INFINITY),
                fd(-0.0),
                fd(f64::MIN),
                fd(f64::MAX),
                fd(f64::MIN_POSITIVE),
            ]
        }
    });
}

// ---------------------------------------------------------------------- E6
/// More arguments than the definition accepts — six integer registers plus a
/// stack spill, and a mixed integer/SSE spill. The SysV AMD64 callee must
/// ignore all of them and leave the caller's stack intact (if it did not, the
/// `Vec` built afterwards would be corrupted and the test would crash).
#[test]
fn errors_e6_excess_arguments_stack_spill() {
    let mut rng = Rng::new(SEED ^ 0xE6);
    let a: Vec<c_longlong> = (0..12).map(|_| rng.next_u64() as c_longlong).collect();

    diff("E6", |p, w| {
        let addr = p.helloworld_addr(w);
        unsafe {
            #[allow(clippy::type_complexity)]
            let f10: unsafe extern "C" fn(
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
                c_longlong,
            ) -> c_int = std::mem::transmute(addr);
            #[allow(clippy::type_complexity)]
            let fmix: unsafe extern "C" fn(
                c_int,
                c_double,
                c_int,
                c_double,
                c_int,
                c_double,
                c_int,
                c_double,
                c_int,
                c_double,
                c_int,
                c_double,
            ) -> c_int = std::mem::transmute(addr);
            let r1 = f10(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9]);
            let r2 = fmix(
                1, 1.5, 2, 2.5, 3, 3.5, 4, 4.5, 5, 5.5, 6, 6.5,
            );
            // Touch the values again afterwards: if the callee had clobbered
            // the caller's frame this would observably differ.
            let checksum: c_longlong = a.iter().fold(0, |acc, &x| acc ^ x);
            vec![r1 as c_longlong, r2 as c_longlong, checksum]
        }
    });
}

// ---------------------------------------------------------------------- E7
/// `printf` fails because `stdout` is not writable. The C discards `printf`'s
/// return value, so the failure is swallowed: `helloworld` still returns `0`.
/// A translation that propagated the I/O error (or panicked) would diverge
/// here — this is the one genuine failure mode inside the function body.
#[test]
fn errors_e7_stdout_write_failure_is_swallowed() {
    for mode in [BrokenStdout::Unwritable, BrokenStdout::Closed] {
        let pair = load_pair();
        let (c_ret, c_errored) = with_broken_stdout(mode, || {
            let f = pair.helloworld(Impl::C);
            (0..4).map(|_| unsafe { f() }).collect::<Vec<c_int>>()
        });
        let (r_ret, r_errored) = with_broken_stdout(mode, || {
            let f = pair.helloworld(Impl::Rust);
            (0..4).map(|_| unsafe { f() }).collect::<Vec<c_int>>()
        });
        // Non-vacuity: the write really must have failed, otherwise this test
        // would be asserting nothing.
        assert!(
            c_errored,
            "E7[{mode:?}]: stdout did not actually fail for the C side — the test would be vacuous"
        );
        assert_eq!(
            c_errored, r_errored,
            "E7[{mode:?}]: one implementation hit a stream error and the other did not \
             (C={c_errored}, Rust={r_errored})"
        );
        assert_eq!(
            c_ret, r_ret,
            "E7[{mode:?}]: return values differ with an unwritable stdout \
             (C={c_ret:?}, Rust={r_ret:?})"
        );
        assert!(
            c_ret.iter().all(|&r| r == 0),
            "E7[{mode:?}]: the C swallows printf failure and must still return 0, got {c_ret:?}"
        );
        assert!(
            r_ret.iter().all(|&r| r == 0),
            "E7[{mode:?}]: Rust must swallow the failure identically, got {r_ret:?}"
        );
    }

    // The library must still work normally once stdout is healthy again — i.e.
    // neither implementation left sticky state behind.
    let pair = load_pair();
    for which in [Impl::C, Impl::Rust] {
        let (ret, out) = capture(Sink::File, Buffering::Default, || unsafe {
            pair.helloworld(which)()
        });
        assert_eq!(ret, 0);
        assert_eq!(
            out,
            EXPECTED_LINE,
            "E7/{}: output broken after a failed write: {:?}",
            which.name(),
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------- E8
/// Concurrent invocation. There is no lock or guard in the C, so there is no
/// rejection: every call must return `0` on both sides and no line may be torn.
#[test]
fn errors_e8_concurrent_calls() {
    let mut rng = Rng::new(SEED ^ 0xE8);
    let threads = rng.range(4, 8) as usize;
    let per = rng.range(8, 32) as usize;
    let total = threads * per;

    let mut results = Vec::new();
    for which in [Impl::C, Impl::Rust] {
        let pair = load_pair();
        let addr = pair.helloworld_addr(which) as usize;
        let (mut rets, out) = capture(Sink::File, Buffering::Default, || {
            let hs: Vec<_> = (0..threads)
                .map(|_| {
                    std::thread::spawn(move || {
                        let f: unsafe extern "C" fn() -> c_int =
                            unsafe { std::mem::transmute(addr) };
                        (0..per).map(|_| unsafe { f() }).collect::<Vec<c_int>>()
                    })
                })
                .collect();
            hs.into_iter()
                .flat_map(|h| h.join().expect("worker thread"))
                .collect::<Vec<c_int>>()
        });
        rets.sort_unstable();
        assert_eq!(rets.len(), total);
        assert!(
            rets.iter().all(|&r| r == 0),
            "E8/{}: concurrent calls must all return 0",
            which.name()
        );
        let lines: Vec<&[u8]> = out.split_inclusive(|&b| b == b'\n').collect();
        assert_eq!(
            lines.len(),
            total,
            "E8/{}: expected {total} intact lines, got {:?}",
            which.name(),
            show(&out)
        );
        assert!(
            lines.iter().all(|l| *l == EXPECTED_LINE),
            "E8/{}: a line was torn under concurrency",
            which.name()
        );
        results.push((rets, out.len()));
    }
    assert_eq!(results[0], results[1], "E8: C and Rust differ under concurrency");
}

// ------------------------------------------------- generic boundary sweep
/// A randomized sweep over the whole abusive-call space at once, so an
/// accidental divergence that only shows up in a particular argument
/// combination still gets caught.
#[test]
fn errors_randomized_abusive_call_sweep() {
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    for iter in 0..60 {
        let shape = rng.range(0, 6);
        let v0 = rng.next_u64();
        let v1 = rng.next_u64();
        let v2 = rng.next_u64();
        diff(&format!("sweep[iter={iter},shape={shape}]"), |p, w| {
            let addr = p.helloworld_addr(w);
            unsafe {
                match shape {
                    0 => {
                        let f: unsafe extern "C" fn() -> c_int = std::mem::transmute(addr);
                        f()
                    }
                    1 => {
                        let f: unsafe extern "C" fn(u64) -> c_int = std::mem::transmute(addr);
                        f(v0)
                    }
                    2 => {
                        let f: unsafe extern "C" fn(*const c_void, usize) -> c_int =
                            std::mem::transmute(addr);
                        f(v0 as *const c_void, v1 as usize)
                    }
                    3 => {
                        let f: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int =
                            std::mem::transmute(addr);
                        f(v0 as c_int, v1 as c_int, v2 as c_int, 0)
                    }
                    4 => {
                        let f: unsafe extern "C" fn(c_double, u64) -> c_int =
                            std::mem::transmute(addr);
                        f(f64::from_bits(v0), v1)
                    }
                    5 => {
                        let f: unsafe extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> c_int =
                            std::mem::transmute(addr);
                        f(v0, v1, v2, v0, v1, v2, v0, v1)
                    }
                    _ => {
                        // Called with a *wrong return type* — the caller reads
                        // only the low 32 bits of rax in C, so a wider read is
                        // the caller's problem, but the low half must agree.
                        let f: unsafe extern "C" fn(u64) -> u32 = std::mem::transmute(addr);
                        f(v0) as c_int
                    }
                }
            }
        });
    }
}
