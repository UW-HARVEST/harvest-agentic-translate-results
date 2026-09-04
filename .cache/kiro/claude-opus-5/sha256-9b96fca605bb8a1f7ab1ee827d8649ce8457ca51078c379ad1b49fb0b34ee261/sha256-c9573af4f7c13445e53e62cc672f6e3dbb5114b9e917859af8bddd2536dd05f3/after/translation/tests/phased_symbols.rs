//! Phase D — symbol parity enforced as a test.
//!
//! Runs `nm -D` on both `.so`s and requires the diff to be empty in both
//! directions, and that no zstd-internal symbol is left undefined in Rust.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, arg: &str) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(arg)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                3 => Some((f[2].to_string(), f[1].to_string())),
                2 if f[0] == "U" || f[0] == "w" => Some((f[1].to_string(), f[0].to_string())),
                _ => None,
            }
        })
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
        .into_iter()
        .filter(|(_, t)| matches!(t.as_str(), "T" | "D" | "B" | "R"))
        .map(|(n, _)| n)
        .collect()
}

#[test]
fn exported_symbols_match_exactly() {
    let c = defined(&c_so_path());
    let r = defined(&rs_so_path());
    assert!(c.len() > 500, "sanity: C .so should export 500+ symbols, got {}", c.len());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        missing.is_empty(),
        "{} symbols exported by C but MISSING from Rust:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert!(
        extra.is_empty(),
        "{} symbols exported by Rust but not C:\n{}",
        extra.len(),
        extra.join("\n")
    );
    eprintln!("symbol parity: {} symbols, diff empty both ways", c.len());
}

/// No zstd-internal symbol may be undefined in the Rust `.so` — that would mean
/// a whole module was never translated.
#[test]
fn no_undefined_zstd_symbols_in_rust() {
    let u: Vec<String> = nm(&rs_so_path(), "--undefined-only")
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| {
            let base = n.split('@').next().unwrap_or(n);
            base.starts_with("ZSTD")
                || base.starts_with("ZDICT")
                || base.starts_with("ZBUFF")
                || base.starts_with("FSE_")
                || base.starts_with("HUF_")
                || base.starts_with("HIST_")
                || base.starts_with("COVER_")
                || base.starts_with("POOL_")
                || base.starts_with("ERR_")
                || base.starts_with("divsufsort")
        })
        .collect();
    assert!(u.is_empty(), "undefined zstd symbols in Rust .so:\n{}", u.join("\n"));
}

/// Every symbol must actually be `dlsym`-able from both handles (catches
/// visibility-only exports).
#[test]
fn every_symbol_is_dlsym_able() {
    let c = defined(&c_so_path());
    let l = libs();
    let mut fails = Vec::new();
    for s in &c {
        let name = format!("{s}\0");
        unsafe {
            let a = l.c.get::<*const ()>(name.as_bytes()).is_ok();
            let b = l.rs.get::<*const ()>(name.as_bytes()).is_ok();
            if a != b {
                fails.push(format!("{s}: C={a} Rust={b}"));
            }
        }
    }
    assert!(fails.is_empty(), "dlsym mismatch:\n{}", fails.join("\n"));
}
