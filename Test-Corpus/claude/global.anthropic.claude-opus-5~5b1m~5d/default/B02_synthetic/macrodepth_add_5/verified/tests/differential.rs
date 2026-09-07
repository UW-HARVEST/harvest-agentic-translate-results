//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare every exported symbol's behaviour through the FFI
//! boundary.  No Rust function is ever called directly — everything goes
//! through the `#[no_mangle]` exports, exactly as an external C consumer would.
//!
//! The C `.so` for the *matching* build configuration is expected at
//! `<repo-root>/cbuild/<OP>_<REPEAT>/libdriver_c.so` (see `build_c.sh`), and the
//! matching driver executable at `<repo-root>/cbuild/<OP>_<REPEAT>/driver_c`.

use std::ffi::{c_char, c_int, CStr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

use libloading::{Library, Symbol};

/// fd 1 is process-global: every test that makes the `.so`s `printf`, and every
/// test that redirects fd 1 to capture that output, must be serialised.
static IO_LOCK: Mutex<()> = Mutex::new(());

fn io_guard() -> MutexGuard<'static, ()> {
    IO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/* ===================================================================== */
/* Build configuration, mirroring c_src/src/mdmacros.h's -DOP / -DREPEAT  */
/* and src/mdconfig.rs's feature-priority resolution.                     */
/* ===================================================================== */

#[cfg(feature = "add")]
const OP: &str = "add";
#[cfg(all(not(feature = "add"), feature = "sub"))]
const OP: &str = "sub";
#[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
const OP: &str = "mul";
#[cfg(not(any(feature = "add", feature = "sub", feature = "mul")))]
const OP: &str = "add";

#[cfg(feature = "0")]
const REPEAT: c_int = 0;
#[cfg(all(not(feature = "0"), feature = "1"))]
const REPEAT: c_int = 1;
#[cfg(all(not(feature = "0"), not(feature = "1"), feature = "2"))]
const REPEAT: c_int = 2;
#[cfg(all(not(feature = "0"), not(feature = "1"), not(feature = "2"), feature = "3"))]
const REPEAT: c_int = 3;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    feature = "4"
))]
const REPEAT: c_int = 4;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    feature = "5"
))]
const REPEAT: c_int = 5;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    feature = "6"
))]
const REPEAT: c_int = 6;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    not(feature = "6"),
    feature = "7"
))]
const REPEAT: c_int = 7;
#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "5",
    feature = "6",
    feature = "7"
)))]
const REPEAT: c_int = 5;

/// `INIT_FOR(OP)` as the C computes it.
fn init_expected() -> c_int {
    if OP == "mul" {
        1
    } else {
        0
    }
}

/* ===================================================================== */
/* Locating the two shared objects / two executables                      */
/* ===================================================================== */

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

fn c_dir() -> PathBuf {
    repo_root().join("cbuild").join(format!("{OP}_{REPEAT}"))
}

fn c_so_path() -> PathBuf {
    c_dir().join("libdriver_c.so")
}

fn c_driver_path() -> PathBuf {
    c_dir().join("driver_c")
}

/// `target/<profile>/deps/<test-exe>` -> `target/<profile>`
fn rust_out_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn rust_so_path() -> PathBuf {
    rust_out_dir().join("libdriver.so")
}

fn rust_driver_path() -> PathBuf {
    rust_out_dir().join("driver")
}

struct Pair {
    c: Library,
    r: Library,
}

impl Pair {
    fn load() -> Pair {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library missing: {}\nrun `./build_c.sh {OP} {REPEAT}` from the repo root",
            cp.display()
        );
        assert!(
            rp.exists(),
            "Rust shared library missing: {}\nrun `cargo build --no-default-features --features {OP},{REPEAT}`",
            rp.display()
        );
        let pair = unsafe {
            Pair {
                c: Library::new(&cp).expect("dlopen C .so"),
                r: Library::new(&rp).expect("dlopen Rust .so"),
            }
        };
        pair.assert_config_matches();
        assert_repeat_matches();
        pair
    }

    /// `cargo test` does NOT rebuild the `cdylib` artifact (it only builds the
    /// lib *test* harness), so `target/<profile>/libdriver.so` can be left over
    /// from a previous `cargo build` with different features.  Detect that
    /// loudly instead of silently comparing two different configurations.
    fn assert_config_matches(&self) {
        for (which, lib) in [("C", &self.c), ("Rust", &self.r)] {
            let n: Symbol<CharPtrVar> = unsafe { lib.get(b"G_OP_NAME\0") }.expect("G_OP_NAME");
            let p = unsafe { **n };
            assert!(!p.is_null(), "{which} G_OP_NAME is NULL");
            let got = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
            assert_eq!(
                got, OP,
                "{which} .so was built for OP={got}, but this test binary was built for \
                 OP={OP}. Run `cargo build --release --no-default-features --features \
                 {OP},{REPEAT}` (and `./build_c.sh {OP} {REPEAT}`) before `cargo test`; \
                 `run_diff.sh` does both."
            );
        }
    }
}

/// Second half of the stale-artifact guard: `G_OP_NAME` only pins `OP`, so
/// probe `REPEAT` too.  `helper_call(0,0)` == `op(0,0) + RUN_LOOP(INIT)` and
/// `op(0,0) == 0` for add/sub/mul, so the return value is exactly the
/// `REPEAT`-dependent accumulator.  Probed in a subprocess so the library's
/// `printf` does not pollute the test harness's stdout.
static CONFIG_CHECKED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn expected_accumulator(reps: c_int) -> c_int {
    let mut acc = init_expected();
    let mut i: c_int = 0;
    while i < reps {
        acc = match OP {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i.wrapping_add(1)),
        };
        i += 1;
    }
    acc
}

fn assert_repeat_matches() {
    CONFIG_CHECKED.get_or_init(|| {
        let helper = socall_path();
        if !helper.exists() {
            return;
        }
        let want = expected_accumulator(REPEAT);
        for (which, so) in [("C", c_so_path()), ("Rust", rust_so_path())] {
            let out = Command::new(&helper)
                .arg(&so)
                .args(["helper_call", "0", "0"])
                .output()
                .expect("spawn socall");
            let txt = String::from_utf8_lossy(&out.stdout).into_owned();
            let ret: c_int = txt
                .lines()
                .find_map(|l| l.strip_prefix("ret="))
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or_else(|| panic!("socall gave no ret= line: {txt:?}"));
            assert_eq!(
                ret, want,
                "{which} .so's RUN_LOOP accumulator is {ret}, expected {want} for \
                 REPEAT={REPEAT}: the .so on disk was built with different features than \
                 this test binary. Run `cargo build --release --no-default-features \
                 --features {OP},{REPEAT}` (and `./build_c.sh {OP} {REPEAT}`) first; \
                 `run_diff.sh` does both."
            );
        }
    });
}

type BinFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnFn = unsafe extern "C" fn(c_int) -> c_int;
/// A data symbol holding an `int (*)(int,int)`: `Symbol<T>` reinterprets the
/// symbol *address* as `T`, so `T` must be a pointer *to* the variable.
type FnPtrVar = *const BinFn;
/// A data symbol holding a `const char *`.
type CharPtrVar = *const *const c_char;

fn sym_bin<'l>(lib: &'l Library, name: &[u8]) -> Symbol<'l, BinFn> {
    unsafe { lib.get(name) }.unwrap_or_else(|e| panic!("missing {:?}: {e}", String::from_utf8_lossy(name)))
}

fn sym_un<'l>(lib: &'l Library, name: &[u8]) -> Symbol<'l, UnFn> {
    unsafe { lib.get(name) }.unwrap_or_else(|e| panic!("missing {:?}: {e}", String::from_utf8_lossy(name)))
}

/* ===================================================================== */
/* Input generation                                                       */
/* ===================================================================== */

/// Fixed-seed xorshift64* — reproducible, no external dependency.
struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        Rng(0x5EED_1234_ABCD_0001)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[allow(dead_code)]
    fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Mix of full-range values and small values (small values exercise the
    /// non-overflowing arithmetic paths, full-range the wrapping ones).
    fn next_mixed(&mut self) -> c_int {
        let v = self.next_u64();
        match v & 3 {
            0 => (v >> 32) as u32 as c_int,
            1 => ((v >> 32) as u32 % 256) as c_int,
            2 => -(((v >> 32) as u32 % 256) as c_int),
            _ => ((v >> 32) as u32 as c_int) / 2,
        }
    }
}

const BOUNDARY: [c_int; 13] = [
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    65535,
    65536,
    c_int::MAX,
    c_int::MAX - 1,
    c_int::MIN,
    c_int::MIN + 1,
];

/// The `(a,b)` corpus used by every binary-function row: full boundary cross
/// product (169) + `n` seeded-random mixed pairs.
fn pair_corpus(n: usize) -> Vec<(c_int, c_int)> {
    let mut v = Vec::with_capacity(169 + n);
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            v.push((a, b));
        }
    }
    let mut rng = Rng::new();
    for _ in 0..n {
        v.push((rng.next_mixed(), rng.next_mixed()));
    }
    v
}

/* ===================================================================== */
/* Phase B — valid-path differential tests                                */
/* ===================================================================== */

fn diff_bin(name: &[u8], corpus: &[(c_int, c_int)]) {
    let _io = io_guard();
    let p = Pair::load();
    let f_c = sym_bin(&p.c, name);
    let f_r = sym_bin(&p.r, name);
    for &(a, b) in corpus {
        let vc = unsafe { f_c(a, b) };
        let vr = unsafe { f_r(a, b) };
        assert_eq!(
            vc,
            vr,
            "{} ({OP}/{REPEAT}) diverged for a={a} b={b}: C={vc} Rust={vr}",
            String::from_utf8_lossy(name)
        );
    }
}

fn diff_un(name: &[u8], inputs: &[c_int]) -> Vec<c_int> {
    let _io = io_guard();
    let p = Pair::load();
    let f_c = sym_un(&p.c, name);
    let f_r = sym_un(&p.r, name);
    let mut out = Vec::with_capacity(inputs.len());
    for &n in inputs {
        let vc = unsafe { f_c(n) };
        let vr = unsafe { f_r(n) };
        assert_eq!(
            vc,
            vr,
            "{} ({OP}/{REPEAT}) diverged for n={n}: C={vc} Rust={vr}",
            String::from_utf8_lossy(name)
        );
        out.push(vc);
    }
    out
}

// --- C1..C6: the three primitives, random + boundary ------------------------

#[test]
fn cfg_c1_c2_op_add() {
    diff_bin(b"op_add\0", &pair_corpus(2000));
}

#[test]
fn cfg_c3_c4_op_sub() {
    diff_bin(b"op_sub\0", &pair_corpus(2000));
}

#[test]
fn cfg_c5_c6_op_mul() {
    diff_bin(b"op_mul\0", &pair_corpus(2000));
}

// --- C7..C9: the G_OP function-pointer global ------------------------------

#[test]
fn cfg_c7_c9_g_op_matches_selected_op() {
    let p = Pair::load();
    let g_c: Symbol<FnPtrVar> = unsafe { p.c.get(b"G_OP\0") }.expect("C G_OP");
    let g_r: Symbol<FnPtrVar> = unsafe { p.r.get(b"G_OP\0") }.expect("Rust G_OP");
    let fc: BinFn = unsafe { **g_c };
    let fr: BinFn = unsafe { **g_r };

    // Identity: G_OP must point at the op_<OP> exported by the same library.
    let expect_name: &[u8] = match OP {
        "add" => b"op_add\0",
        "sub" => b"op_sub\0",
        "mul" => b"op_mul\0",
        _ => unreachable!(),
    };
    let direct_c = sym_bin(&p.c, expect_name);
    let direct_r = sym_bin(&p.r, expect_name);
    assert_eq!(
        fc as usize, *direct_c as usize,
        "C G_OP does not point at {}",
        String::from_utf8_lossy(expect_name)
    );
    assert_eq!(
        fr as usize, *direct_r as usize,
        "Rust G_OP does not point at {}",
        String::from_utf8_lossy(expect_name)
    );

    for (a, b) in pair_corpus(2000) {
        let vc = unsafe { fc(a, b) };
        let vr = unsafe { fr(a, b) };
        assert_eq!(vc, vr, "G_OP ({OP}/{REPEAT}) diverged for a={a} b={b}");
        // and it must agree with the direct call
        assert_eq!(vc, unsafe { direct_c(a, b) }, "C G_OP != direct op call");
        assert_eq!(vr, unsafe { direct_r(a, b) }, "Rust G_OP != direct op call");
    }
}

// --- C10..C12: the G_OP_NAME string global ---------------------------------

#[test]
fn cfg_c10_c12_g_op_name() {
    let p = Pair::load();
    let n_c: Symbol<CharPtrVar> = unsafe { p.c.get(b"G_OP_NAME\0") }.expect("C G_OP_NAME");
    let n_r: Symbol<CharPtrVar> = unsafe { p.r.get(b"G_OP_NAME\0") }.expect("Rust G_OP_NAME");
    let pc = unsafe { **n_c };
    let pr = unsafe { **n_r };
    assert!(!pc.is_null(), "C G_OP_NAME is NULL");
    assert!(!pr.is_null(), "Rust G_OP_NAME is NULL");
    let sc = unsafe { CStr::from_ptr(pc) };
    let sr = unsafe { CStr::from_ptr(pr) };
    assert_eq!(sc.to_bytes(), sr.to_bytes(), "G_OP_NAME bytes differ");
    assert_eq!(sc.to_bytes(), OP.as_bytes(), "G_OP_NAME != STR(OP)");
}

// --- C13..C15: helper_ptr (indirect call) ----------------------------------

#[test]
fn cfg_c13_c15_helper_ptr() {
    diff_bin(b"helper_ptr\0", &pair_corpus(512));
}

// --- C16..C20: use_generated / DISPATCH_REP -------------------------------

#[test]
fn cfg_c16_c18_use_generated_switch_cases() {
    let vals = diff_un(b"use_generated\0", &[0, 1, 2, 3, 4, 5, 6]);
    // Sanity-check against the C semantics spelled out in mdmacros.h, so a
    // *shared* bug in both sides would still be caught.
    let init = init_expected();
    for (n, got) in vals.iter().enumerate() {
        let mut acc = init;
        for i in 0..n as c_int {
            acc = match OP {
                "add" => acc.wrapping_add(i),
                "sub" => acc.wrapping_sub(i),
                _ => acc.wrapping_mul(i.wrapping_add(1)),
            };
        }
        assert_eq!(*got, acc, "use_generated({n}) unexpected for OP={OP}");
    }
}

#[test]
fn cfg_c19_use_generated_at_repeat() {
    // main() calls use_generated(REPEAT); REPEAT==7 hits `default:` (no case 7).
    let v = diff_un(b"use_generated\0", &[REPEAT]);
    if REPEAT == 7 {
        assert_eq!(v[0], init_expected(), "REPEAT=7 must hit DISPATCH_REP default");
    }
}

#[test]
fn cfg_c20_use_generated_random_n() {
    let mut rng = Rng::new();
    let mut inputs: Vec<c_int> = (0..512).map(|_| rng.next_mixed()).collect();
    inputs.extend_from_slice(&BOUNDARY);
    inputs.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100, -1, -100]);
    diff_un(b"use_generated\0", &inputs);
}

// --- C21..C44: helper_call, the composed OP x REPEAT pipeline -------------

#[test]
fn cfg_c21_c44_helper_call() {
    diff_bin(b"helper_call\0", &pair_corpus(512));
}

#[test]
fn cfg_c21_c44_helper_call_accumulator_is_repeat_dependent() {
    let _io = io_guard();
    // helper_call returns op(a,b) + RUN_LOOP-accumulated INIT.  With a=b=0 the
    // op contributes 0 for add/sub and 0 for mul, isolating the accumulator, so
    // this pins the REPEAT-dependent half of the pipeline.
    let p = Pair::load();
    let f_c = sym_bin(&p.c, b"helper_call\0");
    let f_r = sym_bin(&p.r, b"helper_call\0");
    let vc = unsafe { f_c(0, 0) };
    let vr = unsafe { f_r(0, 0) };
    assert_eq!(vc, vr);

    let mut acc = init_expected();
    for i in 0..REPEAT {
        acc = match OP {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i.wrapping_add(1)),
        };
    }
    let op0: c_int = match OP {
        "add" | "sub" => 0,
        _ => 0,
    };
    assert_eq!(
        vc,
        op0.wrapping_add(acc),
        "helper_call(0,0) unexpected for OP={OP} REPEAT={REPEAT}"
    );
}

// --- C45: the whole pipeline, in main()'s order, through the .so ---------

#[test]
fn cfg_c45_full_pipeline_summary() {
    let _io = io_guard();
    let p = Pair::load();
    let op_name: &[u8] = match OP {
        "add" => b"op_add\0",
        "sub" => b"op_sub\0",
        _ => b"op_mul\0",
    };

    let sums = |lib: &Library, a: c_int, b: c_int| -> c_int {
        let op = sym_bin(lib, op_name);
        let hc = sym_bin(lib, b"helper_call\0");
        let hp = sym_bin(lib, b"helper_ptr\0");
        let ug = sym_un(lib, b"use_generated\0");
        let g: Symbol<FnPtrVar> = unsafe { lib.get(b"G_OP\0") }.expect("G_OP");
        let gf: BinFn = unsafe { **g };

        let r_call = unsafe { op(a, b) };
        let mut acc = init_expected();
        for i in 0..REPEAT {
            acc = match OP {
                "add" => acc.wrapping_add(i),
                "sub" => acc.wrapping_sub(i),
                _ => acc.wrapping_mul(i.wrapping_add(1)),
            };
        }
        let x1 = unsafe { hc(a, b) };
        let x2 = unsafe { hp(a, b) };
        let x3 = unsafe { ug(REPEAT) };
        let gv = unsafe { gf(a, b) };
        r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(gv)
    };

    for (a, b) in pair_corpus(256) {
        let sc = sums(&p.c, a, b);
        let sr = sums(&p.r, a, b);
        assert_eq!(sc, sr, "pipeline summary diverged for a={a} b={b} ({OP}/{REPEAT})");
    }
}

/* ===================================================================== */
/* C50..C52: byte-compare the *printf output* of the .so functions         */
/*                                                                        */
/* Done in a subprocess (examples/socall.rs) which dlopens one .so and     */
/* invokes one symbol, so the library's printf lands on that child's        */
/* stdout.  Redirecting this process's fd 1 is not an option: fd 1 is       */
/* shared with the test harness itself.                                    */
/* ===================================================================== */

fn socall_path() -> PathBuf {
    rust_out_dir().join("examples").join("socall")
}

/// Run `socall <so> <sym> <args...>` against both libraries and byte-compare
/// the combined stdout (the library's own `printf` output plus `ret=<value>`).
fn diff_socall(sym: &str, args: &[c_int]) {
    assert_repeat_matches();
    let helper = socall_path();
    assert!(
        helper.exists(),
        "socall helper missing: {} (run `cargo build --examples`)",
        helper.display()
    );
    let run_one = |so: PathBuf| -> (Vec<u8>, Option<i32>) {
        let mut cmd = Command::new(&helper);
        cmd.arg(so).arg(sym);
        for a in args {
            cmd.arg(a.to_string());
        }
        let out = cmd.output().expect("spawn socall");
        assert!(
            out.status.success(),
            "socall failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        (out.stdout, out.status.code())
    };
    let (oc, sc) = run_one(c_so_path());
    let (or, sr) = run_one(rust_so_path());
    assert_eq!(
        String::from_utf8_lossy(&oc),
        String::from_utf8_lossy(&or),
        "{sym}{args:?} printed output differs ({OP}/{REPEAT})"
    );
    assert_eq!(oc, or, "{sym}{args:?} stdout bytes differ");
    assert_eq!(sc, sr, "{sym}{args:?} exit status differs");
}

#[test]
fn cfg_c50_helper_call_printf_text() {
    for (a, b) in pair_corpus(24) {
        diff_socall("helper_call", &[a, b]);
    }
}

#[test]
fn cfg_c51_helper_ptr_printf_text() {
    for (a, b) in pair_corpus(24) {
        diff_socall("helper_ptr", &[a, b]);
    }
}

#[test]
fn cfg_c52_use_generated_printf_text() {
    let mut inputs: Vec<c_int> = vec![-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 100, REPEAT];
    inputs.extend_from_slice(&BOUNDARY);
    let mut rng = Rng::new();
    inputs.extend((0..24).map(|_| rng.next_mixed()));
    for n in inputs {
        diff_socall("use_generated", &[n]);
    }
}

#[test]
fn cfg_c53_op_and_globals_via_subprocess() {
    // Same, for the non-printing entry points: proves the exported symbols are
    // usable by a genuinely external consumer process, not just in-process.
    for sym in ["op_add", "op_sub", "op_mul"] {
        for (a, b) in [(3, 4), (-7, 9), (c_int::MAX, 1), (c_int::MIN, -1), (0, 0)] {
            diff_socall(sym, &[a, b]);
        }
    }
    for (a, b) in [(3, 4), (c_int::MAX, c_int::MAX), (c_int::MIN, c_int::MIN)] {
        diff_socall("G_OP", &[a, b]);
    }
    diff_socall("G_OP_NAME", &[]);
}

/* ===================================================================== */
/* Driver executable: stdout / stderr / exit status byte comparison        */
/* ===================================================================== */

fn run(path: &Path, args: &[&str]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let out = Command::new(path)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", path.display()));
    (out.stdout, out.stderr, out.status.code())
}

fn diff_driver(args: &[&str]) {
    let cd = c_driver_path();
    let rd = rust_driver_path();
    assert!(cd.exists(), "C driver missing: {}", cd.display());
    assert!(rd.exists(), "Rust driver missing: {}", rd.display());

    let (co, ce, cs) = run(&cd, args);
    let (ro, re, rs) = run(&rd, args);
    assert_eq!(
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
        "driver stdout differs for args {args:?} ({OP}/{REPEAT})"
    );
    assert_eq!(co, ro, "driver stdout bytes differ for args {args:?}");
    // argv[0] necessarily differs, so compare stderr only after normalising it.
    let norm = |b: &[u8], prog: &Path| -> String {
        String::from_utf8_lossy(b).replace(&prog.display().to_string(), "<PROG>")
    };
    assert_eq!(
        norm(&ce, &cd),
        norm(&re, &rd),
        "driver stderr differs for args {args:?}"
    );
    assert_eq!(cs, rs, "driver exit status differs for args {args:?}");
}

#[test]
fn cfg_c46_driver_plain_numbers() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let a = rng.next_mixed().to_string();
        let b = rng.next_mixed().to_string();
        diff_driver(&[&a, &b]);
    }
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            diff_driver(&[&a.to_string(), &b.to_string()]);
        }
    }
}

#[test]
fn cfg_c47_driver_extra_args_ignored() {
    for extra in [
        vec!["3", "4", "5"],
        vec!["3", "4", "5", "6"],
        vec!["-1", "-2", "ignored", ""],
    ] {
        diff_driver(&extra);
    }
}

#[test]
fn cfg_c48_driver_atoi_shapes() {
    let shapes = [
        " 12", "+12", "-0", "12abc", "abc", "", "0", "-1", "007", "\t-9", "\n5", "1 2", "0x10",
        "2147483647", "2147483648", "-2147483648", "-2147483649", "4294967296",
        "9223372036854775807", "9223372036854775808", "99999999999999999999",
        "-99999999999999999999", "--5", "+-5", ".5", "5.9",
    ];
    for s in shapes {
        diff_driver(&[s, "3"]);
        diff_driver(&["3", s]);
        diff_driver(&[s, s]);
    }
}

/* ===================================================================== */
/* Phase C — error-path differential tests (one per ERRORS.md row)         */
/* ===================================================================== */

#[test]
fn err_e1_argc_1() {
    diff_driver(&[]);
    let (o, _e, s) = run(&c_driver_path(), &[]);
    assert_eq!(s, Some(2), "C must exit 2 on argc<3");
    assert!(o.is_empty(), "C must print nothing on stdout for argc<3");
}

#[test]
fn err_e2_argc_2() {
    diff_driver(&["5"]);
    diff_driver(&["abc"]);
    diff_driver(&[""]);
    let (_o, _e, s) = run(&rust_driver_path(), &["5"]);
    assert_eq!(s, Some(2), "Rust must exit 2 on argc<3");
}

#[test]
fn err_e3_atoi_nonnumeric() {
    // NB: an embedded NUL cannot be passed through `execve`, so it is not a
    // representable argv value on either side.
    for s in ["abc", "", "+", "-", "x9", "  ", "?", "\u{7f}"] {
        diff_driver(&[s, "1"]);
    }
}

#[test]
fn err_e4_atoi_overflow() {
    for s in [
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775808",
        "-9223372036854775809",
        "111111111111111111111111111111",
    ] {
        diff_driver(&[s, s]);
        diff_driver(&[s, "1"]);
    }
}

#[test]
fn err_e5_atoi_int_trunc() {
    for s in [
        "2147483648",
        "-2147483649",
        "4294967296",
        "4294967295",
        "8589934592",
        "9223372036854775807",
        "-9223372036854775808",
    ] {
        diff_driver(&[s, "2"]);
    }
}

#[test]
fn err_e6_atoi_partial() {
    for s in [" 12", "+12", "12abc", "1 2", "\t\n\r\u{b}\u{c}42", "12.7", "-  3"] {
        diff_driver(&[s, s]);
    }
}

#[test]
fn err_e7_use_generated_7() {
    let v = diff_un(b"use_generated\0", &[7]);
    assert_eq!(
        v[0],
        init_expected(),
        "use_generated(7) must fall into DISPATCH_REP's default (no case 7)"
    );
}

#[test]
fn err_e8_use_generated_above() {
    let v = diff_un(b"use_generated\0", &[8, 9, 100, 1000, c_int::MAX, c_int::MAX - 1]);
    for x in v {
        assert_eq!(x, init_expected(), "n>7 must return INIT_FOR(OP)");
    }
}

#[test]
fn err_e9_use_generated_negative() {
    let v = diff_un(b"use_generated\0", &[-1, -2, -7, -100, c_int::MIN, c_int::MIN + 1]);
    for x in v {
        assert_eq!(x, init_expected(), "n<0 must return INIT_FOR(OP)");
    }
}

#[test]
fn err_e10_use_generated_boundary() {
    let v = diff_un(b"use_generated\0", &[-1, 0, 6, 7]);
    assert_eq!(v[0], init_expected(), "n=-1 -> default");
    assert_eq!(v[1], init_expected(), "n=0 -> REP0, acc untouched");
    assert_eq!(v[3], init_expected(), "n=7 -> default (no case 7)");
    let mut acc = init_expected();
    for i in 0..6 {
        acc = match OP {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i + 1),
        };
    }
    assert_eq!(v[2], acc, "n=6 -> REP6");
}

#[test]
fn err_e11_op_add_overflow() {
    diff_bin(
        b"op_add\0",
        &[
            (c_int::MAX, 1),
            (1, c_int::MAX),
            (c_int::MIN, -1),
            (-1, c_int::MIN),
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, c_int::MIN),
            (c_int::MAX, c_int::MIN),
        ],
    );
}

#[test]
fn err_e12_op_sub_overflow() {
    diff_bin(
        b"op_sub\0",
        &[
            (c_int::MIN, 1),
            (c_int::MAX, c_int::MIN),
            (0, c_int::MIN),
            (c_int::MIN, c_int::MAX),
            (-1, c_int::MAX),
        ],
    );
}

#[test]
fn err_e13_op_mul_overflow() {
    diff_bin(
        b"op_mul\0",
        &[
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, -1),
            (-1, c_int::MIN),
            (c_int::MIN, c_int::MIN),
            (65536, 65536),
            (46341, 46341),
            (c_int::MIN, 2),
        ],
    );
}

#[test]
fn err_e14_helper_call_overflow() {
    diff_bin(
        b"helper_call\0",
        &[
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, c_int::MIN),
            (c_int::MAX, c_int::MIN),
            (c_int::MIN, -1),
            (c_int::MAX, 1),
            (65536, 65536),
        ],
    );
}

#[test]
fn err_e15_helper_ptr_overflow() {
    diff_bin(
        b"helper_ptr\0",
        &[
            (c_int::MAX, c_int::MAX),
            (c_int::MIN, c_int::MIN),
            (c_int::MIN, -1),
            (c_int::MAX, 1),
        ],
    );
}

#[test]
fn err_e16_g_op_overflow_and_identity() {
    let p = Pair::load();
    let g_c: Symbol<FnPtrVar> = unsafe { p.c.get(b"G_OP\0") }.expect("C G_OP");
    let g_r: Symbol<FnPtrVar> = unsafe { p.r.get(b"G_OP\0") }.expect("Rust G_OP");
    let fc: BinFn = unsafe { **g_c };
    let fr: BinFn = unsafe { **g_r };
    assert_ne!(fc as usize, 0, "C G_OP is NULL");
    assert_ne!(fr as usize, 0, "Rust G_OP is NULL");
    for (a, b) in [
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, -1),
        (c_int::MAX, 1),
        (0, 0),
    ] {
        assert_eq!(unsafe { fc(a, b) }, unsafe { fr(a, b) }, "G_OP diverged at ({a},{b})");
    }
}

#[test]
fn err_e17_g_op_name_not_null() {
    let p = Pair::load();
    let n_c: Symbol<CharPtrVar> = unsafe { p.c.get(b"G_OP_NAME\0") }.expect("C G_OP_NAME");
    let n_r: Symbol<CharPtrVar> = unsafe { p.r.get(b"G_OP_NAME\0") }.expect("Rust G_OP_NAME");
    let pc = unsafe { **n_c };
    let pr = unsafe { **n_r };
    assert!(!pc.is_null() && !pr.is_null());
    let bc = unsafe { CStr::from_ptr(pc) }.to_bytes_with_nul();
    let br = unsafe { CStr::from_ptr(pr) }.to_bytes_with_nul();
    assert_eq!(bc, br, "G_OP_NAME (incl. NUL) differs");
}

#[test]
fn err_e18_full_int_boundary_sweep() {
    let sweep: [c_int; 9] = [
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut pairs = Vec::new();
    for &a in sweep.iter() {
        for &b in sweep.iter() {
            pairs.push((a, b));
        }
    }
    for name in [
        &b"op_add\0"[..],
        &b"op_sub\0"[..],
        &b"op_mul\0"[..],
        &b"helper_call\0"[..],
        &b"helper_ptr\0"[..],
    ] {
        diff_bin(name, &pairs);
    }
    diff_un(b"use_generated\0", &sweep);
}

#[test]
fn err_e20_argc_extra() {
    // argc > 3: identical to argc == 3 with the same first two args.
    let cd = c_driver_path();
    let (o3, _, s3) = run(&cd, &["4", "5"]);
    let (o5, _, s5) = run(&cd, &["4", "5", "6", "7"]);
    assert_eq!(o3, o5, "C: extra args must be ignored");
    assert_eq!(s3, s5);
    diff_driver(&["4", "5", "6", "7"]);
}

/* ===================================================================== */
/* Phase D — symbol parity, checked from inside the test suite too         */
/* ===================================================================== */

#[test]
fn symbols_parity_nm() {
    fn dyn_syms(p: &Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = dyn_syms(&c_so_path());
    let r = dyn_syms(&rust_so_path());
    assert!(!c.is_empty(), "no symbols read from the C .so");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The eight documented public symbols must be present on both sides.
    for want in [
        "op_add",
        "op_sub",
        "op_mul",
        "helper_call",
        "helper_ptr",
        "use_generated",
        "G_OP",
        "G_OP_NAME",
    ] {
        assert!(c.contains(&want.to_string()), "C .so lacks {want}");
        assert!(r.contains(&want.to_string()), "Rust .so lacks {want}");
    }

    // `accum_<OP>` is `static` in C -> must NOT be a dynamic symbol anywhere.
    let accum = format!("accum_{OP}");
    assert!(!c.contains(&accum), "C .so unexpectedly exports {accum}");
    assert!(!r.contains(&accum), "Rust .so must not export {accum}");
}

#[test]
fn no_undefined_non_libc_symbols_in_rust_so() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let txt = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = txt
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|s| {
            // Anything that looks like one of *our* symbols must not be undefined.
            matches!(
                *s,
                "op_add"
                    | "op_sub"
                    | "op_mul"
                    | "helper_call"
                    | "helper_ptr"
                    | "use_generated"
                    | "G_OP"
                    | "G_OP_NAME"
            )
        })
        .collect();
    assert!(bad.is_empty(), "Rust .so has undefined project symbols: {bad:?}");
}
