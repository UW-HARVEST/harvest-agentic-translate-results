//! Area 1 — `sodium/core.c`: init, the critical section, and the misuse handler.
//!
//! Separate binary: `sodium_set_misuse_handler` mutates process-global state.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

type Handler = unsafe extern "C" fn();
type SetHandler = unsafe extern "C" fn(Option<Handler>) -> c_int;
type IntFn = unsafe extern "C" fn() -> c_int;
type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

#[test]
fn sodium_init_returns_one_when_already_initialised() {
    let (c, r) = both::<IntFn>("sodium_init");
    unsafe {
        // The harness already called sodium_init() once on each library.
        eqi("sodium_init (2nd)", c(), r());
        assert_eq!(c(), 1);
        eqi("sodium_init (3rd)", c(), r());
    }
}

#[test]
fn crit_enter_leave_never_fail_in_this_build() {
    // No _WIN32 / HAVE_PTHREAD / HAVE_ATOMIC_OPS  =>  both are `return 0` stubs,
    // including leave-without-enter.
    for name in ["sodium_crit_enter", "sodium_crit_leave"] {
        if !has(name) {
            continue;
        }
        let (c, r) = both::<IntFn>(name);
        unsafe {
            for _ in 0..4 {
                eqi(name, c(), r());
                assert_eq!(c(), 0, "{name} should always succeed in this build");
            }
        }
    }
}

#[test]
fn set_misuse_handler_returns_zero() {
    let (c, r) = both::<SetHandler>("sodium_set_misuse_handler");
    unsafe {
        eqi("set_misuse_handler(NULL)", c(None), r(None));
        assert_eq!(c(None), 0);
    }
}

unsafe extern "C" fn handler_exit_77() {
    // `sodium_misuse()` calls the handler and then abort()s unconditionally, so
    // a handler that never returns is the only way to observe it.
    exit_now(77)
}

fn exit_now(code: c_int) -> ! {
    extern "C" {
        fn _exit(code: c_int) -> !;
    }
    unsafe { _exit(code) }
}

/// `sodium_misuse()` must invoke the installed handler *before* aborting, in
/// both implementations. Observed through a forked child's exit status.
#[test]
fn misuse_handler_is_invoked_before_abort() {
    let (sc, sr) = both::<SetHandler>("sodium_set_misuse_handler");
    let (mc, mr) = both::<unsafe extern "C" fn()>("sodium_misuse");
    let sc2 = sc.clone();
    let sr2 = sr.clone();
    let mc1 = mc.clone();
    let mr1 = mr.clone();
    eq_abort(
        "sodium_misuse with handler installed",
        move || unsafe {
            assert_eq!(sc2(Some(handler_exit_77)), 0);
            mc1();
        },
        move || unsafe {
            assert_eq!(sr2(Some(handler_exit_77)), 0);
            mr1();
        },
    );
    // ... and with no handler it must abort (SIGABRT), not exit.
    let mc2 = mc.clone();
    let mr2 = mr.clone();
    eq_abort(
        "sodium_misuse with no handler",
        move || unsafe {
            assert_eq!(sc(None), 0);
            mc2();
        },
        move || unsafe {
            assert_eq!(sr(None), 0);
            mr2();
        },
    );
}

/// The misuse handler must also be reached from a real misuse site, not just a
/// direct `sodium_misuse()` call.
#[test]
fn misuse_handler_reached_from_a_real_misuse_site() {
    let (sc, sr) = both::<SetHandler>("sodium_set_misuse_handler");
    let (hc, hr) = both::<Bin2Hex>("sodium_bin2hex");
    eq_abort(
        "sodium_bin2hex misuse routes through the handler",
        move || unsafe {
            assert_eq!(sc(Some(handler_exit_77)), 0);
            let bin = [1u8, 2, 3, 4];
            let mut out = [0u8; 4];
            // hex_maxlen <= bin_len * 2  =>  sodium_misuse()
            hc(out.as_mut_ptr() as *mut c_char, 4, bin.as_ptr(), 4);
        },
        move || unsafe {
            assert_eq!(sr(Some(handler_exit_77)), 0);
            let bin = [1u8, 2, 3, 4];
            let mut out = [0u8; 4];
            hr(out.as_mut_ptr() as *mut c_char, 4, bin.as_ptr(), 4);
        },
    );
}

/// `bin_len >= SIZE_MAX / 2` in `sodium_bin2hex` (errors row 1.40). The length
/// check happens before any dereference, so a null `bin` is safe here.
#[test]
fn bin2hex_bin_len_overflow_aborts() {
    let (c, r) = both::<Bin2Hex>("sodium_bin2hex");
    for bin_len in [usize::MAX / 2, usize::MAX / 2 + 1, usize::MAX] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("bin2hex bin_len={bin_len}"),
            move || unsafe {
                let mut out = [0u8; 64];
                cc(out.as_mut_ptr() as *mut c_char, usize::MAX, std::ptr::null(), bin_len);
            },
            move || unsafe {
                let mut out = [0u8; 64];
                rr(out.as_mut_ptr() as *mut c_char, usize::MAX, std::ptr::null(), bin_len);
            },
        );
    }
}
