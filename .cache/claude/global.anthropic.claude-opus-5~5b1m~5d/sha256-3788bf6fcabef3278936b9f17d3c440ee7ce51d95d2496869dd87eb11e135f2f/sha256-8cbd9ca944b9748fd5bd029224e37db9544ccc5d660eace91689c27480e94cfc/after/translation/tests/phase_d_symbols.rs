//! Phase D - symbol parity between the two shared objects, enforced as a test.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn c_so() -> PathBuf {
    let build = root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("build the C first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().unwrap()
}

fn rust_so() -> PathBuf {
    for profile in ["release", "debug"] {
        let p = root().join("translation/target").join(profile).join("libtritanopia_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust .so not built");
}

/// Exported (defined) dynamic symbols, excluding the special ones every ELF
/// shared object gets from the linker rather than from the source.
fn exported(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Keep real code/data symbols; drop linker-synthesised ones.
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V") {
                return None;
            }
            const SYNTHETIC: [&str; 8] = [
                "_init", "_fini", "__bss_start", "_edata", "_end",
                "__libc_csu_init", "__libc_csu_fini", "_IO_stdin_used",
            ];
            if SYNTHETIC.contains(&name) || name.starts_with("__gmon") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Undefined (imported) symbols.
fn imported(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The core Phase D gate: the symbol diff must be EMPTY.
#[test]
fn d1_symbol_parity_c_vs_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    println!("C   exports ({}): {:?}", c.len(), c);
    println!("Rust exports ({}): {:?}", r.len(), r);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) the C .so exports: {:?}",
        missing.len(),
        missing
    );

    // The C's whole public surface is a single function.
    assert!(c.contains("tritanopia"), "C must export tritanopia; got {c:?}");
    assert!(r.contains("tritanopia"), "Rust must export tritanopia; got {r:?}");
    assert_eq!(c.len(), 1, "C surface changed; re-derive SYMBOLS.md: {c:?}");
}

/// No non-libc/libm symbol may be left undefined in the Rust `.so`.
#[test]
fn d2_no_unresolved_non_libc_symbols_in_rust() {
    let imports = imported(&rust_so());
    println!("Rust imports: {imports:?}");

    // Classify by ELF symbol-version tag rather than by a hand-written name
    // list: anything resolved out of glibc/libgcc carries an `@GLIBC_*` or
    // `@GCC_*` version, and the only unversioned imports a normal Rust cdylib
    // has are the three weak linker-synthesised hooks. Anything else would be a
    // reference to code that was never translated.
    const WEAK_SYNTHETIC: [&str; 3] = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let unexpected: Vec<&String> = imports
        .iter()
        .filter(|s| {
            let versioned = s.contains("@GLIBC_") || s.contains("@GCC_") || s.contains("@GLIBCXX_");
            !versioned && !WEAK_SYNTHETIC.contains(&s.as_str())
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved NON-libc symbols (a skipped module?): {unexpected:?}"
    );

    // The C links `m` for `pow`; the Rust must resolve the SAME platform `pow`
    // rather than lowering to its own implementation, or the transcendental
    // results could differ in the last bit.
    assert!(
        imports.iter().any(|s| s.starts_with("pow@") || s == "pow"),
        "Rust .so does not import the platform `pow`; it must not use f64::powf. Imports: {imports:?}"
    );

    // Sanity: the C side imports pow too, and nothing exotic.
    let c_imports = imported(&c_so());
    println!("C imports: {c_imports:?}");
    assert!(
        c_imports.iter().any(|s| s.starts_with("pow@") || s == "pow"),
        "expected the C to import pow"
    );
}

/// Both libraries must resolve all their dependencies at load time - i.e. they
/// actually `dlopen` and the symbol is callable. A stub that lied about its
/// behaviour would still pass `nm`, so the value is checked too.
#[test]
fn d3_both_symbols_are_real_and_callable() {
    let p = common::Pair::load();
    // A stub returning zeros / echoing its input would pass a naive symbol
    // check; assert the transform is genuinely applied and non-trivial.
    let white = p.check(common::CbRgb255::new(255, 255, 255));
    let black = p.check(common::CbRgb255::new(0, 0, 0));
    let blue = p.check(common::CbRgb255::new(0, 0, 255));

    assert_eq!(black, common::CbRgb255::new(0, 0, 0), "black must map to black");
    assert_ne!(blue, common::CbRgb255::new(0, 0, 255), "blue must be transformed");
    println!("d3: white={white:?} black={black:?} blue={blue:?}");
}

/// There is no binary/driver executable in this project, so the
/// "compare C and Rust stdout" gate is not applicable. Asserted, not assumed.
#[test]
fn d4_no_executable_to_compare() {
    let cml = std::fs::read_to_string(root().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cml.contains("add_executable"),
        "c_src now builds an executable - the stdout-comparison gate applies and \
         must be implemented"
    );
    assert!(
        !root().join("translation/src/main.rs").exists(),
        "translation now has a binary target - implement the stdout comparison"
    );
    let toml = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(!toml.contains("[[bin]]"), "translation declares a [[bin]] target");
    // And no features, so there is exactly one build configuration.
    assert!(
        !toml.contains("[features]"),
        "translation now declares features - Phases B/C must be re-run per combo"
    );
}
