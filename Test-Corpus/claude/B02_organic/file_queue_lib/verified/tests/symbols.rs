//! Phase D — exported symbol parity between the C and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    let cs = defined_symbols(&c_so_path());
    let rs = defined_symbols(&rust_so_path());

    let expected: BTreeSet<String> = [
        "FreeAlertData",
        "GetAlertData",
        "Init_FileQueue",
        "Read_FileMon",
        "driver",
        "merror",
        "os_calloc",
        "os_realloc",
        "os_strdup",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(cs, expected, "C .so exports changed unexpectedly");

    let missing: Vec<_> = cs.difference(&rs).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}"
    );

    // The Rust cdylib legitimately exports nothing else of its own.
    let extra: Vec<_> = rs.difference(&cs).cloned().collect();
    assert!(extra.is_empty(), "unexpected extra Rust exports: {extra:?}");
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let und = undefined_symbols(&rust_so_path());
    let allowed_prefix = ["_Unwind_", "_ITM_", "__cxa_", "__gmon_start__"];
    let leftovers: Vec<_> = und
        .iter()
        .filter(|s| {
            // Everything resolved from libc carries a @GLIBC_ version tag.
            !s.contains("@GLIBC_")
                && !s.contains("@GCC_")
                && !allowed_prefix.iter().any(|p| s.starts_with(p))
        })
        .cloned()
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has unresolved non-libc symbols: {leftovers:?}"
    );
}

/// The harness mirrors of the C structs must agree with the C ABI; the Rust
/// crate asserts its own sizes at compile time (`const _: () = assert!(...)`),
/// and these are the numbers the C compiler produces on x86_64 glibc.
#[test]
fn struct_sizes_match_c_abi() {
    assert_eq!(size_of::<stat>(), 144, "struct stat");
    assert_eq!(size_of::<Tm>(), 56, "struct tm");
    assert_eq!(size_of::<AlertData>(), 96, "alert_data");
    assert_eq!(size_of::<FileQueue>(), 440, "file_queue");
    assert_eq!(align_of::<FileQueue>(), 8);

    // Field offsets that the differential comparisons rely on.
    let q = FileQueue::zeroed();
    let base = &q as *const FileQueue as usize;
    assert_eq!(&q.last_change as *const _ as usize - base, 0);
    assert_eq!(&q.year as *const _ as usize - base, 8);
    assert_eq!(&q.day as *const _ as usize - base, 12);
    assert_eq!(&q.flags as *const _ as usize - base, 16);
    assert_eq!(&q.mon as *const _ as usize - base, 20);
    assert_eq!(&q.file_name as *const _ as usize - base, 24);
    assert_eq!(&q.fp as *const _ as usize - base, 288);
    assert_eq!(&q.f_status as *const _ as usize - base, 296);
}

/// Both `.so`s load and every symbol resolves through `dlsym`.
#[test]
fn both_libraries_load_and_resolve() {
    let (c, r) = apis();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    assert_ne!(c.driver as usize, r.driver as usize);
    assert_ne!(c.GetAlertData as usize, r.GetAlertData as usize);
}
