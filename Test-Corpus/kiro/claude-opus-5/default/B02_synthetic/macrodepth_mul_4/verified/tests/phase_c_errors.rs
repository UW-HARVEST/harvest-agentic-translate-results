//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! The library has no error codes: `op_*`, `helper_*` and `use_generated` are total
//! on `int`. "Same rejection" therefore means the same returned `int` **and** the
//! same stdout bytes, or for `main` the same stderr bytes and the same exit status.

mod common;

use common::{
    assert_same, c_exe_path, capture_stdout, pair, repo_root, rust_exe_path, Rng, INIT, OP_TAG,
    REPEAT, SEED,
};
use std::ffi::c_int;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers for the executable-level rows
// ---------------------------------------------------------------------------

struct Run {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(exe: &Path, args: &[&str]) -> Run {
    let out = Command::new(exe)
        .arg0("PROG")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    Run {
        status: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn assert_same_run(what: &str, args: &[&str]) -> Run {
    let c = run(&c_exe_path(), args);
    let r = run(&rust_exe_path(), args);
    assert_eq!(
        c.status, r.status,
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: exit status C={:?} Rust={:?}",
        c.status, r.status
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: stdout differs"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: stderr differs"
    );
    c
}

// ---------------------------------------------------------------------------
// Rows 1-3 — the single explicit rejection: mdmain.c:29 `if (argc < 3)`.
// ---------------------------------------------------------------------------

#[test]
fn err_01_argc_1_usage_exit2() {
    let c = assert_same_run("argc==1", &[]);
    assert_eq!(c.status, Some(2), "C must exit 2 on argc<3");
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        "usage: PROG A B\n",
        "usage line must go to stderr verbatim"
    );
    assert!(c.stdout.is_empty(), "nothing may reach stdout on argc<3");
}

#[test]
fn err_02_argc_2_usage_exit2() {
    let c = assert_same_run("argc==2", &["7"]);
    assert_eq!(c.status, Some(2));
    assert_eq!(String::from_utf8_lossy(&c.stderr), "usage: PROG A B\n");
    assert!(c.stdout.is_empty());
}

#[test]
fn err_03_argc_boundary_3_is_accepted() {
    // One step past the rejection: argc == 3 must be accepted, exit 0.
    let c = assert_same_run("argc==3", &["7", "3"]);
    assert_eq!(c.status, Some(0));
    assert!(c.stderr.is_empty());
    assert!(!c.stdout.is_empty());
}

#[test]
fn err_03b_argv0_shapes_in_usage_line() {
    // The usage line interpolates argv[0] with %s; check odd argv[0] values,
    // including the empty string and non-UTF-8 bytes.
    for arg0 in ["", "PROG", "./a b/driver", "\u{1f600}"] {
        let c = Command::new(c_exe_path()).arg0(arg0).output().unwrap();
        let r = Command::new(rust_exe_path()).arg0(arg0).output().unwrap();
        assert_eq!(c.status.code(), r.status.code(), "argv0={arg0:?} status");
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "argv0={arg0:?} stderr"
        );
    }

    // argc == 0: not reachable through Command (arg0 always yields argc >= 1),
    // so exec directly with an empty argv vector.
    let (c_err, c_status) = exec_with_empty_argv(&c_exe_path());
    let (r_err, r_status) = exec_with_empty_argv(&rust_exe_path());
    assert_eq!(c_status, r_status, "argc==0: exit status");
    assert_eq!(
        String::from_utf8_lossy(&c_err),
        String::from_utf8_lossy(&r_err),
        "argc==0: stderr"
    );
    assert_eq!(c_status, 2, "argc==0 still takes the argc<3 branch");
}

extern "C" {
    fn fork() -> i32;
    fn execv(path: *const std::ffi::c_char, argv: *const *const std::ffi::c_char) -> i32;
    fn waitpid(pid: i32, status: *mut c_int, options: c_int) -> i32;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Runs `exe` with `argc == 0` (an empty `argv` vector) and returns its stderr
/// bytes and exit status.
fn exec_with_empty_argv(exe: &Path) -> (Vec<u8>, i32) {
    use std::ffi::CString;
    use std::os::unix::io::AsRawFd;

    let path = CString::new(exe.to_str().expect("utf-8 path")).unwrap();
    let tmp = std::env::temp_dir().join(format!("mdargc0_{}.txt", std::process::id()));
    let file = std::fs::File::create(&tmp).expect("create argc0 capture");

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            dup2(file.as_raw_fd(), 2);
            let argv: [*const std::ffi::c_char; 1] = [std::ptr::null()];
            execv(path.as_ptr(), argv.as_ptr());
            _exit(127);
        }
    }
    let mut raw: c_int = 0;
    assert!(unsafe { waitpid(pid, &mut raw, 0) } == pid, "waitpid failed");
    drop(file);
    let bytes = std::fs::read(&tmp).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    // WEXITSTATUS
    (bytes, (raw >> 8) & 0xff)
}

// ---------------------------------------------------------------------------
// Rows 4-7 — atoi's non-rejections and its two overflow clamps.
// ---------------------------------------------------------------------------

#[test]
fn err_04_atoi_no_digits() {
    // No rejection: atoi yields 0 and the run proceeds with exit status 0.
    for s in ["", "abc", "+", "-", "   ", " \t", "+abc", "-abc", ".5", "e5"] {
        let c = assert_same_run("atoi no digits", &[s, s]);
        assert_eq!(c.status, Some(0), "atoi({s:?}) must not reject");
    }
    // Same result as an explicit zero operand.
    let zero = run(&c_exe_path(), &["0", "0"]);
    let junk = run(&c_exe_path(), &["abc", "xyz"]);
    assert_eq!(
        String::from_utf8_lossy(&zero.stdout),
        String::from_utf8_lossy(&junk.stdout),
        "atoi of a digitless string must behave as 0"
    );
}

#[test]
fn err_05_atoi_trailing_garbage() {
    for (s, equiv) in [
        ("12x", "12"),
        ("-12abc", "-12"),
        ("  -12abc", "-12"),
        ("+9z", "9"),
        ("0x10", "0"),
        ("1e3", "1"),
        ("5 5", "5"),
    ] {
        assert_same_run("atoi trailing garbage", &[s, "1"]);
        let got = run(&c_exe_path(), &[s, "1"]);
        let want = run(&c_exe_path(), &[equiv, "1"]);
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            String::from_utf8_lossy(&want.stdout),
            "atoi({s:?}) should equal atoi({equiv:?})"
        );
    }
}

#[test]
fn err_06_atoi_pos_overflow() {
    // strtol clamps to LONG_MAX; (int)LONG_MAX == -1.
    for s in [
        "99999999999999999999",
        "9223372036854775808",
        "18446744073709551616",
        "+99999999999999999999999999999999",
    ] {
        assert_same_run("atoi positive overflow", &[s, "0"]);
        let got = run(&c_exe_path(), &[s, "0"]);
        let want = run(&c_exe_path(), &["-1", "0"]);
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            String::from_utf8_lossy(&want.stdout),
            "atoi({s:?}) should clamp to LONG_MAX and truncate to -1"
        );
    }
}

#[test]
fn err_07_atoi_neg_overflow() {
    // strtol clamps to LONG_MIN; (int)LONG_MIN == 0 -- one further out than
    // -LONG_MAX, which would truncate to 1.
    for s in [
        "-99999999999999999999",
        "-9223372036854775809",
        "-18446744073709551616",
        "-99999999999999999999999999999999",
    ] {
        assert_same_run("atoi negative overflow", &[s, "0"]);
        let got = run(&c_exe_path(), &[s, "0"]);
        let want = run(&c_exe_path(), &["0", "0"]);
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            String::from_utf8_lossy(&want.stdout),
            "atoi({s:?}) should clamp to LONG_MIN and truncate to 0"
        );
    }
    // LONG_MIN exactly: representable, so no clamp, but still truncates to 0.
    assert_same_run("atoi LONG_MIN exactly", &["-9223372036854775808", "0"]);
    // LONG_MAX exactly: representable, truncates to -1.
    assert_same_run("atoi LONG_MAX exactly", &["9223372036854775807", "0"]);
}

// ---------------------------------------------------------------------------
// Rows 8-11, 23 — DISPATCH_REP's `default: break;` (mdmacros.h:91), i.e. the
// out-of-range selector crossing the FFI boundary. C's `switch` accepts any int.
// ---------------------------------------------------------------------------

fn assert_use_generated_returns(n: c_int, expected: c_int) {
    assert_same(&format!("use_generated({n})"), |imp| imp.use_generated(n));
    let (cv, cout) = capture_stdout(|| pair().c.use_generated(n));
    assert_eq!(
        cv, expected,
        "[{OP_TAG}/{REPEAT}] C use_generated({n}) expected {expected}"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        format!("gen.acc={expected}\n"),
        "[{OP_TAG}/{REPEAT}] C use_generated({n}) stdout"
    );
}

#[test]
fn err_08_use_generated_negative() {
    for n in [-1, -2, -7, -8, -100, i32::MIN, i32::MIN + 1] {
        assert_use_generated_returns(n, INIT);
    }
}

#[test]
fn err_09_use_generated_7_falls_to_default() {
    // REP7 exists in the header but DISPATCH_REP has no `case 7:`.
    assert_use_generated_returns(7, INIT);
}

#[test]
fn err_10_use_generated_above_range() {
    for n in [8, 9, 100, 1_000_000, i32::MAX - 1, i32::MAX] {
        assert_use_generated_returns(n, INIT);
    }
}

#[test]
fn err_11_use_generated_boundaries() {
    // The accepted values immediately inside the rejection on both sides.
    let step_add = |n: c_int| -> c_int { (0..n).sum() };
    for n in 0..=6 {
        let expected = match OP_TAG {
            "add" => step_add(n),
            "sub" => -step_add(n),
            _ => (0..n).fold(1, |acc, i| acc * (i + 1)),
        };
        assert_use_generated_returns(n, expected);
    }
    // and the two values that straddle the accepted window
    assert_use_generated_returns(-1, INIT);
    assert_use_generated_returns(7, INIT);
}

// ---------------------------------------------------------------------------
// Rows 12-16 — signed overflow: UB in C, unchecked, must wrap identically.
// ---------------------------------------------------------------------------

const OVERFLOW_PAIRS: &[(c_int, c_int)] = &[
    (i32::MAX, 1),
    (1, i32::MAX),
    (i32::MAX, i32::MAX),
    (i32::MIN, -1),
    (-1, i32::MIN),
    (i32::MIN, i32::MIN),
    (i32::MIN, 1),
    (i32::MAX, -1),
    (65_536, 65_536),
    (46_341, 46_341),
    (-46_341, 46_341),
    (i32::MAX, 2),
    (i32::MIN, 2),
    (i32::MAX / 2 + 1, 2),
];

#[test]
fn err_12_op_add_overflow() {
    let p = pair();
    for &(a, b) in OVERFLOW_PAIRS {
        assert_eq!(
            p.c.op_add(a, b),
            p.rust.op_add(a, b),
            "[{OP_TAG}/{REPEAT}] op_add overflow({a}, {b})"
        );
    }
}

#[test]
fn err_13_op_sub_overflow() {
    let p = pair();
    for &(a, b) in OVERFLOW_PAIRS {
        assert_eq!(
            p.c.op_sub(a, b),
            p.rust.op_sub(a, b),
            "[{OP_TAG}/{REPEAT}] op_sub overflow({a}, {b})"
        );
    }
}

#[test]
fn err_14_op_mul_overflow() {
    let p = pair();
    for &(a, b) in OVERFLOW_PAIRS {
        assert_eq!(
            p.c.op_mul(a, b),
            p.rust.op_mul(a, b),
            "[{OP_TAG}/{REPEAT}] op_mul overflow({a}, {b})"
        );
    }
}

#[test]
fn err_15_helper_call_sum_overflow() {
    // helper_call returns r + acc; with OP=add and a=INT_MAX the sum wraps.
    for &(a, b) in OVERFLOW_PAIRS {
        assert_same(&format!("helper_call overflow({a}, {b})"), |imp| {
            imp.helper_call(a, b)
        });
        assert_same(&format!("helper_ptr overflow({a}, {b})"), |imp| {
            imp.helper_ptr(a, b)
        });
    }
    // Random hunt for wrapping sums, since which pairs overflow depends on OP
    // and on REPEAT's contribution to acc.
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..512 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        assert_same(&format!("helper_call wrap({a}, {b})"), |imp| {
            imp.helper_call(a, b)
        });
    }
}

#[test]
fn err_16_step_mul_overflow_reachable_range() {
    // STEP_mul multiplies by (i+1); from INIT_mul = 1 and REPEAT <= 7 the largest
    // value is 7! = 5040, so the unrolling itself cannot overflow. The overflow
    // that *is* reachable comes from op_mul's operands, checked above; this row
    // pins the unrolled accumulator to its exact C value in every mul build.
    if OP_TAG != "mul" {
        return;
    }
    let expected: c_int = (0..REPEAT).fold(1, |acc, i| acc * (i + 1));
    let (_, out) = capture_stdout(|| pair().c.helper_call(1, 1));
    assert!(
        String::from_utf8_lossy(&out).contains(&format!("helper.acc={expected}\n")),
        "[mul/{REPEAT}] expected helper.acc={expected}, got {:?}",
        String::from_utf8_lossy(&out)
    );
    assert_same("helper_call(1,1) mul", |imp| imp.helper_call(1, 1));
}

// ---------------------------------------------------------------------------
// Row 18-20 — build-time rejections. Asserted by invoking the compilers.
// ---------------------------------------------------------------------------

fn gcc_build_fails(op: &str, repeat: &str) -> String {
    let root = repo_root();
    let out = Command::new("gcc")
        .args(["-O2", "-c", &format!("-DOP={op}"), &format!("-DREPEAT={repeat}")])
        .arg(format!("-I{}", root.join("c_src/src").display()))
        .arg("-o")
        .arg("/dev/null")
        .arg(root.join("c_src/src/mdcore.c"))
        .output()
        .expect("run gcc");
    assert!(
        !out.status.success(),
        "gcc unexpectedly accepted -DOP={op} -DREPEAT={repeat}"
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Cargo features present in `Cargo.toml`, so the Rust side's rejection of an
/// out-of-range configuration can be asserted without a nested cargo invocation.
fn declared_features() -> Vec<String> {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read Cargo.toml");
    let body = text
        .split("[features]")
        .nth(1)
        .expect("Cargo.toml has [features]");
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            out.push(name.trim().trim_matches('"').to_string());
        }
    }
    out
}

#[test]
fn err_18_build_time_bad_op() {
    // mdmacros.h:45/52/59 paste OP into op_/STEP_/INIT_ identifiers, so any token
    // outside add|sub|mul fails to compile.
    let err = gcc_build_fails("div", "5");
    assert!(
        err.contains("INIT_div") || err.contains("op_div") || err.contains("STEP_div"),
        "expected a token-paste failure naming the bad OP, got:\n{err}"
    );
    // Rust side: no such feature exists, so `--features div` is rejected by cargo.
    let feats = declared_features();
    for bad in ["div", "mod", "xor"] {
        assert!(
            !feats.contains(&bad.to_string()),
            "Cargo.toml must not declare an OP feature {bad:?} the C cannot build"
        );
    }
    for good in ["add", "sub", "mul"] {
        assert!(feats.contains(&good.to_string()), "missing feature {good}");
    }
}

#[test]
fn err_19_build_time_repeat_8() {
    // CHOOSE_REP(8) -> REP8, which the header does not define.
    let err = gcc_build_fails("add", "8");
    assert!(
        err.contains("REP8") || err.contains("undeclared"),
        "expected a missing-REP8 failure, got:\n{err}"
    );
    let feats = declared_features();
    for bad in ["8", "9", "10"] {
        assert!(
            !feats.contains(&bad.to_string()),
            "Cargo.toml must not declare REPEAT feature {bad:?}; the C has no REP{bad}"
        );
    }
    for good in ["0", "1", "2", "3", "4", "5", "6", "7"] {
        assert!(feats.contains(&good.to_string()), "missing feature {good}");
    }
}

#[test]
fn err_20_build_time_repeat_negative() {
    let err = gcc_build_fails("add", "-1");
    assert!(
        err.contains("does not give a valid preprocessing token") || err.contains("REP"),
        "expected a token-paste failure for a negative REPEAT, got:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// Row 21 — FOR_EACH / DO_LOOP with a non-positive bound.
//
// The header defines DO_LOOP but mdcore.c never instantiates it, so it produces
// no symbol in either object. To keep the row differential rather than
// assumed, a fixture .so is compiled *from the unmodified header* that does
// instantiate it, and its runtime loop is compared against the unrolled REP<n>
// path that both real libraries expose through use_generated.
// ---------------------------------------------------------------------------

#[test]
fn err_21_do_loop_nonpositive() {
    use libloading::Library;

    let root = repo_root();
    let dir = root.join("cbuild");
    std::fs::create_dir_all(&dir).expect("create cbuild/");
    let src = dir.join(format!("do_loop_fixture_{}.c", common::config_tag()));
    let so = dir.join(format!("libdoloop_{}.so", common::config_tag()));
    std::fs::write(
        &src,
        "#include \"mdmacros.h\"\n\
         int md_do_loop(int acc, int n) { DO_LOOP(OP, acc, n); return acc; }\n\
         int md_init(void) { return INIT_FOR(OP); }\n",
    )
    .expect("write fixture");
    let status = Command::new("gcc")
        .args([
            "-O2",
            "-fPIC",
            "-shared",
            &format!("-DOP={OP_TAG}"),
            &format!("-DREPEAT={REPEAT}"),
        ])
        .arg(format!("-I{}", root.join("c_src/src").display()))
        .arg("-o")
        .arg(&so)
        .arg(&src)
        .status()
        .expect("run gcc for DO_LOOP fixture");
    assert!(status.success(), "DO_LOOP fixture failed to build");

    let lib = unsafe { Library::new(&so) }.expect("dlopen DO_LOOP fixture");
    let do_loop: libloading::Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"md_do_loop\0") }.expect("md_do_loop");
    let md_init: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"md_init\0") }.expect("md_init");

    assert_eq!(unsafe { md_init() }, INIT, "INIT_FOR(OP) mismatch");

    // n <= 0: `i < (n)` is false on entry, so acc is returned untouched.
    for n in [0, -1, -2, -100, i32::MIN] {
        for acc in [INIT, 0, 1, -1, 42, i32::MAX, i32::MIN] {
            assert_eq!(
                unsafe { do_loop(acc, n) },
                acc,
                "DO_LOOP(acc={acc}, n={n}) must not iterate"
            );
        }
    }

    // n in 0..=6: the runtime loop must equal the unrolled REP<n> that both real
    // libraries reach through use_generated / DISPATCH_REP.
    let p = pair();
    for n in 0..=6 {
        let via_macro = unsafe { do_loop(INIT, n) };
        let (c_gen, _) = capture_stdout(|| p.c.use_generated(n));
        let (r_gen, _) = capture_stdout(|| p.rust.use_generated(n));
        assert_eq!(via_macro, c_gen, "DO_LOOP vs C REP{n}");
        assert_eq!(via_macro, r_gen, "DO_LOOP vs Rust REP{n}");
    }
    // n == 7: DO_LOOP still iterates seven times (it is not the switch), which is
    // exactly where use_generated diverges by design.
    let seven = unsafe { do_loop(INIT, 7) };
    let (c_gen7, _) = capture_stdout(|| p.c.use_generated(7));
    assert_eq!(c_gen7, INIT, "use_generated(7) hits `default`");
    let expected7: c_int = match OP_TAG {
        "add" => (0..7).sum(),
        "sub" => -(0..7i32).sum::<c_int>(),
        _ => (0..7).fold(1, |acc, i| acc * (i + 1)),
    };
    assert_eq!(seven, expected7, "DO_LOOP(7) is the full seven-step result");
}

// ---------------------------------------------------------------------------
// Row 22 — the ABI takes no pointer or length arguments, so there is no
// null-pointer / zero-length / oversized-length surface to diverge on. Recorded
// as an explicit assertion so the absence is checked, not assumed.
// ---------------------------------------------------------------------------

#[test]
fn err_22_no_pointer_params() {
    let p = pair();
    // All six exported functions bind successfully with the int-only signatures
    // declared in mdmacros.h:40-42 and 108-110.
    let mut rng = Rng::new(SEED ^ 0x22);
    let (a, b) = (rng.next_i32(), rng.next_i32());
    let _ = p.c.op_add(a, b);
    let _ = p.c.op_sub(a, b);
    let _ = p.c.op_mul(a, b);
    let _ = capture_stdout(|| p.c.helper_call(a, b));
    let _ = capture_stdout(|| p.c.helper_ptr(a, b));
    let _ = capture_stdout(|| p.c.use_generated(0));

    // The only pointer in the ABI is the outbound G_OP_NAME slot; it must be
    // non-null and NUL-terminated in both libraries (g_op_name asserts both).
    assert_eq!(p.c.g_op_name(), p.rust.g_op_name());
    assert!(!p.c.g_op_name().is_empty());

    // And G_OP, the only function-pointer slot, is non-null in both.
    assert!(p.c.g_op() as usize != 0);
    assert!(p.rust.g_op() as usize != 0);
}
