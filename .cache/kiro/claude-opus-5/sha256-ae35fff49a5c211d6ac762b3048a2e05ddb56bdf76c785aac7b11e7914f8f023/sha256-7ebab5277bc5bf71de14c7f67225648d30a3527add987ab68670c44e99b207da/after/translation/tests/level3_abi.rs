//! Level 3: ABI / exported-symbol parity.
//!
//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name (including anything a preprocessor macro
//! could have produced — `lib.h` has no renaming macros here, so the required
//! set is `synth_pair`).

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D --defined-only` output into the set of exported symbol names.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Uppercase type letter == global/external definition.
            if kind.len() == 1 && kind.chars().all(|c| c.is_ascii_uppercase()) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols emitted by the linker/toolchain itself rather than by the source.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "_DYNAMIC"
            | "_GLOBAL_OFFSET_TABLE_"
            | "__TMC_END__"
    ) || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
        || name.starts_with("__gnu")
        || name.starts_with("_ITM_")
        || name.starts_with("__cxa")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_so = c_so();
    let c_syms = exported_symbols(&c_so);

    let required: BTreeSet<String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .cloned()
        .collect();

    // Sanity: the C library must export the documented API, otherwise this test
    // would trivially pass.
    assert!(
        required.contains("synth_pair"),
        "C .so does not export synth_pair; got {c_syms:?}"
    );

    // Check the freshly built `.so` under test and, when present, the artifact
    // that `cargo build --release` produced.
    let mut targets = vec![rust_so().to_path_buf()];
    targets.extend(cargo_release_so());

    for t in targets {
        let r_syms = exported_symbols(&t);
        let missing: Vec<&String> = required.iter().filter(|s| !r_syms.contains(*s)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing C-exported symbols: {missing:?}\nC: {c_syms:?}\nRust: {r_syms:?}",
            t.display()
        );
    }
}

/// The symbol must be loadable and callable through `dlsym` — this exercises
/// the `#[no_mangle]` export wrapper, not a Rust-internal call.
#[test]
fn exported_symbol_is_callable_via_dlsym() {
    let p = Pair::load();
    let mut z = vec![0.0f32; Z_LEN];
    z[448] = 0.5;
    p.check(&z, 2, "dlsym callable");
}

/// Guard against the harness silently testing a stale artifact: the `.so` under
/// test must be newer than `src/lib.rs`.
#[test]
fn rust_so_is_not_stale() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let src_mtime = std::fs::metadata(&src).unwrap().modified().unwrap();
    let so_mtime = std::fs::metadata(rust_so()).unwrap().modified().unwrap();
    assert!(
        so_mtime >= src_mtime,
        "the .so under test ({}) is older than src/lib.rs",
        rust_so().display()
    );
}
