//! Shared differential-test support.
//!
//! Both the C library and the Rust library are loaded as shared objects with
//! `libloading` and driven exclusively through their exported `ima_parse`
//! symbol.  The Rust crate is *never* linked directly, so the `#[no_mangle]`
//! `extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// The ABI under test
// ---------------------------------------------------------------------------

/// `int ima_parse(struct ima_info *info, const void *data);`
pub type ImaParseFn = unsafe extern "C" fn(*mut c_void, *const c_void) -> i32;

/// `struct ima_info` is 40 bytes / 8-aligned (see SYMBOLS.md).  We hand the
/// library a raw byte block so that *every* byte, padding included, can be
/// compared between the two implementations.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InfoBytes(pub [u8; 40]);

/// Sentinel the out-param is pre-filled with, so that "did not write" is
/// distinguishable from "wrote zero".
pub const SENTINEL: u8 = 0xAA;

impl InfoBytes {
    pub fn sentinel() -> Self {
        InfoBytes([SENTINEL; 40])
    }
    pub fn blocks(&self) -> u64 {
        u64::from_le_bytes(self.0[0..8].try_into().unwrap())
    }
    pub fn size(&self) -> u64 {
        u64::from_le_bytes(self.0[8..16].try_into().unwrap())
    }
    pub fn sample_rate_bits(&self) -> u64 {
        u64::from_le_bytes(self.0[16..24].try_into().unwrap())
    }
    pub fn frame_count(&self) -> u64 {
        u64::from_le_bytes(self.0[24..32].try_into().unwrap())
    }
    pub fn channel_count(&self) -> u32 {
        u32::from_le_bytes(self.0[32..36].try_into().unwrap())
    }
    pub fn tail_padding(&self) -> u32 {
        u32::from_le_bytes(self.0[36..40].try_into().unwrap())
    }
    pub fn describe(&self) -> String {
        format!(
            "blocks=0x{:016x} size=0x{:016x} sample_rate_bits=0x{:016x} \
             frame_count=0x{:016x} channel_count=0x{:08x} pad=0x{:08x}",
            self.blocks(),
            self.size(),
            self.sample_rate_bits(),
            self.frame_count(),
            self.channel_count(),
            self.tail_padding()
        )
    }
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: ImaParseFn,
    pub rust: ImaParseFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    // Kept alive for the lifetime of the process; never dropped.
    _keep: Vec<libloading::Library>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the CMake project name is derived
/// from the name of the directory that contains `c_src`, so glob for it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("IMA_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
            {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// `target/<profile>/libima_parse_lib.so`, derived from the location of the
/// running test binary (`target/<profile>/deps/<test>-<hash>`), so that
/// `cargo test` and `cargo test --release` each exercise their own artifact.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("IMA_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps: &Path = exe.parent().expect("deps dir");
    let profile: &Path = deps.parent().expect("profile dir");
    let name = "libima_parse_lib.so";
    for dir in [profile, deps] {
        let cand = dir.join(name);
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "could not find {} next to the test binary ({}).\n\
         `cargo test` does NOT build a `crate-type = [\"cdylib\"]` target, so the \
         .so must be produced first:\n  \
         cargo build            # for `cargo test`\n  \
         cargo build --release  # for `cargo test --release`\n\
         Or just run ./verify.sh, which does everything in the right order.",
        name,
        exe.display()
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let cl = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rl = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c: libloading::Symbol<ImaParseFn> = cl
                .get(b"ima_parse\0")
                .unwrap_or_else(|e| panic!("dlsym ima_parse in C .so: {e}"));
            let rust: libloading::Symbol<ImaParseFn> = rl
                .get(b"ima_parse\0")
                .unwrap_or_else(|e| panic!("dlsym ima_parse in Rust .so: {e}"));
            let c = *c;
            let rust = *rust;
            Libs {
                c,
                rust,
                c_path,
                rust_path,
                _keep: vec![cl, rl],
            }
        }
    })
}

// ---------------------------------------------------------------------------
// The differential call
// ---------------------------------------------------------------------------

pub struct Outcome {
    pub c_ret: i32,
    pub c_info: InfoBytes,
    pub r_ret: i32,
    pub r_info: InfoBytes,
}

impl Outcome {
    pub fn matches(&self) -> bool {
        self.c_ret == self.r_ret && self.c_info == self.r_info
    }
}

/// Calls both implementations with the **same** `data` pointer (so the
/// `info->blocks` output pointer is directly comparable) and a freshly
/// sentinel-filled `ima_info`.
pub fn call_both(data: *const c_void) -> Outcome {
    let l = libs();
    let mut c_info = InfoBytes::sentinel();
    let mut r_info = InfoBytes::sentinel();
    let c_ret = unsafe { (l.c)(&mut c_info as *mut InfoBytes as *mut c_void, data) };
    let r_ret = unsafe { (l.rust)(&mut r_info as *mut InfoBytes as *mut c_void, data) };
    Outcome {
        c_ret,
        c_info,
        r_ret,
        r_info,
    }
}

fn hexdump(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, ch) in b.chunks(16).enumerate() {
        s.push_str(&format!("  {:04x}: ", i * 16));
        for x in ch {
            s.push_str(&format!("{x:02x} "));
        }
        s.push('\n');
    }
    s
}

/// Runs the differential call and panics with a full diagnostic on divergence.
pub fn assert_same(ctx: &str, buf: &[u8], data: *const c_void) -> Outcome {
    let o = call_both(data);
    if !o.matches() {
        panic!(
            "DIVERGENCE [{ctx}]\n\
             C   : ret={} {}\n\
             Rust: ret={} {}\n\
             data ptr = {:p}\n\
             input buffer ({} bytes):\n{}",
            o.c_ret,
            o.c_info.describe(),
            o.r_ret,
            o.r_info.describe(),
            data,
            buf.len(),
            hexdump(buf)
        );
    }
    o
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.u64() as u32
    }
    pub fn u16(&mut self) -> u16 {
        self.u64() as u16
    }
    pub fn u8(&mut self) -> u8 {
        self.u64() as u8
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.u64() % n
    }
    pub fn range_usize(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below((hi_inclusive - lo + 1) as u64) as usize
    }
    pub fn fourcc(&mut self) -> [u8; 4] {
        self.u32().to_le_bytes()
    }
    /// A random `chunk->type` that is not one of the three the C recognises.
    pub fn unknown_fourcc(&mut self) -> [u8; 4] {
        loop {
            let t = self.fourcc();
            if t != *b"desc" && t != *b"pakt" && t != *b"data" {
                return t;
            }
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

// ---------------------------------------------------------------------------
// 8-aligned (or deliberately misaligned) backing store for the input buffer
// ---------------------------------------------------------------------------

/// Copies `bytes` into an 8-aligned allocation at byte offset `off`, plus a
/// tail of slack so that the C library's (unbounded) reads of the structures we
/// laid out stay inside the mapping.
pub struct AlignedBuf {
    store: Vec<u64>,
    off: usize,
    len: usize,
}

impl AlignedBuf {
    pub const SLACK: usize = 256;

    pub fn new(bytes: &[u8], off: usize) -> Self {
        let total = off + bytes.len() + Self::SLACK;
        let mut store = vec![0u64; total / 8 + 2];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (store.as_mut_ptr() as *mut u8).add(off),
                bytes.len(),
            );
        }
        AlignedBuf {
            store,
            off,
            len: bytes.len(),
        }
    }

    pub fn aligned(bytes: &[u8]) -> Self {
        Self::new(bytes, 0)
    }

    pub fn ptr(&self) -> *const c_void {
        unsafe { (self.store.as_ptr() as *const u8).add(self.off) as *const c_void }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

// ---------------------------------------------------------------------------
// CAF file builder
//
// Field endianness, derived from the C source:
//   * every field read through `ima_btoh16/32/64` must be stored BIG-endian
//     (`btoh` is an unconditional byte swap of a native little-endian load):
//     header->type, header->version, chunk->type, chunk->size,
//     desc->format_id, desc->channels_per_frame, pakt->frame_count
//   * `desc->sample_rate` is read as a *native* `double` (no byte swap before
//     the value conversion), so it is stored little-endian.
//   * `struct caf_chunk` is 16 bytes (4 bytes of padding between `type` and
//     `size`), so the chunk stride is `16 + size`, not the 12 real CAF uses.
// ---------------------------------------------------------------------------

pub const MAGIC_CAFF: [u8; 4] = *b"caff";
pub const FMT_IMA4: [u8; 4] = *b"ima4";
pub const T_DESC: [u8; 4] = *b"desc";
pub const T_PAKT: [u8; 4] = *b"pakt";
pub const T_DATA: [u8; 4] = *b"data";

pub const CHUNK_HDR: usize = 16;
pub const DESC_LEN: usize = 32;
pub const PAKT_LEN: usize = 24;
pub const CAF_DATA_LEN: usize = 4;

pub struct FileBuilder {
    pub bytes: Vec<u8>,
    rng: Rng,
    /// Byte offset of the `data` chunk header that the scan will break on.
    pub data_off: Option<usize>,
    pub desc_off: Option<usize>,
    pub pakt_off: Option<usize>,
}

impl FileBuilder {
    pub fn new(rng: Rng, magic: [u8; 4], version: u16) -> Self {
        let mut b = FileBuilder {
            bytes: Vec::new(),
            rng,
            data_off: None,
            desc_off: None,
            pakt_off: None,
        };
        b.bytes.extend_from_slice(&magic);
        b.bytes.extend_from_slice(&version.to_be_bytes());
        // header->flags is never read by the C code: fill with noise.
        let flags = b.rng.u16();
        b.bytes.extend_from_slice(&flags.to_be_bytes());
        b
    }

    pub fn valid_header(rng: Rng) -> Self {
        Self::new(rng, MAGIC_CAFF, 1)
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    pub fn offset(&self) -> usize {
        self.bytes.len()
    }

    fn hdr(&mut self, ctype: [u8; 4], size: i64) -> usize {
        let at = self.bytes.len();
        self.bytes.extend_from_slice(&ctype);
        // struct caf_chunk padding (offsets 4..8) is never read: noise.
        let pad = self.rng.u32();
        self.bytes.extend_from_slice(&pad.to_le_bytes());
        self.bytes.extend_from_slice(&(size as u64).to_be_bytes());
        at
    }

    /// A `desc` chunk whose declared size equals its 32-byte payload, so the
    /// scan lands exactly on the following chunk.
    pub fn desc(&mut self, sample_rate_bits: u64, format_id: [u8; 4], channels: u32) -> &mut Self {
        self.desc_sized(sample_rate_bits, format_id, channels, DESC_LEN as i64, None)
    }

    /// A `desc` chunk with an arbitrary declared size (and optional override of
    /// the `bytes_per_packet`/`frames_per_packet` pair, which is what an
    /// overlapping next-chunk-header lands on).
    pub fn desc_sized(
        &mut self,
        sample_rate_bits: u64,
        format_id: [u8; 4],
        channels: u32,
        declared: i64,
        bpp_fpp_be: Option<u64>,
    ) -> &mut Self {
        let at = self.hdr(T_DESC, declared);
        self.desc_off = Some(at);
        let mut p = [0u8; DESC_LEN];
        p[0..8].copy_from_slice(&sample_rate_bits.to_le_bytes()); // native double
        p[8..12].copy_from_slice(&format_id); // btoh32
        let noise = self.rng.u32();
        p[12..16].copy_from_slice(&noise.to_le_bytes()); // format_flags: unread
        match bpp_fpp_be {
            Some(v) => p[16..24].copy_from_slice(&v.to_be_bytes()),
            None => {
                let a = self.rng.u32();
                let b = self.rng.u32();
                p[16..20].copy_from_slice(&a.to_le_bytes()); // bytes_per_packet: unread
                p[20..24].copy_from_slice(&b.to_le_bytes()); // frames_per_packet: unread
            }
        }
        p[24..28].copy_from_slice(&channels.to_be_bytes()); // btoh32
        let noise = self.rng.u32();
        p[28..32].copy_from_slice(&noise.to_le_bytes()); // bits_per_channel: unread
        self.bytes.extend_from_slice(&p);
        self
    }

    /// A `pakt` chunk whose declared size equals its 24-byte payload.
    pub fn pakt(&mut self, frame_count: u64) -> &mut Self {
        let at = self.hdr(T_PAKT, PAKT_LEN as i64);
        self.pakt_off = Some(at);
        let mut p = [0u8; PAKT_LEN];
        let n = self.rng.u64();
        p[0..8].copy_from_slice(&n.to_le_bytes()); // packet_count: unread
        p[8..16].copy_from_slice(&frame_count.to_be_bytes()); // btoh64
        let a = self.rng.u32();
        let b = self.rng.u32();
        p[16..20].copy_from_slice(&a.to_le_bytes()); // priming_frames: unread
        p[20..24].copy_from_slice(&b.to_le_bytes()); // remainder_frames: unread
        self
            .bytes
            .extend_from_slice(&p);
        self
    }

    /// The `data` chunk.  `declared` is copied verbatim into `info->size`; the
    /// payload only needs the 4-byte `struct caf_data` plus whatever blocks we
    /// feel like adding, because the scan stops here.
    pub fn data(&mut self, declared: i64, payload_len: usize) -> &mut Self {
        let at = self.hdr(T_DATA, declared);
        if self.data_off.is_none() {
            self.data_off = Some(at);
        }
        let n = payload_len.max(CAF_DATA_LEN);
        let noise = self.rng.bytes(n);
        self.bytes.extend_from_slice(&noise);
        self
    }

    /// A chunk with an unrecognised type; declared size == payload length so
    /// the scan lands exactly on the following chunk.
    pub fn unknown(&mut self, ctype: [u8; 4], payload_len: usize) -> &mut Self {
        self.hdr(ctype, payload_len as i64);
        let noise = self.rng.bytes(payload_len);
        self.bytes.extend_from_slice(&noise);
        self
    }

    /// A chunk with an unrecognised type, an arbitrary declared size, and an
    /// independently chosen payload length (used for backwards / overlapping
    /// jumps).
    pub fn unknown_sized(
        &mut self,
        ctype: [u8; 4],
        declared: i64,
        payload_len: usize,
    ) -> &mut Self {
        self.hdr(ctype, declared);
        let noise = self.rng.bytes(payload_len);
        self.bytes.extend_from_slice(&noise);
        self
    }

    pub fn raw(&mut self, n: usize) -> &mut Self {
        let noise = self.rng.bytes(n);
        self.bytes.extend_from_slice(&noise);
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Curated hard `double` values for the `(ima_u64_t)double` value conversion.
pub const HARD_DOUBLES: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    1.5,
    -1.5,
    0.9999999999999999,
    -0.9999999999999999,
    44100.0,
    8000.0,
    22050.0,
    48000.5,
    -48000.5,
    1e18,
    9.2e18,
    9.3e18,
    1.9e19,
    1.8446744073709552e19, // 2^64
    9223372036854775808.0, // 2^63 exactly
    9223372036854774784.0, // largest double < 2^63
    -9223372036854775808.0, // -2^63 exactly
    -9223372036854777856.0, // just below -2^63
    9223372036854777856.0, // just above 2^63
    1e300,
    -1e300,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
    5e-324, // smallest subnormal
    -5e-324,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
    -f64::NAN,
];

/// Bit patterns for `sample_rate` that are not conveniently written as `f64`
/// literals (signalling NaNs, NaN payloads, ...).
pub const HARD_DOUBLE_BITS: &[u64] = &[
    0x7ff0_0000_0000_0001, // signalling NaN
    0xfff0_0000_0000_0001, // negative signalling NaN
    0x7ff8_0000_0000_0000, // quiet NaN
    0xfff8_0000_0000_0000,
    0x7ff7_ffff_ffff_ffff,
    0x0000_0000_0000_0001, // smallest subnormal
    0x8000_0000_0000_0001,
    0x000f_ffff_ffff_ffff, // largest subnormal
    0x43e0_0000_0000_0000, // 2^63
    0x43df_ffff_ffff_ffff, // just under 2^63
    0x43e0_0000_0000_0001, // just over 2^63
    0xc3e0_0000_0000_0000, // -2^63
    0xffff_ffff_ffff_ffff,
    0x0000_0000_0000_0000,
    0x8000_0000_0000_0000,
];
