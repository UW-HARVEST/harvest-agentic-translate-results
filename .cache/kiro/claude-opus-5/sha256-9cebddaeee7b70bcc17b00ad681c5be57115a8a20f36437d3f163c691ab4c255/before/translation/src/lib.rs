//! Rust translation of `c_src/src/lib.c` — a minimal CAF (Core Audio Format)
//! header parser for IMA4 streams.
//!
//! This is a faithful, bug-for-bug translation. In particular:
//!
//! * `struct caf_chunk` is **padded**: `type` sits at offset 0 and `size` at
//!   offset 8, giving `sizeof == 16`. The original C therefore walks chunks
//!   using a 16-byte header (and reads the 64-bit size from offset 8) even
//!   though the real CAF on-disk chunk header is 12 bytes. Reproduced as-is.
//! * `ima_btoh*` are unconditional byte swaps, i.e. the C assumes a
//!   little-endian host. Reproduced as-is.
//! * The sample rate is passed through a `union { double f; uint64_t u; }`
//!   but is written via `conv64.u = desc->sample_rate`, which is a *numeric*
//!   `double` -> `uint64_t` conversion, not a bit reinterpretation. The
//!   truncated integer is then byte-swapped and read back as a `double`.
//!   Reproduced as-is, including x86-64 `cvttsd2si` out-of-range semantics.
//! * `desc` / `pakt` may still be null after the chunk loop; the C
//!   dereferences them unconditionally. Reproduced as-is.

#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};
use core::mem::{offset_of, size_of};
use core::ptr::{self, addr_of, addr_of_mut};

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;

#[repr(C)]
pub struct ima_block {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; 32],
}

#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}

// ---------------------------------------------------------------------------
// src/lib.c local types
// ---------------------------------------------------------------------------

pub type ima_s32_t = i32;
pub type ima_s64_t = i64;

#[repr(C)]
struct caf_header {
    type_: ima_u32_t,
    version: ima_u16_t,
    #[allow(dead_code)]
    flags: ima_u16_t,
}

#[repr(C)]
struct caf_chunk {
    type_: ima_u32_t,
    size: ima_s64_t,
}

#[repr(C)]
struct caf_audio_description {
    sample_rate: ima_f64_t,
    format_id: ima_u32_t,
    #[allow(dead_code)]
    format_flags: ima_u32_t,
    #[allow(dead_code)]
    bytes_per_packet: ima_u32_t,
    #[allow(dead_code)]
    frames_per_packet: ima_u32_t,
    channels_per_frame: ima_u32_t,
    #[allow(dead_code)]
    bits_per_channel: ima_u32_t,
}

#[repr(C)]
struct caf_packet_table {
    #[allow(dead_code)]
    packet_count: ima_s64_t,
    frame_count: ima_s64_t,
    #[allow(dead_code)]
    priming_frames: ima_s32_t,
    #[allow(dead_code)]
    remainder_frames: ima_s32_t,
}

#[repr(C)]
struct caf_data {
    #[allow(dead_code)]
    edit_count: ima_u32_t,
}

// Layout assertions: these must match the C ABI exactly, because every read
// below is a raw reinterpretation of the caller's buffer.
const _: () = assert!(size_of::<caf_header>() == 8);
const _: () = assert!(offset_of!(caf_header, version) == 4);
const _: () = assert!(size_of::<caf_chunk>() == 16);
const _: () = assert!(offset_of!(caf_chunk, size) == 8);
const _: () = assert!(size_of::<caf_audio_description>() == 32);
const _: () = assert!(offset_of!(caf_audio_description, format_id) == 8);
const _: () = assert!(offset_of!(caf_audio_description, channels_per_frame) == 24);
const _: () = assert!(size_of::<caf_packet_table>() == 24);
const _: () = assert!(offset_of!(caf_packet_table, frame_count) == 8);
const _: () = assert!(size_of::<caf_data>() == 4);
const _: () = assert!(size_of::<ima_block>() == 34);
const _: () = assert!(size_of::<ima_info>() == 40);
const _: () = assert!(offset_of!(ima_info, size) == 8);
const _: () = assert!(offset_of!(ima_info, sample_rate) == 16);
const _: () = assert!(offset_of!(ima_info, frame_count) == 24);
const _: () = assert!(offset_of!(ima_info, channel_count) == 32);

// ---------------------------------------------------------------------------
// Byte swapping (written to mirror the C expressions)
// ---------------------------------------------------------------------------

fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    (v << 0x08 & 0xff00u16) | (v >> 0x08 & 0x00ffu16)
}

fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    (v << 0x18 & 0xff000000u32)
        | (v << 0x08 & 0x00ff0000u32)
        | (v >> 0x08 & 0x0000ff00u32)
        | (v >> 0x18 & 0x000000ffu32)
}

fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    (v << 0x38 & 0xff00000000000000u64)
        | (v << 0x28 & 0x00ff000000000000u64)
        | (v << 0x18 & 0x0000ff0000000000u64)
        | (v << 0x08 & 0x000000ff00000000u64)
        | (v >> 0x08 & 0x00000000ff000000u64)
        | (v >> 0x18 & 0x0000000000ff0000u64)
        | (v >> 0x28 & 0x000000000000ff00u64)
        | (v >> 0x38 & 0x00000000000000ffu64)
}

fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    ima_bswap32(v)
}

fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    ima_bswap64(v)
}

// ---------------------------------------------------------------------------
// FourCC helpers
// ---------------------------------------------------------------------------

/// `(u32)(u8)a | ((u32)(u8)b << 8) | ((u32)(u8)c << 16) | ((u32)(u8)d << 24)`
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t) | ((b as ima_u32_t) << 8) | ((c as ima_u32_t) << 16) | ((d as ima_u32_t) << 24)
}

const CAF_FILE_TYPE: ima_u32_t = fourcc(b'f', b'f', b'a', b'c');
const CAF_CHUNK_DESC: ima_u32_t = fourcc(b'c', b's', b'e', b'd');
const CAF_CHUNK_PAKT: ima_u32_t = fourcc(b't', b'k', b'a', b'p');
const CAF_CHUNK_DATA: ima_u32_t = fourcc(b'a', b't', b'a', b'd');
const CAF_FORMAT_IMA4: ima_u32_t = fourcc(b'4', b'a', b'm', b'i');

// ---------------------------------------------------------------------------
// double -> uint64_t conversion, matching x86-64 codegen
// ---------------------------------------------------------------------------

const TWO_POW_63: f64 = 9223372036854775808.0;

/// Emulates the x86-64 `cvttsd2si` instruction: truncate toward zero, yielding
/// the "integer indefinite" value `i64::MIN` when the source is NaN or the
/// truncated result does not fit in a signed 64-bit integer.
fn cvttsd2si(v: f64) -> i64 {
    if v.is_nan() {
        return i64::MIN;
    }
    let t = v.trunc();
    if t >= -TWO_POW_63 && t < TWO_POW_63 {
        // In range, so this cast is exact (no saturation takes effect).
        t as i64
    } else {
        i64::MIN
    }
}

/// Emulates the `double` -> `unsigned long long` conversion gcc emits on
/// x86-64: compare against 2^63, and for values at or above it subtract 2^63,
/// truncate, then flip the sign bit. NaN fails the `>=` comparison and so
/// takes the plain `cvttsd2si` path, exactly as the generated `jnb` does.
fn f64_to_u64(v: f64) -> ima_u64_t {
    if v >= TWO_POW_63 {
        (cvttsd2si(v - TWO_POW_63) as ima_u64_t) ^ (1u64 << 63)
    } else {
        cvttsd2si(v) as ima_u64_t
    }
}

// ---------------------------------------------------------------------------
// ima_parse
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    unsafe {
        let header = data as *const caf_header;
        let mut chunk = header.wrapping_add(1) as *const caf_chunk;
        let mut desc: *const caf_audio_description = ptr::null();
        let mut pakt: *const caf_packet_table = ptr::null();
        let blocks: *const ima_block;
        let chunk_size: ima_s64_t;

        if ima_btoh32(ptr::read_unaligned(addr_of!((*header).type_))) != CAF_FILE_TYPE {
            return -1;
        }
        if ima_btoh16(ptr::read_unaligned(addr_of!((*header).version))) != 1 {
            return -2;
        }

        loop {
            let chunk_type = ima_btoh32(ptr::read_unaligned(addr_of!((*chunk).type_)));
            let size = ima_btoh64(ptr::read_unaligned(addr_of!((*chunk).size)) as ima_u64_t)
                as ima_s64_t;

            if chunk_type == CAF_CHUNK_DESC {
                desc = chunk.wrapping_add(1) as *const caf_audio_description;
            } else if chunk_type == CAF_CHUNK_PAKT {
                pakt = chunk.wrapping_add(1) as *const caf_packet_table;
            } else if chunk_type == CAF_CHUNK_DATA {
                // &((const struct caf_data *)&chunk[1])[1]
                blocks = (chunk.wrapping_add(1) as *const caf_data).wrapping_add(1)
                    as *const ima_block;
                chunk_size = size;
                break;
            }

            // chunk = (const struct caf_chunk *)((const ima_u8_t *)&chunk[1] + chunk_size);
            chunk = (chunk.wrapping_add(1) as *const ima_u8_t).wrapping_offset(size as isize)
                as *const caf_chunk;
        }

        if ima_btoh32(ptr::read_unaligned(addr_of!((*desc).format_id))) != CAF_FORMAT_IMA4 {
            return -3;
        }

        ptr::write_unaligned(addr_of_mut!((*info).blocks), blocks);
        ptr::write_unaligned(addr_of_mut!((*info).size), chunk_size as ima_u64_t);
        ptr::write_unaligned(
            addr_of_mut!((*info).frame_count),
            ima_btoh64(ptr::read_unaligned(addr_of!((*pakt).frame_count)) as ima_u64_t),
        );
        ptr::write_unaligned(
            addr_of_mut!((*info).channel_count),
            ima_btoh32(ptr::read_unaligned(addr_of!((*desc).channels_per_frame))),
        );

        // conv64.u = desc->sample_rate;                          (numeric conversion)
        // conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);  (byte swap)
        // info->sample_rate = conv64.f;                          (bit reinterpretation)
        let raw = ptr::read_unaligned(addr_of!((*desc).sample_rate));
        let converted = f64_to_u64(raw);
        let swapped = ima_btoh64(converted);
        ptr::write_unaligned(addr_of_mut!((*info).sample_rate), f64::from_bits(swapped));

        0
    }
}
