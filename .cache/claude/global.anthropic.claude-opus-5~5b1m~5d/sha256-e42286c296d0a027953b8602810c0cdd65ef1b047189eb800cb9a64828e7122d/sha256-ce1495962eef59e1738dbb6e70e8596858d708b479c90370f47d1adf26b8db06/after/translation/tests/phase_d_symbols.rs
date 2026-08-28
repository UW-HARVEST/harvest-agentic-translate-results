// Phase D -- symbol parity between the C and Rust shared objects.
//
// Runs `nm -D` on both and requires the C-exported set to be a subset of the
// Rust-exported set (exact names, including any macro-generated ones).

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        args,
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of dynamically exported (defined) symbols.
fn exported(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Names of undefined (imported) symbols.
fn undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "-u"], so)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let ty = it.next()?;
            // "U name" or "w name"
            if ty == "U" || ty == "w" { it.next() } else { None }
        })
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    println!("C   .so: {}", c.display());
    println!("Rust.so: {}", r.display());

    let c_syms = exported(c);
    let r_syms = exported(r);

    println!("C exports  ({}): {:?}", c_syms.len(), c_syms);
    println!("Rust exports ({}): {:?}", r_syms.len(), r_syms);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The C library's documented surface must actually be there.
    assert!(c_syms.contains("dataentry"), "C .so must export `dataentry`");
    assert!(r_syms.contains("dataentry"), "Rust .so must export `dataentry`");

    // `static` C functions must not have leaked into the dynamic table.
    for s in ["find_entry", "process_name", "calculate_lookup", "create_entries", "modify_entries"] {
        assert!(!c_syms.contains(s), "unexpected: C exports static fn {s}");
        assert!(!r_syms.contains(s), "Rust must not export private fn {s}");
    }
}

/// Every symbol the Rust `.so` imports must be a libc / compiler-runtime
/// symbol, i.e. there must be no unresolved non-libc dependency.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();
    let imports = undefined(r);
    println!("Rust imports ({}): {:?}", imports.len(), imports);

    // Prefixes/names provided by glibc, libgcc's unwinder, or the ELF runtime.
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_", "pthread_", "gettid", "statx"];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "strlen", "syscall", "write",
        "writev", "sprintf", "strcpy", "sigaltstack", "sysconf", "mprotect", "pipe2",
        "poll", "sched_getaffinity", "sigaction", "sigaddset", "sigemptyset", "environ",
    ]
    .into_iter()
    .collect();

    let mut bad = Vec::new();
    for s in &imports {
        let ok = allowed_exact.contains(s.as_str())
            || allowed_prefixes.iter().any(|p| s.starts_with(p));
        if !ok {
            bad.push(s.clone());
        }
    }
    assert!(bad.is_empty(), "Rust .so imports non-libc symbols (unresolved deps?): {bad:?}");

    // Heap functions must come from libc so allocation-failure behaviour is
    // identical to the C library's.
    assert!(imports.contains("malloc"), "Rust must call libc malloc");
    assert!(imports.contains("free"), "Rust must call libc free");
}

/// Both libraries must be loadable and expose a working `dataentry` through
/// `dlsym` -- this is what validates the `#[unsafe(no_mangle)]` wrapper.
#[test]
fn both_libraries_load_and_dlsym_dataentry() {
    let p = pair();
    assert_eq!(p.c.label, "C");
    assert_eq!(p.rust.label, "Rust");

    // The two targets MUST be distinct shared objects, otherwise every
    // "differential" assertion would be comparing a library against itself.
    assert_ne!(p.c.path, p.rust.path, "C and Rust .so paths must differ");
    let c_addr = p.c.dataentry as usize;
    let r_addr = p.rust.dataentry as usize;
    println!("C dataentry @ {c_addr:#x}, Rust dataentry @ {r_addr:#x}");
    assert_ne!(c_addr, r_addr, "dlsym returned the SAME address for both .so files");

    // A trivially-known value through both function pointers.
    let (c, r) = call_both(3, 0, 0, 0);
    assert_eq!(c, 20);
    assert_eq!(r, 20);
}

/// The FFI ABI shape assumed by the tests matches the C struct layout.
#[test]
fn abi_assumptions() {
    assert_eq!(std::mem::size_of::<std::ffi::c_int>(), 4, "int must be 4 bytes");
    assert_eq!(NAME_LENGTH, 32);
    assert_eq!(SIZEOF_DATAENTRY, 4 + 4 + NAME_LENGTH);
    assert_eq!(MAX_ENTRIES, 10);
}
