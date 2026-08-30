//! Phase D — symbol parity between the two shared objects, and a dual-load
//! smoke test.
//!
//! The symbol diff is computed here rather than trusted from `SYMBOLS.md`, so
//! the artifact can never drift away from reality: if someone adds a function to
//! `driver.c` and forgets to translate it, `parity_01_...` fails.

mod common;

use std::collections::BTreeSet;
use std::ffi::{c_char, c_int};
use std::process::Command;

/// Dynamic symbols that every shared object gets from the toolchain / libc and
/// that are not part of the library's own API.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__gmon")
        || name.starts_with("__cxa")
        || name.starts_with("_Unwind")
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
        || matches!(
            name,
            "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "__libc_start_main"
        )
}

/// `nm -D --defined-only <so>` reduced to the set of exported symbol names.
fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .filter(|n| !is_toolchain_symbol(n))
        .map(str::to_owned)
        .collect()
}

/// The five symbols `driver.c` gives external linkage. `goodG2B` and `goodB2G`
/// are `static` and must NOT appear.
const EXPECTED: [&str; 5] = ["bad", "driver", "good", "printIntLine", "printLine"];

#[test]
fn parity_01_symbol_diff_is_empty() {
    let a = common::artifacts();
    let c_syms = defined_dynamic_symbols(&a.c_so);
    let rust_syms = defined_dynamic_symbols(&a.rust_so);

    let missing: Vec<_> = c_syms.difference(&rust_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   : {:?}\nRust: {:?}",
        missing.len(),
        missing,
        c_syms,
        rust_syms
    );

    // The C set must be exactly the five documented API functions, so that a new
    // C function silently appearing is noticed rather than absorbed.
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's exported API changed; SYMBOLS.md / CONFIGS.md need re-deriving"
    );
    assert_eq!(rust_syms, expected, "the Rust .so exports a different API");
}

#[test]
fn parity_02_static_helpers_are_not_exported() {
    let a = common::artifacts();
    for so in [&a.c_so, &a.rust_so] {
        let syms = defined_dynamic_symbols(so);
        for hidden in ["goodG2B", "goodB2G"] {
            assert!(
                !syms.contains(hidden),
                "{} exports {hidden}, but it is `static` in driver.c",
                so.display()
            );
        }
    }
}

/// No undefined non-libc symbols: every import of the Rust `.so` must actually
/// resolve, so a plain C consumer can link and load it exactly like the C one.
///
/// This is checked the authoritative way — by asking the dynamic loader — rather
/// than by matching import names against a hand-written libc allowlist, which
/// would silently rot as the Rust standard library changes which syscalls it
/// wraps.
#[test]
fn parity_03_no_undefined_non_libc_symbols() {
    let a = common::artifacts();

    for so in [&a.c_so, &a.rust_so] {
        // `ldd -r` performs both data and function relocation checks and prints
        // an "undefined symbol: X (path)" line for anything unresolvable.
        let out = Command::new("ldd").arg("-r").arg(so).output().expect("run ldd -r");
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
            "{} has unresolved imports:\n{}",
            so.display(),
            unresolved.join("\n")
        );
        assert!(
            !text.contains("not found"),
            "{} has a missing shared-library dependency:\n{text}",
            so.display()
        );
    }

    // And the library's *own* API surface must contain no import that another
    // build of this same library would have to provide: the Rust .so must not
    // import any of the five API symbols (which would mean a wrapper was
    // exported without an implementation behind it).
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&a.rust_so)
        .output()
        .expect("run nm");
    let undefined: BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_owned())
        .collect();
    for api in EXPECTED {
        assert!(
            !undefined.contains(api),
            "the Rust .so imports {api} instead of defining it"
        );
    }
    assert!(
        undefined.contains("printf"),
        "the Rust .so should reach stdout through libc's printf, exactly as the \
         C library does, so that buffering and interleaving match; imports were: \
         {undefined:?}"
    );
}

/// The task's literal requirement: load BOTH objects into one process with
/// `libloading` and resolve every exported symbol from each. `dlopen` defaults to
/// `RTLD_LOCAL`, so the identically named symbols do not interpose; each handle
/// yields its own implementation.
///
/// Only side-effect-free calls are made here (`printLine(NULL)` prints nothing,
/// and the guarded error branches print to the inherited stdout, which the test
/// harness captures) — the byte-exact comparisons live in the phase B/C suites,
/// which use a child process so that a crashing `bad()` cannot take the test
/// runner with it.
#[test]
fn parity_04_dual_load_all_symbols_resolve_and_are_distinct() {
    let a = common::artifacts();
    unsafe {
        let c_lib = libloading::Library::new(&a.c_so).expect("dlopen C .so");
        let rust_lib = libloading::Library::new(&a.rust_so).expect("dlopen Rust .so");

        type FnPtr = unsafe extern "C" fn(*const c_char);
        type FnInt = unsafe extern "C" fn(c_int);
        type FnIntInt = unsafe extern "C" fn(c_int, c_int);

        // Every documented symbol must resolve from BOTH handles.
        for name in ["printLine"] {
            let n = format!("{name}\0");
            let c: libloading::Symbol<FnPtr> =
                c_lib.get(n.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}"));
            let r: libloading::Symbol<FnPtr> = rust_lib
                .get(n.as_bytes())
                .unwrap_or_else(|e| panic!("Rust {name}: {e}"));
            assert_ne!(
                *c as usize, *r as usize,
                "{name} resolved to the same address from both handles — one \
                 object interposed on the other and the differential test would \
                 be comparing a library with itself"
            );
            // NULL is the one input guaranteed to have no side effect at all.
            c(std::ptr::null());
            r(std::ptr::null());
        }

        for name in ["printIntLine", "bad", "good"] {
            let n = format!("{name}\0");
            let c: libloading::Symbol<FnInt> =
                c_lib.get(n.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}"));
            let r: libloading::Symbol<FnInt> = rust_lib
                .get(n.as_bytes())
                .unwrap_or_else(|e| panic!("Rust {name}: {e}"));
            assert_ne!(*c as usize, *r as usize, "{name} interposed");
        }

        for name in ["driver"] {
            let n = format!("{name}\0");
            let c: libloading::Symbol<FnIntInt> =
                c_lib.get(n.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}"));
            let r: libloading::Symbol<FnIntInt> = rust_lib
                .get(n.as_bytes())
                .unwrap_or_else(|e| panic!("Rust {name}: {e}"));
            assert_ne!(*c as usize, *r as usize, "{name} interposed");
        }

        // The `static` helpers must not be reachable through either handle.
        for name in ["goodG2B\0", "goodB2G\0"] {
            assert!(
                c_lib.get::<FnInt>(name.as_bytes()).is_err(),
                "C .so exposes {name:?}"
            );
            assert!(
                rust_lib.get::<FnInt>(name.as_bytes()).is_err(),
                "Rust .so exposes {name:?}"
            );
        }
    }
}

/// `SYMBOLS.md` claims the crate has no feature flags, which is what makes
/// "every feature combination" a single configuration. Verify that against the
/// manifest instead of trusting the prose.
#[test]
fn parity_05_manifest_declares_no_features() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml now has a [features] table: phases B and C must be re-run \
         for every combination, and SYMBOLS.md/CONFIGS.md updated"
    );
}
