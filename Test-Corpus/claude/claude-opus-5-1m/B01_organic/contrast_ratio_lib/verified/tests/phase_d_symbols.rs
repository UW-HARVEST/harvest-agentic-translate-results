//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforced as a test so it cannot silently regress. Uses `nm -D` when
//! available, and always verifies loadability of every C-exported symbol from
//! the Rust `.so` via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let kind = it.next().unwrap_or("");
        // Skip the standard CRT / glibc scaffolding that every shared object
        // gets from the linker rather than from the translated source.
        const SCAFFOLD: &[&str] = &[
            "_init",
            "_fini",
            "_edata",
            "_end",
            "__bss_start",
            "__bss_start__",
            "_bss_end__",
            "__end__",
            "__data_start",
            "__dso_handle",
            "_IO_stdin_used",
            "__TMC_END__",
            "_DYNAMIC",
            "_GLOBAL_OFFSET_TABLE_",
        ];
        if SCAFFOLD.contains(&name) {
            continue;
        }
        // Only code / data definitions.
        if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "V" | "v") {
            set.insert(format!("{name} {kind}"));
        }
    }
    Some(set)
}

fn names_only(set: &BTreeSet<String>) -> BTreeSet<String> {
    set.iter()
        .map(|s| s.split_whitespace().next().unwrap().to_string())
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let p = pair();
    let c = match nm_defined(&p.c.path) {
        Some(s) => s,
        None => {
            eprintln!("`nm` unavailable — skipping the nm-based comparison");
            return;
        }
    };
    let r = nm_defined(&p.rust.path).expect("nm on the Rust .so");

    let cn = names_only(&c);
    let rn = names_only(&r);

    let missing: Vec<&String> = cn.difference(&rn).collect();
    eprintln!("C   exports ({}): {:?}", cn.len(), cn);
    eprintln!("Rust exports ({}): {:?}", rn.len(), rn);

    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // The C's only public symbol must be present and be a text (code) symbol.
    assert!(
        c.contains("contrast_ratio T"),
        "unexpected C symbol surface: {c:?}"
    );
    assert!(
        r.contains("contrast_ratio T"),
        "`contrast_ratio` is not a global text symbol in the Rust .so: {r:?}"
    );

    // The `static` C helpers must NOT be part of either dynamic surface.
    for hidden in ["cbLuminance", "cbContrastRatio"] {
        assert!(
            !cn.contains(hidden),
            "unexpected: {hidden} is a dynamic symbol of the C .so"
        );
        assert!(
            !rn.contains(hidden),
            "the Rust .so must not export the C `static` helper {hidden}"
        );
    }
}

#[test]
fn d2_rust_so_has_no_unresolved_non_libc_symbols() {
    let p = pair();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&p.rust.path)
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => {
            eprintln!("`nm` unavailable — skipping");
            return;
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut unresolved = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let kind = it.next().unwrap_or("");
        if kind == "w" || kind == "v" {
            continue; // weak undefined — fine
        }
        let base = name.split('@').next().unwrap();
        // Anything carrying a symbol-version tag (`@GLIBC_x.y`, `@GCC_x.y`) is
        // satisfied by the platform C library / libgcc, exactly like the C
        // `.so`'s own `pow@GLIBC_2.29`. Anything else must be a compiler
        // builtin, and must not be a leftover reference to untranslated code.
        let versioned = name.contains("@GLIBC_")
            || name.contains("@GCC_")
            || name.contains("@GLIBCXX_")
            || name.contains("@CXXABI_");
        let is_platform = versioned
            || base.starts_with("__")
            || base.starts_with("_ITM_")
            || base.starts_with("_Unwind_")
            || base.starts_with("_dl_")
            || base.starts_with("pthread_")
            || base.starts_with("dl");
        if !is_platform {
            unresolved.push(name.to_string());
        }
    }
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unresolved:?}"
    );

    // The Rust `.so` must import the SAME libm `pow` the C uses; a private
    // reimplementation would not be bit-identical.
    assert!(
        text.contains("pow"),
        "the Rust .so does not import libm `pow` — f64::powf must lower to the \
         same glibc entry point the C `pow(...)` call uses.\n{text}"
    );
}

/// Every symbol the C `.so` exports must also be resolvable from the Rust `.so`
/// via `dlsym`, which is the check that actually matters to a consumer that
/// swaps one library for the other.
#[test]
fn d3_all_c_symbols_resolvable_from_rust_via_dlsym() {
    let p = pair();
    let c_syms = match nm_defined(&p.c.path) {
        Some(s) => names_only(&s),
        None => ["contrast_ratio".to_string()].into_iter().collect(),
    };
    let lib = unsafe { libloading::Library::new(&p.rust.path) }.expect("dlopen Rust .so");
    for sym in &c_syms {
        let mut name = sym.clone().into_bytes();
        name.push(0);
        let found = unsafe { lib.get::<*const ()>(&name) };
        assert!(
            found.is_ok(),
            "dlsym(\"{sym}\") failed on the Rust .so: {:?}",
            found.err()
        );
    }
    assert!(c_syms.contains("contrast_ratio"));
}
