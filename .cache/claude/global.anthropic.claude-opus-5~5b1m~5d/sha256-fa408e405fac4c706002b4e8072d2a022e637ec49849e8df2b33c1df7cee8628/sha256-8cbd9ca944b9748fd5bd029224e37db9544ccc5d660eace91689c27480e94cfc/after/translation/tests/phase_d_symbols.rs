//! Phase D — symbol parity. Mechanically diffs `nm -D` on the two `.so`s and
//! additionally proves every C symbol is `dlsym`-able from the Rust `.so`.

mod harness;
use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` -> the set of exported symbol names.
fn exported(so: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let name = it.next()?;
                let kind = it.next()?;
                // T/t text, B/b bss, D/d data, R/r rodata — exclude weak/undef
                if "TBDR".contains(kind) {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect(),
    )
}

/// Symbols the Rust `.so` may export in addition to the C ones: toolchain and
/// libc/loader boilerplate, never library API.
fn is_toolchain_noise(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_R")
        || s.starts_with("__rust")
        || s.starts_with("rust_")
        || s.starts_with("_ITM_")
        || s.starts_with("__cxa")
        || s.starts_with("_Unwind")
        || s.starts_with("__gnu")
        || s.starts_with("__libc")
        || s.starts_with("__tls")
        || s.starts_with("__emutls")
        || s.starts_with("_fini")
        || s.starts_with("_init")
        || s.starts_with("__bss_start")
        || s.starts_with("_edata")
        || s.starts_with("_end")
        || s == "__odr_asan_gen"
}

/// The complete C ABI, transcribed from `SYMBOLS.md` so the test fails if the
/// C library ever grows a symbol nobody noticed.
const EXPECTED: &[&str] = &[
    "add_op",
    "multiply_op",
    "subtract_op",
    "divide_op",
    "modulo_op",
    "find_node_by_id",
    "add_tree_node",
    "calculate_tree_sum",
    "parse_operation",
    "get_operation_func",
    "inreftree",
    "node_table",
    "node_count",
];

#[test]
fn d01_symbol_diff_is_empty() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let (Some(c), Some(r)) = (exported(&c_so), exported(&r_so)) else {
        eprintln!("SKIP: `nm` unavailable");
        return;
    };

    let c_api: BTreeSet<&str> = c
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !is_toolchain_noise(s))
        .collect();

    // 1. the C .so must export exactly the ABI recorded in SYMBOLS.md
    let expected: BTreeSet<&str> = EXPECTED.iter().copied().collect();
    assert_eq!(
        c_api, expected,
        "the C .so's exported ABI changed; update SYMBOLS.md.\n\
         only in .so: {:?}\nonly in SYMBOLS.md: {:?}",
        c_api.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&c_api).collect::<Vec<_>>()
    );

    // 2. every C symbol must be exported by the Rust .so with the exact name
    let missing: Vec<&&str> = c_api.iter().filter(|s| !r.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "MISSING from the Rust .so ({}): {missing:?}\n\
         Rust exports: {:?}",
        missing.len(),
        r.iter().filter(|s| !is_toolchain_noise(s)).collect::<Vec<_>>()
    );
}

#[test]
fn d02_no_undefined_non_libc_symbols_in_rust_so() {
    let r_so = rust_so_path();
    let out = match Command::new("nm").args(["-D", "--undefined-only", "--format=posix"]).arg(&r_so).output() {
        Ok(o) if o.status.success() => o,
        _ => {
            eprintln!("SKIP: `nm` unavailable");
            return;
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        // drop the `@GLIBC_x.y` version suffix
        .map(|s| s.split('@').next().unwrap())
        .filter(|s| {
            // everything the Rust std runtime legitimately imports from libc /
            // libgcc / the loader
            !(s.starts_with("__")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind")
                || KNOWN_LIBC.contains(s))
        })
        .collect();
    assert!(bad.is_empty(), "unexpected undefined symbols in {}: {bad:?}", r_so.display());
}

const KNOWN_LIBC: &[&str] = &[
    "abort", "calloc", "free", "malloc", "realloc", "memcpy", "memmove", "memset", "memcmp",
    "bcmp", "strlen", "write", "writev", "close", "open", "open64", "read", "readlink", "mmap",
    "mmap64", "munmap", "mprotect", "sysconf", "getenv", "getcwd", "dl_iterate_phdr", "dlsym",
    "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy", "pthread_self",
    "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
    "pthread_mutex_destroy", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
    "pthread_rwlock_wrlock", "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
    "pthread_setspecific", "pthread_condattr_init", "pthread_condattr_setclock",
    "pthread_cond_init", "pthread_cond_destroy", "pthread_condattr_destroy", "sigaltstack",
    "sigaction", "sigemptyset", "sigaddset", "signal", "raise", "syscall", "poll", "gettid",
    "getpid", "sched_yield", "nanosleep", "clock_gettime", "futex", "madvise", "environ",
    "stat", "stat64", "lstat", "lstat64", "fstat", "fstat64", "unlink", "rename", "strerror_r",
    "lseek", "lseek64", "posix_memalign", "realpath", "statx", "aligned_alloc", "memrchr",
    "strchr", "strncpy", "fwrite", "fputs", "fflush", "exit", "dlopen", "dlclose", "dlerror",
];

/// Additionally: `RTLD_NOW` forces every relocation to be resolved at load time.
/// If the Rust `.so` referenced anything unavailable, this `dlopen` would fail.
#[test]
fn d02b_rust_so_loads_with_rtld_now() {
    let r_so = rust_so_path();
    unsafe {
        libloading::os::unix::Library::open(
            Some(&r_so),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen({}) failed: {e}", r_so.display()));
        libloading::os::unix::Library::open(
            Some(c_so_path()),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
        .expect("RTLD_NOW dlopen of the C .so failed");
    }
}

/// Belt-and-braces: `dlsym` every symbol from the Rust `.so` through libloading,
/// which is what an external consumer actually does.
#[test]
fn d03_every_symbol_is_dlsym_able_from_rust_so() {
    // `pair()` already resolves all 13 symbols from BOTH libraries in `Lib::open`
    // and panics naming any that is missing.
    let (p, _g) = fresh();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.r.name, "Rust");
    // sanity: the resolved pointers are distinct between the two libraries
    assert_ne!(p.c.add_op as usize, p.r.add_op as usize);
    assert_ne!(p.c.node_table as usize, p.r.node_table as usize);
    assert_ne!(p.c.node_count as usize, p.r.node_count as usize);
    // and the data objects have the sizes recorded in SYMBOLS.md
    assert_eq!(p.c.table_bytes().len(), 2600);
    assert_eq!(p.r.table_bytes().len(), 2600);
}

/// The `node_table` / `node_count` objects must be *writable* exports (`B`),
/// not read-only copies — the C code mutates them and so must the Rust.
#[test]
fn d04_data_symbols_are_writable_and_shared_with_the_code() {
    let (p, _g) = fresh();
    for lib in [&p.c, &p.r] {
        lib.set_count(0);
        let s = std::ffi::CString::new("probe").unwrap();
        unsafe { (lib.add_tree_node)(4242, 7, -1, s.as_ptr()) };
        assert_eq!(lib.count(), 1, "{}: node_count not updated in place", lib.name);
        assert_eq!(lib.table()[0].id, 4242, "{}: node_table not the same object", lib.name);
        // write through the exported object and let the library observe it
        let mut n = lib.table()[0];
        n.id = 99;
        lib.set_table(&[n]);
        assert_eq!(
            lib.find_index(99),
            Some(0),
            "{}: library does not see writes to the exported node_table",
            lib.name
        );
    }
    assert_state_eq(p, "d04");
}
