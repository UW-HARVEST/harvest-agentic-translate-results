//! Shared differential-testing harness.
//!
//! Loads BOTH the C `libdriver.so` and the Rust `libdriver.so` through
//! `libloading` and calls `parse_number` across the FFI boundary in both.
//! No Rust function is ever called directly.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type cJSON_bool = std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ParseBuffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CJson {
    pub type_: std::ffi::c_int,
    pub valueint: std::ffi::c_int,
    pub valuedouble: f64,
}

impl CJson {
    pub fn garbage() -> Self {
        CJson {
            type_: -0x5A5A_5A5A,
            valueint: 0x1234_5678,
            valuedouble: f64::from_bits(0x7FF8_0000_DEAD_BEEF),
        }
    }
}

/// Everything observable after a `parse_number` call.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub ret: cJSON_bool,
    pub type_: std::ffi::c_int,
    pub valueint: std::ffi::c_int,
    /// raw bits so NaN / -0.0 compare exactly
    pub valuedouble_bits: u64,
    pub buf_content_is_null: bool,
    pub buf_length: usize,
    pub buf_offset: usize,
    pub buf_depth: usize,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ ret: {}, type: {}, valueint: {}, valuedouble: {:?} (bits {:#018x}), \
             buf: {{ content_null: {}, length: {}, offset: {}, depth: {} }} }}",
            self.ret,
            self.type_,
            self.valueint,
            f64::from_bits(self.valuedouble_bits),
            self.valuedouble_bits,
            self.buf_content_is_null,
            self.buf_length,
            self.buf_offset,
            self.buf_depth,
        )
    }
}

type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> cJSON_bool;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    parse_number: ParseNumberFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {name} from {path:?}: {e}"));
            let sym: Symbol<ParseNumberFn> = lib
                .get(b"parse_number\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `parse_number`: {e}"));
            let parse_number = *sym;
            Impl {
                name,
                _lib: lib,
                parse_number,
            }
        }
    }

    /// Raw call: caller fully controls both structs.
    pub unsafe fn parse_number_raw(
        &self,
        item: *mut CJson,
        buf: *mut ParseBuffer,
    ) -> cJSON_bool {
        unsafe { (self.parse_number)(item, buf) }
    }

    /// Call with a byte window plus explicit `length`/`offset`/`depth` and a
    /// pre-populated `item`, returning the full observable outcome.
    pub fn run(
        &self,
        content: Option<&[u8]>,
        length: usize,
        offset: usize,
        depth: usize,
        item_pre: CJson,
    ) -> Outcome {
        let mut item = item_pre;
        let ptr = match content {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        };
        let mut buf = ParseBuffer {
            content: ptr,
            length,
            offset,
            depth,
        };
        let ret = unsafe { (self.parse_number)(&mut item, &mut buf) };
        Outcome {
            ret,
            type_: item.type_,
            valueint: item.valueint,
            valuedouble_bits: item.valuedouble.to_bits(),
            buf_content_is_null: buf.content.is_null(),
            buf_length: buf.length,
            buf_offset: buf.offset,
            buf_depth: buf.depth,
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let root = repo_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
        root.join("c_src/build/Debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:#?}"
    );
}

/// Build the Rust cdylib FRESH and return its path.
///
/// This must NOT reuse `target/<profile>/libdriver.so`: `cargo test` does not
/// rebuild `crate-type = ["cdylib"]` artifacts, so that file can be arbitrarily
/// stale and the whole differential suite would silently validate old code.
/// (Verified: editing `src/lib.rs` and running `cargo test` leaves the `.so`
/// byte-identical.)
///
/// We build into a SEPARATE `--target-dir` so we do not deadlock on the build
/// lock that the enclosing `cargo test` invocation already holds.
fn build_rust_so() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("target/ffi-harness");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut built = false;
    for offline in [true, false] {
        let mut cmd = std::process::Command::new(&cargo);
        cmd.current_dir(manifest)
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&out_dir);
        if offline {
            cmd.arg("--offline");
        }
        // Cargo sets these for the test process; they would confuse the nested
        // invocation into thinking it is a build-script/test context.
        for k in [
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET_DIR",
            "RUSTFLAGS",
        ] {
            cmd.env_remove(k);
        }
        match cmd.output() {
            Ok(o) if o.status.success() => {
                built = true;
                break;
            }
            Ok(o) => {
                if !offline {
                    panic!(
                        "failed to build the Rust cdylib:\n--- stdout ---\n{}\n--- stderr ---\n{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr),
                    );
                }
            }
            Err(e) => {
                if !offline {
                    panic!("could not run `{cargo} build`: {e}");
                }
            }
        }
    }
    assert!(built, "cdylib build did not succeed");

    let so = out_dir.join("release/libdriver.so");
    assert!(
        so.is_file(),
        "expected freshly built cdylib at {}",
        so.display()
    );
    so
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = build_rust_so();
        eprintln!("[harness] C   .so: {}", c_path.display());
        eprintln!("[harness] Rust .so: {} (freshly built)", rust_path.display());
        Pair {
            c: Impl::load("C", &c_path),
            rust: Impl::load("Rust", &rust_path),
        }
    })
}

/// Compare C vs Rust for one full configuration; panics with detail on divergence.
#[track_caller]
pub fn diff(
    label: &str,
    content: Option<&[u8]>,
    length: usize,
    offset: usize,
    depth: usize,
    item_pre: CJson,
) -> Outcome {
    let p = pair();
    let c = p.c.run(content, length, offset, depth, item_pre);
    let r = p.rust.run(content, length, offset, depth, item_pre);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  content: {}\n  length={length} offset={offset} depth={depth}\n\
             \x20 item_pre: type={} valueint={} valuedouble_bits={:#018x}\n\
             \x20 C   : {c:?}\n  Rust: {r:?}",
            match content {
                None => "NULL".to_string(),
                Some(s) => format!("{:?} (len {})", String::from_utf8_lossy(s), s.len()),
            },
            item_pre.type_,
            item_pre.valueint,
            item_pre.valuedouble.to_bits(),
        );
    }
    c
}

/// Convenience: whole slice is the window, offset 0, depth 0, garbage item.
#[track_caller]
pub fn diff_str(label: &str, s: &[u8]) -> Outcome {
    diff(label, Some(s), s.len(), 0, 0, CJson::garbage())
}

/* ----------------------------- PRNG ----------------------------------- */

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

/// xorshift64* — deterministic, no external dependency.
///
/// Uses `Cell` interior mutability so calls can nest
/// (`rng.digits(rng.range(1, 5))`) without tripping the borrow checker.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(std::cell::Cell::new(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        }))
    }
    pub fn next_u64(&self) -> u64 {
        let mut x = self.0.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    /// inclusive range
    pub fn range(&self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn pick<'a, T>(&self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    pub fn bool(&self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn digits(&self, n: usize) -> Vec<u8> {
        (0..n).map(|_| b'0' + self.below(10) as u8).collect()
    }
    pub fn from(&self, alphabet: &[u8], len: usize) -> Vec<u8> {
        (0..len).map(|_| *self.pick(alphabet)).collect()
    }
}

/// The exact character class the C scanner accepts (lib.c:35-55).
pub const NUMERIC_ALPHABET: &[u8] = b"0123456789+-eE.";

pub fn rand_from(rng: &Rng, alphabet: &[u8], len: usize) -> Vec<u8> {
    (0..len).map(|_| *rng.pick(alphabet)).collect()
}

/// How many randomized cases each property-style row runs.
pub const CASES: usize = 2000;
