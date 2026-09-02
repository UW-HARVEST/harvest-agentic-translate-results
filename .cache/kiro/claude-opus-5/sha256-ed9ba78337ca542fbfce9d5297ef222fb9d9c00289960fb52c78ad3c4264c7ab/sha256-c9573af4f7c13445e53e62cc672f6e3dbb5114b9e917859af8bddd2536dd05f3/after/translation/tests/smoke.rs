//! Harness sanity checks and Phase D symbol parity.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn harness_loads_both_shared_objects() {
    let i = impls();
    assert!(i.c_path.is_file(), "C .so missing: {}", i.c_path.display());
    assert!(
        i.rust_path.is_file(),
        "Rust .so missing: {}",
        i.rust_path.display()
    );
    eprintln!("C   : {}", i.c_path.display());
    eprintln!("RUST: {}", i.rust_path.display());
}

/// Guards against `dlsym` accidentally resolving glibc's own `wcscat`
/// (`wchar_t *wcscat(wchar_t *, const wchar_t *)`), which would silently make
/// every comparison meaningless. The library's own `wcscat` returns 22 for a
/// NULL destination; glibc's would dereference it.
#[test]
fn resolved_symbol_is_the_library_not_glibc() {
    let i = impls();
    let r_c = unsafe { (i.c)(std::ptr::null_mut(), 4, std::ptr::null()) };
    let r_r = unsafe { (i.rust)(std::ptr::null_mut(), 4, std::ptr::null()) };
    assert_eq!(r_c, 22, "C .so did not resolve to the library's own wcscat");
    assert_eq!(
        r_r, 22,
        "Rust .so did not resolve to the crate's own wcscat"
    );
}

#[test]
fn smoke_basic_append_matches() {
    // "ab" + "cd" in a 16-element buffer.
    let mut dst = vec![0i32; 16];
    dst[0] = 'a' as i32;
    dst[1] = 'b' as i32;
    let src = vec!['c' as i32, 'd' as i32, 0];
    let c = Case::new(dst, 16, Src::Own(src));
    let out = both(&c);
    assert_eq!(out.ret, 0);
    assert_eq!(&out.dst[0..5], &['a' as i32, 'b' as i32, 'c' as i32, 'd' as i32, 0]);
}

/// Phase D: every symbol the C `.so` exports must also be exported by the Rust
/// `.so` under the exact same name.
#[test]
fn symbol_parity_c_vs_rust() {
    let i = impls();

    let defined = |p: &PathBuf| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("nm must be available");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            p.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let c_syms = defined(&i.c_path);
    let rust_syms = defined(&i.rust_path);

    assert!(
        c_syms.contains(&"wcscat".to_string()),
        "C .so unexpectedly does not export wcscat: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    eprintln!("C exports {} symbol(s), all present in Rust .so", c_syms.len());
}

/// Phase D: the Rust `.so` must have no unresolved non-libc / non-unwinder
/// symbol. `dlopen` succeeding is the strong form of this check; this test
/// additionally reports the import list for the record.
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let i = impls();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", i.rust_path.to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);

    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
        "gettid",
        "statx",
        "syscall",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath", "stat64",
        "strlen", "write", "writev", "sysconf", "getrandom", "clock_gettime", "sigaltstack",
        "sigaction", "mprotect", "pipe2", "poll", "environ", "memrchr", "memchr", "strerror_r",
        "abs", "qsort", "exit",
    ];

    let mut unexpected = Vec::new();
    for line in text.lines() {
        let Some(name) = line.split_whitespace().last() else {
            continue;
        };
        let base = name.split('@').next().unwrap_or(name);
        if allowed_prefixes.iter().any(|p| base.starts_with(p))
            || allowed_exact.contains(&base)
        {
            continue;
        }
        unexpected.push(base.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

#[test]
fn phase_a_artifacts_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for f in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let p = root.join(f);
        assert!(p.is_file(), "missing Phase A artifact {}", p.display());
        let n = std::fs::read_to_string(&p).unwrap().lines().count();
        assert!(n > 10, "{f} looks empty ({n} lines)");
    }
    // And the C tree must be untouched.
    assert!(workspace_root().join("c_src/src/lib.c").is_file());
}
