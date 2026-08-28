//! Exported-symbol parity between the C shared library and the Rust cdylib.
//! Kept in its own test binary so it never runs concurrently with the
//! stdout-capturing differential tests.

mod common;

use common::{c_lib_path, rust_lib_path};

// ---------------------------------------------------------------------------
// Exported-symbol parity
// ---------------------------------------------------------------------------

fn dynamic_symbols(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
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
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global/weak text or data symbols only; skip local (lowercase)
            // and compiler/runtime bookkeeping.
            if kind.chars().all(|c| c.is_uppercase()) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = dynamic_symbols(&c_lib_path());
    let r_syms = dynamic_symbols(&rust_lib_path());

    // The C library's own API surface (everything it defines that is not part
    // of the platform's per-DSO boilerplate).
    let boilerplate = [
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
        "__TMC_END__",
        "_DYNAMIC",
        "_GLOBAL_OFFSET_TABLE_",
    ];

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !boilerplate.contains(&s.as_str()))
        .filter(|s| !r_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c_syms:?}\nRust: {r_syms:?}"
    );

    // Sanity: the three documented entry points really are present in both.
    for name in ["cleanup", "print_result", "cleanup_resources"] {
        assert!(c_syms.contains(name), "C .so missing {name}");
        assert!(r_syms.contains(name), "Rust .so missing {name}");
    }
}
