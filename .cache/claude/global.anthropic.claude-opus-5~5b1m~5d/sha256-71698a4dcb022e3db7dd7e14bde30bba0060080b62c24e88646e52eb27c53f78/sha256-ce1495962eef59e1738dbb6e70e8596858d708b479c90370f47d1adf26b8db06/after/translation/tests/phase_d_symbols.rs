// Phase D -- symbol parity enforced as a test, so it cannot silently rot.
//
// Every symbol the C `.so` DEFINES must also be DEFINED by the Rust `.so`, with
// the exact same name. The diff must be empty.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = repo_root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so") && p.is_file()
        })
        .collect();
    v.sort();
    v.into_iter().next().expect("no C lib*.so found")
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = repo_root().join("translation/target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libbuffapp_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("libbuffapp_lib.so not found");
}

/// `nm -D --defined-only`, names with any `@GLIBC_x` version suffix stripped.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|n| n.split('@').next().unwrap().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c_so = find_c_so();
    let r_so = find_rust_so();
    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&r_so);

    // Sanity: nm actually produced something.
    assert!(
        c_syms.len() >= 6,
        "nm found only {} defined symbols in {}",
        c_syms.len(),
        c_so.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n  C:    {}\n  Rust: {}",
        missing.len(),
        missing,
        c_so.display(),
        r_so.display()
    );
}

#[test]
fn phase_d_the_six_documented_symbols_are_present_in_both() {
    // SYMBOLS.md enumerates exactly these; assert them by name so a rename in
    // either direction is caught even if `nm` output changes shape.
    const EXPECTED: [&str; 6] = [
        "create_buffer",
        "append_to_buffer",
        "destroy_buffer",
        "get_operation_name",
        "perform_operation",
        "buffapp",
    ];
    let c_syms = defined_symbols(&find_c_so());
    let r_syms = defined_symbols(&find_rust_so());
    for s in EXPECTED {
        assert!(c_syms.contains(s), "C .so does not define {s}");
        assert!(r_syms.contains(s), "Rust .so does not define {s}");
    }
    // And the C library defines nothing beyond those six.
    let extra: Vec<&String> = c_syms
        .iter()
        .filter(|s| !EXPECTED.contains(&s.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "C .so defines symbols not covered by SYMBOLS.md: {extra:?}"
    );
}

#[test]
fn phase_d_all_symbols_are_callable_via_dlsym() {
    // Exporting a name is not enough -- each must be reachable through dlsym in
    // BOTH objects and behave (this is what `pair()` proves by loading them).
    let (cl, rl) = pair();
    unsafe {
        // One live call per symbol, through the export wrapper.
        assert_eq!(
            read_cstr((cl.get_operation_name)(2)),
            read_cstr((rl.get_operation_name)(2))
        );
        let op = cstring(b"multiply");
        assert_eq!(
            (cl.perform_operation)(6, 7, op.as_ptr()),
            (rl.perform_operation)(6, 7, op.as_ptr())
        );
        let cb = (cl.create_buffer)(8);
        let rb = (rl.create_buffer)(8);
        assert!(!cb.is_null() && !rb.is_null());
        let s = cstring(b"symbol-parity");
        assert_eq!(
            (cl.append_to_buffer)(cb, s.as_ptr()),
            (rl.append_to_buffer)(rb, s.as_ptr())
        );
        assert_eq!(snapshot(cb), snapshot(rb));
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
        let (cv, cout) = capture_stdout(|| (cl.buffapp)(9, 8, 7, 6));
        let (rv, rout) = capture_stdout(|| (rl.buffapp)(9, 8, 7, 6));
        assert_eq!(cv, rv);
        assert_eq!(cout, rout);
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_imports() {
    // Loading the object with RTLD_NOW would fail if anything were unresolvable;
    // `pair()` uses RTLD_LAZY, so force the strict check here by listing the
    // undefined symbols and confirming they all come from the platform runtime.
    let r_so = find_rust_so();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&r_so)
        .output()
        .expect("nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|n| n.split('@').next().unwrap().to_string())
        .filter(|n| !n.is_empty())
        .collect();

    // Anything not provided by libc / libgcc / the ELF boilerplate would be a
    // genuinely missing implementation.
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__"];
    let allowed_exact: BTreeSet<&str> = [
        "malloc", "realloc", "calloc", "free", "posix_memalign",
        "strlen", "strcpy", "strcmp", "bcmp", "memcpy", "memmove", "memset",
        "sprintf", "printf", "abort", "getenv", "getcwd", "readlink", "realpath",
        "open64", "close", "read", "write", "writev", "lseek64",
        "fstat64", "stat64", "statx", "mmap64", "munmap", "syscall", "gettid",
        "dl_iterate_phdr", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific",
    ]
    .into_iter()
    .collect();

    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|n| {
            !allowed_exact.contains(n.as_str())
                && !allowed_prefixes.iter().any(|p| n.starts_with(p))
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc imports (missing implementations?): {suspicious:?}"
    );
}
