//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__gmon"))
        .collect()
}

fn undefined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Anti-vacuity guard: the two `.so`s under test must really be two different
/// libraries (a path mix-up would make every differential test pass trivially).
#[test]
fn d0_two_distinct_libraries_are_loaded() {
    let c = common::c_impl();
    let r = common::rust_impl();
    assert_ne!(c.path, r.path, "C and Rust .so paths are identical");
    assert_eq!(
        c.path.file_name().unwrap().to_str().unwrap(),
        "libtranslated_rust.so",
        "unexpected C .so filename"
    );
    assert_eq!(
        r.path.file_name().unwrap().to_str().unwrap(),
        "libgotomach_lib.so",
        "unexpected Rust .so filename"
    );
    for (name, cf, rf) in [
        ("gotomach", c.gotomach as usize, r.gotomach as usize),
        (
            "process_value",
            c.process_value as usize,
            r.process_value as usize,
        ),
        (
            "double_value",
            c.double_value as usize,
            r.double_value as usize,
        ),
        (
            "triple_value",
            c.triple_value as usize,
            r.triple_value as usize,
        ),
    ] {
        assert_ne!(
            cf, rf,
            "{name} resolved to the SAME address in both libraries - only one .so is loaded"
        );
    }
    eprintln!("C   .so: {}", c.path.display());
    eprintln!("Rust.so: {}", r.path.display());
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();
    let c_syms = defined_dynamic_symbols(&c_path);
    let r_syms = defined_dynamic_symbols(&r_path);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C {} but MISSING from Rust {}: {:?}\nC: {:?}\nRust: {:?}",
        c_path.display(),
        r_path.display(),
        missing,
        c_syms,
        r_syms
    );
    assert!(
        c_syms.len() >= 4,
        "sanity: expected at least 4 C symbols, got {c_syms:?}"
    );
}

#[test]
fn d2_expected_symbol_set() {
    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let expected: BTreeSet<String> = common::PUBLIC_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "C .so symbol set changed; update SYMBOLS.md"
    );

    // The `static` helpers must stay private in Rust too, matching C.
    let r_syms = defined_dynamic_symbols(&common::rust_so_path());
    for hidden in [
        "is_valid_state",
        "check_char_flag",
        "init_processor",
        "cleanup_processor",
    ] {
        assert!(
            !r_syms.contains(hidden),
            "{hidden} is `static` in C and must not be exported by Rust"
        );
    }
}

#[test]
fn d3_all_symbols_resolvable_via_dlsym_in_both() {
    // Loading both impls already dlsym()s all four symbols and panics if any
    // is missing; this makes that explicit and independent of other tests.
    let c = common::c_impl();
    let r = common::rust_impl();
    let (c_ret, _) = common::capture_stdout(|| unsafe {
        (
            (c.gotomach)(3, 1, 0, i32::MAX),
            (c.process_value)(1, 0, std::ptr::null_mut()),
            (c.double_value)(1, 0, std::ptr::null_mut()),
            (c.triple_value)(1, 0, std::ptr::null_mut()),
        )
    });
    let (r_ret, _) = common::capture_stdout(|| unsafe {
        (
            (r.gotomach)(3, 1, 0, i32::MAX),
            (r.process_value)(1, 0, std::ptr::null_mut()),
            (r.double_value)(1, 0, std::ptr::null_mut()),
            (r.triple_value)(1, 0, std::ptr::null_mut()),
        )
    });
    assert_eq!(c_ret, r_ret, "smoke-test results differ");
}

#[test]
fn d4_rust_has_no_non_libc_undefined_symbols() {
    let r = undefined_dynamic_symbols(&common::rust_so_path());
    // Everything the Rust cdylib imports must come from libc / the C++ unwinder
    // / the dynamic loader — i.e. nothing from the translated library itself.
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_", "pthread_"];
    let allowed_exact: BTreeSet<&str> = [
        "malloc", "free", "calloc", "realloc", "posix_memalign", "printf", "puts", "fwrite",
        "memcpy", "memmove", "memset", "memcmp", "bcmp", "strlen", "abort", "getenv", "getcwd",
        "readlink", "realpath", "open", "open64", "close", "read", "write", "writev", "lseek",
        "lseek64", "fstat", "fstat64", "stat", "stat64", "statx", "mmap", "mmap64", "munmap",
        "mprotect", "syscall", "sysconf", "dl_iterate_phdr", "dladdr", "gettid", "sigaction",
        "sigaltstack", "getpid", "poll", "pipe2", "fcntl", "environ", "signal", "raise",
        "pthread_self",
    ]
    .into_iter()
    .collect();

    let leftovers: Vec<&String> = r
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "unexpected undefined (non-libc) symbols in Rust .so: {leftovers:?}"
    );
}

/// Memory-behaviour parity: `gotomach` must release everything it allocates
/// (the C `cleanup:` block frees `temp_buffer` and calls `cleanup_processor`).
/// A missing `free` in the Rust translation, or a double free, is not visible
/// in the return value, so check the glibc heap accounting directly.
#[test]
fn d5_no_heap_growth_in_either_implementation() {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Mallinfo2 {
        arena: usize,
        ordblks: usize,
        smblks: usize,
        hblks: usize,
        hblkhd: usize,
        usmblks: usize,
        fsmblks: usize,
        uordblks: usize,
        fordblks: usize,
        keepcost: usize,
    }
    type Mallinfo2Fn = unsafe extern "C" fn() -> Mallinfo2;

    let this = libloading::os::unix::Library::this();
    let mi: Mallinfo2Fn = match unsafe { this.get::<Mallinfo2Fn>(b"mallinfo2\0") } {
        Ok(s) => *s,
        Err(_) => {
            eprintln!("mallinfo2 unavailable; skipping heap-growth check");
            return;
        }
    };

    let args = [
        common::Args::new(65535, 12345, 0, i32::MAX),
        common::Args::new(65535, 7, 1, 1000),
        common::Args::new(4096, 999, 2, i32::MIN),
        common::Args::new(0, 0, 3, 0),
        common::Args::new(-1, 0, 0, 0),
        common::Args::new(10, 99999, 0, 0),
    ];

    for im in [common::c_impl(), common::rust_impl()] {
        // warm up so the arena is already grown
        common::capture_stdout(|| {
            for a in &args {
                unsafe { (im.gotomach)(a.iterations, a.seed, a.mode, a.threshold) };
            }
        });
        let before = unsafe { mi() }.uordblks;
        common::capture_stdout(|| {
            for _ in 0..200 {
                for a in &args {
                    unsafe { (im.gotomach)(a.iterations, a.seed, a.mode, a.threshold) };
                }
            }
        });
        let after = unsafe { mi() }.uordblks;
        let growth = after as i64 - before as i64;
        // 200 * (256 KiB + 256 KiB) would leak > 100 MB; allow a small slack for
        // unrelated allocator bookkeeping.
        assert!(
            growth < 1 << 20,
            "{}: heap grew by {growth} bytes over 1200 gotomach calls - allocations are not being freed",
            im.name
        );
    }
}
