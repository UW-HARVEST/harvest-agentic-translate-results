//! Step 8: ABI surface parity.
//!
//! Every symbol the C `libdriver.so` exports dynamically must also be exported
//! by the Rust `libdriver.so` under the exact same name, and must be resolvable
//! with `dlsym`.

mod harness;

use std::collections::BTreeSet;
use std::path::Path;

/// Dynamic symbols *defined* by a shared object, as `nm -D --defined-only`
/// reports them.  Linker-synthesised bookkeeping symbols are dropped: they are
/// not part of the library's API and are named differently by every toolchain.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let ignored: &[&str] = &[
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__bss_start__",
        "__bss_end__",
        "_bss_end__",
        "__end__",
        "__data_start",
        "__dso_handle",
        "__TMC_END__",
        "_IO_stdin_used",
    ];
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Keep code and data, skip anything the linker made up.
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "G" | "i")
            {
                return None;
            }
            if ignored.contains(&name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    harness::ensure_built();
    let c = exported_symbols(&harness::c_so_path());
    let r = exported_symbols(&harness::rust_so_path());

    assert!(
        !c.is_empty(),
        "no symbols found in {}",
        harness::c_so_path().display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {:?}",
        missing.len(),
        r.intersection(&c).collect::<Vec<_>>()
    );
}

/// The documented public API from the two headers, spelled out so a silent
/// rename cannot slip through even if the C build changed.
#[test]
fn public_api_is_dlsym_resolvable() {
    harness::ensure_built();
    const API: &[&str] = &[
        // logger.h
        "initialize_logger",
        "log_info",
        "log_warning",
        "log_error",
        "finalize_logger",
        // task_manager.h
        "create_task_manager",
        "add_task",
        "print_tasks",
        "destroy_task_manager",
        // driver.c
        "driver",
    ];
    for so in [harness::c_so_path(), harness::rust_so_path()] {
        let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen");
        for name in API {
            let mut sym = name.as_bytes().to_vec();
            sym.push(0);
            let found = unsafe { lib.get::<*const ()>(&sym) };
            assert!(
                found.is_ok(),
                "{} does not export `{name}`",
                so.display()
            );
        }
        std::mem::forget(lib);
    }
}
