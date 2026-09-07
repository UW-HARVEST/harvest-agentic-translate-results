//! Phase D — symbol parity and configuration-matrix assertions.
//!
//! These checks are executable rather than prose so `SYMBOLS.md`, `CONFIGS.md`
//! and the build can never silently drift apart.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

use common::*;

/// Dynamic symbols DEFINED by a shared object, per `nm -D --defined-only`.
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>" for defined symbols.
        if cols.len() >= 3 {
            set.insert(cols[2].to_string());
        }
    }
    set
}

/// Dynamic symbols UNDEFINED by a shared object (its imports).
fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<type> <name>" (no address) for undefined symbols.
        if let Some(name) = cols.last() {
            if cols.len() >= 2 {
                set.insert(name.to_string());
            }
        }
    }
    set
}

/// SYMBOLS.md gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name. The diff must be EMPTY.
#[test]
fn symbol_parity_c_to_rust() {
    let (c_so, rust_so) = paths();
    let c_syms = defined_symbols(&c_so);
    let rust_syms = defined_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        rust_syms
    );

    // The C library's entire public surface, per include/lib.h.
    assert!(
        c_syms.contains("premultiply"),
        "C .so must export premultiply; got {c_syms:?}"
    );
    assert!(
        rust_syms.contains("premultiply"),
        "Rust .so must export premultiply; got {rust_syms:?}"
    );
    // Exactly one public function in the header => exactly one C export.
    assert_eq!(
        c_syms.len(),
        1,
        "C .so export set changed; SYMBOLS.md needs updating: {c_syms:?}"
    );
}

/// SYMBOLS.md gate: the Rust `.so` has no unresolved NON-libc symbol. Everything
/// it imports must be a libc / libgcc-unwind / weak-toolchain symbol.
#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let (_, rust_so) = paths();
    let undef = undefined_symbols(&rust_so);

    let allowed_prefix = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "strlen", "syscall", "sysconf", "write", "writev", "poll", "sigaction",
        "sigaltstack", "signal", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock",
        "__libc_start_main",
    ]
    .into_iter()
    .collect();

    let mut unexpected = Vec::new();
    for sym in &undef {
        // Strip the "@GLIBC_x.y" / "@GCC_x.y" version suffix.
        let bare = sym.split('@').next().unwrap_or(sym);
        if allowed_prefix.iter().any(|p| bare.starts_with(p)) || allowed_exact.contains(bare) {
            continue;
        }
        unexpected.push(sym.clone());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports {} symbol(s) that are not libc/unwinder: {:?}",
        unexpected.len(),
        unexpected
    );

    // The library must be loadable and its symbol resolvable, which is the
    // practical proof that nothing is genuinely unresolved.
    let pair = load_pair();
    let mut px = [10u8, 20, 30, 40];
    let mut img = CpImage {
        w: 1,
        h: 1,
        pix: px.as_mut_ptr() as *mut CpPixel,
    };
    unsafe { pair.rust.premultiply(&mut img) };
    assert_eq!(px, model_pixel([10, 20, 30, 40]));
}

/// CONFIGS.md / Phase D gate: the crate declares NO cargo features, so the
/// default build is the only configuration. If a feature is ever added, this
/// test fails and forces the feature matrix to be re-verified.
#[test]
fn no_cargo_features_declared() {
    let manifest = std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("Cargo.toml");

    // Collect the contents of a [features] table, if any.
    let mut in_features = false;
    let mut feature_names: Vec<String> = Vec::new();
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if in_features && !line.is_empty() && !line.starts_with('#') {
            if let Some((name, _)) = line.split_once('=') {
                feature_names.push(name.trim().to_string());
            }
        }
    }
    assert!(
        feature_names.is_empty(),
        "Cargo.toml now declares features {feature_names:?}; Phases B and C must be \
         re-run for every combination and SYMBOLS.md/CONFIGS.md updated"
    );
}

/// Phase B gate: the project builds no driver binary on either side, so there is
/// no stdout to compare. If one is ever added, this test fails.
#[test]
fn project_builds_no_driver_binary() {
    let manifest = std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        !manifest.contains("[[bin]]"),
        "Rust crate now declares a [[bin]] target; Phase B must compare stdout"
    );
    assert_eq!(
        manifest.matches("crate-type").count(),
        1,
        "unexpected crate-type configuration"
    );
    assert!(
        manifest.contains("cdylib"),
        "the lib must stay a cdylib so the tests can dlopen it"
    );
    assert!(
        !crate_root().join("src/main.rs").exists(),
        "src/main.rs appeared; Phase B must compare stdout"
    );
    assert!(
        !crate_root().join("src/bin").exists(),
        "src/bin appeared; Phase B must compare stdout"
    );

    let cmake = std::fs::read_to_string(crate_root().join("../c_src/CMakeLists.txt"))
        .expect("c_src/CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; Phase B must compare stdout"
    );
    assert!(
        cmake.contains("add_library"),
        "c_src should build a shared library"
    );
}

/// The C source surface is fully accounted for: two files, one public function.
/// Guards against the "a whole C module was never translated" failure mode.
#[test]
fn c_source_surface_fully_accounted_for() {
    let c_root = crate_root().join("../c_src");
    let mut c_files: Vec<String> = Vec::new();
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                // Skip the cmake build tree.
                if p.file_name().map(|n| n == "build").unwrap_or(false) {
                    continue;
                }
                walk(root, &p, out);
            } else if let Some(ext) = p.extension() {
                if ext == "c" || ext == "h" {
                    let rel = p.strip_prefix(root).unwrap_or(&p);
                    out.push(rel.display().to_string());
                }
            }
        }
    }
    walk(&c_root, &c_root, &mut c_files);
    c_files.sort();
    assert_eq!(
        c_files,
        vec!["include/lib.h".to_string(), "src/lib.c".to_string()],
        "the C source tree changed; re-derive SYMBOLS.md / ERRORS.md / CONFIGS.md"
    );

    // The single C translation unit defines exactly the one exported function.
    let c_src = std::fs::read_to_string(c_root.join("src/lib.c")).unwrap();
    assert_eq!(
        c_src.matches("premultiply(").count(),
        1,
        "lib.c no longer contains exactly one premultiply definition"
    );
    let header = std::fs::read_to_string(c_root.join("include/lib.h")).unwrap();
    assert_eq!(
        header.matches("premultiply").count(),
        1,
        "include/lib.h no longer declares exactly one function"
    );
    assert_eq!(
        header.matches("typedef struct").count(),
        2,
        "include/lib.h no longer declares exactly the two known structs"
    );
    // No namespace-renaming macros that would change the linker symbol name.
    assert!(
        !header.contains("#define"),
        "include/lib.h now has macros; symbol names may be renamed"
    );
    // No error enum / status type was introduced.
    assert!(
        !header.contains("enum"),
        "include/lib.h now declares an enum; ERRORS.md must cover out-of-range values"
    );
}

/// ABI layout parity of the shared structs.
#[test]
fn struct_abi_layout_matches_c() {
    assert_abi_layout();
    // Field offsets, matching the C layout `{ int; int; ptr; }`.
    let img = CpImage {
        w: 0,
        h: 0,
        pix: std::ptr::null_mut(),
    };
    let base = &img as *const CpImage as usize;
    assert_eq!(&img.w as *const i32 as usize - base, 0, "offsetof(w)");
    assert_eq!(&img.h as *const i32 as usize - base, 4, "offsetof(h)");
    assert_eq!(
        &img.pix as *const *mut CpPixel as usize - base,
        8,
        "offsetof(pix)"
    );

    let px = CpPixel {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    let pbase = &px as *const CpPixel as usize;
    assert_eq!(&px.r as *const u8 as usize - pbase, 0, "offsetof(r)");
    assert_eq!(&px.g as *const u8 as usize - pbase, 1, "offsetof(g)");
    assert_eq!(&px.b as *const u8 as usize - pbase, 2, "offsetof(b)");
    assert_eq!(&px.a as *const u8 as usize - pbase, 3, "offsetof(a)");
}
