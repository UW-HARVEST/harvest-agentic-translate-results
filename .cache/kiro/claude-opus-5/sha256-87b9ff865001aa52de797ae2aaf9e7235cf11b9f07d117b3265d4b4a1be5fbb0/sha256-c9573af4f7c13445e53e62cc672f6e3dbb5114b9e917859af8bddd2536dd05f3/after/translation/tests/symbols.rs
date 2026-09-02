//! Phase D — symbol-parity gate, enforced as a test rather than only in
//! `SYMBOLS.md`, so a later regression cannot silently drop an export.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;

/// `nm -D --defined-only <so>` reduced to the set of exported names.
fn exported(so: &Path) -> BTreeSet<String> {
    nm(so, "--defined-only")
        .into_iter()
        .filter_map(|(kind, name)| {
            // Only global/weak *text or data* definitions are part of the ABI
            // surface a consumer can bind to.
            if matches!(kind.as_str(), "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i") {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

/// `nm -D --undefined-only <so>` reduced to the set of imported names.
fn imported(so: &Path) -> BTreeSet<String> {
    nm(so, "--undefined-only")
        .into_iter()
        .map(|(_, name)| name)
        .collect()
}

fn nm(so: &Path, flag: &str) -> Vec<(String, String)> {
    let out = std::process::Command::new("nm")
        .args(["-D", flag])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D {flag} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().collect::<Vec<_>>();
            // "<addr> <kind> <name>" or "<kind> <name>" for undefined.
            let name = it.pop()?.to_string();
            let kind = it.pop()?.to_string();
            // Strip glibc/gcc version suffixes: puts@GLIBC_2.2.5 -> puts
            let name = name.split('@').next().unwrap_or(&name).to_string();
            Some((kind, name))
        })
        .collect()
}

/// Names that are supplied by the platform (libc, the unwinder, the loader,
/// the Rust runtime's own libc usage) rather than by the translated project.
fn is_platform_symbol(name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__libc_", "__tls_", "__errno",
        "pthread_", "_dl_", "__stack_chk", "__gxx_", "_Z",
    ];
    const EXACT: &[&str] = &[
        // stdio / string / memory
        "puts", "printf", "fputs", "fwrite", "fflush", "memcpy", "memmove", "memset",
        "memcmp", "bcmp", "strlen", "malloc", "calloc", "realloc", "free",
        "posix_memalign", "aligned_alloc", "abort", "exit", "getenv", "getcwd",
        "readlink", "realpath", "open", "open64", "close", "read", "write", "writev",
        "lseek", "lseek64", "stat", "stat64", "fstat", "fstat64", "statx", "mmap",
        "mmap64", "munmap", "mprotect", "syscall", "gettid", "dl_iterate_phdr",
        "sigaltstack", "sigaction", "sigaddset", "sigemptyset", "getpid", "sysconf",
        "poll", "nanosleep", "clock_gettime", "dlsym", "dladdr", "environ",
    ];
    PREFIXES.iter().any(|p| name.starts_with(p)) || EXACT.contains(&name)
}

#[test]
fn phase_d_every_c_export_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exports in the C .so — the parity check would be vacuous"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) the C .so exports: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );

    // The one symbol the header declares must genuinely be there, spelled
    // exactly as the C spells it.
    assert!(c.contains("helloworld"), "C must export `helloworld`: {c:?}");
    assert!(r.contains("helloworld"), "Rust must export `helloworld`: {r:?}");
}

#[test]
fn phase_d_rust_has_no_unresolved_project_symbols() {
    let undefined = imported(&rust_so_path());
    let leftovers: Vec<&String> = undefined
        .iter()
        .filter(|n| !is_platform_symbol(n))
        .collect();
    assert!(
        leftovers.is_empty(),
        "the Rust .so imports {} symbol(s) that are not platform/libc symbols, \
         which would mean an untranslated dependency: {leftovers:?}",
        leftovers.len()
    );
}

#[test]
fn phase_d_no_stubbed_implementations() {
    // A symbol that exists only to satisfy `nm -D` is worse than a missing one.
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for needle in ["unimplemented!", "todo!", "unreachable!"] {
        assert!(
            !src.contains(needle),
            "src/lib.rs contains `{needle}` — a stub cannot stand in for a translation"
        );
    }
}

#[test]
fn phase_d_every_c_source_file_has_a_translation() {
    // Guards against the failure mode where a whole C module was skipped: the
    // symbol diff would only catch it if the module exported something, so
    // count the translation units too.
    let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src");
    let mut units = Vec::new();
    collect_c_files(&c_src, &mut units);
    assert_eq!(
        units.len(),
        1,
        "the C project has {} translation unit(s) ({units:?}); SYMBOLS.md documents 1. \
         If this changed, a module may be untranslated.",
        units.len()
    );
}

fn collect_c_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // `build/` holds CMake's own generated scratch files.
            if p.file_name().map(|n| n == "build").unwrap_or(false) {
                continue;
            }
            collect_c_files(&p, out);
        } else if p.extension().map(|x| x == "c").unwrap_or(false) {
            out.push(p.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
}
