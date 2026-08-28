//! Phase D — symbol parity and artifact/test cross-checks.
//!
//! These tests recompute the claims made in `SYMBOLS.md`, `ERRORS.md` and
//! `CONFIGS.md` instead of trusting them, so the documents cannot silently rot.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `(type_char, name)` for every line of an `nm -D` dump.
fn parse(dump: &str) -> Vec<(char, String)> {
    let mut v = Vec::new();
    for line in dump.lines() {
        let mut it = line.split_whitespace();
        let first = match it.next() {
            Some(f) => f,
            None => continue,
        };
        let (ty, name) = if first.len() == 1 {
            // undefined/weak: "w _ITM_..." (no address column)
            match it.next() {
                Some(n) => (first.chars().next().unwrap(), n),
                None => continue,
            }
        } else {
            // defined: "0000000000001109 T hsv_to_rgb"
            let ty = match it.next() {
                Some(t) if t.len() == 1 => t.chars().next().unwrap(),
                _ => continue,
            };
            match it.next() {
                Some(n) => (ty, n),
                None => continue,
            }
        };
        v.push((ty, name.to_string()));
    }
    v
}

/// Everything the object *exports* (any defined, globally visible symbol —
/// text, data, bss, weak-defined and unique-global alike).
fn exported(so: &Path) -> BTreeSet<String> {
    parse(&nm(&["-D", "--defined-only"], so))
        .into_iter()
        .filter(|(ty, _)| matches!(ty, 'T' | 'D' | 'B' | 'R' | 'W' | 'V' | 'u' | 'i' | 'G' | 'S'))
        .map(|(_, n)| n)
        .collect()
}

fn c_so() -> PathBuf {
    common::c_so_path()
}
fn rust_so() -> PathBuf {
    common::rust_so_path()
}

#[test]
fn exported_symbol_sets_match() {
    let cs = exported(&c_so());
    let rs = exported(&rust_so());
    assert!(
        !cs.is_empty(),
        "the C .so exports nothing — wrong file? ({})",
        c_so().display()
    );
    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {cs:?}\n\
         Rust({}): {rs:?}",
        missing.len(),
        c_so().display(),
        rust_so().display()
    );
    // Also record the exact C surface: if a new C module ever appears, this
    // fails and the translation must be extended rather than silently skipped.
    assert_eq!(
        cs.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["hsv_to_rgb"],
        "the C .so's exported surface changed"
    );
}

#[test]
fn exported_symbols_have_the_same_type() {
    let ct: Vec<(char, String)> = parse(&nm(&["-D", "--defined-only"], &c_so()));
    let rt: Vec<(char, String)> = parse(&nm(&["-D", "--defined-only"], &rust_so()));
    for (ty, name) in ct {
        if !matches!(ty, 'T' | 'D' | 'B' | 'R' | 'W' | 'V') {
            continue;
        }
        let found = rt.iter().find(|(_, n)| *n == name);
        match found {
            Some((rty, _)) => assert_eq!(
                *rty, ty,
                "symbol `{name}` is `{ty}` in the C .so but `{rty}` in the Rust .so"
            ),
            None => panic!("symbol `{name}` missing from the Rust .so"),
        }
    }
}

#[test]
fn rust_so_has_no_unresolvable_undefined_symbols() {
    // Every import must be either versioned (a real libc / libgcc import) or
    // weak (optional). Anything else would be a dangling reference to code that
    // was never translated.
    let dump = nm(&["-D", "-u"], &rust_so());
    let allowed_unversioned: &[&str] = &[
        "__tls_get_addr",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
    ];
    let mut bad = Vec::new();
    for (ty, name) in parse(&dump) {
        if ty == 'w' || ty == 'v' {
            continue; // weak/optional
        }
        if name.contains('@') || allowed_unversioned.contains(&name.as_str()) {
            continue;
        }
        bad.push(name);
    }
    assert!(
        bad.is_empty(),
        "the Rust .so has undefined non-libc symbols: {bad:?}"
    );

    // and the ultimate proof that everything resolves: the loader accepts it
    // (`common::rust()` dlopen's it with immediate symbol lookup).
    let _ = common::rust();
    let _ = common::c();
}

#[test]
fn both_objects_are_separate_shared_libraries() {
    let c = c_so();
    let r = rust_so();
    assert_ne!(c, r);
    for p in [&c, &r] {
        let head = std::fs::read(p).expect("read .so");
        assert_eq!(&head[0..4], b"\x7fELF", "{} is not an ELF file", p.display());
        assert_eq!(head[4], 2, "{} is not ELF64", p.display());
        // e_type == ET_DYN (3)
        assert_eq!(
            u16::from_le_bytes([head[16], head[17]]),
            3,
            "{} is not a shared object",
            p.display()
        );
    }
}

// ---------------------------------------------------------------------------
// artifact <-> test cross-checks: every table row must name a real test
// ---------------------------------------------------------------------------

fn manifest(p: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn all_test_sources() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut s = String::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("read tests dir").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                s.push_str(&std::fs::read_to_string(&p).expect("read test source"));
            }
        }
    }
    s
}

/// Pull the `test` column (the 4th `|`-separated cell of a row starting with a
/// number) out of a markdown table.
fn row_test_names(md: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in md.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // cells[0] is empty (leading '|')
        if cells.len() < 3 {
            continue;
        }
        let id = cells[1];
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // the test column is the second-to-last non-empty cell (last is the tick)
        let name = cells
            .iter()
            .rev()
            .find_map(|c| {
                let c = c.trim_matches('`');
                if c.starts_with("b0")
                    || c.starts_with("b1")
                    || c.starts_with("b2")
                    || c.starts_with("b3")
                    || c.starts_with("b4")
                    || c.starts_with("err")
                {
                    Some(c.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        out.push((id.to_string(), name));
    }
    out
}

#[test]
fn every_configs_row_names_an_existing_test() {
    let md = manifest("CONFIGS.md");
    let src = all_test_sources();
    let rows = row_test_names(&md);
    assert!(rows.len() >= 42, "CONFIGS.md has only {} rows", rows.len());
    for (id, name) in &rows {
        assert!(!name.is_empty(), "CONFIGS.md row {id} names no test");
        assert!(
            src.contains(&format!("fn {name}(")),
            "CONFIGS.md row {id} names `{name}`, which is not a test function"
        );
    }
    // and every row must be ticked
    for line in md.lines() {
        if line.trim_start().starts_with('|')
            && line.contains("`b")
            && line.contains("| [")
        {
            assert!(
                line.contains("[x]"),
                "unchecked CONFIGS.md row: {}",
                line.trim()
            );
        }
    }
}

#[test]
fn every_errors_row_names_an_existing_test() {
    let md = manifest("ERRORS.md");
    let src = all_test_sources();
    let rows = row_test_names(&md);
    assert!(rows.len() >= 25, "ERRORS.md has only {} rows", rows.len());
    for (id, name) in &rows {
        assert!(!name.is_empty(), "ERRORS.md row {id} names no test");
        assert!(
            src.contains(&format!("fn {name}(")),
            "ERRORS.md row {id} names `{name}`, which is not a test function"
        );
    }
    for line in md.lines() {
        if line.trim_start().starts_with('|') && line.contains("`err") && line.contains("| [") {
            assert!(
                line.contains("[x]"),
                "unchecked ERRORS.md row: {}",
                line.trim()
            );
        }
    }
}

#[test]
fn cargo_toml_declares_no_features() {
    // Phase D requires re-running everything for every feature combination.
    // If a `[features]` section ever appears, this test fails so that
    // `check_all_features.sh` (and this comment) get updated.
    let toml = manifest("Cargo.toml");
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml grew a [features] section: re-run phases B and C for every \
         combination and update CONFIGS.md"
    );
}
