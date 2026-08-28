//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Everything here is derived from `nm -D` at test time, so `SYMBOLS.md` cannot
//! silently drift away from reality.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

fn defined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

fn undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], so).into_iter().collect()
}

/// libc / libgcc / toolchain-glue symbols the Rust `std` runtime legitimately
/// imports. Anything outside this set would mean part of the translated library
/// is missing.
const ALLOWED_UNDEF_PREFIXES: &[&str] = &[
    "_ITM_",
    "__cxa_",
    "__gmon_",
    "_Unwind_",
    "__libc_",
    "__pthread_",
    "pthread_",
    "__tls_get_addr",
    "__errno_location",
    "__rust_probestack",
];

const ALLOWED_UNDEF_EXACT: &[&str] = &[
    "abort",
    "bcmp",
    "calloc",
    "close",
    "dl_iterate_phdr",
    "free",
    "fstat",
    "fstat64",
    "getcwd",
    "getenv",
    "gettid",
    "lseek",
    "lseek64",
    "malloc",
    "memcmp",
    "memcpy",
    "memmove",
    "memset",
    "mmap",
    "mmap64",
    "munmap",
    "open",
    "open64",
    "posix_memalign",
    "read",
    "readlink",
    "realloc",
    "realpath",
    "sigaction",
    "sigaltstack",
    "stat",
    "stat64",
    "statx",
    "strlen",
    "syscall",
    "sysconf",
    "write",
    "writev",
    "__errno_location",
];

fn is_allowed_undef(s: &str) -> bool {
    ALLOWED_UNDEF_EXACT.contains(&s)
        || ALLOWED_UNDEF_PREFIXES.iter().any(|p| s.starts_with(p))
}

/// SYMBOLS.md gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name. The diff must be EMPTY.
#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();

    let c_def = defined(&c);
    let r_def = defined(&r);

    let missing: Vec<&String> = c_def.difference(&r_def).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_def:?}\n\
         Rust({}): {r_def:?}",
        missing.len(),
        c.display(),
        r.display(),
    );

    // Sanity: the one documented entry point really is there.
    assert!(
        c_def.contains("to_barycentric"),
        "C .so must export to_barycentric, got {c_def:?}"
    );
    assert!(r_def.contains("to_barycentric"));
    // And exactly one public symbol, as SYMBOLS.md records.
    assert_eq!(c_def.len(), 1, "C .so defined dynsyms: {c_def:?}");
}

/// The three `static` helpers have internal linkage in C. Exporting them from
/// Rust would be a divergence, so assert neither library does.
#[test]
fn d2_static_helpers_are_not_exported_by_either_library() {
    let c_def = defined(&c_so());
    let r_def = defined(&rust_so());
    for name in ["lm_v2", "lm_sub2", "lm_dot2"] {
        assert!(!c_def.contains(name), "unexpected: C exports {name}");
        assert!(
            !r_def.contains(name),
            "Rust exports {name}, but the C declares it `static` (internal linkage)"
        );
    }
}

/// No dangling non-libc symbol in the Rust `.so`: if a module had been left
/// untranslated but referenced, it would surface here.
#[test]
fn d3_rust_so_has_no_non_libc_undefined_symbols() {
    let undef = undefined(&rust_so());
    let bad: Vec<&String> = undef.iter().filter(|s| !is_allowed_undef(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has {} undefined non-libc symbol(s): {bad:?}",
        bad.len()
    );
}

/// Completeness: `c_src/CMakeLists.txt` compiles `src/lib.c` and nothing else,
/// and there is no other C source in the tree. Guards against a whole module
/// having been skipped by the translation step.
#[test]
fn d4_all_c_sources_are_accounted_for() {
    let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src");

    let mut found: Vec<String> = Vec::new();
    let mut stack = vec![c_src.clone()];
    while let Some(dir) = stack.pop() {
        // Skip the cmake build tree.
        if dir.file_name().and_then(|n| n.to_str()) == Some("build") {
            continue;
        }
        for e in std::fs::read_dir(&dir).expect("read_dir c_src").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ext == "c" || ext == "h" {
                    found.push(
                        p.strip_prefix(&c_src).unwrap().to_string_lossy().into_owned(),
                    );
                }
            }
        }
    }
    found.sort();
    assert_eq!(
        found,
        vec!["include/lib.h".to_string(), "src/lib.c".to_string()],
        "the set of C sources changed; SYMBOLS.md's completeness argument must be redone"
    );
}

/// The loaded Rust symbol is genuinely the `#[no_mangle] extern "C"` export
/// reached through `dlsym`, and it agrees with C on a trivial input. (Smoke
/// test for the harness itself.)
#[test]
fn d5_harness_loads_both_exports_via_dlsym() {
    let l = libs();
    assert_ne!(l.c as usize, l.rust as usize, "must be two distinct symbols");
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(1.0, 0.0);
    let c = Vec2::new(0.0, 1.0);
    let p = Vec2::new(0.25, 0.25);
    let got = diff_get("d5", a, b, c, p);
    assert_eq!(got.bits(), (0x3E80_0000, 0x3E80_0000), "got {got:?}");
}

/// Records which two `.so` files the harness actually loaded, so a run against
/// a stale or wrong artifact is visible in the log rather than silent. Also
/// proves the `RUST_SO_PATH` / `C_SO_PATH` overrides used by
/// `check_all_features.sh` are honoured.
#[test]
fn d6_reports_loaded_libraries() {
    let c = c_so();
    let r = rust_so();
    println!("C   .so: {}", c.display());
    println!("Rust.so: {}", r.display());
    assert!(c.is_file() && r.is_file());
    assert_ne!(c, r);
    if let Ok(want) = std::env::var("RUST_SO_PATH") {
        assert_eq!(
            r,
            std::path::PathBuf::from(&want),
            "RUST_SO_PATH override was ignored by the harness"
        );
    }
    if let Ok(want) = std::env::var("C_SO_PATH") {
        assert_eq!(c, std::path::PathBuf::from(&want));
    }
}
