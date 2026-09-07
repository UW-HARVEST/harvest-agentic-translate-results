//! Phase D — symbol-parity gate.
//!
//! Shells out to `nm -D` on both shared objects and asserts that the set of
//! exported (defined, global) symbols in the C `.so` is a subset of the Rust
//! `.so`'s. This is the automated form of the `SYMBOLS.md` diff, so a
//! regression (a dropped `#[no_mangle]`, an untranslated module) fails CI
//! rather than silently reducing coverage.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build not found — build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.remove(0)
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let profile = deps.parent().unwrap();
    for d in [profile, deps] {
        let p = d.join("libcollided_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libcollided_lib.so not found");
}

/// Defined, globally-visible symbols (`T`/`D`/`B`/`W`/`R`) from `nm -D`.
fn exported(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("`nm` not available");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // "<addr> <kind> <name>"  or  "        <kind> <name>"
            let (kind, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            if matches!(kind, "T" | "t" | "D" | "B" | "W" | "R" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols the C `.so` exports purely as ELF/toolchain boilerplate — they are
/// not part of the library's API and are irrelevant to translation parity.
const ELF_BOILERPLATE: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "__gmon_start__",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__TMC_END__",
    "_DYNAMIC",
    "_GLOBAL_OFFSET_TABLE_",
];

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    let api: BTreeSet<&String> = c
        .iter()
        .filter(|s| !ELF_BOILERPLATE.contains(&s.as_str()))
        .collect();

    assert!(
        !api.is_empty(),
        "no API symbols found in the C .so — the harness is broken"
    );

    let missing: Vec<&&String> = api.iter().filter(|s| !r.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} symbol(s) that the C .so does: {:?}\n\
         C API symbols ({}): {:?}",
        missing.len(),
        missing,
        api.len(),
        api
    );

    // The known-complete API surface, pinned so that a future C addition that
    // is not translated fails here too.
    let expected: BTreeSet<String> = [
        "c2AABBtoAABB",
        "c2CircletoAABB",
        "c2CircletoCircle",
        "c2Clampv",
        "c2Dot",
        "c2Maxv",
        "c2Minv",
        "c2Sub",
        "c2V",
        "collided",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let api_owned: BTreeSet<String> = api.into_iter().cloned().collect();
    assert_eq!(
        api_owned, expected,
        "the C .so's API surface changed; SYMBOLS.md / the translation must be updated"
    );
}

#[test]
fn phase_d_every_symbol_is_callable_through_both_sos() {
    // Resolving each symbol out of each `.so` proves the export is real (not
    // merely present in the symbol table) and that libloading can bind it.
    let l = libs();
    for lib in [&l.c, &l.rs] {
        let _ = lib.c2V(1.0, 2.0);
        let _ = lib.c2Maxv(v(1.0, 2.0), v(3.0, 4.0));
        let _ = lib.c2Minv(v(1.0, 2.0), v(3.0, 4.0));
        let _ = lib.c2Clampv(v(1.0, 2.0), v(0.0, 0.0), v(2.0, 2.0));
        let _ = lib.c2Sub(v(1.0, 2.0), v(3.0, 4.0));
        let _ = lib.c2Dot(v(1.0, 2.0), v(3.0, 4.0));
        let _ = lib.c2CircletoCircle(circle(0.0, 0.0, 1.0), circle(1.0, 1.0, 1.0));
        let _ = lib.c2CircletoAABB(circle(0.0, 0.0, 1.0), aabb(0.0, 0.0, 1.0, 1.0));
        let _ = lib.c2AABBtoAABB(aabb(0.0, 0.0, 1.0, 1.0), aabb(0.0, 0.0, 1.0, 1.0));
        let c0 = circle(0.0, 0.0, 1.0);
        let _ = lib.collided_raw(
            &c0 as *const _ as *const std::ffi::c_void,
            C2_TYPE_CIRCLE,
            &c0 as *const _ as *const std::ffi::c_void,
            C2_TYPE_CIRCLE,
        );
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_imports() {
    // `ldd -r` reports undefined symbols that the loader cannot satisfy. An
    // empty report means every import resolves against the system libraries.
    let out = Command::new("ldd").arg("-r").arg(rust_so()).output();
    if let Ok(out) = out {
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(bad.is_empty(), "Rust .so has unresolved imports: {bad:?}");
    }
}
