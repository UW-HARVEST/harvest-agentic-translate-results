//! Phase D — symbol parity between the C `.so` and the Rust `.so`, asserted
//! mechanically from `nm -D` rather than from a hand-written list.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let _addr = f.next()?;
            let ty = f.next()?;
            let name = f.next()?;
            // Only global text/data symbols; skip toolchain bookkeeping.
            if matches!(ty, "T" | "D" | "B" | "R" | "W" | "V") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|n| {
            !n.starts_with("_init")
                && !n.starts_with("_fini")
                && !n.starts_with("__bss_start")
                && !n.starts_with("_edata")
                && !n.starts_with("_end")
                && !n.starts_with("_ITM_")
                && !n.starts_with("__cxa")
                && !n.starts_with("__gmon")
                && !n.starts_with("_Unwind")
                && !n.starts_with("rust_eh_personality")
        })
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("nm must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under
/// the exact same name. The diff must be empty.
#[test]
fn symbol_parity_is_exact() {
    let p = libs();
    let c = nm_defined(&p.c.path);
    let r = nm_defined(&p.rs.path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {:?}\n  C  ({}): {:?}\n  Rust({}): {:?}",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );

    // The nine documented entry points, spelled out so a silent rename is
    // caught even if `nm` output shape changes.
    for want in [
        "FreeAlertData",
        "GetAlertData",
        "Init_FileQueue",
        "Read_FileMon",
        "driver",
        "merror",
        "os_calloc",
        "os_realloc",
        "os_strdup",
    ] {
        assert!(c.contains(want), "C .so lost {want}");
        assert!(r.contains(want), "Rust .so does not export {want}");
    }
    assert_eq!(c.len(), 9, "unexpected C symbol set: {c:?}");
}

/// The Rust `.so` must not depend on anything beyond libc / the unwinder.
#[test]
fn rust_so_has_no_foreign_undefined_symbols() {
    let p = libs();
    let undef = nm_undefined(&p.rs.path);
    let foreign: Vec<&String> = undef
        .iter()
        .filter(|n| {
            !n.contains("@GLIBC")
                && !n.contains("@GCC")
                && !n.starts_with("_ITM_")
                && !n.starts_with("__gmon_start__")
                && !n.starts_with("_Unwind")
        })
        .collect();
    assert!(
        foreign.is_empty(),
        "Rust .so has non-libc undefined symbols: {foreign:?}"
    );
}

/// The crate declares no cargo features, so the default build is the only
/// configuration. If a feature is ever added this test fails, flagging that
/// Phases B and C must be re-run per combination.
#[test]
fn no_cargo_features_to_permute() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml now declares features; Phases B/C must be run for every \
         combination (see SYMBOLS.md)"
    );
}

/// The C project builds no executable, so there is no binary stdout to diff.
#[test]
fn c_project_builds_no_binary() {
    let cml = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/CMakeLists.txt");
    let text = std::fs::read_to_string(cml).expect("read CMakeLists.txt");
    assert!(!text.contains("add_executable"), "a C binary now exists");
    assert!(text.contains("add_library"));

    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(!manifest.contains("[[bin]]"), "a Rust binary now exists");
}

/// The C ABI layouts the tests assume must match the sizes the C compiler
/// produced; if they did not, every struct comparison above would be reading
/// the wrong bytes.
#[test]
fn abi_layouts_are_as_documented() {
    assert_eq!(size_of::<stat>(), 144);
    assert_eq!(size_of::<tm>(), 56);
    assert_eq!(size_of::<file_queue>(), 440);
    assert_eq!(size_of::<alert_data>(), 96);
    assert_eq!(std::mem::offset_of!(file_queue, fp), 288);
    assert_eq!(std::mem::offset_of!(file_queue, f_status), 296);
    assert_eq!(std::mem::offset_of!(stat, st_mtim), 88);
    assert_eq!(std::mem::offset_of!(alert_data, filename), 88);
}
