//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `bin2hex` only through the dynamic-symbol boundary.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Bin2Hex = unsafe extern "C" fn(*mut i8, usize, *const u8, usize) -> *mut i8;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    let mut stack = vec![build.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    assert!(
        !found.is_empty(),
        "no .so found under {}. Build the C library first:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.sort();
    found.remove(0)
}

fn find_rust_so() -> PathBuf {
    // Allow the driver script to point the tests at a specific profile's
    // cdylib (used to verify the debug build as well as the release one).
    if let Ok(p) = std::env::var("BIN2HEX_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "BIN2HEX_RUST_SO={} does not exist", p.display());
        return p;
    }
    // Prefer the release cdylib (the artifact an external consumer links), fall
    // back to debug.
    let t = repo_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = t.join(profile).join("libbin2hex_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libbin2hex_lib.so not found under {}. Build it first: cd translation && cargo build --release",
        t.display()
    );
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn c_fn(&self) -> Symbol<'_, Bin2Hex> {
        unsafe { self.c.get(b"bin2hex\0") }.expect("C .so must export bin2hex")
    }
    pub fn rust_fn(&self) -> Symbol<'_, Bin2Hex> {
        unsafe { self.rust.get(b"bin2hex\0") }.expect("Rust .so must export bin2hex")
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(find_c_so()).expect("load C .so");
        let rust = Library::new(find_rust_so()).expect("load Rust .so");
        Libs { c, rust }
    })
}

/// Deterministic xorshift64* PRNG so every row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

pub const GUARD: u8 = 0xAA;

/// The outcome of one `bin2hex` call, fully observable by an external caller.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    /// Entire output allocation after the call, guard bytes included.
    pub buf: Vec<u8>,
    /// Offset of the returned pointer within the output allocation
    /// (`None` if the callee returned NULL or something unrelated).
    pub ret_offset: Option<isize>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Outcome {{ ret_offset: {:?}, buf: ", self.ret_offset)?;
        // Show it readably: hex bytes, truncated.
        let n = self.buf.len().min(96);
        for b in &self.buf[..n] {
            write!(f, "{:02x}", b)?;
        }
        if self.buf.len() > n {
            write!(f, "... ({} bytes)", self.buf.len())?;
        }
        write!(f, " }}")
    }
}

/// Runs one implementation on a freshly guard-filled output buffer.
///
/// * `out_alloc` — total size of the output allocation (must be >= `hex_off + ...`).
/// * `hex_off`   — offset within the allocation at which `hex` points (alignment axis).
/// * `bin_off`   — offset within the input allocation at which `bin` points.
pub fn run_one(
    f: &Bin2Hex,
    out_alloc: usize,
    hex_off: usize,
    hex_maxlen: usize,
    bin: &[u8],
    bin_off: usize,
) -> Outcome {
    let mut out = vec![GUARD; out_alloc];
    let mut input = vec![0u8; bin_off + bin.len()];
    input[bin_off..].copy_from_slice(bin);

    let hex_ptr = unsafe { out.as_mut_ptr().add(hex_off) } as *mut i8;
    let bin_ptr = unsafe { input.as_ptr().add(bin_off) };

    let ret = unsafe { f(hex_ptr, hex_maxlen, bin_ptr, bin.len()) };

    let base = out.as_mut_ptr() as isize;
    let ret_offset = if ret.is_null() {
        None
    } else {
        Some(ret as isize - base)
    };

    Outcome {
        buf: out,
        ret_offset,
    }
}

/// The core differential assertion: identical outcome from C and Rust.
pub fn assert_same(
    label: &str,
    out_alloc: usize,
    hex_off: usize,
    hex_maxlen: usize,
    bin: &[u8],
    bin_off: usize,
) {
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    let c = run_one(&cf, out_alloc, hex_off, hex_maxlen, bin, bin_off);
    let r = run_one(&rf, out_alloc, hex_off, hex_maxlen, bin, bin_off);

    if c != r {
        // Pinpoint the first differing byte for a useful message.
        let first = c
            .buf
            .iter()
            .zip(r.buf.iter())
            .position(|(a, b)| a != b)
            .map(|i| format!("first differing output byte at index {i}: C={:#04x} Rust={:#04x}", c.buf[i], r.buf[i]))
            .unwrap_or_else(|| "output buffers equal; return pointers differ".to_string());
        panic!(
            "DIVERGENCE [{label}]\n  out_alloc={out_alloc} hex_off={hex_off} \
             hex_maxlen={hex_maxlen} bin_len={} bin_off={bin_off}\n  bin={}\n  {first}\n  C   = {:?}\n  Rust= {:?}",
            bin.len(),
            hex_prefix(bin),
            c,
            r
        );
    }

    // Sanity: the return pointer must be `hex` itself (C: `return hex;`).
    assert_eq!(
        c.ret_offset,
        Some(hex_off as isize),
        "[{label}] C should return the hex argument"
    );
}

fn hex_prefix(b: &[u8]) -> String {
    let n = b.len().min(64);
    let mut s: String = b[..n].iter().map(|x| format!("{:02x}", x)).collect();
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Convenience wrapper for the common case: `hex` at offset 0 of an allocation
/// sized `bin_len*2 + 1 + slack`, guard bytes proving nothing extra is written.
pub fn assert_same_simple(label: &str, bin: &[u8], hex_maxlen: usize, slack: usize) {
    let need = bin.len() * 2 + 1;
    let out_alloc = need + slack;
    assert_same(label, out_alloc, 0, hex_maxlen, bin, 0);
}
