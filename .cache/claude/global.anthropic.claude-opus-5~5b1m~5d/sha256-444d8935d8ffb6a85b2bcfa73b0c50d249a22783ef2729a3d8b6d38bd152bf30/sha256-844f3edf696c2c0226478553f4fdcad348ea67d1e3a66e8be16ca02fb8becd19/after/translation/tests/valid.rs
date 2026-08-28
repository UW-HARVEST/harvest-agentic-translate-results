//! Phase B — valid-path differential tests, one test function per row group of
//! `CONFIGS.md`.
//!
//! Every test drives **both** shared objects through `dlsym`'d exports and
//! compares the return value *and* the captured stdout/stderr bytes. The
//! `(OP, REPEAT)` axis is supplied by the environment
//! (`MD_OP`/`MD_REPEAT`/`MD_C_SO`/`MD_RUST_SO`), and `scripts/run_all.sh` runs
//! this whole binary once for each of the 24 configurations, so each row is
//! covered for every configuration.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// C01–C04: `op_add` — zero/±1, small random, boundary wrap, full-range random.
#[test]
fn c01_c04_op_add() {
    let p = Pair::load();
    for &(a, b) in BOUNDARY_PAIRS {
        diff_bin(&p, "op_add", |d| d.op_add(), a, b);
    }
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..256 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_bin(&p, "op_add", |d| d.op_add(), a, b);
    }
    for _ in 0..1024 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_bin(&p, "op_add", |d| d.op_add(), a, b);
    }
}

/// C05–C08: `op_sub`.
#[test]
fn c05_c08_op_sub() {
    let p = Pair::load();
    for &(a, b) in BOUNDARY_PAIRS {
        diff_bin(&p, "op_sub", |d| d.op_sub(), a, b);
    }
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..256 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_bin(&p, "op_sub", |d| d.op_sub(), a, b);
    }
    for _ in 0..1024 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_bin(&p, "op_sub", |d| d.op_sub(), a, b);
    }
}

/// C09–C12: `op_mul`.
#[test]
fn c09_c12_op_mul() {
    let p = Pair::load();
    for &(a, b) in BOUNDARY_PAIRS {
        diff_bin(&p, "op_mul", |d| d.op_mul(), a, b);
    }
    // (0,x), (1,x), (-1,x) shapes
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..64 {
        let x = rng.next_i32();
        for a in [0, 1, -1, 2, -2] {
            diff_bin(&p, "op_mul", |d| d.op_mul(), a, x);
            diff_bin(&p, "op_mul", |d| d.op_mul(), x, a);
        }
    }
    for _ in 0..256 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_bin(&p, "op_mul", |d| d.op_mul(), a, b);
    }
    for _ in 0..1024 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_bin(&p, "op_mul", |d| d.op_mul(), a, b);
    }
}

/// C13–C15: `helper_ptr` (indirect call through a local fn pointer + `printf`).
#[test]
fn c13_c15_helper_ptr() {
    let p = Pair::load();
    for &(a, b) in BOUNDARY_PAIRS {
        diff_bin(&p, "helper_ptr", |d| d.helper_ptr(), a, b);
    }
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..256 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_bin(&p, "helper_ptr", |d| d.helper_ptr(), a, b);
    }
    for _ in 0..512 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_bin(&p, "helper_ptr", |d| d.helper_ptr(), a, b);
    }
}

/// C16: `helper_ptr` must keep using the *build-time* op even after a caller
/// stores a different op into the exported `G_OP` global (the C `fp` is
/// macro-expanded, not read from the global).
#[test]
fn c16_helper_ptr_ignores_g_op() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();
    let mut rng = Rng::new(SEED ^ 5);
    for k in 0..3 {
        p.c.set_g_op(p.c.op_addresses()[k]);
        p.rs.set_g_op(p.rs.op_addresses()[k]);
        for &(a, b) in BOUNDARY_PAIRS {
            diff_bin(&p, "helper_ptr/G_OP", |d| d.helper_ptr(), a, b);
            diff_bin(&p, "helper_call/G_OP", |d| d.helper_call(), a, b);
        }
        for _ in 0..128 {
            let (a, b) = (rng.next_i32(), rng.next_i32());
            diff_bin(&p, "helper_ptr/G_OP", |d| d.helper_ptr(), a, b);
            diff_bin(&p, "helper_call/G_OP", |d| d.helper_call(), a, b);
        }
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// C17–C25: `helper_call` — the op plus the statically unrolled `REP<REPEAT>`
/// accumulator, for whichever `REPEAT` this run was built with.
#[test]
fn c17_c25_helper_call() {
    let p = Pair::load();
    for &(a, b) in BOUNDARY_PAIRS {
        diff_bin(&p, "helper_call", |d| d.helper_call(), a, b);
    }
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..256 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_bin(&p, "helper_call", |d| d.helper_call(), a, b);
    }
    for _ in 0..512 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_bin(&p, "helper_call", |d| d.helper_call(), a, b);
    }
    // Independent cross-check that the *C* accumulator really is REP<REPEAT>.
    let (rc, _) = capture(|| unsafe { (p.c.helper_call())(0, 0) });
    assert_eq!(rc, expected_run_loop(&p.op, p.repeat));
}

/// C26–C30: `use_generated` — every `case` arm of `DISPATCH_REP` and a sweep
/// across the arm boundaries, plus randomized `n`.
#[test]
fn c26_c30_use_generated() {
    let p = Pair::load();
    for n in -8..=16 {
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..256 {
        let n = rng.in_range(-3, 9); // dense around the valid 0..=6 window
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
    for _ in 0..256 {
        let n = rng.next_i32();
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
}

/// C31–C32: the `G_OP` data export — read the build-time value, then write all
/// three ops into it and observe the value through `main`.
#[test]
fn c31_c32_g_op_read_write() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();

    // read: both libraries must point at the same *named* op
    assert_eq!(
        p.c.classify_op(p.c.g_op_value()),
        p.rs.classify_op(p.rs.g_op_value()),
        "G_OP points at different ops [OP={} REPEAT={}]",
        p.op,
        p.repeat
    );
    assert_eq!(p.c.classify_op(p.c.g_op_value()), Some(p.op.as_str()));

    // write: the object must be writable (C keeps it in `.data`), the write must
    // be observable, and `main` must dispatch through it
    for k in 0..3 {
        p.c.set_g_op(p.c.op_addresses()[k]);
        p.rs.set_g_op(p.rs.op_addresses()[k]);
        assert_eq!(
            p.c.classify_op(p.c.g_op_value()),
            p.rs.classify_op(p.rs.g_op_value()),
            "G_OP readback differs after write"
        );
        diff_main_strs(&p, &["prog", "6", "7"]);
        diff_main_strs(&p, &["prog", "-2147483648", "-1"]);
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// C33–C34: the `G_OP_NAME` data export — the string must match byte for byte,
/// and the (writable) pointer must be repointable, which `main` then prints.
#[test]
fn c33_c34_g_op_name_read_write() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();

    let c_name = unsafe { std::ffi::CStr::from_ptr(p.c.g_op_name_value()) };
    let r_name = unsafe { std::ffi::CStr::from_ptr(p.rs.g_op_name_value()) };
    assert_eq!(
        c_name.to_bytes_with_nul(),
        r_name.to_bytes_with_nul(),
        "G_OP_NAME string differs [OP={}]",
        p.op
    );
    assert_eq!(c_name.to_str().unwrap(), p.op);

    for repl in ["", "x", "a-much-longer-op-name", "add", "sub", "mul"] {
        let s = CString::new(repl).unwrap();
        p.c.set_g_op_name(s.as_ptr() as *const c_char);
        p.rs.set_g_op_name(s.as_ptr() as *const c_char);
        diff_main_strs(&p, &["prog", "11", "-4"]);
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// C35–C39: `main` on the happy path — decimal text shapes, boundary text,
/// randomized operand pairs, and extra (ignored) arguments.
#[test]
fn c35_c39_main_valid() {
    let p = Pair::load();
    diff_main_strs(&p, &["prog", "3", "4"]);
    for pair in [
        ["0", "0"],
        ["0", "-0"],
        ["-7", "9"],
        ["+7", "-9"],
        ["  12", "  -12"],
        ["2147483647", "1"],
        ["-2147483648", "-1"],
        ["2147483647", "2147483647"],
        ["-2147483648", "-2147483648"],
        ["1000000", "1000000"],
        ["65536", "65536"],
    ] {
        diff_main_strs(&p, &["prog", pair[0], pair[1]]);
    }
    // argc > 3: the extra arguments are ignored
    diff_main_strs(&p, &["prog", "5", "6", "ignored"]);
    diff_main_strs(&p, &["prog", "5", "6", "ignored", "also-ignored"]);

    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..64 {
        let (a, b) = (rng.small_i32(), rng.small_i32());
        diff_main_strs(&p, &["prog", &a.to_string(), &b.to_string()]);
    }
    for _ in 0..64 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        diff_main_strs(&p, &["prog", &a.to_string(), &b.to_string()]);
    }
}

/// C40–C41: `main` observing caller-mutated globals — only `g.call`/`summary`
/// may change when `G_OP` is replaced, only `op=` when `G_OP_NAME` is replaced.
#[test]
fn c40_c41_main_with_mutated_globals() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();
    let names = ["add", "sub", "mul"];
    let mut rng = Rng::new(SEED ^ 9);
    for k in 0..3 {
        p.c.set_g_op(p.c.op_addresses()[k]);
        p.rs.set_g_op(p.rs.op_addresses()[k]);
        let s = CString::new(format!("via-{}", names[k])).unwrap();
        p.c.set_g_op_name(s.as_ptr() as *const c_char);
        p.rs.set_g_op_name(s.as_ptr() as *const c_char);
        diff_main_strs(&p, &["prog", "3", "4"]);
        for _ in 0..16 {
            let (a, b) = (rng.next_i32(), rng.next_i32());
            diff_main_strs(&p, &["prog", &a.to_string(), &b.to_string()]);
        }
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// C42: the composed pipeline — all entry points called in sequence inside one
/// capture, so the *interleaved* byte stream of the whole run is compared (this
/// catches ordering/buffering differences that per-call tests cannot see).
#[test]
fn c42_pipeline() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();

    fn run(d: &Driver, ops: &[(c_int, c_int)], ns: &[c_int], argv: &[&str]) -> c_int {
        let mut acc: c_int = 0;
        unsafe {
            for &(a, b) in ops {
                acc = acc.wrapping_add((d.op_add())(a, b));
                acc = acc.wrapping_add((d.op_sub())(a, b));
                acc = acc.wrapping_add((d.op_mul())(a, b));
                acc = acc.wrapping_add((d.helper_call())(a, b));
                acc = acc.wrapping_add((d.helper_ptr())(a, b));
            }
            for &n in ns {
                acc = acc.wrapping_add((d.use_generated())(n));
            }
            let items: Vec<Option<&str>> = argv.iter().map(|s| Some(*s)).collect();
            let mut av = Argv::new(&items);
            acc = acc.wrapping_add((d.main_fn())(argv.len() as c_int, av.as_ptr()));
        }
        acc
    }

    let mut rng = Rng::new(SEED ^ 10);
    for round in 0..8 {
        let ops: Vec<(c_int, c_int)> = (0..6)
            .map(|_| {
                if round % 2 == 0 {
                    (rng.small_i32(), rng.small_i32())
                } else {
                    (rng.next_i32(), rng.next_i32())
                }
            })
            .collect();
        let ns: Vec<c_int> = (0..8).map(|_| rng.in_range(-2, 9)).collect();
        let a = rng.small_i32().to_string();
        let b = rng.small_i32().to_string();
        let argv = ["prog", a.as_str(), b.as_str()];

        // also vary the mutable global state across rounds
        let k = (round % 4) as usize;
        if k < 3 {
            p.c.set_g_op(p.c.op_addresses()[k]);
            p.rs.set_g_op(p.rs.op_addresses()[k]);
        } else {
            p.c.set_g_op(saved_c.0);
            p.rs.set_g_op(saved_r.0);
        }

        let (rc, cap) = capture(|| run(&p.c, &ops, &ns, &argv));
        let (rr, rap) = capture(|| run(&p.rs, &ops, &ns, &argv));
        assert_eq!(
            rc, rr,
            "pipeline round {round} accumulated-return mismatch [OP={} REPEAT={}]",
            p.op, p.repeat
        );
        assert_eq!(
            cap, rap,
            "pipeline round {round} interleaved-output mismatch [OP={} REPEAT={}]",
            p.op, p.repeat
        );
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}
