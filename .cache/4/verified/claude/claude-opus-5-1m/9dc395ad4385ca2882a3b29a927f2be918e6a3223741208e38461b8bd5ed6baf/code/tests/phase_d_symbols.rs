//! Phase D — symbol parity, translation completeness, and the build-config
//! gate. Everything here is re-derived from the C source and the built shared
//! objects at test time, so SYMBOLS.md / CONFIGS.md cannot silently go stale.

mod common;

use common::*;

/// `RTLD_NOW | RTLD_LOCAL` on Linux: resolve every relocation eagerly.
const RTLD_NOW: std::os::raw::c_int = 2;

/// D1 — every symbol exported by the C `.so` must be exported by the Rust
/// `.so` under the exact same name. The diff must be empty.
#[test]
fn d1_symbol_diff_is_empty() {
    let c = exported_symbols(c_lib());
    let r = exported_symbols(rust_lib());

    assert_eq!(
        c,
        vec!["driver".to_string(), "main".to_string()],
        "the C .so's exported surface is not what SYMBOLS.md records"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {r:?}",
        missing.len()
    );

    // The Rust library must not invent extra public C symbols either: the
    // surfaces are identical here.
    assert_eq!(
        r, c,
        "the Rust .so's exported surface differs from the C .so's"
    );
}

/// D2 — both symbols are reachable through `dlopen`/`dlsym` in both libraries,
/// and the `static` helper is exported by neither.
#[test]
fn d2_symbols_are_reachable_via_dlsym() {
    for imp in [c_impl(), rust_impl()] {
        // `Impl::load` already resolved `driver` and `main`; a failure there
        // panics, so reaching this point proves both are dlsym-able.
        assert!(
            !imp.exports_print_hex(),
            "{}: print_hex must stay internal (it is `static` in C)",
            imp.name
        );
    }
}

/// D3 — the Rust `.so` has no unresolved symbols: loading it with `RTLD_NOW`
/// forces every relocation to be bound immediately, which fails if anything is
/// missing or undefined.
#[test]
fn d3_no_unresolved_symbols() {
    for so in [c_lib(), rust_lib()] {
        let lib = unsafe { libloading::os::unix::Library::open(Some(so), RTLD_NOW) };
        let lib = lib.unwrap_or_else(|e| {
            panic!("{} has unresolved symbols (RTLD_NOW failed): {e}", so.display())
        });
        unsafe {
            lib.get::<unsafe extern "C" fn(std::os::raw::c_int)>(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", so.display()));
            lib.get::<unsafe extern "C" fn() -> std::os::raw::c_int>(b"main\0")
                .unwrap_or_else(|e| panic!("dlsym main in {}: {e}", so.display()));
        }
    }
}

/// D4 — translation completeness: the C project must still consist of exactly
/// the one translation unit that was translated. A new `.c`/`.h` file would
/// mean a whole module is missing from the Rust side.
#[test]
fn d4_no_untranslated_c_sources() {
    let mut found = Vec::new();
    let mut stack = vec![manifest_dir().join("c_src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read c_src") {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                // Ignore build trees produced by this harness.
                if name != "build" && name != "CMakeFiles" {
                    stack.push(path);
                }
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("c") | Some("h")
            ) {
                found.push(name);
            }
        }
    }
    found.sort();
    assert_eq!(
        found,
        vec!["main.c".to_string()],
        "the C project's source set changed; every listed file must have a Rust \
         counterpart before verification can be considered complete"
    );
}

/// D5 — build-configuration gate: the crate has no Cargo features and the C
/// build has no configuration switches, so `{}` (the empty feature set) is the
/// complete matrix that CONFIGS.md claims.
#[test]
fn d5_single_build_configuration() {
    let cargo_toml =
        std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !cargo_toml.contains("[features]"),
        "Cargo.toml now declares [features]; Phases B and C must be repeated \
         for every feature combination and CONFIGS.md updated"
    );

    for rs in ["src/imp.rs", "src/lib.rs", "src/main.rs"] {
        let src = std::fs::read_to_string(manifest_dir().join(rs)).expect("read rust source");
        assert!(
            !src.contains("cfg(feature"),
            "{rs} branches on a Cargo feature that CONFIGS.md does not enumerate"
        );
    }

    let cmake = std::fs::read_to_string(manifest_dir().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    for knob in ["option(", "add_definitions", "target_compile_definitions"] {
        assert!(
            !cmake.contains(knob),
            "c_src/CMakeLists.txt now has a `{knob}` build switch that CONFIGS.md \
             does not enumerate"
        );
    }

    let c_src = std::fs::read_to_string(manifest_dir().join("c_src/src/main.c"))
        .expect("read C source");
    for line in c_src.lines() {
        let t = line.trim_start();
        assert!(
            !(t.starts_with("#if") || t.starts_with("#ifdef") || t.starts_with("#ifndef")),
            "the C source now has conditional compilation: {line:?}"
        );
    }
}
