//! Phase D — symbol parity between the reference C `.so` and the Rust `cdylib`,
//! plus a smoke test that both libraries can actually be `dlopen`ed and that all
//! eight exported symbols resolve through `libloading`.
//!
//! (The full differential suite lives in `tests/differential.rs`, which needs a
//! custom harness because some inputs abort the process.)

use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src").join("build");
    let mut best = None;
    for e in std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("{} missing — build the C library first", dir.display()))
        .flatten()
    {
        let p = e.path();
        let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if n.starts_with("lib") && n.ends_with(".so") {
            best = Some(p);
        }
    }
    best.expect("no lib*.so in c_src/build")
}

fn rust_so_path() -> PathBuf {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["debug", "release"] {
        for sub in ["", "deps"] {
            let mut p = manifest_dir().join("target").join(profile);
            if !sub.is_empty() {
                p = p.join(sub);
            }
            let p = p.join("libpinflate_lib.so");
            if let Ok(md) = std::fs::metadata(&p) {
                let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                    best = Some((t, p));
                }
            }
        }
    }
    best.map(|(_, p)| p).expect("libpinflate_lib.so not built — run `cargo build` first")
}

fn nm(p: &Path) -> BTreeMap<String, (String, u64)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(p)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", p.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (kind, name, size) = match f.len() {
            4 => (f[2], f[3], u64::from_str_radix(f[1], 16).unwrap_or(0)),
            3 => (f[1], f[2], 0u64),
            _ => continue,
        };
        map.insert(name.to_string(), (kind.to_string(), size));
    }
    map
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the same name, the same kind and — for data objects — the same size.
#[test]
fn symbol_parity() {
    let cs = nm(&c_so_path());
    let rs = nm(&rust_so_path());
    assert!(!cs.is_empty(), "the C .so exports nothing?");

    let mut problems = Vec::new();
    for (name, (kind, size)) in &cs {
        match rs.get(name) {
            None => problems.push(format!("MISSING from Rust .so: {name}")),
            Some((rkind, rsize)) => {
                if kind != rkind {
                    problems.push(format!("{name}: kind {kind} (C) vs {rkind} (Rust)"));
                }
                if kind != "T" && size != rsize {
                    problems.push(format!("{name}: size {size:#x} (C) vs {rsize:#x} (Rust)"));
                }
            }
        }
    }
    assert!(problems.is_empty(), "symbol diff is not empty:\n{}", problems.join("\n"));

    // the exact expected set, so a *new* C symbol cannot slip through unnoticed
    let expected = [
        "cp_dist_base",
        "cp_dist_extra_bits",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_len_base",
        "cp_len_extra_bits",
        "cp_permutation_order",
        "pinflate",
    ];
    let got: Vec<&str> = cs.keys().map(|s| s.as_str()).collect();
    assert_eq!(got, expected, "the C .so's symbol set changed");
}

/// No undefined symbol of the Rust `.so` may be anything but a libc/libgcc
/// import — verified by the fact that `dlopen` with immediate binding succeeds.
#[test]
fn both_libraries_load_and_resolve() {
    for p in [c_so_path(), rust_so_path()] {
        unsafe {
            // RTLD_NOW | RTLD_LOCAL: resolves every relocation up front, so an
            // unresolvable symbol fails here.
            let lib = libloading::os::unix::Library::open(Some(&p), 0x2 /* RTLD_NOW */)
                .unwrap_or_else(|e| panic!("dlopen({}, RTLD_NOW) failed: {e}", p.display()));
            let f: libloading::os::unix::Symbol<
                unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int,
            > = lib.get(b"pinflate\0").expect("pinflate");
            for name in [
                &b"cp_error_reason\0"[..],
                &b"cp_fixed_table\0"[..],
                &b"cp_permutation_order\0"[..],
                &b"cp_len_extra_bits\0"[..],
                &b"cp_len_base\0"[..],
                &b"cp_dist_extra_bits\0"[..],
                &b"cp_dist_base\0"[..],
            ] {
                let s: libloading::os::unix::Symbol<*mut u8> =
                    lib.get(name).unwrap_or_else(|e| {
                        panic!("{}: {} missing: {e}", p.display(), String::from_utf8_lossy(name))
                    });
                assert!(!(*s).is_null());
            }

            // a trivial happy path through the FFI: one fixed block, "A" + EOB.
            // bits: 1 (bfinal), 01 (btype=fixed), 01110001 (code 48+65 for 'A',
            // MSB first), 0000000 (end of block) => 0x73 0x04 0x00 0x00
            let mut input: Vec<u8> = vec![0x73, 0x04, 0x00, 0x00];
            let mut out = vec![0u8; 16];
            let er: libloading::os::unix::Symbol<*mut *const c_char> =
                lib.get(b"cp_error_reason\0").unwrap();
            **er = std::ptr::null();
            let ret = f(
                input.as_mut_ptr() as *mut c_void,
                input.len() as c_int,
                out.as_mut_ptr() as *mut c_void,
                1,
            );
            assert_eq!(ret, 1, "{}: pinflate failed on a 1-byte stream", p.display());
            assert_eq!(&out[..1], b"A", "{}: wrong output", p.display());
        }
    }
}
