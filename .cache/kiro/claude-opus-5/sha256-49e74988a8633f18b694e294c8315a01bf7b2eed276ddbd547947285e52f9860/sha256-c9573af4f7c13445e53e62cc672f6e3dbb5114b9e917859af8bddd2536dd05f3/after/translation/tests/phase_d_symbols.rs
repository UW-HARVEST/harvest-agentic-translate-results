// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Uses `nm -D` when available (authoritative, catches macro-generated names),
// and falls back to `dlsym` probing through libloading otherwise.

#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut set = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // "<addr> <type> <name>" or " <type> <name>"
        let name = line.split_whitespace().last().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        // Ignore linker-synthesised / CRT bookkeeping symbols.
        if name.starts_with("_ITM_")
            || name.starts_with("__cxa")
            || name == "__gmon_start__"
            || name.starts_with("_fini")
            || name.starts_with("_init")
            || name.starts_with("__bss_start")
            || name.starts_with("_edata")
            || name.starts_with("_end")
        {
            continue;
        }
        set.insert(name.to_string());
    }
    Some(set)
}

#[test]
fn phase_d_symbol_parity() {
    let c_path = common::c_so_path();
    let rust_path = common::rust_so_path();

    let (c_syms, rust_syms) = match (nm_defined(&c_path), nm_defined(&rust_path)) {
        (Some(c), Some(r)) => (c, r),
        _ => {
            // `nm` unavailable: fall back to probing the one known export.
            assert!(common::c_exports("driver"));
            assert!(common::rust_exports("driver"));
            return;
        }
    };

    // The C library's complete export set, from its only translation unit.
    assert_eq!(
        c_syms.iter().cloned().collect::<Vec<_>>(),
        vec!["driver".to_string()],
        "the C .so export set changed; SYMBOLS.md/CONFIGS.md must be re-derived"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // Every symbol the C exports must also be reachable via dlsym in Rust.
    for s in &c_syms {
        assert!(
            common::rust_exports(s),
            "`{s}` is in the Rust .so symbol table but not resolvable via dlsym"
        );
    }

    // `static` C functions must remain unexported on both sides.
    assert!(!c_syms.contains("print_hex"));
    assert!(!rust_syms.contains("print_hex"));
}

#[test]
fn phase_d_no_unresolved_rust_imports() {
    let rust_path = common::rust_so_path();
    let out = match Command::new("ldd").arg("-r").arg(&rust_path).output() {
        Ok(o) => o,
        Err(_) => return, // ldd unavailable
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.to_lowercase().contains("undefined symbol"))
        .collect();
    assert!(
        bad.is_empty(),
        "the Rust .so has unresolved imports: {bad:?}"
    );
}
