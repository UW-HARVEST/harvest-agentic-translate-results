//! Phase C — one differential test per row of `ERRORS.md`.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/* ---- rows 1-3: the only runtime input validation in the C, `argc < 3` ---- */

/// Row 1 — `main` with no arguments at all: usage on stderr, status 2.
#[test]
fn err_01_main_argc_zero_args() {
    let c = run_driver(&c_exe_path(), &[]);
    let r = run_driver(&rust_exe_path(), &[]);
    assert_eq!(c.status, Some(2), "C must exit 2");
    assert_eq!(r.status, Some(2), "Rust must exit 2");
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&c.stderr), "usage: ./driver A B\n");
    assert!(c.stdout.is_empty() && r.stdout.is_empty());
    assert_driver_eq(&[]);
}

/// Row 2 — `main` with exactly one argument (`argc == 2`).
#[test]
fn err_02_main_argc_one_arg() {
    for arg in ["5", "-1", "", "abc"] {
        let c = run_driver(&c_exe_path(), &[arg]);
        let r = run_driver(&rust_exe_path(), &[arg]);
        assert_eq!(c.status, Some(2));
        assert_eq!(r.status, Some(2));
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr)
        );
        assert_driver_eq(&[arg]);
    }
}

/// Row 3 — the `argc >= 3` boundary: extra arguments are accepted, not rejected.
#[test]
fn err_03_main_extra_args_not_rejected() {
    let c = run_driver(&c_exe_path(), &["2", "3", "ignored"]);
    let r = run_driver(&rust_exe_path(), &["2", "3", "ignored"]);
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
    assert_driver_eq(&["2", "3", "ignored"]);
    assert_driver_eq(&["2", "3", "a", "b", "c", "d", "e"]);
}

/* ------- rows 4-10: DISPATCH_REP's `default: break` silent rejection ------ */

fn assert_dispatch_default(n: c_int) {
    // Both must return INIT_FOR(OP) and print `gen.acc=<INIT>`.
    let (cf, rf) = sym1("use_generated");
    let (cr, cout) = capture(|| unsafe { cf(n) });
    let (rr, rout) = capture(|| unsafe { rf(n) });
    assert_eq!(cr, rr, "use_generated({n}) return mismatch");
    assert_eq!(
        cr,
        init(),
        "C use_generated({n}) should fall into `default: break` and yield INIT={}",
        init()
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout)
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        format!("gen.acc={}\n", init())
    );
}

/// Row 4 — `n == 7`, one past the last `case` (and the largest legal REPEAT).
#[test]
fn err_04_dispatch_n_seven() {
    assert_dispatch_default(7);
}

/// Row 5 — `n == 8`.
#[test]
fn err_05_dispatch_n_eight() {
    assert_dispatch_default(8);
}

/// Row 6 — `n == -1`.
#[test]
fn err_06_dispatch_n_negative_one() {
    assert_dispatch_default(-1);
}

/// Row 7 — `n == INT_MIN`.
#[test]
fn err_07_dispatch_n_int_min() {
    assert_dispatch_default(c_int::MIN);
}

/// Row 8 — `n == INT_MAX`.
#[test]
fn err_08_dispatch_n_int_max() {
    assert_dispatch_default(c_int::MAX);
}

/// Row 9 — the whole switch range plus a fuzz of full-range `int`s.
#[test]
fn err_09_dispatch_full_range_and_fuzz() {
    for n in -64..=64 {
        if (0..=6).contains(&n) {
            assert_fn1_eq("use_generated", n); // accepted cases
        } else {
            assert_dispatch_default(n); // rejected -> INIT
        }
    }
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let n = rng.next_i32();
        if (0..=6).contains(&n) {
            assert_fn1_eq("use_generated", n);
        } else {
            assert_dispatch_default(n);
        }
    }
}

/// Row 10 — out-of-range "enum-like" bit patterns handed across the FFI
/// boundary as plain `int`s (a C `switch` accepts any `int`).
#[test]
fn err_10_dispatch_bit_pattern_ints() {
    let patterns: [c_int; 10] = [
        0x7fff_ffff,
        -0x8000_0000,
        0xdead_beefu32 as i32,
        0xffff_ffffu32 as i32,
        0x0000_00ffu32 as i32,
        0x0000_0100,
        1 << 30,
        -(1 << 30),
        0x5555_5555,
        0x2aaa_aaaa,
    ];
    for n in patterns {
        if (0..=6).contains(&n) {
            assert_fn1_eq("use_generated", n);
        } else {
            assert_dispatch_default(n);
        }
    }
}

/* --------------- rows 11-15: signed-overflow boundary behaviour ----------- */

/// Row 11 — `op_add` at the signed-overflow boundaries.
#[test]
fn err_11_op_add_overflow() {
    let cases = [
        (c_int::MAX, 1),
        (1, c_int::MAX),
        (c_int::MIN, -1),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, c_int::MIN),
    ];
    for (a, b) in cases {
        assert_fn2_eq("op_add", a, b);
    }
}

/// Row 12 — `op_sub` at the signed-overflow boundaries.
#[test]
fn err_12_op_sub_overflow() {
    let cases = [
        (c_int::MIN, 1),
        (c_int::MAX, -1),
        (0, c_int::MIN),
        (c_int::MIN, c_int::MAX),
        (c_int::MAX, c_int::MIN),
        (-1, c_int::MAX),
    ];
    for (a, b) in cases {
        assert_fn2_eq("op_sub", a, b);
    }
}

/// Row 13 — `op_mul` at the signed-overflow boundaries.
#[test]
fn err_13_op_mul_overflow() {
    let cases = [
        (c_int::MIN, -1),
        (-1, c_int::MIN),
        (c_int::MAX, 2),
        (65536, 65536),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, c_int::MAX),
        (46341, 46341),
    ];
    for (a, b) in cases {
        assert_fn2_eq("op_mul", a, b);
    }
}

/// Row 14 — overflow of `helper_call`'s composed `r + acc`.
#[test]
fn err_14_helper_call_overflow() {
    let cases = [
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, 1),
        (c_int::MIN, -1),
        (c_int::MAX, 0),
        (0, c_int::MIN),
        (46341, 46341),
        (65536, 65536),
    ];
    for (a, b) in cases {
        assert_fn2_eq("helper_call", a, b);
    }
}

/// Row 15 — overflow through `helper_ptr`'s indirect call.
#[test]
fn err_15_helper_ptr_overflow() {
    let cases = [
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, 1),
        (c_int::MIN, -1),
        (65536, 65536),
        (c_int::MIN, c_int::MAX),
    ];
    for (a, b) in cases {
        assert_fn2_eq("helper_ptr", a, b);
    }
}

/* ------------- rows 16-17: the exported globals are mutable data ---------- */

/// Row 16 — `int (*G_OP)(int,int)` is a writable object in both libraries; a
/// caller may store into it and read the value back.
#[test]
fn err_16_g_op_is_writable() {
    let (c_slot, r_slot) = data_sym("G_OP");
    let (c_sub, r_sub) = fn_addr("op_sub");
    unsafe {
        let c_orig = *(c_slot as *const usize);
        let r_orig = *(r_slot as *const usize);

        *(c_slot as *mut usize) = c_sub;
        *(r_slot as *mut usize) = r_sub;
        assert_eq!(*(c_slot as *const usize), c_sub, "C G_OP store lost");
        assert_eq!(
            *(r_slot as *const usize),
            r_sub,
            "Rust G_OP is not writable — C's G_OP lives in .data, so a store \
             through the exported symbol must succeed"
        );

        // Storing into G_OP must not change any function's behaviour: no C
        // function in mdcore.c reads G_OP.
        let (cf, rf) = sym2("helper_ptr");
        let (cr, cout) = capture(|| cf(9, 4));
        let (rr, rout) = capture(|| rf(9, 4));
        assert_eq!(cr, rr);
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );

        *(c_slot as *mut usize) = c_orig;
        *(r_slot as *mut usize) = r_orig;
    }
}

/// Row 17 — `const char *G_OP_NAME` is likewise a writable pointer object;
/// storing `NULL` (and an arbitrary string) must work in both.
#[test]
fn err_17_g_op_name_is_writable() {
    let (c_slot, r_slot) = data_sym("G_OP_NAME");
    let replacement = b"zzz\0";
    unsafe {
        let c_orig = *(c_slot as *const *const c_char);
        let r_orig = *(r_slot as *const *const c_char);

        *(c_slot as *mut *const c_char) = std::ptr::null();
        *(r_slot as *mut *const c_char) = std::ptr::null();
        assert!((*(c_slot as *const *const c_char)).is_null());
        assert!(
            (*(r_slot as *const *const c_char)).is_null(),
            "Rust G_OP_NAME is not writable"
        );

        let p = replacement.as_ptr() as *const c_char;
        *(c_slot as *mut *const c_char) = p;
        *(r_slot as *mut *const c_char) = p;
        assert_eq!(
            cstr_bytes(*(c_slot as *const *const c_char)),
            cstr_bytes(*(r_slot as *const *const c_char))
        );

        *(c_slot as *mut *const c_char) = c_orig;
        *(r_slot as *mut *const c_char) = r_orig;
        assert_eq!(cstr_bytes(c_orig), cstr_bytes(r_orig));
    }
}

/* ------------- rows 20-21: `atoi` has no way to report an error ----------- */

/// Row 20 — non-numeric / partially numeric / empty argv.
#[test]
fn err_20_main_atoi_non_numeric() {
    let cases: &[[&str; 2]] = &[
        ["abc", "5"],
        ["", ""],
        ["+", "-"],
        ["-", "+"],
        [" \t\n 42", "  -42"],
        ["12abc", "34xyz"],
        ["0x10", "0b11"],
        ["--5", "++5"],
        ["5.9", "-5.9"],
        ["١٢٣", "5"],
        ["2147483648", "-2147483649"],
        ["007", "-007"],
        ["+0", "-0"],
    ];
    for c in cases {
        assert_driver_eq(&[c[0], c[1]]);
    }
}

/// Row 21 — argv values far outside `int` (glibc `atoi` == `(int)strtol`).
#[test]
fn err_21_main_atoi_overflow() {
    let cases: &[[&str; 2]] = &[
        ["99999999999999999999", "-99999999999999999999"],
        ["9223372036854775807", "-9223372036854775808"],
        ["9223372036854775808", "-9223372036854775809"],
        ["4294967296", "4294967295"],
        ["10000000000", "-10000000000"],
        [
            "123456789012345678901234567890",
            "-123456789012345678901234567890",
        ],
    ];
    for c in cases {
        assert_driver_eq(&[c[0], c[1]]);
    }
}

/* ---------- rows 22-25: process-level boundaries of the `driver` bin -------- */

/// Runs `exe` with a completely empty `argv` (so `argc == 0` and `argv[0]` is
/// `NULL`), capturing stderr and the exit status.
fn run_argc0(exe: &std::path::Path) -> (Vec<u8>, i32) {
    use std::ffi::CString;
    let path = CString::new(exe.to_str().unwrap()).unwrap();
    let mut fds = [0i32; 2];
    unsafe {
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            libc::dup2(fds[1], 2);
            libc::close(fds[0]);
            libc::close(fds[1]);
            let argv: [*const c_char; 1] = [std::ptr::null()];
            let envp: [*const c_char; 1] = [std::ptr::null()];
            libc::execve(path.as_ptr(), argv.as_ptr(), envp.as_ptr());
            libc::_exit(127);
        }
        libc::close(fds[1]);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 512];
        loop {
            let n = libc::read(fds[0], chunk.as_mut_ptr() as *mut libc::c_void, 512);
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(fds[0]);
        let mut status = 0i32;
        libc::waitpid(pid, &mut status, 0);
        let code = if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else {
            -1
        };
        (buf, code)
    }
}

/// Row 22 — `argc == 0`, i.e. `argv[0] == NULL` reaching `%s`.
#[test]
fn err_22_main_argc_zero_null_argv0() {
    let (c_err, c_code) = run_argc0(&c_exe_path());
    let (r_err, r_code) = run_argc0(&rust_exe_path());
    assert_eq!(c_code, 2, "C must still exit 2 with argc==0");
    assert_eq!(c_code, r_code, "exit status mismatch for argc==0");
    assert_eq!(
        String::from_utf8_lossy(&c_err),
        String::from_utf8_lossy(&r_err),
        "stderr mismatch for argc==0"
    );
}

/// Row 23 — every `printf` fails with ENOSPC (stdout is `/dev/full`).
#[test]
fn err_23_stdout_write_error_ignored() {
    fn run(exe: &std::path::Path) -> (i32, Vec<u8>) {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full");
        let out = std::process::Command::new(exe)
            .args(["3", "4"])
            .stdout(full)
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("spawn");
        (out.status.code().unwrap_or(-1), out.stderr)
    }
    let (cc, ce) = run(&c_exe_path());
    let (rc, re) = run(&rust_exe_path());
    assert_eq!(cc, 0, "C ignores printf failures and returns 0");
    assert_eq!(cc, rc, "exit status mismatch with failing stdout");
    assert_eq!(
        String::from_utf8_lossy(&ce),
        String::from_utf8_lossy(&re),
        "stderr mismatch with failing stdout"
    );
}

/// Row 24 — the usage `fprintf` fails (stderr is `/dev/full`).
#[test]
fn err_24_stderr_write_error_ignored() {
    fn run(exe: &std::path::Path) -> i32 {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full");
        std::process::Command::new(exe)
            .args(["5"])
            .stderr(full)
            .stdout(std::process::Stdio::piped())
            .output()
            .expect("spawn")
            .status
            .code()
            .unwrap_or(-1)
    }
    let c = run(&c_exe_path());
    let r = run(&rust_exe_path());
    assert_eq!(c, 2, "C still exits 2 when the usage write fails");
    assert_eq!(c, r, "exit status mismatch with failing stderr");
}

/// Row 25 — fd 1 closed before `exec`, so every write returns EBADF.
#[test]
fn err_25_stdout_closed() {
    fn run(exe: &std::path::Path) -> (i32, Vec<u8>) {
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(exe);
        cmd.args(["3", "4"]).stderr(std::process::Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                libc::close(1);
                Ok(())
            });
        }
        let out = cmd.output().expect("spawn");
        (out.status.code().unwrap_or(-1), out.stderr)
    }
    let (cc, ce) = run(&c_exe_path());
    let (rc, re) = run(&rust_exe_path());
    assert_eq!(cc, 0, "C exits 0 with fd 1 closed");
    assert_eq!(cc, rc, "exit status mismatch with fd 1 closed");
    assert_eq!(
        String::from_utf8_lossy(&ce),
        String::from_utf8_lossy(&re),
        "stderr mismatch with fd 1 closed"
    );
}
