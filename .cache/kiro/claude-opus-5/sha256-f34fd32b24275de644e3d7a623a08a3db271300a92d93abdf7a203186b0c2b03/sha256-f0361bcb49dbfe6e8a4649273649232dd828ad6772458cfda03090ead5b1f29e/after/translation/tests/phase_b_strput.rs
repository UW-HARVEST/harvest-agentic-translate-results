//! Phase B — `CONFIGS.md` rows 39..43: the driver entry points `str_put`
//! (whose only observable output is what it `printf`s) and `strkey`.
//!
//! The cmake project builds a shared library only — there is no separate
//! executable — so "compare the program's stdout byte-for-byte" is done by
//! redirecting fd 1 around the call into each `.so`.

mod common;

use common::*;
use std::ffi::{c_int, CStr};

fn str_put_stdout(num: c_int) -> (Vec<u8>, Vec<u8>) {
    let (lc, lr) = libs();
    // Both libraries own a private copy of `stbds_hash_seed`; pin them so the
    // single hash index `str_put` builds is seeded identically.
    reseed(DEFAULT_SEED);
    let c = capture_stdout("c", || unsafe { (lc.str_put)(num) });
    reseed(DEFAULT_SEED);
    let r = capture_stdout("r", || unsafe { (lr.str_put)(num) });
    (c, r)
}

fn check(num: c_int) {
    let (c, r) = str_put_stdout(num);
    assert_eq!(
        c,
        r,
        "str_put({num}) stdout diverged\n  C   : {:?}\n  RUST: {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // sanity: the C really did print something of the documented shape
    let want = format!("a {num}\n");
    assert_eq!(
        String::from_utf8_lossy(&c),
        want,
        "unexpected C output for str_put({num})"
    );
}

/// row 39 — `num = 0`
#[test]
fn row39_str_put_zero() {
    let _g = serial();
    check(0);
}

/// row 40 — small positive `num`
#[test]
fn row40_str_put_small() {
    let _g = serial();
    for num in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 63, 64, 65, 100, 127, 128, 255, 256] {
        check(num);
    }
}

/// row 41 — large `num`, so the arena walks through several block sizes
#[test]
fn row41_str_put_large() {
    let _g = serial();
    for num in [1000, 5000] {
        check(num);
    }
}

/// row 42 — negative `num`: the `stralloc` loop body never runs
#[test]
fn row42_str_put_negative() {
    let _g = serial();
    for num in [-1, -2, -1000, i32::MIN] {
        check(num);
    }
}

/// row 42b — randomized `num` values
#[test]
fn row42b_str_put_random() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0042);
    for _ in 0..60 {
        let num = (rng.next_u32() % 400) as c_int - 50;
        check(num);
    }
}

/// row 43 — `strkey` over interesting `int` values
#[test]
fn row43_strkey() {
    let _g = serial();
    let (lc, lr) = libs();
    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        99,
        100,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    let mut rng = Rng::new(0xB2_0043);
    for _ in 0..500 {
        cases.push(rng.next_u32() as c_int);
    }
    for n in cases {
        unsafe {
            let c = CStr::from_ptr((lc.strkey)(n)).to_bytes().to_vec();
            let r = CStr::from_ptr((lr.strkey)(n)).to_bytes().to_vec();
            assert_eq!(
                c,
                r,
                "strkey({n}) diverged: C={:?} RUST={:?}",
                String::from_utf8_lossy(&c),
                String::from_utf8_lossy(&r)
            );
            assert_eq!(c, format!("test_{n}").into_bytes());
        }
    }
}

/// The static buffer `strkey` writes into must be reused (not reallocated), and
/// `str_put` must keep working after arbitrary `strkey` traffic.
#[test]
fn row43b_strkey_buffer_is_static() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        let p1 = (lc.strkey)(1);
        let p2 = (lc.strkey)(2);
        assert_eq!(p1, p2, "C strkey must return the same static buffer");
        let q1 = (lr.strkey)(1);
        let q2 = (lr.strkey)(2);
        assert_eq!(q1, q2, "Rust strkey must return the same static buffer");
    }
    check(11);
}
