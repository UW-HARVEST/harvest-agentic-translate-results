//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined, non-weak dynamic symbols (the real exported ABI surface).
fn defined_globals(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (a, b) = (it.next(), it.next());
        let (kind, name) = match (a, b, it.next()) {
            // "<addr> <kind> <name>"
            (Some(_addr), Some(kind), Some(name)) => (kind, name),
            // "<kind> <name>" (weak/undefined without an address)
            (Some(kind), Some(name), None) => (kind, name),
            _ => continue,
        };
        // Skip weak symbols: those are CRT/toolchain glue, not library API.
        if kind == "w" || kind == "v" || kind == "V" || kind == "W" {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

fn all_dynamic(so: &Path) -> Vec<(String, String)> {
    let out = Command::new("nm").arg("-D").arg(so).output().expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.len() {
            3 => v.push((parts[1].to_string(), parts[2].to_string())),
            2 => v.push((parts[0].to_string(), parts[1].to_string())),
            _ => {}
        }
    }
    v
}

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let l = common::libs();
    let c = defined_globals(&l.c_so_path);
    let r = defined_globals(&l.rust_so_path);

    println!("C   .so ({}): {:?}", l.c_so_path.display(), c);
    println!("Rust.so ({}): {:?}", l.rust_so_path.display(), r);

    assert!(
        c.contains("hdr_compare"),
        "sanity: C .so must export hdr_compare, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The C library's whole ABI is exactly one function.
    assert_eq!(
        c,
        BTreeSet::from(["hdr_compare".to_string()]),
        "the C .so's exported ABI changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let l = common::libs();
    let syms = all_dynamic(&l.rust_so_path);

    // Everything undefined must come from libc / libgcc_s / the unwinder.
    let allowed_prefixes = [
        "_Unwind_",
        "__cxa_",
        "__errno_location",
        "__gmon_start__",
        "__tls_get_addr",
        "_ITM_",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "pthread_mutex_lock", "pthread_mutex_unlock", "read", "readlink",
        "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getrandom", "poll", "sigaltstack", "sigaction", "mprotect",
        "pthread_self", "pthread_attr_init", "pthread_attr_destroy", "pthread_getattr_np",
        "pthread_attr_getstack", "environ", "__libc_start_main",
    ];

    let mut bad = Vec::new();
    for (kind, name) in &syms {
        if kind != "U" {
            continue;
        }
        let base = name.split('@').next().unwrap_or(name);
        let ok = allowed_prefixes.iter().any(|p| base.starts_with(p))
            || allowed_exact.contains(&base);
        if !ok {
            bad.push(name.clone());
        }
    }
    assert!(bad.is_empty(), "unresolved non-libc symbols in the Rust .so: {bad:?}");
}

#[test]
fn hdr_valid_stays_internal_in_both() {
    // `hdr_valid` is `static` in the C: it must not be in either dynamic symbol table.
    let l = common::libs();
    for so in [&l.c_so_path, &l.rust_so_path] {
        let names: BTreeSet<String> = all_dynamic(so).into_iter().map(|(_, n)| n).collect();
        assert!(
            !names.iter().any(|n| n == "hdr_valid"),
            "{} unexpectedly exports hdr_valid",
            so.display()
        );
    }
}

/// Regression guard for the debug-profile divergence found in Phase C:
///
/// a plain `*ptr` dereference makes `rustc` emit a null-pointer precondition check whenever
/// `-C debug-assertions=on`. That check panics, and a panic escaping an `extern "C"` function
/// aborts with `SIGABRT` — whereas the C simply faults with `SIGSEGV`. `src/lib.rs` loads its
/// bytes through `core::ptr::read` to avoid the instrumentation; if anyone reintroduces a raw
/// deref, the panic string comes back and this test fails.
#[test]
fn rust_so_has_no_null_pointer_precondition_check() {
    let l = common::libs();
    let bytes = std::fs::read(&l.rust_so_path).expect("read the Rust .so");
    let needle = b"null pointer dereference occurred";
    let found = bytes.windows(needle.len()).any(|w| w == needle);
    assert!(
        !found,
        "{} contains rustc's null-pointer precondition panic. A raw `*ptr` deref was \
         reintroduced in src/lib.rs: under -C debug-assertions=on that panics (SIGABRT) \
         where the C faults (SIGSEGV). Load bytes via core::ptr::read instead.",
        l.rust_so_path.display()
    );
}

#[test]
fn both_sos_expose_a_callable_hdr_compare() {
    let l = common::libs();
    // Smoke: a real MPEG-1 Layer III header compared with itself.
    let h: [u8; 3] = [0xFF, 0xFB, 0x90];
    let a = unsafe { (l.c)(h.as_ptr(), h.as_ptr()) };
    let b = unsafe { (l.rs)(h.as_ptr(), h.as_ptr()) };
    assert_eq!(a, 1, "C should accept a valid self-comparison");
    assert_eq!(a, b);
}
