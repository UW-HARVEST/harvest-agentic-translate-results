//! Phase D: symbol parity between the C shared object and the Rust `cdylib`.
//!
//! `nm -D` is run on both and the sets are compared. Every symbol the C `.so`
//! exports must also be exported by the Rust `.so` under the exact same name.
//! See `SYMBOLS.md` for the recorded output.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` -> the set of exported dynamic symbol names.
/// Weak toolchain stubs (`w`/`V`) are excluded: they are emitted by the compiler
/// driver, not defined by the translation unit.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    parse_nm(&String::from_utf8_lossy(&out.stdout), &['w', 'W', 'v', 'V'])
}

/// Every defined dynamic symbol of `so`, **including weak ones**. Needed when
/// asking "does this library provide symbol X", because glibc exports plenty of
/// POSIX entry points (`close`, `write`, `isatty`, `open64`, ...) as weak
/// aliases, which the strict export-surface view above deliberately filters out.
fn provided_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    parse_nm(&String::from_utf8_lossy(&out.stdout), &[])
}

/// `nm -D --undefined-only` -> the set of imported dynamic symbol names.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    parse_nm(&String::from_utf8_lossy(&out.stdout), &[])
}

fn parse_nm(text: &str, skip_types: &[char]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"  or  "<type> <name>" for undefined/weak
        let (ty, name) = match parts.len() {
            3 => (parts[1], parts[2]),
            2 => (parts[0], parts[1]),
            _ => continue,
        };
        let tc = ty.chars().next().unwrap_or('?');
        if skip_types.contains(&tc) {
            continue;
        }
        // strip the "@GLIBC_2.2.5" version suffix
        let name = name.split('@').next().unwrap_or(name);
        set.insert(name.to_string());
    }
    set
}

/// Ground truth: the C source defines exactly one function, `main`, so its
/// shared object exports exactly one symbol.
#[test]
fn c_so_exports_only_main() {
    let c = exported_symbols(&c_so());
    assert_eq!(
        c,
        ["main"].iter().map(|s| s.to_string()).collect::<BTreeSet<_>>(),
        "unexpected C .so symbol surface: {:?}",
        c
    );
}

/// The completion gate: the symbol diff must be **empty**.
#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "\nthe Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n  C   : {:?}\n  Rust: {:?}\n",
        missing.len(),
        missing,
        c,
        r
    );

    // Report (but do not fail on) extras: a cdylib may legitimately export more.
    let extra: Vec<&String> = r.difference(&c).collect();
    eprintln!(
        "symbol parity: {} C symbol(s) all present in the Rust .so; {} extra Rust symbol(s): {:?}",
        c.len(),
        extra.len(),
        extra
    );
}

/// The libraries `so` actually links against, as reported by `ldd`.
fn resolved_deps(so: &Path) -> Vec<std::path::PathBuf> {
    let out = Command::new("ldd").arg(so).output().expect("run ldd");
    assert!(out.status.success(), "ldd failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v = Vec::new();
    for line in text.lines() {
        // "\tlibc.so.6 => /lib64/libc.so.6 (0x...)"
        if let Some(rhs) = line.split("=>").nth(1) {
            let p = rhs.split_whitespace().next().unwrap_or("");
            if !p.is_empty() && Path::new(p).is_file() {
                v.push(std::path::PathBuf::from(p));
            }
        } else {
            // "\t/lib64/ld-linux-x86-64.so.2 (0x...)"
            let p = line.split_whitespace().next().unwrap_or("");
            if p.starts_with('/') && Path::new(p).is_file() {
                v.push(std::path::PathBuf::from(p));
            }
        }
    }
    v
}

/// The completion gate's other half: `nm -D` must show **0 missing/undefined
/// non-libc symbols** in the Rust `.so`.
///
/// Checked two mechanical ways rather than against a hand-written allowlist:
/// `ldd -r`, which performs the actual relocation and reports anything it cannot
/// resolve, and an explicit subset check of every undefined symbol against the
/// export tables of the libraries the `.so` links to.
#[test]
fn rust_so_has_no_unresolved_symbols() {
    let so = rust_so();

    // 1. let the dynamic loader itself try to resolve everything
    let out = Command::new("ldd").arg("-r").arg(&so).output().expect("run ldd -r");
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let complaints: Vec<&str> = report
        .lines()
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            l.contains("undefined symbol") || l.contains("not found")
        })
        .collect();
    assert!(
        complaints.is_empty(),
        "`ldd -r {}` could not resolve some symbols:\n{}",
        so.display(),
        complaints.join("\n")
    );

    // 2. every undefined symbol must be exported by one of the linked libraries
    let mut provided: BTreeSet<String> = BTreeSet::new();
    let deps = resolved_deps(&so);
    assert!(!deps.is_empty(), "ldd reported no resolvable dependencies");
    for d in &deps {
        provided.extend(provided_symbols(d));
    }
    let imports = undefined_symbols(&so);
    let missing: Vec<&String> = imports
        .iter()
        // ld.so-provided / compiler weak stubs are resolved by the loader itself
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__gmon_start__"))
        .filter(|s| !provided.contains(*s))
        .collect();
    assert!(
        missing.is_empty(),
        "Rust .so imports symbols not provided by any linked library (missing translation?): {:?}\n  libraries checked: {:?}",
        missing,
        deps
    );
    eprintln!(
        "no unresolved symbols in {} ({} imports, all satisfied by {:?})",
        so.display(),
        imports.len(),
        deps.iter().map(|d| d.file_name().unwrap()).collect::<Vec<_>>()
    );
}

/// Records the ground truth for `SYMBOLS.md`: the CMake target is an
/// *executable*, and a non-`-rdynamic` executable exports none of its own
/// functions, so there is no library symbol surface to compare there.
#[test]
fn c_executable_exports_no_user_symbols() {
    let exe = c_exe();
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(&exe)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let set = parse_nm(&String::from_utf8_lossy(&out.stdout), &['w', 'W', 'v', 'V']);
    // Only glibc's own copy relocations may appear (stdin/stdout), never `main`.
    let user: Vec<&String> = set
        .iter()
        .filter(|s| !matches!(s.as_str(), "stdin" | "stdout" | "stderr"))
        .collect();
    assert!(
        user.is_empty(),
        "the C executable unexpectedly exports user symbols: {:?}",
        user
    );
    assert!(
        !set.contains("main"),
        "unexpected: the C executable exports `main`"
    );
}
