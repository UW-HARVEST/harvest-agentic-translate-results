//! Shared harness: loads the C and Rust shared libraries via `libloading`
//! and calls every exported symbol purely through the FFI boundary.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct regmatch_t {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct os_data {
    pub os_name: *mut c_char,
    pub os_version: *mut c_char,
    pub os_major: *mut c_char,
    pub os_minor: *mut c_char,
    pub os_codename: *mut c_char,
    pub os_platform: *mut c_char,
    pub os_build: *mut c_char,
    pub os_uname: *mut c_char,
    pub os_arch: *mut c_char,
}

impl os_data {
    pub fn zeroed() -> Self {
        // SAFETY: an all-zero os_data is a valid struct of nine NULL pointers.
        unsafe { std::mem::zeroed() }
    }

    pub fn fields(&self) -> [*mut c_char; 9] {
        [
            self.os_name,
            self.os_version,
            self.os_major,
            self.os_minor,
            self.os_codename,
            self.os_platform,
            self.os_build,
            self.os_uname,
            self.os_arch,
        ]
    }
}

pub const FIELD_NAMES: [&str; 9] = [
    "os_name",
    "os_version",
    "os_major",
    "os_minor",
    "os_codename",
    "os_platform",
    "os_build",
    "os_uname",
    "os_arch",
];

/// Snapshot of an `os_data` after a call: each field as owned bytes, or `None`
/// for NULL. Lets us compare the two implementations without holding on to
/// pointers into either library's heap.
pub type Snapshot = [Option<Vec<u8>>; 9];

unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) } as u8;
        if b == 0 {
            break;
        }
        out.push(b);
        i += 1;
    }
    Some(out)
}

pub fn snapshot(osd: &os_data) -> Snapshot {
    let f = osd.fields();
    std::array::from_fn(|i| unsafe { read_cstr(f[i]) })
}

unsafe extern "C" {
    fn free(p: *mut c_void);
}

/// Release every non-NULL field. Both libraries allocate with the process's
/// single `malloc`, so freeing here is correct for both.
pub fn free_fields(osd: &os_data) {
    for p in osd.fields() {
        if !p.is_null() {
            unsafe { free(p as *mut c_void) };
        }
    }
}

type ParseUnameFn = unsafe extern "C" fn(*mut c_char, *mut os_data);
type GetOsArchFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type WRegexecFn = unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut regmatch_t) -> c_int;

pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: PathBuf) -> Impl {
        assert!(path.exists(), "{} not found at {}", name, path.display());
        // The C library declares SONAME "libdriver.so" and the Rust cdylib has
        // the same file name. Copy each to a uniquely named file so the dynamic
        // loader keeps them as two distinct objects with independent symbols.
        let unique = std::env::temp_dir().join(format!(
            "libdriver_{}_{}_{}.so",
            name.to_ascii_lowercase(),
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::copy(&path, &unique)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", path.display(), unique.display()));
        let lib = unsafe { Library::new(&unique) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", unique.display()));
        let _ = std::fs::remove_file(&unique);
        Impl { name, lib }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "{} does not export {}: {e}",
                self.name,
                String::from_utf8_lossy(name)
            )
        })
    }

    /// True if `dlsym` resolves `name` (which must be NUL-terminated).
    pub fn has_symbol(&self, name: &[u8]) -> bool {
        unsafe { self.lib.get::<*const c_void>(name) }.is_ok()
    }

    /// `parse_uname_string(buf, &osd)`. Returns the mutated buffer plus a
    /// snapshot of the filled struct.
    pub fn parse_uname_string(&self, input: &[u8]) -> (Vec<u8>, Snapshot) {
        let f: Symbol<ParseUnameFn> = self.sym(b"parse_uname_string\0");
        let mut buf: Vec<u8> = input.to_vec();
        buf.push(0);
        let mut osd = os_data::zeroed();
        unsafe { f(buf.as_mut_ptr() as *mut c_char, &mut osd) };
        let snap = snapshot(&osd);
        free_fields(&osd);
        (buf, snap)
    }

    /// `parse_uname_string(buf, NULL)` — must be a no-op.
    pub fn parse_uname_string_null_osd(&self, input: &[u8]) -> Vec<u8> {
        let f: Symbol<ParseUnameFn> = self.sym(b"parse_uname_string\0");
        let mut buf: Vec<u8> = input.to_vec();
        buf.push(0);
        unsafe { f(buf.as_mut_ptr() as *mut c_char, std::ptr::null_mut()) };
        buf
    }

    pub fn get_os_arch(&self, input: &[u8]) -> Option<Vec<u8>> {
        let f: Symbol<GetOsArchFn> = self.sym(b"get_os_arch\0");
        let mut buf: Vec<u8> = input.to_vec();
        buf.push(0);
        let p = unsafe { f(buf.as_mut_ptr() as *mut c_char) };
        let out = unsafe { read_cstr(p) };
        if !p.is_null() {
            unsafe { free(p as *mut c_void) };
        }
        out
    }

    /// `w_regexec`, with `nmatch` match slots. Returns the result and the
    /// match array as the library left it.
    pub fn w_regexec(
        &self,
        pattern: Option<&[u8]>,
        string: Option<&[u8]>,
        nmatch: usize,
        slots: usize,
    ) -> (c_int, Vec<regmatch_t>) {
        let f: Symbol<WRegexecFn> = self.sym(b"w_regexec\0");

        let pat_buf = pattern.map(|p| {
            let mut v = p.to_vec();
            v.push(0);
            v
        });
        let str_buf = string.map(|s| {
            let mut v = s.to_vec();
            v.push(0);
            v
        });

        let pat_ptr = pat_buf
            .as_ref()
            .map_or(std::ptr::null(), |v| v.as_ptr() as *const c_char);
        let str_ptr = str_buf
            .as_ref()
            .map_or(std::ptr::null(), |v| v.as_ptr() as *const c_char);

        // Sentinel fill so we can see exactly which slots were written.
        let mut m = vec![regmatch_t { rm_so: -7, rm_eo: -7 }; slots];
        let res = unsafe { f(pat_ptr, str_ptr, nmatch, m.as_mut_ptr()) };
        (res, m)
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// The Rust cdylib, as built for this test run.
///
/// `DRIVER_RUST_SO` overrides the path, so the same suite can be re-run against
/// a differently-built artifact (e.g. the `release` profile cdylib).
pub fn rust_so() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // tests live in <target>/<profile>/deps/, so the cdylib is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let c = repo_root().join("translation").join(p);
        if c.exists() {
            return c;
        }
    }
    panic!(
        "Rust libdriver.so not found (looked in {})",
        profile_dir.display()
    );
}

pub fn c_so() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

pub fn load_both() -> (Impl, Impl) {
    (
        Impl::open("C", c_so()),
        Impl::open("Rust", rust_so()),
    )
}

pub fn show(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "NULL".to_string(),
        Some(v) => format!("{:?}", String::from_utf8_lossy(v)),
    }
}
