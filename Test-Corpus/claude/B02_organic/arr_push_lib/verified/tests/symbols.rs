//! Phase A / Phase D: every symbol the C `.so` exports must also be exported by
//! the Rust `.so`, under the exact same name.

mod common;
use common::*;

/// Parse `nm -D --defined-only` output into a sorted symbol-name list.
fn defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            match it.next() {
                Some(name) => Some((b.to_string(), name.to_string())),
                None => Some((a.to_string(), b.to_string())),
            }
        })
        .filter(|(kind, _)| kind == "T" || kind == "D" || kind == "B" || kind == "W")
        .map(|(_, n)| n)
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn c_so_exists() {
    assert!(
        c_so_path().exists(),
        "C shared library not built: {}\n\
         build with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        c_so_path().display()
    );
    assert!(rust_so_path().exists(), "{}", rust_so_path().display());
}

/// The symbol diff MUST be empty.
#[test]
fn symbol_diff_is_empty() {
    let c = defined_symbols(&c_so_path());
    let rs = defined_symbols(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}",
        c.len()
    );
    // sanity: we really did find the 16 documented symbols
    assert_eq!(c.len(), 16, "unexpected C symbol count: {c:?}");
    for s in SYMBOL_NAMES {
        assert!(c.contains(&s.to_string()), "C .so missing {s}");
    }
}

/// Independent of `nm`: `dlsym` every documented name out of BOTH libraries.
#[test]
fn c_symbols_all_present_in_rust() {
    for lib_path in [c_so_path(), rust_so_path()] {
        let lib = unsafe { libloading::Library::new(&lib_path) }.unwrap();
        for name in SYMBOL_NAMES {
            let mut n = name.to_string();
            n.push('\0');
            let sym: Result<libloading::Symbol<*const ()>, _> =
                unsafe { lib.get(n.as_bytes()) };
            assert!(
                sym.is_ok(),
                "{} does not export {name}",
                lib_path.display()
            );
        }
    }
}

/// No unresolved non-libc symbols in the Rust `.so`.
#[test]
fn rust_so_has_no_foreign_undefined_symbols() {
    let out = std::process::Command::new("nm")
        .args([
            "-D",
            "--undefined-only",
            rust_so_path().to_str().unwrap(),
        ])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let unexpected: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // everything must be glibc / libgcc / weak runtime hooks
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("__gmon_start__")
                || s.starts_with("_Unwind_")
                || s.starts_with("__cxa_")
                || s.starts_with("statx")
                || s.starts_with("gettid"))
        })
        .collect();
    assert!(unexpected.is_empty(), "unresolved symbols: {unexpected:?}");
}

/// The library loads and its entry points are callable through the `.so`.
#[test]
fn both_libraries_load_and_call() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        assert_eq_ctx((p.c.arr_push)(0), (p.rs.arr_push)(0), "arr_push(0)");
        let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, 0);
        let b = (p.rs.hash_bytes)(std::ptr::null_mut(), 0, 0);
        assert_eq_ctx(a, b, "hash_bytes(NULL,0,0)");
    }
}
