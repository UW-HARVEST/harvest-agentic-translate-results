//! Phase D — symbol parity, encoded as a test so it cannot silently rot.
//!
//! Also acts as a sanity check on the differential harness itself: it proves
//! that a C `.so` *and* at least one Rust `.so` really were dlopen'd, so a
//! passing Phase B/C run cannot be an artifact of accidentally comparing an
//! implementation against itself.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols every `.so` gets from the toolchain / libc rather than from the
/// library source. These are excluded from the "did we translate everything?"
/// comparison.
const RUNTIME_SYMBOLS: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
    "__errno_location",
    "__tls_get_addr",
    "_Unwind_Backtrace",
    "_Unwind_GetDataRelBase",
    "_Unwind_GetIP",
    "_Unwind_GetIPInfo",
    "_Unwind_GetLanguageSpecificData",
    "_Unwind_GetRegionStart",
    "_Unwind_GetTextRelBase",
    "_Unwind_Resume",
    "_Unwind_SetGR",
    "_Unwind_SetIP",
    "abort",
    "bcmp",
    "calloc",
    "close",
    "dl_iterate_phdr",
    "free",
    "fstat",
    "fstat64",
    "getcwd",
    "getenv",
    "gettid",
    "lseek",
    "lseek64",
    "malloc",
    "memcmp",
    "memcpy",
    "memmove",
    "memset",
    "mmap",
    "mmap64",
    "munmap",
    "open",
    "open64",
    "posix_memalign",
    "pthread_key_create",
    "pthread_key_delete",
    "pthread_getspecific",
    "pthread_setspecific",
    "read",
    "readlink",
    "realloc",
    "realpath",
    "stat",
    "stat64",
    "statx",
    "strlen",
    "syscall",
    "write",
    "writev",
];

fn nm(so: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// True for symbols supplied by libc / the language runtime rather than by the
/// library's own C source.
fn is_runtime(s: &str) -> bool {
    RUNTIME_SYMBOLS.contains(&s)
        || s.starts_with("_Unwind_")
        || s.starts_with("_ITM_")
        || s.starts_with("__cxa_")
        || s.starts_with("__libc_")
        || s.starts_with("pthread_")
}

fn defined(so: &Path) -> BTreeSet<String> {
    nm(so, "--defined-only")
}

fn undefined(so: &Path) -> BTreeSet<String> {
    nm(so, "--undefined-only")
}

#[test]
fn harness_really_loads_two_distinct_shared_objects() {
    let p = pair();
    eprintln!("C   : {}", p.c.path.display());
    for r in &p.rust {
        eprintln!("{:<4}: {}", r.name, r.path.display());
    }
    assert!(p.c.path.is_file());
    assert!(
        !p.rust.is_empty(),
        "no Rust .so under test — Phase B/C would be vacuous"
    );
    for r in &p.rust {
        assert_ne!(r.path, p.c.path, "Rust impl aliases the C .so");
    }
    // Both cargo profiles should be exercised.
    assert!(
        p.rust.len() >= 2,
        "expected both the debug and release Rust .so, got {:?}",
        p.rust.iter().map(|r| r.name).collect::<Vec<_>>()
    );
    // The two shared objects must be genuinely different binaries.
    let c_bytes = std::fs::read(&p.c.path).unwrap();
    for r in &p.rust {
        let r_bytes = std::fs::read(&r.path).unwrap();
        assert_ne!(c_bytes, r_bytes);
    }
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let c_so = c_so_path();
    let c_syms = defined(&c_so);
    let c_lib: BTreeSet<_> = c_syms
        .iter()
        .filter(|s| !is_runtime(s))
        .cloned()
        .collect();

    // The C library must actually export something, or this test is vacuous.
    assert!(
        !c_lib.is_empty(),
        "no library symbols found in {}",
        c_so.display()
    );
    // Sanity: these are the two functions in c_src/src/lib.c.
    for want in ["flac_validate", "tflac_size_memory"] {
        assert!(
            c_lib.contains(want),
            "C .so unexpectedly lacks {want}; symbols: {c_lib:?}"
        );
    }

    for rust_so in [rust_release_so_path(), rust_debug_so_path()] {
        let r_syms = defined(&rust_so);
        let missing: Vec<_> = c_lib.difference(&r_syms).cloned().collect();
        assert!(
            missing.is_empty(),
            "{} is MISSING C-exported symbols: {missing:?}",
            rust_so.display()
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    for rust_so in [rust_release_so_path(), rust_debug_so_path()] {
        let leftover: Vec<_> = undefined(&rust_so)
            .into_iter()
            .filter(|s| !is_runtime(s))
            .collect();
        assert!(
            leftover.is_empty(),
            "{} has unresolved non-libc symbols (untranslated C?): {leftover:?}",
            rust_so.display()
        );
    }
}

#[test]
fn rust_struct_layout_matches_the_c_compiler() {
    // Compile a probe against the real header with the same compiler CMake used
    // and compare against the offsets the harness (and src/lib.rs) assume.
    let dir = std::env::temp_dir().join(format!("tflac_layout_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("probe.c");
    std::fs::write(
        &src,
        r#"
#include <stdio.h>
#include <stddef.h>
#include "lib.h"
int main(void){
  printf("%zu %zu %zu %zu %zu %zu %zu %zu %zu %zu %zu\n",
    sizeof(tflac), _Alignof(tflac),
    offsetof(tflac,blocksize), offsetof(tflac,samplerate),
    offsetof(tflac,channels), offsetof(tflac,bitdepth),
    offsetof(tflac,channel_mode), offsetof(tflac,max_rice_value),
    offsetof(tflac,min_partition_order), offsetof(tflac,max_partition_order),
    offsetof(tflac,partition_order));
  printf("%zu\n", offsetof(tflac,cur_blocksize));
  return 0;
}
"#,
    )
    .unwrap();
    let bin = dir.join("probe");
    let inc = manifest_dir().join("c_src").join("include");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let st = Command::new(&cc)
        .arg("-I")
        .arg(&inc)
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("compile layout probe");
    assert!(
        st.status.success(),
        "probe compile failed: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    let out = Command::new(&bin).output().expect("run layout probe");
    let text = String::from_utf8_lossy(&out.stdout);
    let nums: Vec<usize> = text
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        nums,
        vec![
            TFLAC_SIZE,
            4,
            OFF_BLOCKSIZE,
            OFF_SAMPLERATE,
            OFF_CHANNELS,
            OFF_BITDEPTH,
            OFF_CHANNEL_MODE,
            OFF_MAX_RICE_VALUE,
            OFF_MIN_PARTITION_ORDER,
            OFF_MAX_PARTITION_ORDER,
            OFF_PARTITION_ORDER,
            OFF_CUR_BLOCKSIZE,
        ],
        "struct tflac layout differs from what the harness assumes: {text:?}"
    );
}
