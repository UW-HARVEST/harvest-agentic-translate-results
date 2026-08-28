//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! matching function pointers for every symbol the C library exports.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

pub type FnSafeDoubleToInt = unsafe extern "C" fn(f64) -> c_int;
pub type FnProcessArrayReverse = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnSwitchFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnAllocateAndCompute = unsafe extern "C" fn(c_int, f64) -> c_int;
pub type FnForeachSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnFallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Every exported entry point of one implementation.
pub struct Impl {
    _lib: Library,
    #[allow(dead_code)]
    pub name: &'static str,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_array_reverse: FnProcessArrayReverse,
    pub switch_fallthrough_calculator: FnSwitchFallthrough,
    pub allocate_and_compute: FnAllocateAndCompute,
    pub foreach_sum: FnForeachSum,
    pub fallcalc: FnFallcalc,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

        unsafe fn get<T: Copy>(lib: &Library, sym: &[u8]) -> T {
            let s: Symbol<T> = unsafe { lib.get(sym) }.unwrap_or_else(|e| {
                panic!("missing symbol {}: {e}", String::from_utf8_lossy(sym))
            });
            *s
        }

        unsafe {
            Impl {
                name,
                safe_double_to_int: get(&lib, b"safe_double_to_int\0"),
                process_array_reverse: get(&lib, b"process_array_reverse\0"),
                switch_fallthrough_calculator: get(&lib, b"switch_fallthrough_calculator\0"),
                allocate_and_compute: get(&lib, b"allocate_and_compute\0"),
                foreach_sum: get(&lib, b"foreach_sum\0"),
                fallcalc: get(&lib, b"fallcalc\0"),
                _lib: lib,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut hits: Vec<PathBuf> = Vec::new();
    let mut stack = vec![build.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "so") {
                hits.push(p);
            }
        }
    }
    hits.sort();
    assert!(
        !hits.is_empty(),
        "no .so found under {}; build the C library first",
        build.display()
    );
    hits.remove(0)
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib built for the same profile is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<bin>");
    let candidate = profile_dir.join("libfallcalc_lib.so");
    let so = if candidate.exists() {
        candidate
    } else {
        let mut found = None;
        for p in ["target/release", "target/debug"] {
            let c = workspace_root()
                .join("translation")
                .join(p)
                .join("libfallcalc_lib.so");
            if c.exists() {
                found = Some(c);
                break;
            }
        }
        found.unwrap_or_else(|| {
            panic!(
                "libfallcalc_lib.so not found near {}; run `cargo build` first",
                profile_dir.display()
            )
        })
    };
    assert_not_stale(&so);
    so
}

/// `cargo test` does **not** rebuild a `cdylib` that no test target links
/// against, so the `.so` on disk can lag behind `src/`. Without this guard the
/// whole suite would silently validate a stale library.
fn assert_not_stale(so: &Path) {
    fn mtime(p: &Path) -> Option<std::time::SystemTime> {
        std::fs::metadata(p).ok()?.modified().ok()
    }

    let so_time = match mtime(so) {
        Some(t) => t,
        None => return,
    };

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut consider = |p: PathBuf| {
        if let Some(t) = mtime(&p) {
            if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                newest = Some((p, t));
            }
        }
    };
    consider(manifest.join("Cargo.toml"));

    let mut stack = vec![manifest.join("src")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                consider(p);
            }
        }
    }

    if let Some((src, src_time)) = newest {
        assert!(
            src_time <= so_time,
            "stale Rust library: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib; run `cargo build` (same \
             profile/features) before `cargo test`.",
            so.display(),
            src.display()
        );
    }
}


pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Loads both libraries. Both are loaded through `libloading`, so the Rust side
/// is exercised strictly through its `#[no_mangle]` exports.
pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("c", &find_c_so()),
        rs: Impl::load("rust", &find_rust_so()),
    }
}

/// Bit-exact comparison of two `int` results.
#[track_caller]
pub fn assert_int_eq(ctx: &str, c: c_int, rs: c_int) {
    assert_eq!(
        c, rs,
        "mismatch for {ctx}: C = {c} (0x{c:08x}), Rust = {rs} (0x{rs:08x})"
    );
}

/// Path of the C reference shared library.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path of the Rust shared library under test.
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// Names of the defined, global, code/data symbols in a shared object's dynamic
/// symbol table, excluding the toolchain-provided boilerplate that every ELF
/// gets (`_init`, `_fini`, `__bss_start`, ...).
pub fn dynamic_symbols(so: &Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let ignored = [
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__bss_start__",
        "_bss_end__",
        "__bss_end__",
        "__end__",
        "_DYNAMIC",
        "_GLOBAL_OFFSET_TABLE_",
    ];

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only functions and initialised/uninitialised data with external
            // linkage; skip local ('t', 'd', ...) and toolchain symbols.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            if ignored.contains(&name) || name.starts_with("__rust") || name.starts_with("rust_") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}
