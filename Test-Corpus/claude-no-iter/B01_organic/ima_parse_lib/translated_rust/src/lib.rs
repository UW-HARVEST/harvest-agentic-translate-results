// Translation of c_src/src/lib.c to Rust.
// Preserves C ABI and reproduces original behavior, including bugs.

use std::ffi::c_int;
use std::ffi::c_void;
use std::ptr;

#[repr(C)]
pub struct ImaBlock {
    pub preamble: u16,
    pub data: [u8; 32],
}

#[repr(C)]
pub struct ImaInfo {
    pub blocks: *const ImaBlock,
    pub size: u64,
    pub sample_rate: f64,
    pub frame_count: u64,
    pub channel_count: u32,
}

#[repr(C)]
struct CafHeader {
    r#type: u32,
    version: u16,
    flags: u16,
}

#[repr(C)]
struct CafChunk {
    r#type: u32,
    // 4 bytes implicit padding here due to i64 alignment (matches C layout)
    size: i64,
}

#[repr(C)]
struct CafAudioDescription {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
}

#[repr(C)]
struct CafPacketTable {
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
}

#[repr(C)]
struct CafData {
    edit_count: u32,
}

#[inline]
fn ima_bswap16(v: u16) -> u16 {
    (v << 0x08 & 0xff00u16) | (v >> 0x08 & 0x00ffu16)
}

#[inline]
fn ima_bswap32(v: u32) -> u32 {
    (v << 0x18 & 0xff000000u32)
        | (v << 0x08 & 0x00ff0000u32)
        | (v >> 0x08 & 0x0000ff00u32)
        | (v >> 0x18 & 0x000000ffu32)
}

#[inline]
fn ima_bswap64(v: u64) -> u64 {
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
fn ima_btoh16(v: u16) -> u16 {
    ima_bswap16(v)
}

#[inline]
fn ima_btoh32(v: u32) -> u32 {
    ima_bswap32(v)
}

#[inline]
fn ima_btoh64(v: u64) -> u64 {
    ima_bswap64(v)
}

#[inline]
fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ImaInfo, data: *const c_void) -> c_int {
    let header = data as *const CafHeader;
    // chunk = (const struct caf_chunk *)&header[1];
    let mut chunk: *const CafChunk = header.add(1) as *const CafChunk;
    let mut desc: *const CafAudioDescription = ptr::null();
    let mut pakt: *const CafPacketTable = ptr::null();
    let blocks: *const ImaBlock;
    let mut chunk_size: i64;
    let mut chunk_type: u32;

    // if (ima_btoh32(header->type) != 'caff' (LE-encoded)) return -1;
    let hdr_type = ptr::read_unaligned(ptr::addr_of!((*header).r#type));
    if ima_btoh32(hdr_type) != fourcc(b'f', b'f', b'a', b'c') {
        return -1;
    }
    // if (ima_btoh16(header->version) != 1) return -2;
    let hdr_version = ptr::read_unaligned(ptr::addr_of!((*header).version));
    if ima_btoh16(hdr_version) != 1 {
        return -2;
    }

    loop {
        chunk_type = ima_btoh32(ptr::read_unaligned(ptr::addr_of!((*chunk).r#type)));
        chunk_size =
            ima_btoh64(ptr::read_unaligned(ptr::addr_of!((*chunk).size)) as u64) as i64;

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
        let after_chunk = chunk.add(1) as *const u8;
        chunk = after_chunk.offset(chunk_size as isize) as *const CafChunk;
    }

    // if (ima_btoh32(desc->format_id) != 'ima4' (LE-encoded)) return -3;
    let fmt_id = ptr::read_unaligned(ptr::addr_of!((*desc).format_id));
    if ima_btoh32(fmt_id) != fourcc(b'4', b'a', b'm', b'i') {
        return -3;
    }

    (*info).blocks = blocks;
    (*info).size = chunk_size as u64;
    (*info).frame_count =
        ima_btoh64(ptr::read_unaligned(ptr::addr_of!((*pakt).frame_count)) as u64);
    (*info).channel_count =
        ima_btoh32(ptr::read_unaligned(ptr::addr_of!((*desc).channels_per_frame)));

    // Reproduce original C behavior exactly:
    //   conv64.u = desc->sample_rate;  // implicit double -> u64 numeric conversion
    //   conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);
    //   info->sample_rate = conv64.f;  // reinterpret u64 bits as f64
    let sample_rate_f: f64 = ptr::read_unaligned(ptr::addr_of!((*desc).sample_rate));
    let mut conv_u: u64 = sample_rate_f as u64;
    conv_u = ima_btoh64(conv_u);
    (*info).sample_rate = f64::from_bits(conv_u);

    0
}
