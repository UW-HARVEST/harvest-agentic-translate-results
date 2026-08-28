//! Phase D — symbol-parity gate.
//!
//! Mechanically diffs `nm -D` on the two shared objects. The diff MUST be
//! empty: every symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {path:?}: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of dynamic symbols defined (exported) by `path`.
fn exported(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                // "<addr> <kind> <name>"
                3 => Some(f[2].split('@').next().unwrap().to_string()),
                // "<kind> <name>"  (no address)
                2 => Some(f[1].split('@').next().unwrap().to_string()),
                _ => None,
            }
        })
        .collect()
}

/// Names of dynamic symbols `path` needs from elsewhere.
fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let (kind, name) = match f.len() {
                2 => (f[0], f[1]),
                3 => (f[1], f[2]),
                _ => return None,
            };
            if kind != "U" && kind != "w" && kind != "v" {
                return None;
            }
            Some(name.split('@').next().unwrap().to_string())
        })
        .collect()
}

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    println!("C   .so: {c:?}");
    println!("Rust.so: {r:?}");

    let c_syms = exported(&c);
    let r_syms = exported(&r);

    // Everything the C library exports must be present in the Rust library.
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports ({}): {c_syms:?}",
        c_syms.len()
    );

    // Sanity: the 13 documented symbols really are there.
    for want in [
        "add_op",
        "multiply_op",
        "subtract_op",
        "divide_op",
        "modulo_op",
        "find_node_by_id",
        "add_tree_node",
        "calculate_tree_sum",
        "parse_operation",
        "get_operation_func",
        "inreftree",
        "node_table",
        "node_count",
    ] {
        assert!(c_syms.contains(want), "C .so lost {want}?");
        assert!(r_syms.contains(want), "Rust .so does not export {want}");
    }
    println!("symbol diff is empty ({} C symbols matched)", c_syms.len());
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = common::rust_so_path();
    let undef = undefined(&r);
    // Anything the C reference itself imports is by definition acceptable
    // (`lib.c` includes <string.h>, so it imports strchr/strncpy too).
    let c_undef = undefined(&common::c_so_path());
    // Anything the dynamic loader resolves from libc / libgcc / ld.so is fine.
    let allowed_prefixes = [
        "__", "_ITM_", "_Unwind_", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr",
        "environ", "exit", "free", "getauxval", "getcwd", "getenv", "gettid", "malloc",
        "memcmp", "memcpy", "memmove", "memset", "mmap", "munmap", "open", "poll", "posix_",
        "pthread_", "read", "readlink", "realloc", "sigaction", "sigaltstack", "signal",
        "stat", "strlen", "syscall", "sysconf", "write", "writev", "raise", "memrchr",
        "getrandom", "clock_gettime", "nanosleep", "sched_", "mprotect", "madvise",
        "fstat", "lstat", "lseek", "realpath", "dlsym", "dlopen", "dlclose", "dlerror",
        "getpid", "kill", "sigemptyset", "sigaddset", "strerror", "isatty", "fcntl",
    ];
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| !c_undef.contains(*s))
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}"
    );
    println!(
        "Rust .so imports {} symbols, all libc/runtime; C .so imports {}",
        undef.len(),
        c_undef.len()
    );
}

#[test]
fn data_symbol_sizes_match() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    fn sizes(path: &Path) -> Vec<(String, u64)> {
        let out = Command::new("readelf")
            .args(["-sW"])
            .arg(path)
            .output()
            .expect("readelf");
        let text = String::from_utf8_lossy(&out.stdout);
        let mut v = Vec::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() >= 8 && f[3] == "OBJECT" {
                let name = f[7].split('@').next().unwrap().to_string();
                if name == "node_table" || name == "node_count" {
                    if let Ok(sz) = f[2].parse::<u64>() {
                        v.push((name, sz));
                    }
                }
            }
        }
        v.sort();
        v.dedup();
        v
    }
    let cs = sizes(&c);
    let rs = sizes(&r);
    assert!(!cs.is_empty(), "no OBJECT symbols found in the C .so");
    assert_eq!(cs, rs, "exported data-object sizes differ");
    assert!(
        cs.contains(&("node_table".to_string(), 2600)),
        "node_table must be 50*52 = 2600 bytes, got {cs:?}"
    );
    assert!(cs.contains(&("node_count".to_string(), 4)));
}
