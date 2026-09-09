//! Phase C — remaining `sodium_misuse()` rows from ERRORS.md that need a
//! forked-child comparison: rows 12, 41 and 42.

mod common;
use common::*;
use std::os::raw::c_int;

type Sz = usize;

/// Row 41: `sodium_misuse()` itself. Both libraries must terminate with
/// SIGABRT, and both must run an installed misuse handler first.
#[test]
fn row41_sodium_misuse_and_handler() {
    let (mc, mr) = pair::<unsafe extern "C" fn()>("sodium_misuse");
    let oc = run_forked(|| unsafe { mc() });
    let or = run_forked(|| unsafe { mr() });
    assert_eq!(oc, or, "sodium_misuse termination differs");
    assert_eq!(oc, Outcome::Signaled(libc::SIGABRT), "sodium_misuse must abort");

    // With a handler installed, the handler runs and then abort() still fires.
    // Prove the handler ran by having it _exit(77) before abort() is reached.
    extern "C" fn handler() {
        unsafe { libc::_exit(77) };
    }
    let (sc, sr) =
        pair::<unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int>("sodium_set_misuse_handler");
    let oc = run_forked(|| unsafe {
        assert_eq!(sc(Some(handler)), 0);
        mc();
    });
    let or = run_forked(|| unsafe {
        assert_eq!(sr(Some(handler)), 0);
        mr();
    });
    assert_eq!(oc, or, "sodium_misuse handler behaviour differs");
    assert_eq!(oc, Outcome::Exited(77), "the installed handler must run");

    // Installing and clearing the handler must return 0 on both sides.
    unsafe {
        same_ret("set_misuse_handler(NULL)", sc(None), sr(None));
        same_ret("set_misuse_handler(fn)", sc(Some(handler)), sr(Some(handler)));
        same_ret("set_misuse_handler(NULL) again", sc(None), sr(None));
    }
}

/// Row 12: `sodium_pad` aborts when `SIZE_MAX - unpadded_buflen <= xpadlen`.
/// The check precedes every write, so the call is safe to make with a small
/// buffer inside a forked child.
#[test]
fn row12_sodium_pad_size_overflow_aborts() {
    let (pc, pr) = pair::<unsafe extern "C" fn(*mut Sz, *mut u8, Sz, Sz, Sz) -> c_int>("sodium_pad");
    // xpadlen = blocksize - 1 - (unpadded_buflen % blocksize); we need
    // SIZE_MAX - unpadded_buflen <= xpadlen, i.e. unpadded_buflen within
    // `xpadlen` of SIZE_MAX.
    let cases: &[(usize, usize)] = &[
        (usize::MAX, 2),
        (usize::MAX, 4),
        (usize::MAX, 16),
        (usize::MAX, 1024),
        (usize::MAX - 1, 4),
        (usize::MAX - 1, 16),
        (usize::MAX - 2, 16),
        (usize::MAX - 7, 16),
    ];
    let mut any_abort = false;
    for &(ul, bs) in cases {
        let pc2 = pc.clone();
        let pr2 = pr.clone();
        let oc = run_forked(|| {
            let mut b = vec![0u8; 64];
            let mut l: Sz = 0;
            unsafe { pc2(&mut l, b.as_mut_ptr(), ul, bs, usize::MAX) };
        });
        let or = run_forked(|| {
            let mut b = vec![0u8; 64];
            let mut l: Sz = 0;
            unsafe { pr2(&mut l, b.as_mut_ptr(), ul, bs, usize::MAX) };
        });
        assert_eq!(oc, or, "sodium_pad ul={ul} bs={bs}: termination differs");
        if oc == Outcome::Signaled(libc::SIGABRT) {
            any_abort = true;
        }
    }
    assert!(
        any_abort,
        "expected at least one sodium_pad size-overflow case to abort"
    );
}

/// Row 42: `randombytes_buf_deterministic` aborts for `size > 0x4000000000`.
/// The check precedes any write, so this is safe inside a forked child.
#[test]
fn row42_randombytes_buf_deterministic_oversize_aborts() {
    let (c, r) =
        pair::<unsafe extern "C" fn(*mut u8, Sz, *const u8)>("randombytes_buf_deterministic");
    for size in [
        0x4000_0000_01usize,
        0x4000_0000_0000,
        usize::MAX,
        usize::MAX / 2,
    ] {
        let c2 = c.clone();
        let r2 = r.clone();
        let oc = run_forked(|| {
            let seed = vec![7u8; 32];
            let mut b = vec![0u8; 64];
            unsafe { c2(b.as_mut_ptr(), size, seed.as_ptr()) };
        });
        let or = run_forked(|| {
            let seed = vec![7u8; 32];
            let mut b = vec![0u8; 64];
            unsafe { r2(b.as_mut_ptr(), size, seed.as_ptr()) };
        });
        assert_eq!(oc, or, "randombytes_buf_deterministic size={size}: termination differs");
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGABRT),
            "randombytes_buf_deterministic size={size} must abort"
        );
    }
    // exactly at the limit must NOT abort (but would need 256 GiB of output, so
    // only assert the boundary just below the misuse threshold is not rejected
    // for a small size)
    let seed = vec![7u8; 32];
    let mut a = buf(64);
    let mut b = buf(64);
    unsafe {
        c(a.as_mut_ptr(), 64, seed.as_ptr());
        r(b.as_mut_ptr(), 64, seed.as_ptr());
    }
    same_bytes("randombytes_buf_deterministic small", &a, &b);
}
