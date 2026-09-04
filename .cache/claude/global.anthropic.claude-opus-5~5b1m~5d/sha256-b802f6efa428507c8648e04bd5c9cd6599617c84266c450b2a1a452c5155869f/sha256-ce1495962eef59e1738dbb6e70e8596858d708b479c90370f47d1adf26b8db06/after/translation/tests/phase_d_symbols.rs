//! Phase D -- symbol parity.
//!
//! Asserts, from inside the test suite, that every symbol the C `.so` exports is
//! also exported by the Rust `.so` under the exact same name, and that nothing
//! the C keeps private (`static inner`) leaks out of the Rust build.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn so_paths() -> (PathBuf, PathBuf) {
    // Loading the pair first asserts both files exist and that both symbols
    // resolve; then reuse the harness's own path resolution so this test always
    // inspects exactly the objects the differential tests called into.
    let _ = pair();
    (c_so_path(), rust_so_path())
}

/// Exported (dynamic, defined) global TEXT/DATA symbol names, via `nm -D`.
fn dynamic_defined(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required for the symbol-parity test)");
    assert!(out.status.success(), "nm failed on {path:?}: {}", String::from_utf8_lossy(&out.stderr));

    let mut set = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = line.split_whitespace();
        let (kind, name) = match (parts.next(), parts.next(), parts.next()) {
            // "<addr> <kind> <name>"
            (Some(_), Some(k), Some(n)) => (k.to_string(), n.to_string()),
            // "<kind> <name>" (undefined-address form)
            (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
            _ => continue,
        };
        // Only compare code/data symbols the C library actually publishes;
        // ignore the toolchain's own bookkeeping symbols.
        if !matches!(kind.as_str(), "T" | "t" | "D" | "B" | "W" | "R") {
            continue;
        }
        if matches!(
            name.as_str(),
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_IO_stdin_used"
        ) {
            continue;
        }
        set.insert(name);
    }
    set
}

#[test]
fn d1_rust_so_exports_every_c_symbol() {
    let (c_path, rs_path) = so_paths();
    let c_syms = dynamic_defined(&c_path);
    let rs_syms = dynamic_defined(&rs_path);

    // The ground truth, straight from the C source.
    assert!(
        c_syms.contains("fma_array") && c_syms.contains("driver"),
        "the C .so does not export the expected functions; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rs_syms:?}",
        missing.len(),
        c_syms.len(),
        rs_syms.len()
    );
}

#[test]
fn d2_static_c_function_is_not_exported_by_either() {
    let (c_path, rs_path) = so_paths();
    for (label, p) in [("C", &c_path), ("Rust", &rs_path)] {
        let syms = dynamic_defined(p);
        assert!(
            !syms.contains("inner"),
            "{label} .so exports `inner`, but it is `static` in c_src/src/driver.c"
        );
    }
}

/// Every symbol the Rust `.so` imports must actually resolve against the system
/// libraries it declares as needed. `ldd -r` performs the full data+function
/// relocation check and prints `undefined symbol: X` for anything that would
/// fail to load in a plain C consumer's process.
///
/// (This is the meaningful form of "0 missing/undefined non-libc symbols": every
/// `nm -D --undefined-only` entry in a Rust cdylib is a versioned glibc/libgcc
/// import such as `printf@GLIBC_2.2.5`, so name-matching them is pointless --
/// what matters is that the loader can bind them all.)
#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    let (c_path, rs_path) = so_paths();
    for (label, p) in [("C", &c_path), ("Rust", &rs_path)] {
        let out = Command::new("ldd").arg("-r").arg(p).output().expect("ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "{label} .so ({p:?}) has unresolved dynamic symbols:\n{}",
            bad.join("\n")
        );
    }

    // Also confirm the Rust .so imports nothing exotic: strip the @GLIBC_x.y
    // version suffixes and require every undefined symbol to come from glibc,
    // libgcc's unwinder, or the compiler's ITM/TLS stubs.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rs_path)
        .output()
        .expect("nm");
    let allowed_exact = [
        "printf", "memcpy", "memmove", "memset", "memcmp", "bcmp", "strlen", "malloc", "calloc",
        "realloc", "free", "posix_memalign", "abort", "write", "writev", "read", "close",
        "open64", "lseek64", "fstat64", "stat64", "statx", "mmap64", "munmap", "getcwd",
        "getenv", "readlink", "realpath", "syscall", "gettid", "sysconf", "dl_iterate_phdr",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
    ];
    let mut suspicious = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let raw = line.split_whitespace().last().unwrap_or("");
        if raw.is_empty() {
            continue;
        }
        let name = raw.split('@').next().unwrap_or(raw);
        let ok = name.starts_with("__")
            || name.starts_with("_ITM_")
            || name.starts_with("_Unwind_")
            || allowed_exact.contains(&name);
        if !ok {
            suspicious.push(raw.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so imports symbols outside glibc/libgcc: {suspicious:?}"
    );
}

/// Every symbol the C header/source declares must be dlopen-able from the Rust
/// `.so` by its exact name -- this is what actually validates the `#[no_mangle]`
/// export wrappers, independently of `nm` parsing.
#[test]
fn d4_every_c_symbol_is_dlsym_able_from_rust_so() {
    let p = pair();
    // `pair()` already resolves both `fma_array` and `driver` through
    // `Library::get` on each .so and panics if either is missing; calling them
    // once more here proves the resolved pointers are live code.
    let m = [2i32, 3];
    let mut o = [0i32; 2];
    unsafe {
        (p.rs.fma_array)(o.as_mut_ptr(), m.as_ptr(), m.as_ptr(), m.as_ptr(), 2);
    }
    assert_eq!(o, [6, 12]);
    let bytes = diff_driver(p, &m, 2, "D4");
    assert_eq!(String::from_utf8(bytes).unwrap(), "6\n12\n");
}
