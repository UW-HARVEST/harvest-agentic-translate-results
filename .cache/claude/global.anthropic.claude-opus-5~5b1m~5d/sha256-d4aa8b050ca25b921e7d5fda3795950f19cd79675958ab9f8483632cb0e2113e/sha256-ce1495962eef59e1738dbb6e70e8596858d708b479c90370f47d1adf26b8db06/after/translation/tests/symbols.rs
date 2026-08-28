//! Phase A / Phase D — exported-symbol parity, enforced as a test rather than
//! as prose in `SYMBOLS.md`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Weak, toolchain-injected symbols that GCC/rustc emit into *every* shared
/// object. They are not part of the library API and are excluded by the
/// "0 missing/undefined **non-libc** symbols" criterion.
fn is_toolchain_glue(name: &str) -> bool {
    matches!(
        name,
        "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__gmon_start__"
            | "__cxa_finalize"
            | "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
    ) || name.starts_with("__cxa_")
        || name.starts_with("_ITM_")
        || name.starts_with("_Unwind_")
        || name.starts_with("rust_eh_")
        || name.starts_with("__rust_")
}

/// Defined dynamic symbols (`nm -D --defined-only`), minus toolchain glue.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    parse_nm(so, &["-D", "--defined-only"])
}

/// Undefined dynamic symbols (`nm -D --undefined-only`).
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    parse_nm(so, &["-D", "--undefined-only"])
}

fn parse_nm(so: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running `nm {args:?} {}` failed: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>"
            let name = line.split_whitespace().last()?;
            // strip the "@GLIBC_2.2.5" / "@@VER" version suffix
            let name = name.split('@').next().unwrap_or(name);
            if name.is_empty() || is_toolchain_glue(name) {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

/// **The Phase D gate**: every symbol the C `.so` exports must also be exported
/// by the Rust `.so` under the exact same name. The diff must reach empty.
#[test]
fn c_and_rust_export_identical_symbol_sets() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();

    let c_syms = defined_symbols(&c_so);
    let rust_syms = defined_symbols(&rust_so);

    println!("C   .so {} exports: {:?}", c_so.display(), c_syms);
    println!("Rust.so {} exports: {:?}", rust_so.display(), rust_syms);

    let missing_in_rust: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing_in_rust.is_empty(),
        "PARTIAL TRANSLATION: {} symbol(s) exported by the C .so are missing from \
         the Rust .so: {missing_in_rust:?}",
        missing_in_rust.len()
    );

    // The C header declares exactly one function, so anything extra on the Rust
    // side would be an ABI surface the C library does not have.
    let extra_in_rust: Vec<&String> = rust_syms.difference(&c_syms).collect();
    assert!(
        extra_in_rust.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra_in_rust:?}"
    );

    assert_eq!(c_syms, rust_syms, "symbol sets must be identical");

    // Sanity: the one real symbol is actually there (guards against both sides
    // being empty, which would make the equality vacuous).
    assert!(c_syms.contains("crc16"), "C .so must export `crc16`; got {c_syms:?}");
    assert_eq!(c_syms.len(), 1, "expected exactly one public symbol, got {c_syms:?}");
}

/// The Rust `.so` must have no unresolvable non-libc dependencies. `RTLD_NOW`
/// forces eager binding, so any unresolved relocation makes `dlopen` fail.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let rust_so = common::rust_so_path();

    const RTLD_NOW: i32 = 0x2;
    let lib = unsafe { libloading::os::unix::Library::open(Some(&rust_so), RTLD_NOW) }
        .unwrap_or_else(|e| {
            panic!("eager (RTLD_NOW) dlopen of {} failed: {e}", rust_so.display())
        });
    let sym = unsafe { lib.get::<common::Crc16Fn>(b"crc16\0") };
    assert!(sym.is_ok(), "`crc16` not resolvable in {}", rust_so.display());

    // Report what it imports, for the record.
    let undef = undefined_symbols(&rust_so);
    println!("Rust .so undefined (imported) symbols: {undef:?}");

    // Everything imported must be resolvable from the already-loaded process
    // image (libc / ld.so); RTLD_NOW succeeding above proves exactly that.
    drop(lib);
}

/// `tflac_crc16_tables` is `static const` at file scope in `lib.h`, i.e. internal
/// linkage, so it is *not* part of the C ABI. The Rust translation keeps it
/// `pub(crate)`. Neither `.so` may export it — this is a match, not a gap.
#[test]
fn tflac_crc16_tables_is_not_exported() {
    for so in [common::c_so_path(), common::rust_so_path()] {
        let syms = defined_symbols(&so);
        assert!(
            !syms.iter().any(|s| s.contains("crc16_tables") || s.contains("CRC16_TABLES")),
            "{} unexpectedly exports the internal-linkage table: {syms:?}",
            so.display()
        );
    }
}

/// Every function declared in the public header must be exported by *both* `.so`s.
/// Derived from the header text, so a newly added C declaration cannot silently
/// go untranslated.
#[test]
fn every_header_declaration_is_exported_by_both() {
    let hdr_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/lib.h");
    let hdr = std::fs::read_to_string(&hdr_path).expect("read lib.h");

    // Collect `name(` occurrences from non-table, non-typedef declaration lines.
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for line in hdr.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("0x") {
            continue;
        }
        if !line.contains('(') || !line.ends_with(';') || line.starts_with("typedef") {
            continue;
        }
        let before_paren = &line[..line.find('(').unwrap()];
        if let Some(name) =
            before_paren.split(|c: char| !(c.is_alphanumeric() || c == '_')).next_back()
            && !name.is_empty()
        {
            declared.insert(name.to_string());
        }
    }

    println!("functions declared in lib.h: {declared:?}");
    assert!(!declared.is_empty(), "failed to parse any declaration out of lib.h");
    assert_eq!(
        declared,
        BTreeSet::from(["crc16".to_string()]),
        "header surface changed; SYMBOLS.md/CONFIGS.md/ERRORS.md must be regenerated"
    );

    let c_syms = defined_symbols(&common::c_so_path());
    let rust_syms = defined_symbols(&common::rust_so_path());
    for name in &declared {
        assert!(c_syms.contains(name), "C .so missing declared fn `{name}`");
        assert!(rust_syms.contains(name), "Rust .so missing declared fn `{name}`");
    }
}

/// The C `.so` is built from every C source file in `c_src`, and there is only
/// one — so no whole module was skipped by the translation.
#[test]
fn no_c_source_file_was_left_untranslated() {
    let c_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src");
    let mut c_files: Vec<String> = Vec::new();
    let mut stack = vec![c_src.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().filter_map(|e| e.ok()) {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if p.is_dir() {
                if name != "build" {
                    stack.push(p);
                }
            } else if name.ends_with(".c") {
                c_files.push(name);
            }
        }
    }
    c_files.sort();
    println!("C translation units: {c_files:?}");
    assert_eq!(
        c_files,
        vec!["lib.c".to_string()],
        "a C translation unit appeared/disappeared — re-check translation completeness"
    );
}
