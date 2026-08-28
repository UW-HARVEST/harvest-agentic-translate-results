//! Shared helpers for the differential C-vs-Rust integration tests.
//!
//! Everything is exercised strictly through `libloading`, i.e. through the
//! dynamic-symbol table of the two shared objects, so the `#[no_mangle]`
//! export wrappers are part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// `<workspace>/translation`
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory that holds both `c_src/` and `translation/`.
pub fn root_dir() -> PathBuf {
    crate_dir().parent().unwrap().to_path_buf()
}

pub fn c_src_dir() -> PathBuf {
    root_dir().join("c_src")
}

/// Scratch space for the artefacts the tests compile themselves.
pub fn scratch_dir() -> PathBuf {
    let d = crate_dir().join("target").join("difftest");
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

fn run(cmd: &mut Command) -> String {
    let rendered = format!("{cmd:?}");
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {rendered}: {e}"));
    if !out.status.success() {
        panic!(
            "command failed: {rendered}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A unique sibling path, so concurrent builders never write the same file.
fn temp_sibling(out: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!(
        ".tmp-{}-{}-{}",
        std::process::id(),
        stamp,
        out.file_name().unwrap().to_string_lossy()
    );
    out.with_file_name(name)
}

/// Build into a scratch path and `rename` it over `out`.
///
/// `cargo test` may run the test binaries concurrently and each of them wants
/// the same helper libraries; `rename` is atomic on POSIX, so a reader either
/// sees the previous complete file or the new complete file, never a partial
/// one.
fn build_atomically(out: &Path, build: impl FnOnce(&Path)) {
    let tmp = temp_sibling(out);
    build(&tmp);
    std::fs::rename(&tmp, out).unwrap_or_else(|e| {
        panic!(
            "rename {} -> {}: {e}",
            tmp.display(),
            out.display()
        )
    });
}

fn c_compiler() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

/// Extra flags for the C helper libraries, e.g. `DIFFTEST_CFLAGS="-O2"`.
///
/// `c_src/CMakeLists.txt` pins no optimisation level, so the level is a
/// genuine build-time degree of freedom. It matters here because
/// `call_predict` compares *function addresses*: an optimiser that folds two
/// identical function bodies together would change its result.
fn extra_cflags() -> Vec<String> {
    std::env::var("DIFFTEST_CFLAGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Extra flags for the Rust helper library, e.g. `DIFFTEST_RUSTFLAGS="-O"`.
fn extra_rustflags() -> Vec<String> {
    std::env::var("DIFFTEST_RUSTFLAGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Distinguishes artefacts built with different flag sets.
fn flavour() -> String {
    let key = format!("{:?}{:?}", extra_cflags(), extra_rustflags());
    if key == "[][]" {
        return String::new();
    }
    // Short, filesystem-safe digest (FNV-1a).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("-{h:016x}")
}

/// Build (once) the plain shared library from the untouched `c_src` tree.
///
/// CMake is not used here so that the tests never write into `c_src/`; the
/// compile flags mirror `c_src/CMakeLists.txt` (a bare `SHARED` library with
/// `include/` and `src/` on the include path, no explicit optimisation level).
pub fn c_shared_lib() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let out = scratch_dir().join(format!("libc_reference{}.so", flavour()));
        let src = c_src_dir().join("src").join("lib.c");
        if needs_rebuild(&out, &[&src]) {
            build_atomically(&out, |tmp| {
                run(Command::new(c_compiler())
                    .arg("-shared")
                    .arg("-fPIC")
                    .arg(format!("-I{}", c_src_dir().join("include").display()))
                    .arg(format!("-I{}", c_src_dir().join("src").display()))
                    .args(extra_cflags())
                    .arg(&src)
                    .arg("-o")
                    .arg(tmp));
            });
        }
        out
    })
    .clone()
}

/// The `cdylib` cargo just built for us (tests always run after the lib).
pub fn rust_shared_lib() -> PathBuf {
    // `CARGO_MANIFEST_DIR/target/<profile>/libcall_predict_lib.so`; derive the
    // profile directory from the test executable's own location so that this
    // works for `cargo test` and `cargo test --release` alike.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf();
    let candidate = profile_dir.join("libcall_predict_lib.so");
    assert!(
        candidate.is_file(),
        "rust cdylib not found at {}",
        candidate.display()
    );
    candidate
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

fn needs_rebuild(out: &Path, deps: &[&Path]) -> bool {
    let Some(out_t) = mtime(out) else { return true };
    deps.iter().any(|d| match mtime(d) {
        Some(t) => t > out_t,
        None => true,
    })
}

// ---------------------------------------------------------------------------
// Harness libraries
//
// `call_predict` is the only symbol either library exports, yet the twelve
// `BTAC1C2_PredictSample_PfnN` bodies plus the big `BTAC1C2_PredictSample`
// switch carry all the arithmetic. To compare that arithmetic without editing
// either source tree, each side is recompiled here into an *additional*
// shared object that re-exports the internals via a `harness_*` shim:
//
//   * C: a shim `.c` file `#include`s `c_src/src/lib.c`, which puts the shim
//     in the same translation unit as the `static` functions.
//   * Rust: `src/lib.rs` is copied verbatim (inner attributes moved to the
//     rustc command line) and the shim is appended, putting the shim in the
//     same module as the private functions.
//
// Neither `c_src/` nor the shipped crate is modified, and the real
// `libcall_predict_lib.so` export surface stays byte-identical to C's.
// ---------------------------------------------------------------------------

const C_SHIM: &str = r#"
#include <stddef.h>
#include "lib.c"   /* resolved through -I<c_src>/src */

typedef int (*harness_predfn)(int *, int, int, btac1c_idxstate *);

int harness_call(int sel, int *psamp, int idx, int pfcn, void *ridx) {
    harness_predfn f = (harness_predfn)BTAC1C2_GetPredictFunc(sel);
    return f(psamp, idx, pfcn, (btac1c_idxstate *)ridx);
}

int harness_switch(int *psamp, int idx, int pfcn, void *ridx) {
    return BTAC1C2_PredictSample(psamp, idx, pfcn, (btac1c_idxstate *)ridx);
}

int harness_same_fn(int a, int b) {
    return BTAC1C2_GetPredictFunc(a) == BTAC1C2_GetPredictFunc(b);
}

int harness_sizeof_idxstate(void) { return (int)sizeof(btac1c_idxstate); }
int harness_alignof_idxstate(void) { return (int)_Alignof(btac1c_idxstate); }
int harness_offset_idx(void)   { return (int)offsetof(btac1c_idxstate, idx); }
int harness_offset_lpred(void) { return (int)offsetof(btac1c_idxstate, lpred); }
int harness_offset_rpred(void) { return (int)offsetof(btac1c_idxstate, rpred); }
int harness_offset_tag(void)   { return (int)offsetof(btac1c_idxstate, tag); }
int harness_offset_bcfcn(void) { return (int)offsetof(btac1c_idxstate, bcfcn); }
int harness_offset_bsfcn(void) { return (int)offsetof(btac1c_idxstate, bsfcn); }
int harness_offset_usefx(void) { return (int)offsetof(btac1c_idxstate, usefx); }
int harness_offset_firfx(void) { return (int)offsetof(btac1c_idxstate, firfx); }
"#;

const RUST_SHIM: &str = r#"
// ----- appended test shim (not part of the shipped crate) -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn harness_call(
    sel: c_int,
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let raw = BTAC1C2_GetPredictFunc(sel);
        let f: PredictFn = std::mem::transmute(raw);
        f(psamp, idx, pfcn, ridx)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn harness_switch(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { BTAC1C2_PredictSample(psamp, idx, pfcn, ridx) }
}

#[unsafe(no_mangle)]
pub extern "C" fn harness_same_fn(a: c_int, b: c_int) -> c_int {
    (BTAC1C2_GetPredictFunc(a) == BTAC1C2_GetPredictFunc(b)) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn harness_sizeof_idxstate() -> c_int {
    std::mem::size_of::<btac1c_idxstate>() as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_alignof_idxstate() -> c_int {
    std::mem::align_of::<btac1c_idxstate>() as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_idx() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, idx) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_lpred() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, lpred) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_rpred() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, rpred) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_tag() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, tag) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_bcfcn() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, bcfcn) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_bsfcn() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, bsfcn) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_usefx() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, usefx) as c_int
}
#[unsafe(no_mangle)]
pub extern "C" fn harness_offset_firfx() -> c_int {
    std::mem::offset_of!(btac1c_idxstate, firfx) as c_int
}
"#;

/// C shared object that additionally exports the `harness_*` shims.
pub fn c_harness_lib() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = scratch_dir();
        let shim = dir.join("c_harness.c");
        let out = dir.join(format!("libc_harness{}.so", flavour()));
        let src = c_src_dir().join("src").join("lib.c");
        write_if_changed(&shim, C_SHIM);
        if needs_rebuild(&out, &[&src, &shim]) {
            build_atomically(&out, |tmp| {
                run(Command::new(c_compiler())
                    .arg("-shared")
                    .arg("-fPIC")
                    .arg(format!("-I{}", c_src_dir().join("include").display()))
                    .arg(format!("-I{}", c_src_dir().join("src").display()))
                    .args(extra_cflags())
                    .arg(&shim)
                    .arg("-o")
                    .arg(tmp));
            });
        }
        out
    })
    .clone()
}

/// Rust shared object that additionally exports the `harness_*` shims.
pub fn rust_harness_lib() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = scratch_dir();
        let lib_rs = crate_dir().join("src").join("lib.rs");
        let generated = dir.join("rust_harness.rs");
        let out = dir.join(format!("librust_harness{}.so", flavour()));

        let original = std::fs::read_to_string(&lib_rs).expect("read src/lib.rs");
        // Inner attributes / inner doc comments may only appear at the very top
        // of a file, so strip them here and re-apply the lints on the command
        // line. Nothing else about `lib.rs` is altered.
        let body: String = original
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//!") && !t.starts_with("#![")
            })
            .map(|l| format!("{l}\n"))
            .collect();
        let generated_src = format!("{body}\n{RUST_SHIM}");
        write_if_changed(&generated, &generated_src);

        if needs_rebuild(&out, &[&generated]) {
            build_atomically(&out, |tmp| {
                run(Command::new("rustc")
                    .arg("--edition")
                    .arg("2024")
                    .arg("--crate-type")
                    .arg("cdylib")
                    .arg("--crate-name")
                    .arg("rust_harness")
                    .args(["-A", "non_snake_case"])
                    .args(["-A", "non_camel_case_types"])
                    .args(["-A", "unused_variables"])
                    .args(["-A", "dead_code"])
                    .args(["-A", "unused_unsafe"])
                    .args(extra_rustflags())
                    .arg(&generated)
                    .arg("-o")
                    .arg(tmp));
            });
        }
        out
    })
    .clone()
}

fn write_if_changed(path: &Path, contents: &str) {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() != Some(contents) {
        // Atomic, so a concurrently-running test binary never observes a
        // half-written shim.
        let tmp = temp_sibling(path);
        std::fs::write(&tmp, contents).expect("write generated file");
        std::fs::rename(&tmp, path).expect("install generated file");
    }
}

/// `nm -D --defined-only` names of a shared object, sorted and deduplicated.
pub fn exported_symbols(lib: &Path) -> Vec<String> {
    let out = run(Command::new("nm").arg("-D").arg("--defined-only").arg(lib));
    let mut names: Vec<String> = out
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}
