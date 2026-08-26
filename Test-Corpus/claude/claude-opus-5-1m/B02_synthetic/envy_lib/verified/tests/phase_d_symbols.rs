// Phase D — symbol parity between the C and the Rust shared object.
//
//  * every dynamic symbol DEFINED by the C `.so` must also be defined by the
//    Rust `.so`, under the exact same name;
//  * every one of them must be reachable through `dlsym`;
//  * the Rust `.so` must have no unresolved (non-libc) symbols, which is proven
//    by `dlopen`ing it with `RTLD_NOW` (eager binding of every relocation).

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const RTLD_NOW: i32 = 2;
const RTLD_LOCAL: i32 = 0;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
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
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep the "real" API symbols: text (T/t), data (D/d), bss (B/b),
            // read-only data (R/r), indirect functions (i) and weak (W/V).
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn phase_d_symbol_parity() {
    let _guard = GLOBAL.lock().unwrap();
    let c_so = c_so_path();
    let rust_so = rust_so_path();
    println!("C   : {}", c_so.display());
    println!("RUST: {}", rust_so.display());

    // ---- 1. dlopen both with RTLD_NOW: no unresolved symbols anywhere ------
    for so in [&c_so, &rust_so] {
        let lib = unsafe {
            libloading::os::unix::Library::open(Some(so), RTLD_NOW | RTLD_LOCAL)
        };
        let lib = lib.unwrap_or_else(|e| {
            panic!(
                "dlopen({}, RTLD_NOW) failed - unresolved symbols: {e}",
                so.display()
            )
        });
        std::mem::forget(lib);
    }
    println!("RTLD_NOW dlopen: both shared objects resolve every relocation");

    // ---- 2. nm -D symbol sets ---------------------------------------------
    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);
    assert!(
        !c_syms.is_empty(),
        "nm reported no defined symbols for the C library"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    println!("C defined symbols   : {}", c_syms.len());
    println!("Rust defined symbols: {}", rust_syms.len());
    println!("C symbols           : {c_syms:?}");
    println!("missing in Rust     : {missing:?}");
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} symbol(s) that the C .so exports: {missing:?}",
        missing.len()
    );

    // ---- 3. every C symbol must be usable through dlsym -------------------
    let (c, r) = load_impls();
    for imp in [&c, &r] {
        // `load_impls` already resolved all five function pointers; make sure
        // none of them is NULL and that the two libraries really are distinct
        // objects.
        assert!(imp.envy as usize != 0);
        assert!(imp.parse_env_numeric as usize != 0);
        assert!(imp.init_config_from_env as usize != 0);
        assert!(imp.perform_operation as usize != 0);
        assert!(imp.apply_bit_operations as usize != 0);
    }
    assert_ne!(
        c.envy as usize, r.envy as usize,
        "the C and Rust `envy` resolved to the same address"
    );

    // ---- 4. the exported names are exactly the five from lib.c ------------
    let expected: BTreeSet<String> = [
        "envy",
        "parse_env_numeric",
        "init_config_from_env",
        "perform_operation",
        "apply_bit_operations",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_syms, expected,
        "the C library's exported surface changed; SYMBOLS.md must be updated"
    );
    println!("symbol parity: OK (0 missing)");
}
