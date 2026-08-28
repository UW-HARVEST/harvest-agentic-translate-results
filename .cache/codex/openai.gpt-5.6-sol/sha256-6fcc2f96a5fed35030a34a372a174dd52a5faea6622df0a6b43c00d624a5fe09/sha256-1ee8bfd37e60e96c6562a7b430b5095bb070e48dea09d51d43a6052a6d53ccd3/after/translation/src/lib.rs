use std::ffi::{c_double, c_int, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};
use std::mem::size_of;
use std::ptr::{addr_of, addr_of_mut};

pub type ImaU8 = c_uchar;
pub type ImaU16 = c_ushort;
pub type ImaU32 = c_uint;
pub type ImaU64 = c_ulonglong;
pub type ImaF64 = c_double;

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
    chunk_type: ImaU32,
    version: ImaU16,
    flags: ImaU16,
}

#[repr(C)]
struct CafChunk {
    chunk_type: ImaU32,
    size: i64,
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
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
}

#[repr(C)]
struct CafData {
    edit_count: ImaU32,
}

const CAF_FILE_TYPE: ImaU32 = u32::from_le_bytes(*b"ffac");
const DESC_CHUNK_TYPE: ImaU32 = u32::from_le_bytes(*b"csed");
const PAKT_CHUNK_TYPE: ImaU32 = u32::from_le_bytes(*b"tkap");
const DATA_CHUNK_TYPE: ImaU32 = u32::from_le_bytes(*b"atad");
const IMA4_FORMAT_ID: ImaU32 = u32::from_le_bytes(*b"4ami");

// Match GCC's x86-64 lowering of C's f64-to-unsigned conversion, including
// CVTTSD2SI's indefinite result for values it cannot represent.
fn gcc_f64_to_u64(value: f64) -> u64 {
    const SIGN_BIT: u64 = 1 << 63;
    const TWO_TO_63: f64 = 9_223_372_036_854_775_808.0;

    fn cvttsd2si(value: f64) -> u64 {
        const SIGN_BIT: u64 = 1 << 63;
        const TWO_TO_63: f64 = 9_223_372_036_854_775_808.0;

        if value.is_nan() || !(-TWO_TO_63..TWO_TO_63).contains(&value) {
            SIGN_BIT
        } else {
            (value.trunc() as i64) as u64
        }
    }

    if value >= TWO_TO_63 {
        cvttsd2si(value - TWO_TO_63) ^ SIGN_BIT
    } else {
        cvttsd2si(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ImaInfo, data: *const c_void) -> c_int {
    let header = data.cast::<CafHeader>();
    let mut chunk = data
        .cast::<u8>()
        .wrapping_add(size_of::<CafHeader>())
        .cast::<CafChunk>();
    let mut desc: *const CafAudioDescription = std::ptr::null();
    let mut pakt: *const CafPacketTable = std::ptr::null();
    let blocks: *const ImaBlock;
    let mut chunk_size: i64;

    if unsafe { addr_of!((*header).chunk_type).read_unaligned() }.swap_bytes() != CAF_FILE_TYPE {
        return -1;
    }
    if unsafe { addr_of!((*header).version).read_unaligned() }.swap_bytes() != 1 {
        return -2;
    }

    loop {
        let chunk_type = unsafe { addr_of!((*chunk).chunk_type).read_unaligned() }.swap_bytes();
        chunk_size = unsafe { addr_of!((*chunk).size).read_unaligned() }.swap_bytes();
        let payload = chunk.cast::<u8>().wrapping_add(size_of::<CafChunk>());

        if chunk_type == DESC_CHUNK_TYPE {
            desc = payload.cast();
        } else if chunk_type == PAKT_CHUNK_TYPE {
            pakt = payload.cast();
        } else if chunk_type == DATA_CHUNK_TYPE {
            blocks = payload
                .wrapping_add(size_of::<CafData>())
                .cast::<ImaBlock>();
            break;
        }

        chunk = payload
            .wrapping_offset(chunk_size as isize)
            .cast::<CafChunk>();
    }

    if unsafe { addr_of!((*desc).format_id).read_unaligned() }.swap_bytes() != IMA4_FORMAT_ID {
        return -3;
    }

    unsafe {
        addr_of_mut!((*info).blocks).write_unaligned(blocks);
        addr_of_mut!((*info).size).write_unaligned(chunk_size as ImaU64);
        addr_of_mut!((*info).frame_count)
            .write_unaligned(addr_of!((*pakt).frame_count).read_unaligned().swap_bytes() as ImaU64);
        addr_of_mut!((*info).channel_count).write_unaligned(
            addr_of!((*desc).channels_per_frame)
                .read_unaligned()
                .swap_bytes(),
        );
        let converted_sample_rate = gcc_f64_to_u64(addr_of!((*desc).sample_rate).read_unaligned());
        addr_of_mut!((*info).sample_rate)
            .write_unaligned(ImaF64::from_bits(converted_sample_rate.swap_bytes()));
    }

    0
}
