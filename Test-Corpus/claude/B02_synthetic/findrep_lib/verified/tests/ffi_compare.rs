// Integration tests that load BOTH the C and Rust shared libraries via
// `libloading` and compare their outputs through the FFI boundary.
//
// We never call Rust functions directly — only through the loaded .so —
// to ensure the `#[no_mangle]` exports behave identically to the C ones.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    // Cargo places the cdylib in the standard target/<profile>/ dir.
    // CARGO_TARGET_DIR may override this.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"));

    // Tests are usually built in debug profile.
    let debug = target_dir.join("debug").join("libfindrep_lib.so");
    let release = target_dir.join("release").join("libfindrep_lib.so");
    if debug.exists() {
        debug
    } else {
        release
    }
}

unsafe fn load_pair() -> (Library, Library) {
    let c = unsafe { Library::new(c_so_path()) }.expect("failed to load C .so");
    let r = unsafe { Library::new(rust_so_path()) }.expect("failed to load Rust .so");
    (c, r)
}

// Function-pointer typedefs.
type IntInt2Int = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Int2Int = unsafe extern "C" fn(c_int) -> c_int;
type ProcOctal = unsafe extern "C" fn(*mut c_char, c_int);
type FindReplace = unsafe extern "C" fn(*mut c_char, c_int);
type Findrep = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get::<T>(name) }.unwrap_or_else(|e| {
        panic!(
            "missing symbol {:?}: {}",
            std::str::from_utf8(name).unwrap(),
            e
        )
    })
}

fn cstr_to_vec(buf: &[c_char]) -> Vec<u8> {
    let mut out = Vec::new();
    for &b in buf {
        if b == 0 {
            break;
        }
        out.push(b as u8);
    }
    out
}

#[test]
fn test_validate_and_normalize() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<Int2Int> = sym(&c, b"validate_and_normalize");
        let r_fn: Symbol<Int2Int> = sym(&r, b"validate_and_normalize");

        // Cover boundary, zero, negative, in-range, and over-range cases.
        let inputs: &[c_int] = &[
            0, 1, 2, 63, 64, 65, 100, 256, 510, 511, 512, 1000, -1, -100, -1000,
            i32::MIN, i32::MAX, 0o100, 0o777,
        ];
        for &v in inputs {
            assert_eq!(c_fn(v), r_fn(v), "validate_and_normalize({}) mismatch", v);
        }
    }
}

#[test]
fn test_add_to_accumulator() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<IntInt2Int> = sym(&c, b"add_to_accumulator");
        let r_fn: Symbol<IntInt2Int> = sym(&r, b"add_to_accumulator");

        let inputs: &[(c_int, c_int)] =
            &[(0, 0), (1, 2), (-3, 4), (10, 20), (i32::MAX, 1), (-1, -2)];
        for &(a, b) in inputs {
            assert_eq!(c_fn(a, b), r_fn(a, b), "add({},{}) mismatch", a, b);
        }
    }
}

#[test]
fn test_multiply_with_multiplier() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<IntInt2Int> = sym(&c, b"multiply_with_multiplier");
        let r_fn: Symbol<IntInt2Int> = sym(&r, b"multiply_with_multiplier");

        // Avoid huge values that overflow into UB-territory in C; use moderate.
        let inputs: &[(c_int, c_int)] = &[(0, 0), (1, 1), (2, 3), (-1, 5), (4, 4), (1, 1)];
        for &(a, b) in inputs {
            assert_eq!(
                c_fn(a, b),
                r_fn(a, b),
                "multiply({},{}) mismatch",
                a,
                b
            );
        }
    }
}

#[test]
fn test_subtract_from_accumulator() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<IntInt2Int> = sym(&c, b"subtract_from_accumulator");
        let r_fn: Symbol<IntInt2Int> = sym(&r, b"subtract_from_accumulator");

        let inputs: &[(c_int, c_int)] =
            &[(0, 0), (5, 3), (-1, -2), (100, 50), (10, 20), (1, 1)];
        for &(a, b) in inputs {
            assert_eq!(c_fn(a, b), r_fn(a, b), "subtract({},{}) mismatch", a, b);
        }
    }
}

#[test]
fn test_divide_multiplier() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<IntInt2Int> = sym(&c, b"divide_multiplier");
        let r_fn: Symbol<IntInt2Int> = sym(&r, b"divide_multiplier");

        // Divide-by-zero is explicitly skipped in the implementation.
        let inputs: &[(c_int, c_int)] =
            &[(0, 0), (10, 2), (-10, 3), (5, 1), (7, 0), (100, 4)];
        for &(a, b) in inputs {
            assert_eq!(c_fn(a, b), r_fn(a, b), "divide({},{}) mismatch", a, b);
        }
    }
}

#[test]
fn test_process_octal_string() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<ProcOctal> = sym(&c, b"process_octal_string");
        let r_fn: Symbol<ProcOctal> = sym(&r, b"process_octal_string");

        let inputs: &[c_int] = &[0, 1, 7, 8, 0o123, 100, 1000, 0o777, 65535, 1];
        for &v in inputs {
            let mut c_buf = [0 as c_char; 100];
            let mut r_buf = [0 as c_char; 100];
            c_fn(c_buf.as_mut_ptr(), v);
            r_fn(r_buf.as_mut_ptr(), v);
            let cs = cstr_to_vec(&c_buf);
            let rs = cstr_to_vec(&r_buf);
            assert_eq!(cs, rs, "process_octal_string({}) mismatch", v);
        }
    }
}

#[test]
fn test_find_and_replace_char() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<FindReplace> = sym(&c, b"find_and_replace_char");
        let r_fn: Symbol<FindReplace> = sym(&r, b"find_and_replace_char");

        let cases: &[(&[u8], c_int)] = &[
            (b"hello world", b'o' as c_int),
            (b"hello world", b'z' as c_int),
            (b"", b'a' as c_int),
            (b"aaaa", b'a' as c_int),
            (b"abcabc", b'b' as c_int),
            (b"x", b'x' as c_int),
            (b"foo bar baz", b' ' as c_int),
        ];
        for (s, ch) in cases {
            let mut c_buf = [0 as c_char; 128];
            let mut r_buf = [0 as c_char; 128];
            for (i, b) in s.iter().enumerate() {
                c_buf[i] = *b as c_char;
                r_buf[i] = *b as c_char;
            }
            c_fn(c_buf.as_mut_ptr(), *ch);
            r_fn(r_buf.as_mut_ptr(), *ch);
            assert_eq!(
                cstr_to_vec(&c_buf),
                cstr_to_vec(&r_buf),
                "find_and_replace_char on {:?} ch={} mismatch",
                std::str::from_utf8(s).unwrap_or(""),
                *ch
            );
        }
    }
}

#[test]
fn test_findrep_full_matrix() {
    unsafe {
        let (c, r) = load_pair();
        let c_fn: Symbol<Findrep> = sym(&c, b"findrep");
        let r_fn: Symbol<Findrep> = sym(&r, b"findrep");

        // Run a sequence of calls and ensure each produces matching output.
        // Static state will accumulate identically in both libraries since
        // they're independent loads with their own memory.
        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 1, 1, 1),
            (1, 0, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 1, 0),
            (0, 0, 0, 1),
            (1, 2, 3, 4),
            (10, 20, 30, 40),
            (100, 200, 300, 400),
            (-1, -2, -3, -4),
            (1000, 1, 1, 1),
            (5, 10, 15, 20),
            (64, 128, 256, 512),
            (i32::MAX, 1, 1, 1),
            (50, 50, 50, 50),
            (0, 0, 0, 0),
            (1, 0, 0, 0),
        ];
        for (i, &(a, b, cc, d)) in cases.iter().enumerate() {
            let cv = c_fn(a, b, cc, d);
            let rv = r_fn(a, b, cc, d);
            assert_eq!(
                cv, rv,
                "findrep #{i} ({a},{b},{cc},{d}) -> C={cv} Rust={rv}"
            );
        }
    }
}

#[test]
fn test_exported_symbol_parity() {
    // Compare the public dynamic symbol set: every C export must exist in the
    // Rust .so. We don't care about additional Rust-side symbols.
    use std::process::Command;

    fn dynamic_symbols(path: &PathBuf) -> Vec<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("nm failed");
        let s = String::from_utf8_lossy(&out.stdout);
        let mut syms: Vec<String> = s
            .lines()
            .filter_map(|line| {
                // format: ADDR T name
                let mut parts = line.split_whitespace();
                let _addr = parts.next()?;
                let kind = parts.next()?;
                let name = parts.next()?;
                // We only want function/data exports the C lib defines as user-visible.
                if matches!(kind, "T" | "D" | "B" | "R") {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        syms.sort();
        syms.dedup();
        syms
    }

    // System / linker-generated symbols we don't expect Rust to export.
    let ignore: &[&str] = &[
        "__bss_start",
        "_edata",
        "_end",
        "_fini",
        "_init",
        "__libc_start_main",
        "__cxa_finalize",
    ];

    let c_syms = dynamic_symbols(&c_so_path());
    let r_syms = dynamic_symbols(&rust_so_path());

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !ignore.contains(&s.as_str()) && !r_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so missing C-exported symbols: {:?}",
        missing
    );
}
