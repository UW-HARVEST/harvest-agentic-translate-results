// Translation of c_src/src/lib.c
//
// Reproduces the C library `ima_parse` function exactly. This Rust port uses
// raw pointers to mirror the C code's pointer arithmetic so that the parsed
// `ImaInfo` blocks pointer behaves identically to the C version.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::os::raw::c_void;

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;
pub type ima_u8_t = u8;
pub type ima_u16_t = u16;
pub type ima_s32_t = i32;
pub type ima_s64_t = i64;

pub const IMA_BLOCK_DATA_LEN: usize = 32;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ImaBlock {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; IMA_BLOCK_DATA_LEN],
}

#[repr(C)]
pub struct ImaInfo {
    pub blocks: *const ImaBlock,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CafHeader {
    type_: ima_u32_t,
    version: ima_u16_t,
    flags: ima_u16_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CafChunk {
    type_: ima_u32_t,
    size: ima_s64_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CafAudioDescription {
    sample_rate: ima_f64_t,
    format_id: ima_u32_t,
    format_flags: ima_u32_t,
    bytes_per_packet: ima_u32_t,
    frames_per_packet: ima_u32_t,
    channels_per_frame: ima_u32_t,
    bits_per_channel: ima_u32_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CafPacketTable {
    packet_count: ima_s64_t,
    frame_count: ima_s64_t,
    priming_frames: ima_s32_t,
    remainder_frames: ima_s32_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CafData {
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
fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t)
        | ((b as ima_u32_t) << 8)
        | ((c as ima_u32_t) << 16)
        | ((d as ima_u32_t) << 24)
}

/// Safety: `data` must point to a valid CAF buffer with the same layout
/// expectations as the original C function. This mirrors the C signature
/// and is intended for parity with the C implementation.
pub unsafe fn ima_parse(info: &mut ImaInfo, data: *const c_void) -> i32 {
    let header = data as *const CafHeader;
    let mut chunk = header.add(1) as *const CafChunk;
    let mut desc: *const CafAudioDescription = std::ptr::null();
    let mut pakt: *const CafPacketTable = std::ptr::null();
    let blocks: *const ImaBlock;

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
            desc = chunk.add(1) as *const CafAudioDescription;
        } else if chunk_type == fourcc(b't', b'k', b'a', b'p') {
            pakt = chunk.add(1) as *const CafPacketTable;
        } else if chunk_type == fourcc(b'a', b't', b'a', b'd') {
            // blocks = (const struct ima_block *)&((const struct caf_data *)&chunk[1])[1];
            let data_chunk = chunk.add(1) as *const CafData;
            blocks = data_chunk.add(1) as *const ImaBlock;
            break;
        }

        // chunk = (const struct caf_chunk *)((const ima_u8_t *)&chunk[1] + chunk_size);
        let after_chunk = chunk.add(1) as *const ima_u8_t;
        chunk = after_chunk.offset(chunk_size as isize) as *const CafChunk;
    }

    if ima_btoh32((*desc).format_id) != fourcc(b'4', b'a', b'm', b'i') {
        return -3;
    }

    info.blocks = blocks;
    info.size = chunk_size as ima_u64_t;
    info.frame_count = ima_btoh64((*pakt).frame_count as ima_u64_t);
    info.channel_count = ima_btoh32((*desc).channels_per_frame);

    // union conversion:
    //   conv64.u = desc->sample_rate;            // reinterpret f64 bits as u64
    //   conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);
    //   info->sample_rate = conv64.f;
    let sr_bits_le = (*desc).sample_rate.to_bits();
    let sr_bits = ima_btoh64(sr_bits_le);
    info.sample_rate = f64::from_bits(sr_bits);

    0
}
