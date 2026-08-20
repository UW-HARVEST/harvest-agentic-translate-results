// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Enforced as a test so it cannot drift: every symbol the C shared object
// exports must also be exported by the Rust shared object under the exact same
// name, and the Rust object must not have unresolved non-libc dependencies.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .join("libdriver.so")
}

/// Weak toolchain/runtime housekeeping symbols that are emitted by the
/// compiler, not by the library, and are not part of any API.
const TOOLCHAIN_NOISE: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
    "_edata",
    "_end",
    "__bss_start",
    "_fini",
    "_init",
];

fn nm(args: &[&str], so: &Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse `nm -D` output into a set of names, dropping version suffixes
/// (`printf@GLIBC_2.2.5` -> `printf`) and toolchain noise.
fn parse_names(text: &str, keep_types: &[char]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }
        let name = parts.pop().unwrap();
        // The symbol type is the field right before the name.
        let ty = parts
            .last()
            .and_then(|t| t.chars().next())
            .unwrap_or('?');
        if !keep_types.contains(&ty) {
            continue;
        }
        let name = name.split('@').next().unwrap_or(name);
        if TOOLCHAIN_NOISE.contains(&name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn d1_every_c_exported_symbol_is_exported_by_rust() {
    let c_text = nm(&["-D", "--defined-only"], &c_so());
    let r_text = nm(&["-D", "--defined-only"], &rust_so());

    // Global (T/D/B/R) and weak (W/V) *defined* symbols with real content.
    let keep = ['T', 'D', 'B', 'R', 'W', 'V', 'G', 'S'];
    let c_syms = parse_names(&c_text, &keep);
    let r_syms = parse_names(&r_text, &keep);

    assert!(
        !c_syms.is_empty(),
        "no exported symbols parsed from the C .so — the parity check would be \
         vacuous. Raw nm output:\n{c_text}"
    );
    // Guard against a regression in the parser/table itself.
    for expected in ["driver", "printHexCharLine"] {
        assert!(
            c_syms.contains(expected),
            "expected C export `{expected}` not parsed; got {c_syms:?}"
        );
    }

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {r_syms:?}",
        missing.len()
    );
}

#[test]
fn d2_rust_so_has_no_unresolved_non_libc_symbols() {
    let text = nm(&["-D", "--undefined-only"], &rust_so());
    let undef = parse_names(&text, &['U', 'w', 'v']);

    // Everything the Rust cdylib may legitimately import: the C runtime and the
    // libgcc unwinder that backs Rust panics.
    let allowed_prefixes = ["_Unwind_", "__", "pthread_"];
    let allowed: &[&str] = &[
        "printf", "malloc", "calloc", "realloc", "free", "posix_memalign", "memcpy", "memmove",
        "memset", "memcmp", "bcmp", "strlen", "read", "write", "writev", "close", "open", "open64",
        "lseek", "lseek64", "stat", "stat64", "fstat", "fstat64", "statx", "mmap", "mmap64",
        "munmap", "getcwd", "getenv", "readlink", "realpath", "abort", "syscall", "gettid",
        "dl_iterate_phdr", "sysconf", "getpid", "poll", "sigaction", "sigaltstack", "mprotect",
        "signal", "raise", "exit", "_exit", "environ", "dlsym", "dladdr",
    ];

    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|n| {
            !allowed.contains(&n.as_str()) && !allowed_prefixes.iter().any(|p| n.starts_with(p))
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "the Rust .so has unresolved symbols that are neither libc nor the Rust \
         unwinding runtime: {unexpected:?}\nfull undefined set: {undef:?}"
    );
}

#[test]
fn d3_both_shared_objects_fully_resolve_at_load_time() {
    // `ldd -r` reports every unresolvable symbol. Anything reported means the
    // object could not actually be used by a real consumer.
    for so in [c_so(), rust_so()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&so)
            .output()
            .unwrap_or_else(|e| panic!("running ldd -r on {}: {e}", so.display()));
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.contains("undefined symbol"),
            "ldd -r reported undefined symbols for {}:\n{combined}",
            so.display()
        );
    }
}

/// Both objects must be loadable and both symbols callable through `dlopen` /
/// `dlsym` — which is what the differential harness does, and what proves the
/// `#[no_mangle]` export wrappers are real and reachable.
#[test]
fn d4_both_symbols_are_callable_through_dlopen() {
    let out_c = common::capture(|| {
        common::c_api().driver(0x41);
        common::c_api().print_hex_char_line(0x41);
    });
    let out_r = common::capture(|| {
        common::rust_api().driver(0x41);
        common::rust_api().print_hex_char_line(0x41);
    });
    assert_eq!(out_c, b"42\n41\n", "unexpected C ground truth: {out_c:?}");
    common::assert_bytes_eq("D4 dlopen callability", &out_c, &out_r);
}
