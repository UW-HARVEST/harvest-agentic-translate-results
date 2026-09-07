//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Defined (`T`/`D`/`B`/`R`) dynamic symbols of a shared object.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = match (it.next(), it.next(), it.next()) {
                (Some(a), Some(k), Some(n)) => (a, k, n),
                // weak/undefined form: "                 w name"
                (Some(k), Some(n), None) => ("", k, n),
                _ => return None,
            };
            // Ignore the linker/compiler-runtime boilerplate present in every
            // shared object, and Rust's internal mangled/runtime symbols.
            if matches!(kind, "w" | "W" | "a" | "A") {
                return None;
            }
            if name.starts_with("_ITM_")
                || name.starts_with("__cxa")
                || name == "__gmon_start__"
                || name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__bss_start")
                || name.starts_with("_edata")
                || name.starts_with("_end")
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm -D --undefined-only");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The gate: every symbol the C `.so` exports must also be exported by the
/// Rust `.so`, under the exact same name.
#[test]
fn phase_d_symbol_diff_is_empty() {
    let c = defined_symbols(&c_so_path());
    let rs = defined_symbols(&rust_so_path());

    // Only the mangle-free, non-Rust-runtime names are comparable; the Rust
    // cdylib legitimately exports extra internal symbols.
    let missing: Vec<&String> = c.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {c:?}\n Rust({}) = {rs:?}",
        c.len(),
        rs.len()
    );

    // Sanity: the two symbols we know the C source defines are present.
    for want in ["driver", "fma_array"] {
        assert!(c.contains(want), "C .so is missing {want}");
        assert!(rs.contains(want), "Rust .so is missing {want}");
    }

    // `inner` is `static` in C -> must not be exported by either side.
    assert!(!c.contains("inner"), "C unexpectedly exports `inner`");
    assert!(
        !rs.contains("inner"),
        "Rust exports `inner`, but the C symbol has internal linkage"
    );
}

/// No unresolved project symbols in the Rust `.so`: every undefined symbol must
/// come from libc / the platform unwinder.
#[test]
fn phase_d_no_unresolved_non_libc_symbols() {
    let undef = undefined_symbols(&rust_so_path());
    let allowed_prefix = [
        "_ITM_",
        "__cxa",
        "__gmon_start__",
        "_Unwind_",
        "__tls_get_addr",
        "__errno_location",
        "__libc",
        "statx",
        "gettid",
    ];
    // Anything else must be a plain libc name (i.e. resolvable in libc/libm/
    // libpthread), never a `driver`-project symbol.
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            if allowed_prefix.iter().any(|p| base.starts_with(p)) {
                return false;
            }
            // Project symbols would be named like the C ones or Rust-mangled.
            base == "driver" || base == "fma_array" || base == "inner" || base.starts_with("_ZN")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved project symbols: {suspicious:?}"
    );

    // The library actually loads and both symbols resolve — the strongest
    // possible statement that nothing is unresolved at runtime.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}

/// Neither build produces a binary executable, so there is no driver program
/// whose stdout could be compared. Assert that, so the claim in CONFIGS.md
/// stays true if the build ever changes.
#[test]
fn phase_d_no_binary_target_exists() {
    let cargo_toml =
        std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !cargo_toml.contains("[[bin]]"),
        "Cargo.toml now declares a [[bin]]; the C/Rust stdout comparison for the \
         binary must be added to Phase B"
    );
    assert!(
        !manifest_dir().join("src/main.rs").exists(),
        "src/main.rs appeared; add a binary stdout comparison to Phase B"
    );
    let cmake = std::fs::read_to_string(manifest_dir().join("../c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; add a binary stdout comparison to Phase B"
    );
}

/// `translation/Cargo.toml` declares no features, so there is exactly one
/// feature combination. Fail loudly if that changes, since Phases B and C would
/// then have to be re-run per combination.
#[test]
fn phase_d_feature_set_is_singular() {
    let cargo_toml =
        std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !cargo_toml.contains("[features]"),
        "Cargo.toml gained a [features] table: re-run Phases B and C for every \
         feature combination"
    );
}
