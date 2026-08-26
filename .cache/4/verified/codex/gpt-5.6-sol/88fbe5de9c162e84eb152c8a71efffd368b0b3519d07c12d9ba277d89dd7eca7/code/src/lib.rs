use std::ffi::{c_int, c_void};
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
    kind: u32,
    version: u16,
    flags: u16,
}

#[repr(C)]
struct CafChunk {
    kind: u32,
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

const fn tag(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

const CAF_FILE_TYPE: u32 = tag(b'f', b'f', b'a', b'c');
const DESCRIPTION_TYPE: u32 = tag(b'c', b's', b'e', b'd');
const PACKET_TABLE_TYPE: u32 = tag(b't', b'k', b'a', b'p');
const DATA_TYPE: u32 = tag(b'a', b't', b'a', b'd');
const IMA4_FORMAT: u32 = tag(b'4', b'a', b'm', b'i');

fn c_double_to_u64(value: f64) -> u64 {
    const U64_HIGH_BIT: u64 = 1_u64 << 63;
    const TWO_TO_63: f64 = 9_223_372_036_854_775_808.0;

    // GCC implements the C conversion with signed truncations around 2^63.
    fn cvttsd2si(value: f64) -> u64 {
        if value.is_finite() && value >= i64::MIN as f64 && value < TWO_TO_63 {
            value.trunc() as i64 as u64
        } else {
            U64_HIGH_BIT
        }
    }

    if value >= TWO_TO_63 {
        cvttsd2si(value - TWO_TO_63) ^ U64_HIGH_BIT
    } else {
        cvttsd2si(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ImaInfo, data: *const c_void) -> c_int {
    let header = data.cast::<CafHeader>();
    let mut chunk = unsafe { header.add(1).cast::<CafChunk>() };
    let mut description: *const CafAudioDescription = ptr::null();
    let mut packet_table: *const CafPacketTable = ptr::null();
    let blocks: *const ImaBlock;
    let chunk_size: i64;

    if unsafe { ptr::read(ptr::addr_of!((*header).kind)) }.swap_bytes() != CAF_FILE_TYPE {
        return -1;
    }
    if unsafe { ptr::read(ptr::addr_of!((*header).version)) }.swap_bytes() != 1 {
        return -2;
    }

    loop {
        let current_type = unsafe { ptr::read(ptr::addr_of!((*chunk).kind)) }.swap_bytes();
        let current_size = unsafe { ptr::read(ptr::addr_of!((*chunk).size)) }.swap_bytes();

        if current_type == DESCRIPTION_TYPE {
            description = unsafe { chunk.add(1).cast::<CafAudioDescription>() };
        } else if current_type == PACKET_TABLE_TYPE {
            packet_table = unsafe { chunk.add(1).cast::<CafPacketTable>() };
        } else if current_type == DATA_TYPE {
            blocks = unsafe { chunk.add(1).cast::<CafData>().add(1).cast::<ImaBlock>() };
            chunk_size = current_size;
            break;
        }

        chunk = unsafe {
            chunk
                .add(1)
                .cast::<u8>()
                .offset(current_size as isize)
                .cast::<CafChunk>()
        };
    }

    let format_id = unsafe { ptr::read(ptr::addr_of!((*description).format_id)) }.swap_bytes();
    if format_id != IMA4_FORMAT {
        return -3;
    }

    unsafe {
        ptr::write(ptr::addr_of_mut!((*info).blocks), blocks);
        ptr::write(ptr::addr_of_mut!((*info).size), chunk_size as u64);
        ptr::write(
            ptr::addr_of_mut!((*info).frame_count),
            ptr::read(ptr::addr_of!((*packet_table).frame_count)).swap_bytes() as u64,
        );
        ptr::write(
            ptr::addr_of_mut!((*info).channel_count),
            ptr::read(ptr::addr_of!((*description).channels_per_frame)).swap_bytes(),
        );

        let converted_sample_rate =
            c_double_to_u64(ptr::read(ptr::addr_of!((*description).sample_rate)));
        ptr::write(
            ptr::addr_of_mut!((*info).sample_rate),
            f64::from_bits(converted_sample_rate.swap_bytes()),
        );
    }

    0
}
