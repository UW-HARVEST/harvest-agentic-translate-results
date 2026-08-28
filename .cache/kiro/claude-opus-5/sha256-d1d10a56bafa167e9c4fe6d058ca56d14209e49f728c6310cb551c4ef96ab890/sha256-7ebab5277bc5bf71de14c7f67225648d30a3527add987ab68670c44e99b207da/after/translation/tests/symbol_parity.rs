//! Step 8: the Rust `cdylib` must export every dynamic symbol the C shared
//! library exports, under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined (i.e. exported, not imported) dynamic symbols of a shared object.
///
/// `nm -D --defined-only` lists them; weak toolchain-internal symbols such as
/// `__gmon_start__` or `_ITM_registerTMCloneTable` are undefined placeholders
/// and are filtered out by `--defined-only`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>" or "         <type> <name>"
        let mut parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }
        let name = parts.pop().unwrap().to_string();
        let kind = parts.pop().unwrap();
        // Only global/weak *code or data* definitions; skip the ELF sections
        // and debug entries nm sometimes emits.
        if matches!(kind, "T" | "t" | "W" | "w" | "D" | "d" | "B" | "b" | "R" | "r" | "V" | "v" | "G" | "g" | "S" | "s" | "i" | "u")
        {
            set.insert(name);
        }
    }
    set
}

/// Symbols the C toolchain injects into every shared object; they are not part
/// of the library's API and Rust's `cdylib` has its own equivalents.
fn is_toolchain_internal(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__gmon_start__")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__cxa_")
        || name.starts_with("__bss_start")
        || name.starts_with("_edata")
        || name.starts_with("_end")
        || name.starts_with("__gcc_")
        || name.starts_with("_Jv_")
        || name.starts_with("__odr_asan")
}

#[test]
fn rust_exports_every_c_symbol() {
    let c = common::c_so();
    let r = common::rust_so();
    let c_syms = exported_symbols(&c);
    let r_syms = exported_symbols(&r);

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_internal(s))
        .filter(|s| !r_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}\n\
         C exports: {:?}\nRust exports: {:?}",
        r.display(),
        c.display(),
        missing,
        c_syms,
        r_syms
    );
}

/// The public API of `c_src` is a single function; make sure it really is
/// present in both, so the test above cannot pass vacuously.
#[test]
fn get_predict_func_is_exported_by_both() {
    let c_syms = exported_symbols(&common::c_so());
    let r_syms = exported_symbols(&common::rust_so());
    assert!(
        c_syms.contains("get_predict_func"),
        "C .so does not export get_predict_func; exports: {c_syms:?}"
    );
    assert!(
        r_syms.contains("get_predict_func"),
        "Rust .so does not export get_predict_func; exports: {r_syms:?}"
    );
}

/// The C library's `static` predictors are intentionally *not* exported. The
/// Rust translation must not export them either, or an external caller could
/// bind to a symbol that does not exist in the reference build.
#[test]
fn internal_predictors_stay_private_in_both() {
    let c_syms = exported_symbols(&common::c_so());
    let r_syms = exported_symbols(&common::rust_so());
    let mut internal = vec![
        "BTAC1C2_PredictSample".to_string(),
        "BTAC1C2_GetPredictFunc".to_string(),
    ];
    for n in 0..12 {
        internal.push(format!("BTAC1C2_PredictSample_Pfn{n}"));
    }
    for name in internal {
        assert!(
            !c_syms.contains(&name),
            "unexpected: C .so exports {name} (test assumption wrong)"
        );
        assert!(
            !r_syms.contains(&name),
            "Rust .so exports {name}, but the C .so keeps it static"
        );
    }
}
