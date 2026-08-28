//! Shared harness: loads the C and Rust shared libraries via `libloading` and
//! drives `parse_number` through the FFI boundary on both sides.

#![allow(non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_double, c_int, c_uchar};
use std::path::PathBuf;

pub type cJSON_bool = c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

pub type ParseNumberFn =
    unsafe extern "C" fn(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool;

/// Result snapshot of one `parse_number` invocation: everything an external
/// caller can observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub ret: cJSON_bool,
    pub type_: c_int,
    pub valueint: c_int,
    /// Raw bits so that `-0.0` vs `0.0` and NaN payloads are compared exactly.
    pub valuedouble_bits: u64,
    pub offset: usize,
    /// The remaining `parse_buffer` fields must be left untouched.
    pub length: usize,
    pub depth: usize,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let root = workspace_root()
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf();
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn find_rust_so() -> PathBuf {
    // An explicit override wins (used by run_tests.sh, which rebuilds first).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
        return p;
    }

    // `cargo test` places the cdylib next to the test binaries' profile dir.
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test-bin>
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libdriver.so"));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libdriver.so"));
            }
        }
    }
    let target = workspace_root().join("target");
    candidates.push(target.join("debug/libdriver.so"));
    candidates.push(target.join("release/libdriver.so"));

    let found = candidates
        .iter()
        .find(|c| c.exists())
        .unwrap_or_else(|| panic!("Rust cdylib libdriver.so not found; looked in {candidates:?}"))
        .clone();

    assert_fresh(&found);
    found
}

/// Guard against silently testing a stale artifact: `cargo test` does not
/// relink the `cdylib`, so an old `libdriver.so` would be loaded even after
/// `src/lib.rs` changed.
fn assert_fresh(so: &std::path::Path) {
    let src = workspace_root().join("src/lib.rs");
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        mtime(so) >= mtime(&src),
        "stale Rust cdylib: {so:?} is older than {src:?}.\n\
         Run ./run_tests.sh (which rebuilds the cdylib) instead of a bare `cargo test`."
    );
}


pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c_parse_number: ParseNumberFn,
    rust_parse_number: ParseNumberFn,
}

impl Harness {
    pub fn new() -> Self {
        unsafe {
            let c_lib = Library::new(find_c_so()).expect("load C .so");
            let rust_lib = Library::new(find_rust_so()).expect("load Rust .so");

            let c_sym: Symbol<ParseNumberFn> = c_lib
                .get(b"parse_number\0")
                .expect("C .so exports parse_number");
            let rust_sym: Symbol<ParseNumberFn> = rust_lib
                .get(b"parse_number\0")
                .expect("Rust .so exports parse_number");

            let c_parse_number = *c_sym;
            let rust_parse_number = *rust_sym;

            Harness {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_parse_number,
                rust_parse_number,
            }
        }
    }

    /// Sentinel values so that an untouched `cJSON` is distinguishable.
    fn fresh_item() -> cJSON {
        cJSON {
            type_: -0x5A5A5A5,
            valueint: -0x3C3C3C3,
            valuedouble: f64::from_bits(0xDEAD_BEEF_DEAD_BEEF),
        }
    }

    fn invoke(f: ParseNumberFn, bytes: &[u8], length: usize, offset: usize, depth: usize) -> Outcome {
        let mut item = Self::fresh_item();
        let mut buf = parse_buffer {
            content: bytes.as_ptr(),
            length,
            offset,
            depth,
        };
        let ret = unsafe { f(&mut item, &mut buf) };
        Outcome {
            ret,
            type_: item.type_,
            valueint: item.valueint,
            valuedouble_bits: item.valuedouble.to_bits(),
            offset: buf.offset,
            length: buf.length,
            depth: buf.depth,
        }
    }

    /// Run one case on both implementations and assert byte-identical results.
    pub fn check_raw(&self, bytes: &[u8], length: usize, offset: usize, depth: usize) {
        let c = Self::invoke(self.c_parse_number, bytes, length, offset, depth);
        let r = Self::invoke(self.rust_parse_number, bytes, length, offset, depth);
        assert_eq!(
            c, r,
            "mismatch for input {:?} (len={length}, offset={offset}, depth={depth})\n  C   : {c:?}\n  Rust: {r:?}",
            String::from_utf8_lossy(bytes)
        );
    }

    /// Convenience: whole slice is the buffer, offset 0.
    pub fn check(&self, s: &[u8]) {
        self.check_raw(s, s.len(), 0, 0);
    }

    /// Exercise every offset from 0..=len for the same backing buffer.
    pub fn check_all_offsets(&self, s: &[u8]) {
        for off in 0..=s.len() {
            self.check_raw(s, s.len(), off, 7);
        }
    }

    /// Exercise truncated lengths, so `can_access_at_index` cuts the scan short.
    pub fn check_all_lengths(&self, s: &[u8]) {
        for len in 0..=s.len() {
            self.check_raw(s, len, 0, 3);
        }
    }

    /// NULL `input_buffer` -> both must return false without touching anything.
    pub fn check_null_input_buffer(&self) {
        let run = |f: ParseNumberFn| {
            let mut item = Self::fresh_item();
            let ret = unsafe { f(&mut item, std::ptr::null_mut()) };
            (ret, item.type_, item.valueint, item.valuedouble.to_bits())
        };
        assert_eq!(
            run(self.c_parse_number),
            run(self.rust_parse_number),
            "mismatch for NULL input_buffer"
        );
    }

    /// NULL `content` -> both must return false without touching anything.
    pub fn check_null_content(&self, length: usize, offset: usize) {
        let run = |f: ParseNumberFn| {
            let mut item = Self::fresh_item();
            let mut buf = parse_buffer {
                content: std::ptr::null(),
                length,
                offset,
                depth: 11,
            };
            let ret = unsafe { f(&mut item, &mut buf) };
            (
                ret,
                item.type_,
                item.valueint,
                item.valuedouble.to_bits(),
                buf.offset,
            )
        };
        assert_eq!(
            run(self.c_parse_number),
            run(self.rust_parse_number),
            "mismatch for NULL content (len={length}, offset={offset})"
        );
    }
}
