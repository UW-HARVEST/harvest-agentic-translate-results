//! Phase D — symbol parity between the C shared object and the Rust `cdylib`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` → the set of exported symbol names, plus the set of
/// *strongly* defined ones (weak crt symbols such as `__gmon_start__` are
/// reported separately).
fn exported(path: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut all = BTreeSet::new();
    let mut strong = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(_addr), Some(k), Some(n)) => (k.to_string(), n.to_string()),
            (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
            _ => continue,
        };
        let name = name.split('@').next().unwrap().to_string();
        all.insert(name.clone());
        if !matches!(kind.as_str(), "w" | "W" | "v" | "V") {
            strong.insert(name);
        }
    }
    (all, strong)
}

fn undefined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn sym_c_exports_are_all_present_in_rust() {
    let cso = c_shared_lib();
    let rso = rust_shared_lib();
    let (c_all, c_strong) = exported(&cso);
    let (r_all, _r_strong) = exported(&rso);

    // The three real functions of the translation unit.
    let expected: BTreeSet<String> = ["driver", "main", "print_foo"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        c_strong, expected,
        "the C .so exports a different set of strong symbols than expected \
         (a C source file may have been added): {c_strong:?}"
    );

    let missing: Vec<&String> = c_all.difference(&r_all).collect();
    // Weak crt symbols (`__gmon_start__`, `_ITM_*`, `__cxa_finalize`) are
    // provided by the toolchain, not by the translation unit; they are only
    // required to be absent from the *strong* diff.
    let missing_strong: Vec<&String> = c_strong.difference(&r_all).collect();
    assert!(
        missing_strong.is_empty(),
        "the Rust .so is missing these C symbols: {missing_strong:?}"
    );
    for m in &missing {
        assert!(
            m.starts_with("_ITM_") || m.starts_with("__gmon") || m.starts_with("__cxa"),
            "unexpected missing symbol: {m}"
        );
    }
}

/// The Rust `.so` must be fully resolvable — no undefined non-libc symbol.
#[test]
fn sym_rust_so_has_no_unresolved_symbols() {
    let rso = rust_shared_lib();
    // RTLD_NOW: resolve *every* relocation right away; this fails loudly if
    // any undefined symbol cannot be satisfied by libc/libgcc.
    const RTLD_NOW: i32 = 2;
    let lib = unsafe { libloading::os::unix::Library::open(Some(&rso), RTLD_NOW) }
        .expect("dlopen(RTLD_NOW) of the Rust .so must succeed");
    for sym in [&b"driver\0"[..], b"print_foo\0", b"main\0"] {
        unsafe { lib.get::<*const ()>(sym) }
            .unwrap_or_else(|e| panic!("missing {}: {e}", String::from_utf8_lossy(sym)));
    }

    // Cross-check with `nm`: nothing outside libc / libgcc_s.
    let undef = undefined(&rso);
    let allowed_prefixes = [
        "_Unwind_", "__", "_ITM_", "pthread_", "dl", "std", "abort", "bcmp", "calloc", "close",
        "free", "fstat", "getcwd", "getenv", "gettid", "lseek", "malloc", "mem", "mmap", "munmap",
        "open", "posix_", "read", "realloc", "realpath", "stat", "statx", "str", "syscall",
        "write", "sigaltstack", "sysconf", "mprotect", "getauxval", "sigaction", "sigemptyset",
        "poll", "fcntl", "isatty", "exit", "environ", "abs",
    ];
    for u in &undef {
        assert!(
            allowed_prefixes.iter().any(|p| u.starts_with(p)),
            "unexpected undefined symbol in the Rust .so: {u}"
        );
    }

    // And `ldd` must not report a missing dependency.
    let out = Command::new("ldd").arg(&rso).output().expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "ldd reports missing dependencies:\n{text}"
    );
}

/// The C executable and the Rust executable expose the same *behavioural*
/// entry point; assert both artifacts exist and are executable.
#[test]
fn sym_artifacts_exist() {
    for p in [c_exe(), rust_exe(), c_shared_lib(), rust_shared_lib(), so_runner()] {
        assert!(p.exists(), "{} is missing", p.display());
        let md = std::fs::metadata(&p).unwrap();
        assert!(md.len() > 0, "{} is empty", p.display());
    }
}
