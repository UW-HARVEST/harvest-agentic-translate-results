//! Phase B — CONFIGS.md rows 61..62: the `str_put` driver, whose observable
//! output is what it writes to `stdout`.

mod common;

use common::*;
use std::ffi::c_int;

fn run(c: &Api, r: &Api, num: c_int) {
    unsafe {
        // `str_put` builds a hash index, which consumes the process-wide
        // `stbds_hash_seed`; sync both libraries first so the runs are
        // comparable.
        (c.rand_seed)(0x31415926);
        (r.rand_seed)(0x31415926);
    }
    let oc = capture_stdout(&format!("c{num}"), || unsafe { (c.str_put)(num) });
    let or = capture_stdout(&format!("r{num}"), || unsafe { (r.str_put)(num) });
    same(&format!("str_put({num}) stdout"), &oc, &or);
    // guard against a silently empty capture: the C prints exactly one line
    assert_eq!(
        oc,
        format!("a {num}\n").into_bytes(),
        "str_put({num}) unexpected stdout"
    );
}

#[test]
fn row61_str_put_valid() {
    let (c, r, _g) = both();
    for num in [0, 1, 2, 3, 7, 8, 9, 15, 16, 64, 100, 511, 512, 513, 1000] {
        run(c, r, num);
    }
}

#[test]
fn row61_str_put_random() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 61);
    for _ in 0..200 {
        let num = rng.below(2000) as c_int;
        run(c, r, num);
    }
}

#[test]
fn row62_str_put_non_positive() {
    let (c, r, _g) = both();
    for num in [0, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        run(c, r, num);
    }
}

#[test]
fn row61_str_put_repeated_no_state_leak() {
    let (c, r, _g) = both();
    // `strkey` writes into a file-scope buffer and `stbds_hash_seed` is global;
    // repeated calls must stay in lockstep.
    for i in 0..50 {
        run(c, r, (i * 7) % 200);
    }
}
