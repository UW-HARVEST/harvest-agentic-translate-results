//! Shared differential-test harness.
//!
//! Both the C and the Rust implementations are loaded as **shared objects** via
//! `libloading` and driven only through their exported `ima_parse` symbol. The
//! Rust side is deliberately *not* linked directly, so the `#[no_mangle]`
//! `extern "C"` wrapper and the `repr(C)` struct layout are part of what is
//! under test.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// struct ima_info — 40 bytes, align 8 (see SYMBOLS.md for the verified offsets)
// ---------------------------------------------------------------------------

pub const INFO_SIZE: usize = 40;
pub const OFF_BLOCKS: usize = 0;
pub const OFF_SIZE: usize = 8;
pub const OFF_SAMPLE_RATE: usize = 16;
pub const OFF_FRAME_COUNT: usize = 24;
pub const OFF_CHANNEL_COUNT: usize = 32;
pub const OFF_TAIL_PADDING: usize = 36;

/// A byte-exact, 8-byte-aligned stand-in for `struct ima_info`.
///
/// Kept as raw bytes so that the *whole* struct — including the 4 tail padding
/// bytes after `channel_count`, which neither implementation may write — can be
/// compared literally, and so that `sample_rate` is compared by bit pattern
/// (NaN payloads included) rather than by `f64` equality.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InfoBuf(pub [u8; INFO_SIZE]);

/// Distinctive fill so that "field never written" is detectable.
pub const POISON: u8 = 0xA5;

impl InfoBuf {
    pub fn poisoned() -> Self {
        InfoBuf([POISON; INFO_SIZE])
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
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
        u32::from_le_bytes(self.0[OFF_CHANNEL_COUNT..OFF_CHANNEL_COUNT + 4].try_into().unwrap())
    }
    pub fn tail_padding(&self) -> [u8; 4] {
        self.0[OFF_TAIL_PADDING..INFO_SIZE].try_into().unwrap()
    }
    fn u64_at(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.0[off..off + 8].try_into().unwrap())
    }
}

impl std::fmt::Debug for InfoBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ima_info {{ blocks: {:#018x}, size: {:#018x}, sample_rate_bits: {:#018x}, \
             frame_count: {:#018x}, channel_count: {:#010x}, tail_padding: {:02x?} }}",
            self.blocks(),
            self.size(),
            self.sample_rate_bits(),
            self.frame_count(),
            self.channel_count(),
            self.tail_padding()
        )
    }
}

/// The full observable result of one `ima_parse` call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub ret: c_int,
    pub info: InfoBuf,
}

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

pub type ImaParseFn = unsafe extern "C" fn(*mut c_void, *const c_void) -> c_int;

struct Impls {
    // Keep the libraries alive for the whole process; the function pointers
    // below are borrowed from them.
    _c_lib: Library,
    _rust_lib: Library,
    c: ImaParseFn,
    rust: ImaParseFn,
}

// The loaded code is stateless (`ima_parse` touches no globals), so sharing the
// resolved function pointers across test threads is sound.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared object under test. `IMA_C_SO` overrides the default cmake
/// output, which lets the same suite be replayed against C builds made at other
/// optimisation levels (the `double` -> `unsigned long long` lowering is
/// compiler- and `-O`-dependent, so that is worth cross-checking).
fn c_so_path() -> PathBuf {
    match std::env::var_os("IMA_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src/build/libtranslated_rust.so"),
    }
}

/// The Rust cdylib lives next to the test binary's parent directory:
/// `target/<profile>/deps/<test-bin>` -> `target/<profile>/libima_parse_lib.so`.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let direct = profile.join("libima_parse_lib.so");
    if direct.exists() {
        return direct;
    }
    let in_deps = deps.join("libima_parse_lib.so");
    if in_deps.exists() {
        return in_deps;
    }
    panic!(
        "libima_parse_lib.so not found in {} or {}. Run `cargo build` first.",
        profile.display(),
        deps.display()
    );
}

fn load() -> Impls {
    let c_path = c_so_path();
    assert!(
        c_path.exists(),
        "C shared object missing at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        c_path.display()
    );
    let rust_path = rust_so_path();

    unsafe {
        let c_lib = Library::new(&c_path).expect("dlopen C .so");
        let rust_lib = Library::new(&rust_path).expect("dlopen Rust .so");

        let c_sym: Symbol<ImaParseFn> = c_lib.get(b"ima_parse\0").expect("C ima_parse");
        let rust_sym: Symbol<ImaParseFn> = rust_lib.get(b"ima_parse\0").expect("Rust ima_parse");

        let c = *c_sym;
        let rust = *rust_sym;

        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

fn impls() -> &'static Impls {
    IMPLS.get_or_init(load)
}

pub fn c_ima_parse() -> ImaParseFn {
    impls().c
}

pub fn rust_ima_parse() -> ImaParseFn {
    impls().rust
}

// ---------------------------------------------------------------------------
// Driving both implementations
// ---------------------------------------------------------------------------

/// Calls one implementation with a poisoned `info` and returns everything
/// observable.
///
/// # Safety
/// `data` must satisfy `ima_parse`'s (unchecked) contract, i.e. it must point at
/// a chunk list that terminates with a `data` chunk, or fail the `caff`/version
/// check first.
pub unsafe fn run_one(f: ImaParseFn, data: *const c_void) -> Outcome {
    let mut info = InfoBuf::poisoned();
    let ret = unsafe { f(info.as_mut_ptr(), data) };
    Outcome { ret, info }
}

/// Runs both implementations against the **same** buffer, so `info.blocks` (an
/// interior pointer) must come back bit-identical, and returns both outcomes.
pub fn run_both(buf: &[u8]) -> (Outcome, Outcome) {
    let ptr: *const c_void = buf.as_ptr().cast();
    unsafe { (run_one(c_ima_parse(), ptr), run_one(rust_ima_parse(), ptr)) }
}

/// Asserts C and Rust agree byte-for-byte on `buf`, and returns the (shared)
/// outcome.
#[track_caller]
pub fn assert_same(label: &str, buf: &[u8]) -> Outcome {
    let (c, r) = run_both(buf);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  C    ret={:3} {:?}\n  Rust ret={:3} {:?}\n  \
             buffer ({} bytes) = {}\n",
            c.ret,
            c.info,
            r.ret,
            r.info,
            buf.len(),
            hex(buf),
        );
    }
    c
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every failure reproducible
// ---------------------------------------------------------------------------

/// Uses interior mutability so that several draws can appear in one expression
/// (e.g. `f(&rng, rng.next_u32())`) without tripping the borrow checker.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(std::cell::Cell::new(seed))
    }
    pub fn next_u64(&self) -> u64 {
        let s = self.0.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.0.set(s);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u16(&self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_u8(&self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)`.
    pub fn below(&self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn bytes(&self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn arr4(&self) -> [u8; 4] {
        [self.next_u8(), self.next_u8(), self.next_u8(), self.next_u8()]
    }
    pub fn arr8(&self) -> [u8; 8] {
        let mut a = [0u8; 8];
        for b in a.iter_mut() {
            *b = self.next_u8();
        }
        a
    }
    pub fn pick<T: Copy>(&self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// CAF-as-the-C-sees-it builder
//
// NOTE: this is deliberately *not* the real CAF layout. `struct caf_chunk` is
// `{ u32 type; s64 size; }`, which the x86-64 ABI lays out as 4 bytes of type,
// **4 bytes of padding**, then 8 bytes of size — 16 bytes total. The C walks the
// buffer with that struct, so the padding is really there. Real CAF uses 12-byte
// chunk headers; the C's disagreement with the spec is part of the ground truth.
// ---------------------------------------------------------------------------

pub const CHUNK_HEADER_LEN: usize = 16;
pub const CHUNK_SIZE_OFF: usize = 8;
pub const DESC_PAYLOAD_LEN: usize = 32;
pub const PAKT_PAYLOAD_LEN: usize = 24;
pub const CAF_DATA_LEN: usize = 4;
/// `blocks` = data-chunk start + `sizeof(caf_chunk)` + `sizeof(caf_data)`.
pub const BLOCKS_OFF_FROM_CHUNK: usize = CHUNK_HEADER_LEN + CAF_DATA_LEN;

pub struct Caf {
    pub buf: Vec<u8>,
    /// Offset of the first `data` chunk header, if one was appended.
    pub data_chunk_off: Option<usize>,
}

impl Caf {
    /// `header->type` and `header->version`/`flags` are compared after
    /// `ima_btoh*`, i.e. they are big-endian in the byte stream.
    pub fn new(type4: [u8; 4], version: u16, flags: u16) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&type4);
        buf.extend_from_slice(&version.to_be_bytes());
        buf.extend_from_slice(&flags.to_be_bytes());
        Caf {
            buf,
            data_chunk_off: None,
        }
    }

    /// A well-formed `"caff"` v1 header with random `flags`.
    pub fn valid_header(rng: &Rng) -> Self {
        Self::new(*b"caff", 1, rng.next_u16())
    }

    pub fn offset(&self) -> usize {
        self.buf.len()
    }

    /// Appends a raw chunk: 4-byte type, 4 bytes of struct padding, big-endian
    /// `size` field, then `payload` verbatim. `size` is *independent* of
    /// `payload.len()` on purpose — the C uses it only as the walk stride.
    pub fn chunk_raw(&mut self, type4: [u8; 4], pad: [u8; 4], size: i64, payload: &[u8]) -> usize {
        let off = self.buf.len();
        self.buf.extend_from_slice(&type4);
        self.buf.extend_from_slice(&pad);
        self.buf.extend_from_slice(&size.to_be_bytes());
        self.buf.extend_from_slice(payload);
        off
    }

    /// Appends a chunk whose `size` field matches its payload, so the walk lands
    /// exactly on the next chunk.
    pub fn chunk(&mut self, type4: [u8; 4], pad: [u8; 4], payload: &[u8]) -> usize {
        self.chunk_raw(type4, pad, payload.len() as i64, payload)
    }

    pub fn desc(&mut self, pad: [u8; 4], d: &Desc) -> usize {
        self.chunk(*b"desc", pad, &d.encode())
    }

    pub fn pakt(&mut self, pad: [u8; 4], p: &Pakt) -> usize {
        self.chunk(*b"pakt", pad, &p.encode())
    }

    /// Appends a `data` chunk. Its `size` field is what lands in `info->size`,
    /// and it is never used as a stride (the loop breaks), so any `i64` is safe.
    pub fn data(&mut self, pad: [u8; 4], size_field: i64, edit_count: u32, trailing: &[u8]) -> usize {
        let mut payload = Vec::new();
        payload.extend_from_slice(&edit_count.to_be_bytes());
        payload.extend_from_slice(trailing);
        let off = self.chunk_raw(*b"data", pad, size_field, &payload);
        if self.data_chunk_off.is_none() {
            self.data_chunk_off = Some(off);
        }
        off
    }

    /// Rewrites the big-endian `size` field of an already-appended chunk. Used
    /// to build forward/backward jumps whose targets are only known later.
    pub fn set_chunk_size(&mut self, chunk_off: usize, size: i64) {
        let at = chunk_off + CHUNK_SIZE_OFF;
        self.buf[at..at + 8].copy_from_slice(&size.to_be_bytes());
    }

    /// Places the stream inside a larger allocation so that its base address has
    /// the requested residue mod 8, exercising the misaligned `struct` loads.
    /// Returns the backing allocation and the start offset.
    pub fn aligned_copy(&self, residue: usize) -> (Vec<u8>, usize) {
        assert!(residue < 8);
        let mut v = vec![0xCCu8; self.buf.len() + 8];
        let base = v.as_ptr() as usize;
        let k = (residue + 8 - (base % 8)) % 8;
        v[k..k + self.buf.len()].copy_from_slice(&self.buf);
        debug_assert_eq!((v.as_ptr() as usize + k) % 8, residue);
        (v, k)
    }

    /// Expected `info->blocks` for the first `data` chunk, given the base
    /// address the buffer is passed at.
    pub fn expected_blocks(&self, base: *const u8) -> u64 {
        let off = self.data_chunk_off.expect("no data chunk appended");
        (base as u64).wrapping_add((off + BLOCKS_OFF_FROM_CHUNK) as u64)
    }
}

/// `struct caf_audio_description` (32 bytes). Every multi-byte field is stored
/// big-endian, except `sample_rate`, which is kept as **raw bytes** because the
/// C reads it as a *native* (little-endian) `double` and then value-converts it.
#[derive(Clone, Copy, Debug)]
pub struct Desc {
    pub sample_rate_raw: [u8; 8],
    pub format_id: [u8; 4],
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
}

impl Desc {
    /// A valid `ima4` description with a realistic big-endian 44100 Hz rate.
    pub fn ima4() -> Self {
        Desc {
            sample_rate_raw: 44100.0f64.to_be_bytes(),
            format_id: *b"ima4",
            format_flags: 0,
            bytes_per_packet: 34,
            frames_per_packet: 64,
            channels_per_frame: 1,
            bits_per_channel: 4,
        }
    }

    /// `ima4`, with every ignored field filled with garbage and a random rate.
    pub fn random_ima4(rng: &Rng) -> Self {
        Desc {
            sample_rate_raw: rng.arr8(),
            format_id: *b"ima4",
            format_flags: rng.next_u32(),
            bytes_per_packet: rng.next_u32(),
            frames_per_packet: rng.next_u32(),
            channels_per_frame: rng.next_u32(),
            bits_per_channel: rng.next_u32(),
        }
    }

    /// Sets `sample_rate` so the C's *native* `double` read sees exactly `v`.
    pub fn with_native_rate(mut self, v: f64) -> Self {
        self.sample_rate_raw = v.to_bits().to_le_bytes();
        self
    }

    /// Sets `sample_rate` so the C's native read sees the `double` with bit
    /// pattern `bits`.
    pub fn with_native_rate_bits(mut self, bits: u64) -> Self {
        self.sample_rate_raw = bits.to_le_bytes();
        self
    }

    /// Stores `v` the way a real CAF writer would (big-endian).
    pub fn with_be_rate(mut self, v: f64) -> Self {
        self.sample_rate_raw = v.to_be_bytes();
        self
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(DESC_PAYLOAD_LEN);
        v.extend_from_slice(&self.sample_rate_raw);
        v.extend_from_slice(&self.format_id);
        v.extend_from_slice(&self.format_flags.to_be_bytes());
        v.extend_from_slice(&self.bytes_per_packet.to_be_bytes());
        v.extend_from_slice(&self.frames_per_packet.to_be_bytes());
        v.extend_from_slice(&self.channels_per_frame.to_be_bytes());
        v.extend_from_slice(&self.bits_per_channel.to_be_bytes());
        debug_assert_eq!(v.len(), DESC_PAYLOAD_LEN);
        v
    }
}

/// `struct caf_packet_table` (24 bytes), all fields big-endian.
#[derive(Clone, Copy, Debug)]
pub struct Pakt {
    pub packet_count: i64,
    pub frame_count: i64,
    pub priming_frames: i32,
    pub remainder_frames: i32,
}

impl Pakt {
    pub fn new(frame_count: i64) -> Self {
        Pakt {
            packet_count: 0,
            frame_count,
            priming_frames: 0,
            remainder_frames: 0,
        }
    }

    /// Random `frame_count` plus garbage in every ignored field.
    pub fn random(rng: &Rng) -> Self {
        Pakt {
            packet_count: rng.next_u64() as i64,
            frame_count: rng.next_u64() as i64,
            priming_frames: rng.next_u32() as i32,
            remainder_frames: rng.next_u32() as i32,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(PAKT_PAYLOAD_LEN);
        v.extend_from_slice(&self.packet_count.to_be_bytes());
        v.extend_from_slice(&self.frame_count.to_be_bytes());
        v.extend_from_slice(&self.priming_frames.to_be_bytes());
        v.extend_from_slice(&self.remainder_frames.to_be_bytes());
        debug_assert_eq!(v.len(), PAKT_PAYLOAD_LEN);
        v
    }
}

/// Four-byte codes that are *not* `desc`/`pakt`/`data`, for filler chunks.
pub const UNKNOWN_TYPES: &[[u8; 4]] = &[
    *b"free", *b"kuki", *b"strg", *b"uuid", *b"info", *b"ovvw", *b"peak", *b"mark", *b"regn",
    *b"umid", *b"chan", *b"AAAA", *b"\x00\x00\x00\x00", *b"\xff\xff\xff\xff",
];

pub fn is_known_type(t: [u8; 4]) -> bool {
    t == *b"desc" || t == *b"pakt" || t == *b"data"
}

/// The reference model of the C's `double` -> `unsigned long long` conversion,
/// used only to *describe* expectations in assertions; correctness is always
/// decided by the C `.so` itself.
#[allow(clippy::manual_range_contains)]
pub fn model_f64_to_u64(x: f64) -> u64 {
    const TWO63: f64 = 9_223_372_036_854_775_808.0;
    fn cvttsd2si(x: f64) -> i64 {
        if x.is_nan() {
            return i64::MIN;
        }
        let t = x.trunc();
        if t >= -TWO63 && t < TWO63 {
            t as i64
        } else {
            i64::MIN
        }
    }
    if x >= TWO63 {
        (cvttsd2si(x - TWO63) as u64) ^ (1u64 << 63)
    } else {
        cvttsd2si(x) as u64
    }
}
