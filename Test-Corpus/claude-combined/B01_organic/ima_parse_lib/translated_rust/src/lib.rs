#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::os::raw::c_int;

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;

pub type ima_s32_t = i32;
pub type ima_s64_t = i64;

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

#[repr(C)]
struct caf_header {
    type_: ima_u32_t,
    version: ima_u16_t,
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
    format_flags: ima_u32_t,
    bytes_per_packet: ima_u32_t,
    frames_per_packet: ima_u32_t,
    channels_per_frame: ima_u32_t,
    bits_per_channel: ima_u32_t,
}

#[repr(C)]
struct caf_packet_table {
    packet_count: ima_s64_t,
    frame_count: ima_s64_t,
    priming_frames: ima_s32_t,
    remainder_frames: ima_s32_t,
}

#[repr(C)]
struct caf_data {
    edit_count: ima_u32_t,
}

#[inline]
fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    ((v << 0x08) & 0xff00u16) | ((v >> 0x08) & 0x00ffu16)
}

#[inline]
fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    ((v << 0x18) & 0xff000000u32)
        | ((v << 0x08) & 0x00ff0000u32)
        | ((v >> 0x08) & 0x0000ff00u32)
        | ((v >> 0x18) & 0x000000ffu32)
}

#[inline]
fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    ((v << 0x38) & 0xff00000000000000u64)
        | ((v << 0x28) & 0x00ff000000000000u64)
        | ((v << 0x18) & 0x0000ff0000000000u64)
        | ((v << 0x08) & 0x000000ff00000000u64)
        | ((v >> 0x08) & 0x00000000ff000000u64)
        | ((v >> 0x18) & 0x0000000000ff0000u64)
        | ((v >> 0x28) & 0x000000000000ff00u64)
        | ((v >> 0x38) & 0x00000000000000ffu64)
}

#[inline]
fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

#[inline]
fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    ima_bswap32(v)
}

#[inline]
fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    ima_bswap64(v)
}

#[inline]
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t)
        | ((b as ima_u32_t) << 8)
        | ((c as ima_u32_t) << 16)
        | ((d as ima_u32_t) << 24)
}

/// SAFETY: caller must provide a valid `info` pointer and a `data` pointer
/// pointing to enough bytes to satisfy the parse logic, mirroring the
/// requirements of the C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header = data as *const caf_header;
    let mut chunk = header.add(1) as *const caf_chunk;
    let mut desc: *const caf_audio_description = std::ptr::null();
    let mut pakt: *const caf_packet_table = std::ptr::null();
    let blocks: *const ima_block;
    let mut chunk_size: ima_s64_t;
    let mut chunk_type: ima_u32_t;

    if ima_btoh32((*header).type_) != fourcc(b'f', b'f', b'a', b'c') {
        return -1;
    }
    if ima_btoh16((*header).version) != 1 {
        return -2;
    }

    loop {
        chunk_type = ima_btoh32((*chunk).type_);
        chunk_size = ima_btoh64((*chunk).size as ima_u64_t) as ima_s64_t;
        if chunk_type == fourcc(b'c', b's', b'e', b'd') {
            desc = chunk.add(1) as *const caf_audio_description;
        } else if chunk_type == fourcc(b't', b'k', b'a', b'p') {
            pakt = chunk.add(1) as *const caf_packet_table;
        } else if chunk_type == fourcc(b'a', b't', b'a', b'd') {
            // blocks = (const struct ima_block *)&((const struct caf_data *)&chunk[1])[1];
            let cd = chunk.add(1) as *const caf_data;
            blocks = cd.add(1) as *const ima_block;
            break;
        }
        let after = (chunk.add(1) as *const ima_u8_t).offset(chunk_size as isize);
        chunk = after as *const caf_chunk;
    }

    if ima_btoh32((*desc).format_id) != fourcc(b'4', b'a', b'm', b'i') {
        return -3;
    }

    (*info).blocks = blocks;
    (*info).size = chunk_size as ima_u64_t;
    (*info).frame_count = ima_btoh64((*pakt).frame_count as ima_u64_t);
    (*info).channel_count = ima_btoh32((*desc).channels_per_frame);

    // Reproduce the C union-based conversion exactly:
    //   union { f64 f; u64 u; } conv64;
    //   conv64.u = desc->sample_rate;            // float-to-int conversion
    //   conv64.u = ima_btoh64(*(const u64*)&conv64.u); // byteswap (no-op semantically here)
    //   info->sample_rate = conv64.f;            // bit-level reinterpret
    let sr_f: ima_f64_t = (*desc).sample_rate;
    // float-to-int conversion matching C semantics on x86-64 (cvttsd2si style):
    //   in-range positive values: truncated value
    //   negative values, NaN, out-of-range: 0x8000000000000000 ("indefinite")
    let conv_u: ima_u64_t = f64_to_u64_c_style(sr_f);
    let conv_u_after_bswap: ima_u64_t = ima_btoh64(conv_u);
    let result_f: ima_f64_t = f64::from_bits(conv_u_after_bswap);
    (*info).sample_rate = result_f;

    0
}

/// Mimic the x86-64 cvttsd2si-style conversion of a double to a 64-bit unsigned
/// integer as performed by `(uint64_t)double_value` on most platforms when the
/// source is implicitly converted from f64 to u64. Out-of-range and NaN values
/// produce the "integer indeterminate" pattern (0x8000000000000000).
#[inline]
fn f64_to_u64_c_style(v: ima_f64_t) -> ima_u64_t {
    // Replicate behavior of x86-64 `cvttsd2si %xmm, %r64` which produces a
    // signed 64-bit result; bits are then reinterpreted as u64.
    if v.is_nan() {
        return 0x8000_0000_0000_0000u64;
    }
    // Range of representable signed 64-bit values is [-2^63, 2^63).
    // cvttsd2si returns 0x8000000000000000 for values outside that range.
    if v >= 9223372036854775808.0_f64 || v < -9223372036854775808.0_f64 {
        return 0x8000_0000_0000_0000u64;
    }
    // Truncation toward zero, then reinterpret signed bits as unsigned.
    (v as i64) as ima_u64_t
}
