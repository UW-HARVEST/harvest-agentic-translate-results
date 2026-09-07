//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::{c_so_path, nm_dynamic_defined, rust_so_path};

#[test]
fn parity_every_c_export_exists_in_rust() {
    let c = nm_dynamic_defined(&c_so_path());
    let rust = nm_dynamic_defined(&rust_so_path());

    let missing: Vec<&String> = c.iter().filter(|s| !rust.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing exports present in the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rust:?}"
    );
    // Guard the test: the C library really does export the API under test.
    for expected in ["driver", "printLine"] {
        assert!(c.contains(&expected.to_string()), "C .so lost `{expected}`");
        assert!(rust.contains(&expected.to_string()), "Rust .so lost `{expected}`");
    }
}

#[test]
fn parity_rust_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol in the Rust .so must be resolvable by the dynamic
    // loader, i.e. it must come from the system libraries. `dlopen` succeeding
    // with RTLD_NOW is the mechanical proof of that.
    let path = rust_so_path();
    let cstr = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        // RTLD_NOW forces immediate resolution of every relocation.
        let h = libc::dlopen(cstr.as_ptr(), libc::RTLD_NOW);
        if h.is_null() {
            let err = libc::dlerror();
            let msg = if err.is_null() {
                String::from("unknown")
            } else {
                std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            panic!("Rust .so has unresolved symbols: {msg}");
        }
        libc::dlclose(h);
    }
}

#[test]
fn parity_no_stub_like_exports() {
    // A stub that merely satisfies `nm` would not produce output; both real
    // entry points must actually do work. (Checked against the Rust .so only —
    // the C is ground truth.)
    let libs = common::Libs::load();
    let out = libs.call_driver(common::Impl::Rust, 5);
    assert_eq!(out, b"AAAAA\n".to_vec(), "Rust `driver` export looks like a stub");
    let out = common::capture_stdout(|| {
        let p = libs.print_line(common::Impl::Rust);
        unsafe { p(b"hi\0".as_ptr() as *const std::ffi::c_char) }
    });
    assert_eq!(out, b"hi\n".to_vec(), "Rust `printLine` export looks like a stub");
}

#[test]
fn parity_project_builds_no_binary_so_no_stdout_diff_needed() {
    // Documents the "if the project builds a binary, compare stdout" gate:
    // neither build system produces an executable.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; a stdout-vs-stdout driver comparison must be added"
    );
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!cargo.contains("[[bin]]"), "the crate now builds a binary; add a stdout comparison");
    assert!(!root.join("src/main.rs").exists(), "src/main.rs appeared; add a stdout comparison");
}
