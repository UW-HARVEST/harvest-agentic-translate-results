//! Phase D — symbol parity.
//!
//! Compares the dynamic symbol tables of the C `.so` and the Rust `.so`.
//! Every symbol the C exports must also be exported by the Rust cdylib under
//! the exact same linker name, and every one must be resolvable through
//! `dlsym` (which is what `libloading::Library::get` does).

mod common;
use common::*;
use std::process::Command;

/// `nm -D --defined-only <so>` restricted to global text symbols (`T`).
fn exported(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let (kind, name) = match it.next() {
                Some(k) if it.clone().next().is_some() => (k, it.next()?),
                Some(k) => (a, k), // "         w name" form
                None => return None,
            };
            if kind == "T" { Some(name.to_string()) } else { None }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// `nm -D --undefined-only <so>`.
fn undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("`nm` must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn phase_d_symbol_diff_is_empty() {
    let cp = c_so_path();
    let rp = rust_so_path();
    let c = exported(&cp);
    let r = exported(&rp);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();

    eprintln!("C   .so ({}): {} exported symbols", cp.display(), c.len());
    eprintln!("Rust.so ({}): {} exported symbols", rp.display(), r.len());

    assert!(
        missing.is_empty(),
        "the Rust .so does NOT export {} symbol(s) that the C .so exports: {missing:?}",
        missing.len()
    );
    assert!(extra.is_empty(), "the Rust .so exports {} symbol(s) the C does not: {extra:?}", extra.len());
    assert_eq!(c, r, "symbol sets differ");
    assert_eq!(c.len(), 38, "the C .so is expected to export exactly 38 symbols, found {}", c.len());
}

/// The symbol list recorded in `SYMBOLS.md` must still match reality.
#[test]
fn phase_d_symbols_md_is_up_to_date() {
    let c = exported(&c_so_path());
    let mut documented: Vec<String> = ALL_SYMBOLS.iter().map(|s| s.to_string()).collect();
    documented.sort();
    assert_eq!(c, documented, "SYMBOLS.md / ALL_SYMBOLS is out of sync with `nm -D` on the C .so");
}

/// Every C symbol must actually be *callable* through `dlsym` on both libraries
/// (a name in `.dynsym` that cannot be resolved would still show up in `nm`).
#[test]
fn phase_d_every_symbol_resolves_in_both() {
    // `api()` performs `Library::get` for all 38 symbols in both libraries and
    // panics with the offending name if any lookup fails.
    let a = api();
    // Touch one function pointer from each library so the loads cannot be
    // optimised away, and prove the pair really is two distinct addresses.
    let (cf, rf) = a.c2V;
    assert!(cf as usize != rf as usize, "the same library got loaded twice");
    assert_eq!(ALL_SYMBOLS.len(), 38);
}

/// No undefined symbol in the Rust `.so` may be anything other than a
/// libc / libgcc / Rust-runtime import.
#[test]
fn phase_d_no_unresolved_non_libc_symbols() {
    let und = undefined(&rust_so_path());
    let allowed_prefix = ["_Unwind_", "__", "_ITM_"];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap64", "munmap", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific", "read", "readlink",
        "realloc", "realpath", "sqrtf", "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sysconf", "getauxval", "poll", "sigaltstack", "mprotect", "pthread_self",
        "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy", "environ",
    ];
    let mut bad = Vec::new();
    for s in &und {
        let name = s.split('@').next().unwrap_or(s);
        if allowed_prefix.iter().any(|p| name.starts_with(p)) || allowed_exact.contains(&name) {
            continue;
        }
        bad.push(name.to_string());
    }
    assert!(bad.is_empty(), "the Rust .so has unresolved non-libc symbols: {bad:?}");

    // And the C's only real import is `sqrtf`.
    let cund: Vec<String> = undefined(&c_so_path())
        .iter()
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.starts_with("__") && !s.starts_with("_ITM_"))
        .collect();
    assert_eq!(cund, vec!["sqrtf".to_string()], "unexpected C imports: {cund:?}");
}
