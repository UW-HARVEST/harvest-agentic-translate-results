//! Step 8: exported-symbol parity.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, with a compatible symbol type (function vs
//! object) and, for objects, the same size.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
struct Sym {
    kind: char,
    size: u64,
}

fn dynamic_symbols(path: &PathBuf) -> BTreeMap<String, Sym> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "-S"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "<addr> [<size>] <type> <name>"
        let (kind, name, size) = match f.len() {
            4 => (
                f[2].chars().next().unwrap(),
                f[3].to_string(),
                u64::from_str_radix(f[1], 16).unwrap_or(0),
            ),
            3 => (f[1].chars().next().unwrap(), f[2].to_string(), 0),
            _ => continue,
        };
        // Skip linker/runtime bookkeeping that is not part of the API.
        if matches!(
            name.as_str(),
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
        ) || name.starts_with("__cxa")
            || name.starts_with("_ITM_")
            || name.starts_with("__gmon")
        {
            continue;
        }
        map.insert(name, Sym { kind, size });
    }
    map
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let p = common::load();
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();

    let c_syms = dynamic_symbols(&c_path);
    let r_syms = dynamic_symbols(&r_path);
    let _ = p;

    assert!(
        !c_syms.is_empty(),
        "no symbols read from {}",
        c_path.display()
    );

    let mut missing = Vec::new();
    let mut mismatched = Vec::new();
    for (name, c_sym) in &c_syms {
        match r_syms.get(name) {
            None => missing.push(name.clone()),
            Some(r_sym) => {
                // 'T' = text/function, 'B'/'D' = data object.
                let c_is_fn = c_sym.kind == 'T' || c_sym.kind == 'W' || c_sym.kind == 'i';
                let r_is_fn = r_sym.kind == 'T' || r_sym.kind == 'W' || r_sym.kind == 'i';
                if c_is_fn != r_is_fn {
                    mismatched.push(format!(
                        "{name}: C kind {} vs Rust kind {}",
                        c_sym.kind, r_sym.kind
                    ));
                } else if !c_is_fn && c_sym.size != 0 && r_sym.size != 0 && c_sym.size != r_sym.size
                {
                    mismatched.push(format!(
                        "{name}: C object size {} vs Rust object size {}",
                        c_sym.size, r_sym.size
                    ));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );
    assert!(
        mismatched.is_empty(),
        "symbol kind/size mismatches: {mismatched:?}"
    );

    // Sanity: the known public API is actually present, so a silently empty
    // comparison cannot pass this test.
    for expected in [
        "inreftree",
        "add_op",
        "multiply_op",
        "subtract_op",
        "divide_op",
        "modulo_op",
        "find_node_by_id",
        "add_tree_node",
        "calculate_tree_sum",
        "parse_operation",
        "get_operation_func",
        "node_table",
        "node_count",
    ] {
        assert!(c_syms.contains_key(expected), "C is missing {expected}");
        assert!(r_syms.contains_key(expected), "Rust is missing {expected}");
    }
}

#[test]
fn global_object_sizes_match_c_layout() {
    let p = common::load();
    let c_syms = dynamic_symbols(&common::c_so_path());
    let r_syms = dynamic_symbols(&common::rust_so_path());
    let _ = p;

    let expect_size = |name: &str, want: u64| {
        let c = &c_syms[name];
        let r = &r_syms[name];
        assert_eq!(c.size, want, "C {name} size");
        assert_eq!(r.size, want, "Rust {name} size");
    };
    // TreeNode is 5 ints + 32 chars = 52 bytes, no padding needed (align 4).
    expect_size("node_table", 52 * common::MAX_NODES as u64);
    expect_size("node_count", 4);
    assert_eq!(std::mem::size_of::<common::TreeNode>(), 52);
}
