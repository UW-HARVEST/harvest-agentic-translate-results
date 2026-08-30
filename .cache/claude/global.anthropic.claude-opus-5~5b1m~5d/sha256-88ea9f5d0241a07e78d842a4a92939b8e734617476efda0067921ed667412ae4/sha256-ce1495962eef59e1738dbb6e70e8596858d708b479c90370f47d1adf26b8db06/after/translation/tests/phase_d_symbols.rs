// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every dynamic symbol the C library exports must also be exported by the Rust
// library under the exact same name, and must be callable through `dlsym`.

mod common;

use common::*;
use std::process::Command;

fn dynamic_defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .filter(|s| {
            // Ignore linker/runtime bookkeeping symbols that are not part of
            // the library's own API surface.
            !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__")
                && !s.starts_with("_edata")
                && !s.starts_with("_end")
                && !s.starts_with("_ITM_")
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = dynamic_defined_symbols(&c_lib_path());
    let r = dynamic_defined_symbols(&rust_lib_path());
    assert!(!c.is_empty(), "no symbols found in the C library");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C:    {c:?}\nRust: {r:?}"
    );

    // The set must be exactly equal for this library.
    assert_eq!(c, r, "symbol sets differ");
}

#[test]
fn d2_all_symbols_resolve_via_dlsym_in_both_libraries() {
    let l = libs();
    for name in dynamic_defined_symbols(&c_lib_path()) {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        let in_c: Result<libloading::Symbol<*const ()>, _> = unsafe { l.c.get(&sym) };
        let in_rs: Result<libloading::Symbol<*const ()>, _> = unsafe { l.rs.get(&sym) };
        assert!(in_c.is_ok(), "`{name}` not resolvable in the C library");
        assert!(in_rs.is_ok(), "`{name}` not resolvable in the Rust library");
    }
}

/// Every undefined (imported) symbol of the Rust `.so` must be satisfiable by
/// the system libraries it links against, exactly like the C `.so`: no
/// non-libc/non-runtime symbol may be left dangling. `ldd -r` performs the full
/// data+function relocation check and reports anything unresolved.
#[test]
fn d3_rust_library_has_no_undefined_non_libc_symbols() {
    for (label, path) in [("C", c_lib_path()), ("Rust", rust_lib_path())] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&path)
            .output()
            .expect("run ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let unresolved: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            unresolved.is_empty(),
            "{label} .so ({path:?}) has unresolved symbols:\n{}",
            unresolved.join("\n")
        );
    }

    // The Rust library must produce its output through the C runtime's stdout,
    // i.e. it must import one of libc's stdout writers. (Both LLVM and GCC are
    // free to rewrite `printf("%s\n", s)` into `puts(s)`, which emits the exact
    // same bytes on the same stream, so either import is acceptable.)
    let undef = |path: std::path::PathBuf| -> Vec<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--undefined-only")
            .arg(path)
            .output()
            .expect("run nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .map(|s| s.split('@').next().unwrap_or("").to_string())
            .collect()
    };
    let writers = ["printf", "puts", "fputs", "fwrite", "vfprintf", "fprintf"];
    for (label, path) in [("C", c_lib_path()), ("Rust", rust_lib_path())] {
        let imports = undef(path);
        assert!(
            writers.iter().any(|w| imports.iter().any(|s| s == w)),
            "{label} .so imports no libc stdout writer; imports: {imports:?}"
        );
    }
}
