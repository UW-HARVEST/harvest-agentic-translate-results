// Phase D — symbol parity between the C `.so` and the Rust `.so`.
// Re-derives the SYMBOLS.md table at test time so it cannot silently rot.

mod common;
use common::*;

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    libs(); // makes sure both .so files exist / are loadable
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(
        c.contains(&"driver".to_string()) && c.contains(&"run".to_string()),
        "unexpected C symbol set: {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn static_c_functions_are_not_exported_by_either() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    for s in ["add_floor", "add_bedrooms", "print_house", "parse_val"] {
        assert!(
            !c.contains(&s.to_string()),
            "C unexpectedly exports static fn {s}"
        );
        assert!(
            !r.contains(&s.to_string()),
            "Rust exports {s}, but it is `static` in the C source"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Loading the library at all proves the dynamic linker resolved everything.
    let l = libs();
    let mut h = HouseT::canonical();
    let out = capture(|| unsafe { (l.rust.run)(&mut h, 0) });
    assert!(!out.is_empty(), "Rust run() produced no output");

    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    // The libc/runtime imports the C .so also needs must be present.
    for sym in ["printf", "strtol", "__errno_location"] {
        assert!(
            text.contains(sym),
            "Rust .so does not import {sym}; formatting/parsing would not be \
             guaranteed byte-identical.\n{text}"
        );
    }
}

#[test]
fn abi_layout_of_house_t_matches_c() {
    // offsets 0 / 4 / 8, size 16, align 8 -- required for `run`'s ABI
    assert_eq!(std::mem::size_of::<HouseT>(), 16);
    assert_eq!(std::mem::align_of::<HouseT>(), 8);
    let h = HouseT::new(0x0102_0304, 0x0506_0708, 1.0);
    let raw = h.raw();
    assert_eq!(&raw[0..4], &0x0102_0304i32.to_ne_bytes());
    assert_eq!(&raw[4..8], &0x0506_0708i32.to_ne_bytes());
    assert_eq!(&raw[8..16], &1.0f64.to_ne_bytes());
}
