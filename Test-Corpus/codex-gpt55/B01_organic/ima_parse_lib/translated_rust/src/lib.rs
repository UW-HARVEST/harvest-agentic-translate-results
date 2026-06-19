#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};
use std::ptr;

pub type ima_u8_t = u8;
pub type ima_u16_t = u16;
pub type ima_u32_t = u32;
pub type ima_u64_t = u64;
pub type ima_f64_t = f64;

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

const CAFF: ima_u32_t = (b'f' as ima_u32_t)
    | ((b'f' as ima_u32_t) << 8)
    | ((b'a' as ima_u32_t) << 16)
    | ((b'c' as ima_u32_t) << 24);
const DESC: ima_u32_t = (b'c' as ima_u32_t)
    | ((b's' as ima_u32_t) << 8)
    | ((b'e' as ima_u32_t) << 16)
    | ((b'd' as ima_u32_t) << 24);
const PAKT: ima_u32_t = (b't' as ima_u32_t)
    | ((b'k' as ima_u32_t) << 8)
    | ((b'a' as ima_u32_t) << 16)
    | ((b'p' as ima_u32_t) << 24);
const DATA: ima_u32_t = (b'a' as ima_u32_t)
    | ((b't' as ima_u32_t) << 8)
    | ((b'a' as ima_u32_t) << 16)
    | ((b'd' as ima_u32_t) << 24);
const IMA4: ima_u32_t = (b'4' as ima_u32_t)
    | ((b'a' as ima_u32_t) << 8)
    | ((b'm' as ima_u32_t) << 16)
    | ((b'i' as ima_u32_t) << 24);

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
unsafe fn read_field<T>(field: *const T) -> T {
    unsafe { ptr::read_unaligned(field) }
}

#[inline]
unsafe fn write_field<T>(field: *mut T, value: T) {
    unsafe { ptr::write_unaligned(field, value) }
}

/// # Safety
///
/// This function follows the C ABI and expects `info` and `data` to point to
/// memory laid out as the original C implementation expects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header = data as *const caf_header;
    let mut chunk = unsafe { header.add(1) as *const caf_chunk };
    let mut desc: *const caf_audio_description = ptr::null();
    let mut pakt: *const caf_packet_table = ptr::null();
    let blocks: *const ima_block;
    let chunk_size: ima_s64_t;

    if ima_btoh32(unsafe { read_field(ptr::addr_of!((*header).type_)) }) != CAFF {
        return -1;
    }
    if ima_btoh16(unsafe { read_field(ptr::addr_of!((*header).version)) }) != 1 {
        return -2;
    }

    loop {
        let chunk_type = ima_btoh32(unsafe { read_field(ptr::addr_of!((*chunk).type_)) });
        let size = ima_btoh64(unsafe { read_field(ptr::addr_of!((*chunk).size)) as ima_u64_t })
            as ima_s64_t;
        if chunk_type == DESC {
            desc = unsafe { chunk.add(1) as *const caf_audio_description };
        } else if chunk_type == PAKT {
            pakt = unsafe { chunk.add(1) as *const caf_packet_table };
        } else if chunk_type == DATA {
            blocks = unsafe {
                ((chunk.add(1) as *const caf_data).add(1)) as *const ima_block
            };
            chunk_size = size;
            break;
        }
        chunk = unsafe {
            (chunk.add(1) as *const ima_u8_t).offset(size as isize) as *const caf_chunk
        };
    }

    if ima_btoh32(unsafe { read_field(ptr::addr_of!((*desc).format_id)) }) != IMA4 {
        return -3;
    }

    unsafe {
        write_field(ptr::addr_of_mut!((*info).blocks), blocks);
        write_field(ptr::addr_of_mut!((*info).size), chunk_size as ima_u64_t);
        write_field(
            ptr::addr_of_mut!((*info).frame_count),
            ima_btoh64(read_field(ptr::addr_of!((*pakt).frame_count)) as ima_u64_t),
        );
        write_field(
            ptr::addr_of_mut!((*info).channel_count),
            ima_btoh32(read_field(ptr::addr_of!((*desc).channels_per_frame))),
        );

        let mut conv64_u = read_field(ptr::addr_of!((*desc).sample_rate)) as ima_u64_t;
        conv64_u = ima_btoh64(read_field(ptr::addr_of!(conv64_u)));
        write_field(
            ptr::addr_of_mut!((*info).sample_rate),
            f64::from_bits(conv64_u),
        );
    }

    0
}
