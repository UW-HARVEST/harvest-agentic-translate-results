//! Phase D — symbol parity, enforced as a test so it cannot regress.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must not import any
//! non-libc/non-unwind symbol.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path).into_iter().collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let p = pair();
    let c = defined(&p.c.path);
    let r = defined(&p.rs.path);

    assert!(!c.is_empty(), "nm found no exported symbols in the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}",
        missing.len()
    );

    // The two functions in c_src/src/lib.c, spelled out so an accidental
    // rename or a dropped #[no_mangle] fails loudly.
    for want in ["flac_validate", "tflac_size_memory"] {
        assert!(c.contains(want), "C .so unexpectedly lacks {want}");
        assert!(r.contains(want), "Rust .so lacks {want}");
    }
    eprintln!("[D1] symbol diff empty; {} shared exports", c.len());
}

#[test]
fn d2_rust_so_has_no_unresolved_non_libc_imports() {
    let p = pair();
    let undef = nm(&["-D", "--undefined-only"], &p.rs.path);
    let allowed_prefixes = [
        "_Unwind_",
        "_ITM_",
        "__cxa_",
        "__gmon_start__",
        "__tls_get_addr",
        "__errno_location",
        "__libc_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_getspecific", "pthread_key_create", "pthread_key_delete", "pthread_mutex_lock",
        "pthread_mutex_trylock", "pthread_mutex_unlock", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_self", "pthread_setspecific", "read", "readlink",
        "realloc", "realpath", "sigaction", "sigaltstack", "stat", "stat64", "statx", "strlen",
        "syscall", "sysconf", "write", "writev",
    ]
    .into_iter()
    .collect();

    let mut offenders = Vec::new();
    for sym in undef {
        // strip the @GLIBC_x.y / @GCC_x.y version suffix
        let name = sym.split('@').next().unwrap_or(&sym);
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) || allowed_exact.contains(name) {
            continue;
        }
        offenders.push(sym);
    }
    assert!(
        offenders.is_empty(),
        "Rust .so imports non-libc symbols (would mean a missing translation): {offenders:?}"
    );
}

#[test]
fn d3_struct_abi_matches_the_c_layout() {
    // sizeof(tflac)==28, alignof==4, cur_blocksize at offset 24 (verified
    // against the C compiler with an offsetof probe). If this ever changes the
    // whole byte-for-byte comparison becomes meaningless, so pin it here too.
    assert_eq!(std::mem::size_of::<Tflac>(), 28);
    assert_eq!(std::mem::align_of::<Tflac>(), 4);
    assert_eq!(OFF_CUR_BLOCKSIZE, 24);
    assert_eq!(OFF_PARTITION_ORDER, 20);

    // Round-trip proof that the Rust .so reads the fields at the offsets this
    // harness writes them to: a value only reachable if cur_blocksize is read
    // back from offset 24 after being written from offset 0.
    let p = pair();
    let mut t = Tflac::valid();
    t.set_u32(OFF_BLOCKSIZE, 1234 * 16).set_u32(OFF_CUR_BLOCKSIZE, 0xDEAD_BEEF);
    let mut tr = t;
    assert_eq!(p.rs.flac_validate(&mut tr), 0);
    assert_eq!(tr.cur_blocksize(), 1234 * 16);
    let mut tc = t;
    assert_eq!(p.c.flac_validate(&mut tc), 0);
    assert_eq!(tc.0, tr.0);
}
