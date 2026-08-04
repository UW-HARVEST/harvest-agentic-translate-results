#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::os::raw::c_int;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;
pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_s32_t = i32;
pub type ima_s64_t = i64;
pub type ima_f64_t = f64;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header = data as *const caf_header;
    let mut chunk = header.add(1) as *const caf_chunk;
    let mut desc: *const caf_audio_description = std::ptr::null();
    let mut pakt: *const caf_packet_table = std::ptr::null();
    let blocks: *const ima_block;
    let mut chunk_size: ima_s64_t;
    let mut chunk_type: u32;

    if ima_btoh32((*header).type_)
        != ((b'f' as ima_u32_t)
            | ((b'f' as ima_u32_t) << 8)
            | ((b'a' as ima_u32_t) << 16)
            | ((b'c' as ima_u32_t) << 24))
    {
        return -1;
    }
    if ima_btoh16((*header).version) != 1 {
        return -2;
    }
    loop {
        chunk_type = ima_btoh32((*chunk).type_);
        chunk_size = ima_btoh64((*chunk).size as ima_u64_t) as ima_s64_t;
        if chunk_type
            == ((b'c' as ima_u32_t)
                | ((b's' as ima_u32_t) << 8)
                | ((b'e' as ima_u32_t) << 16)
                | ((b'd' as ima_u32_t) << 24))
        {
            desc = chunk.add(1) as *const caf_audio_description;
        } else if chunk_type
            == ((b't' as ima_u32_t)
                | ((b'k' as ima_u32_t) << 8)
                | ((b'a' as ima_u32_t) << 16)
                | ((b'p' as ima_u32_t) << 24))
        {
            pakt = chunk.add(1) as *const caf_packet_table;
        } else if chunk_type
            == ((b'a' as ima_u32_t)
                | ((b't' as ima_u32_t) << 8)
                | ((b'a' as ima_u32_t) << 16)
                | ((b'd' as ima_u32_t) << 24))
        {
            // blocks = (const struct ima_block *)&((const struct caf_data *)&chunk[1])[1];
            let cdata = chunk.add(1) as *const caf_data;
            blocks = cdata.add(1) as *const ima_block;
            break;
        }
        // chunk = (const struct caf_chunk *)((const ima_u8_t *)&chunk[1] + chunk_size);
        let after_chunk = chunk.add(1) as *const ima_u8_t;
        chunk = after_chunk.offset(chunk_size as isize) as *const caf_chunk;
    }
    if ima_btoh32((*desc).format_id)
        != ((b'4' as ima_u32_t)
            | ((b'a' as ima_u32_t) << 8)
            | ((b'm' as ima_u32_t) << 16)
            | ((b'i' as ima_u32_t) << 24))
    {
        return -3;
    }
    (*info).blocks = blocks;
    (*info).size = chunk_size as ima_u64_t;
    (*info).frame_count = ima_btoh64((*pakt).frame_count as ima_u64_t);
    (*info).channel_count = ima_btoh32((*desc).channels_per_frame);
    // conv64 union: read sample_rate as u64, byte-swap, then bitcast to f64
    let raw_sample_rate: ima_u64_t = (*desc).sample_rate.to_bits();
    let swapped = ima_btoh64(raw_sample_rate);
    (*info).sample_rate = f64::from_bits(swapped);
    0
}
