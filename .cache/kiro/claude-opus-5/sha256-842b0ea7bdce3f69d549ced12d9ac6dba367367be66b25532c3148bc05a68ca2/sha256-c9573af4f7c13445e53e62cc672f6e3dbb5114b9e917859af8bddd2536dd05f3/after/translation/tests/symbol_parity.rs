//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must import nothing outside
//! libc / the unwinder.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Toolchain / loader artifacts that are not library API.
fn is_toolchain_artifact(sym: &str) -> bool {
    sym.starts_with('_')
        || sym.starts_with("__")
        || sym.contains("@GLIBC")
        || sym.contains("@GCC")
        || matches!(sym, "abort" | "memcpy" | "memmove" | "memset" | "malloc" | "free")
}

#[test]
fn exported_symbols_match() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_defined: BTreeSet<String> = nm(&["-D", "--defined-only"], &c)
        .into_iter()
        .filter(|s| !is_toolchain_artifact(s))
        .collect();
    let r_defined: BTreeSet<String> = nm(&["-D", "--defined-only"], &r)
        .into_iter()
        .filter(|s| !is_toolchain_artifact(s))
        .collect();

    assert!(
        c_defined.contains("merge_sort"),
        "sanity: C .so must export merge_sort, got {c_defined:?}"
    );

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING symbols exported by the C .so: {missing:?}\n\
         C   = {c_defined:?}\n\
         Rust= {r_defined:?}"
    );
}

/// The Rust `.so` must not have undefined references beyond libc / libgcc.
#[test]
fn no_unexpected_undefined_symbols() {
    let r = rust_so_path();
    let undef: Vec<String> = nm(&["-D", "--undefined-only"], &r)
        .into_iter()
        .filter(|s| !is_toolchain_artifact(s))
        // Everything the Rust std runtime imports from libc.
        .filter(|s| {
            !matches!(
                s.split('@').next().unwrap_or(s),
                "bcmp"
                    | "calloc"
                    | "close"
                    | "dl_iterate_phdr"
                    | "fstat64"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "lseek64"
                    | "mmap64"
                    | "munmap"
                    | "open64"
                    | "posix_memalign"
                    | "pthread_key_create"
                    | "pthread_key_delete"
                    | "pthread_setspecific"
                    | "read"
                    | "readlink"
                    | "realloc"
                    | "realpath"
                    | "stat64"
                    | "statx"
                    | "strlen"
                    | "syscall"
                    | "write"
                    | "writev"
            )
        })
        .collect();
    assert!(
        undef.is_empty(),
        "Rust .so has unexpected undefined (non-libc) symbols: {undef:?}"
    );
}

/// `merge_sort` must be reachable by `dlsym` in both, with a compatible ABI.
#[test]
fn both_libraries_expose_callable_merge_sort() {
    let pair = Pair::load();
    let mut a_c = vec![Sprite::new(9, 5, [1; 4]), Sprite::new(8, 1, [2; 4])];
    let mut b_c = vec![Sprite::zeroed(); 2];
    let mut a_r = a_c.clone();
    let mut b_r = b_c.clone();
    unsafe { (pair.c)(a_c.as_mut_ptr(), b_c.as_mut_ptr(), 2) };
    unsafe { (pair.rust)(a_r.as_mut_ptr(), b_r.as_mut_ptr(), 2) };
    assert_eq!(a_c, a_r);
    assert_eq!(b_c, b_r);
    assert_eq!(a_c[0].sort_bits(), 1, "sanity: the C actually sorted");
}
