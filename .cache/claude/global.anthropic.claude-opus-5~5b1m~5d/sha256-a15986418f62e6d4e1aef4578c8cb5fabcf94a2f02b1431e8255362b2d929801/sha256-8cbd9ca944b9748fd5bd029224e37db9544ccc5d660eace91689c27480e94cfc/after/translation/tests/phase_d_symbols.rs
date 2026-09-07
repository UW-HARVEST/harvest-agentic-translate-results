//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both libraries and requires that every
//! symbol the C library exports is also exported by the Rust library under the
//! exact same name. Also asserts the reverse direction for the C file's
//! `static` helpers, which must NOT be exported by either side.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_so(dir: PathBuf, prefix: &str) -> PathBuf {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix) && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
        .unwrap_or_else(|| panic!("no {prefix}*.so in {}", dir.display()))
}

fn c_so() -> PathBuf {
    std::env::var("C_LIB_PATH").map(PathBuf::from).unwrap_or_else(|_| {
        find_so(workspace_root().join("c_src").join("build"), "lib")
    })
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap().to_path_buf();
    find_so(profile, "libto_barycentric_lib")
}

/// Linker/toolchain-synthesised entries that are not library API.
const TOOLCHAIN: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "__gmon_start__",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__tls_get_addr",
    "_Jv_RegisterClasses",
];

fn exported(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (a, b) = (f.next()?, f.next()?);
            // "<addr> <type> <name>" or "         <type> <name>"
            let (ty, name) = match f.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // keep only real definitions in text/data/bss/weak
            if !matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "R" | "r" | "i") {
                return None;
            }
            let base = name.split('@').next().unwrap();
            if TOOLCHAIN.contains(&base) {
                return None;
            }
            Some(base.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    let c_syms = exported(&c);
    let r_syms = exported(&r);

    eprintln!("C   .so {} -> {c_syms:?}", c.display());
    eprintln!("Rust.so {} -> {r_syms:?}", r.display());

    assert!(
        c_syms.contains(&"to_barycentric".to_string()),
        "sanity: the C .so must export to_barycentric, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {r_syms:?}"
    );
}

#[test]
fn static_c_helpers_are_not_exported() {
    let c_syms = exported(&c_so());
    let r_syms = exported(&rust_so());
    for internal in ["lm_v2", "lm_sub2", "lm_dot2"] {
        assert!(
            !c_syms.iter().any(|s| s == internal),
            "sanity: {internal} is `static` in C and must not be exported"
        );
        assert!(
            !r_syms.iter().any(|s| s == internal),
            "{internal} is `static` in C but the Rust .so exports it (surface mismatch)"
        );
    }
}

/// `nm -D -u` on a `.so` lists every dynamic import. A "missing/undefined
/// non-libc symbol" is one that no shared library on the link line provides —
/// `ldd -r` is the authoritative check for that, since it actually resolves
/// every import against the recorded `DT_NEEDED` libraries.
fn unresolved(path: &PathBuf) -> Vec<String> {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(path)
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    text.lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .map(|l| l.trim().to_string())
        .collect()
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Baseline: the C .so must be clean too, so the check is meaningful.
    let c_bad = unresolved(&c_so());
    assert!(c_bad.is_empty(), "the C .so itself has unresolved symbols: {c_bad:?}");

    let r = rust_so();
    let bad = unresolved(&r);
    assert!(
        bad.is_empty(),
        "Rust .so {} has unresolved / missing symbols:\n{}",
        r.display(),
        bad.join("\n")
    );

    // Every dynamic import of the Rust .so must come from a system library
    // (glibc / ld.so / libgcc), never from an untranslated C module: no import
    // may share a name with a function defined in c_src.
    let out = Command::new("nm")
        .args(["-D", "-u", r.to_str().unwrap()])
        .output()
        .expect("run nm -u");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let imports: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|n| n.split('@').next().unwrap().to_string())
        .collect();
    for c_fn in ["to_barycentric", "lm_v2", "lm_sub2", "lm_dot2"] {
        assert!(
            !imports.iter().any(|i| i == c_fn),
            "the Rust .so IMPORTS {c_fn} instead of defining it — that module was not translated"
        );
    }
}
