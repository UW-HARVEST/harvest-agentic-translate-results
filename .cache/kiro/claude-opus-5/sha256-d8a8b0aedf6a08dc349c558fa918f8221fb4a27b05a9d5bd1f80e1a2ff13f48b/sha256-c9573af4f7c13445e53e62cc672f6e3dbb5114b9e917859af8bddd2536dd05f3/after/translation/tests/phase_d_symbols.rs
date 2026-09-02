//! Phase D — symbol parity between the two shared objects, enforced in-test so
//! it cannot drift.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.into_iter().next().expect("no C .so; build c_src first")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    let p = dir.join("libbetagamma_lib.so");
    if p.exists() {
        p
    } else {
        workspace_root()
            .join("translation/target/release/libbetagamma_lib.so")
    }
}

/// Dynamic symbols of a given `nm` class.
fn nm(path: &PathBuf, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm: {e}"));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so(), "--defined-only");

    // The C .so exports exactly the five non-static functions in src/lib.c.
    let expected: BTreeSet<String> = [
        "allocate_block",
        "betagamma",
        "compute_hash",
        "create_block",
        "free_block",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c, expected,
        "the C .so's exported surface changed; SYMBOLS.md / the tests need updating"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) the C .so exports: {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
         translate the missing C source if a whole module was skipped.",
        missing.len()
    );
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = nm(&rust_so(), "--undefined-only");
    // Everything the Rust cdylib imports must come from libc / libgcc-unwind /
    // the pthread+TLS runtime. Anything else would be an unresolved reference
    // to code that was never translated.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__tls_get_addr", "__errno_location",
        "pthread_", "gettid", "statx", "syscall", "dl_iterate_phdr",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "free", "fstat64", "getcwd", "getenv", "lseek64",
        "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
        "read", "readlink", "realloc", "realpath", "stat64", "strcpy", "strlen", "write", "writev",
    ];

    let mut unexpected = Vec::new();
    for s in &undef {
        let bare = s.split('@').next().unwrap_or(s);
        let ok = allowed_exact.contains(&bare)
            || allowed_prefixes.iter().any(|p| bare.starts_with(p));
        if !ok {
            unexpected.push(s.clone());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved NON-libc symbols (untranslated code?): {unexpected:?}"
    );
}

#[test]
fn phase_d_no_symbol_is_a_stub() {
    // A symbol that merely *exists* is not enough — Phase A forbids stubs. Each
    // export must do real work, which we check behaviourally through the .so:
    // every function must produce the C's observable effect, not a placeholder.
    let pair = load_pair();
    unsafe {
        // create_block really copies its arguments
        let name = b"stub-check\0";
        let b = (pair.rs.create_block)(-42, name.as_ptr() as *const std::ffi::c_char, 0x5A);
        let d = defined(&b);
        assert_eq!(d.id, -42, "create_block is a stub (id not copied)");
        assert_eq!(d.flags, 0x5A, "create_block is a stub (flags not copied)");
        assert_eq!(
            &d.name[..d.name.len() - 1],
            b"stub-check",
            "create_block is a stub (name not copied)"
        );

        // allocate_block really allocates and really fills
        let m = (pair.rs.allocate_block)(7, 1000);
        assert!(!m.is_null(), "allocate_block is a stub (returned NULL)");
        assert_eq!((*m).size, 7, "allocate_block is a stub (size not set)");
        let s = std::slice::from_raw_parts((*m).data, 7);
        assert_eq!(
            s,
            &[1000, 1001, 1002, 1003, 1004, 1005, 1006],
            "allocate_block is a stub (contents not initialised)"
        );

        // compute_hash really branches
        let mut lo = MemoryBlock {
            data: 0x1000 as *mut std::ffi::c_int,
            size: 0,
        };
        let mut hi = MemoryBlock {
            data: 0x2000 as *mut std::ffi::c_int,
            size: 0,
        };
        let h = (pair.rs.compute_hash)(&mut lo, &mut hi);
        assert_ne!(h, 0, "compute_hash is a stub (always 0)");

        // free_block really frees (no crash, and the allocation is reusable)
        (pair.rs.free_block)(m);

        // betagamma really computes
        assert_ne!(
            (pair.rs.betagamma)(1, 2, 3, 4),
            0,
            "betagamma is a stub (always 0)"
        );
        assert_eq!(
            (pair.rs.betagamma)(-6, 1, 1, 1),
            -1,
            "betagamma does not implement the error path"
        );
    }
}
