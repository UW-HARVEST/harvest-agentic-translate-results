//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! This is the mechanical check behind `SYMBOLS.md`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

fn undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "-u"], so).into_iter().collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = defined(&common::c_so_path());
    let r = defined(&common::rust_so_path());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} of the C .so's symbols: {missing:?}",
        missing.len()
    );

    // The ten functions of task_manager.h / logger.h / driver.c.
    let expected: BTreeSet<String> = [
        "create_task_manager",
        "add_task",
        "print_tasks",
        "destroy_task_manager",
        "initialize_logger",
        "log_info",
        "log_warning",
        "log_error",
        "finalize_logger",
        "driver",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(c, expected, "C .so export set changed unexpectedly");
    assert_eq!(
        r, expected,
        "Rust .so exports a different set than the C .so"
    );
}

#[test]
fn d2_logger_static_is_not_exported_by_either() {
    // `logger.c`'s `static FILE *log_file` has internal linkage; the Rust
    // `static mut LOG_FILE` must likewise stay private.
    for so in [common::c_so_path(), common::rust_so_path()] {
        let d = defined(&so);
        assert!(!d.contains("log_file"), "{}", so.display());
        assert!(!d.contains("LOG_FILE"), "{}", so.display());
    }
}

#[test]
fn d3_rust_has_no_unresolved_non_libc_imports() {
    // Everything the Rust cdylib imports must come from libc / libgcc, which
    // `ldd` confirms are its only shared-library dependencies.
    let ldd = Command::new("ldd")
        .arg(common::rust_so_path())
        .output()
        .expect("ldd");
    let ldd = String::from_utf8_lossy(&ldd.stdout);
    for line in ldd.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('/') || line.contains("linux-vdso") {
            continue;
        }
        assert!(
            line.contains("libc.so") || line.contains("libgcc_s.so"),
            "unexpected shared-library dependency: {line}"
        );
    }

    // Every C import must also be an import of the Rust .so (i.e. the Rust
    // build really calls the same libc entry points rather than substituting
    // its own reimplementations).
    let cu: BTreeSet<String> = undefined(&common::c_so_path())
        .into_iter()
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect();
    let ru: BTreeSet<String> = undefined(&common::rust_so_path())
        .into_iter()
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect();
    let missing: Vec<_> = cu.difference(&ru).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not import libc symbols the C .so uses: {missing:?}"
    );
}
