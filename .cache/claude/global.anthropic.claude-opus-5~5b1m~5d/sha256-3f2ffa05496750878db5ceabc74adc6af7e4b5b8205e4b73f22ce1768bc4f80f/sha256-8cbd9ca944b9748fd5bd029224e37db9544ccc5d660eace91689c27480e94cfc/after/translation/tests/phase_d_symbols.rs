//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only global/weak text & data symbols form the public ABI.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
                Some(name.split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        // Ignore the linker/CRT boilerplate that is not part of either API.
        .filter(|n| {
            !matches!(
                n.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
/// exact same name.  The diff must be empty.
#[test]
fn phase_d_symbol_diff_is_empty() {
    let c_syms = defined_dynamic_symbols(c_so_path());
    let r_syms = defined_dynamic_symbols(rust_so_path());

    assert!(
        c_syms.contains("get_predict_func"),
        "sanity: C .so must export get_predict_func, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {r_syms:?}"
    );

    // Report (but do not fail on) Rust-only symbols; under the `difftest`
    // feature the verification hooks are expected extras.
    let extra: Vec<&String> = r_syms.difference(&c_syms).collect();
    if cfg!(feature = "difftest") {
        for e in &extra {
            assert!(
                e.starts_with("__difftest_"),
                "unexpected Rust-only exported symbol: {e}"
            );
        }
    } else {
        assert!(
            extra.is_empty(),
            "default build must export exactly the C ABI, extra: {extra:?}"
        );
    }
}

/// The Rust `.so` must not import anything outside libc / the Rust unwinder.
#[test]
fn phase_d_no_unresolved_non_libc_imports() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let kind = it.next().unwrap_or("");
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        // Weak (w) references are optional; only hard `U` matters.
        if kind != "U" {
            continue;
        }
        let base = name.split('@').next().unwrap();
        let is_libc_or_runtime = name.contains("@GLIBC")
            || name.contains("@GCC")
            || base.starts_with("_Unwind_")
            || base.starts_with("__")
            || matches!(
                base,
                "malloc" | "calloc" | "realloc" | "free" | "memcpy" | "memmove" | "memset"
                    | "bcmp" | "strlen" | "abort" | "getenv" | "getcwd" | "readlink"
                    | "realpath" | "open" | "close" | "read" | "write" | "writev"
                    | "lseek" | "mmap" | "munmap" | "syscall" | "dl_iterate_phdr"
                    | "posix_memalign" | "statx" | "gettid"
            );
        if !is_libc_or_runtime {
            suspicious.push(base.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc imports: {suspicious:?}"
    );
}

/// The public symbol must be callable through `dlsym` with C linkage and the
/// documented signature from `include/lib.h`.
#[test]
fn phase_d_public_symbol_is_callable_from_both() {
    let (c, r) = pair_public();
    assert!(c.has("get_predict_func"));
    assert!(r.has("get_predict_func"));
    let cf: libloading::Symbol<GetPredictFunc> = c.sym("get_predict_func");
    let rf: libloading::Symbol<GetPredictFunc> = r.sym("get_predict_func");
    for pfcn in -20i32..=20 {
        assert_eq!(unsafe { cf(pfcn) }, unsafe { rf(pfcn) }, "pfcn={pfcn}");
    }
}

/// Under the default (featureless) build the Rust `.so` must NOT leak the
/// verification hooks.
#[test]
fn phase_d_difftest_hooks_gated_by_feature() {
    let syms = defined_dynamic_symbols(rust_so_path());
    let has_hooks = syms.iter().any(|s| s.starts_with("__difftest_"));
    assert_eq!(
        has_hooks,
        cfg!(feature = "difftest"),
        "__difftest_* export presence must track the `difftest` feature \
         (feature={}, present={})",
        cfg!(feature = "difftest"),
        has_hooks
    );
}
