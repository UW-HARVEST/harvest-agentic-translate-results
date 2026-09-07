//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! `ima_parse` only through the dynamic symbol, exactly as an external C
//! consumer would. Rust functions are never called directly.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI mirror of `struct ima_info` (include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ImaInfo {
    pub blocks: *const u8,
    pub size: u64,
    pub sample_rate: f64,
    pub frame_count: u64,
    pub channel_count: u32,
}

pub const IMA_INFO_SIZE: usize = 40;

impl ImaInfo {
    /// Pre-fill every byte (including the tail padding) with `fill` so that a
    /// byte-for-byte comparison also proves which bytes the callee wrote.
    pub fn prefilled(fill: u8) -> Self {
        assert_eq!(std::mem::size_of::<ImaInfo>(), IMA_INFO_SIZE);
        let mut raw = [fill; IMA_INFO_SIZE];
        // SAFETY: ImaInfo is repr(C), 40 bytes, all-integer/pointer fields; any
        // bit pattern is a valid value for the purposes of this comparison.
        unsafe { std::ptr::read_unaligned(raw.as_mut_ptr() as *const ImaInfo) }
    }

    pub fn raw(&self) -> [u8; IMA_INFO_SIZE] {
        let mut out = [0u8; IMA_INFO_SIZE];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const ImaInfo as *const u8,
                out.as_mut_ptr(),
                IMA_INFO_SIZE,
            );
        }
        out
    }

    pub fn describe(&self) -> String {
        format!(
            "blocks={:p} size={:#018x} sample_rate_bits={:#018x} frame_count={:#018x} channel_count={:#010x}",
            self.blocks,
            self.size,
            self.sample_rate.to_bits(),
            self.frame_count,
            self.channel_count
        )
    }
}

pub type ImaParseFn = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// Locate the C shared library produced by the CMake build. Its name is derived
/// from the working-directory name, so glob for it instead of hard-coding.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("IMA_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

/// Locate the Rust cdylib. Defaults to the release artifact (what an external
/// consumer links against); `IMA_RUST_SO` selects a specific one, and
/// `scripts/check_features.sh` uses that to exercise every profile.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("IMA_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = crate_root().join("target");
    for dir in ["release", "debug"] {
        let p = target.join(dir).join("libima_parse_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {}. Build it with `cargo build --release`.",
        target.display()
    );
}

fn load_parse(path: &Path) -> (libloading::Library, ImaParseFn) {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let sym: libloading::Symbol<ImaParseFn> = lib
            .get(b"ima_parse\0")
            .unwrap_or_else(|e| panic!("dlsym(ima_parse) in {} failed: {e}", path.display()));
        let f: ImaParseFn = *sym;
        (lib, f)
    }
}

/// Both implementations, reached exclusively through their exported symbols.
pub struct Pair {
    _c_lib: libloading::Library,
    _r_lib: libloading::Library,
    pub c: ImaParseFn,
    pub r: ImaParseFn,
}

impl Pair {
    pub fn load() -> Self {
        let (cl, c) = load_parse(&c_so_path());
        let (rl, r) = load_parse(&rust_so_path());
        Pair { _c_lib: cl, _r_lib: rl, c, r }
    }

    /// Call both exports on the same buffer and assert byte-identical results.
    pub fn diff(&self, buf: &AlignedBuf, ctx: &dyn std::fmt::Display) {
        self.diff_fill(buf, 0xAA, ctx);
        self.diff_fill(buf, 0x00, ctx);
    }

    pub fn diff_fill(&self, buf: &AlignedBuf, fill: u8, ctx: &dyn std::fmt::Display) {
        let mut ci = ImaInfo::prefilled(fill);
        let mut ri = ImaInfo::prefilled(fill);
        let p = buf.ptr() as *const c_void;
        let rc_c = unsafe { (self.c)(&mut ci, p) };
        let rc_r = unsafe { (self.r)(&mut ri, p) };
        assert_eq!(rc_c, rc_r, "return code mismatch ({ctx})\n  buf={}", buf.hex());
        let (a, b) = (ci.raw(), ri.raw());
        if a != b {
            panic!(
                "ima_info mismatch ({ctx})\n  rc={rc_c}\n  C   : {}\n  Rust: {}\n  C   raw: {}\n  Rust raw: {}\n  buf={}",
                ci.describe(),
                ri.describe(),
                hex(&a),
                hex(&b),
                buf.hex()
            );
        }
    }

    /// Same as `diff_fill`, but the caller passes `info == NULL`.
    pub fn diff_null_info(&self, buf: &AlignedBuf, ctx: &dyn std::fmt::Display) {
        let p = buf.ptr() as *const c_void;
        let rc_c = unsafe { (self.c)(std::ptr::null_mut(), p) };
        let rc_r = unsafe { (self.r)(std::ptr::null_mut(), p) };
        assert_eq!(rc_c, rc_r, "return code mismatch with NULL info ({ctx})");
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

// ---------------------------------------------------------------------------
// Aligned / skewed byte buffer
// ---------------------------------------------------------------------------

/// Heap buffer whose exposed pointer has a controllable misalignment (`skew`).
pub struct AlignedBuf {
    base: *mut u8,
    layout: std::alloc::Layout,
    skew: usize,
    len: usize,
}

impl AlignedBuf {
    pub fn new(len: usize, skew: usize) -> Self {
        assert!(skew < 64);
        let layout = std::alloc::Layout::from_size_align(len + 64 + 64, 64).unwrap();
        let base = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!base.is_null());
        AlignedBuf { base, layout, skew: 64 + skew, len }
    }

    pub fn ptr(&self) -> *const u8 {
        unsafe { self.base.add(self.skew) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr(), self.len) }
    }

    pub fn write(&mut self, off: usize, data: &[u8]) {
        assert!(off + data.len() <= self.len, "write {off}+{} > {}", data.len(), self.len);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.base.add(self.skew + off), data.len());
        }
    }

    pub fn hex(&self) -> String {
        let b = self.bytes();
        let n = b.len().min(320);
        format!("skew={} len={} [{}{}]", self.skew - 64, self.len, hex(&b[..n]), if n < b.len() { ".." } else { "" })
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.base, self.layout) }
    }
}

// ---------------------------------------------------------------------------
// CAF file builder
// ---------------------------------------------------------------------------

pub const CHUNK_HDR: usize = 16; // sizeof(struct caf_chunk)
pub const DESC_LEN: usize = 32; // sizeof(struct caf_audio_description)
pub const PAKT_LEN: usize = 24; // sizeof(struct caf_packet_table)
pub const CAF_DATA_LEN: usize = 4; // sizeof(struct caf_data)

#[derive(Clone, Copy, Debug)]
pub struct DescFields {
    /// Raw 8 bytes at `desc->sample_rate`; the C loads them as a native double.
    pub sample_rate_raw: u64,
    /// Big-endian on the wire; `b"ima4"` for a valid file.
    pub format_id: [u8; 4],
    pub channels_per_frame: u32,
    // never read by the C -- fuzzed to prove it
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bits_per_channel: u32,
}

impl DescFields {
    pub fn valid() -> Self {
        DescFields {
            sample_rate_raw: 44100.0f64.to_bits(),
            format_id: *b"ima4",
            channels_per_frame: 2,
            format_flags: 0,
            bytes_per_packet: 34,
            frames_per_packet: 64,
            bits_per_channel: 0,
        }
    }

    pub fn encode(&self) -> [u8; DESC_LEN] {
        let mut o = [0u8; DESC_LEN];
        o[0..8].copy_from_slice(&self.sample_rate_raw.to_ne_bytes());
        o[8..12].copy_from_slice(&self.format_id);
        o[12..16].copy_from_slice(&self.format_flags.to_be_bytes());
        o[16..20].copy_from_slice(&self.bytes_per_packet.to_be_bytes());
        o[20..24].copy_from_slice(&self.frames_per_packet.to_be_bytes());
        o[24..28].copy_from_slice(&self.channels_per_frame.to_be_bytes());
        o[28..32].copy_from_slice(&self.bits_per_channel.to_be_bytes());
        o
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PaktFields {
    pub frame_count: i64,
    // never read
    pub packet_count: i64,
    pub priming_frames: i32,
    pub remainder_frames: i32,
}

impl PaktFields {
    pub fn valid() -> Self {
        PaktFields { frame_count: 1024, packet_count: 16, priming_frames: 0, remainder_frames: 0 }
    }

    pub fn encode(&self) -> [u8; PAKT_LEN] {
        let mut o = [0u8; PAKT_LEN];
        o[0..8].copy_from_slice(&self.packet_count.to_be_bytes());
        o[8..16].copy_from_slice(&self.frame_count.to_be_bytes());
        o[16..20].copy_from_slice(&self.priming_frames.to_be_bytes());
        o[20..24].copy_from_slice(&self.remainder_frames.to_be_bytes());
        o
    }
}

/// One chunk to emit: 16-byte header (`type`, 4 pad bytes, big-endian `size`)
/// followed by `payload`, then `pad_after` filler bytes.
#[derive(Clone, Debug)]
pub struct Chunk {
    pub ty: [u8; 4],
    /// Bytes 4..8 of `struct caf_chunk` -- ABI padding, never read by the C.
    pub hdr_pad: [u8; 4],
    /// The `size` field written to the wire (need not match `payload.len()`).
    pub declared_size: i64,
    pub payload: Vec<u8>,
    pub pad_after: usize,
}

impl Chunk {
    pub fn new(ty: &[u8; 4], payload: Vec<u8>) -> Self {
        Chunk {
            ty: *ty,
            hdr_pad: [0; 4],
            declared_size: payload.len() as i64,
            payload,
            pad_after: 0,
        }
    }
    pub fn desc(d: &DescFields) -> Self {
        Chunk::new(b"desc", d.encode().to_vec())
    }
    pub fn pakt(p: &PaktFields) -> Self {
        Chunk::new(b"pakt", p.encode().to_vec())
    }
    /// The `data` chunk: 4-byte `caf_data` then the audio blocks.
    pub fn data(edit_count: u32, blocks_len: usize) -> Self {
        let mut payload = Vec::with_capacity(CAF_DATA_LEN + blocks_len);
        payload.extend_from_slice(&edit_count.to_be_bytes());
        payload.extend(std::iter::repeat(0u8).take(blocks_len));
        Chunk::new(b"data", payload)
    }
    pub fn wire_len(&self) -> usize {
        CHUNK_HDR + self.payload.len() + self.pad_after
    }
    pub fn with_declared_size(mut self, n: i64) -> Self {
        self.declared_size = n;
        self
    }
    pub fn with_pad_after(mut self, n: usize) -> Self {
        self.pad_after = n;
        self
    }
    pub fn with_hdr_pad(mut self, p: [u8; 4]) -> Self {
        self.hdr_pad = p;
        self
    }
}

#[derive(Clone, Debug)]
pub struct CafBuilder {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub chunks: Vec<Chunk>,
    pub skew: usize,
    /// Extra bytes appended after all chunks (slack for negative-size walks).
    pub tail: usize,
    /// Filler byte for otherwise-unspecified space.
    pub filler: u8,
}

impl CafBuilder {
    pub fn new() -> Self {
        CafBuilder {
            magic: *b"caff",
            version: 1,
            flags: 0,
            chunks: Vec::new(),
            skew: 0,
            tail: 0,
            filler: 0,
        }
    }

    pub fn valid() -> Self {
        let mut b = CafBuilder::new();
        b.chunks.push(Chunk::desc(&DescFields::valid()));
        b.chunks.push(Chunk::pakt(&PaktFields::valid()));
        b.chunks.push(Chunk::data(0, 34 * 4));
        b
    }

    pub fn push(&mut self, c: Chunk) -> &mut Self {
        self.chunks.push(c);
        self
    }

    /// Byte offset of chunk `i` inside the produced buffer.
    pub fn chunk_offset(&self, i: usize) -> usize {
        let mut off = 8; // sizeof(struct caf_header)
        for c in &self.chunks[..i] {
            off += c.wire_len();
        }
        off
    }

    pub fn total_len(&self) -> usize {
        self.chunk_offset(self.chunks.len()) + self.tail
    }

    pub fn build(&self) -> AlignedBuf {
        let len = self.total_len().max(64);
        let mut buf = AlignedBuf::new(len, self.skew);
        if self.filler != 0 {
            let f = vec![self.filler; len];
            buf.write(0, &f);
        }
        buf.write(0, &self.magic);
        buf.write(4, &self.version.to_be_bytes());
        buf.write(6, &self.flags.to_be_bytes());
        let mut off = 8;
        for c in &self.chunks {
            buf.write(off, &c.ty);
            buf.write(off + 4, &c.hdr_pad);
            buf.write(off + 8, &c.declared_size.to_be_bytes());
            buf.write(off + 16, &c.payload);
            off += c.wire_len();
        }
        buf
    }

    /// Address the C computes for `info->blocks`, given the built buffer:
    /// `&((const struct caf_data *)&data_chunk[1])[1]`.
    pub fn expected_blocks(&self, buf: &AlignedBuf) -> Option<*const u8> {
        let i = self.chunks.iter().position(|c| &c.ty == b"data")?;
        let off = self.chunk_offset(i) + CHUNK_HDR + CAF_DATA_LEN;
        Some(unsafe { buf.ptr().add(off) })
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        (self.u64() >> 32) as u32
    }
    pub fn u16(&mut self) -> u16 {
        (self.u64() >> 48) as u16
    }
    pub fn u8(&mut self) -> u8 {
        (self.u64() >> 56) as u8
    }
    pub fn i64(&mut self) -> i64 {
        self.u64() as i64
    }
    pub fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    /// A four-byte chunk type that is none of the four recognised FourCCs.
    pub fn unknown_fourcc(&mut self) -> [u8; 4] {
        loop {
            let b = self.u32().to_be_bytes();
            if &b != b"desc" && &b != b"pakt" && &b != b"data" {
                return b;
            }
        }
    }
    pub fn desc(&mut self) -> DescFields {
        DescFields {
            sample_rate_raw: self.u64(),
            format_id: *b"ima4",
            channels_per_frame: self.u32(),
            format_flags: self.u32(),
            bytes_per_packet: self.u32(),
            frames_per_packet: self.u32(),
            bits_per_channel: self.u32(),
        }
    }
    pub fn pakt(&mut self) -> PaktFields {
        PaktFields {
            frame_count: self.i64(),
            packet_count: self.i64(),
            priming_frames: self.i32(),
            remainder_frames: self.i32(),
        }
    }
}

/// Iterations per randomized `CONFIGS.md` row.
pub const ITERS: usize = 400;

/// The interesting `f64` bit patterns for the `double`->`u64` conversion at
/// `lib.c:127` (its whole undefined-behaviour domain plus every boundary).
pub fn interesting_f64_bits() -> Vec<u64> {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        8000.0,
        22050.0,
        44100.0,
        48000.0,
        96000.0,
        192000.0,
        -44100.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        9223372036854775808.0,       // 2^63 exactly
        9223372036854774784.0,       // 2^63 - 1024 (largest f64 < 2^63)
        -9223372036854775808.0,      // -2^63 exactly
        -9223372036854777856.0,      // -2^63 - 2048
        9223372036854777856.0,       // 2^63 + 2048
        18446744073709551616.0,      // 2^64
        18446744073709549568.0,      // largest f64 < 2^64
        -18446744073709551616.0,     // -2^64
        1e300,
        -1e300,
        1e18,
        -1e18,
        4503599627370496.0,          // 2^52
        9007199254740992.0,          // 2^53
        1.0 / 3.0,
        -1.0 / 3.0,
    ];
    v.push(f64::from_bits(0x0000_0000_0000_0001)); // smallest subnormal
    v.push(f64::from_bits(0x8000_0000_0000_0001));
    let mut bits: Vec<u64> = v.iter().map(|x| x.to_bits()).collect();
    // signalling NaNs and NaNs with assorted payloads
    bits.push(0x7FF0_0000_0000_0001); // sNaN
    bits.push(0xFFF0_0000_0000_0001);
    bits.push(0x7FF8_0000_0000_0000); // qNaN
    bits.push(0xFFF8_0000_0000_0000);
    bits.push(0x7FFF_FFFF_FFFF_FFFF);
    bits.push(0xFFFF_FFFF_FFFF_FFFF);
    bits.push(0x7FF4_2424_2424_2424);
    // exact wire patterns you get when a big-endian rate is read as native LE
    for r in [8000.0f64, 22050.0, 44100.0, 48000.0, 96000.0] {
        bits.push(u64::from_le_bytes(r.to_be_bytes()));
    }
    bits
}
