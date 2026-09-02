//! Differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! `ima_parse` across the FFI boundary in both. No Rust function is ever called
//! directly — everything goes through the exported `#[no_mangle]` symbol, so the
//! export wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror of `struct ima_info` (include/lib.h): size 40, align 8.
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

impl ImaInfo {
    /// Recognisable sentinel so we can prove the error paths write nothing.
    pub fn sentinel() -> Self {
        ImaInfo {
            blocks: 0xDEAD_BEEF_0000_1111u64 as *const u8,
            size: 0xA5A5_A5A5_A5A5_A5A5,
            sample_rate: f64::from_bits(0x1234_5678_9ABC_DEF0),
            frame_count: 0x5A5A_5A5A_5A5A_5A5A,
            channel_count: 0xCAFE_BABE,
        }
    }
}

pub type ImaParseFn = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

/// Everything `ima_parse` can observably produce.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    /// absolute `blocks` pointer as an integer (both impls get the same buffer,
    /// so this must be bit-identical, not merely "similar")
    pub blocks: usize,
    pub size: u64,
    /// raw bits, so NaN payloads and signed zeros are compared exactly
    pub sample_bits: u64,
    pub frame_count: u64,
    pub channel_count: u32,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ ret: {}, blocks: {:#x}, size: {:#x} ({}), sample_bits: {:#018x} ({:?}), \
             frame_count: {:#x} ({}), channel_count: {:#x} }}",
            self.ret,
            self.blocks,
            self.size,
            self.size as i64,
            self.sample_bits,
            f64::from_bits(self.sample_bits),
            self.frame_count,
            self.frame_count as i64,
            self.channel_count
        )
    }
}

impl Outcome {
    fn from(ret: c_int, info: &ImaInfo) -> Self {
        Outcome {
            ret,
            blocks: info.blocks as usize,
            size: info.size,
            sample_bits: info.sample_rate.to_bits(),
            frame_count: info.frame_count,
            channel_count: info.channel_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

struct Impls {
    c: ImaParseFn,
    rust: ImaParseFn,
    _c_lib: Library,
    _rust_lib: Library,
}

// Raw `fn` pointers and `Library` are both Send+Sync.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        dir.display()
    );
    found.remove(0)
}

/// `cargo test` does **not** build a `cdylib`-only lib target, so the `.so`
/// must be produced by an explicit `cargo build` (see `run_tests.sh`). We look
/// only in the *current* profile directory — never in the sibling profile —
/// and we refuse to run against a `.so` older than `src/lib.rs`, so a stale
/// artifact can never silently pass the differential tests.
fn find_rust_so() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let so = profile_dir.join("libima_parse_lib.so");
    assert!(
        so.exists(),
        "Rust cdylib {} not found.\n`cargo test` does not build a cdylib-only lib target — run\n  \
         cargo build            (for `cargo test`)\n  cargo build --release  (for `cargo test --release`)\n\
         or just use ./run_tests.sh",
        so.display()
    );

    let src = manifest_dir().join("src/lib.rs");
    let so_t = std::fs::metadata(&so).and_then(|m| m.modified()).ok();
    let src_t = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
    if let (Some(a), Some(b)) = (so_t, src_t) {
        assert!(
            a >= b,
            "STALE ARTIFACT: {} is older than {}. Re-run `cargo build` \
             (or ./run_tests.sh) — the differential tests would otherwise be \
             testing an old translation.",
            so.display(),
            src.display()
        );
    }
    so
}

fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));
        let c: Symbol<ImaParseFn> = c_lib
            .get(b"ima_parse\0")
            .expect("C .so does not export ima_parse");
        let rust: Symbol<ImaParseFn> = rust_lib
            .get(b"ima_parse\0")
            .expect("Rust .so does not export ima_parse");
        let c = *c;
        let rust = *rust;
        Impls {
            c,
            rust,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    })
}

/// Force both libraries to be loaded (used by the symbol-parity test).
pub fn ensure_loaded() {
    let _ = impls();
}

/// Raw addresses of the two resolved `ima_parse` implementations. Used to prove
/// the harness really loaded two different libraries (and did not resolve both
/// names to the same symbol through global interposition).
pub fn fn_ptrs() -> (usize, usize) {
    let i = impls();
    (i.c as usize, i.rust as usize)
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

// ---------------------------------------------------------------------------
// 8-byte aligned scratch buffer with a controllable misalignment offset
// ---------------------------------------------------------------------------

pub struct Buf {
    words: Vec<u64>,
    off: usize,
    len: usize,
}

impl Buf {
    /// `off` is the *misalignment* of the returned pointer relative to an
    /// 8-byte boundary.
    pub fn new(bytes: &[u8], off: usize) -> Self {
        let total = bytes.len() + off + 64;
        let mut words = vec![0u64; total / 8 + 2];
        let len = bytes.len();
        unsafe {
            let base = (words.as_mut_ptr() as *mut u8).add(off);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base, len);
        }
        Buf { words, off, len }
    }

    pub fn ptr(&self) -> *const c_void {
        unsafe { (self.words.as_ptr() as *const u8).add(self.off) as *const c_void }
    }

    /// Restore the original contents (and re-zero the slack) so the same
    /// allocation — and therefore the same address — can be reused.
    pub fn reset(&mut self, bytes: &[u8]) {
        assert_eq!(bytes.len(), self.len);
        for w in self.words.iter_mut() {
            *w = 0;
        }
        unsafe {
            let base = (self.words.as_mut_ptr() as *mut u8).add(self.off);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base, self.len);
        }
    }

    /// Patch bytes in place (used by the high-volume sweeps so they do not have
    /// to rebuild and reallocate a whole file per iteration).
    pub fn write_at(&mut self, at: usize, src: &[u8]) {
        assert!(at + src.len() <= self.len);
        unsafe {
            let base = (self.words.as_mut_ptr() as *mut u8).add(self.off + at);
            std::ptr::copy_nonoverlapping(src.as_ptr(), base, src.len());
        }
    }

    pub fn base(&self) -> usize {
        self.ptr() as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

// ---------------------------------------------------------------------------
// The differential call
// ---------------------------------------------------------------------------

/// Call both `.so`s with the same buffer pointer and the same pre-filled
/// `ima_info`, and return `(c_outcome, rust_outcome)`.
pub fn call_both(buf: &Buf) -> (Outcome, Outcome) {
    let f = impls();
    let mut ci = ImaInfo::sentinel();
    let mut ri = ImaInfo::sentinel();
    let c_ret = unsafe { (f.c)(&mut ci, buf.ptr()) };
    let r_ret = unsafe { (f.rust)(&mut ri, buf.ptr()) };
    (Outcome::from(c_ret, &ci), Outcome::from(r_ret, &ri))
}

/// Same, but passes `info == NULL` to both (only legal on error paths).
pub fn call_both_null_info(buf: &Buf) -> (c_int, c_int) {
    let f = impls();
    let c_ret = unsafe { (f.c)(std::ptr::null_mut(), buf.ptr()) };
    let r_ret = unsafe { (f.rust)(std::ptr::null_mut(), buf.ptr()) };
    (c_ret, r_ret)
}

/// Call both with `info` pointing *into the input buffer itself*, so the order
/// in which `ima_parse` interleaves its writes to `*info` with its reads of
/// `desc`/`pakt` becomes observable.
///
/// Both implementations are run against the **same allocation at the same
/// address** (the buffer is restored to its original contents in between), so
/// every byte — including any pointer value written into the buffer — is
/// directly comparable.
pub fn call_both_aliased(
    bytes: &[u8],
    off: usize,
    info_off: usize,
) -> ((Outcome, Vec<u8>), (Outcome, Vec<u8>)) {
    assert!(
        info_off + core::mem::size_of::<ImaInfo>() <= bytes.len() + 64,
        "aliased info must stay inside the allocation"
    );
    let f = impls();
    let mut buf = Buf::new(bytes, off);
    let base = buf.base();
    let info = (base + info_off) as *mut ImaInfo;
    let mut out: Vec<(Outcome, Vec<u8>)> = Vec::with_capacity(2);
    for which in 0..2 {
        buf.reset(bytes);
        let ret = unsafe {
            let g = if which == 0 { f.c } else { f.rust };
            g(info, buf.ptr())
        };
        let info_val = unsafe { std::ptr::read_unaligned(info) };
        let o = Outcome::from(ret, &info_val);
        let final_bytes =
            unsafe { std::slice::from_raw_parts(base as *const u8, bytes.len()) }.to_vec();
        out.push((o, final_bytes));
    }
    let b = out.pop().unwrap();
    let a = out.pop().unwrap();
    (a, b)
}

/// Assert the two implementations agree, printing a hex dump on divergence.
#[track_caller]
pub fn assert_same(label: &str, bytes: &[u8], off: usize) -> Outcome {
    let buf = Buf::new(bytes, off);
    let (c, r) = call_both(&buf);
    if c != r {
        panic!(
            "DIVERGENCE in {label} (misalign={off})\n  C    = {c:?}\n  RUST = {r:?}\n  \
             base = {:#x}\n  input ({} bytes) = {}\n",
            buf.base(),
            bytes.len(),
            hex(bytes)
        );
    }
    c
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for (i, x) in b.iter().enumerate() {
        if i % 16 == 0 {
            s.push_str(&format!("\n    {:04x}: ", i));
        }
        s.push_str(&format!("{:02x} ", x));
    }
    s
}

// ---------------------------------------------------------------------------
// SplitMix64 — deterministic, seeded PRNG (no external crate)
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
        (self.u64() >> 32) as u32
    }
    pub fn u16(&mut self) -> u16 {
        (self.u64() >> 48) as u16
    }
    pub fn u8(&mut self) -> u8 {
        (self.u64() >> 56) as u8
    }
    /// uniform-ish in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        (self.u64() % (n as u64)) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// CAF-ish file builder matching the *C struct layouts* (not the real CAF spec)
// ---------------------------------------------------------------------------

/// `struct caf_chunk` is `{ u32 type; i64 size; }` => size 16, `size` at
/// offset 8 (4 bytes of padding at offset 4). The C walks
/// `chunk = (u8*)&chunk[1] + chunk_size`, i.e. `chunk + 16 + size`.
pub const CHUNK_HDR: usize = 16;
/// `struct caf_header` is `{ u32 type; u16 version; u16 flags; }` => 8 bytes.
pub const FILE_HDR: usize = 8;

pub const FOURCC_DESC: [u8; 4] = *b"desc";
pub const FOURCC_PAKT: [u8; 4] = *b"pakt";
pub const FOURCC_DATA: [u8; 4] = *b"data";
pub const FOURCC_IMA4: [u8; 4] = *b"ima4";

#[derive(Clone)]
pub struct Chunk {
    pub fourcc: [u8; 4],
    /// the 4 padding bytes of `struct caf_chunk` — never read by the C
    pub pad: [u8; 4],
    /// value written to the `size` field (big-endian i64)
    pub size: i64,
    /// bytes physically emitted after the 16-byte chunk header
    pub payload: Vec<u8>,
}

impl Chunk {
    /// A chunk whose declared `size` matches its physical payload length.
    pub fn exact(fourcc: [u8; 4], payload: Vec<u8>) -> Self {
        Chunk {
            fourcc,
            pad: [0; 4],
            size: payload.len() as i64,
            payload,
        }
    }
    pub fn with_pad(mut self, pad: [u8; 4]) -> Self {
        self.pad = pad;
        self
    }
    pub fn with_size(mut self, size: i64) -> Self {
        self.size = size;
        self
    }
    pub fn total(&self) -> usize {
        CHUNK_HDR + self.payload.len()
    }
}

/// `struct caf_audio_description`: f64 @0, then six u32 at 8,12,16,20,24,28.
#[derive(Clone, Copy)]
pub struct DescBody {
    /// stored as native-endian f64 bits (the C reads it as a plain `double`)
    pub sample_rate_bits: u64,
    /// stored big-endian
    pub format_id: [u8; 4],
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    /// stored big-endian
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
}

impl DescBody {
    pub fn ima4(sample_rate: f64, channels: u32) -> Self {
        DescBody {
            sample_rate_bits: sample_rate.to_bits(),
            format_id: FOURCC_IMA4,
            format_flags: 0,
            bytes_per_packet: 34,
            frames_per_packet: 64,
            channels_per_frame: channels,
            bits_per_channel: 4,
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(32);
        v.extend_from_slice(&self.sample_rate_bits.to_ne_bytes());
        v.extend_from_slice(&self.format_id);
        v.extend_from_slice(&self.format_flags.to_be_bytes());
        v.extend_from_slice(&self.bytes_per_packet.to_be_bytes());
        v.extend_from_slice(&self.frames_per_packet.to_be_bytes());
        v.extend_from_slice(&self.channels_per_frame.to_be_bytes());
        v.extend_from_slice(&self.bits_per_channel.to_be_bytes());
        assert_eq!(v.len(), 32);
        v
    }
}

/// `struct caf_packet_table`: i64 @0, i64 @8, i32 @16, i32 @20 => 24 bytes.
#[derive(Clone, Copy)]
pub struct PaktBody {
    pub packet_count: i64,
    pub frame_count: i64,
    pub priming_frames: i32,
    pub remainder_frames: i32,
}

impl PaktBody {
    pub fn simple(frame_count: i64) -> Self {
        PaktBody {
            packet_count: 1,
            frame_count,
            priming_frames: 0,
            remainder_frames: 0,
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(24);
        v.extend_from_slice(&self.packet_count.to_be_bytes());
        v.extend_from_slice(&self.frame_count.to_be_bytes());
        v.extend_from_slice(&self.priming_frames.to_be_bytes());
        v.extend_from_slice(&self.remainder_frames.to_be_bytes());
        assert_eq!(v.len(), 24);
        v
    }
}

/// `data` chunk payload: `struct caf_data { u32 edit_count; }` then the blocks.
pub fn data_payload(edit_count: u32, block_bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + block_bytes.len());
    v.extend_from_slice(&edit_count.to_be_bytes());
    v.extend_from_slice(block_bytes);
    v
}

/// A whole synthetic file plus the physical offsets of each chunk.
pub struct File {
    pub bytes: Vec<u8>,
    pub chunk_offsets: Vec<usize>,
}

pub fn build(magic: [u8; 4], version: u16, flags: u16, chunks: &[Chunk]) -> File {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&magic);
    bytes.extend_from_slice(&version.to_be_bytes());
    bytes.extend_from_slice(&flags.to_be_bytes());
    assert_eq!(bytes.len(), FILE_HDR);
    let mut chunk_offsets = Vec::with_capacity(chunks.len());
    for ch in chunks {
        chunk_offsets.push(bytes.len());
        bytes.extend_from_slice(&ch.fourcc);
        bytes.extend_from_slice(&ch.pad);
        bytes.extend_from_slice(&ch.size.to_be_bytes());
        bytes.extend_from_slice(&ch.payload);
    }
    File {
        bytes,
        chunk_offsets,
    }
}

/// Convenience: a valid `caff`/v1 file.
pub fn build_valid(flags: u16, chunks: &[Chunk]) -> File {
    build(*b"caff", 1, flags, chunks)
}

/// Any 4-byte FourCC that is none of `desc`/`pakt`/`data`.
pub fn unknown_fourcc(rng: &mut Rng) -> [u8; 4] {
    loop {
        let c = rng.u32().to_be_bytes();
        if c != FOURCC_DESC && c != FOURCC_PAKT && c != FOURCC_DATA {
            return c;
        }
    }
}

/// Pad the tail so that reads past the declared payloads stay inside the
/// allocation (the C happily reads past logical ends; we only need the memory
/// to exist so the *test process* does not fault).
pub fn with_tail(mut f: File, n: usize, rng: &mut Rng) -> File {
    f.bytes.extend(rng.bytes(n));
    f
}
