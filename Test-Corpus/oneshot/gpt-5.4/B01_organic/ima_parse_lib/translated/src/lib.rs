use core::ffi::{c_double, c_int, c_void};
use core::mem;
use core::ptr;

pub type ImaU32T = u32;
pub type ImaU64T = u64;
pub type ImaF64T = c_double;
pub type ImaU8T = u8;
pub type ImaU16T = u16;
pub type ImaS32T = i32;
pub type ImaS64T = i64;

#[repr(C)]
pub struct ima_block {
    pub preamble: ImaU16T,
    pub data: [ImaU8T; 32],
}

#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ImaU64T,
    pub sample_rate: ImaF64T,
    pub frame_count: ImaU64T,
    pub channel_count: ImaU32T,
}

#[repr(C)]
struct caf_header {
    type_: ImaU32T,
    version: ImaU16T,
    flags: ImaU16T,
}

#[repr(C)]
struct caf_chunk {
    type_: ImaU32T,
    size: ImaS64T,
}

#[repr(C)]
struct caf_audio_description {
    sample_rate: ImaF64T,
    format_id: ImaU32T,
    format_flags: ImaU32T,
    bytes_per_packet: ImaU32T,
    frames_per_packet: ImaU32T,
    channels_per_frame: ImaU32T,
    bits_per_channel: ImaU32T,
}

#[repr(C)]
struct caf_packet_table {
    packet_count: ImaS64T,
    frame_count: ImaS64T,
    priming_frames: ImaS32T,
    remainder_frames: ImaS32T,
}

#[repr(C)]
struct caf_data {
    edit_count: ImaU32T,
}

fn ima_btoh16(v: ImaU16T) -> ImaU16T {
    u16::from_be(v)
}

fn ima_btoh32(v: ImaU32T) -> ImaU32T {
    u32::from_be(v)
}

fn ima_btoh64(v: ImaU64T) -> ImaU64T {
    u64::from_be(v)
}

fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    let header = data as *const caf_header;
    let mut chunk = header.add(1) as *const caf_chunk;
    let mut desc: *const caf_audio_description = ptr::null();
    let mut pakt: *const caf_packet_table = ptr::null();
    let mut blocks: *const ima_block = ptr::null();
    let chunk_size: ImaS64T;

    if ima_btoh32((*header).type_) != fourcc(b'f', b'f', b'a', b'c') {
        return -1;
    }
    if ima_btoh16((*header).version) != 1 {
        return -2;
    }

    loop {
        let chunk_type = ima_btoh32((*chunk).type_);
        let current_chunk_size = ima_btoh64((*chunk).size as u64) as ImaS64T;

        if chunk_type == fourcc(b'c', b's', b'e', b'd') {
            desc = chunk.add(1) as *const caf_audio_description;
        } else if chunk_type == fourcc(b't', b'k', b'a', b'p') {
            pakt = chunk.add(1) as *const caf_packet_table;
        } else if chunk_type == fourcc(b'a', b't', b'a', b'd') {
            blocks = ((chunk.add(1) as *const caf_data).add(1)) as *const ima_block;
            chunk_size = current_chunk_size;
            break;
        }

        chunk = ((chunk.add(1) as *const u8).add(current_chunk_size as usize)) as *const caf_chunk;
    }

    if ima_btoh32((*desc).format_id) != fourcc(b'4', b'a', b'm', b'i') {
        return -3;
    }

    (*info).blocks = blocks;
    (*info).size = chunk_size as ImaU64T;
    (*info).frame_count = ima_btoh64((*pakt).frame_count as u64);
    (*info).channel_count = ima_btoh32((*desc).channels_per_frame);

    let sample_bits = ima_btoh64(mem::transmute::<ImaF64T, u64>((*desc).sample_rate));
    (*info).sample_rate = mem::transmute::<u64, ImaF64T>(sample_bits);

    0
}
