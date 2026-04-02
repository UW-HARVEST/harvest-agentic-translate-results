use std::os::raw::c_int;

type ImaU8 = u8;
type ImaU16 = u16;
type ImaU32 = u32;
type ImaU64 = u64;
type ImaF64 = f64;
type ImaS64 = i64;

#[repr(C)]
pub struct ImaBlock {
    pub preamble: ImaU16,
    pub data: [ImaU8; 32],
}

#[repr(C)]
pub struct ImaInfo {
    pub blocks: *const ImaBlock,
    pub size: ImaU64,
    pub sample_rate: ImaF64,
    pub frame_count: ImaU64,
    pub channel_count: ImaU32,
}

#[repr(C)]
struct CafHeader {
    type_: ImaU32,
    version: ImaU16,
    flags: ImaU16,
}

#[repr(C)]
struct CafChunk {
    type_: ImaU32,
    size: ImaS64,
}

#[repr(C)]
struct CafAudioDescription {
    sample_rate: ImaF64,
    format_id: ImaU32,
    format_flags: ImaU32,
    bytes_per_packet: ImaU32,
    frames_per_packet: ImaU32,
    channels_per_frame: ImaU32,
    bits_per_channel: ImaU32,
}

#[repr(C)]
struct CafPacketTable {
    packet_count: ImaS64,
    frame_count: ImaS64,
    priming_frames: i32,
    remainder_frames: i32,
}

#[repr(C)]
struct CafData {
    edit_count: ImaU32,
}

fn ima_bswap16(v: ImaU16) -> ImaU16 {
    (v << 0x08 & 0xff00u16) | (v >> 0x08 & 0x00ffu16)
}

fn ima_bswap32(v: ImaU32) -> ImaU32 {
    (v << 0x18 & 0xff000000u32)
        | (v << 0x08 & 0x00ff0000u32)
        | (v >> 0x08 & 0x0000ff00u32)
        | (v >> 0x18 & 0x000000ffu32)
}

fn ima_bswap64(v: ImaU64) -> ImaU64 {
    (v << 0x38 & 0xff00000000000000u64)
        | (v << 0x28 & 0x00ff000000000000u64)
        | (v << 0x18 & 0x0000ff0000000000u64)
        | (v << 0x08 & 0x000000ff00000000u64)
        | (v >> 0x08 & 0x00000000ff000000u64)
        | (v >> 0x18 & 0x0000000000ff0000u64)
        | (v >> 0x28 & 0x000000000000ff00u64)
        | (v >> 0x38 & 0x00000000000000ffu64)
}

#[inline]
fn ima_btoh16(v: ImaU16) -> ImaU16 {
    ima_bswap16(v)
}

#[inline]
fn ima_btoh32(v: ImaU32) -> ImaU32 {
    ima_bswap32(v)
}

#[inline]
fn ima_btoh64(v: ImaU64) -> ImaU64 {
    ima_bswap64(v)
}

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> ImaU32 {
    (a as ImaU32) | ((b as ImaU32) << 8) | ((c as ImaU32) << 16) | ((d as ImaU32) << 24)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ImaInfo, data: *const u8) -> c_int {
    let header = data as *const CafHeader;
    let mut chunk = unsafe { header.add(1) } as *const CafChunk;
    let mut desc: *const CafAudioDescription = std::ptr::null();
    let mut pakt: *const CafPacketTable = std::ptr::null();
    let blocks: *const ImaBlock;
    let chunk_size: ImaS64;

    if ima_btoh32(unsafe { (*header).type_ }) != fourcc(b'f', b'f', b'a', b'c') {
        return -1;
    }
    if ima_btoh16(unsafe { (*header).version }) != 1 {
        return -2;
    }

    loop {
        let chunk_type = ima_btoh32(unsafe { (*chunk).type_ });
        let cs = ima_btoh64(unsafe { (*chunk).size } as ImaU64);

        if chunk_type == fourcc(b'c', b's', b'e', b'd') {
            desc = unsafe { chunk.add(1) } as *const CafAudioDescription;
        } else if chunk_type == fourcc(b't', b'k', b'a', b'p') {
            pakt = unsafe { chunk.add(1) } as *const CafPacketTable;
        } else if chunk_type == fourcc(b'a', b't', b'a', b'd') {
            let caf_data = unsafe { chunk.add(1) } as *const CafData;
            blocks = unsafe { caf_data.add(1) } as *const ImaBlock;
            chunk_size = cs as ImaS64;

            // Check format_id after finding data chunk (matches C control flow)
            if ima_btoh32(unsafe { (*desc).format_id }) != fourcc(b'4', b'a', b'm', b'i') {
                return -3;
            }

            unsafe {
                (*info).blocks = blocks;
                (*info).size = chunk_size as ImaU64;
                (*info).frame_count = ima_btoh64((*pakt).frame_count as ImaU64);
                (*info).channel_count = ima_btoh32((*desc).channels_per_frame);
                // Match C: conv64.u = desc->sample_rate (numeric f64→u64 conversion)
                // then conv64.u = ima_btoh64(conv64.u); info->sample_rate = conv64.f
                let sr_as_u64 = (*desc).sample_rate as ImaU64;
                let swapped = ima_btoh64(sr_as_u64);
                (*info).sample_rate = ImaF64::from_bits(swapped);
            }

            return 0;
        }

        chunk = unsafe { (chunk.add(1) as *const ImaU8).add(cs as usize) } as *const CafChunk;
    }
}
