#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};
use std::ptr;

pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;
pub type ima_u8_t = u8;
pub type ima_u16_t = u16;

type ima_s32_t = i32;
type ima_s64_t = i64;

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
#[derive(Copy, Clone)]
struct caf_header {
    type_: ima_u32_t,
    version: ima_u16_t,
    flags: ima_u16_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct caf_chunk {
    type_: ima_u32_t,
    size: ima_s64_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
struct caf_packet_table {
    packet_count: ima_s64_t,
    frame_count: ima_s64_t,
    priming_frames: ima_s32_t,
    remainder_frames: ima_s32_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct caf_data {
    edit_count: ima_u32_t,
}

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ima_u32_t {
    (a as ima_u32_t)
        | ((b as ima_u32_t) << 8)
        | ((c as ima_u32_t) << 16)
        | ((d as ima_u32_t) << 24)
}

fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    ((v << 0x08) & 0xff00u16) | ((v >> 0x08) & 0x00ffu16)
}

fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    ((v << 0x18) & 0xff00_0000u32)
        | ((v << 0x08) & 0x00ff_0000u32)
        | ((v >> 0x08) & 0x0000_ff00u32)
        | ((v >> 0x18) & 0x0000_00ffu32)
}

fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    ((v << 0x38) & 0xff00_0000_0000_0000u64)
        | ((v << 0x28) & 0x00ff_0000_0000_0000u64)
        | ((v << 0x18) & 0x0000_ff00_0000_0000u64)
        | ((v << 0x08) & 0x0000_00ff_0000_0000u64)
        | ((v >> 0x08) & 0x0000_0000_ff00_0000u64)
        | ((v >> 0x18) & 0x0000_0000_00ff_0000u64)
        | ((v >> 0x28) & 0x0000_0000_0000_ff00u64)
        | ((v >> 0x38) & 0x0000_0000_0000_00ffu64)
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header = data as *const caf_header;
    let mut chunk = unsafe { header.add(1) as *const caf_chunk };
    let mut desc: *const caf_audio_description = ptr::null();
    let mut pakt: *const caf_packet_table = ptr::null();
    let blocks: *const ima_block;
    let chunk_size: ima_s64_t;

    let header_value = unsafe { ptr::read_unaligned(header) };
    if ima_btoh32(header_value.type_) != fourcc(b'f', b'f', b'a', b'c') {
        return -1;
    }
    if ima_btoh16(header_value.version) != 1 {
        return -2;
    }

    loop {
        let chunk_value = unsafe { ptr::read_unaligned(chunk) };
        let current_chunk_type = ima_btoh32(chunk_value.type_);
        let current_chunk_size = ima_btoh64(chunk_value.size as ima_u64_t) as ima_s64_t;

        if current_chunk_type == fourcc(b'c', b's', b'e', b'd') {
            desc = unsafe { chunk.add(1) as *const caf_audio_description };
        } else if current_chunk_type == fourcc(b't', b'k', b'a', b'p') {
            pakt = unsafe { chunk.add(1) as *const caf_packet_table };
        } else if current_chunk_type == fourcc(b'a', b't', b'a', b'd') {
            blocks = unsafe { (chunk.add(1) as *const caf_data).add(1) as *const ima_block };
            chunk_size = current_chunk_size;
            break;
        }

        chunk = unsafe { ((chunk.add(1) as *const ima_u8_t).offset(current_chunk_size as isize)) as *const caf_chunk };
    }

    let desc_value = unsafe { ptr::read_unaligned(desc) };
    if ima_btoh32(desc_value.format_id) != fourcc(b'4', b'a', b'm', b'i') {
        return -3;
    }

    let pakt_value = unsafe { ptr::read_unaligned(pakt) };
    unsafe {
        (*info).blocks = blocks;
        (*info).size = chunk_size as ima_u64_t;
        (*info).frame_count = ima_btoh64(pakt_value.frame_count as ima_u64_t);
        (*info).channel_count = ima_btoh32(desc_value.channels_per_frame);
        (*info).sample_rate = ima_f64_t::from_bits(ima_btoh64(desc_value.sample_rate.to_bits()));
    }
    0
}
