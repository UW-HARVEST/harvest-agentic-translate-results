//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Both libraries are inspected with `nm -D` and the *defined* dynamic symbol
//! sets are diffed. The diff must be empty in the C→Rust direction (every C
//! export exists in Rust with the exact same name), and the Rust library must
//! not leave any non-libc symbol undefined.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, args: &[&str]) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                3 => Some((f[1].to_string(), f[2].to_string())),
                2 => Some((f[0].to_string(), f[1].to_string())),
                _ => None,
            }
        })
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .into_iter()
        .filter(|(k, _)| k != "U" && k != "w" && k != "v")
        .map(|(_, n)| n.split('@').next().unwrap().to_string())
        .collect()
}

fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
        .into_iter()
        .map(|(_, n)| n.split('@').next().unwrap().to_string())
        .collect()
}

/// Symbols that legitimately come from libc / the platform runtime.
fn is_platform_symbol(n: &str) -> bool {
    const LIBC: &[&str] = &[
        "calloc", "free", "malloc", "realloc", "memcpy", "memmove", "memset", "memcmp",
        "__assert_fail", "abort", "raise", "write", "writev", "close", "open", "open64", "read",
        "fstat", "fstat64", "statx", "lseek", "lseek64", "sysconf", "getenv", "getcwd", "readlink",
        "gettid", "realpath", "free", "abort", "dlsym", "dladdr", "dlopen", "dlclose", "dlerror",
        "strlen", "bcmp", "posix_memalign", "pthread_self", "pthread_mutex_lock",
        "pthread_mutex_unlock", "pthread_mutex_trylock", "pthread_mutex_destroy",
        "pthread_rwlock_rdlock", "pthread_rwlock_unlock", "pthread_getattr_np",
        "pthread_attr_getstack", "pthread_attr_destroy", "pthread_key_create",
        "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "sigaltstack",
        "sigaction", "sigaddset", "sigemptyset", "sigaltstack", "mmap", "mmap64", "munmap",
        "mprotect", "poll", "dl_iterate_phdr", "getrandom", "__errno_location", "__libc_start_main",
        "__tls_get_addr", "syscall", "_Unwind_Backtrace", "_Unwind_GetIP",
        "_Unwind_GetIPInfo", "_Unwind_Resume", "_Unwind_RaiseException",
        "_Unwind_DeleteException", "_Unwind_GetLanguageSpecificData", "_Unwind_GetRegionStart",
        "_Unwind_GetTextRelBase", "_Unwind_GetDataRelBase", "_Unwind_SetGR", "_Unwind_SetIP",
        "_Unwind_GetCFA", "_Unwind_FindEnclosingFunction", "_Unwind_Backtrace",
    ];
    LIBC.contains(&n)
        || n.starts_with("__")
        || n.starts_with("_ITM_")
        || n.starts_with("_Unwind")
        || n.starts_with("_ZN")
        || n.starts_with("pthread_")
        || n.starts_with("_dl")
        || n.starts_with("_r_debug")
        || n.starts_with("_rust")
        || n.starts_with("stat")
        || n.starts_with("gnu_")
        || n.starts_with("_edata")
}

#[test]
fn symbol_parity() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    println!("C   : {}", c.display());
    println!("Rust: {}", r.display());

    let cd = defined(&c);
    let rd = defined(&r);

    println!("\nC defined symbols ({}):", cd.len());
    for s in &cd {
        println!("  {s}{}", if rd.contains(s) { "" } else { "   <== MISSING IN RUST" });
    }

    let missing: Vec<&String> = cd.difference(&rd).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but not by the Rust .so: {missing:?}"
    );

    // The set of C exports is the whole documented surface; the Rust library may
    // add nothing that shadows a different C symbol name.
    let extra: Vec<&String> = rd
        .difference(&cd)
        .filter(|n| !is_platform_symbol(n))
        .collect();
    println!("\nRust-only defined symbols (informational): {extra:?}");

    // No dangling non-libc dependencies in the Rust .so.
    let ru: Vec<String> = undefined(&r)
        .into_iter()
        .filter(|n| !is_platform_symbol(n))
        .collect();
    println!("Rust undefined non-libc symbols: {ru:?}");
    assert!(ru.is_empty(), "Rust .so has undefined non-libc symbols: {ru:?}");
}

/// Every C export must also be reachable through `dlsym`, not merely present in
/// `nm` output (i.e. it must have default visibility and the right binding).
#[test]
fn dlsym_reachability() {
    let pair = common::load_pair();
    for lib in [&pair.c, &pair.rs] {
        assert!(!(lib.pinflate as usize == 0));
        assert!(!lib.error_reason.is_null());
        assert!(!lib.fixed_table.is_null());
        assert!(!lib.permutation_order.is_null());
        assert!(!lib.len_extra_bits.is_null());
        assert!(!lib.len_base.is_null());
        assert!(!lib.dist_extra_bits.is_null());
        assert!(!lib.dist_base.is_null());
        println!("{}: all 8 symbols resolved via dlsym", lib.label);
    }
}

/// The exported data tables must have byte-identical initial contents.
#[test]
fn global_table_contents_match() {
    let pair = common::load_pair();
    unsafe {
        let cmp_u8 = |name: &str, a: *const u8, b: *const u8, n: usize| {
            let x = std::slice::from_raw_parts(a, n);
            let y = std::slice::from_raw_parts(b, n);
            assert_eq!(x, y, "{name} differs\n C  ={x:?}\n Rust={y:?}");
        };
        let cmp_u32 = |name: &str, a: *const u32, b: *const u32, n: usize| {
            let x = std::slice::from_raw_parts(a, n);
            let y = std::slice::from_raw_parts(b, n);
            assert_eq!(x, y, "{name} differs\n C  ={x:?}\n Rust={y:?}");
        };
        cmp_u8("cp_fixed_table", pair.c.fixed_table, pair.rs.fixed_table, 320);
        cmp_u8(
            "cp_permutation_order",
            pair.c.permutation_order,
            pair.rs.permutation_order,
            19,
        );
        cmp_u8(
            "cp_len_extra_bits",
            pair.c.len_extra_bits,
            pair.rs.len_extra_bits,
            31,
        );
        cmp_u32("cp_len_base", pair.c.len_base, pair.rs.len_base, 31);
        cmp_u8(
            "cp_dist_extra_bits",
            pair.c.dist_extra_bits,
            pair.rs.dist_extra_bits,
            32,
        );
        cmp_u32("cp_dist_base", pair.c.dist_base, pair.rs.dist_base, 32);
        assert!((*pair.c.error_reason).is_null(), "C cp_error_reason not initially NULL");
        assert!((*pair.rs.error_reason).is_null(), "Rust cp_error_reason not initially NULL");
    }
    println!("all six exported tables byte-identical; cp_error_reason NULL in both");
}
