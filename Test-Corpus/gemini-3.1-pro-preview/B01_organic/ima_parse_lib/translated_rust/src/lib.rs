use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(C)]
pub struct ima_block {
    pub preamble: u16,
    pub data: [u8; 32],
}

#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: u64,
    pub sample_rate: f64,
    pub frame_count: u64,
    pub channel_count: u32,
}

#[repr(C)]
struct caf_header {
    type_: u32,
    version: u16,
    flags: u16,
}

#[repr(C)]
struct caf_chunk {
    type_: u32,
    size: i64,
}

#[repr(C)]
struct caf_audio_description {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
}

#[repr(C)]
struct caf_packet_table {
    packet_count: i64,
    frame_count: i64,
    priming_frames: i32,
    remainder_frames: i32,
}

#[repr(C)]
struct caf_data {
    edit_count: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    unsafe {
        let data_ptr = data as *const u8;
        let header = data_ptr as *const caf_header;
        let mut chunk = data_ptr.add(std::mem::size_of::<caf_header>()) as *const caf_chunk;

        let mut desc: *const caf_audio_description = std::ptr::null();
        let mut pakt: *const caf_packet_table = std::ptr::null();
        let mut blocks: *const ima_block = std::ptr::null();

        let mut chunk_size: i64 = 0;
        let mut chunk_type: u32;

        let expected_type = (b'f' as u32) | ((b'f' as u32) << 8) | ((b'a' as u32) << 16) | ((b'c' as u32) << 24);
        if (*header).type_.swap_bytes() != expected_type {
            return -1;
        }
        if (*header).version.swap_bytes() != 1 {
            return -2;
        }

        loop {
            chunk_type = (*chunk).type_.swap_bytes();
            chunk_size = (*chunk).size.swap_bytes();

            let desc_type = (b'c' as u32) | ((b's' as u32) << 8) | ((b'e' as u32) << 16) | ((b'd' as u32) << 24);
            let pakt_type = (b't' as u32) | ((b'k' as u32) << 8) | ((b'a' as u32) << 16) | ((b'p' as u32) << 24);
            let data_type = (b'a' as u32) | ((b't' as u32) << 8) | ((b'a' as u32) << 16) | ((b'd' as u32) << 24);

            if chunk_type == desc_type {
                desc = chunk.add(1) as *const caf_audio_description;
            } else if chunk_type == pakt_type {
                pakt = chunk.add(1) as *const caf_packet_table;
            } else if chunk_type == data_type {
                let caf_data_ptr = chunk.add(1) as *const caf_data;
                blocks = caf_data_ptr.add(1) as *const ima_block;
                break;
            }

            chunk = (chunk.add(1) as *const u8).add(chunk_size as usize) as *const caf_chunk;
        }

        let format_id_expected = (b'4' as u32) | ((b'a' as u32) << 8) | ((b'm' as u32) << 16) | ((b'i' as u32) << 24);
        if (*desc).format_id.swap_bytes() != format_id_expected {
            return -3;
        }

        (*info).blocks = blocks;
        (*info).size = chunk_size as u64;
        (*info).frame_count = (*pakt).frame_count.swap_bytes() as u64;
        (*info).channel_count = (*desc).channels_per_frame.swap_bytes();

        let sample_rate_u64 = (*desc).sample_rate.to_bits();
        let sample_rate_swapped = sample_rate_u64.swap_bytes();
        (*info).sample_rate = f64::from_bits(sample_rate_swapped);

        0
    }
}
