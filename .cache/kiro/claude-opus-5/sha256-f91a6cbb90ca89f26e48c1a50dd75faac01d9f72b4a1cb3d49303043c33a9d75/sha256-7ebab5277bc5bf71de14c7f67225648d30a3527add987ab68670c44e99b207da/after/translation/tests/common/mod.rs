//! Shared harness: loads the C shared library and the Rust cdylib through
//! `libloading` and drives `dequantize_granule` in both, comparing raw bytes.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Mirrors the C `bs_t` (pointer + two ints => 16 bytes on LP64).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

pub type DequantizeGranuleFn =
    unsafe extern "C" fn(*mut f32, *mut BsT, *mut u8, c_int) -> c_int;

// ---------------------------------------------------------------------------
// L12_scale_info layout (recomputed here so the C and Rust sides observe the
// *identical* byte image, including the trailing padding and the bytes that
// follow the struct -- the C code intentionally indexes `bitalloc` past its
// declared 64 elements when `total_bands > 32`).
// ---------------------------------------------------------------------------

pub const SCI_OFF_SCF: usize = 0;
pub const SCI_LEN_SCF: usize = 3 * 64 * 4;
pub const SCI_OFF_TOTAL_BANDS: usize = SCI_OFF_SCF + SCI_LEN_SCF; // 768
pub const SCI_OFF_STEREO_BANDS: usize = SCI_OFF_TOTAL_BANDS + 1; // 769
pub const SCI_OFF_BITALLOC: usize = SCI_OFF_STEREO_BANDS + 1; // 770
pub const SCI_OFF_SCFCOD: usize = SCI_OFF_BITALLOC + 64; // 834
pub const SCI_SIZE: usize = 900; // sizeof(L12_scale_info) with 4-byte align

/// Total backing store handed to the callee. Generous so that the intentional
/// out-of-bounds `bitalloc[i]` reads (up to i == 509) stay inside a real,
/// deterministically initialised allocation.
pub const SCI_BACKING_BYTES: usize = 8192;

/// An 8-byte aligned, deterministically filled `L12_scale_info` image.
#[derive(Clone)]
pub struct SciBuf {
    words: Vec<u64>,
}

impl SciBuf {
    pub fn new(seed: u64) -> Self {
        let mut words = vec![0u64; SCI_BACKING_BYTES / 8];
        // Deterministic filler so both libraries read identical bytes even far
        // past the end of the struct.
        let mut s = seed | 1;
        for w in words.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *w = s;
        }
        let mut me = Self { words };
        // Keep `scf` finite-ish; it is unused by dequantize_granule but avoid
        // surprising signalling patterns in case anything ever reads it.
        for i in 0..(3 * 64) {
            let v = ((i as f32) * 0.5 - 16.0).to_bits();
            me.write_u32(SCI_OFF_SCF + i * 4, v);
        }
        me
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        let len = self.words.len() * 8;
        unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, len) }
    }

    pub fn bytes(&self) -> &[u8] {
        let len = self.words.len() * 8;
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, len) }
    }

    fn write_u32(&mut self, off: usize, v: u32) {
        self.bytes_mut()[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn set_u8(&mut self, off: usize, v: u8) {
        self.bytes_mut()[off] = v;
    }

    pub fn set_total_bands(&mut self, v: u8) {
        self.set_u8(SCI_OFF_TOTAL_BANDS, v);
    }

    pub fn set_stereo_bands(&mut self, v: u8) {
        self.set_u8(SCI_OFF_STEREO_BANDS, v);
    }

    /// Writes `vals` into the `bitalloc` region (index may exceed 64; the C
    /// code reads there too).
    pub fn set_bitalloc(&mut self, idx: usize, v: u8) {
        self.set_u8(SCI_OFF_BITALLOC + idx, v);
    }

    /// Zero every byte from `bitalloc[0]` up to the end of the backing store so
    /// that only explicitly configured band allocations are non-zero.
    pub fn clear_bitalloc_tail(&mut self) {
        let start = SCI_OFF_BITALLOC;
        let b = self.bytes_mut();
        for x in &mut b[start..] {
            *x = 0;
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
}

// ---------------------------------------------------------------------------
// grbuf: a large f32 arena. The caller-visible base sits in the middle so that
// negative `group_size` values (which make the C code compute, but never
// dereference, a below-base pointer) remain inside the allocation.
// ---------------------------------------------------------------------------

pub const GRBUF_WORDS: usize = 1 << 17;
pub const GRBUF_BASE: usize = 1 << 16;

#[derive(Clone)]
pub struct GrBuf {
    data: Vec<f32>,
}

impl GrBuf {
    pub fn new() -> Self {
        // Fill with a recognisable sentinel so untouched slots are comparable.
        let mut data = vec![0.0f32; GRBUF_WORDS];
        for (i, v) in data.iter_mut().enumerate() {
            *v = f32::from_bits(0xDEAD_0000u32 ^ (i as u32));
        }
        Self { data }
    }

    pub fn base_ptr(&mut self) -> *mut f32 {
        unsafe { self.data.as_mut_ptr().add(GRBUF_BASE) }
    }

    pub fn bits(&self) -> Vec<u32> {
        self.data.iter().map(|v| v.to_bits()).collect()
    }
}

// ---------------------------------------------------------------------------
// Bitstream backing store
// ---------------------------------------------------------------------------

pub const BS_BYTES: usize = 1 << 20;
/// Leave slack so that a read ending exactly at `limit` cannot touch memory
/// past the allocation.
pub const BS_LIMIT_BITS: c_int = ((BS_BYTES - 16) * 8) as c_int;

pub fn make_bitstream(seed: u64) -> Vec<u8> {
    let mut out = vec![0u8; BS_BYTES];
    let mut s = seed | 1;
    for chunk in out.chunks_mut(8) {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = s.to_le_bytes();
        chunk.copy_from_slice(&b[..chunk.len()]);
    }
    out
}

// ---------------------------------------------------------------------------
// Library discovery / loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let e = e.ok()?;
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if !name.starts_with("lib") || !name.ends_with(".so") {
            continue;
        }
        if let Some(frag) = must_contain {
            if !name.contains(frag) {
                continue;
            }
        }
        best = Some(p);
    }
    best
}

pub fn c_library_path() -> PathBuf {
    // Escape hatch used to re-run the whole suite against a differently
    // optimised C build without touching `c_src/`.
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    find_so(&dir, None).unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

pub fn rust_library_path() -> PathBuf {
    // current_exe = <target>/<profile>/deps/<test-bin>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");

    if let Some(p) = locate_rust_so(profile, deps) {
        return p;
    }

    // `cargo test` builds the test harness but not the `cdylib` artifact, so
    // produce it on demand (once per process). The cargo build lock is free at
    // this point because compilation of the test binary already finished.
    static BUILD_ONCE: std::sync::Once = std::sync::Once::new();
    BUILD_ONCE.call_once(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build")
            .arg("--lib")
            .current_dir(manifest_dir())
            // Avoid inheriting the test harness's rustc wrapper/flags oddities.
            .env_remove("RUSTC_WORKSPACE_WRAPPER");
        if profile.file_name().map(|n| n == "release").unwrap_or(false) {
            cmd.arg("--release");
        }
        let out = cmd.output().expect("spawn cargo build --lib");
        if !out.status.success() {
            panic!(
                "cargo build --lib failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    });

    locate_rust_so(profile, deps).unwrap_or_else(|| {
        panic!(
            "Rust cdylib libdequantize_granule_lib.so not found under {} even after \
             `cargo build --lib`",
            profile.display()
        )
    })
}

fn locate_rust_so(profile: &Path, deps: &Path) -> Option<PathBuf> {
    for dir in [profile, deps] {
        if let Some(p) = find_so(dir, Some("dequantize_granule_lib")) {
            return Some(p);
        }
    }
    None
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: DequantizeGranuleFn,
    pub rust: DequantizeGranuleFn,
}

pub fn load() -> Impls {
    unsafe {
        let c_lib = Library::new(c_library_path()).expect("load C .so");
        let rust_lib = Library::new(rust_library_path()).expect("load Rust .so");
        let c: Symbol<DequantizeGranuleFn> =
            c_lib.get(b"dequantize_granule\0").expect("C dequantize_granule");
        let rust: Symbol<DequantizeGranuleFn> = rust_lib
            .get(b"dequantize_granule\0")
            .expect("Rust dequantize_granule");
        let c = *c;
        let rust = *rust;
        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

// ---------------------------------------------------------------------------
// One comparison run
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    pub pos: c_int,
    pub limit: c_int,
    pub grbuf: Vec<u32>,
    pub sci: Vec<u8>,
}

pub struct Case {
    pub label: String,
    pub sci: SciBuf,
    pub group_size: c_int,
    pub start_pos: c_int,
    pub limit: c_int,
    pub bs_seed: u64,
}

impl Case {
    pub fn new(label: impl Into<String>, group_size: c_int) -> Self {
        let mut sci = SciBuf::new(0x1234_5678_9abc_def0);
        sci.clear_bitalloc_tail();
        sci.set_total_bands(0);
        sci.set_stereo_bands(0);
        Self {
            label: label.into(),
            sci,
            group_size,
            start_pos: 0,
            limit: BS_LIMIT_BITS,
            bs_seed: 0xA5A5_5A5A_DEAD_BEEF,
        }
    }
}

fn run_one(f: DequantizeGranuleFn, case: &Case) -> Outcome {    let mut gr = GrBuf::new();
    let mut sci = case.sci.clone();
    let mut bits = make_bitstream(case.bs_seed);
    let mut bs = BsT {
        buf: bits.as_ptr(),
        pos: case.start_pos,
        limit: case.limit,
    };
    let ret = unsafe { f(gr.base_ptr(), &mut bs, sci.as_mut_ptr(), case.group_size) };
    // Keep the bitstream alive until after the call.
    let _ = bits.as_mut_ptr();
    Outcome {
        ret,
        pos: bs.pos,
        limit: bs.limit,
        grbuf: gr.bits(),
        sci: sci.bytes().to_vec(),
    }
}

/// Runs a single implementation without comparing; used to attribute crashes.
pub fn run_only(f: DequantizeGranuleFn, case: &Case) -> Outcome {
    run_one(f, case)
}

/// Runs `case` through both implementations and asserts byte-identical results.
pub fn compare(impls: &Impls, case: &Case) {
    let c = run_one(impls.c, case);
    let r = run_one(impls.rust, case);

    assert_eq!(c.ret, r.ret, "[{}] return value mismatch", case.label);
    assert_eq!(c.pos, r.pos, "[{}] bs->pos mismatch", case.label);
    assert_eq!(c.limit, r.limit, "[{}] bs->limit mismatch", case.label);
    assert_eq!(
        c.sci, r.sci,
        "[{}] L12_scale_info was mutated differently",
        case.label
    );

    if c.grbuf != r.grbuf {
        let mut diffs = Vec::new();
        for i in 0..c.grbuf.len() {
            if c.grbuf[i] != r.grbuf[i] {
                diffs.push((i as isize - GRBUF_BASE as isize, c.grbuf[i], r.grbuf[i]));
                if diffs.len() == 12 {
                    break;
                }
            }
        }
        panic!(
            "[{}] grbuf mismatch ({} differing slots). First diffs (offset, C bits, Rust bits): {:?}",
            case.label,
            c.grbuf.iter().zip(r.grbuf.iter()).filter(|(a, b)| a != b).count(),
            diffs
        );
    }
}
