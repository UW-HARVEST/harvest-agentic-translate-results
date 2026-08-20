//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! with the exact same name, and the Rust `.so` must have no undefined
//! non-libc/non-toolchain symbols.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parses `nm -D` output into (defined, undefined) name sets.
fn nm(path: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace().peekable();
        let first = match it.peek() {
            Some(s) => *s,
            None => continue,
        };
        // Layout is either "<addr> <type> <name>" or "<type> <name>".
        let (ty, name) = if first.len() == 1 {
            let ty = it.next().unwrap().to_string();
            match it.next() {
                Some(n) => (ty, n.to_string()),
                None => continue,
            }
        } else {
            let _addr = it.next().unwrap();
            let ty = match it.next() {
                Some(t) => t.to_string(),
                None => continue,
            };
            match it.next() {
                Some(n) => (ty, n.to_string()),
                None => continue,
            }
        };
        // Strip the version suffix so `abort@GLIBC_2.2.5` -> `abort`.
        let bare = name.split('@').next().unwrap().to_string();
        match ty.as_str() {
            "U" => {
                undefined.insert(bare);
            }
            "w" | "v" => { /* weak-undefined toolchain hooks: ignored */ }
            _ => {
                defined.insert(bare);
            }
        }
    }
    (defined, undefined)
}

/// `DT_NEEDED` entries of a shared object.
fn dt_needed(path: &Path) -> BTreeSet<String> {
    let out = Command::new("objdump")
        .arg("-p")
        .arg(path)
        .output()
        .expect("run objdump -p");
    assert!(out.status.success(), "objdump -p {} failed", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            if it.next() == Some("NEEDED") {
                it.next().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Union of the symbols defined by every object `path` actually loads at
/// runtime (resolved through `ldd`), i.e. the whole C runtime surface it is
/// allowed to import from.
fn runtime_defined_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("ldd")
        .arg(path)
        .output()
        .expect("run ldd");
    let mut libs: Vec<String> = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // "  libc.so.6 => /lib64/libc.so.6 (0x...)" or "  /lib64/ld-linux... (0x...)"
        let line = line.trim();
        let cand = if let Some(idx) = line.find("=> ") {
            line[idx + 3..].split_whitespace().next().unwrap_or("")
        } else {
            line.split_whitespace().next().unwrap_or("")
        };
        if cand.starts_with('/') && Path::new(cand).is_file() {
            libs.push(cand.to_string());
        }
    }
    let mut all = BTreeSet::new();
    for lib in libs {
        let (def, _) = nm(Path::new(&lib));
        all.extend(def);
    }
    all
}

#[test]
fn phase_d_symbol_parity() {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    eprintln!("C   .so: {}", c_path.display());
    eprintln!("Rust.so: {}", r_path.display());

    let (c_def, _c_undef) = nm(&c_path);
    let (r_def, r_undef) = nm(&r_path);

    eprintln!("C defined:    {c_def:?}");
    eprintln!("Rust defined: {r_def:?}");

    // The whole C API surface.
    assert!(
        c_def.contains("premultiply"),
        "the C .so must export `premultiply`; defined = {c_def:?}"
    );

    let missing: Vec<&String> = c_def.difference(&r_def).collect();
    assert!(
        missing.is_empty(),
        "PHASE D FAILURE: the Rust .so is missing {} symbol(s) exported by the \
         C .so: {missing:?}\n\
         Per the Phase A rule these must be translated (not stubbed).",
        missing.len()
    );

    // No undefined symbol in the Rust .so may be anything other than something
    // provided by the C runtime it links against. Determined mechanically: every
    // `U` entry must be defined by one of the DT_NEEDED objects.
    let needed = dt_needed(&r_path);
    eprintln!("Rust DT_NEEDED: {needed:?}");
    let allowed_libs: BTreeSet<&str> = [
        "libc.so.6",
        "libm.so.6",
        "libgcc_s.so.1",
        "libdl.so.2",
        "libpthread.so.0",
        "librt.so.1",
        "ld-linux-x86-64.so.2",
        "ld-linux-aarch64.so.1",
    ]
    .into_iter()
    .collect();
    let unexpected_libs: Vec<&String> = needed
        .iter()
        .filter(|n| !allowed_libs.contains(n.as_str()))
        .collect();
    assert!(
        unexpected_libs.is_empty(),
        "PHASE D FAILURE: the Rust .so links against non-runtime libraries: \
         {unexpected_libs:?}"
    );

    let runtime_defined = runtime_defined_symbols(&r_path);
    let suspicious: Vec<&String> = r_undef
        .iter()
        .filter(|n| !runtime_defined.contains(n.as_str()))
        .collect();
    assert!(
        suspicious.is_empty(),
        "PHASE D FAILURE: the Rust .so has {} undefined symbol(s) that no linked \
         C-runtime object provides: {suspicious:?}",
        suspicious.len()
    );

    // Belt and braces: the dynamic loader itself must be able to resolve
    // everything (`ldd -r` reports any leftover undefined symbol).
    let ldd = Command::new("ldd").arg("-r").arg(&r_path).output();
    if let Ok(o) = ldd {
        let txt = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let bad: Vec<&str> = txt
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "PHASE D FAILURE: `ldd -r` reports unresolved symbols in the Rust .so:\n{bad:#?}"
        );
    }

    // The C .so must not export anything the header does not declare, and the
    // Rust .so must not be a superset that hides a stub either.
    assert_eq!(
        c_def.iter().collect::<Vec<_>>(),
        vec!["premultiply"],
        "the C API surface changed; SYMBOLS.md must be regenerated"
    );
    assert_eq!(
        r_def.iter().collect::<Vec<_>>(),
        vec!["premultiply"],
        "the Rust .so exports an unexpected symbol set: {r_def:?}"
    );
}

/// Both `.so`s must resolve `premultiply` via `dlsym` (this is what every other
/// test relies on, asserted explicitly here).
#[test]
fn phase_d_dlsym_both() {
    let c = c_fn();
    let r = rust_fn();
    assert!(!(c as usize == 0), "C `premultiply` resolved to NULL");
    assert!(!(r as usize == 0), "Rust `premultiply` resolved to NULL");
    assert_ne!(
        c as usize, r as usize,
        "the same function was loaded twice — the two .so paths must differ"
    );
}
