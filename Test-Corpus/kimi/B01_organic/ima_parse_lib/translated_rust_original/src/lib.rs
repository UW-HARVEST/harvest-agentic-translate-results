use std::os::raw::{c_int, c_void};
use std::slice;

pub type ImaU32T = u32;
pub type ImaU64T = u64;
pub type ImaF64T = f64;
pub type ImaU8T = u8;
pub type ImaU16T = u16;

type ImaS32T = i32;
type ImaS64T = i64;

#[repr(C)]
pub struct ImaBlock {
    pub preamble: ImaU16T,
    pub data: [ImaU8T; 32],
}

#[repr(C)]
pub struct ImaInfo {
    pub blocks: *const ImaBlock,
    pub size: ImaU64T,
    pub sample_rate: ImaF64T,
    pub frame_count: ImaU64T,
    pub channel_count: ImaU32T,
}

#[repr(C)]
struct CafHeader {
    type_: ImaU32T,
    version: ImaU16T,
    flags: ImaU16T,
}

#[repr(C)]
struct CafChunk {
    type_: ImaU32T,
    size: ImaS64T,
}

#[repr(C)]
struct CAFAudioDescription {
    sample_rate: ImaF64T,
    format_id: ImaU32T,
    format_flags: ImaU32T,
    bytes_per_packet: ImaU32T,
    frames_per_packet: ImaU32T,
    channels_per_frame: ImaU32T,
    bits_per_channel: ImaU32T,
}

#[repr(C)]
struct CAFPacketTable {
    packet_count: ImaS64T,
    frame_count: ImaS64T,
    priming_frames: ImaS32T,
    remainder_frames: ImaS32T,
}

#[repr(C)]
struct CAFData {
    edit_count: ImaU32T,
}

fn ima_bswap16(v: ImaU16T) -> ImaU16T {
    (v << 0x08 & 0xff00u16) | (v >> 0x08 & 0x00ffu16)
}

fn ima_bswap32(v: ImaU32T) -> ImaU32T {
    (v << 0x18 & 0xff000000u32) | (v << 0x08 & 0x00ff0000u32) |
    (v >> 0x08 & 0x0000ff00u32) | (v >> 0x18 & 0x000000ffu32)
}

fn ima_bswap64(v: ImaU64T) -> ImaU64T {
    (v << 0x38 & 0xff00000000000000u64) |
    (v << 0x28 & 0x00ff000000000000u64) |
    (v << 0x18 & 0x0000ff0000000000u64) |
    (v << 0x08 & 0x000000ff00000000u64) |
    (v >> 0x08 & 0x00000000ff000000u64) |
    (v >> 0x18 & 0x0000000000ff0000u64) |
    (v >> 0x28 & 0x000000000000ff00u64) |
    (v >> 0x38 & 0x00000000000000ffu64)
}

fn ima_btoh16(v: ImaU16T) -> ImaU16T {
    ima_bswap16(v)
}

fn ima_btoh32(v: ImaU32T) -> ImaU32T {
    ima_bswap32(v)
}

fn ima_btoh64(v: ImaU64T) -> ImaU64T {
    ima_bswap64(v)
}

#[unsafe(no_mangle)]
pub extern "C" fn ima_parse(info: *mut ImaInfo, data: *const c_void) -> c_int {
    unsafe {
        let header = data as *const CafHeader;
        let mut chunk = header.add(1) as *const CAFChunk;
        let mut desc: *const CAFAudioDescription = std::ptr::null();
        let mut pakt: *const CAFPacketTable = std::ptr::null();
        let mut blocks: *const ImaBlock = std::ptr::null();
        let mut chunk_size: ImaS64T = 0;
        let mut chunk_type: ImaU32T = 0;

        if ima_btoh32((*header).type_) !=
            (b'f' as ImaU32T) | ((b'f' as ImaU32T) << 8) |
            ((b'a' as ImaU32T) << 16) | ((b'c' as ImaU32T) << 24) {
            return -1;
        }
        if ima_btoh16((*header).version) != 1 {
            return -2;
        }

        loop {
            chunk_type = ima_btoh32((*chunk).type_);
            chunk_size = ima_btoh64((*chunk).size);

            if chunk_type ==
                (b'c' as ImaU32T) | ((b's' as ImaU32T) << 8) |
                ((b'e' as ImaU32T) << 16) | ((b'd' as ImaU32T) << 24) {
                desc = chunk.add(1) as *const CAFAudioDescription;
            } else if chunk_type ==
                (b't' as ImaU32T) | ((b'k' as ImaU32T) << 8) |
                ((b'a' as ImaU32T) << 16) | ((b'p' as ImaU32T) << 24) {
                pakt = chunk.add(1) as *const CAFPacketTable;
            } else if chunk_type ==
                (b'a' as ImaU32T) | ((b't' as ImaU32T) << 8) |
                ((b'a' as ImaU32T) << 16) | ((b'd' as ImaU32T) << 24) {
                let caf_data = chunk.add(1) as *const CAFData;
                blocks = caf_data.add(1) as *const ImaBlock;
                break;
            }

            chunk = (chunk.add(1) as *const u8).add(chunk_size as usize) as *const CAFChunk;
        }

        if ima_btoh32((*desc).format_id) !=
            (b'4' as ImaU32T) | ((b'a' as ImaU32T) << 8) |
            ((b'm' as ImaU32T) << 16) | ((b'i' as ImaU32T) << 24) {
            return -3;
        }

        (*info).blocks = blocks;
        (*info).size = chunk_size as ImaU64T;
        (*info).frame_count = ima_btoh64((*pakt).frame_count);
        (*info).channel_count = ima_btoh32((*desc).channels_per_frame);

        let sample_rate_bits = ima_btoh64((*desc).sample_rate.to_bits());
        (*info).sample_rate = ImaF64T::from_bits(sample_rate_bits);

        0
    }
}
