//! Phase D - symbol parity between the C `.so` and the Rust `.so`, enforced as a
//! test so it cannot silently rot. Every symbol the C library exports must be
//! exported by the Rust library under the exact same name, and the exported data
//! objects must have identical sizes.

mod common;
use common::*;
use std::collections::BTreeMap;
use std::process::Command;

/// `nm -D --defined-only -S <so>` -> {name: (kind, size)}
fn defined_symbols(so: &std::path::Path) -> BTreeMap<String, (String, Option<u64>)> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "-S", "--"])
        .arg(so)
        .output()
        .expect("run nm (binutils must be installed)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <size> <kind> <name>" or "<addr> <kind> <name>"
        let (kind, name, size) = match f.len() {
            4 => (f[2], f[3], u64::from_str_radix(f[1], 16).ok()),
            3 => (f[1], f[2], None),
            _ => continue,
        };
        map.insert(name.to_string(), (kind.to_string(), size));
    }
    map
}

/// Symbols the ELF/linker/runtime adds; not part of the translated surface.
fn is_toolchain_symbol(n: &str) -> bool {
    matches!(
        n,
        "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "__bss_start__" | "_bss_end__"
            | "__bss_end__" | "__end__" | "_edata__" | "__data_start" | "data_start"
            | "__dso_handle" | "_DYNAMIC" | "_GLOBAL_OFFSET_TABLE_"
    ) || n.starts_with("__gnu")
        || n.starts_with("_ITM_")
        || n.starts_with("__cxa")
        || n.starts_with("rust_eh_")
        || n.starts_with("__rust_")
        || n.starts_with("_Unwind")
}

/// The 13 symbols `c_src/src/lib.c` defines with external linkage.
const EXPECTED: &[(&str, &str)] = &[
    ("add_op", "T"),
    ("multiply_op", "T"),
    ("subtract_op", "T"),
    ("divide_op", "T"),
    ("modulo_op", "T"),
    ("find_node_by_id", "T"),
    ("add_tree_node", "T"),
    ("calculate_tree_sum", "T"),
    ("parse_operation", "T"),
    ("get_operation_func", "T"),
    ("inreftree", "T"),
    ("node_table", "B"),
    ("node_count", "B"),
];

#[test]
fn phase_d_rust_so_exports_every_c_symbol() {
    with_libs(|p| {
        let c = defined_symbols(&p.c.path);
        let r = defined_symbols(&p.rust.path);

        let missing: Vec<&String> = c
            .keys()
            .filter(|n| !is_toolchain_symbol(n) && !r.contains_key(*n))
            .collect();
        assert!(
            missing.is_empty(),
            "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
             (per SYMBOLS.md: add the #[no_mangle] wrapper, or translate the missing C source)",
            missing.len()
        );

        // every symbol lib.c defines must actually be there, in both
        for (name, kind) in EXPECTED {
            let cs = c.get(*name).unwrap_or_else(|| panic!("C .so lacks {name}"));
            let rs = r.get(*name).unwrap_or_else(|| panic!("Rust .so lacks {name}"));
            assert_eq!(&cs.0, kind, "{name}: unexpected kind in the C .so");
            assert_eq!(&rs.0, kind, "{name}: kind mismatch (C={} Rust={})", cs.0, rs.0);
        }

        // exported data objects must have identical sizes
        assert_eq!(
            c["node_table"].1,
            Some(NODE_TABLE_BYTES as u64),
            "C node_table size"
        );
        assert_eq!(r["node_table"].1, c["node_table"].1, "node_table size mismatch");
        assert_eq!(c["node_count"].1, Some(4), "C node_count size");
        assert_eq!(r["node_count"].1, c["node_count"].1, "node_count size mismatch");

        println!(
            "symbol parity OK: {} C symbols, {} present in Rust, 0 missing",
            c.len(),
            c.keys().filter(|n| r.contains_key(*n)).count()
        );
    });
}

#[test]
fn phase_d_no_unresolved_symbols() {
    with_libs(|p| {
        for so in [&p.c.path, &p.rust.path] {
            let out = Command::new("ldd").arg("-r").arg(so).output().expect("run ldd");
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                !text.contains("undefined symbol") && !text.contains("not found"),
                "{} has unresolved symbols:\n{text}",
                so.display()
            );
        }
    });
}

/// The Rust `.so` must not export a stub. Every exported function has to be
/// reachable and produce a value (a stub that panicked or aborted would have been
/// caught by Phases B/C, but assert the trivial liveness here too).
#[test]
fn phase_d_every_export_is_callable() {
    with_libs(|p| {
        for lib in [&p.c, &p.rust] {
            assert_eq!((lib.add_op)(2, 3, 0, 0), 5, "{}: add_op", lib.name);
            assert_eq!((lib.multiply_op)(2, 3, 0, 0), 6, "{}: multiply_op", lib.name);
            assert_eq!((lib.subtract_op)(2, 3, 0, 0), -1, "{}: subtract_op", lib.name);
            assert_eq!((lib.divide_op)(7, 2, 0, 0), 3, "{}: divide_op", lib.name);
            assert_eq!((lib.modulo_op)(7, 2, 0, 0), 1, "{}: modulo_op", lib.name);
            lib.reset();
            assert_eq!(lib.add_node(1, 5, -1, b"root"), 0, "{}: add_tree_node", lib.name);
            assert_eq!(lib.find_index(1), Some(0), "{}: find_node_by_id", lib.name);
            assert_eq!((lib.calculate_tree_sum)(1), 5, "{}: calculate_tree_sum", lib.name);
            assert_eq!(lib.parse_op(b"*"), OP_MULTIPLY, "{}: parse_operation", lib.name);
            assert_ne!(
                (lib.get_operation_func)(OP_MODULO) as usize,
                0,
                "{}: get_operation_func",
                lib.name
            );
            assert_eq!((lib.inreftree)(1, 2, 3, 4), 8, "{}: inreftree", lib.name);
            assert_eq!(lib.get_count(), 4, "{}: node_count", lib.name);
            assert_eq!(lib.table_image().len(), NODE_TABLE_BYTES, "{}: node_table", lib.name);
        }
    });
}
