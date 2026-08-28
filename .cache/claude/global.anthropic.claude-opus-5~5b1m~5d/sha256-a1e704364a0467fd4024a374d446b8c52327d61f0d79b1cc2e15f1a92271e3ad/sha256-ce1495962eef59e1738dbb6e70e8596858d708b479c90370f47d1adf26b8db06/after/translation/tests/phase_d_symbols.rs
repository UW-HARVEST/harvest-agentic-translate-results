// Phase D -- symbol parity between the C .so and the Rust .so.
mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only` on `path`, returning the set of exported symbol names.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let a = it.next();
        let b = it.next();
        let c = it.next();
        // Either "<addr> <type> <name>" or "<type> <name>" (weak/undefined).
        let (ty, name) = match (a, b, c) {
            (Some(_addr), Some(ty), Some(name)) => (ty, name),
            (Some(ty), Some(name), None) => (ty, name),
            _ => continue,
        };
        // Only real, globally visible code/data definitions.
        if matches!(ty, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i") {
            set.insert(name.to_string());
        }
    }
    set
}

/// Symbols the Rust standard library / compiler runtime unavoidably exports from
/// a `cdylib` and which are not part of the translated surface.
fn is_toolchain_noise(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa")
        || name.starts_with("__gmon")
        || name.starts_with("_Unwind")
        || name.starts_with("_Znw")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
        || name == "gettid"
        || name == "statx"
}

#[test]
fn phase_d_symbol_parity() {
    let c_path = c_so_path();
    let rs_path = rust_so_path();
    eprintln!("C   .so: {}", c_path.display());
    eprintln!("Rust.so: {}", rs_path.display());

    let c_syms: BTreeSet<String> = defined_symbols(&c_path)
        .into_iter()
        .filter(|s| !is_toolchain_noise(s))
        .collect();
    let rs_syms: BTreeSet<String> = defined_symbols(&rs_path)
        .into_iter()
        .filter(|s| !is_toolchain_noise(s))
        .collect();

    eprintln!("C exports {} symbol(s): {:?}", c_syms.len(), c_syms);
    eprintln!("Rust exports {} symbol(s): {:?}", rs_syms.len(), rs_syms);

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );

    // Every symbol the C library defines must be one of the seven translated
    // functions -- this catches the "a whole module was never translated" case
    // from the other direction.
    let expected: BTreeSet<String> = [
        "apply_multiplier",
        "classify_mode",
        "convert_negative_overflow",
        "convert_time_factor",
        "get_modified_time",
        "hash_time_value",
        "modeselect",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_syms, expected,
        "the C library's export set changed; SYMBOLS.md/tests must be updated"
    );
    assert!(
        expected.is_subset(&rs_syms),
        "Rust .so does not export all seven translated functions"
    );
}

/// All seven symbols must be *callable* through `dlsym` with the C ABI --
/// `Lib::open` resolves every one of them and panics otherwise.
#[test]
fn phase_d_all_symbols_resolve_and_are_callable() {
    let l = libs();
    unsafe {
        // One trivial, side-effect-free call per symbol, through the .so exports.
        let _ = (l.c.classify_mode)(b"standard\0".as_ptr() as *const _);
        let _ = (l.rs.classify_mode)(b"standard\0".as_ptr() as *const _);
        eq_int(
            "D-callable/classify_mode",
            "standard",
            (l.c.classify_mode)(b"standard\0".as_ptr() as *const _),
            (l.rs.classify_mode)(b"standard\0".as_ptr() as *const _),
        );
        eq_int(
            "D-callable/apply_multiplier",
            (0xA0, 2),
            (l.c.apply_multiplier)(0xA0, 2),
            (l.rs.apply_multiplier)(0xA0, 2),
        );
        eq_int(
            "D-callable/convert_time_factor",
            1e-4,
            (l.c.convert_time_factor)(1e-4),
            (l.rs.convert_time_factor)(1e-4),
        );
        eq_int(
            "D-callable/convert_negative_overflow",
            1e-7,
            (l.c.convert_negative_overflow)(1e-7),
            (l.rs.convert_negative_overflow)(1e-7),
        );
        eq_i64(
            "D-callable/get_modified_time",
            (1, 1),
            (l.c.get_modified_time)(1, 1),
            (l.rs.get_modified_time)(1, 1),
        );
        eq_int(
            "D-callable/hash_time_value",
            3i64,
            (l.c.hash_time_value)(3),
            (l.rs.hash_time_value)(3),
        );
        let (cr, cout) = capture(|| (l.c.modeselect)(1, 2, 3, 4));
        let (rr, rout) = capture(|| (l.rs.modeselect)(1, 2, 3, 4));
        eq_int("D-callable/modeselect", (1, 2, 3, 4), cr, rr);
        eq_bytes("D-callable/modeselect", (1, 2, 3, 4), &cout, &rout);
    }
}
