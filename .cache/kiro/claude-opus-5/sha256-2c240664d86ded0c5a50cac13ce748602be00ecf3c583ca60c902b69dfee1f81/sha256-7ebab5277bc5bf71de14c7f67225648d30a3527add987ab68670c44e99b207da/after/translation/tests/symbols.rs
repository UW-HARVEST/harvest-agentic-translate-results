//! Level 0: exported symbols and exported data tables.

mod common;

use common::*;

/// Every table the C `.so` exports must exist in the Rust `.so` with identical
/// bytes.
#[test]
fn exported_tables_match() {
    let p = pair();
    let tables: &[(&[u8], usize)] = &[
        (b"cp_fixed_table\0", 288 + 32),
        (b"cp_permutation_order\0", 19),
        (b"cp_len_extra_bits\0", 29 + 2),
        (b"cp_len_base\0", (29 + 2) * 4),
        (b"cp_dist_extra_bits\0", 30 + 2),
        (b"cp_dist_base\0", (30 + 2) * 4),
    ];
    for (sym, len) in tables {
        let a = p.c.sym_ptr(sym);
        let b = p.rs.sym_ptr(sym);
        let sa = unsafe { std::slice::from_raw_parts(a, *len) };
        let sb = unsafe { std::slice::from_raw_parts(b, *len) };
        let name = String::from_utf8_lossy(&sym[..sym.len() - 1]).to_string();
        for i in 0..*len {
            assert_eq!(
                sa[i], sb[i],
                "{name}: byte {i} differs (C={:#04x} Rust={:#04x})",
                sa[i], sb[i]
            );
        }
    }
}

#[test]
fn error_reason_symbol_is_writable_pointer() {
    let p = pair();
    p.c.set_error_reason_null();
    p.rs.set_error_reason_null();
    assert!(p.c.error_reason().is_none());
    assert!(p.rs.error_reason().is_none());
}

/// `nm -D --defined-only` on both libraries: every symbol exported by the C
/// library must also be exported by the Rust library.
#[test]
fn dynamic_symbols_superset() {
    fn dynsyms(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("failed to run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c_so = std::env::var("C_SO").map(std::path::PathBuf::from).ok();
    let rs_so = std::env::var("RUST_SO").map(std::path::PathBuf::from).ok();
    let (c_so, rs_so) = match (c_so, rs_so) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            // Fall back to the same discovery logic the loader uses.
            let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let build = root.parent().unwrap().join("c_src").join("build");
            let c = std::fs::read_dir(&build)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
                .expect("C .so not built");
            let mut r = root.join("target").join("release").join("libload_png_mem_lib.so");
            if !r.exists() {
                r = root.join("target").join("debug").join("libload_png_mem_lib.so");
            }
            (c, r)
        }
    };

    let c_syms = dynsyms(&c_so);
    let rs_syms = dynsyms(&rs_so);
    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}
