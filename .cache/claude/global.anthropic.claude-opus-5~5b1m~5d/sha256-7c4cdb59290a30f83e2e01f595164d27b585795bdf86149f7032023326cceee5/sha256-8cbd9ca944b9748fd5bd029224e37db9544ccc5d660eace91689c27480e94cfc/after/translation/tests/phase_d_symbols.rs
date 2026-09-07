// Phase D — symbol parity, enforced as a test so it cannot silently rot.
//
// Every symbol the C `.so` exports must also be exported by the Rust `.so`
// under the exact same name. The diff must be empty.

mod harness;

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().and_then(|p| p.parent()).expect("target/<profile>");
    for c in [
        dir.join("libdriver.so"),
        manifest_dir().join("target/release/libdriver.so"),
        manifest_dir().join("target/debug/libdriver.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("Rust cdylib not found");
}

/// Linker/loader-synthesised entries that GCC and rustc both emit; they are not
/// part of either library's public API.
const SYNTHETIC: &[&str] = &["_init", "_fini", "__bss_start", "_edata", "_end", "_etext"];

fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        // Drop any `@@GLIBC_x.y` version suffix so names compare cleanly.
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !SYNTHETIC.contains(&s.as_str()))
        // Rust's own mangled std internals inside the cdylib are not API.
        .filter(|s| !s.starts_with("_ZN") && !s.starts_with("_R"))
        // Symbols the statically-linked Rust std re-exports (rust_eh_*, etc.).
        .filter(|s| !s.starts_with("rust_") && !s.starts_with("__rust"))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d_symbol_parity_c_subset_of_rust() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    assert!(!c.is_empty(), "nm found no symbols in the C .so");
    assert!(c.contains(&"driver".to_string()), "C .so must export `driver`");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   symbols: {c:?}\n\
         Rust symbols (filtered): {r:?}"
    );
}

#[test]
fn d_rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the cdylib imports must come from libc / libgcc / the dynamic
    // loader. Anything else would mean a module was left untranslated.
    let unresolved: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        // A `@GLIBC_x.y` / `@@GLIBC_x.y` suffix proves the symbol is satisfied
        // by glibc itself, so it is by definition a libc import.
        .filter(|s| !s.contains("@GLIBC") && !s.contains("@GCC"))
        // Unversioned imports: the loader/unwinder weak symbols and the
        // libgcc/compiler-rt helpers every Rust cdylib references.
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| {
            !(s.starts_with("__") // __libc_start_main, __errno_location, __tls_get_addr, ...
                || s.starts_with('_') // _Unwind_*, _ITM_*, ...
                || s.starts_with("pthread_")
                || s.starts_with("dl")
                || matches!(
                    s.as_str(),
                    "printf" | "memcpy" | "memmove" | "memset" | "memcmp" | "malloc" | "free"
                        | "calloc" | "realloc" | "abort" | "write" | "writev" | "close" | "open"
                        | "read" | "poll" | "getenv" | "sysconf" | "mmap" | "munmap" | "mprotect"
                        | "sigaction" | "sigaltstack" | "syscall" | "bcmp" | "strlen"
                        | "posix_memalign" | "getcwd" | "readlink" | "realpath" | "gettid"
                ))
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so imports non-libc symbols (untranslated module?): {unresolved:?}"
    );
}

/// The dynamically resolvable entry point must be reachable via `dlsym` in both
/// libraries — i.e. the `#[no_mangle]` wrapper is genuinely exported, not just
/// present in the symbol table with local visibility.
#[test]
fn d_dlsym_resolves_driver_in_both() {
    let _c = harness::c_driver();
    let _r = harness::rust_driver();
}
