//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! exposes a differential-testing helper for `ima_parse`.
//!
//! Both implementations are always reached through their exported `ima_parse`
//! symbol -- the Rust crate is never linked directly -- so the `#[no_mangle]`
//! wrapper is part of what gets exercised.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `int ima_parse(struct ima_info *info, const void *data)`
pub type ImaParseFn = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

pub const INFO_SIZE: usize = 40;
pub const OFF_BLOCKS: usize = 0;
pub const OFF_SIZE: usize = 8;
pub const OFF_SAMPLE_RATE: usize = 16;
pub const OFF_FRAME_COUNT: usize = 24;
pub const OFF_CHANNEL_COUNT: usize = 32;

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn find_so_in(dir: &Path, wanted_stem: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        match wanted_stem {
            Some(stem) => {
                if p.file_stem().and_then(|s| s.to_str()) == Some(stem) {
                    return Some(p);
                }
            }
            None => return Some(p),
        }
    }
    None
}

fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_dir = manifest.parent().unwrap().join("c_src").join("build");
    find_so_in(&build_dir, None).unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  cd c_src && mkdir -p build \
             && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // current_exe is <target>/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();

    // `cargo test` does not build (or refresh) the cdylib artifact, because no
    // test target links against it. Build it explicitly, every time -- if this
    // were done only when the file is missing, a stale .so from an earlier
    // source revision would silently be tested instead.
    let target_dir = profile_dir.parent().expect("target dir").to_path_buf();
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Preserve the feature selection the test binary itself was built with.
    if let Ok(features) = std::env::var("IMA_TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !features.is_empty() {
            cmd.arg("--features").arg(features);
        }
    }
    let status = cmd.status().expect("spawn cargo build --lib");
    assert!(status.success(), "cargo build --lib failed");

    find_so_in(&profile_dir, Some("libima_parse_lib")).unwrap_or_else(|| {
        panic!(
            "libima_parse_lib.so not found in {} even after cargo build --lib",
            profile_dir.display()
        )
    })
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            Libs {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("load {}: {e}", rust_path.display())),
            }
        }
    })
}

pub fn c_ima_parse() -> Symbol<'static, ImaParseFn> {
    unsafe { libs().c.get(b"ima_parse\0").expect("C ima_parse") }
}

pub fn rust_ima_parse() -> Symbol<'static, ImaParseFn> {
    unsafe { libs().rust.get(b"ima_parse\0").expect("Rust ima_parse") }
}

// ---------------------------------------------------------------------------
// 8-byte aligned byte buffer (the C reads u64 fields directly out of `data`)
// ---------------------------------------------------------------------------

pub struct AlignedBuf {
    words: Vec<u64>,
    len: usize,
    /// Extra byte offset applied to the pointer handed to `ima_parse`.
    skew: usize,
}

impl AlignedBuf {
    pub fn new(bytes: &[u8]) -> Self {
        Self::with_skew(bytes, 0)
    }

    /// Places `bytes` so that the returned pointer is `skew` bytes past an
    /// 8-byte boundary. Used to confirm both sides agree on unaligned input.
    pub fn with_skew(bytes: &[u8], skew: usize) -> Self {
        let total = bytes.len() + skew;
        let mut words = vec![0u64; total / 8 + 2];
        unsafe {
            let base = words.as_mut_ptr() as *mut u8;
            std::ptr::write_bytes(base, 0, (words.len()) * 8);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(skew), bytes.len());
        }
        AlignedBuf {
            words,
            len: bytes.len(),
            skew,
        }
    }

    pub fn ptr(&self) -> *const u8 {
        unsafe { (self.words.as_ptr() as *const u8).add(self.skew) }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

// ---------------------------------------------------------------------------
// info buffer: raw 40 bytes so padding is compared too
// ---------------------------------------------------------------------------

#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct InfoBuf(pub [u8; INFO_SIZE]);

impl InfoBuf {
    pub fn poisoned() -> Self {
        InfoBuf([0xAA; INFO_SIZE])
    }
    pub fn u64_at(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.0[off..off + 8].try_into().unwrap())
    }
    pub fn u32_at(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    pub fn blocks(&self) -> u64 {
        self.u64_at(OFF_BLOCKS)
    }
    pub fn size(&self) -> u64 {
        self.u64_at(OFF_SIZE)
    }
    pub fn sample_rate_bits(&self) -> u64 {
        self.u64_at(OFF_SAMPLE_RATE)
    }
    pub fn frame_count(&self) -> u64 {
        self.u64_at(OFF_FRAME_COUNT)
    }
    pub fn channel_count(&self) -> u32 {
        self.u32_at(OFF_CHANNEL_COUNT)
    }
}

pub struct Outcome {
    pub ret: i32,
    pub info: InfoBuf,
}

fn call(f: &ImaParseFn, data: &AlignedBuf) -> Outcome {
    let mut info = InfoBuf::poisoned();
    let ret = unsafe { f(info.0.as_mut_ptr(), data.ptr()) };
    Outcome { ret, info }
}

/// Runs both implementations on the same buffer and asserts byte-identical
/// results (return value plus all 40 bytes of `struct ima_info`, padding
/// included).
#[track_caller]
pub fn assert_same(label: &str, bytes: &[u8]) -> Outcome {
    assert_same_skew(label, bytes, 0)
}

#[track_caller]
pub fn assert_same_skew(label: &str, bytes: &[u8], skew: usize) -> Outcome {
    let cf = c_ima_parse();
    let rf = rust_ima_parse();

    // Separate buffers at the same relative alignment: the `blocks` field is a
    // pointer into `data`, so compare it as an offset from the base pointer.
    let cbuf = AlignedBuf::with_skew(bytes, skew);
    let rbuf = AlignedBuf::with_skew(bytes, skew);

    let cout = call(&cf, &cbuf);
    let rout = call(&rf, &rbuf);

    assert_eq!(
        cout.ret, rout.ret,
        "[{label}] return value mismatch: C={} Rust={}",
        cout.ret, rout.ret
    );

    // Normalise the pointer field into an offset relative to each base.
    let cbase = cbuf.ptr() as u64;
    let rbase = rbuf.ptr() as u64;
    let mut c_norm = cout.info;
    let mut r_norm = rout.info;
    if cout.ret == 0 {
        let c_off = cout.info.blocks().wrapping_sub(cbase);
        let r_off = rout.info.blocks().wrapping_sub(rbase);
        assert_eq!(
            c_off, r_off,
            "[{label}] blocks offset mismatch: C=+{c_off} Rust=+{r_off}"
        );
        c_norm.0[OFF_BLOCKS..OFF_BLOCKS + 8].copy_from_slice(&c_off.to_le_bytes());
        r_norm.0[OFF_BLOCKS..OFF_BLOCKS + 8].copy_from_slice(&r_off.to_le_bytes());
    }

    if c_norm.0 != r_norm.0 {
        panic!(
            "[{label}] struct ima_info mismatch\n  \
             C   : blocks=+{:#x} size={:#x} sample_rate_bits={:#018x} ({:?}) \
             frame_count={:#x} channel_count={:#x}\n  \
             Rust: blocks=+{:#x} size={:#x} sample_rate_bits={:#018x} ({:?}) \
             frame_count={:#x} channel_count={:#x}\n  \
             C   bytes: {:02x?}\n  Rust bytes: {:02x?}",
            c_norm.blocks(),
            c_norm.size(),
            c_norm.sample_rate_bits(),
            f64::from_bits(c_norm.sample_rate_bits()),
            c_norm.frame_count(),
            c_norm.channel_count(),
            r_norm.blocks(),
            r_norm.size(),
            r_norm.sample_rate_bits(),
            f64::from_bits(r_norm.sample_rate_bits()),
            r_norm.frame_count(),
            r_norm.channel_count(),
            c_norm.0,
            r_norm.0,
        );
    }

    cout
}

// ---------------------------------------------------------------------------
// CAF-ish buffer builder
//
// Mirrors the layout the C actually reads (not the real on-disk CAF layout):
//   struct caf_header { u32 type; u16 version; u16 flags; }         -- 8 bytes
//   struct caf_chunk  { u32 type; <4 pad>; s64 size; }              -- 16 bytes
// Chunk walking is `chunk = (u8*)&chunk[1] + size`, i.e. +16 + size.
// ---------------------------------------------------------------------------

pub const FOURCC_CAFF: &[u8; 4] = b"caff";
pub const FOURCC_DESC: &[u8; 4] = b"desc";
pub const FOURCC_PAKT: &[u8; 4] = b"pakt";
pub const FOURCC_DATA: &[u8; 4] = b"data";
pub const FOURCC_IMA4: &[u8; 4] = b"ima4";

/// Byte filled into the 4 bytes of `caf_chunk` padding, which the C never
/// reads. Non-zero on purpose.
const CHUNK_PAD: u8 = 0x5A;

#[derive(Clone, Default)]
pub struct Caf {
    pub buf: Vec<u8>,
}

impl Caf {
    pub fn new() -> Self {
        Caf { buf: Vec::new() }
    }

    pub fn header(mut self, file_type: &[u8; 4], version: u16, flags: u16) -> Self {
        self.buf.extend_from_slice(file_type);
        self.buf.extend_from_slice(&version.to_be_bytes());
        self.buf.extend_from_slice(&flags.to_be_bytes());
        self
    }

    pub fn valid_header(self) -> Self {
        self.header(FOURCC_CAFF, 1, 0)
    }

    /// Emits a 16-byte chunk header followed by `body`, with `size` written to
    /// the 64-bit size field (defaults to `body.len()` via `chunk`).
    pub fn chunk_raw(mut self, ty: &[u8; 4], size: i64, body: &[u8]) -> Self {
        self.buf.extend_from_slice(ty);
        self.buf.extend_from_slice(&[CHUNK_PAD; 4]);
        self.buf.extend_from_slice(&size.to_be_bytes());
        self.buf.extend_from_slice(body);
        self
    }

    pub fn chunk(self, ty: &[u8; 4], body: &[u8]) -> Self {
        let n = body.len() as i64;
        self.chunk_raw(ty, n, body)
    }

    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    pub fn build(self) -> Vec<u8> {
        self.buf
    }
}

/// `struct caf_audio_description` body (32 bytes). `sample_rate_bytes` are
/// stored verbatim so pathological bit patterns can be exercised.
pub fn desc_body(
    sample_rate_bytes: [u8; 8],
    format_id: &[u8; 4],
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
) -> Vec<u8> {
    let mut v = Vec::with_capacity(32);
    v.extend_from_slice(&sample_rate_bytes);
    v.extend_from_slice(format_id);
    v.extend_from_slice(&format_flags.to_be_bytes());
    v.extend_from_slice(&bytes_per_packet.to_be_bytes());
    v.extend_from_slice(&frames_per_packet.to_be_bytes());
    v.extend_from_slice(&channels_per_frame.to_be_bytes());
    v.extend_from_slice(&bits_per_channel.to_be_bytes());
    debug_assert_eq!(v.len(), 32);
    v
}

/// Convenience: a `desc` body whose sample rate is the big-endian encoding of
/// `rate` (i.e. what a real CAF file contains).
pub fn desc_body_rate(rate: f64, channels: u32) -> Vec<u8> {
    desc_body(
        rate.to_bits().to_be_bytes(),
        FOURCC_IMA4,
        0,
        34,
        64,
        channels,
        16,
    )
}

/// `struct caf_packet_table` body (24 bytes).
pub fn pakt_body(
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
) -> Vec<u8> {
    let mut v = Vec::with_capacity(24);
    v.extend_from_slice(&packet_count.to_be_bytes());
    v.extend_from_slice(&frame_count.to_be_bytes());
    v.extend_from_slice(&priming_frames.to_be_bytes());
    v.extend_from_slice(&remainder_frames.to_be_bytes());
    debug_assert_eq!(v.len(), 24);
    v
}

/// `struct caf_data` body: u32 edit_count followed by `payload`.
pub fn data_body(edit_count: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + payload.len());
    v.extend_from_slice(&edit_count.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

// ---------------------------------------------------------------------------
// deterministic pseudo-random source
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u64() as u8).collect()
    }
}
