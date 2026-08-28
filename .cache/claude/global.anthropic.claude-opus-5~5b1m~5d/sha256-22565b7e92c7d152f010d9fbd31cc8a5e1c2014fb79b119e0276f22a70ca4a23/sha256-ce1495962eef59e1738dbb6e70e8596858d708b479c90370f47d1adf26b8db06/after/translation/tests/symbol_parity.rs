// Phase D — symbol parity, checked mechanically rather than by hand.
//
// Turns the claims in SYMBOLS.md into assertions:
//   * every symbol the C `.so` exports, the Rust `.so` exports too, with the
//     exact same name (the diff must reach EMPTY);
//   * the Rust `.so` has no unresolved / non-libc undefined symbols;
//   * every exported symbol is actually callable through `dlsym`.

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
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of dynamic symbols DEFINED by `so` (nm type `T`/`t`/`D`/`B`/`R`/`W`),
/// excluding Rust-internal mangled names and the compiler/runtime housekeeping
/// symbols that are not part of any public API.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let text = nm(&["-D", "--defined-only"], so);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(_addr_or_type), Some(rest)) = (it.next(), it.next()) else {
            continue;
        };
        // `nm -D` prints "<addr> <type> <name>"; for undefined-address entries
        // the first column is the type instead.
        let (ty, name) = match it.next() {
            Some(n) => (rest, n),
            None => continue,
        };
        let ty = ty.chars().next().unwrap_or('?');
        if !matches!(ty, 'T' | 't' | 'D' | 'd' | 'B' | 'b' | 'R' | 'r' | 'W' | 'V') {
            continue;
        }
        if name.starts_with("_ZN") || name.starts_with("_R") {
            continue; // Rust-mangled internals
        }
        if name.starts_with("__rust") || name.starts_with("rust_") {
            continue; // Rust runtime hooks
        }
        if name.starts_with("_ITM_") || name.starts_with("__") || name == "_init" || name == "_fini"
        {
            continue; // toolchain housekeeping
        }
        set.insert(name.to_string());
    }
    set
}

/// The five functions declared in `c_src/src/lib.c`, all with external linkage.
const EXPECTED: [&str; 5] = [
    "apply_bit_operations",
    "envy",
    "init_config_from_env",
    "parse_env_numeric",
    "perform_operation",
];

#[test]
fn phase_d_rust_exports_every_c_symbol() {
    let _g = lock();
    // Force the harness to rebuild + locate both objects.
    let (_c, _r) = both();
    let c_so = c_so_path();
    let r_so = rust_so_path();

    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&r_so);

    println!("C   .so {} exports: {:?}", c_so.display(), c_syms);
    println!("Rust.so {} exports: {:?}", r_so.display(), r_syms);

    // The C library must export exactly the five non-static functions; if this
    // ever grows, the translation is incomplete and the test must be updated
    // deliberately rather than silently passing.
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the set of symbols exported by the C .so changed — a C source file may \
         have been added that is not yet translated"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "SYMBOL DIFF NOT EMPTY — the Rust .so is missing {} of the C .so's {} \
         exported symbols: {:?}\n\
         For each: add the #[no_mangle] extern \"C\" wrapper if the impl exists, \
         or translate the missing C source.",
        missing.len(),
        c_syms.len(),
        missing
    );
}

#[test]
fn phase_d_no_unresolved_symbols_in_rust_so() {
    let _g = lock();
    let (_c, _r) = both();
    let r_so = rust_so_path();

    // `ldd -r` reports every symbol (function and data) that cannot be resolved
    // against the standard library search path. Anything listed here would be a
    // genuinely missing implementation rather than a libc import.
    let out = Command::new("ldd").arg("-r").arg(&r_so).output();
    match out {
        Ok(out) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let unresolved: Vec<&str> = text
                .lines()
                .filter(|l| l.contains("undefined symbol"))
                .collect();
            assert!(
                unresolved.is_empty(),
                "the Rust .so has unresolved symbols:\n{}",
                unresolved.join("\n")
            );
        }
        Err(e) => println!("skipping ldd check ({e})"),
    }

    // And cross-check the undefined list from nm: every entry must be either a
    // weak symbol or resolvable from libc / the platform runtime.
    let text = nm(&["-D", "--undefined-only"], &r_so);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        let (ty, name) = match cols.as_slice() {
            [ty, name] => (*ty, *name),
            [_addr, ty, name] => (*ty, *name),
            _ => continue,
        };
        if ty == "w" || ty == "v" {
            continue; // weak: optional by definition
        }
        let base = name.split('@').next().unwrap_or(name);
        // Everything the Rust std / libgcc runtime legitimately imports.
        let ok = base.starts_with("_Unwind_")
            || base.starts_with("__")
            || base.starts_with("pthread_")
            || KNOWN_LIBC.contains(&base);
        if !ok {
            suspicious.push(name.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "the Rust .so imports non-libc symbols that are not provided anywhere: {suspicious:?}"
    );
}

/// libc entry points the two libraries are allowed to import. The first eight
/// are exactly the ones the C `.so` itself imports, which is what makes the
/// formatted output byte-identical; the rest come from the Rust standard library.
const KNOWN_LIBC: &[&str] = &[
    // imported by the C .so too
    "getenv", "atoi", "strchr", "printf", "fprintf", "snprintf", "stderr", "puts", "memcpy",
    // Rust std / allocator / platform
    "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd", "gettid",
    "lseek64", "malloc", "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
    "read", "readlink", "realloc", "realpath", "stat64", "statx", "strlen", "syscall", "write",
    "writev", "sysconf", "getrandom", "clock_gettime", "environ", "exit", "fcntl", "poll",
    "sigaction", "sigaltstack", "signal", "mprotect", "madvise", "openat64", "pipe2", "prctl",
    "sched_getaffinity", "sched_yield", "nanosleep", "isatty", "memchr", "strerror_r", "unlink",
];

#[test]
fn phase_d_every_exported_symbol_is_callable_via_dlsym() {
    let _g = lock();
    // `both()` resolves all five symbols in both objects through `dlsym` and
    // panics on any that is absent, so simply getting here proves callability.
    let (c, r) = both();
    for name in EXPECTED {
        println!("{name}: resolved in C and Rust");
    }
    // Exercise each one once so a symbol that resolves but is a stub / panics
    // immediately cannot pass this test.
    env_clear_prog();
    for api in [c, r] {
        let mut f = Flags4([0, 0, 0, 0]);
        let got = capture(|| unsafe {
            (api.init_config_from_env)(f.as_mut_ptr());
            let a = (api.parse_env_numeric)(cstring("PROG_MULTIPLIER").as_ptr(), 10);
            let b = (api.perform_operation)(3, 4, f.as_mut_ptr());
            let c2 = (api.apply_bit_operations)(b, f.as_mut_ptr());
            let d = (api.envy)(1, 2, 3, 4);
            a ^ b ^ c2 ^ d
        });
        println!("{} smoke-called all five symbols: ret={}", api.name, got.ret);
        assert_ne!(
            f.0[0], 0,
            "{}: init_config_from_env produced an all-zero flag byte — looks like a stub",
            api.name
        );
    }
}

/// The crate declares no `[features]`, so there is exactly one configuration.
/// Assert that mechanically so that adding a feature later forces the feature
/// matrix in `tests/feature_matrix.sh` to be revisited.
#[test]
fn phase_d_cargo_toml_declares_no_features() {
    let manifest = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");

    let mut in_features = false;
    let mut features = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
            continue;
        }
        if in_features && !t.is_empty() && !t.starts_with('#') {
            if let Some((name, _)) = t.split_once('=') {
                features.push(name.trim().to_string());
            }
        }
    }
    assert!(
        features.is_empty(),
        "Cargo.toml now declares features {features:?} — Phases B and C must be \
         re-run for every combination; update tests/feature_matrix.sh"
    );
}
