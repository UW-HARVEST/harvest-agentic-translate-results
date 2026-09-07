//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name, and must be reachable via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn so_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = std::env::var("C_HELLO_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../c_src/build/libhello.so"));
    let rust = std::env::var("RUST_HELLO_SO").map(PathBuf::from).unwrap_or_else(|_| {
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for profile in ["debug", "release"] {
            let p = manifest.join("target").join(profile).join("libhello.so");
            if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                if best.as_ref().map_or(true, |(bt, _)| t > *bt) {
                    best = Some((t, p));
                }
            }
        }
        best.expect("Rust cdylib not built").1
    });
    (c, rust)
}

/// Linker/toolchain-provided entries that are not part of the library API.
fn is_infrastructure(name: &str) -> bool {
    matches!(
        name,
        "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "__gmon_start__"
    ) || name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("__gnu_")
        || name.starts_with("_Jv_")
        || name.starts_with("__deregister_frame")
        || name.starts_with("__register_frame")
}

/// Global/weak *defined* dynamic symbols, per `nm -D --defined-only`.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` — is binutils installed?");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // T/t text, D/d data, B/b bss, W/w weak, i/u indirect
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "i") {
                return None;
            }
            if is_infrastructure(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let (c_so, rust_so) = so_paths();
    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);

    assert!(
        c_syms.contains("helloworld"),
        "sanity: C .so must export `helloworld`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   ({}): {:?}\nRust ({}): {:?}",
        missing.len(),
        missing,
        c_so.display(),
        c_syms,
        rust_so.display(),
        rust_syms
    );
}

#[test]
fn d02_every_c_symbol_is_dlsym_reachable_in_rust() {
    let (c_so, rust_so) = so_paths();
    let c_syms = defined_dynamic_symbols(&c_so);
    unsafe {
        let lib = libloading::Library::new(&rust_so).expect("dlopen Rust .so");
        for name in &c_syms {
            let mut key = name.clone().into_bytes();
            key.push(0);
            let sym: Result<libloading::Symbol<*const ()>, _> = lib.get(&key);
            assert!(
                sym.is_ok(),
                "symbol `{name}` present in the C .so is not dlsym-reachable in {}",
                rust_so.display()
            );
        }
    }
}

#[test]
fn d03_rust_so_has_no_unresolved_non_libc_symbols() {
    let (_, rust_so) = so_paths();
    // A successful dlopen with RTLD_NOW proves every undefined symbol resolves
    // against the already-loaded libc/libgcc.
    unsafe {
        let path = std::ffi::CString::new(rust_so.to_str().unwrap()).unwrap();
        let h = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if h.is_null() {
            let err = libc::dlerror();
            let msg = if err.is_null() {
                "unknown".to_string()
            } else {
                std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            panic!("RTLD_NOW dlopen of {} failed: {msg}", rust_so.display());
        }
        libc::dlclose(h);
    }
}

#[test]
fn d04_no_stubbed_symbol() {
    // A stub that merely satisfies `nm` would not produce the C output; assert the
    // exported Rust symbol really does the work.
    let _g = fd_lock();
    let f = helloworld_of(Impl::Rust);
    let cap = capture_stdout("d04", Buffering::Default, Flush::Stdout, &[], false, || {
        vec![unsafe { f() }]
    });
    assert_eq!(cap.bytes, EXPECTED_LINE, "Rust `helloworld` is a stub / wrong output");
    assert_eq!(cap.rets, vec![0]);
}
