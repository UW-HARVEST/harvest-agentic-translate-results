//! Cross-language parity tests.
//!
//! Both implementations are exercised strictly through their shared objects:
//! the C reference (`mdcore.c` compiled with the matching `-DOP` / `-DREPEAT`)
//! and this crate's `cdylib`, both loaded with `libloading`. No Rust function is
//! called directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! Run per configuration, e.g.
//! `cargo test --no-default-features --features mul,3`.

use std::ffi::{CStr, OsStr};
use std::io::Write;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------- config ----
//
// The feature cascade below mirrors `mdmacros.rs` exactly: for OP
// mul > sub > add with add as the `#ifndef OP` fallback, and for REPEAT the
// lowest enabled value wins with 5 as the `#ifndef REPEAT` fallback.

#[cfg(feature = "mul")]
const OP: &str = "mul";
#[cfg(all(not(feature = "mul"), feature = "sub"))]
const OP: &str = "sub";
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
const OP: &str = "add";

#[cfg(feature = "0")]
const REPEAT: c_int = 0;
#[cfg(all(not(feature = "0"), feature = "1"))]
const REPEAT: c_int = 1;
#[cfg(all(not(feature = "0"), not(feature = "1"), feature = "2"))]
const REPEAT: c_int = 2;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    feature = "3"
))]
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

// ------------------------------------------------------------- libraries ----

fn target_dir() -> PathBuf {
    // .../target/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// The exact set of features enabled for this test binary, so the `cdylib` we
/// load is built from the identical configuration.
fn enabled_features() -> Vec<&'static str> {
    let all: [(&str, bool); 22] = [
        ("add", cfg!(feature = "add")),
        ("sub", cfg!(feature = "sub")),
        ("mul", cfg!(feature = "mul")),
        ("0", cfg!(feature = "0")),
        ("1", cfg!(feature = "1")),
        ("2", cfg!(feature = "2")),
        ("3", cfg!(feature = "3")),
        ("4", cfg!(feature = "4")),
        ("5", cfg!(feature = "5")),
        ("6", cfg!(feature = "6")),
        ("7", cfg!(feature = "7")),
        ("op_add", cfg!(feature = "op_add")),
        ("op_sub", cfg!(feature = "op_sub")),
        ("op_mul", cfg!(feature = "op_mul")),
        ("repeat_0", cfg!(feature = "repeat_0")),
        ("repeat_1", cfg!(feature = "repeat_1")),
        ("repeat_2", cfg!(feature = "repeat_2")),
        ("repeat_3", cfg!(feature = "repeat_3")),
        ("repeat_4", cfg!(feature = "repeat_4")),
        ("repeat_5", cfg!(feature = "repeat_5")),
        ("repeat_6", cfg!(feature = "repeat_6")),
        ("repeat_7", cfg!(feature = "repeat_7")),
    ];
    all.into_iter()
        .filter(|(_, on)| *on)
        .map(|(n, _)| n)
        .collect()
}

/// This crate's `cdylib`, built with the same features as this test binary.
///
/// `cargo test` only builds the test harness, not the `cdylib`, so the shared
/// object is produced here with a nested `cargo build --lib` into its own target
/// directory (avoiding any lock contention with the outer invocation).
fn rust_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = target_dir().join("ffi-so");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--no-default-features")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&out_dir);
    let feats = enabled_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }
    let out = cmd.output().expect("run cargo build --lib");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = out_dir.join("debug");
    let direct = dir.join("libmacrodepth_add_5.so");
    if direct.exists() {
        return direct;
    }
    // Fall back to a scan in case the artifact name ever changes.
    let hit = std::fs::read_dir(&dir)
        .expect("read cdylib dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.extension() == Some(OsStr::new("so"))
                && p.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|n| n.starts_with("lib"))
        });
    hit.unwrap_or_else(|| panic!("no cdylib produced in {}", dir.display()))
}

/// Compile `c_src/src/mdcore.c` into a shared object for the active config.
///
/// `c_src/` itself is never written to; the object lands in the cargo target
/// directory.
fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest
        .parent()
        .expect("workspace root")
        .join("c_src/src/mdcore.c");
    assert!(src.exists(), "missing C source at {}", src.display());

    let out_dir = target_dir().join("cref");
    std::fs::create_dir_all(&out_dir).expect("create cref dir");
    let out = out_dir.join(format!("libmd_{OP}_{REPEAT}.so"));

    let status = std::process::Command::new("cc")
        .args(["-O2", "-fPIC", "-shared"])
        .arg(format!("-DOP={OP}"))
        .arg(format!("-DREPEAT={REPEAT}"))
        .arg("-o")
        .arg(&out)
        .arg(&src)
        .status()
        .expect("run cc");
    assert!(status.success(), "compiling the C reference failed");
    out
}

struct Libs {
    c: Library,
    rust: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        // SAFETY: both objects are freshly built from the sources in this repo
        // and only contain the plain C-ABI functions under test.
        let c = unsafe { Library::new(&c_path) }.expect("load C .so");
        let rust = unsafe { Library::new(&rust_path) }.expect("load Rust .so");
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

type BinFn = extern "C" fn(c_int, c_int) -> c_int;
type UnFn = extern "C" fn(c_int) -> c_int;

fn bin_fn<'l>(lib: &'l Library, name: &str) -> Symbol<'l, BinFn> {
    unsafe { lib.get(format!("{name}\0").as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol {name}: {e}"))
}

fn un_fn<'l>(lib: &'l Library, name: &str) -> Symbol<'l, UnFn> {
    unsafe { lib.get(format!("{name}\0").as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol {name}: {e}"))
}

/// Read the `int (*G_OP)(int,int)` data symbol and return the pointee.
fn g_op(lib: &Library) -> BinFn {
    let sym: Symbol<*const BinFn> = unsafe { lib.get(b"G_OP\0") }.expect("symbol G_OP");
    unsafe { std::ptr::read(*sym) }
}

/// Read the `const char *G_OP_NAME` data symbol and return the C string.
fn g_op_name(lib: &Library) -> Vec<u8> {
    let sym: Symbol<*const *const c_char> =
        unsafe { lib.get(b"G_OP_NAME\0") }.expect("symbol G_OP_NAME");
    unsafe { CStr::from_ptr(std::ptr::read(*sym)) }
        .to_bytes()
        .to_vec()
}

// ------------------------------------------------------- stdout capturing ---

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Run `f` with file descriptor 1 redirected into a pipe and return whatever it
/// wrote there. Both implementations print through `stdout`, so this is what
/// makes their textual output comparable byte for byte.
fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe()");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1)");
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();
        assert!(dup2(fds[1], 1) >= 0, "dup2 redirect");

        let value = f();

        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();
        assert!(dup2(saved, 1) >= 0, "dup2 restore");
        close(saved);
        close(fds[1]);

        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(fds[0], buf.as_mut_ptr().cast::<c_void>(), buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fds[0]);
        (value, out)
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Inputs covering zero, both signs, overflow boundaries and the values the
/// shell-level harness uses.
fn pairs() -> Vec<(c_int, c_int)> {
    let interesting = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        9,
        123,
        -456,
        1000,
        65535,
        65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 15,
        1 << 30,
        -(1 << 30),
    ];
    let mut v = Vec::new();
    for &a in &interesting {
        for &b in &interesting {
            v.push((a, b));
        }
    }
    v
}

// ------------------------------------------------------------------ tests ---

/// Step 8: every symbol exported by the C shared object must also be exported,
/// under the same name, by the Rust one.
fn exported_symbols_match() {
    let l = libs();
    let dyn_syms = |p: &Path| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(p)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2).map(str::to_string))
            .collect()
    };

    let c_syms = dyn_syms(&l.c_path);
    let rust_syms = dyn_syms(&l.rust_path);
    assert!(!c_syms.is_empty(), "C .so exported nothing");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\n  C: {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}

/// Lowest level: the three plain operations. They are configuration
/// independent, so all three are checked in every feature combination.
fn op_functions_match() {
    let l = libs();
    for name in ["op_add", "op_sub", "op_mul"] {
        let c = bin_fn(&l.c, name);
        let r = bin_fn(&l.rust, name);
        for (a, b) in pairs() {
            assert_eq!(c(a, b), r(a, b), "{name}({a}, {b})");
        }
    }
}

/// `const char *G_OP_NAME = STR(OP);`
fn g_op_name_matches() {
    let l = libs();
    let c = g_op_name(&l.c);
    let r = g_op_name(&l.rust);
    assert_eq!(c, r, "G_OP_NAME: C {:?} vs Rust {:?}", show(&c), show(&r));
    assert_eq!(show(&c), OP, "G_OP_NAME should be the selected OP");
}

/// `int (*G_OP)(int,int) = OP_FN(OP);` — the pointer must select the same
/// operation in both objects.
fn g_op_pointer_matches() {
    let l = libs();
    let c = g_op(&l.c);
    let r = g_op(&l.rust);
    for (a, b) in pairs() {
        assert_eq!(c(a, b), r(a, b), "G_OP({a}, {b})");
    }
}

/// `helper_ptr` — calls the selected op through a function pointer and prints.
fn helper_ptr_matches() {
    let l = libs();
    let c = bin_fn(&l.c, "helper_ptr");
    let r = bin_fn(&l.rust, "helper_ptr");
    for (a, b) in pairs() {
        let (cv, cout) = capture(|| c(a, b));
        let (rv, rout) = capture(|| r(a, b));
        assert_eq!(cv, rv, "helper_ptr({a}, {b}) return value");
        assert_eq!(
            cout,
            rout,
            "helper_ptr({a}, {b}) stdout: C {:?} vs Rust {:?}",
            show(&cout),
            show(&rout)
        );
    }
}

/// `helper_call` — op result plus the unrolled `RUN_LOOP(OP, acc, REPEAT)`
/// accumulator, both printed and folded into the return value.
fn helper_call_matches() {
    let l = libs();
    let c = bin_fn(&l.c, "helper_call");
    let r = bin_fn(&l.rust, "helper_call");
    for (a, b) in pairs() {
        let (cv, cout) = capture(|| c(a, b));
        let (rv, rout) = capture(|| r(a, b));
        assert_eq!(cv, rv, "helper_call({a}, {b}) return value");
        assert_eq!(
            cout,
            rout,
            "helper_call({a}, {b}) stdout: C {:?} vs Rust {:?}",
            show(&cout),
            show(&rout)
        );
    }
}

/// `use_generated` — the macro-generated `accum_<OP>`, whose `switch` only
/// handles 0..=6 and leaves the accumulator at its seed for anything else.
fn use_generated_matches() {
    let l = libs();
    let c = un_fn(&l.c, "use_generated");
    let r = un_fn(&l.rust, "use_generated");
    let mut ns: Vec<c_int> = (-8..=16).collect();
    ns.extend([
        REPEAT,
        100,
        -100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ]);
    for n in ns {
        let (cv, cout) = capture(|| c(n));
        let (rv, rout) = capture(|| r(n));
        assert_eq!(cv, rv, "use_generated({n}) return value");
        assert_eq!(
            cout,
            rout,
            "use_generated({n}) stdout: C {:?} vs Rust {:?}",
            show(&cout),
            show(&rout)
        );
    }
}

/// The whole `mdmain.c` sequence, driven entirely through the two shared
/// objects: same call order, same folding of the results.
fn main_sequence_matches() {
    let l = libs();
    let run = |lib: &Library, a: c_int, b: c_int| -> (Vec<c_int>, Vec<u8>) {
        let op_fn = g_op(lib); // OP_FN(OP) and G_OP are the same function
        let helper_call = bin_fn(lib, "helper_call");
        let helper_ptr = bin_fn(lib, "helper_ptr");
        let use_generated = un_fn(lib, "use_generated");
        capture(|| {
            let r_call = op_fn(a, b);
            let x1 = helper_call(a, b);
            let x2 = helper_ptr(a, b);
            let x3 = use_generated(REPEAT);
            let g = op_fn(a, b);
            vec![r_call, x1, x2, x3, g]
        })
    };
    for (a, b) in pairs() {
        let (cv, cout) = run(&l.c, a, b);
        let (rv, rout) = run(&l.rust, a, b);
        assert_eq!(cv, rv, "main sequence values for ({a}, {b})");
        assert_eq!(
            cout,
            rout,
            "main sequence stdout for ({a}, {b}): C {:?} vs Rust {:?}",
            show(&cout),
            show(&rout)
        );
    }
}

// ----------------------------------------------------------------- runner ---

/// Checks in call-hierarchy order: the leaf `op_*` functions first, then the
/// globals that select one of them, then the helpers built on top, and finally
/// the full `main` sequence.
fn main() {
    let checks: &[(&str, fn())] = &[
        ("exported_symbols_match", exported_symbols_match),
        ("op_functions_match", op_functions_match),
        ("g_op_name_matches", g_op_name_matches),
        ("g_op_pointer_matches", g_op_pointer_matches),
        ("helper_ptr_matches", helper_ptr_matches),
        ("helper_call_matches", helper_call_matches),
        ("use_generated_matches", use_generated_matches),
        ("main_sequence_matches", main_sequence_matches),
    ];

    println!(
        "config: OP={OP} REPEAT={REPEAT} (features {:?})",
        enabled_features()
    );
    let mut failed = Vec::new();
    for (name, f) in checks {
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push(*name);
            }
        }
        let _ = std::io::stdout().flush();
    }
    if failed.is_empty() {
        println!("\nall {} checks passed", checks.len());
    } else {
        println!("\nFAILURES: {failed:?}");
        std::process::exit(1);
    }
}
