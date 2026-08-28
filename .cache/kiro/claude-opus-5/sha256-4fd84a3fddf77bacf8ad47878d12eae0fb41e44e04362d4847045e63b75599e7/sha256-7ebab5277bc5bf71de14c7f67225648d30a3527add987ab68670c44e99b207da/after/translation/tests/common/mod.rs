//! Shared harness: loads the C `libdriver.so` and the Rust `libdriver.so`
//! through `libloading` and calls both purely through their exported
//! C ABI symbols.

use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn free(p: *mut c_void);
}

pub type SearchAndReplaceFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

pub struct Impls {
    pub c_lib: Library,
    pub rust_lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

/// The Rust cdylib lives next to (or one directory above) the test binary.
/// `DRIVER_RUST_SO` overrides the search, which lets the same suite be run
/// against e.g. the release-profile artifact.
fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("test dir").to_path_buf();
    for _ in 0..4 {
        let candidate = dir.join("libdriver.so");
        if candidate.is_file() {
            return candidate;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    panic!(
        "could not locate the Rust libdriver.so starting from {}",
        exe.display()
    );
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        assert!(
            c_path.is_file(),
            "C shared library missing at {} - build it with cmake first",
            c_path.display()
        );
        unsafe {
            Impls {
                c_lib: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", c_path.display())),
                rust_lib: Library::new(&r_path)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", r_path.display())),
            }
        }
    })
}

fn sym<'a>(lib: &'a Library, name: &[u8]) -> Symbol<'a, SearchAndReplaceFn> {
    unsafe { lib.get(name).expect("symbol searchAndReplace") }
}

/// Result of one call: `None` for a NULL return, otherwise the returned
/// C string's bytes (NUL excluded).
#[derive(Debug, PartialEq, Eq)]
pub struct CallResult(pub Option<Vec<u8>>);

unsafe fn call(f: &Symbol<'_, SearchAndReplaceFn>, orig: &[u8], search: &[u8], value: &[u8]) -> CallResult {
    let o = nul(orig);
    let s = nul(search);
    let v = nul(value);
    unsafe {
        let p = f(o.as_ptr() as *const c_char, s.as_ptr() as *const c_char, v.as_ptr() as *const c_char);
        if p.is_null() {
            return CallResult(None);
        }
        let mut out = Vec::new();
        let mut i = 0usize;
        loop {
            let b = *(p as *const u8).add(i);
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
        }
        free(p as *mut c_void);
        CallResult(Some(out))
    }
}

fn nul(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

/// Call both implementations and assert byte-identical results.
pub fn assert_same(orig: &[u8], search: &[u8], value: &[u8]) {
    let i = impls();
    let cf = sym(&i.c_lib, b"searchAndReplace");
    let rf = sym(&i.rust_lib, b"searchAndReplace");
    let c = unsafe { call(&cf, orig, search, value) };
    let r = unsafe { call(&rf, orig, search, value) };
    assert_eq!(
        c,
        r,
        "mismatch for orig={:?} search={:?} value={:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(orig),
        String::from_utf8_lossy(search),
        String::from_utf8_lossy(value),
        c.0.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        r.0.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
    );
}
