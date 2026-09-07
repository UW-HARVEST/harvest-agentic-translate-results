//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Two independent checks:
//!  1. `nm -D --defined-only` on both, diffed — the C set must be a subset of
//!     the Rust set, with an empty difference.
//!  2. Every C symbol must actually be `dlsym`-resolvable out of the Rust `.so`
//!     (an `nm` entry that cannot be looked up would still be a broken export).
//!
//! Also asserts the Rust `.so` has no undefined non-libc / non-runtime symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .expect("`nm` not available — required for Phase D symbol parity");
    assert!(out.status.success(), "nm failed on {}: {:?}", path.display(), out.status);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn c_defined() -> BTreeSet<String> {
    nm(&c_so_path(), "--defined-only")
}

fn rust_defined() -> BTreeSet<String> {
    nm(&rust_so_path(), "--defined-only")
}

#[test]
fn phase_d_c_symbol_set_is_fully_exported_by_rust() {
    let c = c_defined();
    let r = rust_defined();
    assert!(!c.is_empty(), "no defined symbols found in the C .so — is it built?");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) the C .so exports: {:?}\n\
         C exports {} symbols, Rust exports {} (incl. Rust-internal ones).",
        missing.len(),
        missing,
        c.len(),
        r.len()
    );
}

#[test]
fn phase_d_expected_twelve_public_functions() {
    // Guards against the C .so silently shrinking (e.g. a source file dropped
    // from CMakeLists), which would make the diff above vacuously pass.
    const EXPECTED: [&str; 12] = [
        "add_three",
        "apply_operation",
        "complex_calc",
        "compute_with_dynamic_memory",
        "get_time_based_value",
        "hatch",
        "increment_counter",
        "manipulate_records",
        "multiply_add",
        "process_pointer_data",
        "shift_array_data",
        "update_accumulator",
    ];
    let c = c_defined();
    let r = rust_defined();
    for name in EXPECTED {
        assert!(c.contains(name), "C .so does not export `{name}`");
        assert!(r.contains(name), "Rust .so does not export `{name}`");
    }
    // Every C symbol must be one of the twelve; a new one means the C grew and
    // the translation/tests need extending.
    let unexpected: Vec<&String> = c.iter().filter(|s| !EXPECTED.contains(&s.as_str())).collect();
    assert!(unexpected.is_empty(), "C .so exports unexpected symbols: {unexpected:?}");
}

#[test]
fn phase_d_every_c_symbol_is_dlsym_resolvable_from_rust() {
    let path = rust_so_path();
    let lib = unsafe { libloading::Library::new(&path) }.expect("load Rust .so");
    for name in c_defined() {
        let mut key = name.clone().into_bytes();
        key.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&key) };
        assert!(sym.is_ok(), "`{name}` is in nm output but not dlsym-resolvable from the Rust .so");
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = nm(&rust_so_path(), "--undefined-only");
    // Everything the Rust cdylib may legitimately import: libc, the C runtime,
    // libgcc's unwinder, and the linker's optional weak hooks.
    let allowed_prefix = ["_Unwind_", "__", "_ITM_", "pthread_"];
    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            // A libc import always carries a version tag or is a known bare hook.
            let versioned = s.contains('@');
            let hooked = allowed_prefix.iter().any(|p| base.starts_with(p));
            !(versioned || hooked)
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has unresolved non-libc symbols (a missing translation unit?): {leftovers:?}"
    );
}

#[test]
fn phase_d_project_builds_no_binary_executable() {
    // The stdout-comparison clause of Phase B only applies if there is a driver
    // binary. Assert mechanically that neither build system produces one, so
    // this stays true if the project changes.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();

    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).expect("CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src/CMakeLists.txt now declares an executable — Phase B must also \
         compare the C and Rust binaries' stdout byte-for-byte"
    );

    let cargo = std::fs::read_to_string(root.join("translation/Cargo.toml")).expect("Cargo.toml");
    assert!(!cargo.contains("[[bin]]"), "translation/Cargo.toml now declares a [[bin]] target");
    assert!(
        !root.join("translation/src/main.rs").exists(),
        "translation/src/main.rs now exists — the crate builds a binary"
    );

    let crate_types = cargo
        .lines()
        .find(|l| l.trim_start().starts_with("crate-type"))
        .expect("crate-type must be declared");
    assert!(
        crate_types.contains("cdylib"),
        "the crate must build a cdylib for the differential tests to load it: {crate_types}"
    );
}

#[test]
fn phase_d_no_stub_or_panicking_export() {
    // A symbol that exists but aborts/panics instead of computing is worse than
    // a missing one. Call every export once with benign arguments in a forked
    // child and require a clean exit from BOTH libraries.
    let (_g, p) = locked();
    let mut buf = [1i32, 2, 3, 4, 5, 6, 7, 8];
    let mut recs = [DataRecord::default(); 4];
    let mut scalar = 42i32;

    for (name, run) in [
        ("increment_counter", 0u8),
        ("update_accumulator", 1),
        ("apply_operation", 2),
        ("add_three", 3),
        ("multiply_add", 4),
        ("complex_calc", 5),
        ("shift_array_data", 6),
        ("process_pointer_data", 7),
        ("compute_with_dynamic_memory", 8),
        ("get_time_based_value", 9),
        ("manipulate_records", 10),
        ("hatch", 11),
    ] {
        for lib in [&p.c, &p.r] {
            let o = outcome_of(|| unsafe {
                match run {
                    0 => (lib.increment_counter)(1, 0),
                    1 => (lib.update_accumulator)(1, 0),
                    2 => blackhole((lib.apply_operation)(Some(lib.add_three), 1, 2, 3)),
                    3 => blackhole((lib.add_three)(1, 2, 3)),
                    4 => blackhole((lib.multiply_add)(1, 2, 3)),
                    5 => blackhole((lib.complex_calc)(1, 2, 3)),
                    6 => (lib.shift_array_data)(buf.as_mut_ptr(), 8, 3),
                    7 => blackhole((lib.process_pointer_data)(&mut scalar, 2)),
                    8 => blackhole((lib.compute_with_dynamic_memory)(5, 8)),
                    9 => blackhole((lib.get_time_based_value)(3)),
                    10 => blackhole((lib.manipulate_records)(recs.as_mut_ptr(), 4, 1)),
                    _ => blackhole((lib.hatch)(1, 2, 3, 4)),
                }
            });
            assert_eq!(
                o,
                Outcome::Exited(0),
                "{} `{name}` did not return cleanly on a benign call: {o:?} \
                 (a stub / unimplemented!() export?)",
                lib.tag
            );
        }
    }
}
