//! Phase D — symbol parity and ABI/layout parity between the C `.so` and the
//! Rust `cdylib`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_globals(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (_addr, kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(a), Some(k), Some(n)) => (a, k, n),
            _ => continue,
        };
        // Global text/data symbols only; skip weak and libc/CRT boilerplate.
        if !matches!(kind, "T" | "D" | "B" | "R") {
            continue;
        }
        if name.starts_with("_") || name == "atexit" {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Guard against a degenerate harness: the two `.so`s must be distinct files,
/// living in the C build tree and the Rust target tree respectively, and the
/// function addresses resolved out of them must differ.
#[test]
fn harness_loads_two_distinct_libraries() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    eprintln!("C   .so = {}", c.display());
    eprintln!("Rust.so = {}", r.display());

    assert_ne!(c, r, "same file loaded twice");
    assert!(
        c.components().any(|s| s.as_os_str() == "c_src"),
        "C .so not from c_src/: {}",
        c.display()
    );
    assert!(
        r.components().any(|s| s.as_os_str() == "target")
            && r.file_name().unwrap() == "libflac_validate_lib.so",
        "Rust .so not the crate cdylib: {}",
        r.display()
    );
    // The Rust artifact must match this test binary's profile.
    let want = if cfg!(debug_assertions) { "debug" } else { "release" };
    assert!(
        r.components().any(|s| s.as_os_str() == want),
        "Rust .so profile mismatch: wanted {want}, got {}",
        r.display()
    );

    let ca = *common::c_api().validate as usize;
    let ra = *common::rust_api().validate as usize;
    assert_ne!(ca, ra, "flac_validate resolved to the same address in both");
    let cs = *common::c_api().size_memory as usize;
    let rs = *common::rust_api().size_memory as usize;
    assert_ne!(cs, rs, "tflac_size_memory resolved to the same address");
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = defined_globals(common::c_so_path());
    let r = defined_globals(common::rust_so_path());

    assert!(
        c.contains("flac_validate") && c.contains("tflac_size_memory"),
        "unexpected C symbol set: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust(filtered) = {r:?}"
    );
}

#[test]
fn no_undefined_non_libc_symbols_in_rust() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(common::rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| n.starts_with("tflac_") || n.starts_with("flac_"))
        .collect();
    assert!(bad.is_empty(), "unresolved library symbols in Rust .so: {bad:?}");
}

/// Both `.so`s must agree on `sizeof(struct tflac)` implicitly: the Rust side
/// writes `cur_blocksize` at offset 24 and never touches offsets 21..24. Prove
/// it by driving a struct through both and checking the observable bytes.
#[test]
fn layout_matches_c() {
    assert_eq!(common::STRUCT_SIZE, 28);

    let mut r = common::Raw::zeroed();
    r.set_blocksize(0x0000_1000)
        .set_samplerate(44100)
        .set_channels(2)
        .set_bitdepth(16);
    // sentinel garbage in the padding
    r.0[21] = 0xA1;
    r.0[22] = 0xA2;
    r.0[23] = 0xA3;

    let (rc, out) = common::diff_validate(r);
    assert_eq!(rc, 0);
    // cur_blocksize written at offset 24 (little- or big-endian native)
    assert_eq!(out.cur_blocksize(), 0x1000);
    // padding untouched -> the u32 really lives at 24, not 20
    assert_eq!([out.0[21], out.0[22], out.0[23]], [0xA1, 0xA2, 0xA3]);
    // partition_order at offset 20
    assert_eq!(out.0[20], out.partition_order());
}

/// The project builds no binary/driver, so there is no stdout to compare.
#[test]
fn no_binary_target_to_compare() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root.join("src/main.rs").exists(),
        "a binary target appeared; Phase B must now diff C vs Rust stdout"
    );
    let cmake =
        std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "the C project gained an executable; Phase B must now diff stdout"
    );
}
