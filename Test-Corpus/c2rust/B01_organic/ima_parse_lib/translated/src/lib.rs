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
unsafe extern "C" fn ima_bswap16(mut v: ima_u16_t) -> ima_u16_t {
    return (((v as ::core::ffi::c_int) << 0x8 as ::core::ffi::c_int) as ::core::ffi::c_uint
        & 0xff00 as ::core::ffi::c_uint
        | (v as ::core::ffi::c_int >> 0x8 as ::core::ffi::c_int) as ::core::ffi::c_uint
            & 0xff as ::core::ffi::c_uint) as ima_u16_t;
}
unsafe extern "C" fn ima_bswap32(mut v: ima_u32_t) -> ima_u32_t {
    return ((v << 0x18 as ::core::ffi::c_int) as ::core::ffi::c_ulong
        & 0xff000000 as ::core::ffi::c_ulong
        | (v << 0x8 as ::core::ffi::c_int) as ::core::ffi::c_ulong
            & 0xff0000 as ::core::ffi::c_ulong
        | (v >> 0x8 as ::core::ffi::c_int) as ::core::ffi::c_ulong & 0xff00 as ::core::ffi::c_ulong
        | (v >> 0x18 as ::core::ffi::c_int) as ::core::ffi::c_ulong & 0xff as ::core::ffi::c_ulong)
        as ima_u32_t;
}
unsafe extern "C" fn ima_bswap64(mut v: ima_u64_t) -> ima_u64_t {
    return v << 0x38 as ::core::ffi::c_int & 0xff00000000000000 as ima_u64_t
        | v << 0x28 as ::core::ffi::c_int & 0xff000000000000 as ima_u64_t
        | v << 0x18 as ::core::ffi::c_int & 0xff0000000000 as ima_u64_t
        | v << 0x8 as ::core::ffi::c_int & 0xff00000000 as ima_u64_t
        | v >> 0x8 as ::core::ffi::c_int & 0xff000000 as ima_u64_t
        | v >> 0x18 as ::core::ffi::c_int & 0xff0000 as ima_u64_t
        | v >> 0x28 as ::core::ffi::c_int & 0xff00 as ima_u64_t
        | v >> 0x38 as ::core::ffi::c_int & 0xff as ima_u64_t;
}
unsafe extern "C" fn ima_btoh16(mut v: ima_u16_t) -> ima_u16_t {
    return ima_bswap16(v);
}
unsafe extern "C" fn ima_btoh32(mut v: ima_u32_t) -> ima_u32_t {
    return ima_bswap32(v);
}
unsafe extern "C" fn ima_btoh64(mut v: ima_u64_t) -> ima_u64_t {
    return ima_bswap64(v);
}
#[no_mangle]
pub unsafe extern "C" fn ima_parse(
    mut info: *mut ima_info,
    mut data: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut header: *const caf_header = data as *const caf_header;
    let mut chunk: *const caf_chunk =
        header.offset(1 as ::core::ffi::c_int as isize) as *const caf_header as *const caf_chunk;
    let mut desc: *const caf_audio_description = ::core::ptr::null::<caf_audio_description>();
    let mut pakt: *const caf_packet_table = ::core::ptr::null::<caf_packet_table>();
    let mut blocks: *const ima_block = ::core::ptr::null::<ima_block>();
    let mut conv64: C2RustUnnamed = C2RustUnnamed { f: 0. };
    let mut chunk_size: ima_s64_t = 0;
    let mut chunk_type: ::core::ffi::c_uint = 0;
    if ima_btoh32((*header).type_0)
        != 'f' as i32 as ima_u8_t as ima_u32_t
            | ('f' as i32 as ima_u8_t as ima_u32_t) << 8 as ::core::ffi::c_int
            | ('a' as i32 as ima_u8_t as ima_u32_t) << 16 as ::core::ffi::c_int
            | ('c' as i32 as ima_u8_t as ima_u32_t) << 24 as ::core::ffi::c_int
    {
        return -(1 as ::core::ffi::c_int);
    }
    if ima_btoh16((*header).version) as ::core::ffi::c_int != 1 as ::core::ffi::c_int {
        return -(2 as ::core::ffi::c_int);
    }
    loop {
        chunk_type = ima_btoh32((*chunk).type_0) as ::core::ffi::c_uint;
        chunk_size = ima_btoh64((*chunk).size as ima_u64_t) as ima_s64_t;
        if chunk_type
            == 'c' as i32 as ima_u8_t as ima_u32_t
                | ('s' as i32 as ima_u8_t as ima_u32_t) << 8 as ::core::ffi::c_int
                | ('e' as i32 as ima_u8_t as ima_u32_t) << 16 as ::core::ffi::c_int
                | ('d' as i32 as ima_u8_t as ima_u32_t) << 24 as ::core::ffi::c_int
        {
            desc = chunk.offset(1 as ::core::ffi::c_int as isize) as *const caf_chunk
                as *const caf_audio_description;
        } else if chunk_type
            == 't' as i32 as ima_u8_t as ima_u32_t
                | ('k' as i32 as ima_u8_t as ima_u32_t) << 8 as ::core::ffi::c_int
                | ('a' as i32 as ima_u8_t as ima_u32_t) << 16 as ::core::ffi::c_int
                | ('p' as i32 as ima_u8_t as ima_u32_t) << 24 as ::core::ffi::c_int
        {
            pakt = chunk.offset(1 as ::core::ffi::c_int as isize) as *const caf_chunk
                as *const caf_packet_table;
        } else if chunk_type
            == 'a' as i32 as ima_u8_t as ima_u32_t
                | ('t' as i32 as ima_u8_t as ima_u32_t) << 8 as ::core::ffi::c_int
                | ('a' as i32 as ima_u8_t as ima_u32_t) << 16 as ::core::ffi::c_int
                | ('d' as i32 as ima_u8_t as ima_u32_t) << 24 as ::core::ffi::c_int
        {
            blocks = (chunk.offset(1 as ::core::ffi::c_int as isize) as *const caf_chunk
                as *const caf_data)
                .offset(1 as ::core::ffi::c_int as isize) as *const caf_data
                as *const ima_block;
            break;
        }
        chunk = (chunk.offset(1 as ::core::ffi::c_int as isize) as *const caf_chunk
            as *const ima_u8_t)
            .offset(chunk_size as isize) as *const caf_chunk;
    }
    if ima_btoh32((*desc).format_id)
        != '4' as i32 as ima_u8_t as ima_u32_t
            | ('a' as i32 as ima_u8_t as ima_u32_t) << 8 as ::core::ffi::c_int
            | ('m' as i32 as ima_u8_t as ima_u32_t) << 16 as ::core::ffi::c_int
            | ('i' as i32 as ima_u8_t as ima_u32_t) << 24 as ::core::ffi::c_int
    {
        return -(3 as ::core::ffi::c_int);
    }
    (*info).blocks = blocks;
    (*info).size = chunk_size as ima_u64_t;
    (*info).frame_count = ima_btoh64((*pakt).frame_count as ima_u64_t);
    (*info).channel_count = ima_btoh32((*desc).channels_per_frame);
    conv64.u = (*desc).sample_rate as ima_u64_t;
    conv64.u = ima_btoh64(*(&raw mut conv64.u as *const ima_u64_t));
    (*info).sample_rate = conv64.f;
    return 0 as ::core::ffi::c_int;
}
