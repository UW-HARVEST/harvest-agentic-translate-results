//! Phase D — dynamic-symbol parity between the two `.so`s, plus `CONFIGS.md`
//! row 28 (the header's `#ifndef` fallbacks).

mod common;

use common::{c_lib_path, repo_root, rust_lib_path, OP_TAG, REPEAT};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` as a set of `name`, and as a set of `type name`.
fn defined_symbols(so: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names = BTreeSet::new();
    let mut typed = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>" for defined symbols.
        if cols.len() >= 3 {
            let (ty, name) = (cols[cols.len() - 2], cols[cols.len() - 1]);
            names.insert(name.to_string());
            typed.insert(format!("{ty} {name}"));
        }
    }
    (names, typed)
}

/// Undefined symbols, minus the weak toolchain hooks every ELF object carries.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// The eight symbols `mdcore.c` contributes to a link.
const EXPECTED: &[&str] = &[
    "G_OP",
    "G_OP_NAME",
    "helper_call",
    "helper_ptr",
    "op_add",
    "op_mul",
    "op_sub",
    "use_generated",
];

#[test]
fn sym_01_defined_symbol_sets_are_identical() {
    let c = c_lib_path();
    let r = rust_lib_path();
    let (c_names, _) = defined_symbols(&c);
    let (r_names, _) = defined_symbols(&r);

    let missing: Vec<&String> = c_names.difference(&r_names).collect();
    assert!(
        missing.is_empty(),
        "[{OP_TAG}/{REPEAT}] Rust .so is missing C symbols: {missing:?}"
    );

    // The Rust cdylib exports nothing beyond the C surface either.
    let extra: Vec<&String> = r_names.difference(&c_names).collect();
    assert!(
        extra.is_empty(),
        "[{OP_TAG}/{REPEAT}] Rust .so exports symbols the C .so does not: {extra:?}"
    );

    assert_eq!(
        c_names,
        EXPECTED.iter().map(|s| s.to_string()).collect(),
        "[{OP_TAG}/{REPEAT}] the C surface changed; update SYMBOLS.md"
    );
}

#[test]
fn sym_02_symbol_kinds_match() {
    // G_OP / G_OP_NAME must be data (`D`), the six functions text (`T`), so a
    // caller reading the globals through dlsym sees the same shape.
    let (_, c_typed) = defined_symbols(&c_lib_path());
    let (_, r_typed) = defined_symbols(&rust_lib_path());
    assert_eq!(
        c_typed, r_typed,
        "[{OP_TAG}/{REPEAT}] nm type letters differ between C and Rust"
    );
    for want in ["D G_OP", "D G_OP_NAME"] {
        assert!(c_typed.contains(want), "C .so: expected `{want}`");
        assert!(r_typed.contains(want), "Rust .so: expected `{want}`");
    }
    for f in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr", "use_generated"] {
        assert!(c_typed.contains(&format!("T {f}")), "C .so: expected `T {f}`");
        assert!(
            r_typed.contains(&format!("T {f}")),
            "Rust .so: expected `T {f}`"
        );
    }
}

#[test]
fn sym_03_no_unresolved_non_libc_symbols() {
    // Mechanical check instead of a hand-maintained allowlist: `ldd -r` performs
    // both data and function relocation resolution against the object's declared
    // dependencies and reports anything it cannot satisfy. A left-behind
    // translation unit would show up here as an undefined symbol.
    for so in [c_lib_path(), rust_lib_path()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&so)
            .output()
            .unwrap_or_else(|e| panic!("run ldd -r on {}: {e}", so.display()));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| {
                let l = l.to_ascii_lowercase();
                l.contains("undefined symbol") || l.contains("not found")
            })
            .collect();
        assert!(
            bad.is_empty(),
            "[{OP_TAG}/{REPEAT}] {} has unresolved symbols:\n{}",
            so.display(),
            bad.join("\n")
        );
    }

    // Every undefined symbol must additionally be either version-tagged against
    // glibc/libgcc or one of the four weak ELF/toolchain hooks - i.e. supplied by
    // the C runtime, never by a module that was never translated.
    for so in [c_lib_path(), rust_lib_path()] {
        for sym in undefined_symbols(&so) {
            let weak_hook = matches!(
                sym.as_str(),
                "_ITM_deregisterTMCloneTable" | "_ITM_registerTMCloneTable" | "__gmon_start__"
            );
            assert!(
                weak_hook || sym.contains("@GLIBC_") || sym.contains("@GCC_"),
                "[{OP_TAG}/{REPEAT}] {} imports {sym:?}, which is neither a C-runtime \
                 symbol nor a weak toolchain hook",
                so.display()
            );
        }
    }
}

#[test]
fn sym_04_every_symbol_is_reachable_not_a_stub() {
    // Each exported symbol must actually run translated logic: dlsym it and check
    // it produces the C result, so a symbol added only to satisfy `nm` would fail.
    let p = common::pair();
    assert_eq!(p.c.op_add(7, 3), p.rust.op_add(7, 3));
    assert_eq!(p.c.op_sub(7, 3), p.rust.op_sub(7, 3));
    assert_eq!(p.c.op_mul(7, 3), p.rust.op_mul(7, 3));
    assert_eq!(p.c.g_op_name(), p.rust.g_op_name());
    assert_eq!(p.c.g_op() as usize, p.c.op_addr(OP_TAG));
    assert_eq!(p.rust.g_op() as usize, p.rust.op_addr(OP_TAG));
    let (c1, o1) = common::capture_stdout(|| p.c.helper_call(7, 3));
    let (r1, o1r) = common::capture_stdout(|| p.rust.helper_call(7, 3));
    assert_eq!((c1, o1), (r1, o1r));
    let (c2, o2) = common::capture_stdout(|| p.c.helper_ptr(7, 3));
    let (r2, o2r) = common::capture_stdout(|| p.rust.helper_ptr(7, 3));
    assert_eq!((c2, o2), (r2, o2r));
    let (c3, o3) = common::capture_stdout(|| p.c.use_generated(4));
    let (r3, o3r) = common::capture_stdout(|| p.rust.use_generated(4));
    assert_eq!((c3, o3), (r3, o3r));
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 28 — the `#ifndef OP` / `#ifndef REPEAT` fallbacks.
// ---------------------------------------------------------------------------

#[test]
fn cfg_28_no_define_falls_back_to_add_5() {
    // Build the C with no -DOP/-DREPEAT at all: mdmacros.h:27-32 must supply
    // add / 5, i.e. the same object as an explicit -DOP=add -DREPEAT=5 build.
    let root = repo_root();
    let dir = root.join("cbuild");
    std::fs::create_dir_all(&dir).expect("create cbuild/");

    let plain = dir.join("exe_nodefine/driver");
    std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
    let status = Command::new("gcc")
        .arg("-O2")
        .arg(format!("-I{}", root.join("c_src/src").display()))
        .arg("-o")
        .arg(&plain)
        .arg(root.join("c_src/src/mdcore.c"))
        .arg(root.join("c_src/src/mdmain.c"))
        .status()
        .expect("run gcc");
    assert!(status.success(), "no-define C build failed");

    let explicit = dir.join("exe_add_5/driver");
    assert!(
        explicit.exists(),
        "expected the add/5 reference at {}; run build_c.sh",
        explicit.display()
    );

    let a = Command::new(&plain).args(["7", "3"]).output().unwrap();
    let b = Command::new(&explicit).args(["7", "3"]).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&a.stdout),
        String::from_utf8_lossy(&b.stdout),
        "the C #ifndef fallbacks must equal -DOP=add -DREPEAT=5"
    );
    assert!(
        String::from_utf8_lossy(&a.stdout).contains("op=add call=10 acc=10 g.call=10"),
        "fallback build should report op=add with acc=10 (REPEAT=5), got {:?}",
        String::from_utf8_lossy(&a.stdout)
    );
}
