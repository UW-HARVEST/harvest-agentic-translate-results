// Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn dynsyms(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Exported code/data only; skip glibc/CRT bookkeeping.
            if !matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "V" | "i") {
                return None;
            }
            if name.starts_with("__") || name.starts_with("_ITM") || name == "_init" || name == "_fini" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();
    let c = dynsyms(&c_path);
    let r = dynsyms(&r_path);

    println!("C   .so: {} ({} symbols)", c_path.display(), c.len());
    println!("Rust.so: {} ({} symbols)", r_path.display(), r.len());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Sanity: the 12 documented entry points really are all there.
    for s in [
        "increment_counter",
        "update_accumulator",
        "apply_operation",
        "add_three",
        "multiply_add",
        "complex_calc",
        "shift_array_data",
        "process_pointer_data",
        "compute_with_dynamic_memory",
        "get_time_based_value",
        "manipulate_records",
        "hatch",
    ] {
        assert!(c.contains(s), "C .so is missing {s}");
        assert!(r.contains(s), "Rust .so is missing {s}");
    }
    assert_eq!(c.len(), 12, "unexpected C symbol set: {c:?}");
}

/// Every undefined symbol of the Rust `.so` must be satisfiable by the system
/// libraries it links (glibc / libgcc / ld.so), exactly like the C `.so`.
/// `ldd -r` performs both data and function relocation resolution and prints
/// `undefined symbol: X` for anything it cannot resolve.
#[test]
fn no_undefined_non_libc_symbols_in_rust_so() {
    for path in [common::c_so_path(), common::rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(&path).output().expect("ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved symbols:\n{}",
            path.display(),
            bad.join("\n")
        );
    }

    // Belt and braces: no undefined symbol may be a Rust-crate symbol (which
    // would mean the cdylib expects to be linked against another Rust crate).
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(common::rust_so_path())
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let leftovers: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| n.starts_with("_ZN") || n.starts_with("_R"))
        .map(|s| s.to_string())
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined Rust-mangled symbols: {leftovers:?}"
    );

    // And the set of libc entry points the C .so needs must be a subset of what
    // the Rust .so imports (same allocator, same memmove, same snprintf).
    let c_undef = undefined_names(&common::c_so_path());
    let r_undef = undefined_names(&common::rust_so_path());
    let libc_core = ["malloc", "free", "memmove", "memset", "time", "difftime", "snprintf"];
    for f in libc_core {
        assert!(c_undef.contains(f), "C .so unexpectedly does not import {f}");
        assert!(
            r_undef.contains(f),
            "Rust .so does not import libc {f} — it must use the same libc primitive"
        );
    }
}

fn undefined_names(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        // strip the @GLIBC_x.y version suffix
        .map(|n| n.split('@').next().unwrap_or(n).to_string())
        .collect()
}

#[test]
fn both_libraries_load_and_agree_on_hidden_state_readback() {
    let l = common::libs();
    l.set_state(0, 0);
    assert_eq!(l.c.read_counter(), 0);
    assert_eq!(l.r.read_counter(), 0);
    assert_eq!(l.c.read_accumulator(), 0);
    assert_eq!(l.r.read_accumulator(), 0);
    // set_state itself is a differential assertion (it cross-checks readback).
    l.set_state(-12345, 987654321);
    l.set_state(i32::MIN, i32::MAX);
    l.reset();
}
