//! Phase C: error-path differential tests.
//!
//! We build a valid baseline PNG (via the roundtrip harness), then mutate its
//! chunk structure in Rust to construct each invalid condition from ERRORS.md.
//! The mutated stream is decoded by BOTH the C `libpng.so` and the Rust
//! `liblibpng.so` through `harness_decode_raw`, and we assert both react
//! identically (same fired/not-fired AND same error message).

mod common;
use common::{c_so_path, crate_root, rust_so_path};

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;

#[allow(non_camel_case_types)]
type HarnessRoundtrip = unsafe extern "C" fn(
    *const c_char, c_uint, c_uint, c_int, c_int, c_int, c_int, c_int, c_int,
    *const u8, usize, *const u8, c_int,
    *mut *mut u8, *mut usize, *mut u8, usize,
    *mut c_uint, *mut c_uint, *mut c_int, *mut c_int, *mut c_int, *mut usize,
) -> c_int;

#[allow(non_camel_case_types)]
type HarnessDecodeRaw = unsafe extern "C" fn(
    *const c_char,
    *const u8,
    usize,
    *mut c_char,
    usize,
    *mut c_uint,
    *mut c_uint,
    *mut c_int,
    *mut c_int,
) -> c_int;

fn harness() -> Library {
    let p = crate_root().join("tests/libharness.so");
    unsafe { Library::new(&p).unwrap_or_else(|e| panic!("load harness {:?}: {e}", p)) }
}

unsafe fn libc_free(p: *mut u8) {
    extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    free(p as *mut _);
}

/// Build a valid baseline PNG of the given shape using the C library.
fn make_valid_png(width: u32, height: u32, bit_depth: c_int, color_type: c_int) -> Vec<u8> {
    let h = harness();
    let rt: Symbol<HarnessRoundtrip> = unsafe { h.get(b"harness_roundtrip").unwrap() };
    let ch = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    };
    let rb = (width as usize * ch * bit_depth as usize + 7) / 8;
    let src = vec![0x5au8; rb * height as usize];
    let path = CString::new(c_so_path().to_str().unwrap()).unwrap();
    let mut enc_out: *mut u8 = ptr::null_mut();
    let mut enc_len = 0usize;
    let mut dec = vec![0u8; (rb + 16) * height as usize + 16];
    let (mut dw, mut dh, mut dbd, mut dct, mut dil, mut drb) = (0, 0, 0, 0, 0, 0usize);
    let ret = unsafe {
        rt(
            path.as_ptr(), width, height, bit_depth, color_type, 0, -1, 6, 0,
            src.as_ptr(), rb, ptr::null(), 0,
            &mut enc_out, &mut enc_len, dec.as_mut_ptr(), dec.len(),
            &mut dw, &mut dh, &mut dbd, &mut dct, &mut dil, &mut drb,
        )
    };
    assert_eq!(ret, 0, "baseline PNG generation failed");
    let v = unsafe { std::slice::from_raw_parts(enc_out, enc_len).to_vec() };
    unsafe { libc_free(enc_out) };
    v
}

/// Decode `stream` with the given library path; return (fired, message).
fn decode_with(lib_path: &str, stream: &[u8]) -> (c_int, String) {
    let h = harness();
    let f: Symbol<HarnessDecodeRaw> = unsafe { h.get(b"harness_decode_raw").unwrap() };
    let path = CString::new(lib_path).unwrap();
    let mut msg = vec![0i8; 256];
    let (mut w, mut ht, mut bd, mut ct) = (0u32, 0u32, 0i32, 0i32);
    let fired = unsafe {
        f(
            path.as_ptr(),
            stream.as_ptr(),
            stream.len(),
            msg.as_mut_ptr(),
            msg.len(),
            &mut w,
            &mut ht,
            &mut bd,
            &mut ct,
        )
    };
    let s = unsafe { std::ffi::CStr::from_ptr(msg.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    (fired, s)
}

/// Assert both libraries react identically to `stream`.
fn assert_same_reaction(label: &str, stream: &[u8]) {
    let (cf, cm) = decode_with(c_so_path().to_str().unwrap(), stream);
    let (rf, rm) = decode_with(rust_so_path().to_str().unwrap(), stream);
    assert_eq!(cf, rf, "[{label}] fired differs: C={cf} ('{cm}') Rust={rf} ('{rm}')");
    assert_eq!(cm, rm, "[{label}] message differs: C='{cm}' Rust='{rm}'");
}

// ---- CRC helper (matches zlib crc32 used by libpng) ----
fn crc32(bytes: &[u8]) -> u32 {
    // zlib CRC-32 (IEEE 802.3), same polynomial libpng uses.
    let mut crc: u32 = 0xffff_ffff;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Parse a PNG into (signature, Vec<(type[4], data)>).
fn parse_chunks(png: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 8; // skip signature
    while i + 8 <= png.len() {
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let ty = [png[i + 4], png[i + 5], png[i + 6], png[i + 7]];
        let data_start = i + 8;
        let data_end = data_start + len;
        if data_end + 4 > png.len() {
            break;
        }
        let data = png[data_start..data_end].to_vec();
        out.push((ty, data));
        i = data_end + 4; // skip CRC
    }
    out
}

/// Rebuild a PNG stream from a chunk list, recomputing each CRC.
fn build_png(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&SIG);
    for (ty, data) in chunks {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(ty);
        out.extend_from_slice(data);
        let mut crc_input = Vec::with_capacity(4 + data.len());
        crc_input.extend_from_slice(ty);
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }
    out
}

// Sanity: our CRC + rebuild reproduce a valid, decodable PNG identically.
#[test]
fn baseline_rebuild_is_valid() {
    let png = make_valid_png(8, 6, 8, 2);
    let chunks = parse_chunks(&png);
    assert!(chunks.iter().any(|(t, _)| t == b"IHDR"));
    let rebuilt = build_png(&chunks);
    // Rebuilt stream should decode with NO error on both libs.
    assert_same_reaction("baseline rebuild", &rebuilt);
    let (fired, _) = decode_with(c_so_path().to_str().unwrap(), &rebuilt);
    assert_eq!(fired, 0, "rebuilt baseline should decode cleanly");
}

// ERRORS row 14 / 19: bad signature.
#[test]
fn bad_signature() {
    let mut png = make_valid_png(8, 6, 8, 2);
    png[0] = 0x00; // corrupt first signature byte
    assert_same_reaction("bad signature", &png);
}

// ERRORS row 1 / 19: first chunk not IHDR (drop IHDR, put another first).
#[test]
fn missing_ihdr_first() {
    let png = make_valid_png(8, 6, 8, 2);
    let chunks = parse_chunks(&png);
    // Reorder: move a non-IHDR chunk to the front by dropping IHDR entirely.
    let without_ihdr: Vec<_> = chunks
        .iter()
        .filter(|(t, _)| t != b"IHDR")
        .cloned()
        .collect();
    let stream = build_png(&without_ihdr);
    assert_same_reaction("missing IHDR", &stream);
}

// ERRORS row 20 / 3: duplicate IHDR (critical, multiple==0) -> critical error.
#[test]
fn duplicate_ihdr() {
    let png = make_valid_png(8, 6, 8, 2);
    let chunks = parse_chunks(&png);
    let ihdr = chunks.iter().find(|(t, _)| t == b"IHDR").unwrap().clone();
    let mut new_chunks = vec![ihdr.clone(), ihdr.clone()];
    for c in &chunks {
        if c.0 != *b"IHDR" {
            new_chunks.push(c.clone());
        }
    }
    let stream = build_png(&new_chunks);
    assert_same_reaction("duplicate IHDR", &stream);
}

// ERRORS row 2: known ancillary chunk out of place (gAMA after IDAT).
#[test]
fn ancillary_out_of_place() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    // Insert a gAMA chunk (4 bytes) AFTER IDAT (invalid: gAMA must be before).
    let gama = (*b"gAMA", 100000u32.to_be_bytes().to_vec());
    // find IEND position, insert gAMA right before it (after IDAT)
    let iend_pos = chunks.iter().position(|(t, _)| t == b"IEND").unwrap();
    chunks.insert(iend_pos, gama);
    let stream = build_png(&chunks);
    assert_same_reaction("gAMA out of place", &stream);
}

// ERRORS row 3: duplicate ancillary non-multiple chunk (two gAMA before IDAT).
#[test]
fn duplicate_ancillary() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    let gama = (*b"gAMA", 100000u32.to_be_bytes().to_vec());
    let idat_pos = chunks.iter().position(|(t, _)| t == b"IDAT").unwrap();
    chunks.insert(idat_pos, gama.clone());
    chunks.insert(idat_pos, gama);
    let stream = build_png(&chunks);
    assert_same_reaction("duplicate gAMA", &stream);
}

// ERRORS row 4: known chunk too short (gAMA with <4 bytes).
#[test]
fn chunk_too_short() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    let gama = (*b"gAMA", vec![0u8, 0u8]); // only 2 bytes, min is 4
    let idat_pos = chunks.iter().position(|(t, _)| t == b"IDAT").unwrap();
    chunks.insert(idat_pos, gama);
    let stream = build_png(&chunks);
    assert_same_reaction("gAMA too short", &stream);
}

// ERRORS row 5: known chunk too long (gAMA with >4 bytes).
#[test]
fn chunk_too_long() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    let gama = (*b"gAMA", vec![0u8; 8]); // 8 bytes, max is 4
    let idat_pos = chunks.iter().position(|(t, _)| t == b"IDAT").unwrap();
    chunks.insert(idat_pos, gama);
    let stream = build_png(&chunks);
    assert_same_reaction("gAMA too long", &stream);
}

// ERRORS row 8 (via handle_unknown default): unknown CRITICAL chunk -> error.
#[test]
fn unknown_critical_chunk() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    // "ABCD" - uppercase first letter => critical, unknown.
    let bogus = (*b"ABCD", vec![1u8, 2, 3]);
    let idat_pos = chunks.iter().position(|(t, _)| t == b"IDAT").unwrap();
    chunks.insert(idat_pos, bogus);
    let stream = build_png(&chunks);
    assert_same_reaction("unknown critical chunk", &stream);
}

// Unknown ANCILLARY chunk -> discarded silently (handled the same by both).
#[test]
fn unknown_ancillary_chunk() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut chunks = parse_chunks(&png);
    // "abCD" - lowercase first letter => ancillary, unknown; should be skipped.
    let bogus = (*b"teDx", vec![9u8, 8, 7, 6]);
    let idat_pos = chunks.iter().position(|(t, _)| t == b"IDAT").unwrap();
    chunks.insert(idat_pos, bogus);
    let stream = build_png(&chunks);
    assert_same_reaction("unknown ancillary chunk", &stream);
}

// ERRORS row 13: bad CRC on a chunk.
#[test]
fn bad_crc() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut stream = png.clone();
    // Corrupt the CRC of the IHDR chunk. IHDR: len(4)+type(4)+13 data +crc(4).
    // Signature 8 bytes; IHDR data starts at 16, ends at 29, CRC at 29..33.
    let crc_pos = 8 + 8 + 13;
    stream[crc_pos] ^= 0xff;
    assert_same_reaction("bad IHDR CRC", &stream);
}

// ERRORS row 12: chunk length exceeds 31-bit PNG limit.
#[test]
fn oversized_chunk_length() {
    let png = make_valid_png(8, 6, 8, 2);
    let mut stream = png.clone();
    // Set the first (IHDR) chunk length to 0x80000000 (> PNG_UINT_31_MAX).
    stream[8] = 0x80;
    stream[9] = 0x00;
    stream[10] = 0x00;
    stream[11] = 0x00;
    assert_same_reaction("oversized chunk length", &stream);
}

// Truncated stream (EOF mid-chunk).
#[test]
fn truncated_stream() {
    let png = make_valid_png(8, 6, 8, 2);
    let stream = png[..png.len().saturating_sub(20)].to_vec();
    assert_same_reaction("truncated", &stream);
}

// ERRORS row 11 + out-of-range enum across FFI: IHDR with an invalid
// color_type value (C enums accept any int; libpng's png_check_IHDR rejects it).
#[test]
fn invalid_color_type_in_ihdr() {
    for bad_ct in [1u8, 5, 7, 99, 255] {
        let png = make_valid_png(8, 6, 8, 2);
        let mut chunks = parse_chunks(&png);
        let ihdr = &mut chunks.iter_mut().find(|(t, _)| t == b"IHDR").unwrap().1;
        // IHDR data layout: width(4) height(4) bit_depth(1) color_type(1) ...
        ihdr[9] = bad_ct;
        let stream = build_png(&chunks);
        assert_same_reaction(&format!("invalid color_type {bad_ct}"), &stream);
    }
}

// Out-of-range bit_depth in IHDR (e.g. 3, 5, 7, 32).
#[test]
fn invalid_bit_depth_in_ihdr() {
    for bad_bd in [0u8, 3, 5, 6, 7, 9, 32, 255] {
        let png = make_valid_png(8, 6, 8, 2);
        let mut chunks = parse_chunks(&png);
        let ihdr = &mut chunks.iter_mut().find(|(t, _)| t == b"IHDR").unwrap().1;
        ihdr[8] = bad_bd; // bit_depth byte
        let stream = build_png(&chunks);
        assert_same_reaction(&format!("invalid bit_depth {bad_bd}"), &stream);
    }
}

// Zero width/height in IHDR.
#[test]
fn zero_dimensions_in_ihdr() {
    for which in ["width", "height"] {
        let png = make_valid_png(8, 6, 8, 2);
        let mut chunks = parse_chunks(&png);
        let ihdr = &mut chunks.iter_mut().find(|(t, _)| t == b"IHDR").unwrap().1;
        if which == "width" {
            ihdr[0..4].copy_from_slice(&0u32.to_be_bytes());
        } else {
            ihdr[4..8].copy_from_slice(&0u32.to_be_bytes());
        }
        let stream = build_png(&chunks);
        assert_same_reaction(&format!("zero {which}"), &stream);
    }
}

// Empty / tiny inputs.
#[test]
fn empty_and_tiny() {
    assert_same_reaction("empty", &[]);
    assert_same_reaction("one byte", &[137]);
    assert_same_reaction("sig only", &SIG);
}
