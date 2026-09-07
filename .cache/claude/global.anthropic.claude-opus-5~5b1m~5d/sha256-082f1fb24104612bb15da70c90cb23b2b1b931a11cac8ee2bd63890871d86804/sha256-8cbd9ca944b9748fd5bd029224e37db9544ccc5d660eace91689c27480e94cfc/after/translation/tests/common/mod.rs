//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! only through their exported `ima_parse` symbol — the Rust crate is never
//! linked directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
#![allow(dead_code)]

use std::ffi::c_void;
use std::path::PathBuf;

pub type ImaParseFn = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> i32;

/// `struct ima_info` — size 40, align 8 (verified against the C `sizeof`).
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
    /// Poison the whole 40-byte footprint (including the tail padding) so that
    /// "the C left `*info` untouched" is actually observable.
    pub fn poisoned(pattern: u8) -> Self {
        unsafe {
            let mut raw = [pattern; core::mem::size_of::<ImaInfo>()];
            // keep the pattern deterministic but not uniform
            for (i, b) in raw.iter_mut().enumerate() {
                *b = pattern ^ (i as u8);
            }
            core::mem::transmute::<[u8; 40], ImaInfo>(raw)
        }
    }

    pub fn raw(&self) -> [u8; 40] {
        unsafe { core::mem::transmute::<ImaInfo, [u8; 40]>(*self) }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found = Some(e.path());
            }
        }
    }
    found.unwrap_or_else(|| panic!("no C .so found in {}", build.display()))
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libima_parse_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libima_parse_lib.so not built; run `cargo build --release` first");
}

pub struct Libs {
    _c: libloading::Library,
    _r: libloading::Library,
    pub c_parse: ImaParseFn,
    pub r_parse: ImaParseFn,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = libloading::Library::new(c_so_path()).expect("load C .so");
            let r = libloading::Library::new(rust_so_path()).expect("load Rust .so");
            let c_parse = *c
                .get::<ImaParseFn>(b"ima_parse\0")
                .expect("C .so does not export ima_parse");
            let r_parse = *r
                .get::<ImaParseFn>(b"ima_parse\0")
                .expect("Rust .so does not export ima_parse");
            Libs {
                _c: c,
                _r: r,
                c_parse,
                r_parse,
            }
        }
    }

    /// Call both libraries on the *same* buffer address with identically
    /// poisoned output structs and return `(rc, raw info bytes)` for each.
    pub fn call_both(&self, buf: &[u8], poison: u8) -> ((i32, [u8; 40]), (i32, [u8; 40])) {
        let mut ci = ImaInfo::poisoned(poison);
        let mut ri = ImaInfo::poisoned(poison);
        let p = buf.as_ptr() as *const c_void;
        let crc = unsafe { (self.c_parse)(&mut ci, p) };
        let rrc = unsafe { (self.r_parse)(&mut ri, p) };
        ((crc, ci.raw()), (rrc, ri.raw()))
    }

    /// Assert byte-identical results. `ctx` identifies the failing case.
    pub fn assert_same(&self, ctx: &str, buf: &[u8]) {
        let ((crc, cb), (rrc, rb)) = self.call_both(buf, 0xA5);
        if crc != rrc || cb != rb {
            panic!(
                "DIVERGENCE [{ctx}]\n  C  rc={crc} info={}\n  Rust rc={rrc} info={}\n  buf({} bytes)={}",
                hex(&cb),
                hex(&rb),
                buf.len(),
                hex(&buf[..buf.len().min(256)])
            );
        }
        // A second run with a different poison pattern proves neither library
        // leaves fields half-written in a pattern-dependent way.
        let ((crc2, cb2), (rrc2, rb2)) = self.call_both(buf, 0x00);
        assert_eq!(crc2, rrc2, "rc mismatch (poison 0x00) [{ctx}]");
        assert_eq!(cb2, rb2, "info mismatch (poison 0x00) [{ctx}]");
        assert_eq!(crc, crc2, "C not deterministic [{ctx}]");
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng {
    state: core::cell::Cell<u64>,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng {
            state: core::cell::Cell::new(seed),
        }
    }
    pub fn next_u64(&self) -> u64 {
        let s = self.state.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.state.set(s);
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
    /// uniform in `0..n`
    pub fn below(&self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn fill(&self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// CAF document builder — mirrors the *padded* layout the C structs impose.
//
//   struct caf_header  size  8: type@0 (BE u32), version@4 (BE u16), flags@6
//   struct caf_chunk   size 16: type@0 (BE u32), [4 bytes padding @4], size@8 (BE s64)
//   desc body (32): sample_rate@0 format_id@8 format_flags@12 bytes_per_packet@16
//                   frames_per_packet@20 channels_per_frame@24 bits_per_channel@28
//   pakt body (24): packet_count@0 frame_count@8 priming@16 remainder@20
//   data body:      edit_count@0, then struct ima_block[] (34 bytes each)
// ---------------------------------------------------------------------------

pub const FOURCC_CAFF: u32 = 0x6361_6666; // 'c','a','f','f' big-endian
pub const FOURCC_DESC: u32 = 0x6465_7363; // 'd','e','s','c'
pub const FOURCC_PAKT: u32 = 0x7061_6b74; // 'p','a','k','t'
pub const FOURCC_DATA: u32 = 0x6461_7461; // 'd','a','t','a'
pub const FOURCC_IMA4: u32 = 0x696d_6134; // 'i','m','a','4'

pub const CHUNK_HDR: usize = 16;
pub const DESC_BODY: usize = 32;
pub const PAKT_BODY: usize = 24;

pub struct Doc {
    pub bytes: Vec<u8>,
}

impl Doc {
    pub fn new(rng: &Rng) -> Doc {
        Doc::with_header(rng, FOURCC_CAFF, 1)
    }

    pub fn with_header(rng: &Rng, ty: u32, version: u16) -> Doc {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&ty.to_be_bytes());
        bytes.extend_from_slice(&version.to_be_bytes());
        // flags is never read by the C — randomize it.
        bytes.extend_from_slice(&rng.next_u16().to_be_bytes());
        Doc { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Emit a 16-byte chunk header. The 4 padding bytes at +4 are never read by
    /// the C, so they are randomized.
    pub fn chunk_hdr(&mut self, rng: &Rng, ty: u32, size: i64) {
        self.bytes.extend_from_slice(&ty.to_be_bytes());
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes());
        self.bytes.extend_from_slice(&size.to_be_bytes());
    }

    /// `desc` chunk. `sample_rate_raw` is written verbatim (the C loads those 8
    /// bytes as a *native* double, so this is the direct control knob for A9).
    pub fn desc(&mut self, rng: &Rng, sample_rate_raw: u64, format_id: u32, channels: u32) {
        self.chunk_hdr(rng, FOURCC_DESC, DESC_BODY as i64);
        self.bytes.extend_from_slice(&sample_rate_raw.to_le_bytes()); // native order
        self.bytes.extend_from_slice(&format_id.to_be_bytes());
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // format_flags (unread)
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // bytes_per_packet
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // frames_per_packet
        self.bytes.extend_from_slice(&channels.to_be_bytes());
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // bits_per_channel
    }

    pub fn pakt(&mut self, rng: &Rng, frame_count: u64) {
        self.chunk_hdr(rng, FOURCC_PAKT, PAKT_BODY as i64);
        self.bytes.extend_from_slice(&rng.next_u64().to_be_bytes()); // packet_count (unread)
        self.bytes.extend_from_slice(&frame_count.to_be_bytes());
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // priming (unread)
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // remainder (unread)
    }

    /// Unknown chunk of `body` random bytes, declaring `size` (defaults to the
    /// real body length so the scan stays in step).
    pub fn unknown(&mut self, rng: &Rng, body: usize, size: i64) {
        let mut ty = rng.next_u32();
        while ty == FOURCC_DESC || ty == FOURCC_PAKT || ty == FOURCC_DATA {
            ty = rng.next_u32();
        }
        self.chunk_hdr(rng, ty, size);
        let mut b = vec![0u8; body];
        rng.fill(&mut b);
        self.bytes.extend_from_slice(&b);
    }

    /// `data` chunk header + `edit_count` + `nblocks` random `ima_block`s.
    /// Returns the file offset of the `data` chunk header.
    pub fn data(&mut self, rng: &Rng, size: i64, nblocks: usize) -> usize {
        let off = self.bytes.len();
        self.chunk_hdr(rng, FOURCC_DATA, size);
        self.bytes.extend_from_slice(&rng.next_u32().to_be_bytes()); // edit_count (unread)
        let mut b = vec![0u8; nblocks * 34];
        rng.fill(&mut b);
        self.bytes.extend_from_slice(&b);
        off
    }

    pub fn pad(&mut self, rng: &Rng, n: usize) {
        let mut b = vec![0u8; n];
        rng.fill(&mut b);
        self.bytes.extend_from_slice(&b);
    }
}

/// A canonical valid document: `desc` -> `pakt` -> `data`.
pub fn valid_doc(
    rng: &Rng,
    sample_rate_raw: u64,
    channels: u32,
    frame_count: u64,
    data_size: i64,
    nblocks: usize,
) -> Vec<u8> {
    let mut d = Doc::new(rng);
    d.desc(rng, sample_rate_raw, FOURCC_IMA4, channels);
    d.pakt(rng, frame_count);
    d.data(rng, data_size, nblocks);
    d.bytes
}

/// Buffer that starts at an address `off` bytes past an 8-aligned address.
pub struct Misaligned {
    backing: Vec<u8>,
    start: usize,
    len: usize,
}

impl Misaligned {
    pub fn new(data: &[u8], off: usize) -> Misaligned {
        let mut backing = vec![0u8; data.len() + 64];
        let base = backing.as_ptr() as usize;
        let aligned = (base + 15) & !15usize;
        let start = (aligned - base) + off;
        backing[start..start + data.len()].copy_from_slice(data);
        Misaligned {
            backing,
            start,
            len: data.len(),
        }
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.backing[self.start..self.start + self.len]
    }
    pub fn addr_mod8(&self) -> usize {
        self.as_slice().as_ptr() as usize % 8
    }
}
