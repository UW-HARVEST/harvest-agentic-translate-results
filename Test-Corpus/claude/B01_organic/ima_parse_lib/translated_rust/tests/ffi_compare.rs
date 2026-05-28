// Integration test: load both the C .so and the Rust .so via libloading,
// call ima_parse on the same inputs, and compare outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImaBlock {
    pub preamble: u16,
    pub data: [u8; 32],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImaInfo {
    pub blocks: *const ImaBlock,
    pub size: u64,
    pub sample_rate: f64,
    pub frame_count: u64,
    pub channel_count: u32,
}

type ImaParseFn = unsafe extern "C" fn(*mut ImaInfo, *const c_void) -> c_int;

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    // cargo test default profile = debug
    if std::path::Path::new("target/debug/libima_parse_lib.so").exists() {
        "target/debug/libima_parse_lib.so"
    } else {
        "target/release/libima_parse_lib.so"
    }
}

unsafe fn load(lib_path: &str) -> (Library, ImaParseFn) {
    let lib = Library::new(lib_path).expect("failed to load .so");
    let sym: Symbol<ImaParseFn> = lib
        .get(b"ima_parse\0")
        .expect("failed to find ima_parse symbol");
    let f: ImaParseFn = *sym;
    (lib, f)
}

// Helpers: write big-endian primitives at an offset into a Vec<u8>.
fn write_u16_be(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_be_bytes());
}
fn write_u32_be(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}
fn write_u64_be(buf: &mut [u8], off: usize, v: u64) {
    buf[off..off + 8].copy_from_slice(&v.to_be_bytes());
}
fn write_i64_be(buf: &mut [u8], off: usize, v: i64) {
    buf[off..off + 8].copy_from_slice(&v.to_be_bytes());
}
fn write_i32_be(buf: &mut [u8], off: usize, v: i32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}
fn write_f64_be(buf: &mut [u8], off: usize, v: f64) {
    buf[off..off + 8].copy_from_slice(&v.to_be_bytes());
}

// Layout (matching the C struct sizes including natural padding):
//   caf_header: 8 bytes (u32 type, u16 version, u16 flags)
//   caf_chunk:  16 bytes (u32 type + 4 pad + s64 size)
//   caf_audio_description: 32 bytes (f64 + 6*u32)
//   caf_packet_table: 24 bytes (s64 + s64 + s32 + s32)
//   caf_data: 4 bytes (u32)
//   ima_block: 34 bytes (u16 preamble + 32 data)

const HEADER: usize = 8;
const CHUNK: usize = 16;
const DESC: usize = 32;
const PAKT: usize = 24;
const CAFDATA: usize = 4;
const BLOCK: usize = 34;

fn fourcc_be(s: &[u8; 4]) -> u32 {
    // The C parser does: ima_btoh32(header->type) == ('f' | 'f'<<8 | 'a'<<16 | 'c'<<24)
    // ima_btoh32 is byte-swap. So memory must hold "caff" left-to-right (i.e. 'c','a','f','f').
    // Therefore stored value (big-endian) is the chars in original order.
    u32::from_be_bytes(*s)
}

struct CafBuilder {
    sample_rate: f64,
    frames_per_packet: u32,
    channels_per_frame: u32,
    packet_count: i64,
    frame_count: i64,
    n_blocks: usize,
    desc_size: i64,
    pakt_size: i64,
    data_size: i64,
    type_str: [u8; 4],
    version: u16,
    desc_format_id: [u8; 4],
}

impl Default for CafBuilder {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            frames_per_packet: 64,
            channels_per_frame: 1,
            packet_count: 10,
            frame_count: 640,
            n_blocks: 1,
            // typically: desc_size = sizeof(audio_description) = 32, but the parser only reads
            // chunk_size to know how to skip; for desc and pakt we set their actual struct sizes
            // because the parser advances `chunk += sizeof(chunk_header) + chunk_size`.
            desc_size: DESC as i64,
            pakt_size: PAKT as i64,
            data_size: 0, // computed
            type_str: *b"caff",
            version: 1,
            desc_format_id: *b"ima4",
        }
    }
}

fn build_caf(b: &CafBuilder) -> Vec<u8> {
    let data_chunk_body = CAFDATA + b.n_blocks * BLOCK;
    let total = HEADER
        + CHUNK + DESC
        + CHUNK + PAKT
        + CHUNK + data_chunk_body;
    let mut buf = vec![0u8; total];

    // Header
    let mut off = 0usize;
    buf[off..off + 4].copy_from_slice(&fourcc_be(&b.type_str).to_be_bytes());
    write_u16_be(&mut buf, off + 4, b.version);
    write_u16_be(&mut buf, off + 6, 0); // flags
    off += HEADER;

    // desc chunk
    buf[off..off + 4].copy_from_slice(&fourcc_be(b"desc").to_be_bytes());
    // 4 bytes padding
    write_i64_be(&mut buf, off + 8, b.desc_size);
    off += CHUNK;
    // audio description body
    write_f64_be(&mut buf, off, b.sample_rate);
    buf[off + 8..off + 12].copy_from_slice(&fourcc_be(&b.desc_format_id).to_be_bytes());
    write_u32_be(&mut buf, off + 12, 0); // format_flags
    write_u32_be(&mut buf, off + 16, 0); // bytes_per_packet
    write_u32_be(&mut buf, off + 20, b.frames_per_packet);
    write_u32_be(&mut buf, off + 24, b.channels_per_frame);
    write_u32_be(&mut buf, off + 28, 16); // bits_per_channel
    off += DESC;

    // pakt chunk
    buf[off..off + 4].copy_from_slice(&fourcc_be(b"pakt").to_be_bytes());
    write_i64_be(&mut buf, off + 8, b.pakt_size);
    off += CHUNK;
    write_i64_be(&mut buf, off, b.packet_count);
    write_i64_be(&mut buf, off + 8, b.frame_count);
    write_i32_be(&mut buf, off + 16, 0);
    write_i32_be(&mut buf, off + 20, 0);
    off += PAKT;

    // data chunk
    buf[off..off + 4].copy_from_slice(&fourcc_be(b"data").to_be_bytes());
    let actual_data_size = if b.data_size == 0 { data_chunk_body as i64 } else { b.data_size };
    write_i64_be(&mut buf, off + 8, actual_data_size);
    off += CHUNK;
    // caf_data: edit_count
    write_u32_be(&mut buf, off, 0);
    off += CAFDATA;
    // ima_block(s)
    for i in 0..b.n_blocks {
        let preamble = (0x1234u16).wrapping_add(i as u16);
        write_u16_be(&mut buf, off, preamble);
        for j in 0..32 {
            buf[off + 2 + j] = ((i * 32 + j) & 0xFF) as u8;
        }
        off += BLOCK;
    }
    assert_eq!(off, total);
    buf
}

fn run_both(buf: &[u8]) -> ((c_int, ImaInfo), (c_int, ImaInfo)) {
    unsafe {
        let (_c_lib, c_fn) = load(c_lib_path());
        let (_r_lib, r_fn) = load(rust_lib_path());

        // Make sure buffer is properly aligned. Re-allocate into an aligned vec.
        let mut aligned: Vec<u64> = vec![0u64; (buf.len() + 7) / 8];
        let dst = aligned.as_mut_ptr() as *mut u8;
        std::ptr::copy_nonoverlapping(buf.as_ptr(), dst, buf.len());

        let mut c_info = ImaInfo {
            blocks: std::ptr::null(),
            size: 0,
            sample_rate: 0.0,
            frame_count: 0,
            channel_count: 0,
        };
        let mut r_info = c_info;
        let c_rc = c_fn(&mut c_info as *mut ImaInfo, dst as *const c_void);
        let r_rc = r_fn(&mut r_info as *mut ImaInfo, dst as *const c_void);
        ((c_rc, c_info), (r_rc, r_info))
    }
}

fn assert_info_match(c: &(c_int, ImaInfo), r: &(c_int, ImaInfo)) {
    assert_eq!(c.0, r.0, "return code mismatch (C={}, Rust={})", c.0, r.0);
    if c.0 != 0 {
        return;
    }
    let ci = &c.1;
    let ri = &r.1;
    assert_eq!(ci.size, ri.size, "size mismatch");
    assert_eq!(ci.frame_count, ri.frame_count, "frame_count mismatch");
    assert_eq!(ci.channel_count, ri.channel_count, "channel_count mismatch");
    // sample_rate is a (probably garbage) f64; compare bit pattern.
    assert_eq!(
        ci.sample_rate.to_bits(),
        ri.sample_rate.to_bits(),
        "sample_rate bit pattern mismatch (C=0x{:016x}, Rust=0x{:016x})",
        ci.sample_rate.to_bits(),
        ri.sample_rate.to_bits()
    );
    // blocks is a pointer into the input buffer. Both should point to the same offset.
    // We can't directly compare ImaBlocks because both pointers come from different
    // calls with the same input pointer; they should be the exact same address.
    assert_eq!(
        ci.blocks as usize, ri.blocks as usize,
        "blocks pointer mismatch"
    );
}

#[test]
fn test_default_caf() {
    let b = CafBuilder::default();
    let buf = build_caf(&b);
    let (c_res, r_res) = run_both(&buf);
    assert_info_match(&c_res, &r_res);
    assert_eq!(c_res.0, 0, "default should parse OK");
}

#[test]
fn test_bad_magic() {
    let mut b = CafBuilder::default();
    b.type_str = *b"xaff";
    let buf = build_caf(&b);
    let (c_res, r_res) = run_both(&buf);
    assert_info_match(&c_res, &r_res);
    assert_eq!(c_res.0, -1);
}

#[test]
fn test_bad_version() {
    let mut b = CafBuilder::default();
    b.version = 2;
    let buf = build_caf(&b);
    let (c_res, r_res) = run_both(&buf);
    assert_info_match(&c_res, &r_res);
    assert_eq!(c_res.0, -2);
}

#[test]
fn test_bad_format_id() {
    let mut b = CafBuilder::default();
    b.desc_format_id = *b"xxxx";
    let buf = build_caf(&b);
    let (c_res, r_res) = run_both(&buf);
    assert_info_match(&c_res, &r_res);
    assert_eq!(c_res.0, -3);
}

#[test]
fn test_various_sample_rates() {
    for rate in &[8000.0f64, 11025.0, 22050.0, 44100.0, 48000.0, 96000.0, 0.5, 1.0e10, 0.0] {
        let mut b = CafBuilder::default();
        b.sample_rate = *rate;
        let buf = build_caf(&b);
        let (c_res, r_res) = run_both(&buf);
        assert_info_match(&c_res, &r_res);
    }
}

#[test]
fn test_various_channel_counts() {
    for &chan in &[1u32, 2, 4, 8] {
        let mut b = CafBuilder::default();
        b.channels_per_frame = chan;
        let buf = build_caf(&b);
        let (c_res, r_res) = run_both(&buf);
        assert_info_match(&c_res, &r_res);
        assert_eq!(c_res.1.channel_count, chan);
    }
}

#[test]
fn test_various_frame_counts() {
    for &fc in &[0i64, 1, 64, 1024, 1_000_000, i64::MAX / 2] {
        let mut b = CafBuilder::default();
        b.frame_count = fc;
        let buf = build_caf(&b);
        let (c_res, r_res) = run_both(&buf);
        assert_info_match(&c_res, &r_res);
        assert_eq!(c_res.1.frame_count, fc as u64);
    }
}

#[test]
fn test_multiple_blocks() {
    for &n in &[1usize, 2, 5, 10, 100] {
        let mut b = CafBuilder::default();
        b.n_blocks = n;
        let buf = build_caf(&b);
        let (c_res, r_res) = run_both(&buf);
        assert_info_match(&c_res, &r_res);
    }
}
