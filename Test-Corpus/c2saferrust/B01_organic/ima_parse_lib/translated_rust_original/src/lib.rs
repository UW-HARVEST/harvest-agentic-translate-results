






pub type ima_u32_t = ::core::ffi::c_uint;
pub type ima_u64_t = ::core::ffi::c_ulonglong;
pub type ima_f64_t = ::core::ffi::c_double;
pub type ima_u8_t = ::core::ffi::c_uchar;
pub type ima_u16_t = ::core::ffi::c_ushort;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ima_block {
    pub preamble: ima_u16_t,
    pub data: [ima_u8_t; 32],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ima_info {
    pub blocks: *const ima_block,
    pub size: ima_u64_t,
    pub sample_rate: ima_f64_t,
    pub frame_count: ima_u64_t,
    pub channel_count: ima_u32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub f: ima_f64_t,
    pub u: ima_u64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct caf_audio_description {
    pub sample_rate: ima_f64_t,
    pub format_id: ima_u32_t,
    pub format_flags: ima_u32_t,
    pub bytes_per_packet: ima_u32_t,
    pub frames_per_packet: ima_u32_t,
    pub channels_per_frame: ima_u32_t,
    pub bits_per_channel: ima_u32_t,
}
pub type ima_s64_t = ::core::ffi::c_longlong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct caf_packet_table {
    pub packet_count: ima_s64_t,
    pub frame_count: ima_s64_t,
    pub priming_frames: ima_s32_t,
    pub remainder_frames: ima_s32_t,
}
pub type ima_s32_t = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct caf_chunk {
    pub type_0: ima_u32_t,
    pub size: ima_s64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct caf_header {
    pub type_0: ima_u32_t,
    pub version: ima_u16_t,
    pub flags: ima_u16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct caf_data {
    pub edit_count: ima_u32_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    v.swap_bytes()
}

fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    v.swap_bytes()
}

fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    v.swap_bytes()
}

fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    u32::from_be(v)
}

fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    u64::from_be(v as u64) as ima_u64_t
}

#[no_mangle]
pub fn ima_parse(info: &mut ima_info, data: &[u8]) -> ::core::ffi::c_int {
    let header_size = ::core::mem::size_of::<caf_header>();
    let chunk_header_size = ::core::mem::size_of::<caf_chunk>();
    let desc_size = ::core::mem::size_of::<caf_audio_description>();
    let pakt_size = ::core::mem::size_of::<caf_packet_table>();
    let caf_data_size = ::core::mem::size_of::<caf_data>();

    if data.len() < header_size {
        return -1;
    }

    let header = unsafe { &*(data.as_ptr() as *const caf_header) };

    let caff_magic = 'f' as i32 as ima_u8_t as ima_u32_t
        | ('f' as i32 as ima_u8_t as ima_u32_t) << 8
        | ('a' as i32 as ima_u8_t as ima_u32_t) << 16
        | ('c' as i32 as ima_u8_t as ima_u32_t) << 24;
    let desc_magic = 'c' as i32 as ima_u8_t as ima_u32_t
        | ('s' as i32 as ima_u8_t as ima_u32_t) << 8
        | ('e' as i32 as ima_u8_t as ima_u32_t) << 16
        | ('d' as i32 as ima_u8_t as ima_u32_t) << 24;
    let pakt_magic = 't' as i32 as ima_u8_t as ima_u32_t
        | ('k' as i32 as ima_u8_t as ima_u32_t) << 8
        | ('a' as i32 as ima_u8_t as ima_u32_t) << 16
        | ('p' as i32 as ima_u8_t as ima_u32_t) << 24;
    let data_magic = 'a' as i32 as ima_u8_t as ima_u32_t
        | ('t' as i32 as ima_u8_t as ima_u32_t) << 8
        | ('a' as i32 as ima_u8_t as ima_u32_t) << 16
        | ('d' as i32 as ima_u8_t as ima_u32_t) << 24;
    let ima4_magic = '4' as i32 as ima_u8_t as ima_u32_t
        | ('a' as i32 as ima_u8_t as ima_u32_t) << 8
        | ('m' as i32 as ima_u8_t as ima_u32_t) << 16
        | ('i' as i32 as ima_u8_t as ima_u32_t) << 24;

    if unsafe { ima_btoh32(header.type_0) } != caff_magic {
        return -1;
    }

    if unsafe { ima_btoh16(header.version) } as ::core::ffi::c_int != 1 {
        return -2;
    }

    let mut offset = header_size;
    let mut desc_offset = None;
    let mut pakt_offset = None;
    let mut blocks_offset = None;
    let mut chunk_size: ima_s64_t = 0;

    while offset + chunk_header_size <= data.len() {
        let chunk = unsafe { &*(data[offset..].as_ptr() as *const caf_chunk) };
        let chunk_type = unsafe { ima_btoh32(chunk.type_0) } as ::core::ffi::c_uint;
        let current_chunk_size = unsafe { ima_btoh64(chunk.size as ima_u64_t) } as ima_s64_t;

        if current_chunk_size < 0 {
            return -1;
        }

        let payload_offset = offset + chunk_header_size;
        let payload_size = current_chunk_size as usize;

        if payload_offset > data.len() || payload_offset.saturating_add(payload_size) > data.len() {
            return -1;
        }

        if chunk_type == desc_magic {
            if payload_offset + desc_size > data.len() {
                return -1;
            }
            desc_offset = Some(payload_offset);
        } else if chunk_type == pakt_magic {
            if payload_offset + pakt_size > data.len() {
                return -1;
            }
            pakt_offset = Some(payload_offset);
        } else if chunk_type == data_magic {
            let block_start = payload_offset + caf_data_size;
            if block_start > data.len() {
                return -1;
            }
            blocks_offset = Some(block_start);
            chunk_size = current_chunk_size;
            break;
        }

        let next_offset = payload_offset + payload_size;
        if next_offset <= offset {
            return -1;
        }
        offset = next_offset;
    }

    let desc = match desc_offset {
        Some(off) => unsafe { &*(data[off..].as_ptr() as *const caf_audio_description) },
        None => return -3,
    };

    let pakt = match pakt_offset {
        Some(off) => unsafe { &*(data[off..].as_ptr() as *const caf_packet_table) },
        None => return -1,
    };

    if unsafe { ima_btoh32(desc.format_id) } != ima4_magic {
        return -3;
    }

    let blocks = match blocks_offset {
        Some(off) => unsafe { data.as_ptr().add(off) as *const ima_block },
        None => return -1,
    };

    info.blocks = blocks;
    info.size = chunk_size as ima_u64_t;
    info.frame_count = unsafe { ima_btoh64(pakt.frame_count as ima_u64_t) };
    info.channel_count = unsafe { ima_btoh32(desc.channels_per_frame) };

    let sample_rate_bits = unsafe { ima_btoh64(desc.sample_rate as ima_u64_t) };
    info.sample_rate = f64::from_bits(sample_rate_bits);

    0
}

