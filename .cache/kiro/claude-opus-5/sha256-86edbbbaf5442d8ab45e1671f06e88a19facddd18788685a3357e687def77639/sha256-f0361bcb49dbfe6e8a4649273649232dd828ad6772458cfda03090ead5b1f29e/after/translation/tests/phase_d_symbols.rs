//! Phase D — symbol parity between the C `.so` and the Rust `.so`, enforced as
//! a test so it cannot silently regress.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"]).into_iter().collect()
}

fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "-u"]).into_iter().collect()
}

/// Completion gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name.
#[test]
fn symbol_parity_no_missing() {
    let c = c_so_path();
    let r = rust_so_path();
    let cs = defined(&c);
    let rs = defined(&r);
    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are missing from the Rust .so: {:?}\n\
         C .so:    {}\nRust .so: {}",
        missing.len(),
        missing,
        c.display(),
        r.display()
    );
    // sanity: the C library really does export the 16 documented names
    for want in [
        "intput",
        "strkey",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ] {
        assert!(cs.contains(want), "C .so is missing {want}; SYMBOLS.md is stale");
        assert!(rs.contains(want), "Rust .so is missing {want}");
    }
    assert_eq!(cs.len(), 16, "C .so symbol count changed; regenerate SYMBOLS.md");
}

/// The Rust `.so` must not depend on anything beyond libc / the platform
/// runtime — i.e. no non-libc undefined symbols.
#[test]
fn rust_so_has_no_foreign_undefined_symbols() {
    let r = rust_so_path();
    let u = undefined(&r);
    // libc + ELF/gcc/pthread runtime plumbing that any cdylib pulls in
    let allowed_exact: BTreeSet<&str> = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__tls_get_addr",
        "_Unwind_Backtrace",
        "_Unwind_GetIP",
        "_Unwind_GetIPInfo",
        "_Unwind_GetLanguageSpecificData",
        "_Unwind_GetRegionStart",
        "_Unwind_GetTextRelBase",
        "_Unwind_GetDataRelBase",
        "_Unwind_SetIP",
        "_Unwind_SetGR",
        "_Unwind_Resume",
        "_Unwind_RaiseException",
        "_Unwind_DeleteException",
        "_Unwind_FindEnclosingFunction",
        "_Unwind_GetCFA",
    ]
    .into_iter()
    .collect();
    let mut suspicious = Vec::new();
    for s in &u {
        if allowed_exact.contains(s.as_str()) {
            continue;
        }
        // anything that looks like a libc/libm/libgcc/libpthread/librt entry
        if s.starts_with("__")
            || s.starts_with("_dl_")
            || s.starts_with("pthread_")
            || s.starts_with("dl")
            || matches!(
                s.as_str(),
                "realloc" | "malloc" | "calloc" | "free" | "memcpy" | "memmove" | "memset"
                    | "memcmp" | "strcmp" | "strlen" | "abort" | "write" | "writev" | "read"
                    | "close" | "open" | "open64" | "readlink" | "getcwd" | "getenv" | "exit"
                    | "sysconf" | "mmap" | "mmap64" | "munmap" | "mprotect" | "sigaction"
                    | "sigaltstack" | "signal" | "raise" | "poll" | "bcmp" | "strerror_r"
                    | "syscall" | "gnu_get_libc_version" | "environ" | "stat" | "stat64"
                    | "fstat" | "fstat64" | "lseek" | "lseek64" | "getpid" | "gettid"
                    | "sched_yield" | "nanosleep" | "clock_gettime" | "posix_memalign"
                    | "realpath" | "dirfd" | "opendir" | "readdir64" | "closedir"
                    | "pipe2" | "dup2" | "execvp" | "fork" | "kill" | "waitpid"
                    | "strchr" | "strncmp" | "strnlen" | "qsort" | "getrandom"
                    | "statx" | "getauxval" | "prctl" | "madvise" | "sigemptyset"
                    | "sigaddset" | "pipe" | "fcntl" | "ioctl" | "isatty" | "abs"
            )
        {
            continue;
        }
        suspicious.push(s.clone());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has non-libc undefined symbols (a missing translation would show up here): {suspicious:?}"
    );
}

/// The Rust `.so` must not export *extra* public names that the C `.so` does
/// not have, other than the compiler/runtime metadata every cdylib emits.
#[test]
fn rust_so_exports_are_not_a_superset() {
    let cs = defined(&c_so_path());
    let rs = defined(&rust_so_path());
    let extra: Vec<&String> = rs
        .difference(&cs)
        .filter(|s| !s.starts_with("_") && !s.starts_with("rust_"))
        .collect();
    assert!(extra.is_empty(), "Rust .so exports unexpected extra symbols: {extra:?}");
}

/// Both libraries must agree on the sizes/offsets the header layout implies,
/// otherwise every pointer-arithmetic-based comparison in the other tests would
/// be meaningless. `stbds_arrgrowf` writes `length`, `capacity`, `hash_table`
/// and `temp` at fixed offsets, so a mismatch shows up immediately.
#[test]
fn header_layout_agrees() {
    let _g = lock();
    let (c, r) = libs();
    assert_eq!(HEADER_SIZE, 32, "stbds_array_header must be 32 bytes on x86-64");
    unsafe {
        // min_len = 0 + 7 = 7; 7 > min_cap(0) so min_cap = 7; 7 is neither
        // < 2*0 nor < 4, so the capacity stays exactly 7.
        let cp = (c.arrgrowf)(std::ptr::null_mut(), 8, 7, 0);
        let rp = (r.arrgrowf)(std::ptr::null_mut(), 8, 7, 0);
        let ch = *((cp as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader);
        let rh = *((rp as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader);
        assert_eq!(ch.length, 0);
        assert_eq!(ch.capacity, 7);
        assert_eq!(ch.temp, 0);
        assert!(ch.hash_table.is_null());
        assert_eq!(rh.length, ch.length);
        assert_eq!(rh.capacity, ch.capacity);
        assert_eq!(rh.temp, ch.temp);
        assert_eq!(rh.hash_table.is_null(), ch.hash_table.is_null());
        (c.arrfreef)(cp);
        (r.arrfreef)(rp);
    }
    // stbds_hash_index layout: shmode_func fills in every field, so comparing
    // all of them (as snapshot_map does) validates the offsets end to end.
    unsafe {
        (c.rand_seed)(0x1234);
        (r.rand_seed)(0x1234);
        let ct = (c.shmode_func)(16, 3);
        let rt = (r.shmode_func)(16, 3);
        let a = snapshot_map(ct, 16, 0, KeyKind::Bytes);
        let b = snapshot_map(rt, 16, 0, KeyKind::Bytes);
        assert_eq!(a, b, "stbds_hash_index layout mismatch");
        assert_eq!(a.slot_count, 8);
        assert_eq!(a.slot_count_log2, 3);
        assert_eq!(a.used_count_threshold, 6);
        assert_eq!(a.tombstone_count_threshold, 1);
        assert_eq!(a.used_count_shrink_threshold, 0);
        assert_eq!(a.string_mode, 3);
        assert_eq!(a.bucket_hash.len(), 8);
        assert_eq!(a.bucket_index, vec![-1isize; 8]);
        (c.hmfree_func)((ct as *mut u8).sub(16) as *mut _, 16);
        (r.hmfree_func)((rt as *mut u8).sub(16) as *mut _, 16);
    }
}
