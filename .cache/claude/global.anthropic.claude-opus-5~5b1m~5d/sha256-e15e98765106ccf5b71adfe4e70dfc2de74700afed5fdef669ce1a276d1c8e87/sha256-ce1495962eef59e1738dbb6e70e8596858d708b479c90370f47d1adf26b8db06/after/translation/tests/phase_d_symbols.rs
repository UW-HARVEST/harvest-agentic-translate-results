//! Phase D — symbol parity between the two shared libraries.
//!
//! Runs `nm -D` on both `.so`s and asserts the C library's exported-symbol set
//! is a subset of the Rust library's, with byte-identical names (macro-generated
//! names included). Also asserts every symbol is genuinely *callable* through
//! `dlsym` and that the Rust `.so` leaves no non-libc symbol undefined.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

fn defined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_syms = defined_symbols(&l.c.path);
    let rust_syms = defined_symbols(&l.rust.path);

    assert!(
        !c_syms.is_empty(),
        "nm found no symbols in {} — the test is not actually checking anything",
        l.c.path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   ({}): {:?}\nRust ({}): {:?}",
        missing.len(),
        missing,
        c_syms.len(),
        c_syms,
        rust_syms.len(),
        rust_syms
    );
}

#[test]
fn d1b_release_so_also_has_full_symbol_parity() {
    // The release artifact is what an external consumer links; check it too.
    let l = libs_release();
    let c_syms = defined_symbols(&l.c.path);
    let rust_syms = defined_symbols(&l.rust.path);
    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "release Rust .so is missing symbol(s): {missing:?}"
    );
}

#[test]
fn d2_expected_symbol_list_matches_the_c_library() {
    // Guards against the C library silently gaining or losing an export without
    // SYMBOLS.md being updated.
    let l = libs();
    let c_syms = defined_symbols(&l.c.path);
    let expected: BTreeSet<String> = EXPECTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's export set differs from SYMBOLS.md.\n\
         only in .so: {:?}\nonly in SYMBOLS.md: {:?}",
        c_syms.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&c_syms).collect::<Vec<_>>()
    );
}

#[test]
fn d3_every_symbol_is_dlsym_resolvable_in_both() {
    // `libs()` already resolves all 11 symbols in both libraries via dlsym
    // during construction, so reaching this point proves resolvability; assert
    // the count explicitly so the intent is recorded.
    let l = libs();
    assert_eq!(EXPECTED_SYMBOLS.len(), 11);
    // Touch every function pointer so nothing is optimised out.
    let ptrs: Vec<usize> = vec![
        l.rust.add_operation as usize,
        l.rust.multiply_operation as usize,
        l.rust.subtract_operation as usize,
        l.rust.modulo_operation as usize,
        l.rust.safe_double_to_int as usize,
        l.rust.compute_scaled_value as usize,
        l.rust.compare_results_in_array as usize,
        l.rust.init_result_array as usize,
        l.rust.process_with_foreach as usize,
        l.rust.compute_weighted_sum as usize,
        l.rust.arrayfunc as usize,
    ];
    assert_eq!(ptrs.len(), EXPECTED_SYMBOLS.len());
    for (name, p) in EXPECTED_SYMBOLS.iter().zip(&ptrs) {
        assert_ne!(*p, 0, "Rust symbol `{name}` resolved to NULL");
    }
    // Distinct symbols must have distinct addresses (no accidental aliasing of
    // e.g. add_operation and subtract_operation onto one function).
    let unique: BTreeSet<usize> = ptrs.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ptrs.len(),
        "two Rust exports share an address — an implementation was aliased \
         instead of translated: {:?}",
        EXPECTED_SYMBOLS
            .iter()
            .zip(&ptrs)
            .map(|(n, p)| format!("{n}=0x{p:x}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn d4_rust_so_has_no_undefined_non_libc_symbols() {
    let l = libs_release();
    let undefined = nm(&l.rust.path, "--undefined-only");

    // Everything the Rust runtime legitimately imports from libc / libgcc.
    let allowed_exact = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__tls_get_addr",
        "__libc_start_main",
    ];
    let allowed_prefixes = ["_Unwind_", "__libc_", "pthread_", "__pthread_"];
    let allowed_libc = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getauxval", "qsort", "sigaction", "sigaltstack", "signal",
        "pipe2", "poll", "madvise", "mprotect", "munmap", "environ", "__environ",
    ];

    let mut unexpected = Vec::new();
    for sym in undefined {
        // Strip the @GLIBC_x.y / @GCC_x.y version suffix.
        let bare = sym.split('@').next().unwrap_or(&sym).to_string();
        let ok = allowed_exact.contains(&bare.as_str())
            || allowed_libc.contains(&bare.as_str())
            || allowed_prefixes.iter().any(|p| bare.starts_with(p));
        if !ok {
            unexpected.push(sym);
        }
    }
    assert!(
        unexpected.is_empty(),
        "the Rust .so imports {} non-libc symbol(s), meaning part of the library \
         is not self-contained: {:?}",
        unexpected.len(),
        unexpected
    );
}

#[test]
fn d5_struct_layout_matches_across_the_abi() {
    // If `Result`/`ResultArray` disagreed on size or offsets, every struct-based
    // differential test would be comparing different bytes. Pin the ABI down.
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<Result_>(), 24, "sizeof(Result)");
    assert_eq!(align_of::<Result_>(), 8, "alignof(Result)");
    assert_eq!(size_of::<ResultArray>(), 248, "sizeof(ResultArray)");
    assert_eq!(align_of::<ResultArray>(), 8, "alignof(ResultArray)");

    let a = ResultArray::dirty(1);
    let base = &a as *const ResultArray as usize;
    assert_eq!(
        &a.data as *const _ as usize - base,
        0,
        "offsetof(ResultArray, data)"
    );
    assert_eq!(
        &a.count as *const _ as usize - base,
        240,
        "offsetof(ResultArray, count)"
    );
    let e = &a.data[0];
    let ebase = e as *const Result_ as usize;
    assert_eq!(&e.value as *const _ as usize - ebase, 0, "offsetof(value)");
    assert_eq!(&e.scaled as *const _ as usize - ebase, 8, "offsetof(scaled)");
    assert_eq!(&e.rank as *const _ as usize - ebase, 16, "offsetof(rank)");

    // The C library agrees: `init_result_array` writes rank = i for each element,
    // so reading them back at the offsets above must yield 0..count-1.
    let l = libs();
    let mut arr = ResultArray::dirty(2);
    let mut vals: Vec<std::os::raw::c_int> = (0..10).map(|i| i * 11 - 3).collect();
    unsafe {
        (l.c.init_result_array)(&mut arr, vals.as_mut_ptr(), 10);
    }
    for k in 0..10 {
        assert_eq!(arr.data[k].rank, k as std::os::raw::c_int, "layout probe rank");
        assert_eq!(arr.data[k].value, vals[k], "layout probe value");
        assert_eq!(arr.data[k].scaled, vals[k] as f64 * 1.5, "layout probe scaled");
    }
    assert_eq!(arr.count, 10, "layout probe count");
}

// ===========================================================================
// Feature-combination guard
// ===========================================================================

/// `Cargo.toml` currently declares no `[features]`, so the complete feature space
/// is the single default (empty) combination — that is what makes "every feature
/// combination is covered" true today.
///
/// This test fails the moment a feature is added, forcing the new axis to be
/// added to `CONFIGS.md` and picked up by `check_features.sh`, rather than
/// letting an untested configuration appear silently.
#[test]
fn d6_feature_space_is_still_the_single_default_combination() {
    let manifest = include_str!("../Cargo.toml");

    // Ignore commented-out lines; look for a real [features] table.
    let has_features_table = manifest
        .lines()
        .map(str::trim)
        .any(|l| l == "[features]" || l.starts_with("[features."));

    assert!(
        !has_features_table,
        "Cargo.toml has gained a [features] table.\n\
         The verification artifacts assume a single (empty) feature combination.\n\
         Add the new axis to CONFIGS.md and re-run ./check_features.sh, which \
         enumerates the power set of features automatically."
    );

    // Also assert no cfg(feature = ...) has crept into the library source, which
    // would mean conditionally-compiled code paths outside the matrix.
    let lib = include_str!("../src/lib.rs");
    let feature_cfgs: Vec<&str> = lib
        .lines()
        .filter(|l| l.contains("feature = \"") || l.contains("feature=\""))
        .collect();
    assert!(
        feature_cfgs.is_empty(),
        "src/lib.rs contains feature-gated code, but Cargo.toml declares no \
         features: {feature_cfgs:?}"
    );
}

/// The C source has no `#ifdef`-based configuration either, so there is no
/// build-time C configuration axis that the Rust would have to mirror.
#[test]
fn d7_c_source_has_no_conditional_compilation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let mut offenders = Vec::new();
    for rel in ["c_src/src/lib.c", "c_src/include/lib.h"] {
        let path = root.join(rel);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for (n, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("#if") || t.starts_with("#ifdef") || t.starts_with("#ifndef") {
                offenders.push(format!("{rel}:{}: {t}", n + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the C source has conditional compilation that the feature matrix does \
         not model: {offenders:?}"
    );
}
