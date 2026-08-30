//! Level 0: the exported surface itself.
//!
//! * every dynamic symbol the C `.so` defines must also be defined by the Rust
//!   `.so`, under the same name;
//! * every exported table must be byte-identical.

mod harness;

use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("running nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(_), Some(name)) => Some(name.to_string()),
                // "         <type> <name>"
                (Some(_), Some(name), None) => Some(name.to_string()),
                _ => None,
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Symbols that only exist because of how the Rust/`cdylib` runtime is put
/// together; they are additions, never omissions, so they do not matter.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__gmon")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__cxa")
        || name.starts_with("rust_")
        || name.starts_with("_R")
        || name.starts_with("__rust")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = harness::c_so_path();
    let rs = harness::rust_so_path();
    let c_syms = defined_dynamic_symbols(&c);
    let rs_syms = defined_dynamic_symbols(&rs);

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s) && !rs_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}\nC symbols: {:?}",
        rs.display(),
        c.display(),
        missing,
        c_syms
    );

    // Sanity: the symbols we actually care about are all there.
    for want in [
        "pinflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ] {
        assert!(c_syms.iter().any(|s| s == want), "C lacks {want}");
        assert!(rs_syms.iter().any(|s| s == want), "Rust lacks {want}");
    }
}

#[test]
fn exported_tables_are_byte_identical() {
    let c = harness::c_impl();
    let rs = harness::rust_impl();
    for t in harness::TABLES {
        let a = c.table_bytes(t.name, t.bytes);
        let b = rs.table_bytes(t.name, t.bytes);
        assert_eq!(
            a,
            b,
            "table {} differs\n  C    = {}\n  Rust = {}",
            t.name,
            harness::hexdump(&a),
            harness::hexdump(&b)
        );
    }
}

#[test]
fn error_reason_starts_null_and_is_writable() {
    for imp in [harness::c_impl(), harness::rust_impl()] {
        let slot = imp.error_reason_slot();
        unsafe {
            assert!((*slot).is_null(), "{}: cp_error_reason not NULL", imp.label);
            *slot = 1 as *const std::ffi::c_char;
            assert_eq!(*slot as usize, 1, "{}: slot not writable", imp.label);
            *slot = std::ptr::null();
        }
    }
}
