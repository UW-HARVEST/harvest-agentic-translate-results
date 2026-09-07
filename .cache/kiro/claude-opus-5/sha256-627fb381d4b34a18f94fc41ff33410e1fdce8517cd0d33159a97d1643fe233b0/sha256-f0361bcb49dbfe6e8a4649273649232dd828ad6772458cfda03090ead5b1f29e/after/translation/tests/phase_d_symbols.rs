// Phase D — symbol parity, enforced as a test so it cannot silently regress.
//
// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
// exact same name, and the Rust `.so` must have no undefined non-libc symbols.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(so: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

fn defined(so: &Path) -> BTreeSet<String> {
    nm(so, "--defined-only").into_iter().collect()
}

fn undefined(so: &Path) -> BTreeSet<String> {
    nm(so, "--undefined-only").into_iter().collect()
}

fn symbols_every_c_export_is_exported_by_rust() {
    let c = defined(&common::c_so());
    let r = defined(&common::rust_so());

    let missing: Vec<_> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}",
        missing.len()
    );

    // The two documented external-linkage functions must actually be there.
    for expected in ["driver", "run"] {
        assert!(c.contains(expected), "C .so lost `{expected}`");
        assert!(r.contains(expected), "Rust .so lost `{expected}`");
    }
}

fn symbols_static_c_functions_stay_internal_on_both_sides() {
    let c = defined(&common::c_so());
    let r = defined(&common::rust_so());
    for internal in [
        "the_house",
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
        "parse_val",
    ] {
        assert!(
            !c.contains(internal),
            "unexpected: C exports the `static` item `{internal}`"
        );
        assert!(
            !r.contains(internal),
            "Rust exports `{internal}`, which has internal linkage in the C"
        );
    }
}

fn symbols_rust_has_no_undefined_non_libc_symbols() {
    let u = undefined(&common::rust_so());
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location", "__libc_",
        "__rust_",
    ];
    let libc_syms: BTreeSet<&str> = [
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
        "printf",
        "pthread_getspecific",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_setspecific",
        "puts",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sigaltstack",
        "sigaction",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "strtol",
        "sysconf",
        "syscall",
        "write",
        "writev",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = u
        .iter()
        .filter(|s| {
            !libc_syms.contains(s.as_str()) && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols: {unexpected:?}"
    );
}

fn no_driver_binary_is_built_by_either_side() {
    // Documents the vacuous "compare binaries' stdout" clause: neither build
    // system declares an executable.
    let cmake = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/CMakeLists.txt"),
    )
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; add a binary stdout comparison"
    );

    let cargo = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !cargo.contains("[[bin]]"),
        "the crate now builds a binary; add a binary stdout comparison"
    );
    assert!(
        !cargo.contains("[features]"),
        "the crate now declares features; extend the feature-combination sweep"
    );
}

// ------------------------------------------------------------------- runner

fn main() -> ! {
    common::run_suite(
        "phase D (symbol parity)",
        &[
            ("symbols_every_c_export_is_exported_by_rust", symbols_every_c_export_is_exported_by_rust as common::TestFn),
            ("symbols_static_c_functions_stay_internal_on_both_sides", symbols_static_c_functions_stay_internal_on_both_sides as common::TestFn),
            ("symbols_rust_has_no_undefined_non_libc_symbols", symbols_rust_has_no_undefined_non_libc_symbols as common::TestFn),
            ("no_driver_binary_is_built_by_either_side", no_driver_binary_is_built_by_either_side as common::TestFn),
        ],
    )
}
