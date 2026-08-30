//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and every dynamic symbol the Rust `.so` imports
//! must be satisfiable from libc / libgcc.

#![allow(dead_code)]

include!("common/harness.rs");

use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn exported(path: &std::path::Path) -> Vec<String> {
    nm(&["-D", "--defined-only"], path)
}

/// The C `.so`'s exported set must be a SUBSET of the Rust `.so`'s.
#[test]
fn sym_every_c_export_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        c.iter().any(|s| s == "driver") && c.iter().any(|s| s == "printHexCharLine"),
        "unexpected C export set: {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\nRust exports: {r:?}",
        missing.len()
    );
}

/// Both entry points must be resolvable via `dlsym` on the Rust handle — i.e.
/// the `#[unsafe(no_mangle)] extern "C"` wrappers really are there.
#[test]
fn sym_both_entry_points_resolve_via_dlsym() {
    for sym in [DRIVER, PRINT_HEX] {
        let name = String::from_utf8_lossy(&sym[..sym.len() - 1]).to_string();
        unsafe {
            c_lib()
                .get::<FnChar>(sym)
                .unwrap_or_else(|e| panic!("C dlsym {name}: {e}"));
            rust_lib()
                .get::<FnChar>(sym)
                .unwrap_or_else(|e| panic!("Rust dlsym {name}: {e}"));
        }
    }
}

/// Every undefined dynamic symbol in the Rust `.so` must be libc/libgcc
/// runtime support — nothing unresolved and non-libc.
#[test]
fn sym_rust_has_no_unresolved_non_libc_imports() {
    let out = Command::new("ldd").arg(rust_so_path()).output().expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "Rust .so has unresolved shared-library dependencies:\n{text}"
    );
    // The library under test must still go through libc's own printf, exactly
    // like the C does, so byte output and stdio buffering match.
    let imports = nm(&["-D", "--undefined-only"], &rust_so_path());
    assert!(
        imports.iter().any(|s| s.starts_with("printf")),
        "Rust .so does not import libc printf; output/buffering fidelity is not guaranteed: {imports:?}"
    );
    let c_imports = nm(&["-D", "--undefined-only"], &c_so_path());
    assert!(
        c_imports.iter().any(|s| s.starts_with("printf")),
        "C .so unexpectedly does not import printf: {c_imports:?}"
    );
}

/// ABI fidelity beyond the symbol *names*: the C `driver` reaches
/// `printHexCharLine` through the PLT, so that call is **interposable** — an
/// `LD_PRELOAD`ed definition replaces the one `driver` uses.  A naive Rust
/// translation loses this in release builds, because LLVM inlines the callee
/// (it assumes ELF symbols are never interposed, and rustc has no
/// `-fsemantic-interposition`).  This test drives a small C consumer that
/// `dlopen`s each library with and without a preloaded shim, and requires the
/// Rust `.so` to react exactly like the C one.
#[test]
fn sym_internal_call_is_interposable_like_the_c() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let tmp = std::env::temp_dir().join(format!("driver_interpose_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir tmp");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let shim = tmp.join("interpose_shim.so");
    let main = tmp.join("interpose_main");

    let build_shim = Command::new(&cc)
        .args(["-shared", "-fPIC", "-o"])
        .arg(&shim)
        .arg(fixtures.join("interpose_shim.c"))
        .status();
    match build_shim {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("skipping: no working C compiler ({cc}) to build the interposition probe");
            return;
        }
    }
    let ok = Command::new(&cc)
        .arg("-o")
        .arg(&main)
        .arg(fixtures.join("interpose_main.c"))
        .arg("-ldl")
        .status()
        .expect("compile interpose_main.c")
        .success();
    assert!(ok, "failed to build the interposition probe consumer");

    let run = |lib: &std::path::Path, preload: bool| -> String {
        let mut cmd = Command::new(&main);
        cmd.arg(lib);
        if preload {
            cmd.env("LD_PRELOAD", &shim);
        }
        let out = cmd.output().expect("run interposition probe");
        assert!(
            out.status.success(),
            "probe failed for {}: {}",
            lib.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let c_path = c_so_path();
    let c_plain = run(&c_path, false);
    let c_pre = run(&c_path, true);

    // Check EVERY built profile: `--release` inlines where `--debug` does not,
    // so testing only one profile would miss the regression entirely.
    for r_path in rust_so_paths() {
        let r_plain = run(&r_path, false);
        assert_eq!(
            c_plain,
            r_plain,
            "plain dlopen consumer ({}): C={c_plain:?} Rust={r_plain:?}",
            r_path.display()
        );

        let r_pre = run(&r_path, true);
        assert_eq!(
            c_pre,
            r_pre,
            "under LD_PRELOAD interposition {} behaved differently from the C .so:\n\
             C={c_pre:?}\nRust={r_pre:?}\n\
             (the C `driver` calls printHexCharLine through the PLT; the Rust one must too)",
            r_path.display()
        );
    }

    // Sanity-check that the probe really is sensitive: the C library *does*
    // change behaviour when the shim is preloaded, so equality above is
    // meaningful rather than vacuous.
    assert_ne!(
        c_plain, c_pre,
        "interposition probe is vacuous -- preloading the shim did not change the C library"
    );
    assert!(c_pre.contains("SHIM("), "expected the C driver to call the shim: {c_pre:?}");

    std::fs::remove_dir_all(&tmp).ok();
}
