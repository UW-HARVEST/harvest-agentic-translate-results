//! Phase D — thread-local `errno` and non-default rounding modes.
//!
//! The whole control flow of `my_pow` hangs off `errno`, which is *thread
//! local* in C (`errno` is `(*__errno_location())`). A port that cached the
//! location, or used a plain global, passes every single-threaded test and
//! fails here. Rounding mode is FPU state that shifts the overflow/underflow
//! thresholds `pow` reports through `errno`, so it is a genuine configuration
//! axis for the composed operation even though the C source never touches it.
//!
//! Own test binary so the global fd-2 redirection and the FPU control word
//! cannot perturb the other suites.

mod common;

use std::ffi::{c_int, c_void};

use common::{c_pow, rust_pow, PowFn, Rng};

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const i8, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
    fn fegetround() -> c_int;
    fn fesetround(mode: c_int) -> c_int;
}

const O_WRONLY: c_int = 1;

/// Silences fd 2 for the whole test binary; installed once, never restored.
fn silence_stderr_forever() {
    unsafe {
        fflush(std::ptr::null_mut());
        let devnull = open(c"/dev/null".as_ptr(), O_WRONLY);
        assert!(devnull >= 0, "open /dev/null failed");
        assert!(dup2(devnull, 2) >= 0, "dup2 onto fd 2 failed");
        if devnull != 2 {
            close(devnull);
        }
    }
}

#[inline]
fn call_quiet(f: PowFn, base: f64, exponent: f64) -> (u64, c_int) {
    unsafe {
        *__errno_location() = 0;
        let v = f(base, exponent);
        (v.to_bits(), *__errno_location())
    }
}

/// Inputs spanning no-error / EDOM / ERANGE, generated deterministically.
fn gen_pair(rng: &mut Rng) -> (f64, f64) {
    match rng.next_u64() % 6 {
        0 => (rng.range(0.0, 10.0), rng.range(-10.0, 10.0)),
        1 => (rng.range(-10.0, 0.0), rng.range(-10.0, 10.0)), // heavy EDOM
        2 => (rng.range(1.5, 8.0), rng.int(-2000, 2000) as f64), // overflow/underflow
        3 => (
            if rng.next_u64() & 1 == 0 { 0.0 } else { -0.0 },
            -(rng.int(1, 50) as f64),
        ), // pole
        4 => (rng.raw_f64(), rng.raw_f64()),
        _ => (
            common::SPECIALS[(rng.next_u64() as usize) % common::SPECIALS.len()],
            common::SPECIALS[(rng.next_u64() as usize) % common::SPECIALS.len()],
        ),
    }
}

#[test]
fn d10_concurrent_calls_from_many_threads() {
    silence_stderr_forever();
    let c = c_pow();
    let r = rust_pow();

    // Threads deliberately drive *different* error branches at the same time:
    // if `errno` were not per-thread, one thread's EDOM would be observed by
    // another and the returned value would flip to -1.0.
    let mut handles = Vec::new();
    for t in 0..8u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0xD10_0000 + t * 7919);
            for i in 0..20_000 {
                let (b, e) = gen_pair(&mut rng);
                let (cv, ce) = call_quiet(c, b, e);
                let (rv, re) = call_quiet(r, b, e);
                assert!(
                    cv == rv && ce == re,
                    "[D10 thread {t}] divergence at iteration {i}: \
                     my_pow({b:?} /*{:#018x}*/, {e:?} /*{:#018x}*/) -> \
                     C: bits={cv:#018x} errno={ce}, Rust: bits={rv:#018x} errno={re}",
                    b.to_bits(),
                    e.to_bits(),
                );
            }
        }));
    }
    for h in handles {
        h.join().expect("a worker thread panicked");
    }
}

#[test]
fn d11_errno_is_read_from_the_calling_thread() {
    silence_stderr_forever();
    let c = c_pow();
    let r = rust_pow();

    // Park a thread with errno permanently set to EDOM, then make successful
    // calls on this thread. If either library read a non-thread-local errno the
    // successful calls would report a domain error and return -1.0.
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let poisoner = std::thread::spawn(move || {
        unsafe { *__errno_location() = common::EDOM };
        done_tx.send(()).unwrap();
        rx.recv().ok(); // stay alive, errno still EDOM
    });
    done_rx.recv().unwrap();

    for (b, e, want) in [
        (2.0f64, 3.0f64, 8.0f64),
        (9.0, 0.5, 3.0),
        (-2.0, 4.0, 16.0),
        (10.0, 0.0, 1.0),
    ] {
        let (cv, ce) = call_quiet(c, b, e);
        let (rv, re) = call_quiet(r, b, e);
        assert_eq!(cv, rv, "[D11] bits differ");
        assert_eq!(ce, re, "[D11] errno differs");
        assert_eq!(
            f64::from_bits(cv),
            want,
            "[D11] my_pow({b:?}, {e:?}) should be {want:?}"
        );
    }

    tx.send(()).ok();
    poisoner.join().unwrap();
}

#[test]
fn d12_non_default_rounding_modes() {
    silence_stderr_forever();
    let c = c_pow();
    let r = rust_pow();

    // FE_TONEAREST, FE_DOWNWARD, FE_UPWARD, FE_TOWARDZERO on x86-64.
    let modes: [(&str, c_int); 4] = [
        ("FE_TONEAREST", 0x000),
        ("FE_DOWNWARD", 0x400),
        ("FE_UPWARD", 0x800),
        ("FE_TOWARDZERO", 0xC00),
    ];
    let original = unsafe { fegetround() };

    for (name, mode) in modes {
        if unsafe { fesetround(mode) } != 0 {
            // Mode unsupported on this target — skip rather than fail.
            continue;
        }
        assert_eq!(unsafe { fegetround() }, mode, "fesetround({name}) did not stick");

        let mut rng = Rng::new(0xD12_5EED);
        for i in 0..40_000 {
            let (b, e) = gen_pair(&mut rng);
            let (cv, ce) = call_quiet(c, b, e);
            let (rv, re) = call_quiet(r, b, e);
            assert!(
                cv == rv && ce == re,
                "[D12 {name}] divergence at iteration {i}: \
                 my_pow({b:?} /*{:#018x}*/, {e:?} /*{:#018x}*/) -> \
                 C: bits={cv:#018x} errno={ce}, Rust: bits={rv:#018x} errno={re}",
                b.to_bits(),
                e.to_bits(),
            );
        }
    }

    unsafe { fesetround(original) };
}
