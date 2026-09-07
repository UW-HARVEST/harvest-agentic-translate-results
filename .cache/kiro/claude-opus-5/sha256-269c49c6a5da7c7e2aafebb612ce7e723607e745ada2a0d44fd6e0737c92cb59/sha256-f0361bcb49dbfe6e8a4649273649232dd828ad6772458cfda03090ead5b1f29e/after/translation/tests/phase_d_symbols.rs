//! Phase D — symbol parity enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! with the exact same name, and the Rust `.so` must not import any
//! non-libc/non-runtime symbol.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("no .so in c_src/build")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let prof = exe.parent().unwrap().parent().unwrap();
    let c = prof.join("libgen_ray_lib.so");
    assert!(c.exists(), "{} does not exist", c.display());
    c
}

fn nm(path: &PathBuf, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    let c_syms = nm(&c, &["-D", "--defined-only"]);
    let r_syms = nm(&r, &["-D", "--defined-only"]);
    eprintln!("C   .so: {} ({} exported symbols)", c.display(), c_syms.len());
    eprintln!("Rust.so: {} ({} exported symbols)", r.display(), r_syms.len());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {:?}",
        missing.len(),
        missing
    );

    // The full expected set, transcribed from SYMBOLS.md.
    let expected = [
        "c2AABBtoAABB",
        "c2AABBtoPoint",
        "c2Absv",
        "c2Add",
        "c2CCW90",
        "c2CastRay",
        "c2CircleToPoint",
        "c2Div",
        "c2Dot",
        "c2Len",
        "c2Maxv",
        "c2Minv",
        "c2MulmvT",
        "c2Mulvs",
        "c2Norm",
        "c2RaytoAABB",
        "c2RaytoCapsule",
        "c2RaytoCircle",
        "c2Skew",
        "c2Sub",
        "c2V",
        "gen_ray",
    ];
    for e in expected {
        assert!(c_syms.iter().any(|s| s == e), "C .so lost symbol {e}");
        assert!(r_syms.iter().any(|s| s == e), "Rust .so lost symbol {e}");
    }
    assert_eq!(
        c_syms.len(),
        expected.len(),
        "the C .so's symbol set changed; SYMBOLS.md needs regenerating: {c_syms:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_runtime_symbols() {
    let r = rust_so();
    let und = nm(&r, &["-D", "--undefined-only"]);
    // Everything the Rust runtime legitimately imports from libc / libgcc.
    let allowed_prefix = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
        "__libc_", "__rust_",
    ];
    let allowed_libc = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "pthread_mutex_lock", "pthread_mutex_unlock", "read",
        "readlink", "realloc", "realpath", "sqrt", "sqrtf", "stat", "stat64", "statx",
        "strlen", "syscall", "write", "writev", "sysconf", "getrandom", "poll", "sigaction",
        "sigaltstack", "mprotect", "pthread_self", "pthread_getattr_np",
        "pthread_attr_getstack", "pthread_attr_destroy",
    ];
    let mut bad = Vec::new();
    for s in &und {
        let base = s.split('@').next().unwrap();
        if allowed_prefix.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_libc.contains(&base) {
            continue;
        }
        bad.push(s.clone());
    }
    assert!(
        bad.is_empty(),
        "Rust .so has unexpected undefined symbols: {bad:?}"
    );
}

#[test]
fn project_builds_no_binary_executable() {
    // The CMake project only declares `add_library(... SHARED)`, and the crate
    // declares only a cdylib, so there is no driver binary whose stdout would
    // need comparing. Assert that stays true.
    let cml = std::fs::read_to_string(root().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cml.to_lowercase().contains("add_executable"),
        "c_src now builds an executable — Phase B needs a stdout comparison"
    );
    let toml = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[[bin]]"),
        "the crate now declares a binary — Phase B needs a stdout comparison"
    );
    assert!(
        !root().join("translation/src/main.rs").exists(),
        "src/main.rs appeared — Phase B needs a stdout comparison"
    );
}

#[test]
fn cargo_toml_declares_no_features() {
    let toml = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml now declares features; CONFIGS.md's combination list and \
         run_all.sh must be extended to cover them"
    );
}
