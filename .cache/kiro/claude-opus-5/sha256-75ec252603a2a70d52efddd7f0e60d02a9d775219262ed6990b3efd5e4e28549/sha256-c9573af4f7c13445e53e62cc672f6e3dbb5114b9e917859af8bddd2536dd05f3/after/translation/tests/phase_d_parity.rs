//! Phase D — symbol parity between the two shared objects.
//!
//! Row 44 of `CONFIGS.md` (contents of the exported tables) lives here too,
//! because it is a pure `dlsym` comparison.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::process::Command;

/// `nm -D -S --defined-only` → { symbol -> (kind, size) }.
fn dyn_defined(path: &std::path::Path) -> BTreeMap<String, (String, Option<u64>)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run nm — binutils required");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.len() {
            // addr size kind name
            4 => {
                m.insert(
                    f[3].to_string(),
                    (f[2].to_string(), u64::from_str_radix(f[1], 16).ok()),
                );
            }
            // addr kind name
            3 => {
                m.insert(f[2].to_string(), (f[1].to_string(), None));
            }
            _ => {}
        }
    }
    m
}

/// `nm -D -u` → undefined (imported) symbols.
fn dyn_undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn symbol_diff_is_empty() {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    eprintln!("C   .so: {}", c_path.display());
    eprintln!("Rust.so: {}", r_path.display());

    let c = dyn_defined(&c_path);
    let r = dyn_defined(&r_path);

    let missing: Vec<&String> = c.keys().filter(|k| !r.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Data symbols must also match in kind (D = initialised, B = .bss) and size,
    // since callers may read/write them directly.
    for (name, (kind, size)) in &c {
        let (rkind, rsize) = &r[name];
        assert_eq!(
            kind, rkind,
            "symbol {name}: C kind {kind} vs Rust kind {rkind}"
        );
        if kind == "D" || kind == "B" {
            assert_eq!(size, rsize, "symbol {name}: C size {size:?} vs Rust {rsize:?}");
        }
    }
    eprintln!(
        "symbol parity: all {} C dynamic symbols present in the Rust .so with matching kind/size",
        c.len()
    );
    for (name, (kind, size)) in &c {
        eprintln!("  {kind} {name} size={size:?}");
    }

    // Extra Rust exports are fine (the Rust runtime adds some), but list them.
    let extra: Vec<&String> = r.keys().filter(|k| !c.contains_key(*k)).collect();
    eprintln!("Rust-only dynamic symbols ({}): {extra:?}", extra.len());
}

#[test]
fn no_unexpected_undefined_symbols() {
    // The Rust .so may only import libc / libgcc-unwind / weak loader symbols.
    let r = dyn_undefined(&rust_so_path());
    let allowed_prefix = ["_Unwind_", "__", "_ITM_"];
    let libc_syms = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "stat64", "statx",
        "strlen", "syscall", "write", "writev", "sysconf", "pthread_getattr_np",
        "pthread_attr_getstack", "pthread_attr_destroy", "pthread_self", "sigaltstack",
        "sigaction", "sigemptyset", "mprotect", "poll", "pipe2", "getrandom", "exit",
    ];
    let mut unexpected = Vec::new();
    for s in &r {
        let base = s.split('@').next().unwrap_or(s);
        if allowed_prefix.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if libc_syms.contains(&base) {
            continue;
        }
        unexpected.push(s.clone());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbols: {unexpected:?}"
    );
    eprintln!("undefined-symbol check: {} imports, all libc/unwind", r.len());
}

/// CONFIGS.md row 44 — the six exported tables must be byte-identical.
#[test]
fn exported_table_contents_match() {
    let libs = libs();
    unsafe {
        macro_rules! cmp_u8 {
            ($field:ident, $n:expr) => {{
                let c = std::slice::from_raw_parts(libs.c.$field, $n);
                let r = std::slice::from_raw_parts(libs.r.$field, $n);
                assert_eq!(c, r, concat!(stringify!($field), " differs"));
                eprintln!("  {} [{}] identical", stringify!($field), $n);
            }};
        }
        macro_rules! cmp_u32 {
            ($field:ident, $n:expr) => {{
                let c = std::slice::from_raw_parts(libs.c.$field, $n);
                let r = std::slice::from_raw_parts(libs.r.$field, $n);
                assert_eq!(c, r, concat!(stringify!($field), " differs"));
                eprintln!("  {} [{}] identical", stringify!($field), $n);
            }};
        }
        cmp_u8!(cp_fixed_table, 288 + 32);
        cmp_u8!(cp_permutation_order, 19);
        cmp_u8!(cp_len_extra_bits, 29 + 2);
        cmp_u32!(cp_len_base, 29 + 2);
        cmp_u8!(cp_dist_extra_bits, 30 + 2);
        cmp_u32!(cp_dist_base, 30 + 2);

        // cp_error_reason starts NULL in a freshly loaded library.
        assert!(
            (*libs.c.cp_error_reason).is_null(),
            "C cp_error_reason not NULL at load"
        );
        assert!(
            (*libs.r.cp_error_reason).is_null(),
            "Rust cp_error_reason not NULL at load"
        );
    }
}

/// The tables are exported as writable data in both libraries (`D`, not `R`),
/// which the Phase B mutation rows rely on.
#[test]
fn exported_tables_are_writable() {
    let libs = libs();
    let (c, r) = run_pair(|lib, shm| unsafe {
        *lib.cp_fixed_table = 0x5A;
        *lib.cp_permutation_order = 0x0F;
        *lib.cp_len_extra_bits = 0x03;
        *lib.cp_len_base = 0xDEAD_BEEF;
        *lib.cp_dist_extra_bits = 0x02;
        *lib.cp_dist_base = 0xFEED_FACE;
        let probe = [
            *lib.cp_fixed_table,
            *lib.cp_permutation_order,
            *lib.cp_len_extra_bits,
            *lib.cp_dist_extra_bits,
        ];
        set_payload(shm, probe.as_ptr(), 4);
        (*shm).ret = (*lib.cp_len_base as i64) ^ (*lib.cp_dist_base as i64);
    });
    assert_same("exported tables writable", &c, &r);
    assert_eq!(c.payload, vec![0x5A, 0x0F, 0x03, 0x02]);
    let _ = libs;
}
