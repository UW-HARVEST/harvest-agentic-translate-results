// Phase D -- symbol parity, enforced as a test rather than a one-off shell diff
// so it keeps holding under every feature combination and profile.

mod common;

use common::{so_path, Impl};

use std::collections::BTreeSet;
use std::process::Command;

/// Defined, globally-visible symbols of a `.so`, via `nm -D --defined-only`.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (kind, name) = match (it.next(), it.next(), it.next()) {
                // "<addr> <kind> <name>"
                (Some(_), Some(k), Some(n)) => (k, n),
                // "<kind> <name>" (weak/undefined-style, no address)
                (Some(k), Some(n), None) => (k, n),
                _ => return None,
            };
            // Keep real code/data definitions; drop the linker/toolchain
            // bookkeeping weak symbols (`w`) that every .so carries.
            match kind {
                "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" => {
                    Some(name.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

/// Undefined (imported) symbols of a `.so`.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Every symbol the C `.so` defines must also be defined by the Rust `.so`,
/// under the exact same name. The diff must reach EMPTY.
#[test]
fn sym_01_rust_exports_every_c_symbol() {
    let c = defined_symbols(&so_path(Impl::C));
    let rust = defined_symbols(&so_path(Impl::Rust));

    // Sanity: we actually parsed something.
    assert!(
        c.contains("driver") && c.contains("printLine") && c.contains("bad") && c.contains("good"),
        "nm parsing looks wrong -- C defined symbols: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {rust:?}",
        missing.len()
    );
}

/// The four documented entry points, spelled out explicitly, so this test fails
/// loudly if `nm` output parsing ever silently degrades to an empty set.
#[test]
fn sym_02_expected_entry_points_present_on_both() {
    let c = defined_symbols(&so_path(Impl::C));
    let rust = defined_symbols(&so_path(Impl::Rust));
    for name in ["printLine", "bad", "good", "driver"] {
        assert!(c.contains(name), "C .so should define {name}");
        assert!(rust.contains(name), "Rust .so should define {name}");
    }
}

/// The `static` C helpers must stay out of both dynamic symbol tables.
#[test]
fn sym_03_static_helpers_absent_from_both() {
    let c = defined_symbols(&so_path(Impl::C));
    let rust = defined_symbols(&so_path(Impl::Rust));
    for name in ["helperGood", "helperBad"] {
        assert!(!c.contains(name), "C .so should not export static {name}");
        assert!(
            !rust.contains(name),
            "Rust .so should not export {name}: it is `static` in the C source"
        );
    }
}

/// No missing/undefined NON-LIBC symbols in the Rust `.so` -- i.e. nothing that
/// the dynamic loader could fail to resolve against the system libraries.
#[test]
fn sym_04_no_unresolvable_undefined_symbols_in_rust() {
    let undef = undefined_symbols(&so_path(Impl::Rust));

    // Everything a Rust cdylib legitimately imports: glibc, and the libgcc
    // unwinder that the panic runtime needs.
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_"];
    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .filter(|s| !is_libc_symbol(s))
        .collect();

    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined non-libc symbols: {leftovers:?}"
    );

    // The whole point of routing printLine through libc `puts`: the C .so's
    // only import is `puts`, and the Rust .so must share that stdout stream.
    assert!(
        undef.contains("puts"),
        "Rust .so should import libc `puts`, like the C .so does, so both write \
         through the same stdout FILE with the same buffering; imports: {undef:?}"
    );
    assert!(undefined_symbols(&so_path(Impl::C)).contains("puts"));
}

fn is_libc_symbol(name: &str) -> bool {
    const LIBC: &[&str] = &[
        "puts", "printf", "malloc", "calloc", "realloc", "free", "posix_memalign", "memcpy",
        "memmove", "memset", "bcmp", "strlen", "abort", "getenv", "getcwd", "readlink", "realpath",
        "open64", "close", "read", "write", "writev", "lseek64", "fstat64", "stat64", "statx",
        "mmap64", "munmap", "syscall", "gettid", "dl_iterate_phdr", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific", "sysconf", "dlsym",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_self", "sigaltstack", "sigaction",
        "mprotect", "poll", "signal", "raise", "environ", "strerror_r",
    ];
    LIBC.contains(&name)
}

/// The `.so` under test really is the freshly built Rust one, not a copy of the
/// C library. Guards the whole suite against comparing a library with itself.
#[test]
fn sym_05_the_two_shared_objects_are_distinct_artifacts() {
    let c = so_path(Impl::C);
    let rust = so_path(Impl::Rust);
    assert_ne!(c, rust, "C and Rust .so paths must differ");

    let c_bytes = std::fs::read(&c).unwrap();
    let rust_bytes = std::fs::read(&rust).unwrap();
    assert_ne!(
        c_bytes, rust_bytes,
        "the two .so files are byte-identical -- the harness is not actually \
         testing two different implementations"
    );

    // The Rust artifact carries the Rust runtime; the C one does not.
    assert!(
        undefined_symbols(&rust).len() > undefined_symbols(&c).len(),
        "expected the Rust cdylib to import more than the C .so"
    );
}
